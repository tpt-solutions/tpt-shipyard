//! Cylindrical shell screening — the pressure-hull slice of the
//! "curved shell" roadmap item (review 7H).
//!
//! Thin-wall closed forms plus the classical Windenburg–Trilling
//! external-pressure buckling pressure:
//!
//! - hoop (circumferential) stress `sigma_theta = p·d/(2t)`,
//! - longitudinal stress `sigma_L = p·d/(4t)` (closed ends),
//! - `p_cr = 2.42·E/(1 - nu^2)^0.75 · (t/d)^2.5` — the 1934
//!   Windenburg & Trilling elastic buckling formula for cylinders under
//!   external lateral pressure, the standard submarine pressure-hull
//!   screening value (elastic buckling; no impression imperfection or
//!   ring-stiffening knockdowns).
//!
//! Full curved-shell finite elements (general shells, ring stiffeners,
//! inelastic knockdown) remain roadmap; this module gives the
//! design-early pressure-hull answer that every later check refines.

/// Errors from the shell screening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellError {
    /// A dimension or property is not finite and physical.
    InvalidInput,
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("shell inputs must be finite and physical")
    }
}

impl std::error::Error for ShellError {}

/// The cylindrical-shell screening outcome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CylinderShellScreen {
    /// Hoop (circumferential) compressive stress `p·d/(2t)`, MPa.
    pub hoop_stress_mpa: f64,
    /// Longitudinal stress `p·d/(4t)` (closed ends), MPa.
    pub longitudinal_stress_mpa: f64,
    /// Windenburg–Trilling elastic buckling pressure, MPa.
    pub buckling_pressure_mpa: f64,
    /// Applied pressure over buckling pressure (<= 1/sf passes).
    pub buckling_utilization: f64,
    /// The depth at which the sea pressure reaches the buckling
    /// pressure, m (salt water).
    pub crush_depth_m: f64,
    /// True when the hoop stress stays within the allowable and the
    /// pressure carries the requested buckling safety factor.
    pub passes: bool,
}

