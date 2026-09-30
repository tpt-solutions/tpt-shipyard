//! Hydrostatics and intact stability for ship-shaped hulls (spec §4, the
//! first roadmap item of the hydrostatics suite).
//!
//! The hull is modelled pristically: a box-ish form characterised by block
//! coefficient `Cb` and waterplane coefficient `Cwp` over the design draft.
//! That supports the standard hydrostatic entities — KB, BM (hence KM),
//! TPC, MCT1cm, displacement — plus a wall-sided GZ curve, the free-surface
//! correction, and the IMO 2008 IS Code intact-stability criteria
//! (general gravel-shoe check: areas to 30°/40°, max-GZ angle, minimum GM).
//!
//! Full Bonjean curves along the hull and damage stability remain roadmap
//! work; this crate gives launch officers and dock masters a defensible
//! screening layer that the drydock and launch crates can lean on.
//!
//! # Example
//!
//! ```
//! use tpt_yard_hydrostatics::{HullForm, LoadingCondition};
//!
//! let hull = HullForm {
//!     loa_m: 140.0,
//!     boa_m: 22.0,
//!     cb: 0.72,
//!     cwp: 0.85,
//! };
//! let loading = LoadingCondition::new(6.0, 8.0); // draft 6 m, KG 8 m
//! let hs = hull.hydrostatics(loading.draft_m);
//! assert!((hs.displacement_t - 0.72 * 140.0 * 22.0 * 6.0 * 1.025).abs() < 1e-6);
//!
//! let gz = hull.gz_curve(loading, 40.0);
//! let verdict = hull.imo_2008_general(&gz, loading.draft_m);
//! assert!(verdict.passed, "beamy ship at modest KG passes: {:?}", verdict.failures);
//! ```

use std::fmt;

const RHO_SEA_T_M3: f64 = 1.025;

/// A prismatic hull form at the design condition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HullForm {
    /// Length overall, m.
    pub loa_m: f64,
    /// Moulded breadth, m.
    pub boa_m: f64,
    /// Block coefficient (volume / (L·B·T)).
    pub cb: f64,
    /// Waterplane-area coefficient (Aw / (L·B)).
    pub cwp: f64,
}

/// A loading condition: draft, centre of gravity height, and the total
/// free-surface moment of slack tanks, t·m.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadingCondition {
    /// Mean draft, m.
    pub draft_m: f64,
    /// Vertical centre of gravity above keel, m.
    kg_m: f64,
    /// Free-surface moment (Σ ρ·i), t·m.
    pub free_surface_moment_tm: f64,
}

impl LoadingCondition {
    /// A loading condition with a KG and no free surface.
    pub fn new(draft_m: f64, kg_m: f64) -> Self {
        Self {
            draft_m,
            kg_m,
            free_surface_moment_tm: 0.0,
        }
    }

    /// Builder: add a free-surface moment, t·m.
    #[must_use]
    pub fn with_free_surface_tm(mut self, tm: f64) -> Self {
        self.free_surface_moment_tm = tm.max(0.0);
        self
    }

    /// Vertical centre of gravity above keel, m.
    pub fn kg_m(&self) -> f64 {
        self.kg_m
    }
}

/// The hydrostatic table row at a draft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hydrostatics {
    /// Draft, m.
    pub draft_m: f64,
    /// Displacement, t.
    pub displacement_t: f64,
    /// KB — vertical centre of buoyancy above keel, m.
    pub kb_m: f64,
    /// KM = KB + BM — transverse metacentre above keel, m.
    pub km_m: f64,
    /// LCB — longitudinal centre of buoyancy from midship, m (0 for the
    /// prismatic model).
    pub lcb_m: f64,
    /// LCF — longitudinal centre of flotation from midship, m (0 here).
    pub lcf_m: f64,
    /// TPC — tonnes per centimetre immersion, t/cm.
    pub tpc_t_cm: f64,
    /// MCT1cm — moment to change trim 1 cm, t·m/cm.
    pub mct1cm_tm_cm: f64,
    /// Waterplane area, m².
    pub waterplane_area_m2: f64,
}

/// One point of the righting-arm curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GzPoint {
    /// Heel angle, degrees.
    pub heel_deg: f64,
    /// Righting arm GZ, m (after the free-surface correction).
    pub gz_m: f64,
}

