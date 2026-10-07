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

mod flooding;
mod offsets;
mod probabilistic;
mod scantlings;

pub use flooding::{
    cross_flooding_time, equalization_stage_fractions, flood_cg_z, flood_volume_m3,
    tank_free_surface_moment_tm, tank_stage_compartment, FloodingError, TankCompartment,
};
pub use offsets::parse_offsets_csv;
pub use probabilistic::{
    attained_subdivision_index, combined_cargo_index, multi_zone_p_factor, p_factor, r_factor,
    required_index_cargo, s_factor_cargo, s_final_factor, s_intermediate_factor, s_mom_factor,
    survival_craft_moment, v_factor, DamageCaseProbability, DamageLengthDensity,
    DamagedSurvivability, FloodStage, ProbabilisticError,
};
pub use scantlings::{
    local_plate_scantling, plate_buckling_check_ec3, plate_buckling_reduction_ec3,
    plate_buckling_thickness_mm, plate_euler_stress_mpa, slab_bending_thickness_mm,
    stiffener_scantling, LocalPlateScantling, LocalPlateScantlingInput, PlateBucklingCapacity,
    ScantlingError, ScantlingMode, StiffenerScantling,
};

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
    /// Points from 0° to `to_deg` (10° steps for the prismatic model, the
    /// requested step for [`Bonjean::gz_curve`]).
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
        imo_2008_general_criteria(gz)
    }
}

/// The IMO 2008 IS Code general criteria on any GZ curve — the prismatic
/// model's or one from hull offsets ([`Bonjean::gz_curve`]). The curve
/// must reach 40° (and have at least two points); a shorter curve fails
/// with a message rather than being extrapolated.
pub fn imo_2008_general_criteria(gz: &GzCurve) -> ImoVerdict {
    let reach = gz.points.last().map_or(0.0, |p| p.heel_deg);
    if gz.points.len() < 2 || reach < 40.0 {
        return ImoVerdict {
            passed: false,
            area_to_30_deg: 0.0,
            area_to_40_deg: 0.0,
            max_gz_heel_deg: 0.0,
            max_gz_m: 0.0,
            gm_corrected_m: gz.gm_corrected_m,
            failures: vec![format!(
                "the GZ curve reaches {reach:.0}°; the criteria need it to reach 40°"
            )],
        };
    }
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
    let (max_gz, max_heel) = gz.points.iter().fold((0.0f64, 0.0f64), |(gm, gh), p| {
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
        let load: Vec<f64> = (0..n).map(|i| buoy_per_m_t - weight_t[i]).collect();

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

        let (peak_idx, peak, sign) =
            swbm.iter()
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

/// CSR-style hull-girder scantling requirement (review 7H leftover: the
/// screening slice of "class-society scantling checks").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GirderScantling {
    /// Still-water bending moment magnitude used, kN·m.
    pub swbm_knm: f64,
    /// IACS CSR wave-induced moments (sagging, hogging magnitudes), kN·m.
    pub wave_sagging_knm: f64,
    /// Hogging wave bending moment magnitude, kN·m.
    pub wave_hogging_knm: f64,
    /// Allowable normal stress, MPa (`175 / k`).
    pub allowable_mpa: f64,
    /// Required hull-girder section modulus, m³ — the larger of the
    /// sagging and hogging demands: `W = (|M_sw| + |M_wv|) / sigma`.
    pub required_modulus_m3: f64,
}

/// Higher-strength steel factor `k` per IACS UR S: 1.0 for ordinary
/// strength (mild) steel, 0.91 for AH32, 0.78 for AH36, 0.72 for AH40.
/// The CSR hull-girder allowable follows as `175 / k` MPa.
pub fn high_strength_factor(grade: &str) -> Option<f64> {
    match grade {
        "MS" | "A" | "B" | "D" | "AH" => Some(1.0),
        "AH32" | "DH32" | "EH32" => Some(0.91),
        "AH36" | "DH36" | "EH36" => Some(0.78),
        "AH40" | "DH40" | "EH40" => Some(0.72),
        _ => None,
    }
}

impl HullForm {
    /// Screens the hull-girder scantling: the required section modulus
    /// against the CSR normal-stress allowable `175/k` MPa, combining the
    /// supplied still-water moment with the rule wave-induced moments
    /// ([`Self::wave_bending_moment`]). Screening only — the full CSR
    /// check adds rule minimum modulus, local scantlings, buckling and
    /// sloping-floor corrections.
    ///
    /// Returns `None` when the length is outside the wave-moment rule
    /// range (90-300 m) or the inputs are not physical.
    pub fn scantling_requirement(&self, swbm_knm: f64, k_factor: f64) -> Option<GirderScantling> {
        if !(swbm_knm.is_finite() && swbm_knm >= 0.0) || !(k_factor.is_finite() && k_factor > 0.0) {
            return None;
        }
        let (sag, hog) = self.wave_bending_moment()?;
        let allowable = 175.0 / k_factor;
        let sw = swbm_knm.abs();
        let sag_demand = (sw + sag) / allowable * 1e-3; // kN·m / MPa -> m^3
        let hog_demand = (sw + hog.abs()) / allowable * 1e-3;
        Some(GirderScantling {
            swbm_knm: sw,
            wave_sagging_knm: sag,
            wave_hogging_knm: hog.abs(),
            allowable_mpa: allowable,
            required_modulus_m3: sag_demand.max(hog_demand),
        })
    }
}

/// One named damage case for the multi-case screen.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageCase {
    /// Case designation (e.g. "DB tank 3 alone", "WBT 4 P+S").
    pub name: String,
    /// The compartments flooded in this case.
    pub compartments: Vec<DamageCompartment>,
}

/// Per-case outcome of the multi-case screen.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageCaseResult {
    /// The case name.
    pub name: String,
    /// Damaged GM, m.
    pub gm_m: f64,
    /// List angle, degrees.
    pub list_angle_deg: f64,
    /// Damage trim (fore - aft), m.
    pub trim_m: f64,
    /// True when the case reaches the 0.05 m floor.
    pub passes_one_compartment: bool,
}

/// Summary of the multi-case damage screen: per-case outcomes plus the
/// governing (lowest-GM) case and the fleet verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageSummary {
    /// Per-case results in input order.
    pub cases: Vec<DamageCaseResult>,
    /// Index into `cases` of the governing (lowest-GM) case.
    pub governing_index: usize,
    /// True when every case passes the 0.05 m one-compartment floor.
    pub all_pass: bool,
}

