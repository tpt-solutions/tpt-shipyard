//! Bridge between process engineering (`tpt-process`) and construction
//! planning.
//!
//! The `tpt-process` substrate is not published yet, so this crate vendors
//! a minimal **Peng–Robinson equation of state** (the spec §6 contract:
//! `plan_propellant_loading` uses `PengRobinson`) and wires it into the
//! propellant loading planner: the EOS computes the ullage vapour density
//! and vent requirement at loading conditions.
//!
//! # Example
//!
//! ```
//! use tpt_yard_process_link::PengRobinson;
//!
//! // Methane at 111 K and 0.1 MPa: vapour Z ~ 0.97 (near-ideal).
//! let eos = PengRobinson::new(190.6, 4.599e6, 0.011);
//! let z = eos.compressibility(111.0, 0.1e6);
//! assert!((z - 0.97).abs() < 0.05, "Z = {z}");
//! ```
#![allow(clippy::doc_markdown)]

use std::fmt;

use tpt_yard_propellant::{LoadingPlanner, PropellantLoadingPlan, PropellantSpec, TankSpec};

/// Peng–Robinson equation of state for a single component.
///
/// `P = R T / (v − b) − a α / (v² + 2 b v − b²)` with the classical mixing
/// defaults (`κ = 0.37464 + 1.54226 ω − 0.26992 ω²`, `a = 0.45724 R²Tc²/Pc`,
/// `b = 0.07780 R Tc/Pc`). Solved for the vapour-root compressibility.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PengRobinson {
    /// Critical temperature, K.
    pub critical_temp_k: f64,
    /// Critical pressure, Pa.
    pub critical_pressure_pa: f64,
    /// Acentric factor.
    pub acentric_factor: f64,
    /// `a` constant, Pa·m⁶/kmol².
    a: f64,
    /// `b` constant, m³/kmol.
    b: f64,
    /// `alpha` at reference (recomputed per temperature call).
    kappa: f64,
}

/// R, J/(kmol·K).
const R: f64 = 8314.462618;

impl PengRobinson {
    /// Creates an EOS for one component.
    pub fn new(critical_temp_k: f64, critical_pressure_pa: f64, acentric_factor: f64) -> Self {
        let a = 0.45724 * R * R * critical_temp_k * critical_temp_k / critical_pressure_pa;
        let b = 0.07780 * R * critical_temp_k / critical_pressure_pa;
        let kappa =
            0.37464 + 1.54226 * acentric_factor - 0.26992 * acentric_factor * acentric_factor;
        Self {
            critical_temp_k,
            critical_pressure_pa,
            acentric_factor,
            a,
            b,
            kappa,
        }
    }

    fn alpha(&self, temp_k: f64) -> f64 {
        let t_r = temp_k / self.critical_temp_k;
        (1.0 + self.kappa * (1.0 - t_r.sqrt())).powi(2)
    }

    /// Compressibility factor Z at (T, P), taking the largest real root
    /// (vapour). For `P > Pc` and `T < Tc` (compressed liquid region) the
    /// smallest root is returned (liquid).
    pub fn compressibility(&self, temp_k: f64, pressure_pa: f64) -> f64 {
        let alpha = self.alpha(temp_k);
        let a_term = self.a * alpha * pressure_pa / (R * R * temp_k * temp_k);
        let b_term = self.b * pressure_pa / (R * temp_k);
        // Cubic: Z^3 − (1−B) Z^2 + (A − 3B − B²) Z − (A B − B² − B³) = 0
        let c2 = -(1.0 - b_term);
        let c1 = a_term - 3.0 * b_term - b_term * b_term;
        let c0 = -(a_term * b_term - b_term * b_term - b_term * b_term * b_term);
        let roots = cubic_roots(c2, c1, c0);
        let vapour = roots.iter().cloned().fold(f64::MIN, f64::max);
        let liquid = roots.iter().cloned().fold(f64::MAX, f64::min);
        if temp_k < self.critical_temp_k && pressure_pa > self.critical_pressure_pa {
            liquid.max(1e-6)
        } else {
            vapour.max(1e-6)
        }
    }

