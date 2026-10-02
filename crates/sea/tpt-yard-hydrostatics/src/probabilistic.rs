//! Probabilistic damage stability — the SOLAS Ch. II-1 Part B-1
//! harmonized method (resolution MSC.216(82), as amended): the
//! damage-length probability density and the longitudinal zone factor
//! `p` (Reg. 7-1), the survival factor `s` for cargo ships (Reg. 7-2),
//! the required subdivision index `R` (Reg. 6) and the attained index
//! `A = Σ p·s` over a damage case set.
//!
//! Sources: the SOLAS Ch. II-1 regulation text (Reg. 6, 7-1, 7-2) and
//! the Revised Explanatory Notes (MSC.429(98)/Rev.1). The bi-linear
//! damage-length density is implemented from its defining properties —
//! cumulative probability `pk = 11/12` at the knuckle `Jkn = 5/33`,
//! total probability 1 over `[0, Jm]`, continuity at the knuckle and
//! zero density at `Jm` — which pin the four published coefficients
//! down exactly; the tests verify the resulting `p(x1, x2)` against the
//! regulation's printed closed forms and against numerical integration
//! of the density.
//!
//! First-slice scope (documented honestly): single-zone damages with no
//! longitudinal bulkheads inside the zone (`r = 1`) and full-height
//! compartments (`v = 1`), cargo-ship survival (`s_intermediate = 1`,
//! `s_moment = 1`). Longitudinal-bulkhead `r`/`v` reductions, passenger
//! intermediate stages and the 80–100 m R interpolation remain
//! class-society work.

use crate::{DamageCompartment, HullForm};

/// Overall normalized maximum damage length (Reg. 7-1.1): `Jmax = 10/33`.
const J_MAX: f64 = 10.0 / 33.0;
/// Knuckle point of the damage-length distribution: `Jkn = 5/33`.
const J_KN: f64 = 5.0 / 33.0;
/// Cumulative probability of the damage-length density at `Jkn`.
const P_K: f64 = 11.0 / 12.0;
/// Maximum absolute damage length, m (Reg. 7-1.1).
const L_MAX_DAMAGE_M: f64 = 60.0;
/// Length above which the normalized distribution is rescaled, m.
const L_STAR_M: f64 = 260.0;

/// Errors from the probabilistic damage stability module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbabilisticError {
    /// The subdivision length must be finite and positive.
    InvalidSubdivisionLength,
    /// The damage zone bounds must satisfy `0 <= x1 < x2 <= Ls` (m).
    InvalidZone,
    /// The damaged equilibrium could not be solved for the given loading.
    UnsolvedDamage,
}

impl std::fmt::Display for ProbabilisticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbabilisticError::InvalidSubdivisionLength => {
                f.write_str("subdivision length must be finite and positive")
            }
            ProbabilisticError::InvalidZone => f.write_str(
                "zone bounds must satisfy 0 <= aft end < forward end <= subdivision length",
            ),
            ProbabilisticError::UnsolvedDamage => {
                f.write_str("damaged equilibrium could not be solved")
            }
        }
    }
}

impl std::error::Error for ProbabilisticError {}

/// The bi-linear damage-length probability density of Reg. 7-1.1,
/// evaluated on the normalized damage length `J = (x2 - x1)/Ls`:
///
/// - `b(J) = b12 + b11·J` on `[0, Jk]` (intercept `b12`, slope `b11`),
/// - `b(J) = b22 + b21·J` on `[Jk, Jm]` (intercept `b22`, slope `b21`).
///
/// The four coefficients are fixed by the published density properties:
/// cumulative `pk = 11/12` at `Jk`, total integral 1 over `[0, Jm]`,
/// continuity at the knuckle, and zero density at `Jm` (the maximum
/// normalized damage length).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageLengthDensity {
    /// Maximum normalized damage length for this ship: `min(Jmax, 60/Ls)`,
    /// rescaled by `L*/Ls` above `L* = 260 m`.
    pub jm: f64,
    /// Knuckle point of the distribution on this ship.
    pub jk: f64,
    /// Slope of the first (short-damage) density piece.
    pub b11: f64,
    /// Intercept of the first density piece (the density at `J = 0`).
    pub b12: f64,
    /// Slope of the second (long-damage) density piece.
    pub b21: f64,
    /// Intercept of the second density piece.
    pub b22: f64,
}

