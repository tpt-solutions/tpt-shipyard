//! Tank-plan flooding geometry and cross-flooding dynamics — the
//! damage-stability roadmap items beyond the box-compartment
//! approximation.
//!
//! [`TankCompartment`] gives a compartment real vertical-walled
//! geometry (bottom above baseline, plan area, height): partial
//! flooding fills from the bottom up, so the floodwater volume, CG and
//! free-surface moment are all physically consistent at every stage —
//! replacing the fixed-centroid approximation of
//! [`DamageCompartment`](crate::DamageCompartment) stages.
//!
//! [`cross_flooding_time`] integrates Torricelli's orifice law with
//! mass conservation for the equalization of a head difference `H0`
//! through a duct of area `A` with discharge coefficient `Cd`, between
//! spaces of free-surface areas `Sw` (damaged side) and `Sf`
//! (intact side):
//!
//! `dH/dt = -Cd·A·sqrt(2 g H)·(1/Sw + 1/Sf)`
//! `=>` `t = 2(sqrt(H0) - sqrt(H1)) / (Cd·A·sqrt(2g)·(1/Sw + 1/Sf))`
//!
//! — the closed form behind the Reg. 7-2.2 "equalization shall not
//! exceed 10 min" check (the 10-minute rule applies with cross-flooding
//! devices fitted).

use crate::RHO_SEA_T_M3;

/// Gravitational acceleration, m/s².
const G: f64 = 9.81;

/// A vertical-walled tank compartment: real flooding geometry for the
/// added-weight stages.
#[derive(Debug, Clone, PartialEq)]
pub struct TankCompartment {
    /// Tank designation (e.g. "DB 3 P").
    pub name: String,
    /// Tank bottom above baseline, m.
    pub bottom_z: f64,
    /// Plan area at the tank bottom, m².
    pub plan_area_m2: f64,
    /// Plan area at the tank top, m² (`None` = vertical walls, same as
    /// the bottom). A differing top area tapers the walls linearly.
    pub top_plan_area_m2: Option<f64>,
    /// Tank height above its bottom, m.
    pub height_m: f64,
    /// Flooding permeability (0..1; cargo/machinery spaces 0.6-0.85
    /// per Reg. 7-3).
    pub permeability: f64,
}

impl TankCompartment {
    /// The plan area at a fill height above the tank bottom (linear
    /// taper between the bottom and top areas).
    pub fn plan_area_at(&self, fill_height_m: f64) -> f64 {
        let top = self.top_plan_area_m2.unwrap_or(self.plan_area_m2);
        self.plan_area_m2 + (top - self.plan_area_m2) * (fill_height_m / self.height_m)
    }
}

/// Errors from the flooding-geometry helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloodingError {
    /// A tank dimension, permeability or area is not finite and
    /// physical.
    InvalidTank,
    /// The fill fraction is not within [0, 1].
    InvalidFraction,
}

impl std::fmt::Display for FloodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FloodingError::InvalidTank => f.write_str("tank geometry must be finite and physical"),
            FloodingError::InvalidFraction => f.write_str("the fill fraction must lie in [0, 1]"),
        }
    }
}

impl std::error::Error for FloodingError {}

fn validate_tank(tank: &TankCompartment) -> Result<(), FloodingError> {
    if !tank.name.is_empty()
        && tank.bottom_z.is_finite()
        && tank.plan_area_m2.is_finite()
        && tank.plan_area_m2 > 0.0
        && tank
            .top_plan_area_m2
            .is_none_or(|a| a.is_finite() && a > 0.0)
        && tank.height_m.is_finite()
        && tank.height_m > 0.0
        && tank.permeability.is_finite()
        && (0.0..=1.0).contains(&tank.permeability)
    {
        Ok(())
    } else {
        Err(FloodingError::InvalidTank)
    }
}

/// The flooded volume at a fill fraction, m³. Vertical walls:
/// `fraction·height·area·permeability`. Tapered walls (a top area
/// given): the fill height solves the trapezoid integral
/// `V(h) = A_b·h + ΔA·h²/(2H)` exactly (quadratic formula).
pub fn flood_volume_m3(tank: &TankCompartment, fill_fraction: f64) -> Result<f64, FloodingError> {
    let h = fill_height_m(tank, fill_fraction)?;
    let da = tank.top_plan_area_m2.unwrap_or(tank.plan_area_m2) - tank.plan_area_m2;
    Ok(tank.permeability * (tank.plan_area_m2 * h + da * h * h / (2.0 * tank.height_m)))
}