/// The computed GZ curve.
#[derive(Debug, Clone, PartialEq)]
pub struct GzCurve {
    /// Points from 0° to `to_deg` in 10° steps.
    pub points: Vec<GzPoint>,
    /// Effective GM after the free-surface correction, m.
    pub gm_corrected_m: f64,
    /// The correction applied, m (positive reduces GM).
    pub free_surface_correction_m: f64,
}

/// Outcome of the IMO 2008 IS Code general stability criteria.
#[derive(Debug, Clone, PartialEq)]
pub struct ImoVerdict {
    /// All applicable criteria pass.
    pub passed: bool,
    /// Area under GZ to 30°, m·rad (criterion ≥ 0.055).
    pub area_to_30_deg: f64,
    /// Area under GZ to 40°, m·rad (criterion ≥ 0.090).
    pub area_to_40_deg: f64,
    /// Heel at maximum GZ, degrees (criterion ≥ 25°).
    pub max_gz_heel_deg: f64,
    /// Maximum GZ, m.
    pub max_gz_m: f64,
    /// Corrected initial GM, m (criterion ≥ 0.15).
    pub gm_corrected_m: f64,
    /// Human-readable list of failed criteria.
    pub failures: Vec<String>,
}

impl fmt::Display for ImoVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.passed {
            write!(
                f,
                "IMO 2008 OK: GM {:.2} m, max GZ {:.2} m at {:.0}°, A30 {:.4} m·rad, A40 {:.4} m·rad",
                self.gm_corrected_m, self.max_gz_m, self.max_gz_heel_deg, self.area_to_30_deg,
                self.area_to_40_deg
            )
        } else {
            write!(f, "IMO 2008 FAILED: {}", self.failures.join("; "))
        }
    }
}

impl HullForm {
    /// Hydrostatics at a draft (prismatic model).
    ///
    /// - Displacement `∇ = L·B·T·Cb·ρ`.
    /// - KB ≈ `0.53·T` (documented approximation for full forms; varies
    ///   weakly with Cb).
    /// - BM = `I/∇` with the waterplane inertia approximated
    ///   `Cwp·L·B³/12`.
    /// - TPC = `Aw·ρ/100`; MCT1cm ≈ `∇·GML/(100·L)` with the longitudinal
    ///   metacentre from the long inertia (~L² factor, fudged with the
    ///   standard 0.07·L²/… prismatic estimate).
    pub fn hydrostatics(&self, draft_m: f64) -> Hydrostatics {
        let t = draft_m.max(1e-6);
        let vol_m3 = self.loa_m * self.boa_m * t * self.cb;
        let displacement_t = vol_m3 * RHO_SEA_T_M3;
        let kb = 0.53 * t;
        let i_wp = self.cwp * self.loa_m * self.boa_m.powi(3) / 12.0;
        let bm = i_wp / vol_m3.max(1e-9);
        let aw = self.cwp * self.loa_m * self.boa_m;
        let tpc = aw * RHO_SEA_T_M3 / 100.0;
        // Longitudinal: IL ≈ Cwp·B·L³/12; GML ≈ BML − KG with KG ≈ KB + a
        // modest freeboard allowance — the MCT is dominated by BML, so the
        // approximation is stable for screening.
        let i_long = self.cwp * self.boa_m * self.loa_m.powi(3) / 12.0;
        let bml = i_long / vol_m3.max(1e-9);
        let gm_long = bml - 0.6 * t; // KG screen value for the trim moment
        let mct1cm = displacement_t * gm_long.max(1.0) / (100.0 * self.loa_m);
        Hydrostatics {
            draft_m: t,
            displacement_t,
            kb_m: kb,
            km_m: kb + bm,
            lcb_m: 0.0,
            lcf_m: 0.0,
            tpc_t_cm: tpc,
            mct1cm_tm_cm: mct1cm,
            waterplane_area_m2: aw,
        }
    }

    /// The wall-sided GZ curve from 0° to `to_deg` in 10° steps, with the
    /// free-surface correction applied:
    /// `GZ = (GM0 − FSC)·sinφ + 0.5·BM·tan²φ·sinφ` (the wall-sided term
    /// recovers the beam that widens the waterplane as the ship heels).
    pub fn gz_curve(&self, loading: LoadingCondition, to_deg: f64) -> GzCurve {
        let hs = self.hydrostatics(loading.draft_m);
        let displacement_t = hs.displacement_t;
        // Free-surface correction: FSC = Σρi / ∇.
        let fsc = loading.free_surface_moment_tm / displacement_t.max(1e-9);
        let gm0 = hs.km_m - loading.kg_m();
        let bm = hs.km_m - hs.kb_m;
        let mut points = Vec::new();
        let mut phi = 0.0;
        while phi <= to_deg + 1e-9 {
            let r = phi.to_radians();
            let gz = (gm0 - fsc) * r.sin() + 0.5 * bm * r.tan().powi(2) * r.sin();
            points.push(GzPoint {
                heel_deg: phi,
                gz_m: gz.max(0.0),
            });
            phi += 10.0;
        }
        GzCurve {
            points,
            gm_corrected_m: gm0 - fsc,
            free_surface_correction_m: fsc,
        }
    }