impl DamageLengthDensity {
    /// Builds the density for a subdivision length `Ls` (m).
    ///
    /// # Errors
    ///
    /// [`ProbabilisticError::InvalidSubdivisionLength`] on a
    /// non-finite or non-positive length.
    pub fn for_subdivision_length(ls_m: f64) -> Result<Self, ProbabilisticError> {
        if !ls_m.is_finite() || ls_m <= 0.0 {
            return Err(ProbabilisticError::InvalidSubdivisionLength);
        }
        let (jm, jk) = if ls_m <= L_STAR_M {
            let jm = J_MAX.min(L_MAX_DAMAGE_M / ls_m);
            (jm, J_KN.min(jm))
        } else {
            // Above L* the reference distribution at 260 m is rescaled:
            // Jm* = min(Jmax, 60/260) = 3/13, Jk* = 5/33.
            let scale = L_STAR_M / ls_m;
            ((L_MAX_DAMAGE_M / L_STAR_M) * scale, J_KN * scale)
        };
        // Second piece through (Jm, 0) with the tail integral 1 - pk:
        //   b22(Jm - Jk) + b21 (Jm^2 - Jk^2)/2 = 1 - pk,  b22 + b21 Jm = 0
        // => b21 = -2(1-pk)/(Jm-Jk)^2,  b22 = -b21 Jm.
        let denom = (jm - jk).powi(2);
        let b21 = -2.0 * (1.0 - P_K) / denom;
        let b22 = -b21 * jm;
        // First piece from head integral pk and continuity at Jk:
        //   b12 Jk + b11 Jk^2/2 = pk,  b12 + b11 Jk = b22 + b21 Jk.
        let b11 = 2.0 * (b22 * jk + b21 * jk * jk - P_K) / (jk * jk);
        let b12 = P_K / jk - b11 * jk / 2.0;
        Ok(Self {
            jm,
            jk,
            b11,
            b12,
            b21,
            b22,
        })
    }

    /// The density value at a normalized damage length (clamped to
    /// `[0, Jm]`; outside the support the density is 0).
    pub fn density(&self, j: f64) -> f64 {
        if j < 0.0 || j > self.jm {
            0.0
        } else if j <= self.jk {
            self.b12 + self.b11 * j
        } else {
            self.b22 + self.b21 * j
        }
    }

    /// The cumulative distribution `∫0^J b(j) dj`.
    pub fn cumulative(&self, j: f64) -> f64 {
        let j = j.clamp(0.0, self.jm);
        let head = self.b12 * j + self.b11 * j * j / 2.0;
        if j <= self.jk {
            head
        } else {
            let head_k = self.b12 * self.jk + self.b11 * self.jk * self.jk / 2.0;
            let tail = self.b22 * (j - self.jk) + self.b21 * (j * j - self.jk * self.jk) / 2.0;
            head_k + tail
        }
    }
}

/// The longitudinal zone factor `p(x1, x2)` of Reg. 7-1.1: the
/// probability that the damage opens exactly the zone `[x1, x2]`
/// (metres from the aft terminal of `Ls`), given the ship's
/// damage-length density.
///
/// Implements the regulation's three cases:
///
/// - whole subdivision length: `p = 1` (Reg. 7-1.1.3);
/// - one zone limit at a terminal: `p = (p_i + J)/2` with `J` the raw
///   normalized zone length (Reg. 7-1.1.2 — the terminal truncates the
///   damage, so long zones reach part of the density the mid-ship
///   containment window misses);
/// - neither limit at a terminal: `p = p1` (`J <= Jk`) or `p2`
///   (`J > Jk`), the printed closed forms of the contained-damage
///   integral `∫0^Jn b(j)(Jn - j) dj` with `Jn = min(J, Jm)`
///   (Reg. 7-1.1.1).
///
/// # Errors
///
/// [`ProbabilisticError::InvalidZone`] unless `0 <= x1 < x2 <= Ls`.
pub fn p_factor(
    density: &DamageLengthDensity,
    ls_m: f64,
    x1_m: f64,
    x2_m: f64,
) -> Result<f64, ProbabilisticError> {
    if !(ls_m.is_finite() && ls_m > 0.0)
        || !(x1_m.is_finite() && x2_m.is_finite())
        || !(0.0..=ls_m).contains(&x1_m)
        || !(x1_m..=ls_m).contains(&x2_m)
        || x2_m <= x1_m
    {
        return Err(ProbabilisticError::InvalidZone);
    }
    if x1_m <= 0.0 && x2_m >= ls_m {
        return Ok(1.0);
    }
    let j = (x2_m - x1_m) / ls_m;
    let jn = j.min(density.jm);
    let mid = if jn <= density.jk {
        // p1 = J^2/6 (b11 J + 3 b12)
        jn * jn / 6.0 * (density.b11 * jn + 3.0 * density.b12)
    } else {
        // p2: the printed continuation of the contained-damage integral
        // beyond the knuckle — exactly Integral[0..Jn] b(j)(J - j) dj:
        // the linear-coefficient terms carry the RAW zone length J (the
        // start window of a longer zone keeps growing past Jm), while
        // the density integration runs only over Jn. Using Jn everywhere
        // saturates long zones and drives the multi-zone combinations
        // negative — caught by the three-zone regulation test.
        let jk = density.jk;
        let (b11, b12, b21, b22) = (density.b11, density.b12, density.b21, density.b22);
        -(1.0 / 3.0) * b11 * jk * jk * jk + (1.0 / 2.0) * (b11 * j - b12) * jk * jk + b12 * j * jk
            - (1.0 / 3.0) * b21 * (jn * jn * jn - jk * jk * jk)
            + (1.0 / 2.0) * (b21 * j - b22) * (jn * jn - jk * jk)
            + b22 * j * (jn - jk)
    };
    if x1_m <= 0.0 || x2_m >= ls_m {
        Ok(0.5 * (mid + j))
    } else {
        Ok(mid)
    }
}