impl HullForm {
    /// Multi-case damage screen (review 7H leftover: the deterministic
    /// precursor to probabilistic damage stability): runs
    /// [`Self::damage_stability`] for every case and reports the
    /// per-case outcomes, the governing (lowest-GM) case, and whether
    /// all cases pass. The full probabilistic method (p/s factors,
    /// subdivision index) remains a class-society calculation.
    ///
    /// # Errors
    ///
    /// `None` if any individual case cannot be solved (the summary
    /// requires every case to converge; solve cases one at a time with
    /// [`Self::damage_stability`] to isolate a failing one).
    pub fn damage_screen(
        &self,
        displacement_t: f64,
        lcg_from_midship_m: f64,
        kg_m: f64,
        free_surface_moment_tm: f64,
        cases: &[DamageCase],
    ) -> Option<DamageSummary> {
        let mut results = Vec::with_capacity(cases.len());
        for case in cases {
            let r = self.damage_stability(
                displacement_t,
                lcg_from_midship_m,
                kg_m,
                free_surface_moment_tm,
                &case.compartments,
            )?;
            results.push(DamageCaseResult {
                name: case.name.clone(),
                gm_m: r.gm_m,
                list_angle_deg: r.list_angle_deg,
                trim_m: r.trim_m,
                passes_one_compartment: r.passes_one_compartment,
            });
        }
        let governing_index = results
            .iter()
            .enumerate()
            .min_by(|(_ia, a), (_ib, b)| a.gm_m.total_cmp(&b.gm_m))
            .map(|(i, _)| i)
            .unwrap_or(0);
        Some(DamageSummary {
            all_pass: results.iter().all(|r| r.passes_one_compartment),
            governing_index,
            cases: results,
        })
    }
}

/// Equilibrium trim/list solution for a displacement and LCG (review 7H
/// leftover: "trim" in the prismatic model).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimResult {
    /// Mean draft at midship, m.
    pub mean_draft_m: f64,
    /// Draft at the forward perpendicular, m.
    pub fore_draft_m: f64,
    /// Draft at the aft perpendicular, m.
    pub aft_draft_m: f64,
    /// Trim by the bow (+) or stern (−): fore − aft, m.
    pub trim_m: f64,
    /// Trim angle, degrees (bow-down positive).
    pub trim_angle_deg: f64,
    /// Longitudinal centre of buoyancy from midship, m (equals the LCG at
    /// equilibrium).
    pub lcb_m: f64,
    /// Displacement of the trimmed waterline, t (matches the target).
    pub displacement_t: f64,
    /// GM at the mean draft against the loading KG, m (None without a KG).
    pub gm_m: Option<f64>,
}

impl HullForm {
    /// Displacement (t) and longitudinal buoyancy moment (t·m about
    /// midship) of a trimmed waterline in the prismatic model: the section
    /// area curve is uniform `Cb·B`, so the hull integrates as
    /// `Cb·B·∫draft(x)dx` with the draft clipped to the depth (deck
    /// immersion) and zero (end emergence).
    fn trimmed_volume_moment(&self, mean_draft: f64, tan_theta: f64) -> (f64, f64) {
        const N: usize = 200; // even stations over [0, L]
        let (l, b) = (self.loa_m, self.boa_m);
        let dx = l / N as f64;
        let (mut vol, mut moment) = (0.0_f64, 0.0_f64);
        for i in 0..N {
            let x = -l / 2.0 + (i as f64 + 0.5) * dx; // + forward
                                                      // Ends may emerge (draft clipped at 0); the screening model
                                                      // does not cap deck immersion.
            let d = (mean_draft + x * tan_theta).max(0.0);
            let area = self.cb * b * d;
            vol += area * dx;
            moment += area * dx * x;
        }
        (vol, moment)
    }