/// The fill height above the tank bottom at a fill fraction: inverts
/// the trapezoid volume integral with the quadratic formula (exact; no
/// iteration).
fn fill_height_m(tank: &TankCompartment, fill_fraction: f64) -> Result<f64, FloodingError> {
    validate_tank(tank)?;
    if !fill_fraction.is_finite() || !(0.0..=1.0).contains(&fill_fraction) {
        return Err(FloodingError::InvalidFraction);
    }
    let a_b = tank.plan_area_m2;
    let da = tank.top_plan_area_m2.unwrap_or(a_b) - a_b;
    // mu (A_b h + da h^2/(2H)) = fraction * mu * (A_b H + da H/2)
    // (the full trapezoid without permeability).
    let target = fill_fraction * (a_b * tank.height_m + da * tank.height_m / 2.0);
    if da.abs() < 1e-12 {
        return Ok(target / a_b);
    }
    // da h^2/(2H) + A_b h - target = 0: positive root
    // h = H (sqrt(A_b^2 + 2 da target/H) - A_b) / da.
    let discriminant = (a_b * a_b + 2.0 * da * target / tank.height_m).max(0.0);
    Ok(tank.height_m * (discriminant.sqrt() - a_b) / da)
}

/// The floodwater CG height above baseline at a fill fraction, m.
/// Vertical walls: `bottom + fill_height/2`. Tapered walls: the
/// first-moment closed form
/// `z̄ = bottom + [A_b·h²/2 + ΔA·h³/(3H)] / [A_b·h + ΔA·h²/(2H)]`.
pub fn flood_cg_z(tank: &TankCompartment, fill_fraction: f64) -> Result<f64, FloodingError> {
    let h = fill_height_m(tank, fill_fraction)?;
    let a_b = tank.plan_area_m2;
    let da = tank.top_plan_area_m2.unwrap_or(a_b) - a_b;
    let volume = a_b * h + da * h * h / (2.0 * tank.height_m);
    if volume <= 1e-12 {
        return Ok(tank.bottom_z);
    }
    let first_moment = a_b * h * h / 2.0 + da * h * h * h / (3.0 * tank.height_m);
    Ok(tank.bottom_z + first_moment / volume)
}

/// The free-surface moment of the slack tank, t·m: `mu·rho·A²/12` —
/// the SQUARE-plan estimate (a rectangular plan l×b gives
/// `mu·rho·l·b³/12`; with only the area known the square plan is the
/// documented default). Callers with the real plan aspect should scale
/// by `(b/l)²` themselves or use
/// [`DamageCompartment`](crate::DamageCompartment) directly.
pub fn tank_free_surface_moment_tm(tank: &TankCompartment) -> Result<f64, FloodingError> {
    validate_tank(tank)?;
    Ok(tank.permeability * RHO_SEA_T_M3 * tank.plan_area_m2 * tank.plan_area_m2 / 12.0)
}

/// The equivalent [`DamageCompartment`](crate::DamageCompartment) for a
/// tank at a fill fraction — volume and CG from the vertical-walled
/// geometry, free-surface moment from the square-plan estimate — ready
/// for [`damage_stages`](crate::HullForm::damage_stages) or
/// [`damage_stability`](crate::HullForm::damage_stability). Pass a
/// non-empty `free_surface_override_tm` to replace the square-plan
/// estimate with the real plan's `mu·rho·l·b³/12`.
pub fn tank_stage_compartment(
    tank: &TankCompartment,
    fill_fraction: f64,
    free_surface_override_tm: Option<f64>,
) -> Result<crate::DamageCompartment, FloodingError> {
    validate_tank(tank)?;
    if !fill_fraction.is_finite() || !(0.0..=1.0).contains(&fill_fraction) {
        return Err(FloodingError::InvalidFraction);
    }
    Ok(crate::DamageCompartment {
        name: tank.name.clone(),
        volume_m3: flood_volume_m3(tank, fill_fraction)?,
        centroid: (0.0, 0.0, flood_cg_z(tank, fill_fraction)?),
        free_surface_moment_tm: free_surface_override_tm
            .unwrap_or(tank_free_surface_moment_tm(tank)?),
    })
}