/// The survival factor `s` of Reg. 7-2 for a cargo ship's final
/// equilibrium stage (`s_intermediate = 1`, `s_moment = 1` for cargo):
///
/// `s = K · [(range/16°)·(GZmax/0.12 m)]^(1/4)`, capped to `[0, 1]`,
/// with the heel factor `K = (30° - θe)/(30° - 15°)` — 1 at or below
/// 15° of equilibrium heel, 0 at or above 30°.
///
/// `GZmax` (m) is the peak righting arm between the equilibrium heel
/// and the vanishing angle; `range` (degrees) is `θv - θe`.
pub fn s_factor_cargo(equilibrium_heel_deg: f64, gz_max_m: f64, range_deg: f64) -> f64 {
    const THETA_MIN_DEG: f64 = 15.0;
    const THETA_MAX_DEG: f64 = 30.0;
    const GZ_REF_M: f64 = 0.12;
    const RANGE_REF_DEG: f64 = 16.0;
    let heel = equilibrium_heel_deg.max(0.0);
    let k = if heel <= THETA_MIN_DEG {
        1.0
    } else if heel >= THETA_MAX_DEG {
        0.0
    } else {
        (THETA_MAX_DEG - heel) / (THETA_MAX_DEG - THETA_MIN_DEG)
    };
    let gz = gz_max_m.max(0.0);
    let range = range_deg.max(0.0);
    (k * (range / RANGE_REF_DEG * gz / GZ_REF_M).powf(0.25)).clamp(0.0, 1.0)
}

/// The required subdivision index `R` of Reg. 6 for cargo ships:
/// `R = 1 - 128/(Ls + 152)`. Only defined above 100 m subdivision
/// length (`None` below; the 80–100 m interpolation stays with the
/// class societies).
pub fn required_index_cargo(ls_m: f64) -> Option<f64> {
    if ls_m.is_finite() && ls_m > 100.0 {
        Some(1.0 - 128.0 / (ls_m + 152.0))
    } else {
        None
    }
}

/// One damage case's contribution to the attained subdivision index.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageCaseProbability {
    /// Case designation (e.g. "hold 3 alone").
    pub name: String,
    /// The longitudinal zone factor (Reg. 7-1), including any `r`/`v`
    /// reductions the caller applied (1.0 in this slice).
    pub p_factor: f64,
    /// The survival factor (Reg. 7-2).
    pub s_factor: f64,
}

/// The attained subdivision index `A = Σ p_i·s_i` (Reg. 7) for one
/// loading condition.
pub fn attained_subdivision_index(cases: &[DamageCaseProbability]) -> f64 {
    cases.iter().map(|c| c.p_factor * c.s_factor).sum()
}

/// The three-condition cargo weighting of Reg. 7.1:
/// `A = 0.4·A_deepest + 0.4·A_partial + 0.2·A_light`.
pub fn combined_cargo_index(deepest: f64, partial: f64, light: f64) -> f64 {
    0.4 * deepest + 0.4 * partial + 0.2 * light
}

/// The Reg. 7-2 survival inputs recovered from a damaged GZ scan.
#[derive(Debug, Clone, PartialEq)]
pub struct DamagedSurvivability {
    /// Equilibrium heel after flooding, degrees (0 for centred damage).
    pub equilibrium_heel_deg: f64,
    /// Peak righting arm between the equilibrium heel and the vanishing
    /// angle, m.
    pub gz_max_m: f64,
    /// Range of positive righting arm from the equilibrium heel,
    /// degrees.
    pub range_deg: f64,
    /// Heel at which the righting arm vanishes, degrees.
    pub vanishing_angle_deg: f64,
    /// The cargo-ship survival factor `s` (Reg. 7-2).
    pub s_factor: f64,
}