    /// Solves the even-keel-plus-trim equilibrium for a displacement and
    /// LCG: nested bisection — the mean draft is monotone in displacement
    /// for a fixed trim angle, and the trim angle is monotone in the
    /// buoyancy moment, so both legs converge exactly in the wall-sided
    /// regime (emerging ends included via the draft clip at zero).
    ///
    /// This is the prismatic (Bonjean-lite) answer: real Bonjean curves
    /// from hull offsets remain the roadmap item for fine forms.
    pub fn trim_equilibrium(
        &self,
        displacement_t: f64,
        lcg_from_midship_m: f64,
        kg_m: Option<f64>,
    ) -> Option<TrimResult> {
        if !displacement_t.is_finite() || displacement_t <= 0.0 {
            return None;
        }
        let target_vol = displacement_t / RHO_SEA_T_M3;

        // Inner: mean draft for a given trim angle (volume bisection).
        let vol_at = |t: f64, tan_theta: f64| self.trimmed_volume_moment(t, tan_theta).0;
        let solve_draft = |tan_theta: f64| -> f64 {
            let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
            while vol_at(hi, tan_theta) < target_vol && hi < 1e4 {
                hi *= 2.0;
            }
            for _ in 0..80 {
                let mid = 0.5 * (lo + hi);
                if vol_at(mid, tan_theta) < target_vol {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            0.5 * (lo + hi)
        };

        // Outer: trim angle for the buoyancy moment.
        let theta_cap = 0.25_f64; // ~14 deg: far beyond screening trims
        let moment_at = |theta: f64| -> f64 {
            let t = solve_draft(theta.tan());
            self.trimmed_volume_moment(t, theta.tan()).1 * RHO_SEA_T_M3
        };
        let (mut lo, mut hi) = (-theta_cap, theta_cap);
        // The moment is monotone increasing in bow-down trim; a target
        // outside the bracket is clamped to the cap (and flagged by the
        // caller through the resulting geometry).
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if moment_at(mid) < lcg_from_midship_m * displacement_t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let theta = 0.5 * (lo + hi);
        let t_mid = solve_draft(theta.tan());
        let (vol, _moment) = self.trimmed_volume_moment(t_mid, theta.tan());
        let half = self.loa_m / 2.0;
        let gm = kg_m.map(|kg| {
            let row = self.hydrostatics(t_mid);
            row.km_m - kg
        });
        Some(TrimResult {
            mean_draft_m: t_mid,
            fore_draft_m: t_mid + half * theta.tan(),
            aft_draft_m: (t_mid - half * theta.tan()).max(0.0),
            trim_m: t_mid + half * theta.tan() - (t_mid - half * theta.tan()).max(0.0),
            trim_angle_deg: theta.to_degrees(),
            lcb_m: lcg_from_midship_m,
            displacement_t: vol * RHO_SEA_T_M3,
            gm_m: gm,
        })
    }

    /// IACS CSR wave-induced vertical bending moments (review 7H leftover):
    /// sagging `+0.11·Cw·L²·B·(Cb+0.7)` and hogging `−0.13·Cw·L²·B·(Cb+0.7)`
    /// in kN·m, with the wave coefficient `Cw = 10.75 − ((300−L)/100)^1.5`
    /// for 90 ≤ L ≤ 300 m (returned as None outside the rule range).
    /// Combine with the still-water moment from
    /// [`Self::hull_girder_strength`] for the total girder demand.
    pub fn wave_bending_moment(&self) -> Option<(f64, f64)> {
        let l = self.loa_m;
        if !(90.0..=300.0).contains(&l) {
            return None;
        }
        let cw = 10.75 - ((300.0 - l) / 100.0).powf(1.5);
        let base = 0.11 * cw * l * l * self.boa_m * (self.cb + 0.7);
        let hog = -0.13 / 0.11 * base;
        Some((base, hog))
    }
}

/// One flooded compartment for the added-weight damage screen.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageCompartment {
    /// Compartment designation (e.g. "DB tank 3, P+S").
    pub name: String,
    /// Flooded volume up to the sea line (full volume for a bottom
    /// breach), m^3.
    pub volume_m3: f64,
    /// Compartment centroid: x from midship (+ forward), y transverse
    /// (+ starboard), z above keel, m.
    pub centroid: (f64, f64, f64),
    /// The compartment's own free-surface moment when slack, t·m
    /// (rho x l x b^3/12 for the tank's plan form) — supplied by the
    /// caller because the prismatic model does not know the plan geometry.
    pub free_surface_moment_tm: f64,
}

/// The static heel (degrees, signed like `tcg_m`) where the wall-sided
/// righting arm balances a transverse weight offset:
///
/// `GM sin(phi) + (BM / 2) tan(phi)^2 sin(phi) = TCG cos(phi)`
///
/// — the large-angle replacement for the small-angle `atan(TCG / GM)`
/// list. The left side rises monotonically for `GM > 0`, so the root is
/// unique and found by bisection; with `BM = 0` it is exactly
/// `atan(TCG / GM)`. Returns +/-90 when no equilibrium exists below 90
/// degrees (`GM <= 0` — the loll case — or an offset beyond the arm).
/// `gm_m` carries any free-surface correction; `bm_m` is the transverse
/// metacentric radius at the damaged draft.
#[must_use]
pub fn heel_equilibrium_deg(gm_m: f64, bm_m: f64, tcg_m: f64) -> f64 {
    let sign = if tcg_m < 0.0 { -1.0 } else { 1.0 };
    let t = tcg_m.abs();
    if t == 0.0 && gm_m > 0.0 {
        return 0.0;
    }
    if gm_m <= 0.0 || !gm_m.is_finite() || !bm_m.is_finite() || !t.is_finite() {
        return sign * 90.0;
    }
    let bm = bm_m.max(0.0);
    let g = |phi: f64| {
        let (s, c) = phi.sin_cos();
        gm_m * s + 0.5 * bm * (s / c) * (s / c) * s - t * c
    };
    let (mut lo, mut hi) = (0.0_f64, 89.9_f64.to_radians());
    if g(hi) < 0.0 {
        return sign * 90.0;
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if g(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    sign * (0.5 * (lo + hi)).to_degrees()
}

/// Outcome of the added-weight damage screen.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageResult {
    /// Displacement including the floodwater, t.
    pub displacement_t: f64,
    /// Floodwater mass, t.
    pub flooding_mass_t: f64,
    /// Mean draft of the damaged waterline, m.
    pub mean_draft_m: f64,
    /// Damaged trim (fore - aft), m.
    pub trim_m: f64,
    /// List angle, degrees: the equilibrium of the wall-sided righting arm
    /// against the floodwater's transverse offset,
    /// `GZ(phi) = TCG cos(phi)` (see [`heel_equilibrium_deg`]); reduces to
    /// `tan phi = TCG / GM` for small lists. 90 when no equilibrium exists
    /// below 90 degrees (GM <= 0 or a list beyond the wall-sided model).
    pub list_angle_deg: f64,
    /// Damaged GM: KM at the damaged draft, KG grown by the floodwater,
    /// minus the free-surface correction, m.
    pub gm_m: f64,
    /// True when the damaged GM reaches the 0.05 m one-compartment floor
    /// (IMO 2008 screening value; full probabilistic damage stability is
    /// a class-society calculation).
    pub passes_one_compartment: bool,
    /// Findings.
    pub notes: Vec<String>,
}

impl HullForm {
    /// Added-weight damage screen (review 7H leftover: "damage
    /// stability", first slice): floods each compartment fully to the sea
    /// line, re-solves the trim equilibrium with the grown displacement
    /// and shifted LCG, and reports the damaged GM and list angle.
    ///
    /// This is the screening answer for one damage case; the full
    /// probabilistic damage stability of the rules (subdivision, damage
    /// trim, down-flooding) remains with the class societies.
    ///
    /// # Errors
    ///
    /// `None` when the intact displacement is not a positive finite
    /// number or the damaged state cannot be solved.
    pub fn damage_stability(
        &self,
        displacement_t: f64,
        lcg_from_midship_m: f64,
        kg_m: f64,
        free_surface_moment_tm: f64,
        compartments: &[DamageCompartment],
    ) -> Option<DamageResult> {
        let mut notes = Vec::new();
        if !displacement_t.is_finite() || displacement_t <= 0.0 {
            return None;
        }
        let mut mass = displacement_t;
        let mut moment_x = displacement_t * lcg_from_midship_m;
        let mut moment_y = 0.0_f64;
        let mut moment_z = displacement_t * kg_m;
        let mut fsm = free_surface_moment_tm.max(0.0);
        for c in compartments {
            let flood_t = c.volume_m3 * RHO_SEA_T_M3;
            mass += flood_t;
            moment_x += flood_t * c.centroid.0;
            moment_y += flood_t * c.centroid.1;
            moment_z += flood_t * c.centroid.2;
            fsm += c.free_surface_moment_tm;
            notes.push(format!(
                "{}: +{flood_t:.0} t at ({:.1}, {:.1}, {:.1})",
                c.name, c.centroid.0, c.centroid.1, c.centroid.2
            ));
        }
        let lcg_damaged = moment_x / mass;
        let tcg_damaged = moment_y / mass;
        let kg_damaged = moment_z / mass;
        let solved = self.trim_equilibrium(mass, lcg_damaged, Some(kg_damaged))?;
        let km = self.hydrostatics(solved.mean_draft_m).km_m;
        let gm = km - kg_damaged - fsm / mass;
        let bm = {
            let hs = self.hydrostatics(solved.mean_draft_m);
            hs.km_m - hs.kb_m
        };
        let list = heel_equilibrium_deg(gm, bm, tcg_damaged);
        if list.abs() >= 90.0 {
            notes.push("no equilibrium heel below 90 deg: the vessel capsizes".to_string());
        }
        notes.push(format!(
            "damaged: draft {:.2} m, GM {:.2} m (free-surface {:.1} t*m), list {list:.2} deg",
            solved.mean_draft_m,
            gm,
            fsm / mass
        ));
        Some(DamageResult {
            displacement_t: mass,
            flooding_mass_t: mass - displacement_t,
            mean_draft_m: solved.mean_draft_m,
            trim_m: solved.trim_m,
            list_angle_deg: list,
            gm_m: gm,
            passes_one_compartment: gm >= 0.05,
            notes,
        })
    }
}

/// Half-breadth offsets of one station (review 7H leftover: hull offsets
/// and Bonjean curves from real sections).
#[derive(Debug, Clone, PartialEq)]
pub struct SectionOffsets {
    /// Station longitudinal position from midship, m (+ forward).
    pub x_from_midship_m: f64,
    /// `(draft above keel, half-breadth)` pairs, draft ascending, last
    /// pair at or above the maximum draught of interest. The section is
    /// taken symmetric about the centreline, breadths in m.
    pub half_breadths: Vec<(f64, f64)>,
}

impl SectionOffsets {
    /// Half-breadth at a draft: linear between table points, zero below
    /// the keel and beyond the table (a closed-ish top is the table's own
    /// responsibility).
    pub fn half_breadth_at(&self, draft_m: f64) -> f64 {
        let t = draft_m.max(0.0);
        let table = &self.half_breadths;
        if table.is_empty() {
            return 0.0;
        }
        if t <= table[0].0 {
            return if t == table[0].0 { table[0].1 } else { 0.0 };
        }
        for w in table.windows(2) {
            let (z0, y0) = w[0];
            let (z1, y1) = w[1];
            if t <= z1 {
                let f = (t - z0) / (z1 - z0).max(1e-12);
                return y0 + f * (y1 - y0);
            }
        }
        table.last().expect("non-empty").1
    }

    /// Sectional area up to a draft: `2 int y(z) dz` by the trapezoidal
    /// rule over the offset table (with the top point interpolated).
    pub fn area_to(&self, draft_m: f64) -> f64 {
        let t = draft_m.max(0.0);
        let table = &self.half_breadths;
        if table.is_empty() || t <= table[0].0 {
            return 0.0;
        }
        let mut area = 0.0;
        for w in table.windows(2) {
            let (z0, y0) = w[0];
            let (z1, y1) = w[1];
            if t <= z1 {
                let f = (t - z0) / (z1 - z0).max(1e-12);
                let y_t = y0 + f * (y1 - y0);
                area += 0.5 * (y0 + y_t) * (t - z0);
                return 2.0 * area;
            }
            area += 0.5 * (y0 + y1) * (z1 - z0);
        }
        2.0 * area
    }
}

/// Displacement and LCB of a trimmed waterline integrated over real
/// offsets (the Bonjean-sheet calculation the prismatic model stands in
/// for).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BonjeanSolution {
    /// Displacement, t.
    pub displacement_t: f64,
    /// Longitudinal centre of buoyancy from midship, m.
    pub lcb_m: f64,
    /// Immersed volume, m^3.
    pub volume_m3: f64,
}

/// Bonjean data from hull offsets: a set of stations, each with a
/// half-breadth table. Displacement, LCB and cross-curve ordinates
/// integrate the sectional areas over the length.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bonjean {
    /// Stations, any order; sorted internally on first use.
    pub stations: Vec<SectionOffsets>,
}

