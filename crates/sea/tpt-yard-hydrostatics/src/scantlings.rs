//! Local scantling checks — the plate-level slice of the
//! class-society scantling work (review 7H roadmap), next to the
//! hull-girder [`GirderScantling`](crate::GirderScantling).
//!
//! Two legs, both closed-form and identity-verified:
//!
//! - **Lateral-pressure plate bending** — the class slab formula
//!   `t_net = 22.4·alpha_p·s·sqrt(k·p / sigma)`. The constant is the
//!   clamped-plate factor `sigma = 0.5·p·s^2/t^2` expressed in
//!   (m, kN/m², N/mm²) units: `1000·sqrt(0.5·1e-3) = 22.36`, printed
//!   rounded to 22.4 in the rules. `alpha_p` is the rule boundary
//!   factor (CSR uses 1.2 for load-carrying plates; 1.0 recovers the
//!   plain clamped-slab formula); `k` scales the demand side
//!   (higher-strength steel is thinner, the same direction as the
//!   `175/k` hull-girder allowable), so rule tables that already
//!   fold the material factor into `sigma` pass `k = 1`.
//! - **Plate Euler buckling** — the classical simply-supported
//!   uniaxial critical stress `sigma_E = k_asp·pi²·E·t² / (12·(1-nu²)·b²)`
//!   with the half-wave aspect factor `k_asp = (m·b/a + a/(m·b))²`,
//!   `m` the nearest half-wave count (`m = ceil(a/b)`): `k = 4` for
//!   square and long panels — the textbook minimum. The required
//!   thickness for a compressive demand is the exact inversion, so
//!   `sigma_E(t_required) = sigma_applied` identically.
//!
//! Class-rule refinements that stay out of this slice (documented,
//! not faked): the CSR buckling reduction curves (post-buckling
//! reserve via usage factors on the slenderness lambda), rule minimum
//! thickness tables, corrosion addition defaults, and stiffener
//! flange/web checks.

/// The clamped-plate slab constant in (m, kN/m², N/mm²):
/// `1000·sqrt(0.5·1e-3)` (the rules print the rounded 22.4).
const SLAB_MM: f64 = 22.360_679_775_0;

/// Errors from the local scantling checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScantlingError {
    /// A geometric or load input is not finite or not positive.
    InvalidInput,
    /// A compressive demand was given without the panel's long span.
    BucklingNeedsLongSpan,
}

impl std::fmt::Display for ScantlingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScantlingError::InvalidInput => {
                f.write_str("scantling inputs must be finite and physical")
            }
            ScantlingError::BucklingNeedsLongSpan => {
                f.write_str("a buckling demand needs the panel long span (Some(long_span_m))")
            }
        }
    }
}

impl std::error::Error for ScantlingError {}

/// Net plate thickness (mm) required by lateral-pressure bending:
/// the clamped-slab relation `sigma = 0.5·p·s²/t²` solved for `t`, in
/// (m, kN/m², N/mm²) units. `material_factor_k` scales the *demand*
/// (higher-strength steel is thinner, matching the `175/k`
/// hull-girder convention); `boundary_factor` scales the demand the
/// way the rule boundary coefficient does (1.0 = plain clamped slab).
pub fn slab_bending_thickness_mm(
    spacing_m: f64,
    pressure_kn_m2: f64,
    allowable_bending_mpa: f64,
    material_factor_k: f64,
    boundary_factor: f64,
) -> f64 {
    boundary_factor
        * SLAB_MM
        * spacing_m
        * (material_factor_k * pressure_kn_m2 / allowable_bending_mpa).sqrt()
}