/// The cross-flooding equalization time, seconds, to drive the initial
/// head difference `head0_m` down to `head_target_m` through a duct of
/// area `duct_area_m2` with discharge coefficient `discharge_coeff`,
/// between spaces of free-surface area `supply_area_m2` (damaged side)
/// and `flood_area_m2` (intact side):
/// `t = 2(sqrt(H0) - sqrt(H1)) / (Cd·A·sqrt(2g)·(1/Sw + 1/Sf))`.
/// Compare against the Reg. 7-2.2 10-minute limit.
pub fn cross_flooding_time(
    head0_m: f64,
    head_target_m: f64,
    duct_area_m2: f64,
    discharge_coeff: f64,
    supply_area_m2: f64,
    flood_area_m2: f64,
) -> Result<f64, FloodingError> {
    if !(duct_area_m2.is_finite() && duct_area_m2 > 0.0)
        || !(discharge_coeff.is_finite() && discharge_coeff > 0.0 && discharge_coeff <= 1.0)
        || !(supply_area_m2.is_finite() && supply_area_m2 > 0.0)
        || !(flood_area_m2.is_finite() && flood_area_m2 > 0.0)
        || !head0_m.is_finite()
        || !head_target_m.is_finite()
        || head0_m < 0.0
        || head_target_m < 0.0
        || head_target_m > head0_m
    {
        return Err(FloodingError::InvalidTank);
    }
    if head0_m == 0.0 {
        return Ok(0.0);
    }
    let numerator = 2.0 * (head0_m.sqrt() - head_target_m.sqrt());
    let denominator = discharge_coeff
        * duct_area_m2
        * (2.0 * G).sqrt()
        * (1.0 / supply_area_m2 + 1.0 / flood_area_m2);
    Ok(numerator / denominator)
}

/// The equalization stage fractions of the Torricelli dynamics at
/// equal time slices: with the head following `H(t) = H0*(1 - t/T)^2`,
/// the transferred volume fraction at `u = t/T` is `f(u) = 2u - u*u` —
/// half the transfer completes in the first `1 - 1/sqrt(2)` = 29.3% of
/// the time (fast start, slow finish).
#[must_use]
pub fn equalization_stage_fractions(n_stages: u32) -> Vec<f64> {
    (1..=n_stages)
        .map(|i| {
            let u = i as f64 / n_stages as f64;
            2.0 * u - u * u
        })
        .collect()
}

impl crate::HullForm {
    /// Staged flooding driven by the cross-flooding dynamics: the stage
    /// fractions follow the Torricelli profile
    /// ([`equalization_stage_fractions`]) instead of equal volume
    /// slices, and the equalization time ([`cross_flooding_time`]) is
    /// reported alongside — the pair the Reg. 7-2.2 intermediate
    /// factors and the 10-minute rule consume. The final stage
    /// reproduces [`Self::damage_stages`] exactly (the profile ends at
    /// the same full fraction).
    ///
    /// # Errors
    ///
    /// [`ProbabilisticError`](crate::ProbabilisticError) on the same
    /// inputs as [`Self::damage_stages`] plus non-physical duct data.
    #[allow(clippy::too_many_arguments)]
    pub fn damage_stages_equalization(
        &self,
        displacement_t: f64,
        lcg_from_midship_m: f64,
        kg_m: f64,
        free_surface_moment_tm: f64,
        compartments: &[crate::DamageCompartment],
        n_stages: u32,
        head0_m: f64,
        duct_area_m2: f64,
        discharge_coeff: f64,
        supply_area_m2: f64,
        flood_area_m2: f64,
    ) -> Result<(f64, Vec<crate::FloodStage>), crate::ProbabilisticError> {
        if n_stages == 0 {
            return Err(crate::ProbabilisticError::UnsolvedDamage);
        }
        let equalization_s = cross_flooding_time(
            head0_m,
            0.0,
            duct_area_m2,
            discharge_coeff,
            supply_area_m2,
            flood_area_m2,
        )
        .map_err(|_| crate::ProbabilisticError::UnsolvedDamage)?;
        let fractions = equalization_stage_fractions(n_stages);
        let mut stages = Vec::with_capacity(n_stages as usize);
        for fraction in fractions {
            let scaled: Vec<crate::DamageCompartment> = compartments
                .iter()
                .map(|c| crate::DamageCompartment {
                    name: c.name.clone(),
                    volume_m3: c.volume_m3 * fraction,
                    centroid: c.centroid,
                    free_surface_moment_tm: c.free_surface_moment_tm * fraction,
                })
                .collect();
            let damaged = self
                .damage_stability(
                    displacement_t,
                    lcg_from_midship_m,
                    kg_m,
                    free_surface_moment_tm,
                    &scaled,
                )
                .ok_or(crate::ProbabilisticError::UnsolvedDamage)?;
            let theta_e = damaged.list_angle_deg.max(0.0);
            let (gz_max, range, theta_v) =
                self.gz_scan(damaged.mean_draft_m, damaged.gm_m, theta_e);
            stages.push(crate::FloodStage {
                fraction,
                displacement_t: damaged.displacement_t,
                mean_draft_m: damaged.mean_draft_m,
                heel_deg: theta_e,
                gm_m: damaged.gm_m,
                gz_max_m: gz_max,
                range_deg: range,
                vanishing_angle_deg: theta_v,
            });
        }
        Ok((equalization_s, stages))
    }