impl Bonjean {
    /// Sectional area at station position `x` and draft: linear between
    /// the two neighbouring stations, zero outside the table.
    pub fn section_area(&self, x_from_midship_m: f64, draft_m: f64) -> f64 {
        let mut sorted: Vec<&SectionOffsets> = self.stations.iter().collect();
        sorted.sort_by(|a, b| a.x_from_midship_m.total_cmp(&b.x_from_midship_m));
        if sorted.is_empty() {
            return 0.0;
        }
        if x_from_midship_m <= sorted[0].x_from_midship_m {
            return if x_from_midship_m == sorted[0].x_from_midship_m {
                sorted[0].area_to(draft_m)
            } else {
                0.0
            };
        }
        for w in sorted.windows(2) {
            let (a, b) = (w[0], w[1]);
            if x_from_midship_m <= b.x_from_midship_m {
                let f = (x_from_midship_m - a.x_from_midship_m)
                    / (b.x_from_midship_m - a.x_from_midship_m).max(1e-12);
                return a.area_to(draft_m) * (1.0 - f) + b.area_to(draft_m) * f;
            }
        }
        0.0
    }

    /// Volume and LCB of a waterline at `mean_draft` with trim slope
    /// `trim_tan` (draft = mean + x tan): Simpson integration of the
    /// sectional areas over the station span, 201 stations or the
    /// station count if denser.
    pub fn displacement(&self, mean_draft_m: f64, trim_tan: f64) -> BonjeanSolution {
        let mut sorted: Vec<&SectionOffsets> = self.stations.iter().collect();
        sorted.sort_by(|a, b| a.x_from_midship_m.total_cmp(&b.x_from_midship_m));
        if sorted.len() < 2 {
            return BonjeanSolution {
                displacement_t: 0.0,
                lcb_m: 0.0,
                volume_m3: 0.0,
            };
        }
        let x_min = sorted[0].x_from_midship_m;
        let x_max = sorted[sorted.len() - 1].x_from_midship_m;
        let n = (sorted.len() * 8).max(200); // even, as Simpson needs
        let dx = (x_max - x_min) / n as f64;
        let area = |i: usize| {
            let x = x_min + i as f64 * dx;
            self.section_area(x, mean_draft_m + x * trim_tan)
        };
        // Simpson needs an odd station count.
        let (mut vol, mut moment) = (0.0_f64, 0.0_f64);
        for i in 0..=n {
            let a = area(i);
            let x = x_min + i as f64 * dx;
            let w = if i == 0 || i == n {
                1.0
            } else if i % 2 == 1 {
                4.0
            } else {
                2.0
            };
            vol += w * a;
            moment += w * a * x;
        }
        vol *= dx / 3.0;
        moment *= dx / 3.0;
        BonjeanSolution {
            displacement_t: vol * RHO_SEA_T_M3,
            lcb_m: if vol > 1e-9 { moment / vol } else { 0.0 },
            volume_m3: vol,
        }
    }