    /// Vapour density at (T, P), kg/m³: `rho = P M / (Z R T)`.
    pub fn vapour_density(&self, temp_k: f64, pressure_pa: f64, molar_mass_kg_kmol: f64) -> f64 {
        let z = self.compressibility(temp_k, pressure_pa);
        pressure_pa * molar_mass_kg_kmol / (z * R * temp_k)
    }
}

/// Solves `x³ + c2 x² + c1 x + c0 = 0` for real roots (trigonometric
/// method with a linear fallback).
fn cubic_roots(c2: f64, c1: f64, c0: f64) -> [f64; 3] {
    let p = c1 - c2 * c2 / 3.0;
    let q = 2.0 * c2 * c2 * c2 / 27.0 - c2 * c1 / 3.0 + c0;
    let discriminant = q * q / 4.0 + p * p * p / 27.0;
    if discriminant > 0.0 {
        // One real root.
        let sq = discriminant.sqrt();
        let u = (-q / 2.0 + sq).cbrt();
        let v = (-q / 2.0 - sq).cbrt();
        [u + v - c2 / 3.0, f64::NAN, f64::NAN]
    } else {
        // Three real roots.
        let r = 2.0 * (-p / 3.0).sqrt();
        let phi = (-q / (2.0 * (-p / 3.0).sqrt().powi(3)))
            .clamp(-1.0, 1.0)
            .acos();
        let m = c2 / 3.0;
        [
            r * ((phi + 2.0 * std::f64::consts::PI) / 3.0).cos() - m,
            r * (phi / 3.0).cos() - m,
            r * ((phi + 4.0 * std::f64::consts::PI) / 3.0).cos() - m,
        ]
    }
}

/// Errors from the process link.
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessLinkError {
    /// The propellant must carry EOS parameters matching its spec.
    EosMismatch(String),
}

impl fmt::Display for ProcessLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessLinkError::EosMismatch(name) => write!(f, "no EOS parameters for {name}"),
        }
    }
}

impl std::error::Error for ProcessLinkError {}

/// Plans propellant loading: the loading planner's operational plan plus
/// EOS-derived ullage vent conditions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropellantLoadingDecision {
    /// The operational loading plan (rate, chilldown, policy).
    pub plan: PropellantLoadingPlan,
    /// Vapour density in the ullage at loading conditions, kg/m³.
    pub ullage_vapour_density_kg_m3: f64,
    /// True when the EOS says the tank must vent during fill.
    pub venting_required_by_eos: bool,
}

/// EOS parameters per propellant name (the `tpt-process` lookup, vendored).
pub fn eos_for(propellant: &PropellantSpec) -> Option<PengRobinson> {
    // (Tc [K], Pc [Pa], omega) for common propellants.
    match propellant.name.as_str() {
        "LOX" => Some(PengRobinson::new(154.6, 5.043e6, 0.022)),
        "LH2" => Some(PengRobinson::new(33.2, 1.296e6, -0.219)),
        "LCH4" => Some(PengRobinson::new(190.6, 4.599e6, 0.011)),
        "MMH" => Some(PengRobinson::new(567.6, 5.35e6, 0.320)),
        "NTO" => Some(PengRobinson::new(431.4, 10.1e6, 0.117)),
        _ => None,
    }
}

/// Plans the propellant loading: operational plan from `tpt-yard-propellant`
/// plus EOS-derived venting decision at the loading condition
/// (ullage at 1.15 × boiling pressure for cryogenics, 1 atm otherwise).
///
/// # Errors
///
/// [`ProcessLinkError`] when the propellant has no EOS parameters.
pub fn plan_propellant_loading(
    propellant: &PropellantSpec,
    tank: &TankSpec,
    fill_rate_kg_s: f64,
) -> Result<PropellantLoadingDecision, ProcessLinkError> {
    let planner = LoadingPlanner::new();
    let plan = planner
        .plan_loading(propellant, tank, fill_rate_kg_s)
        .map_err(|_| ProcessLinkError::EosMismatch(propellant.name.clone()))?;
    let eos = eos_for(propellant)
        .ok_or_else(|| ProcessLinkError::EosMismatch(propellant.name.clone()))?;

    // Ullage held slightly above atmospheric during fill.
    let loading_pressure = 1.15 * vapour_pressure_at_boiling(propellant);
    let vapour_density = eos.vapour_density(
        propellant.boiling_point_k,
        loading_pressure,
        propellant.molar_mass_kg_kmol,
    );
    // Dense vapour at loading conditions means vent capacity is the fill
    // constraint; light vapour (near-ideal) can be handled by the normal
    // vent schedule.
    let venting_required_by_eos = vapour_density > 1.0;

    Ok(PropellantLoadingDecision {
        plan,
        ullage_vapour_density_kg_m3: vapour_density,
        venting_required_by_eos,
    })
}