impl HullForm {
    /// Damaged survivability for a cargo ship (Reg. 7-2 inputs from the
    /// prismatic model): floods the compartments with
    /// [`HullForm::damage_stability`], then scans the wall-sided GZ
    /// curve at the damaged draft and effective KG (the free-surface
    /// correction folded in) for the peak arm and the vanishing angle.
    /// Centred damages sit at zero equilibrium heel, where this is the
    /// standard added-weight treatment; a transverse flood offset shows
    /// up as the small-angle list and the scan runs from there.
    ///
    /// # Errors
    ///
    /// [`ProbabilisticError::UnsolvedDamage`] when the damaged
    /// equilibrium cannot be solved, `InvalidSubdivisionLength`-style
    /// checks are left to the caller.
    pub fn damaged_survivability_cargo(
        &self,
        displacement_t: f64,
        lcg_from_midship_m: f64,
        kg_m: f64,
        free_surface_moment_tm: f64,
        compartments: &[DamageCompartment],
    ) -> Result<DamagedSurvivability, ProbabilisticError> {
        let damaged = self
            .damage_stability(
                displacement_t,
                lcg_from_midship_m,
                kg_m,
                free_surface_moment_tm,
                compartments,
            )
            .ok_or(ProbabilisticError::UnsolvedDamage)?;
        let hs = self.hydrostatics(damaged.mean_draft_m);
        // GM already carries the free-surface correction; the wall-sided
        // arm rebuilds from GM and BM at the damaged draft.
        let gm = damaged.gm_m;
        let bm = hs.km_m - hs.kb_m;
        let gz_at = |phi_deg: f64| {
            let phi = phi_deg.to_radians();
            let (sin, tan) = (phi.sin(), phi.tan());
            gm * sin + 0.5 * bm * tan * tan * sin
        };
        let theta_e = damaged.list_angle_deg.max(0.0);
        if gm <= 0.0 {
            return Ok(DamagedSurvivability {
                equilibrium_heel_deg: theta_e,
                gz_max_m: 0.0,
                range_deg: 0.0,
                vanishing_angle_deg: theta_e,
                s_factor: 0.0,
            });
        }
        // Scan in 0.25 deg steps to 90 deg for the peak and the vanish.
        let step = 0.25_f64;
        let (mut gz_max, mut theta_v) = (0.0_f64, 90.0_f64);
        let mut phi = theta_e;
        while phi <= 90.0 {
            let gz = gz_at(phi);
            if gz <= 0.0 && phi > theta_e {
                theta_v = phi;
                break;
            }
            if gz > gz_max {
                gz_max = gz;
            }
            phi += step;
        }
        let s = s_factor_cargo(theta_e, gz_max, theta_v - theta_e);
        Ok(DamagedSurvivability {
            equilibrium_heel_deg: theta_e,
            gz_max_m: gz_max,
            range_deg: theta_v - theta_e,
            vanishing_angle_deg: theta_v,
            s_factor: s,
        })
    }
}

/// The transverse penetration factor `r(x1, x2, b)` of Reg. 7-1.2: the
/// probability that a damage reaching the longitudinal barrier at
/// distance `b` (m off the shell, at the deepest subdivision draft)
/// occurs, within the zone `[x1, x2]`:
///
/// `r = 1 - (1 - C)·[1 - G / p(x1, x2)]`
///
/// with `C = 12·Jb·(4 - 45·Jb)`, `Jb = b/(15B)` (B = moulded breadth),
/// and `G` per the zone's terminal case — `G1 = b11·Jb²/2 + b12·Jb`
/// (whole length), `G2 = -b11·J0³/3 + (b11·J - b12)·J0²/2 + b12·J·J0`
/// with `J0 = min(J, Jb)` (neither limit at a terminal), or
/// `G = (G2 + G1·J)/2` (one limit at a terminal). Full penetration
/// (`b = B/2`) gives `C = 1` and `r = 1`; zero penetration gives
/// `r = 0`.
///
/// # Errors
///
/// [`ProbabilisticError::InvalidZone`] on bad zone bounds, plus the
/// [`p_factor`] validation for the `p(x1, x2)` inside the formula.
pub fn r_factor(
    density: &DamageLengthDensity,
    ls_m: f64,
    breadth_m: f64,
    x1_m: f64,
    x2_m: f64,
    penetration_b_m: f64,
) -> Result<f64, ProbabilisticError> {
    if !(breadth_m.is_finite() && breadth_m > 0.0)
        || !(penetration_b_m.is_finite() && penetration_b_m >= 0.0)
        || penetration_b_m > breadth_m / 2.0
    {
        return Err(ProbabilisticError::InvalidZone);
    }
    let p = p_factor(density, ls_m, x1_m, x2_m)?;
    if penetration_b_m <= 0.0 {
        return Ok(0.0);
    }
    let j = (x2_m - x1_m) / ls_m;
    let jn = j.min(density.jm);
    let jb = penetration_b_m / (15.0 * breadth_m);
    // C peaks at exactly 1 for full penetration (Jb = 1/30).
    let c = 12.0 * jb * (4.0 - 45.0 * jb);
    let g1 = density.b11 * jb * jb / 2.0 + density.b12 * jb;
    let g = if x1_m <= 0.0 && x2_m >= ls_m {
        g1
    } else {
        let j0 = jn.min(jb);
        let g2 = -density.b11 * j0 * j0 * j0 / 3.0
            + (density.b11 * jn - density.b12) * j0 * j0 / 2.0
            + density.b12 * jn * j0;
        if x1_m <= 0.0 || x2_m >= ls_m {
            (g2 + g1 * jn) / 2.0
        } else {
            g2
        }
    };
    let r = 1.0 - (1.0 - c) * (1.0 - g / p.max(1e-12));
    Ok(r.clamp(0.0, 1.0))
}