/// Screens a circular cylindrical pressure hull under uniform external
/// lateral pressure: thin-wall hoop/longitudinal stresses and the
/// Windenburg–Trilling elastic buckling pressure, against a allowable
/// stress and a buckling safety factor (practice uses 1.5 on `p_cr`).
///
/// # Errors
///
/// [`ShellError::InvalidInput`] on non-physical dimensions,
/// non-finite inputs, or a safety factor not greater than 1.
pub fn cylinder_shell_screen(
    external_pressure_mpa: f64,
    diameter_m: f64,
    thickness_m: f64,
    youngs_modulus_mpa: f64,
    poissons_ratio: f64,
    allowable_stress_mpa: f64,
    buckling_safety_factor: f64,
) -> Result<CylinderShellScreen, ShellError> {
    if !(external_pressure_mpa.is_finite() && external_pressure_mpa > 0.0)
        || !(diameter_m.is_finite() && diameter_m > 0.0)
        || !(thickness_m.is_finite() && thickness_m > 0.0 && thickness_m < diameter_m / 2.0)
        || !(youngs_modulus_mpa.is_finite() && youngs_modulus_mpa > 0.0)
        || !(poissons_ratio.is_finite() && poissons_ratio > -1.0 && poissons_ratio < 0.5)
        || !(allowable_stress_mpa.is_finite() && allowable_stress_mpa > 0.0)
        || !(buckling_safety_factor.is_finite() && buckling_safety_factor >= 1.0)
    {
        return Err(ShellError::InvalidInput);
    }
    let hoop = external_pressure_mpa * diameter_m / (2.0 * thickness_m);
    let longitudinal = hoop / 2.0;
    // Windenburg-Trilling: p_cr = 2.42 E (t/d)^2.5 / (1 - nu^2)^0.75.
    let p_cr = 2.42 * youngs_modulus_mpa * (thickness_m / diameter_m).powf(2.5)
        / (1.0 - poissons_ratio * poissons_ratio).powf(0.75);
    // Salt-water crush depth: rho g z = p_cr, rho g = 10.045e-3 MPa/m.
    let crush_depth_m = p_cr / (1.025 * 9.81) * 1000.0;
    let buckling_utilization = external_pressure_mpa / p_cr;
    Ok(CylinderShellScreen {
        hoop_stress_mpa: hoop,
        longitudinal_stress_mpa: longitudinal,
        buckling_pressure_mpa: p_cr,
        buckling_utilization,
        crush_depth_m,
        passes: hoop <= allowable_stress_mpa
            && buckling_utilization <= 1.0 / buckling_safety_factor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand values: E 210 GPa, nu 0.3, t/d = 1/200 gives
    /// p_cr = 0.964 MPa; hoop = p/(2 t/d), long = hoop/2; the crush
    /// depth follows from rho g z = p_cr.
    #[test]
    fn windenburg_trilling_matches_hand_values() {
        // t/d = 0.005: p_cr = 2.42 * 210000 * (0.005)^2.5 / 0.91^0.75
        // = 0.9642 MPa.
        let screen = cylinder_shell_screen(0.5, 2.0, 0.010, 210_000.0, 0.3, 200.0, 1.5).unwrap();
        let p_cr = 2.42 * 210_000.0 * 0.005_f64.powf(2.5) / 0.91_f64.powf(0.75);
        assert!((screen.buckling_pressure_mpa - p_cr).abs() < 1e-12);
        assert!((p_cr - 0.9642).abs() < 0.001, "hand anchor {p_cr}");
        // Hoop and longitudinal closed forms.
        assert!((screen.hoop_stress_mpa - 0.5 * 2.0 / (2.0 * 0.010)).abs() < 1e-12);
        assert!((screen.longitudinal_stress_mpa - screen.hoop_stress_mpa / 2.0).abs() < 1e-12);
        // Crush depth: rho g z = p_cr with rho g = 1025 * 9.81 N/m3.
        let depth = p_cr * 1000.0 / (1.025 * 9.81);
        assert!((screen.crush_depth_m - depth).abs() < 1e-9);
        assert!((depth - 95.8).abs() < 0.2, "hand anchor {depth}");
        // 0.5 MPa against 0.964 MPa with SF 1.5: 0.5 <= 0.643 passes.
        assert!(screen.passes);
    }

    /// Scaling identities: (t/d)^2.5 buckling (doubling thickness gives
    /// 2^2.5 = 5.657x), the 2:1 hoop/longitudinal ratio, and the
    /// safety-factor gate (a pressure between Pcr/1.0 and Pcr/1.5
    /// passes at SF 1 but fails at 1.5).
    #[test]
    fn shell_scaling_and_safety_gate() {
        let base = cylinder_shell_screen(0.5, 2.0, 0.010, 210_000.0, 0.3, 200.0, 1.0).unwrap();
        let thick = cylinder_shell_screen(0.5, 2.0, 0.020, 210_000.0, 0.3, 200.0, 1.0).unwrap();
        assert!(
            (thick.buckling_pressure_mpa / base.buckling_pressure_mpa - 2.0_f64.powf(2.5)).abs()
                < 1e-9
        );
        assert!(
            (thick.hoop_stress_mpa / base.hoop_stress_mpa - 0.5).abs() < 1e-12,
            "double thickness halves the hoop stress"
        );
        // SF gate: p between Pcr and Pcr/1.5.
        let marginal = cylinder_shell_screen(
            base.buckling_pressure_mpa * 0.8,
            2.0,
            0.010,
            210_000.0,
            0.3,
            200.0,
            1.0,
        )
        .unwrap();
        assert!(marginal.passes, "passes at SF 1");
        let sf15 = cylinder_shell_screen(
            base.buckling_pressure_mpa * 0.8,
            2.0,
            0.010,
            210_000.0,
            0.3,
            200.0,
            1.5,
        )
        .unwrap();
        assert!(!sf15.passes, "fails at SF 1.5");
        assert!((sf15.buckling_utilization - 0.8).abs() < 1e-12);

        // Errors: thickness past half-diameter, SF below 1, negative
        // pressure.
        assert_eq!(
            cylinder_shell_screen(0.5, 2.0, 1.5, 210_000.0, 0.3, 200.0, 1.0),
            Err(ShellError::InvalidInput)
        );
        assert_eq!(
            cylinder_shell_screen(0.5, 2.0, 0.010, 210_000.0, 0.3, 200.0, 0.9),
            Err(ShellError::InvalidInput)
        );
        assert_eq!(
            cylinder_shell_screen(-0.1, 2.0, 0.010, 210_000.0, 0.3, 200.0, 1.0),
            Err(ShellError::InvalidInput)
        );
    }
}