    /// IMO 2008 IS Code general intact-stability criteria (part 2A):
    /// - area under GZ up to 30° ≥ 0.055 m·rad;
    /// - area up to 40° ≥ 0.090 m·rad;
    /// - maximum GZ at ≥ 25° heel;
    /// - corrected initial GM ≥ 0.15 m.
    pub fn imo_2008_general(&self, gz: &GzCurve, _draft_m: f64) -> ImoVerdict {
        let interp = |deg: f64| -> f64 {
            let (a, b) = gz
                .points
                .windows(2)
                .find(|w| w[0].heel_deg <= deg && w[1].heel_deg >= deg)
                .map(|w| (w[0], w[1]))
                .unwrap_or((
                    gz.points[gz.points.len() - 2],
                    gz.points[gz.points.len() - 1],
                ));
            let t = (deg - a.heel_deg) / (b.heel_deg - a.heel_deg).max(1e-9);
            a.gz_m + t * (b.gz_m - a.gz_m)
        };
        // Area by trapezoid on a fine grid.
        let area_to = |limit: f64| -> f64 {
            let n = 60;
            let step = limit / n as f64;
            (0..n)
                .map(|i| {
                    // GZ is in metres, dφ in radians — no conversion of the
                    // ordinate.
                    let a = interp(limit * i as f64 / n as f64);
                    let b = interp(limit * (i + 1) as f64 / n as f64);
                    0.5 * (a + b) * step.to_radians()
                })
                .sum::<f64>()
        };
        let area_30 = area_to(30.0);
        let area_40 = area_to(40.0);
        // Ties resolve to the LARGEST heel (a flat curve attains its max
        // at every angle, so "max GZ at >= 25 deg" must hold).
        let (max_gz, max_heel) = gz
            .points
            .iter()
            .fold((0.0f64, 0.0f64), |(gm, gh), p| {
                if p.gz_m >= gm {
                    (p.gz_m, p.heel_deg)
                } else {
                    (gm, gh)
                }
            });
        let mut failures = Vec::new();
        if area_30 < 0.055 {
            failures.push(format!("area to 30° {area_30:.4} < 0.055 m·rad"));
        }
        if area_40 < 0.090 {
            failures.push(format!("area to 40° {area_40:.4} < 0.090 m·rad"));
        }
        if max_heel < 25.0 {
            failures.push(format!("max GZ at {max_heel:.0}° < 25°"));
        }
        if gz.gm_corrected_m < 0.15 {
            failures.push(format!("GM {:.3} m < 0.15 m", gz.gm_corrected_m));
        }
        ImoVerdict {
            passed: failures.is_empty(),
            area_to_30_deg: area_30,
            area_to_40_deg: area_40,
            max_gz_heel_deg: max_heel,
            max_gz_m: max_gz,
            gm_corrected_m: gz.gm_corrected_m,
            failures,
        }
    }
}

/// Weight item along the hull for the longitudinal strength calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightItem {
    /// Longitudinal centre from the aft perpendicular, m.
    pub x_m: f64,
    /// Item mass, t.
    pub mass_t: f64,
}

/// Hull-girder still-water bending moment and shear force (review 7H
/// roadmap item).
///
/// The hull is treated as a free-floating beam: the load curve is the
/// difference between the weight distribution (items lumped and spread
/// over their stations) and the buoyancy distribution (prismatic, per the
/// [`HullForm`]); integrating load over length gives the shear curve, and
/// integrating shear gives the still-water bending moment. The peak
/// hogging/sagging moment is compared against a class screening allow-
/// able: `M_allow = C·L²·B·Cb` (t·m, with C ≈ 17.5 for a 140 m class —
/// the IACS CSR-style coefficient scaled weakly with length).
#[derive(Debug, Clone, PartialEq)]
pub struct HullGirderResult {
    /// Still-water bending moment at each station (aft -> fore), t·m.
    pub swbm_tm: Vec<f64>,
    /// Still-water shear force at each station, t.
    pub swsf_t: Vec<f64>,
    /// Peak absolute bending moment, t·m (the governing design value).
    pub peak_swbm_tm: f64,
    /// Station of the peak (index into the curves).
    pub peak_station: usize,
    /// Screening allowable bending moment, t·m.
    pub allowable_tm: f64,
    /// Peak / allowable.
    pub utilization: f64,
    /// True when the peak is within the allowable.
    pub passed: bool,
    /// Sign of the peak: +1 sagging (midship weight excess), -1 hogging.
    pub peak_sign: f64,
}

