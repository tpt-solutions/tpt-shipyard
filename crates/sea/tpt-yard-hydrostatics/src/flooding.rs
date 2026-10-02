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
    /// Vertical-walled plan area, m².
    pub plan_area_m2: f64,
    /// Tank height above its bottom, m.
    pub height_m: f64,
    /// Flooding permeability (0..1; cargo/machinery spaces 0.6-0.85
    /// per Reg. 7-3).
    pub permeability: f64,
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

/// The flooded volume at a fill fraction, m³:
/// `fraction·height·area·permeability`.
pub fn flood_volume_m3(tank: &TankCompartment, fill_fraction: f64) -> Result<f64, FloodingError> {
    validate_tank(tank)?;
    if !fill_fraction.is_finite() || !(0.0..=1.0).contains(&fill_fraction) {
        return Err(FloodingError::InvalidFraction);
    }
    Ok(fill_fraction * tank.height_m * tank.plan_area_m2 * tank.permeability)
}

/// The floodwater CG height above baseline at a fill fraction, m: the
/// flooded body sits from the tank bottom up to the fill height, so its
/// CG is `bottom + fill_height/2` (permeability divides volume and CG
/// position identically for a vertical-walled tank).
pub fn flood_cg_z(tank: &TankCompartment, fill_fraction: f64) -> Result<f64, FloodingError> {
    validate_tank(tank)?;
    if !fill_fraction.is_finite() || !(0.0..=1.0).contains(&fill_fraction) {
        return Err(FloodingError::InvalidFraction);
    }
    Ok(tank.bottom_z + fill_fraction * tank.height_m / 2.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tank() -> TankCompartment {
        TankCompartment {
            name: "DB 3 P".into(),
            bottom_z: 1.0,
            plan_area_m2: 50.0,
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