/// The classical simply-supported plate Euler stress (MPa) for
/// uniaxial compression: `k_asp·pi²·E·t² / (12·(1-nu²)·b²)` with
/// `k_asp = (m·b/a + a/(m·b))²`, `m = ceil(a/b) >= 1` half-waves over
/// the long span `a` (spans in mm; the factors are dimensionless).
pub fn plate_euler_stress_mpa(
    thickness_mm: f64,
    short_span_mm: f64,
    long_span_mm: f64,
    youngs_modulus_mpa: f64,
    poissons_ratio: f64,
) -> Result<f64, ScantlingError> {
    if !(thickness_mm.is_finite() && thickness_mm > 0.0)
        || !(short_span_mm.is_finite() && short_span_mm > 0.0)
        || !(long_span_mm.is_finite() && long_span_mm >= short_span_mm)
        || !(youngs_modulus_mpa.is_finite() && youngs_modulus_mpa > 0.0)
        || !(poissons_ratio.is_finite() && poissons_ratio > -1.0 && poissons_ratio < 0.5)
    {
        return Err(ScantlingError::InvalidInput);
    }
    let (k_asp, _) = aspect_factor(short_span_mm, long_span_mm);
    Ok(k_asp
        * std::f64::consts::PI
        * std::f64::consts::PI
        * youngs_modulus_mpa
        * thickness_mm
        * thickness_mm
        / (12.0 * (1.0 - poissons_ratio * poissons_ratio) * short_span_mm * short_span_mm))
}

/// The net plate thickness (mm) for which the Euler stress equals the
/// applied compressive stress — the exact inversion of
/// [`plate_euler_stress_mpa`].
pub fn plate_buckling_thickness_mm(
    short_span_mm: f64,
    long_span_mm: f64,
    applied_compression_mpa: f64,
    youngs_modulus_mpa: f64,
    poissons_ratio: f64,
) -> Result<f64, ScantlingError> {
    if !(applied_compression_mpa.is_finite() && applied_compression_mpa >= 0.0) {
        return Err(ScantlingError::InvalidInput);
    }
    let (k_asp, _) = aspect_factor(short_span_mm, long_span_mm);
    Ok(short_span_mm
        * (12.0 * (1.0 - poissons_ratio * poissons_ratio) * applied_compression_mpa
            / (k_asp * std::f64::consts::PI * std::f64::consts::PI * youngs_modulus_mpa))
            .sqrt())
}

/// The half-wave count `m = ceil(a/b)` and the aspect factor
/// `(m·b/a + a/(m·b))²` — 4.0 for square panels and for long panels at
/// the half-wave resonance, larger between resonances.
fn aspect_factor(short_span_mm: f64, long_span_mm: f64) -> (f64, u32) {
    let ratio = long_span_mm / short_span_mm;
    let m = (ratio.ceil() as u32).max(1);
    let mf = m as f64;
    let k = (mf * short_span_mm / long_span_mm + long_span_mm / (mf * short_span_mm)).powi(2);
    (k, m)
}

/// Combined local plate scantling outcome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPlateScantling {
    /// Net thickness from the lateral-pressure bending leg, mm.
    pub bending_net_mm: f64,
    /// Net thickness from the Euler buckling leg, mm (`None` without a
    /// compressive demand).
    pub buckling_net_mm: Option<f64>,
    /// Euler stress at the governing thickness, MPa (equals the applied
    /// compression when buckling governs).
    pub euler_stress_mpa: Option<f64>,
    /// Governing net thickness, mm.
    pub governing_net_mm: f64,
    /// Governing thickness plus the corrosion addition, mm.
    pub with_corrosion_mm: f64,
    /// Which leg governs.
    pub mode: ScantlingMode,
}

/// Which leg of the local scantling check governs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScantlingMode {
    /// Lateral-pressure bending.
    Bending,
    /// Euler plate buckling under in-plane compression.
    Buckling,
}

/// Inputs for [`local_plate_scantling`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPlateScantlingInput {
    /// Short panel span: stiffener spacing, m.
    pub spacing_m: f64,
    /// Long panel span for the buckling leg, m (`None` skips buckling).
    pub long_span_m: Option<f64>,
    /// Design lateral pressure, kN/m².
    pub pressure_kn_m2: f64,
    /// Allowable bending stress for the load case (rule table input),
    /// N/mm².
    pub allowable_bending_mpa: f64,
    /// Demand-side material factor
    /// ([`high_strength_factor`](crate::high_strength_factor));
    /// 1.0 when the allowable stress already includes the material.
    pub material_factor_k: f64,
    /// Rule boundary factor (CSR 1.2 for load-carrying plates; 1.0 =
    /// plain clamped slab).
    pub boundary_factor: f64,
    /// Corrosion addition on top of the net scantling, mm.
    pub corrosion_addition_mm: f64,
    /// Applied in-plane compressive stress for the buckling leg, N/mm².
    pub applied_compression_mpa: Option<f64>,
    /// Young's modulus, GPa (206 for steel).
    pub youngs_modulus_gpa: f64,
}