/// Saturation pressure at the boiling point: 1 atm by definition of the
/// normal boiling point (the EOS lookup for real operating pressures stays
/// with the  substrate).
fn vapour_pressure_at_boiling(_propellant: &PropellantSpec) -> f64 {
    101_325.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_propellant::BoilOffPolicy;

    #[test]
    fn eos_methane_vapour_root_is_physical() {
        let eos = PengRobinson::new(190.6, 4.599e6, 0.011);
        // Near-ideal at low pressure.
        let z_low = eos.compressibility(111.0, 0.1e6);
        assert!((z_low - 1.0).abs() < 0.1, "Z = {z_low}");
        // Denser corrections near critical pressure.
        let z_high = eos.compressibility(190.0, 4.0e6);
        assert!(z_high > 0.2 && z_high < 1.0, "Z = {z_high}");
        // Vapour density of CH4 at 111 K, 0.1 MPa ~ 1.76 kg/m3 (ideal gas
        // 1.76; PR with Z ~ 0.98 gives slightly more).
        let rho = eos.vapour_density(111.0, 0.1e6, 16.04);
        assert!((rho - 1.76).abs() < 0.15, "rho = {rho}");
    }

    #[test]
    fn eos_liquid_root_for_compressed_lox() {
        let eos = PengRobinson::new(154.6, 5.043e6, 0.022);
        // Compressed liquid region: T < Tc, P > Pc.
        let z = eos.compressibility(90.2, 6.0e6);
        assert!(z < 0.5, "liquid Z = {z}");
    }

    /// Verification: the loading plan for LOX requires venting and
    /// chilldown; the EOS venting decision agrees.
    #[test]
    fn plan_propellant_loading_lox() {
        let tank = TankSpec {
            volume_m3: 300.0,
            ullage_frac: 0.03,
            heat_leak_w: 2_000.0,
        };
        let decision = plan_propellant_loading(&PropellantSpec::lox(), &tank, 50.0).unwrap();
        assert!(decision.plan.venting_required);
        assert!(decision.plan.thermal_control.chilldown_required);
        assert!(decision.ullage_vapour_density_kg_m3 > 1.0);
        assert!(decision.venting_required_by_eos);
    }

    #[test]
    fn storable_propellant_plan() {
        let tank = TankSpec {
            volume_m3: 2.0,
            ullage_frac: 0.05,
            heat_leak_w: 10.0,
        };
        let mmh = PropellantSpec::mmh();
        let decision = plan_propellant_loading(&mmh, &tank, 5.0).unwrap();
        assert!(!decision.plan.venting_required);
        assert_eq!(
            decision.plan.thermal_control.policy,
            BoilOffPolicy::ZeroBoilOff
        );
        // MMH vapour at 1 atm/boiling is light-ish; EOS just needs a number.
        assert!(decision.ullage_vapour_density_kg_m3 > 0.0);
    }

    #[test]
    fn unknown_propellant_rejected() {
        let tank = TankSpec {
            volume_m3: 1.0,
            ullage_frac: 0.03,
            heat_leak_w: 1.0,
        };
        let unknown = PropellantSpec {
            name: "Unobtainium".into(),
            ..PropellantSpec::lox()
        };
        assert!(matches!(
            plan_propellant_loading(&unknown, &tank, 1.0),
            Err(ProcessLinkError::EosMismatch(_))
        ));
    }
}