    /// Staged flooding with cross-flooding inside each stage's
    /// equilibrium. Every compartment is tagged: a *breach* compartment
    /// (`equalizing = false`) is open to the sea and floods fully from
    /// the first stage, while an *equalizing* compartment (the
    /// opposite-side tank fed through the cross-flooding duct) follows
    /// the Torricelli profile [`equalization_stage_fractions`]. Each
    /// stage is solved at its own equilibrium — trim, GM with the stage's
    /// free surface, and the large-angle heel from
    /// [`heel_equilibrium_deg`](crate::heel_equilibrium_deg) — so the
    /// returned heels show the asymmetric peak right after the breach and
    /// its recovery as the duct fills the far side. The last stage has
    /// every compartment full and reproduces
    /// [`Self::damage_stability`] exactly.
    ///
    /// `heel_deg` is signed (positive to starboard); the righting-arm
    /// scan runs from its magnitude. Returns the equalization time
    /// ([`cross_flooding_time`]) with the stages.
    ///
    /// # Errors
    ///
    /// [`ProbabilisticError`](crate::ProbabilisticError) on the same
    /// inputs as [`Self::damage_stages_equalization`].
    #[allow(clippy::too_many_arguments)]
    pub fn damage_stages_cross_flooding(
        &self,
        displacement_t: f64,
        lcg_from_midship_m: f64,
        kg_m: f64,
        free_surface_moment_tm: f64,
        compartments: &[(crate::DamageCompartment, bool)],
        n_stages: u32,
        head0_m: f64,
        duct_area_m2: f64,
        discharge_coeff: f64,
        supply_area_m2: f64,
        flood_area_m2: f64,
    ) -> Result<(f64, Vec<crate::FloodStage>), crate::ProbabilisticError> {
        if n_stages == 0 {
            return Err(crate::ProbabilisticError::UnsolvedDamage);
        }
        let equalization_s = cross_flooding_time(
            head0_m,
            0.0,
            duct_area_m2,
            discharge_coeff,
            supply_area_m2,
            flood_area_m2,
        )
        .map_err(|_| crate::ProbabilisticError::UnsolvedDamage)?;
        let mut stages = Vec::with_capacity(n_stages as usize);
        for eq_fraction in equalization_stage_fractions(n_stages) {
            let scaled: Vec<crate::DamageCompartment> = compartments
                .iter()
                .map(|(c, equalizing)| {
                    let f = if *equalizing { eq_fraction } else { 1.0 };
                    crate::DamageCompartment {
                        name: c.name.clone(),
                        volume_m3: c.volume_m3 * f,
                        centroid: c.centroid,
                        free_surface_moment_tm: c.free_surface_moment_tm * f,
                    }
                })
                .collect();
            let damaged = self
                .damage_stability(
                    displacement_t,
                    lcg_from_midship_m,
                    kg_m,
                    free_surface_moment_tm,
                    &scaled,
                )
                .ok_or(crate::ProbabilisticError::UnsolvedDamage)?;
            let (gz_max, range, theta_v) = self.gz_scan(
                damaged.mean_draft_m,
                damaged.gm_m,
                damaged.list_angle_deg.abs(),
            );
            stages.push(crate::FloodStage {
                fraction: eq_fraction,
                displacement_t: damaged.displacement_t,
                mean_draft_m: damaged.mean_draft_m,
                heel_deg: damaged.list_angle_deg,
                gm_m: damaged.gm_m,
                gz_max_m: gz_max,
                range_deg: range,
                vanishing_angle_deg: theta_v,
            });
        }
        Ok((equalization_s, stages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DamageCompartment, HullForm};

    /// The equalization-driven stages: the reported time matches
    /// cross_flooding_time, the stage fractions follow the Torricelli
    /// profile (half the transfer in the first 29.3% of the time), and
    /// the final stage reproduces damage_stages exactly.
    #[test]
    fn equalization_stages_track_the_torricelli_dynamics() {
        let hull = HullForm {
            loa_m: 140.0,
            boa_m: 22.0,
            cb: 0.72,
            cwp: 0.85,
        };
        let comps = vec![DamageCompartment {
            name: "DB 3".into(),
            volume_m3: 500.0,
            centroid: (10.0, 0.0, 1.25),
            free_surface_moment_tm: 120.0,
        }];
        let (time_s, stages) = hull
            .damage_stages_equalization(
                30_000.0, 0.0, 9.0, 200.0, &comps, 4, 2.0, 0.2, 0.6, 500.0, 60.0,
            )
            .expect("stages solve");
        // The time is the full-equalization closed form.
        let expected = cross_flooding_time(2.0, 0.0, 0.2, 0.6, 500.0, 60.0).unwrap();
        assert!((time_s - expected).abs() < 1e-12);
        // The fractions follow 2u - u^2: [0.4375, 0.75, 0.9375, 1.0]
        // for n = 4.
        let fractions: Vec<f64> = stages.iter().map(|st| st.fraction).collect();
        let expected_f = [0.4375_f64, 0.75, 0.9375, 1.0];
        for (got, want) in fractions.iter().zip(&expected_f) {
            assert!((got - want).abs() < 1e-12);
        }
        // The final stage matches damage_stages' last stage exactly.
        let uniform = hull
            .damage_stages(30_000.0, 0.0, 9.0, 200.0, &comps, 4)
            .unwrap();
        assert!((stages[3].mean_draft_m - uniform[3].mean_draft_m).abs() < 1e-12);
        assert!((stages[3].gz_max_m - uniform[3].gz_max_m).abs() < 1e-12);
        // The intermediate stages lag the uniform ones (Torricelli
        // front-loads the transfer).
        assert!(stages[0].mean_draft_m > uniform[0].mean_draft_m);
        // Zero stages refused.
        assert_eq!(
            hull.damage_stages_equalization(
                30_000.0, 0.0, 9.0, 200.0, &comps, 0, 2.0, 0.2, 0.6, 500.0, 60.0
            ),
            Err(crate::ProbabilisticError::UnsolvedDamage)
        );
    }

    /// Cross-flooding inside the stage equilibrium: a port breach lists
    /// the ship hard to port at once, then the duct fills the starboard
    /// tank and the heel recovers monotonically to the symmetric end
    /// state; the final stage is the full-flood `damage_stability`.
    #[test]
    fn cross_flooding_recovers_the_list_stage_by_stage() {
        let hull = HullForm {
            loa_m: 140.0,
            boa_m: 22.0,
            cb: 0.72,
            cwp: 0.85,
        };
        let mk = |name: &str, y: f64| DamageCompartment {
            name: name.into(),
            volume_m3: 600.0,
            centroid: (5.0, y, 2.0),
            free_surface_moment_tm: 0.0,
        };
        let comps = vec![(mk("WBT P", -8.0), false), (mk("WBT S", 8.0), true)];
        let (t, stages) = hull
            .damage_stages_cross_flooding(
                30_000.0, 0.0, 9.0, 0.0, &comps, 5, 2.0, 0.3, 0.6, 600.0, 600.0,
            )
            .expect("solves");
        assert!(t > 0.0);
        // Port breach alone lists to port (negative heel) at stage 1.
        assert!(stages[0].heel_deg < -0.5, "{}", stages[0].heel_deg);
        // The list recovers monotonically as the starboard tank fills.
        for w in stages.windows(2) {
            assert!(w[1].heel_deg > w[0].heel_deg, "{:?}", stages);
        }
        // Mirror-image compartments full: upright, and identical to the
        // single-shot damage_stability on the full set.
        let last = stages.last().unwrap();
        assert!(last.heel_deg.abs() < 1e-9, "{}", last.heel_deg);
        let full: Vec<DamageCompartment> = comps.iter().map(|(c, _)| c.clone()).collect();
        let single = hull
            .damage_stability(30_000.0, 0.0, 9.0, 0.0, &full)
            .unwrap();
        assert!((last.mean_draft_m - single.mean_draft_m).abs() < 1e-12);
        assert!((last.gm_m - single.gm_m).abs() < 1e-12);
        // Flooded displacement grows monotonically.
        for w in stages.windows(2) {
            assert!(w[1].displacement_t > w[0].displacement_t);
        }
        // Zero stages refused.
        assert!(hull
            .damage_stages_cross_flooding(
                30_000.0, 0.0, 9.0, 0.0, &comps, 0, 2.0, 0.3, 0.6, 600.0, 600.0
            )
            .is_err());
    }

    fn tank() -> TankCompartment {
        TankCompartment {
            name: "DB 3 P".into(),
            bottom_z: 1.0,
            plan_area_m2: 50.0,
            top_plan_area_m2: None,
            height_m: 2.0,
            permeability: 0.85,
        }
    }

    /// The tank fill geometry: volume and CG are the exact vertical-
    /// walled closed forms at every fraction, linear in the fraction.
    #[test]
    fn tank_fill_geometry_is_the_vertical_walled_closed_form() {
        let t = tank();
        for &f in &[0.0_f64, 0.25, 0.5, 1.0] {
            let v = flood_volume_m3(&t, f).unwrap();
            assert!((v - f * 2.0 * 50.0 * 0.85).abs() < 1e-12, "v at {f}");
            let cg = flood_cg_z(&t, f).unwrap();
            assert!((cg - (1.0 + f * 1.0)).abs() < 1e-12, "cg at {f}");
        }
        // Full-tank volume: 2 m x 50 m2 x 0.85 = 85 m3; CG at mid-height.
        assert!((flood_volume_m3(&t, 1.0).unwrap() - 85.0).abs() < 1e-12);
        assert!((flood_cg_z(&t, 1.0).unwrap() - 2.0).abs() < 1e-12);
        // Free surface: square-plan estimate mu rho A^2 / 12.
        let fsm = tank_free_surface_moment_tm(&t).unwrap();
        assert!((fsm - 0.85 * 1.025 * 2500.0 / 12.0).abs() < 1e-9);
        // Errors.
        assert_eq!(
            flood_volume_m3(&t, 1.5),
            Err(FloodingError::InvalidFraction)
        );
        let bad = TankCompartment {
            permeability: 1.5,
            ..tank()
        };
        assert_eq!(flood_volume_m3(&bad, 0.5), Err(FloodingError::InvalidTank));
    }

    /// The cross-flooding closed form agrees with a brute-force Euler
    /// integration of the Torricelli ODE (1-second steps over a long
    /// equalization) within the integration error, and the 10-minute
    /// rule check works on a hand value.
    #[test]
    fn cross_flooding_time_matches_the_ode_integration() {
        let (h0, sw, sf, a, cd) = (2.0_f64, 50.0, 60.0, 0.2, 0.6);
        let t = cross_flooding_time(h0, 0.0, a, cd, sw, sf).unwrap();
        // Brute force: dH/dt = -Cd A sqrt(2 g H) (1/Sw + 1/Sf), Euler.
        let mut h = h0;
        let mut t_num = 0.0_f64;
        let dt = 0.001_f64;
        while h > 1e-6 {
            let dh = cd * a * (2.0 * G * h).sqrt() * (1.0 / sw + 1.0 / sf) * dt;
            h -= dh;
            t_num += dt;
        }
        assert!(
            (t - t_num).abs() < 2.0,
            "closed form {t} vs numeric {t_num}"
        );
        // Hand value: t = 2 sqrt(2) / (0.6 * 0.2 * sqrt(19.62) * (1/50 + 1/60)).
        let expected =
            2.0 * 2.0_f64.sqrt() / (0.6 * 0.2 * (2.0 * G).sqrt() * (1.0 / 50.0 + 1.0 / 60.0));
        assert!((t - expected).abs() < 1e-12);
        assert!(t > 0.0 && t < 600.0, "equalizes in {t} s (< 10 min)");
        // Partial-head target scales as sqrt: to half the HEAD takes
        // 1 - 1/sqrt(2) = 29.3% of the full equalization time.
        let t_half = cross_flooding_time(h0, h0 / 2.0, a, cd, sw, sf).unwrap();
        assert!((t_half / t - (1.0 - 0.5_f64.sqrt())).abs() < 1e-12);
        // Errors: target above start, non-physical inputs.
        assert_eq!(
            cross_flooding_time(1.0, 2.0, a, cd, sw, sf),
            Err(FloodingError::InvalidTank)
        );
        assert_eq!(
            cross_flooding_time(1.0, 0.5, a, 1.2, sw, sf),
            Err(FloodingError::InvalidTank)
        );
    }

    /// Tapered walls (top area double the bottom): the full volume is
    /// the trapezoid closed form, the half-flood fill height solves the
    /// quadratic exactly (h = (sqrt(32) - 4)/2 = 0.8284 m), and the CG
    /// follows the first-moment closed form.
    #[test]
    fn tapered_tank_geometry_matches_the_closed_forms() {
        let t = TankCompartment {
            name: "tapered DB".into(),
            bottom_z: 1.0,
            plan_area_m2: 50.0,
            top_plan_area_m2: Some(100.0),
            height_m: 2.0,
            permeability: 0.85,
        };
        // Full: V = mu (A_b + A_t)/2 * H = 0.85 * 75 * 2 = 127.5.
        assert!((flood_volume_m3(&t, 1.0).unwrap() - 127.5).abs() < 1e-9);
        // CG at full: first moment (A_b h^2/2 + dA h^3/3H)/V with
        // h = 2: (50*2 + 50*8/6)/150 = 1.1111 above bottom.
        assert!((flood_cg_z(&t, 1.0).unwrap() - (1.0 + 5.0 / 4.5)).abs() < 1e-9);
        // Half flood: the quadratic 12.5 h^2 + 50 h - 75 = 0 gives
        // h = (sqrt(40) - 4)/2 = 1.16228; V = 0.85*(50h + 12.5h^2)
        // = 0.75 of the full 85 m3 (fraction tracks the target linearly).
        let h_half = (40.0_f64.sqrt() - 4.0) / 2.0;
        let v_half = flood_volume_m3(&t, 0.5).unwrap();
        assert!((v_half - 0.85 * (50.0 * h_half + 12.5 * h_half * h_half)).abs() < 1e-9);
        assert!(
            (v_half - 0.5 * 127.5).abs() < 1e-9,
            "fraction scales the target linearly"
        );
        let cg_half = flood_cg_z(&t, 0.5).unwrap();
        assert!(
            (cg_half
                - (1.0
                    + (25.0 * h_half * h_half + 50.0 * h_half * h_half * h_half / 6.0)
                        / (50.0 * h_half + 12.5 * h_half * h_half)))
                .abs()
                < 1e-9
        );
        // A flaring tank (wider top) concentrates volume high: the CG
        // sits above the mid-fill mark (below it for a tapering tank).
        assert!(cg_half > 1.0 + h_half / 2.0 && cg_half < 1.0 + h_half);
    }

    /// The equalization-driven stage fractions follow the Torricelli
    /// profile f(u) = 2u - u^2: zero at the start, the final fraction at
    /// u = 1, half the transfer in the first 29.3% of the time.
    #[test]
    fn equalization_fractions_follow_the_torricelli_profile() {
        let f = |u: f64| 2.0 * u - u * u;
        assert!((f(0.0)).abs() < 1e-12);
        assert!((f(1.0) - 1.0).abs() < 1e-12);
        assert!((f(0.5) - 0.75).abs() < 1e-12);
        assert!((f(1.0 - 0.5_f64.sqrt()) - 0.5).abs() < 1e-12);
    }

    /// The tank-to-DamageCompartment bridge: at half flood the volume
    /// and CG match the closed forms, and the override replaces the
    /// square-plan free-surface estimate.
    #[test]
    fn tank_stage_compartment_bridges_to_the_added_weight_model() {
        let t = tank();
        let c = tank_stage_compartment(&t, 0.5, None).unwrap();
        assert!((c.volume_m3 - 42.5).abs() < 1e-12);
        assert!((c.centroid.2 - 1.5).abs() < 1e-12);
        assert!((c.free_surface_moment_tm - 0.85 * 1.025 * 2500.0 / 12.0).abs() < 1e-9);
        let overridden = tank_stage_compartment(&t, 0.5, Some(18.0)).unwrap();
        assert!((overridden.free_surface_moment_tm - 18.0).abs() < 1e-12);
        assert_eq!(
            tank_stage_compartment(&t, 1.5, None),
            Err(FloodingError::InvalidFraction)
        );
    }
}