/// Combined local plate scantling: the lateral-pressure bending leg
/// always runs; the Euler buckling leg runs when both a long span and
/// a compressive demand are supplied. The governing thickness is the
/// larger of the two legs plus the corrosion addition.
///
/// # Errors
///
/// [`ScantlingError`] on non-physical inputs, and
/// `BucklingNeedsLongSpan` when a compressive demand is given without
/// the panel's long span.
pub fn local_plate_scantling(
    input: LocalPlateScantlingInput,
) -> Result<LocalPlateScantling, ScantlingError> {
    if !(input.spacing_m.is_finite() && input.spacing_m > 0.0)
        || !(input.pressure_kn_m2.is_finite() && input.pressure_kn_m2 >= 0.0)
        || !(input.allowable_bending_mpa.is_finite() && input.allowable_bending_mpa > 0.0)
        || !(input.material_factor_k.is_finite() && input.material_factor_k > 0.0)
        || !(input.boundary_factor.is_finite() && input.boundary_factor > 0.0)
        || !(input.corrosion_addition_mm.is_finite() && input.corrosion_addition_mm >= 0.0)
        || !(input.youngs_modulus_gpa.is_finite() && input.youngs_modulus_gpa > 0.0)
    {
        return Err(ScantlingError::InvalidInput);
    }
    if let Some(sigma) = input.applied_compression_mpa {
        if !sigma.is_finite() || sigma < 0.0 {
            return Err(ScantlingError::InvalidInput);
        }
        if input.long_span_m.is_none() {
            return Err(ScantlingError::BucklingNeedsLongSpan);
        }
    }
    if let Some(long) = input.long_span_m {
        if !long.is_finite() || long < input.spacing_m {
            return Err(ScantlingError::InvalidInput);
        }
    }
    let bending = slab_bending_thickness_mm(
        input.spacing_m,
        input.pressure_kn_m2,
        input.allowable_bending_mpa,
        input.material_factor_k,
        input.boundary_factor,
    );
    let buckling = match (input.long_span_m, input.applied_compression_mpa) {
        (Some(long), Some(sigma)) if sigma > 0.0 => Some(plate_buckling_thickness_mm(
            input.spacing_m * 1000.0,
            long * 1000.0,
            sigma,
            input.youngs_modulus_gpa * 1000.0,
            0.3,
        )?),
        _ => None,
    };
    let governing = buckling.map_or(bending, |b| b.max(bending));
    let mode = match buckling {
        Some(b) if b > bending => ScantlingMode::Buckling,
        _ => ScantlingMode::Bending,
    };
    // Euler stress at the governing thickness (identity: equals the
    // applied compression when the buckling leg governs).
    let euler = match (input.long_span_m, input.applied_compression_mpa) {
        (Some(long), Some(_)) => Some(plate_euler_stress_mpa(
            governing,
            input.spacing_m * 1000.0,
            long * 1000.0,
            input.youngs_modulus_gpa * 1000.0,
            0.3,
        )?),
        _ => None,
    };
    Ok(LocalPlateScantling {
        bending_net_mm: bending,
        buckling_net_mm: buckling,
        euler_stress_mpa: euler,
        governing_net_mm: governing,
        with_corrosion_mm: governing + input.corrosion_addition_mm,
        mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::high_strength_factor;

    /// The slab constant is the clamped-plate relation in rule units:
    /// the returned thickness reproduces sigma = 0.5 p s^2 / t^2
    /// exactly, and s = 1 m, p = 1 kN/m2, sigma = 1 MPa gives 22.36 mm.
    #[test]
    fn slab_formula_round_trips_the_clamped_plate_relation() {
        let t = slab_bending_thickness_mm(1.0, 1.0, 1.0, 1.0, 1.0);
        assert!((t - 22.360_679_775_0).abs() < 1e-9);
        for &(s, p, sigma, k, alpha) in &[
            (0.8_f64, 100.0, 100.0, 1.0, 1.0),
            (0.7, 250.0, 180.0, 0.78, 1.2),
            (1.1, 45.0, 160.0, 0.91, 1.0),
        ] {
            let t = slab_bending_thickness_mm(s, p, sigma, k, alpha);
            // Back out the boundary-factor scaling, then the plate's
            // clamped-slab stress must reproduce sigma / k exactly
            // (t in m, p in N/m2, sigma in N/m2).
            let t0_m = t / 1000.0 / alpha;
            let sigma_check = 0.5 * (p * 1000.0) * s * s / (t0_m * t0_m) / 1e6;
            assert!(
                (sigma_check - sigma / k).abs() < 1e-9 * sigma,
                "s {s} p {p}: {sigma_check} vs {sigma}"
            );
        }
    }

    /// Scaling identities: thickness is linear in spacing and boundary
    /// factor, and in the square roots of pressure and 1/(k·sigma).
    #[test]
    fn slab_scaling_identities() {
        let base = slab_bending_thickness_mm(0.8, 100.0, 150.0, 1.0, 1.0);
        assert!(
            (slab_bending_thickness_mm(1.6, 100.0, 150.0, 1.0, 1.0) / base - 2.0).abs() < 1e-12
        );
        assert!(
            (slab_bending_thickness_mm(0.8, 400.0, 150.0, 1.0, 1.0) / base - 2.0).abs() < 1e-12
        );
        assert!(
            (slab_bending_thickness_mm(0.8, 100.0, 600.0, 1.0, 1.0) / base - 0.5).abs() < 1e-12
        );
        // AH36 (k = 0.78) thins the plate by sqrt(0.78).
        let k36 = high_strength_factor("AH36").unwrap();
        let ah36 = slab_bending_thickness_mm(0.8, 100.0, 150.0, k36, 1.0);
        assert!((ah36 / base - k36.sqrt()).abs() < 1e-12);
        assert!(
            (slab_bending_thickness_mm(0.8, 100.0, 150.0, 1.0, 1.2) / base - 1.2).abs() < 1e-12
        );
    }

    /// The aspect factor hits the classical value 4 at the square panel
    /// and at every half-wave resonance, and the Euler stress matches a
    /// fully hand-computed value.
    #[test]
    fn euler_stress_matches_hand_values() {
        // Square panel: m = 1, k = (1 + 1)^2 = 4.
        let (k, m) = aspect_factor(800.0, 800.0);
        assert_eq!(m, 1);
        assert!((k - 4.0).abs() < 1e-12);
        // Long panel 4:1: m = 4, k = (1 + 1)^2 = 4.
        let (k, m) = aspect_factor(800.0, 3200.0);
        assert_eq!(m, 4);
        assert!((k - 4.0).abs() < 1e-12);
        // 2.5:1: m = 3, k = (3/2.5 + 2.5/3)^2 = 4.1344...
        let (k, m) = aspect_factor(800.0, 2000.0);
        assert_eq!(m, 3);
        let expected_k: f64 = (3.0_f64 / 2.5 + 2.5 / 3.0).powi(2);
        assert!((k - expected_k).abs() < 1e-12);

        // Hand value: b = 800, a = 3200, t = 10, E = 206 GPa, nu = 0.3:
        // sigma_E = 4 pi^2 206000 100 / (12 0.91 640000) = 116.4 MPa.
        let sigma = plate_euler_stress_mpa(10.0, 800.0, 3200.0, 206_000.0, 0.3).unwrap();
        let expected = 4.0 * std::f64::consts::PI * std::f64::consts::PI * 206_000.0 * 100.0
            / (12.0 * 0.91 * 640_000.0);
        assert!((sigma - expected).abs() < 1e-9 * expected);
        assert!((expected - 116.36).abs() < 0.05, "hand anchor {expected}");
    }

    /// Euler scaling: doubling thickness quadruples sigma_E; the
    /// required thickness is the exact inversion (sigma_E(t_req) =
    /// sigma_applied).
    #[test]
    fn euler_scaling_and_inversion_identity() {
        let s = plate_euler_stress_mpa(10.0, 800.0, 3200.0, 206_000.0, 0.3).unwrap();
        let doubled = plate_euler_stress_mpa(20.0, 800.0, 3200.0, 206_000.0, 0.3).unwrap();
        assert!((doubled / s - 4.0).abs() < 1e-12);

        for applied in [50.0_f64, 116.36, 200.0] {
            let t_req =
                plate_buckling_thickness_mm(800.0, 3200.0, applied, 206_000.0, 0.3).unwrap();
            let back = plate_euler_stress_mpa(t_req, 800.0, 3200.0, 206_000.0, 0.3).unwrap();
            assert!(
                (back - applied).abs() < 1e-9 * applied,
                "inversion: {back} vs {applied}"
            );
        }
    }

    /// The combined check: bending alone governs without a compressive
    /// demand; a heavy compression flips the mode and the reported
    /// Euler stress at the governing thickness equals the demand;
    /// corrosion adds linearly; bad inputs are typed errors.
    #[test]
    fn combined_local_scantling_behaviour() {
        let input = LocalPlateScantlingInput {
            spacing_m: 0.8,
            long_span_m: None,
            pressure_kn_m2: 100.0,
            allowable_bending_mpa: 150.0,
            material_factor_k: 1.0,
            boundary_factor: 1.2,
            corrosion_addition_mm: 1.5,
            applied_compression_mpa: None,
            youngs_modulus_gpa: 206.0,
        };
        let bending_only = local_plate_scantling(input).unwrap();
        assert_eq!(bending_only.mode, ScantlingMode::Bending);
        assert!(bending_only.buckling_net_mm.is_none());
        assert!(bending_only.euler_stress_mpa.is_none());
        let bending = slab_bending_thickness_mm(0.8, 100.0, 150.0, 1.0, 1.2);
        assert!((bending_only.governing_net_mm - bending).abs() < 1e-12);
        assert!((bending_only.with_corrosion_mm - (bending + 1.5)).abs() < 1e-12);

        // A compressive demand below the bending thickness's Euler
        // capacity keeps bending in charge and reports the margin.
        let mut margin = input;
        margin.long_span_m = Some(3.2);
        margin.applied_compression_mpa = Some(30.0);
        let with_margin = local_plate_scantling(margin).unwrap();
        assert_eq!(with_margin.mode, ScantlingMode::Bending);
        let t_req = plate_buckling_thickness_mm(800.0, 3200.0, 30.0, 206_000.0, 0.3).unwrap();
        assert!(t_req < bending, "30 MPa buckles a thinner plate");
        // Euler stress at the (thicker) bending thickness: a margin
        // above the applied demand.
        assert!(with_margin.euler_stress_mpa.unwrap() > 30.0);

        // A heavy compressive demand (500 MPa needs ~20.7 mm vs the
        // 17.5 mm bending thickness) flips the mode; the inversion
        // identity pins the reported Euler stress at the demand.
        let mut heavy = margin;
        heavy.applied_compression_mpa = Some(500.0);
        let buckled = local_plate_scantling(heavy).unwrap();
        assert_eq!(buckled.mode, ScantlingMode::Buckling);
        let t_heavy = plate_buckling_thickness_mm(800.0, 3200.0, 500.0, 206_000.0, 0.3).unwrap();
        assert!((buckled.buckling_net_mm.unwrap() - t_heavy).abs() < 1e-12);
        assert!((buckled.euler_stress_mpa.unwrap() - 500.0).abs() < 1e-6);
        assert!(buckled.governing_net_mm >= buckled.bending_net_mm);

        // Errors.
        let mut bad = input;
        bad.spacing_m = -1.0;
        assert_eq!(
            local_plate_scantling(bad),
            Err(ScantlingError::InvalidInput)
        );
        let mut no_span = input;
        no_span.long_span_m = None;
        no_span.applied_compression_mpa = Some(50.0);
        assert_eq!(
            local_plate_scantling(no_span),
            Err(ScantlingError::BucklingNeedsLongSpan)
        );
        let mut swapped = input;
        swapped.long_span_m = Some(0.5); // shorter than the spacing
        assert_eq!(
            local_plate_scantling(swapped),
            Err(ScantlingError::InvalidInput)
        );
    }
}