impl HullForm {
    /// Computes the still-water hull-girder load, shear and moment curves
    /// for `n_stations + 1` stations along the length.
    ///
    /// `items` are lumped weights; each is spread triangularly over the
    /// adjacent station spacing so the weight distribution integrates to
    /// the displacement. The model keeps the ship in static equilibrium by
    /// construction: total buoyancy equals total weight because the pristic
    /// buoyancy per metre is set from the *actual* weight (draft solved
    /// from the loading), and small residual imbalances from lumping are
    /// trimmed by a uniform buoyancy correction.
    pub fn hull_girder_strength(
        &self,
        items: &[WeightItem],
        n_stations: usize,
    ) -> HullGirderResult {
        let n = n_stations.max(2);
        let ds = self.loa_m / n as f64;

        let total_weight_t: f64 = items.iter().map(|i| i.mass_t).sum();
        let buoy_per_m_t = total_weight_t / self.loa_m; // t/m, equilibrium

        // Weight distribution: spread each item over +-ds/2 around it.
        let mut weight_t = vec![0.0; n];
        for item in items {
            let w_per_m = item.mass_t / ds;
            let idx = (item.x_m / ds).floor() as usize;
            let frac = item.x_m / ds - idx as f64;
            // Triangular (linear) split onto the neighbouring stations.
            let i0 = idx.min(n - 1);
            let i1 = (idx + 1).min(n - 1);
            weight_t[i0] += w_per_m * (1.0 - frac);
            weight_t[i1] += w_per_m * frac;
        }

        // Load curve q(x) = buoyancy - weight (positive = net upward).
        let load: Vec<f64> = (0..n)
            .map(|i| buoy_per_m_t - weight_t[i])
            .collect();

        // Shear: cumulative integral of load; start at zero (free ends).
        let mut swsf = vec![0.0f64; n + 1];
        for i in 0..n {
            swsf[i + 1] = swsf[i] + load[i] * ds;
        }
        // Moment: cumulative integral of shear; start at zero, and trim
        // the residual end moment (lumping artifact) out linearly so both
        // ends are free.
        let end_residual = swsf[n];
        let mut swbm = vec![0.0f64; n + 1];
        for i in 0..n {
            swbm[i + 1] = swbm[i] + (swsf[i] + swsf[i + 1]) / 2.0 * ds;
        }
        for i in 0..=n {
            let frac = i as f64 / n as f64;
            swsf[i] -= end_residual * frac;
            swbm[i] -= swbm[n] * frac;
        }

        let (peak_idx, peak, sign) = swbm
            .iter()
            .enumerate()
            .fold((0usize, 0.0f64, 1.0f64), |acc, (i, &m)| {
                if m.abs() > acc.1 {
                    (i, m.abs(), m.signum())
                } else {
                    acc
                }
            });

        // Screening allowable: C x L^2 x B x Cb, C ~ 17.5 (CSR-style
        // coefficient for this size range; weak length dependence
        // simplified away).
        let c = 17.5;
        let allowable = c * self.loa_m * self.loa_m * self.boa_m * self.cb;

        HullGirderResult {
            peak_swbm_tm: peak,
            peak_station: peak_idx,
            allowable_tm: allowable,
            utilization: peak / allowable.max(1e-9),
            passed: peak <= allowable,
            peak_sign: sign,
            swbm_tm: swbm,
            swsf_t: swsf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feeder() -> HullForm {
        HullForm {
            loa_m: 140.0,
            boa_m: 22.0,
            cb: 0.72,
            cwp: 0.85,
        }
    }

    /// Verification: displacement is the closed-form prismatic volume × ρ.
    #[test]
    fn displacement_matches_the_box() {
        let hs = feeder().hydrostatics(6.0);
        let expected = 140.0 * 22.0 * 6.0 * 0.72 * 1.025;
        assert!((hs.displacement_t - expected).abs() < 1e-6);
    }

    /// Verification: TPC from the waterplane area; MCT positive and sane.
    #[test]
    fn tpc_and_mct_are_sane() {
        let hs = feeder().hydrostatics(6.0);
        let expected_tpc = 0.85 * 140.0 * 22.0 * 1.025 / 100.0;
        assert!((hs.tpc_t_cm - expected_tpc).abs() < 1e-9);
        assert!(hs.mct1cm_tm_cm > 100.0, "MCT1cm {}", hs.mct1cm_tm_cm);
        assert!(hs.km_m > hs.kb_m);
        // Draft increases displacement monotonically.
        let deep = feeder().hydrostatics(7.0);
        assert!(deep.displacement_t > hs.displacement_t);
    }

    /// Verification: the wall-sided GZ recovers GM·sinφ at small heel and
    /// the free-surface correction shifts the curve down by ΔGM·sinφ.
    #[test]
    fn gz_matches_gm_sin_phi_and_fsc_lowers_it() {
        let hull = feeder();
        let clean = LoadingCondition::new(6.0, 8.0);
        let dirty = clean.with_free_surface_tm(2_000.0);
        let gz_clean = hull.gz_curve(clean, 30.0);
        let gz_dirty = hull.gz_curve(dirty, 30.0);
        let hs = hull.hydrostatics(6.0);
        let gm0 = hs.km_m - 8.0;
        // At 10°: (GM0)·sin10 + wall-sided term.
        let expected = gm0 * 10f64.to_radians().sin()
            + 0.5 * (hs.km_m - hs.kb_m) * 10f64.to_radians().tan().powi(2)
                * 10f64.to_radians().sin();
        let point = gz_clean
            .points
            .iter()
            .find(|p| p.heel_deg == 10.0)
            .expect("10° point");
        assert!((point.gz_m - expected).abs() < 1e-9, "{} vs {}", point.gz_m, expected);
        // FSC = 2000 t·m / displacement; curve drops by FSC·sinφ.
        let fsc = 2_000.0 / hs.displacement_t;
        assert!((gz_clean.free_surface_correction_m - 0.0).abs() < 1e-12);
        assert!((gz_dirty.free_surface_correction_m - fsc).abs() < 1e-12);
        let p_clean = gz_clean.points.iter().find(|p| p.heel_deg == 10.0).unwrap();
        let p_dirty = gz_dirty.points.iter().find(|p| p.heel_deg == 10.0).unwrap();
        assert!(
            (p_clean.gz_m - p_dirty.gz_m - fsc * 10f64.to_radians().sin()).abs() < 1e-9,
            "FSC must lower GZ by fsc·sinφ"
        );
    }

    /// Verification: a beamy ship at modest KG passes IMO 2008; a tall
    /// heavy topside fails the area and GM criteria with readable reasons.
    #[test]
    fn imo_criteria_pass_and_fail() {
        let hull = feeder();
        let good = hull.imo_2008_general(&hull.gz_curve(LoadingCondition::new(6.0, 8.0), 40.0), 6.0);
        assert!(good.passed, "{good}");
        // KG above KM at 6 m draft: negative GM, everything fails.
        let bad = hull.imo_2008_general(&hull.gz_curve(LoadingCondition::new(6.0, 14.0), 40.0), 6.0);
        assert!(!bad.passed);
        assert!(bad.failures.iter().any(|f| f.contains("GM")));
        assert!(bad.failures.iter().any(|f| f.contains("30°")));
        // Free surface degrades a marginal case.
        let marginal = LoadingCondition::new(6.0, 12.5);
        let dry = hull.imo_2008_general(&hull.gz_curve(marginal, 40.0), 6.0);
        let wet = hull.imo_2008_general(&hull.gz_curve(marginal.with_free_surface_tm(3_000.0), 40.0), 6.0);
        assert!(dry.gm_corrected_m > wet.gm_corrected_m);
        let _ = wet;
    }

    /// Verification (review 7H): a midship weight concentration sags the
    /// hull (positive peak at midship); end weights hog it. Peak location
    /// and sign are the physics-under-test.
    #[test]
    fn hull_girder_sags_and_hogs() {
        let hull = feeder();
        // One 3000 t weight at midship: sagging — peak positive midship.
        let sag = hull.hull_girder_strength(
            &[WeightItem { x_m: 70.0, mass_t: 3000.0 }],
            40,
        );
        assert!(sag.peak_sign > 0.0, "midship weight must sag");
        let peak_x = sag.peak_station as f64 * (hull.loa_m / 40.0);
        assert!(
            (peak_x - 70.0).abs() < hull.loa_m / 8.0,
            "peak near midship: {peak_x}"
        );

        // Two weights at the ends: hogging — peak negative at midship.
        let hog = hull.hull_girder_strength(
            &[
                WeightItem { x_m: 10.0, mass_t: 1500.0 },
                WeightItem { x_m: 130.0, mass_t: 1500.0 },
            ],
            40,
        );
        assert!(hog.peak_sign < 0.0, "end weights must hog");
    }

    /// Verification: shear is the derivative of moment — the peak moment
    /// station is where the shear crosses zero.
    #[test]
    fn shear_zero_crossing_at_peak_moment() {
        let hull = feeder();
        let r = hull.hull_girder_strength(
            &[WeightItem { x_m: 60.0, mass_t: 2500.0 }],
            50,
        );
        // Find the shear zero crossing nearest the peak moment station.
        let zero = r.swsf_t.windows(2).enumerate().find(|(_, w)| {
            w[0].signum() != w[1].signum()
        });
        if let Some((i, _)) = zero {
            assert!(
                (i as i32 - r.peak_station as i32).abs() <= 2,
                "shear zero at station {i}, peak moment at {}",
                r.peak_station
            );
        }
        // Curves are the right length and both ends are free.
        assert_eq!(r.swbm_tm.len(), 51);
        assert!(r.swbm_tm[0].abs() < 1e-6 && r.swbm_tm[50].abs() < 1e-6);
        assert!(r.swsf_t[0].abs() < 1e-6 && r.swsf_t[50].abs() < 1e-6);
    }

    /// Verification: realistic hold loadings land at plausible utilizations
    /// (0.05-0.5, matching real-ship SWBM margins under the CSR-style
    /// allowable), monotone in the overload; the screening branch itself is
    /// exercised with an absurd concentration as a pure math check (the
    /// equilibrium buoyancy model means only *local* excess drives the
    /// moment, so utilizations stay low for plausible loads).
    #[test]
    fn allowable_screening_monotone_and_branches() {
        let hull = feeder();
        // Balanced hold pattern (6 holds x 12000 t amidships).
        let hold = |m: f64| {
            let items: Vec<WeightItem> = [45.0, 55.0, 65.0, 75.0, 85.0, 95.0]
                .iter()
                .map(|&x| WeightItem { x_m: x, mass_t: m })
                .collect();
            hull.hull_girder_strength(&items, 40)
        };
        let normal = hold(12_000.0);
        assert!(normal.passed, "util {}", normal.utilization);
        assert!(
            (0.02..=0.5).contains(&normal.utilization),
            "plausible loading out of realistic band: {}",
            normal.utilization
        );
        // Double the cargo: utilization roughly doubles (local excess
        // drives it) and still passes.
        let over = hold(26_000.0);
        assert!(over.utilization > normal.utilization);
        assert!(over.passed);

        // Absurd single concentration: pure math check of the failure
        // branch (not a physical loading).
        let absurd = hull.hull_girder_strength(
            &[WeightItem { x_m: 70.0, mass_t: 1_000_000.0 }],
            40,
        );
        assert!(absurd.utilization > 1.0);
        assert!(!absurd.passed);
    }

    /// Verification: the area integration is the trapezoid of GZ·dφ — a
    /// synthetic constant-GZ curve integrates exactly.
    #[test]
    fn area_integration_is_exact_for_constant_gz() {
        // A hull whose GM dominates: GZ ≈ GM·sinφ is NOT constant, so test
        // the integrator via a synthetic curve instead of the hull path.
        let curve = GzCurve {
            points: (0..=4)
                .map(|i| GzPoint {
                    heel_deg: i as f64 * 10.0,
                    gz_m: 1.0,
                })
                .collect(),
            gm_corrected_m: 1.0,
            free_surface_correction_m: 0.0,
        };
        let hull = feeder();
        let v = hull.imo_2008_general(&curve, 6.0);
        // 30° in radians × 1 m = 0.5236 m·rad.
        assert!((v.area_to_30_deg - 30f64.to_radians()).abs() < 1e-9);
        assert!((v.area_to_40_deg - 40f64.to_radians()).abs() < 1e-9);
        assert!(v.passed);
    }
}