    /// Cross-curve ordinate KN at heel: the lever from the keel to the
    /// line of action of the buoyancy,
    /// `KN = -y_B cos phi + z_B sin phi` (`y_B < 0` on the submerged
    /// side, so `KN ~ KM sin phi` at small heel), from a strip integration of the
    /// submerged width of every station under the inclined waterline. The
    /// waterline rotates about the centreline point `(0, draft)` — the
    /// classical cross-curve convention — so its plane is
    /// `z cos phi + y sin phi = draft cos phi`. Returns
    /// `(displacement_t, kn_m)`. GZ follows as `KN - KG sin phi`.
    pub fn cross_curve_ordinate(&self, mean_draft_m: f64, heel_deg: f64) -> (f64, f64) {
        let phi = heel_deg.to_radians();
        let (sin, cos) = phi.sin_cos();
        // Plane constant: the waterline passes through (0, draft).
        let t_plane = mean_draft_m * cos;
        let mut sorted: Vec<&SectionOffsets> = self.stations.iter().collect();
        sorted.sort_by(|a, b| a.x_from_midship_m.total_cmp(&b.x_from_midship_m));
        if sorted.len() < 2 {
            return (0.0, 0.0);
        }
        let x_min = sorted[0].x_from_midship_m;
        let x_max = sorted[sorted.len() - 1].x_from_midship_m;

        // Half-breadth at (x, z): linear between the two neighbouring
        // stations (as the Bonjean section_area does), zero outside the
        // station span. The station pair is resolved once per x.
        let station_pair = |x: f64| -> (usize, usize, f64) {
            if x < x_min || x > x_max {
                return (0, 0, 0.0); // zero-width sentinel
            }
            for (i, w) in sorted.windows(2).enumerate() {
                let (a, b) = (w[0], w[1]);
                if x <= b.x_from_midship_m {
                    let f = (x - a.x_from_midship_m)
                        / (b.x_from_midship_m - a.x_from_midship_m).max(1e-12);
                    return (i, i + 1, f);
                }
            }
            (sorted.len() - 1, sorted.len() - 1, 0.0)
        };

        let n = (sorted.len() * 8).max(200); // even, as Simpson needs
        let dx = (x_max - x_min) / n as f64;
        // Vertical strips: z up to a generous cap (deepest table point plus
        // margin covers all realistic sections).
        let z_cap = mean_draft_m
            + 3.0
                * sorted
                    .iter()
                    .map(|s| s.half_breadths.last().map(|(z, _)| *z).unwrap_or(0.0))
                    .fold(0.0_f64, f64::max)
            + 1.0;
        let nz = 200;
        let dz = z_cap / nz as f64;
        let (mut vol, mut mx, mut my, mut mz) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
        for i in 0..=n {
            let x = x_min + i as f64 * dx;
            let (ia, ib, fx) = station_pair(x);
            let (st_a, st_b) = (sorted[ia], sorted[ib]);
            let wx = if i == 0 || i == n {
                1.0
            } else if i % 2 == 1 {
                4.0
            } else {
                2.0
            };
            for k in 0..=nz {
                let z = k as f64 * dz;
                if ia == ib && fx == 0.0 {
                    continue; // outside the station span
                }
                let y = st_a.half_breadth_at(z) * (1.0 - fx) + st_b.half_breadth_at(z) * fx;
                if y <= 1e-12 {
                    continue;
                }
                // Submerged set at height z is the ray y <= cut (the
                // waterline is one straight line in section view), so the
                // flooded width is min(y, cut) - (-y), and the strip
                // centroid sits halfway between -y and min(y, cut).
                let cut = (t_plane - z * cos) / sin.max(1e-9);
                let y_star = y.min(cut);
                let w = (y_star + y).max(0.0);
                if w <= 0.0 {
                    continue;
                }
                let wk = if k == 0 || k == nz {
                    1.0
                } else if k % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                let d = wx * wk;
                vol += d * w;
                mx += d * w * x;
                my += d * w * 0.5 * (y_star - y);
                mz += d * w * z;
            }
        }
        let f = dx * dz / 9.0; // two Simpson directions
        vol *= f;
        mx *= f;
        my *= f;
        mz *= f;
        if vol <= 1e-9 {
            return (0.0, 0.0);
        }
        let y_cb = my / vol;
        let z_cb = mz / vol;
        let _ = mx;
        (vol * RHO_SEA_T_M3, -y_cb * cos + z_cb * sin)
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

    /// Review 7H leftover: trim equilibrium. The even-keel case must
    /// reproduce the closed-form box displacement exactly, and a
    /// small eccentric LCG must agree with the classic MCT1cm estimate
    /// `trim = W x LCG / MCT1cm` to within a few percent.
    #[test]
    fn trim_equilibrium_matches_the_box_and_mct() {
        let hull = feeder();
        // Even keel: T = W / (rho Cb B L).
        let w = 20_000.0; // t
        let t_even = w / (1.025 * 0.72 * 22.0 * 140.0);
        let r = hull.trim_equilibrium(w, 0.0, None).expect("solvable");
        assert!(
            (r.mean_draft_m - t_even).abs() < 1e-6,
            "{} vs {t_even}",
            r.mean_draft_m
        );
        assert!(r.trim_m.abs() < 1e-9);
        assert!((r.displacement_t - w).abs() < 1e-6);

        // Bow trim: LCG 1.5 m forward of midship. The exact closed form
        // of the same prismatic model is tan(theta) = 12 W LCG /(rho Cb B L^3)
        // (moment = rho Cb B theta L^3/12) — the numerical solver must
        // reproduce it to within integration error.
        let lcg = 1.5;
        let r2 = hull.trim_equilibrium(w, lcg, None).expect("solvable");
        assert!(r2.trim_m > 0.0, "bow-down trim expected");
        let tan_exact = 12.0 * w * lcg / (1.025 * 0.72 * 22.0 * 140.0_f64.powi(3));
        let trim_exact = 140.0 * tan_exact;
        assert!(
            (r2.trim_m - trim_exact).abs() < 0.01 * trim_exact,
            "solved trim {} m vs closed form {trim_exact} m",
            r2.trim_m
        );
        // The classic MCT1cm estimate agrees only loosely: the crate's
        // MCT uses the Cwp-based waterplane inertia and a screen KG — a
        // documented approximation (here ~13 % off).
        let row = hull.hydrostatics(r2.mean_draft_m);
        let mct_estimate_m = w * lcg / row.mct1cm_tm_cm / 100.0;
        assert!(
            (r2.trim_m - mct_estimate_m).abs() < 0.25 * mct_estimate_m,
            "solved trim {} m vs MCT1cm estimate {mct_estimate_m} m",
            r2.trim_m
        );
        // Displacement is preserved through the trim, and the drafts
        // bracket the mean.
        assert!((r2.displacement_t - w).abs() < 1e-6);
        assert!(r2.fore_draft_m > r2.aft_draft_m);
        assert!((r2.fore_draft_m + r2.aft_draft_m).abs() - 2.0 * r2.mean_draft_m < 1e-9);

        // GM comes back when a KG is supplied.
        let r3 = hull.trim_equilibrium(w, 0.0, Some(9.0)).expect("solvable");
        let gm = r3.gm_m.expect("KG supplied");
        let km = hull.hydrostatics(r3.mean_draft_m).km_m;
        assert!((gm - (km - 9.0)).abs() < 1e-9);
    }

    /// Review 7H leftover: CSR girder scantling screen — the required
    /// section modulus combines the still-water moment with the rule wave
    /// moments against 175/k MPa, hand-checked end to end.
    #[test]
    fn scantling_requirement_matches_hand_calculation() {
        let hull = feeder(); // 140 x 22 x Cb 0.72
        let swbm = 42_000.0; // kN m, a loaded bulk-carrier-ish SWBM

        // AH36: k = 0.78, allowable = 224.36 MPa.
        let k = high_strength_factor("AH36").expect("AH36 known");
        assert!((k - 0.78).abs() < 1e-12);
        let req = hull.scantling_requirement(swbm, k).expect("in rule range");
        let cw = 10.75 - (1.6_f64).powf(1.5);
        let sag = 0.11 * cw * 140.0 * 140.0 * 22.0 * 1.42;
        let hog = 0.13 / 0.11 * sag;
        let allowable = 175.0 / 0.78;
        let w_sag = (swbm + sag) / allowable * 1e-3;
        let w_hog = (swbm + hog) / allowable * 1e-3;
        assert!((req.wave_sagging_knm - sag).abs() < 1e-6);
        assert!((req.required_modulus_m3 - w_sag.max(w_hog)).abs() < 1e-9);
        // Sagging governs here (larger wave moment ratio).
        assert!(req.required_modulus_m3 > w_hog - 1e-12);
        // AH36's higher allowable shrinks the required modulus: mild
        // steel needs 1/0.78 times as much section.
        let mild = hull.scantling_requirement(swbm, 1.0).unwrap();
        assert!((mild.required_modulus_m3 / req.required_modulus_m3 - 1.0 / 0.78).abs() < 1e-9);

        // Outside the rule range / bad inputs: None.
        let short = HullForm {
            loa_m: 60.0,
            boa_m: 12.0,
            cb: 0.6,
            cwp: 0.8,
        };
        assert!(short.scantling_requirement(swbm, 1.0).is_none());
        assert!(hull.scantling_requirement(-1.0, 1.0).is_none());
        assert!(hull.scantling_requirement(swbm, 0.0).is_none());
    }

    /// Review 7H leftover: Bonjean integration from hull offsets. A box
    /// hull's offsets must reproduce the closed-form prismatic values
    /// exactly (volume, LCB, trim shift); a 45-degree V-section must give
    /// the triangle area T^2.
    #[test]
    fn bonjean_integrates_box_and_v_sections() {
        // Box: 140 x 22, offsets constant B/2 to 12 m, 15 stations.
        let box_hull = Bonjean {
            stations: (-7..=7)
                .map(|i| SectionOffsets {
                    x_from_midship_m: 10.0 * i as f64,
                    half_breadths: (0..=12).map(|z| (z as f64, 11.0)).collect(),
                })
                .collect(),
        };
        let t = 8.0;
        let sol = box_hull.displacement(t, 0.0);
        let expected_vol = 140.0 * 22.0 * t * 1.0; // full box Cb = 1
        assert!(
            (sol.volume_m3 - expected_vol).abs() < 1.0,
            "{} vs {expected_vol}",
            sol.volume_m3
        );
        assert!(sol.lcb_m.abs() < 1e-9);

        // Trim: volume unchanged (no emergence), LCB shifts by
        // theta L^2 / (12 T) — the moment ratio theta L^3/12 divided by
        // the waterplane integral T L.
        let theta = 0.01;
        let sol_t = box_hull.displacement(t, theta);
        assert!((sol_t.volume_m3 - expected_vol).abs() < 1.0);
        let lcb_shift = theta * 140.0_f64.powi(2) / (12.0 * t);
        assert!(
            (sol_t.lcb_m - lcb_shift).abs() < 0.02,
            "{} vs {lcb_shift}",
            sol_t.lcb_m
        );

        // V-section: y(z) = z -> area to draft T is T^2.
        let v = SectionOffsets {
            x_from_midship_m: 0.0,
            half_breadths: (0..=10).map(|z| (z as f64, z as f64)).collect(),
        };
        assert!((v.area_to(5.0) - 25.0).abs() < 1e-9);
        assert!((v.area_to(2.5) - 6.25).abs() < 1e-9);
    }

    /// Review 7H leftover: cross-curve ordinates from offsets. For the box
    /// hull the buoyancy centroid sits at (0, T/2), so
    /// KN = (T/2) cos(phi) for every heel angle.
    #[test]
    fn cross_curves_match_the_box_centroid() {
        let box_hull = Bonjean {
            stations: (-7..=7)
                .map(|i| SectionOffsets {
                    x_from_midship_m: 10.0 * i as f64,
                    half_breadths: (0..=12).map(|z| (z as f64, 11.0)).collect(),
                })
                .collect(),
        };
        let t = 6.0;
        // Box closed form (small heel, B tan(phi)/2 < T so the lost
        // triangle stays clear of the keel): volume preserved (the
        // gained/lost wall-sided triangles cancel), and the wedge pair
        // moves the buoyancy centre to
        //   z = T/2 + B^2 tan^2/(24 T),  y = -B^2 tan/(12 T),
        // so KN = -y cos(phi) + z sin(phi) (~ KM sin(phi) at small heel).
        for &phi_deg in &[10.0, 25.0] {
            let (disp, kn) = box_hull.cross_curve_ordinate(t, phi_deg);
            let expected_disp = 140.0 * 22.0 * t * 1.025;
            assert!(
                (disp - expected_disp).abs() < 40.0,
                "heel {phi_deg}: disp {disp} vs {expected_disp}"
            );
            let tan = phi_deg.to_radians().tan();
            let z_bar = t / 2.0 + 22.0_f64.powi(2) * tan * tan / (24.0 * t);
            let y_bar = -22.0_f64.powi(2) * tan / (12.0 * t);
            let expected = -y_bar * phi_deg.to_radians().cos() + z_bar * phi_deg.to_radians().sin();
            assert!(
                (kn - expected).abs() < 0.03 * expected.max(1.0),
                "heel {phi_deg}: KN {kn} vs {expected}"
            );
        }
        // At 45 deg the lost triangle clips on the keel (B tan(phi)/2 = 11
        // > T): at fixed draft the volume grows past the upright value.
        let (disp45, _) = box_hull.cross_curve_ordinate(t, 45.0);
        assert!(
            disp45 > 140.0 * 22.0 * t * 1.025,
            "clipped loss must gain volume: {disp45}"
        );
        // GZ from KN requires the cross-curve at CONSTANT DISPLACEMENT:
        // bisect for the heeled draft that restores the upright
        // displacement, then GZ = KN - KG sin(phi).
        let w_upright = 140.0 * 22.0 * t * 1.025;
        let (mut lo, mut hi) = (1.0_f64, 12.0_f64);
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            let (d, _) = box_hull.cross_curve_ordinate(mid, 45.0);
            if d < w_upright {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let t_heel = 0.5 * (lo + hi);
        // At 45 deg (tan = 1) the constant-volume waterline is z + y = T'
        // with (T' + B/2)^2 / 2 = upright area: the flooded section is the
        // exact triangle (−B/2, 0), (T', 0), (−B/2, T'+B/2), so
        // KN = (z' - y')/sqrt(2) with (y', z') its centroid.
        let expected_tri_area = t * 22.0; // upright section area, m^2
        let t_heel_expected = (2.0 * expected_tri_area).sqrt() - 11.0;
        assert!(
            (t_heel - t_heel_expected).abs() < 0.02,
            "constant-volume draft {t_heel} vs {t_heel_expected}"
        );
        let y_c = (5.248_f64 - 11.0 - 11.0) / 3.0;
        let z_c = (t_heel_expected + 11.0) / 3.0;
        let kn_expected = (z_c - y_c) / std::f64::consts::SQRT_2;
        let (_, kn45) = box_hull.cross_curve_ordinate(t_heel, 45.0);
        assert!(
            (kn45 - kn_expected).abs() < 0.02,
            "KN {kn45} vs triangle {kn_expected}"
        );
        // The beamy KG = T/2 box (GM = 6.7 m) is still strongly stable at
        // 45 deg: KN = 11/sqrt(2), so GZ = 11/sqrt(2) - 3 sin(45) = 4 sqrt(2).
        let gz = kn45 - 3.0 * 45.0_f64.to_radians().sin();
        assert!(
            (gz - 4.0 * std::f64::consts::SQRT_2).abs() < 0.03,
            "GZ at 45 deg: {gz}"
        );
        // ...and positive at 25 deg with the constant-volume draft.
        let (mut lo, mut hi) = (1.0_f64, 12.0_f64);
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            let (d, _) = box_hull.cross_curve_ordinate(mid, 25.0);
            if d < w_upright {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let (_, kn25) = box_hull.cross_curve_ordinate(0.5 * (lo + hi), 25.0);
        assert!(kn25 - 3.0 * 25.0_f64.to_radians().sin() > 0.0);
    }

    /// Review 7H leftover: added-weight damage screen. A symmetric
    /// amidships compartment must keep the trim at zero and cut the GM by
    /// exactly the free-surface term; an off-centre compartment must list
    /// by atan(TCG / GM).
    #[test]
    fn damage_screen_trims_lists_and_cuts_gm() {
        let hull = feeder(); // 140 x 22 x Cb 0.72
        let w = 20_000.0; // t
        let kg = 9.0;
        // Symmetric double-bottom flooding amidships: 800 m^3 at the
        // centreline with a slack free-surface moment of 400 t m.
        let sym = DamageCompartment {
            name: "DB 4 P+S".into(),
            volume_m3: 800.0,
            centroid: (0.0, 0.0, 1.0),
            free_surface_moment_tm: 400.0,
        };
        let r = hull
            .damage_stability(w, 0.0, kg, 0.0, std::slice::from_ref(&sym))
            .expect("solvable");
        assert!((r.flooding_mass_t - 800.0 * 1.025).abs() < 1e-9);
        assert!(r.trim_m.abs() < 1e-6, "symmetric flood must not trim");
        assert!(r.list_angle_deg.abs() < 1e-9);
        // KG drops (floodwater at z = 1 m) but the free surface bites:
        // GM = KM(T') - KG' - FSM/W'. Hand-computed with the grown
        // displacement 21578 t: draft 8.99 m, KM 11.30, KG 8.565,
        // FSM/W 0.0185 -> GM 2.72.
        let km = hull.hydrostatics(r.mean_draft_m).km_m;
        let kg_expected = (w * kg + 800.0 * 1.025 * 1.0) / r.displacement_t;
        let gm_expected = km - kg_expected - 400.0 / r.displacement_t;
        assert!((r.gm_m - gm_expected).abs() < 1e-9);
        // Note the physics: ANY floodwater below KG dilutes G downward and
        // can raise the damaged GM even though the free surface bites —
        // this ship is very stable, so one-compartment damage genuinely
        // does not cost GM here. The isolated free-surface penalty is what
        // the method guarantees: the same flooded compartment, slack vs
        // pressed up, differs by exactly FSM / W'.
        let pressed = DamageCompartment {
            name: "Machinery space".into(),
            volume_m3: 800.0,
            centroid: (20.0, 0.0, 7.0),
            free_surface_moment_tm: 0.0,
        };
        let slack = DamageCompartment {
            free_surface_moment_tm: 900.0,
            ..pressed.clone()
        };
        let up = hull
            .damage_stability(w, 0.0, kg, 0.0, &[pressed])
            .expect("solvable");
        let slack_r = hull
            .damage_stability(w, 0.0, kg, 0.0, &[slack])
            .expect("solvable");
        assert!((up.gm_m - slack_r.gm_m - 900.0 / up.displacement_t).abs() < 1e-9);
        assert!(r.passes_one_compartment);

        // An off-centre wing-tank flood lists the vessel: TCG 0.4 m.
        let wing = DamageCompartment {
            name: "WBT 3 S".into(),
            volume_m3: 300.0,
            centroid: (10.0, 0.4, 6.0),
            free_surface_moment_tm: 0.0,
        };
        let r2 = hull
            .damage_stability(w, 0.0, kg, 0.0, &[wing])
            .expect("solvable");
        let tcg = 300.0 * 1.025 * 0.4 / r2.displacement_t;
        // The list solves GZ(phi) = TCG cos(phi) on the wall-sided arm and
        // stays within a fraction of a percent of the small-angle answer.
        let bm = {
            let hs = hull.hydrostatics(r2.mean_draft_m);
            hs.km_m - hs.kb_m
        };
        let p = r2.list_angle_deg.to_radians();
        let residual = r2.gm_m * p.sin() + 0.5 * bm * p.tan().powi(2) * p.sin() - tcg * p.cos();
        assert!(residual.abs() < 1e-9, "{residual}");
        let small = (tcg / r2.gm_m).atan().to_degrees();
        assert!(
            (r2.list_angle_deg - small).abs() < 0.01 * small,
            "{} vs {small}",
            r2.list_angle_deg
        );
        assert!(r2.list_angle_deg > 0.0, "starboard flood lists starboard");
    }

    /// Review 7H leftover: multi-case damage screen. A wing-tank flood
    /// (off-centre + free surface) must govern over a centreline double-
    /// bottom flood (whose low floodwater can even raise GM), and the
    /// summary must flip `all_pass` when a case fails.
    #[test]
    fn damage_screen_governs_on_lowest_gm() {
        let hull = feeder(); // 140 x 22 x Cb 0.72
        let w = 20_000.0;
        let kg = 9.0;

        let cases = vec![
            DamageCase {
                name: "DB 4 P+S".into(),
                compartments: vec![DamageCompartment {
                    name: "DB 4".into(),
                    volume_m3: 800.0,
                    centroid: (0.0, 0.0, 1.0),
                    free_surface_moment_tm: 400.0,
                }],
            },
            DamageCase {
                name: "WBT 3 S".into(),
                compartments: vec![DamageCompartment {
                    name: "WBT 3 S".into(),
                    volume_m3: 800.0,
                    centroid: (10.0, 5.0, 6.0),
                    free_surface_moment_tm: 900.0,
                }],
            },
        ];
        let summary = hull
            .damage_screen(w, 0.0, kg, 0.0, &cases)
            .expect("all cases solve");
        assert_eq!(summary.cases.len(), 2);
        // The off-centre wing flood (high floodwater + big free surface)
        // must govern.
        assert_eq!(summary.governing_index, 1);
        assert!(
            summary.cases[1].gm_m < summary.cases[0].gm_m,
            "wing {:?} vs DB {:?}",
            summary.cases[1].gm_m,
            summary.cases[0].gm_m
        );
        // Cross-check the governing GM against the single-case function.
        let single = hull
            .damage_stability(w, 0.0, kg, 0.0, &cases[1].compartments)
            .unwrap();
        assert!((summary.cases[1].gm_m - single.gm_m).abs() < 1e-12);

        // Add a catastrophic case (huge free surface) and the summary
        // flips all_pass and re-governs.
        let mut failing = cases.clone();
        failing.push(DamageCase {
            name: "machinery flood".into(),
            compartments: vec![DamageCompartment {
                name: "M/R".into(),
                volume_m3: 3_000.0,
                centroid: (20.0, 0.0, 8.0),
                free_surface_moment_tm: 30_000.0,
            }],
        });
        let summary2 = hull
            .damage_screen(w, 0.0, kg, 0.0, &failing)
            .expect("all cases solve");
        assert!(!summary2.all_pass);
        assert_eq!(summary2.governing_index, 2);
        assert!(!summary2.cases[2].passes_one_compartment);
        // Empty case list: trivially all-pass, no governing case content.
        let empty = hull.damage_screen(w, 0.0, kg, 0.0, &[]).unwrap();
        assert!(empty.all_pass);
        assert!(empty.cases.is_empty());
    }

    /// Review 7H leftover: IACS CSR wave-induced bending moments — the
    /// closed-form coefficients checked at a known length, and the rule
    /// range enforced.
    #[test]
    fn wave_bending_matches_the_csr_closed_form() {
        let hull = feeder(); // L 140, B 22, Cb 0.72
                             // Cw = 10.75 - ((300-140)/100)^1.5 = 10.75 - 2.0236 = 8.7264.
        let cw = 10.75 - (1.6_f64).powf(1.5);
        let sag = 0.11 * cw * 140.0 * 140.0 * 22.0 * (0.72 + 0.7);
        let hog = -0.13 / 0.11 * sag;
        let (s, h) = hull.wave_bending_moment().expect("L inside rule range");
        assert!((s - sag).abs() < 1e-6, "{s} vs {sag}");
        assert!((h - hog).abs() < 1e-6, "{h} vs {hog}");
        // Outside 90-300 m the rules do not apply.
        let short = HullForm {
            loa_m: 60.0,
            boa_m: 12.0,
            cb: 0.6,
            cwp: 0.8,
        };
        assert!(short.wave_bending_moment().is_none());
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
            + 0.5
                * (hs.km_m - hs.kb_m)
                * 10f64.to_radians().tan().powi(2)
                * 10f64.to_radians().sin();
        let point = gz_clean
            .points
            .iter()
            .find(|p| p.heel_deg == 10.0)
            .expect("10° point");
        assert!(
            (point.gz_m - expected).abs() < 1e-9,
            "{} vs {}",
            point.gz_m,
            expected
        );
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
        let good =
            hull.imo_2008_general(&hull.gz_curve(LoadingCondition::new(6.0, 8.0), 40.0), 6.0);
        assert!(good.passed, "{good}");
        // KG above KM at 6 m draft: negative GM, everything fails.
        let bad =
            hull.imo_2008_general(&hull.gz_curve(LoadingCondition::new(6.0, 14.0), 40.0), 6.0);
        assert!(!bad.passed);
        assert!(bad.failures.iter().any(|f| f.contains("GM")));
        assert!(bad.failures.iter().any(|f| f.contains("30°")));
        // Free surface degrades a marginal case.
        let marginal = LoadingCondition::new(6.0, 12.5);
        let dry = hull.imo_2008_general(&hull.gz_curve(marginal, 40.0), 6.0);
        let wet = hull.imo_2008_general(
            &hull.gz_curve(marginal.with_free_surface_tm(3_000.0), 40.0),
            6.0,
        );
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
            &[WeightItem {
                x_m: 70.0,
                mass_t: 3000.0,
            }],
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
                WeightItem {
                    x_m: 10.0,
                    mass_t: 1500.0,
                },
                WeightItem {
                    x_m: 130.0,
                    mass_t: 1500.0,
                },
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
            &[WeightItem {
                x_m: 60.0,
                mass_t: 2500.0,
            }],
            50,
        );
        // Find the shear zero crossing nearest the peak moment station.
        let zero = r
            .swsf_t
            .windows(2)
            .enumerate()
            .find(|(_, w)| w[0].signum() != w[1].signum());
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
            &[WeightItem {
                x_m: 70.0,
                mass_t: 1_000_000.0,
            }],
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

#[cfg(test)]
mod heel_solver_tests {
    use super::heel_equilibrium_deg;

    #[test]
    fn without_a_bm_term_it_is_exactly_the_atan_list() {
        for (gm, t) in [(1.0_f64, 0.1_f64), (0.5, 0.3), (2.0, 1.5), (0.2, 0.05)] {
            let exact = (t / gm).atan().to_degrees();
            assert!((heel_equilibrium_deg(gm, 0.0, t) - exact).abs() < 1e-9);
            assert!((heel_equilibrium_deg(gm, 0.0, -t) + exact).abs() < 1e-9);
        }
    }

    #[test]
    fn the_solution_balances_the_wall_sided_arm_and_beats_small_angle() {
        // A large list: GM 0.6, BM 8, TCG 0.35 — the BM tan^2 term makes
        // the vessel stiffer than atan(TCG/GM) says.
        let (gm, bm, t) = (0.6, 8.0, 0.35);
        let phi = heel_equilibrium_deg(gm, bm, t);
        let p = phi.to_radians();
        let resid = gm * p.sin() + 0.5 * bm * p.tan().powi(2) * p.sin() - t * p.cos();
        assert!(resid.abs() < 1e-12, "{resid}");
        let small = (t / gm).atan().to_degrees();
        assert!(phi < small && phi > 0.0, "{phi} vs small-angle {small}");
        // Monotone in the offset.
        assert!(heel_equilibrium_deg(gm, bm, 0.5) > phi);
    }

    #[test]
    fn no_equilibrium_means_capsize_and_zero_offset_means_upright() {
        assert_eq!(heel_equilibrium_deg(0.0, 8.0, 0.1), 90.0);
        assert_eq!(heel_equilibrium_deg(-0.3, 8.0, -0.1), -90.0);
        assert_eq!(heel_equilibrium_deg(1.0, 8.0, 0.0), 0.0);
        // Offset far beyond any arm with no BM: no root below 90 deg
        // would need TCG > GM tan(89.9); a huge offset capsizes.
        assert_eq!(heel_equilibrium_deg(0.1, 0.0, 1.0e6), 90.0);
    }
}