/// The longitudinal zone factor for a *group of adjacent zones*
/// (Reg. 7-1, the p·r combinations): the chain `[x1_j, x2_(j+n-1)]`
/// with the regulation's alternating form
///
/// - one zone: `p(x1, x2)·[r(b_k) - r(b_(k-1))]`,
/// - two zones: `p(both)·[...] - p(aft)·[...] - p(fwd)·[...]`,
/// - three or more: the four-term form
///   `p(all)·[...] - p(all but fore)·[...] - p(all but aft)·[...] +
///   p(intermediate)·[...]`,
///
/// where each bracket is the r-difference across the longitudinal
/// bulkhead pair `(b_outer, b_inner)` — `r(·, 0) = 0`, so a single
/// bulkhead at `b` reduces the factor by the wing-compartment share.
/// With `bulkhead_b_m = None` (pure longitudinal subdivision, no
/// barrier) every bracket is 1 and the formulas reduce to the printed
/// pure-p combinations.
///
/// `zones` are the adjacent `(aft_end, fore_end)` metre bounds in
/// order; gaps between zones are allowed (the p terms use the printed
/// endpoints, so a gap simply contributes its own span).
///
/// # Errors
///
/// [`ProbabilisticError::InvalidZone`] on bad bounds or an empty list.
pub fn multi_zone_p_factor(
    density: &DamageLengthDensity,
    ls_m: f64,
    breadth_m: f64,
    zones: &[(f64, f64)],
    bulkhead_b_m: Option<f64>,
) -> Result<f64, ProbabilisticError> {
    if zones.is_empty() {
        return Err(ProbabilisticError::InvalidZone);
    }
    // r(·, 0) = 0, so a single bulkhead's bracket is just r(·, b); no
    // barrier means every bracket is 1 (the pure-p combinations).
    let rdiff = |x1: f64, x2: f64| -> Result<f64, ProbabilisticError> {
        match bulkhead_b_m {
            None => Ok(1.0),
            Some(b) => r_factor(density, ls_m, breadth_m, x1, x2, b),
        }
    };
    let term = |a: f64, b: f64| -> Result<f64, ProbabilisticError> {
        Ok(p_factor(density, ls_m, a, b)? * rdiff(a, b)?)
    };
    let n = zones.len();
    let (aft, fore) = (zones[0].0, zones[n - 1].1);
    let total = term(aft, fore)?;
    match n {
        1 => Ok(total),
        2 => {
            let aft_only = term(zones[0].0, zones[0].1)?;
            let fwd_only = term(zones[1].0, zones[1].1)?;
            Ok(total - aft_only - fwd_only)
        }
        _ => {
            let drop_fore = term(zones[0].0, zones[n - 2].1)?;
            let drop_aft = term(zones[1].0, fore)?;
            let middle = term(zones[1].0, zones[n - 2].1)?;
            Ok(total - drop_fore - drop_aft + middle)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The penetration factor's boundary conditions from the
    /// regulation: r = 0 at zero penetration, r = 1 at B/2 (where
    /// C = 12*(1/30)*(4 - 45/30) = 1 exactly), monotone growth in
    /// between, and a mid-ship wing compartment at 0.1*B keeps most of
    /// the probability off the inner space.
    #[test]
    fn r_factor_boundary_conditions() {
        let d = DamageLengthDensity::for_subdivision_length(150.0).unwrap();
        // Zero penetration: r = 0 (r(x1, x2, b0) = 0).
        assert_eq!(r_factor(&d, 150.0, 22.0, 40.0, 60.0, 0.0).unwrap(), 0.0);
        // Full penetration: C = 1 exactly at Jb = 1/30 -> r = 1.
        assert_eq!(r_factor(&d, 150.0, 22.0, 40.0, 60.0, 11.0).unwrap(), 1.0);
        // Monotone in the penetration depth.
        let mut prev = 0.0;
        for i in 1..=20 {
            let b = 11.0 * i as f64 / 20.0;
            let r = r_factor(&d, 150.0, 22.0, 40.0, 60.0, b).unwrap();
            assert!(r >= prev - 1e-12, "r fell at b = {b}");
            assert!((0.0..=1.0).contains(&r));
            prev = r;
        }
        // A wing compartment at 0.1 B (2.2 m barrier): the inner space
        // still takes the majority share (the HARDER statistics give
        // shallow penetrations the bulk of the probability).
        let wing = r_factor(&d, 150.0, 22.0, 40.0, 60.0, 2.2).unwrap();
        assert!(wing < 0.9, "wing barrier share {wing}");
        assert!(wing > 0.0);
        // Penetration beyond B/2 is not physical.
        assert_eq!(
            r_factor(&d, 150.0, 22.0, 40.0, 60.0, 12.0),
            Err(ProbabilisticError::InvalidZone)
        );
    }

    /// The multi-zone combination: a two-zone group is the union minus
    /// both singles (each with its r bracket); a three-zone group takes
    /// the four-term form and stays below the union probability; no
    /// barrier reduces every bracket to 1 (the pure-p combinations).
    #[test]
    fn multi_zone_combinations_match_the_regulation_forms() {
        let d = DamageLengthDensity::for_subdivision_length(150.0).unwrap();
        // Two zones, no barrier: pj,2 = p(union) - p(aft) - p(fwd).
        let zones = [(30.0, 55.0), (55.0, 80.0)];
        let two = multi_zone_p_factor(&d, 150.0, 22.0, &zones, None).unwrap();
        let union = p_factor(&d, 150.0, 30.0, 80.0).unwrap();
        let aft = p_factor(&d, 150.0, 30.0, 55.0).unwrap();
        let fwd = p_factor(&d, 150.0, 55.0, 80.0).unwrap();
        assert!((two - (union - aft - fwd)).abs() < 1e-12);
        assert!(two > 0.0 && two < union);

        // Three zones, no barrier: the four-term form.
        let zones3 = [(30.0, 50.0), (50.0, 70.0), (70.0, 95.0)];
        let three = multi_zone_p_factor(&d, 150.0, 22.0, &zones3, None).unwrap();
        let full = p_factor(&d, 150.0, 30.0, 95.0).unwrap();
        let drop_fore = p_factor(&d, 150.0, 30.0, 70.0).unwrap();
        let drop_aft = p_factor(&d, 150.0, 50.0, 95.0).unwrap();
        let middle = p_factor(&d, 150.0, 50.0, 70.0).unwrap();
        assert!((three - (full - drop_fore - drop_aft + middle)).abs() < 1e-12);
        assert!(three > 0.0);

        // With a longitudinal bulkhead at 2 m, every zone group scores
        // less than its pure form (the wing share is taken out).
        let two_b = multi_zone_p_factor(&d, 150.0, 22.0, &zones, Some(2.0)).unwrap();
        assert!(two_b < two, "bulkhead must reduce the group factor");

        // Single zone passes through with the r bracket.
        let single = multi_zone_p_factor(&d, 150.0, 22.0, &[(30.0, 55.0)], Some(2.0)).unwrap();
        let p = p_factor(&d, 150.0, 30.0, 55.0).unwrap();
        let r = r_factor(&d, 150.0, 22.0, 30.0, 55.0, 2.0).unwrap();
        assert!((single - p * r).abs() < 1e-12);

        // Empty zone list is refused.
        assert_eq!(
            multi_zone_p_factor(&d, 150.0, 22.0, &[], None),
            Err(ProbabilisticError::InvalidZone)
        );
    }

    /// The reference coefficients for `Ls <= 198 m` in exact fractions:
    /// Jm = 10/33, Jk = 5/33 give b11 = -3267/50, b12 = 11, b21 = -363/50,
    /// b22 = 11/5 — hand-derived from the four density constraints.
    #[test]
    fn density_coefficients_match_hand_fractions() {
        let d = DamageLengthDensity::for_subdivision_length(150.0).unwrap();
        assert!((d.jm - 10.0 / 33.0).abs() < 1e-15);
        assert!((d.jk - 5.0 / 33.0).abs() < 1e-15);
        assert!((d.b11 - (-3267.0 / 50.0)).abs() < 1e-9);
        assert!((d.b12 - 11.0).abs() < 1e-9);
        assert!((d.b21 - (-363.0 / 50.0)).abs() < 1e-9);
        assert!((d.b22 - 11.0 / 5.0).abs() < 1e-9);
        // The density at J = 0 is the intercept b12 (11 per unit J).
        assert!((d.density(0.0) - 11.0).abs() < 1e-12);
    }

    /// The density integrates to pk over [0, Jk] and to 1 over [0, Jm],
    /// is continuous at the knuckle and vanishes at Jm — for both the
    /// short-ship and the rescaled above-L* case.
    #[test]
    fn density_satisfies_the_published_constraints() {
        for ls in [80.0_f64, 150.0, 250.0, 300.0, 400.0] {
            let d = DamageLengthDensity::for_subdivision_length(ls).unwrap();
            let n = 20_000;
            let integrate = |a: f64, b: f64, f: &dyn Fn(f64) -> f64| {
                let h = (b - a) / n as f64;
                let s: f64 = (0..=n)
                    .map(|i| {
                        let w = if i == 0 || i == n {
                            1.0
                        } else if i % 2 == 1 {
                            4.0
                        } else {
                            2.0
                        };
                        w * f(a + i as f64 * h)
                    })
                    .sum::<f64>()
                    * h
                    / 3.0;
                s
            };
            let head = integrate(0.0, d.jk, &|j| d.density(j));
            let total = integrate(0.0, d.jm, &|j| d.density(j));
            assert!(
                (head - P_K).abs() < 1e-6,
                "ls {ls}: head {head} vs pk {P_K}"
            );
            assert!((total - 1.0).abs() < 1e-6, "ls {ls}: total {total} vs 1");
            assert!(
                (d.density(d.jk) - (d.b12 + d.b11 * d.jk)).abs() < 1e-12,
                "continuity is built into the piecewise evaluation"
            );
            assert!(d.density(d.jm).abs() < 1e-9, "vanishes at Jm");
            assert!(d.jk < d.jm);
        }
    }

    /// `p` for a mid-ship zone equals the contained-damage integral
    /// `∫0^Jn b(j)(Jn - j) dj` evaluated numerically — verifying the
    /// printed p1 form (J <= Jk) and the p2 continuation (J > Jk)
    /// against an independent numerical path.
    #[test]
    fn p_factor_matches_the_contained_damage_integral() {
        let d = DamageLengthDensity::for_subdivision_length(150.0).unwrap();
        let integral = |jn: f64| {
            let n = 4_000;
            let h = jn / n as f64;
            // ∫0^jn b(j) (jn - j) dj by Simpson on the full integrand.
            let f = |j: f64| d.density(j) * (jn - j);
            (0..=n)
                .map(|i| {
                    let w = if i == 0 || i == n {
                        1.0
                    } else if i % 2 == 1 {
                        4.0
                    } else {
                        2.0
                    };
                    w * f(i as f64 * h)
                })
                .sum::<f64>()
                * h
                / 3.0
        };
        for zone_len in [0.02_f64, 0.05, d.jk, 0.20, 0.25, d.jm] {
            let x1 = 0.4; // strictly inside (0, 1)
            let x2 = x1 + zone_len;
            let p = p_factor(&d, 150.0, x1 * 150.0, x2 * 150.0).unwrap();
            let reference = integral(zone_len.min(d.jm));
            assert!(
                (p - reference).abs() < 1e-6,
                "zone {zone_len:.3}: p {p} vs integral {reference}"
            );
        }
        // Continuity of p at the knuckle.
        let below = p_factor(&d, 150.0, 40.0, 40.0 + d.jk * 150.0 - 1e-9).unwrap();
        let above = p_factor(&d, 150.0, 40.0, 40.0 + d.jk * 150.0 + 1e-9).unwrap();
        assert!((below - above).abs() < 1e-6);
    }

    /// The p2 continuation has the cumulative density as its derivative
    /// (finite difference) and saturates at `p(Jm) = Jm - E[J]`, checked
    /// against a Simpson mean of the density.
    #[test]
    fn p2_continuation_is_the_exact_antiderivative() {
        let d = DamageLengthDensity::for_subdivision_length(150.0).unwrap();
        let p = |jn: f64| p_factor(&d, 150.0, 30.0, 30.0 + jn * 150.0).unwrap();
        // d/dJn p = cumulative(Jn) — central difference at mid points.
        for jn in [0.18_f64, 0.22, 0.28] {
            let h = 1e-6;
            let slope = (p(jn + h) - p(jn - h)) / (2.0 * h);
            let c = d.cumulative(jn);
            assert!(
                (slope - c).abs() < 1e-5,
                "jn {jn}: slope {slope} vs cumulative {c}"
            );
        }
        // p(Jm) = Jm - E[J] with E[J] by Simpson.
        let n = 20_000;
        let h = d.jm / n as f64;
        let e_j = (0..=n)
            .map(|i| {
                let w = if i == 0 || i == n {
                    1.0
                } else if i % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                let j = i as f64 * h;
                w * j * d.density(j)
            })
            .sum::<f64>()
            * h
            / 3.0;
        let at_jm = p(d.jm);
        assert!(
            (at_jm - (d.jm - e_j)).abs() < 1e-6,
            "p(Jm) {at_jm} vs Jm - E[J] {}",
            d.jm - e_j
        );
    }

    /// Terminal zones take `(p_i + J)/2` and the whole length is 1; p is
    /// monotone in zone width and never exceeds 1.
    #[test]
    fn terminal_and_whole_length_cases() {
        let d = DamageLengthDensity::for_subdivision_length(150.0).unwrap();
        // Whole length.
        assert_eq!(p_factor(&d, 150.0, 0.0, 150.0).unwrap(), 1.0);
        // Aft-terminal zone: (p_mid + J)/2 with p_mid at the same width.
        let j = 0.10_f64;
        let term = p_factor(&d, 150.0, 0.0, j * 150.0).unwrap();
        let mid = p_factor(&d, 150.0, 75.0 - j * 75.0, 75.0 + j * 75.0).unwrap();
        assert!((term - 0.5 * (mid + j)).abs() < 1e-12);
        assert!(term > mid, "terminal truncation catches more probability");
        // Monotone in width, bounded by 1.
        let mut prev = 0.0;
        for w in [0.02_f64, 0.06, 0.12, 0.2, 0.3, 0.5] {
            let p = p_factor(&d, 150.0, 0.0, w * 150.0).unwrap();
            assert!(p > prev, "width {w}: {p} after {prev}");
            assert!(p <= 1.0);
            prev = p;
        }
        // Invalid zones are rejected.
        assert_eq!(
            p_factor(&d, 150.0, 50.0, 50.0),
            Err(ProbabilisticError::InvalidZone)
        );
        assert_eq!(
            p_factor(&d, 150.0, -1.0, 50.0),
            Err(ProbabilisticError::InvalidZone)
        );
        assert_eq!(
            p_factor(&d, 150.0, 100.0, 200.0),
            Err(ProbabilisticError::InvalidZone)
        );
    }

    /// Hand values of the cargo s-factor: the reference point (16 deg
    /// range, 0.12 m arm) scores 1, the heel gate kills s at 30 deg and
    /// scales linearly, and the fourth-root scaling is exact.
    #[test]
    fn s_factor_matches_hand_values() {
        // Reference: K = 1, bracket = 1 -> s = 1.
        assert!((s_factor_cargo(0.0, 0.12, 16.0) - 1.0).abs() < 1e-12);
        // Clamped at 1 beyond the reference.
        assert_eq!(s_factor_cargo(0.0, 0.30, 40.0), 1.0);
        // At the 30 deg heel gate s = 0; at 15 deg it is full.
        assert_eq!(s_factor_cargo(30.0, 0.12, 16.0), 0.0);
        assert!((s_factor_cargo(15.0, 0.12, 16.0) - 1.0).abs() < 1e-12);
        // Linear K at 22.5 deg: K = (30 - 22.5)/15 = 0.5; bracket
        // (8/16 * 0.06/0.12)^(1/4) = 0.25^0.25 = 0.7071 -> 0.35355.
        assert!((s_factor_cargo(22.5, 0.06, 8.0) - 0.5 * 0.25_f64.powf(0.25)).abs() < 1e-12);
        // K = 1, bracket (4/16)(0.03/0.12) = 0.0625, fourth root = 0.5.
        assert!((s_factor_cargo(0.0, 0.03, 4.0) - 0.5).abs() < 1e-12);
    }

    /// The required cargo index: R = 1 - 128/(Ls + 152) above 100 m,
    /// nothing below (the 80-100 m interpolation is out of scope).
    #[test]
    fn required_index_follows_regulation_6() {
        let r = required_index_cargo(150.0).unwrap();
        assert!((r - (1.0 - 128.0 / 302.0)).abs() < 1e-15);
        let r = required_index_cargo(101.0).unwrap();
        assert!((r - (1.0 - 128.0 / 253.0)).abs() < 1e-15);
        assert_eq!(required_index_cargo(100.0), None);
        assert_eq!(required_index_cargo(80.0), None);
        // R grows with length and stays in (0, 1).
        assert!(required_index_cargo(300.0).unwrap() > r);
        assert!(required_index_cargo(350.0).unwrap() < 1.0);
    }

    /// The attained index sums p*s and the three-condition weighting is
    /// 0.4/0.4/0.2.
    #[test]
    fn attained_index_sums_and_weights() {
        let cases = vec![
            DamageCaseProbability {
                name: "hold 2".into(),
                p_factor: 0.10,
                s_factor: 1.0,
            },
            DamageCaseProbability {
                name: "hold 3".into(),
                p_factor: 0.12,
                s_factor: 0.5,
            },
        ];
        let a = attained_subdivision_index(&cases);
        assert!((a - (0.10 + 0.06)).abs() < 1e-12);
        let combined = combined_cargo_index(0.5, 0.4, 0.3);
        assert!((combined - (0.2 + 0.16 + 0.06)).abs() < 1e-12);
    }

    /// End-to-end: a boxy cargo hull floods a midship hold — zero
    /// equilibrium heel, positive range and arm, s in (0, 1]; a
    /// catastrophic flood that sinks the GM scores s = 0.
    #[test]
    fn damaged_survivability_end_to_end() {
        let hull = HullForm {
            loa_m: 140.0,
            boa_m: 22.0,
            cb: 0.72,
            cwp: 0.85,
        };
        // A modest double-bottom flood keeps ample damaged GM.
        let db = DamageCompartment {
            name: "DB hold 3".into(),
            volume_m3: 300.0,
            centroid: (10.0, 0.0, 1.0),
            free_surface_moment_tm: 120.0,
        };
        let surv = hull
            .damaged_survivability_cargo(30_000.0, 0.0, 9.0, 200.0, &[db])
            .unwrap();
        assert!(surv.equilibrium_heel_deg.abs() < 1e-12, "centred flood");
        assert!(surv.gz_max_m > 0.0);
        assert!(surv.range_deg > 0.0);
        assert!(surv.s_factor > 0.0 && surv.s_factor <= 1.0);
        // A free-surface moment large enough to sink the damaged GM: s = 0.
        let huge = DamageCompartment {
            name: "ER full".into(),
            volume_m3: 12_000.0,
            centroid: (-30.0, 0.0, 6.0),
            free_surface_moment_tm: 500_000.0,
        };
        let sunk = hull
            .damaged_survivability_cargo(30_000.0, 0.0, 9.0, 200.0, &[huge])
            .unwrap();
        assert_eq!(sunk.s_factor, 0.0);
    }
}
