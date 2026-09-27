//! Structural design without launch constraints.
//!
//! A structure assembled in orbit never rides a rocket: no fairing diameter,
//! no Max-Q, no ascent vibration, no 1-g handling loads.
//! [`SpaceStructuralDesigner`] quantifies that design freedom and implements
//! the load cases that *do* govern in space (RFC 0005):
//!
//! - **rotation** — hoop stress in a spinning habitat ring,
//!   `sigma = rho * omega^2 * r^2`;
//! - **thermal cycling** — orbit day/night strain ranges and fatigue life
//!   (Coffin-Manson form);
//! - **micrometeoroid protection** — Whipple-shield sizing from the
//!   required particle protection;
//! - **launch-constraint removal** — the `no_launch_constraint` statement
//!   of freedom used by planners.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::Material;
//! use tpt_yard_space_structural::SpaceStructuralDesigner;
//!
//! let designer = SpaceStructuralDesigner::new(Material::aa5083());
//! // 100 m radius at 1 rpm — the classic ring-station class.
//! let stress = designer.rotating_habitat_stress(100.0, 1.0, 1.0e6);
//! let omega = 2.0 * std::f64::consts::PI / 60.0;
//! let rho = Material::aa5083().density_kg_m3; // 2660 kg/m3
//! let expected = rho * omega * omega * 100.0 * 100.0; // ~0.29 MPa
//! assert!((stress.hoop_stress_mpa - expected / 1e6).abs() < 1e-9);
//! ```

use tpt_yard_core::Material;

/// The space environment a structure must survive.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceEnvironment {
    /// Orbit designation (e.g. "LEO 400 km", "EML1").
    pub orbit: String,
    /// Thermal cycling between eclipses.
    pub thermal_cycling: ThermalCycleEnv,
    /// Micrometeoroid flux for particles >= 1 mm, impacts per m² per year.
    pub micrometeoroid_flux: f64,
    /// Radiation description (e.g. "trapped belt, 5 yr design").
    pub radiation: String,
    /// Atomic oxygen present (LEO only) — erodes polymers and some coatings.
    pub atomic_oxygen: bool,
}

/// Thermal-cycling environment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalCycleEnv {
    /// Hot-case temperature, °C.
    pub hot_c: f64,
    /// Cold-case (eclipse) temperature, °C.
    pub cold_c: f64,
    /// Orbit period, minutes.
    pub period_min: f64,
}

impl Default for SpaceEnvironment {
    fn default() -> Self {
        SpaceEnvironment {
            orbit: "LEO 400 km".into(),
            thermal_cycling: ThermalCycleEnv {
                hot_c: 90.0,
                cold_c: -120.0,
                period_min: 92.0,
            },
            micrometeoroid_flux: 1.0e-3,
            radiation: "trapped belt".into(),
            atomic_oxygen: true,
        }
    }
}

/// The design freedom gained by assembling in space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesignFreedom {
    /// No physical size cap (theoretical), m.
    pub max_dimension_m: f64,
    /// What constrains the outer mould line.
    pub shape_constraint: ShapeConstraint,
    /// False when assembled in orbit — no launch ride.
    pub launch_required: bool,
}

/// Outer-mould-line constraints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeConstraint {
    /// Arbitrary geometry — the orbital case.
    None,
    /// Must survive atmosphere (ascent/descent).
    Aerodynamic,
    /// Must fit a payload fairing.
    FairingLimit {
        /// Fairing diameter, m.
        diameter_m: f64,
    },
}

/// Result of the rotating-ring stress analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotatingStressResult {
    /// Hoop (circumferential) stress, MPa.
    pub hoop_stress_mpa: f64,
    /// Angular velocity, rad/s.
    pub angular_velocity_rad_s: f64,
    /// Radial growth of the ring from elastic strain, m.
    pub radial_growth_m: f64,
    /// `hoop_stress / yield` with the designer's material.
    pub utilization: f64,
}

/// Thermal-fatigue assessment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FatigueResult {
    /// Total strain range per cycle (thermal), dimensionless.
    pub strain_range: f64,
    /// Estimated cycles to failure (Coffin-Manson form).
    pub cycles_to_failure: f64,
    /// Used life fraction for the demanded cycles (Miner's rule).
    pub life_fraction: f64,
    /// `life_fraction <= 1` passes the design check.
    pub safe: bool,
}

/// Whipple-shield sizing result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShieldDesign {
    /// Bumper (outer sacrificial) thickness, mm.
    pub bumper_thickness_mm: f64,
    /// Standoff between bumper and pressure wall, m.
    pub standoff_m: f64,
    /// Rear (pressure) wall thickness, mm.
    pub rear_wall_thickness_mm: f64,
    /// Protected particle diameter, mm.
    pub protected_diameter_mm: f64,
    /// Total areal density of the shield package, kg/m².
    pub areal_density_kg_m2: f64,
}

/// Design checks for space structures (RFC 0005).
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceStructuralDesigner {
    /// Space environment.
    pub environment: SpaceEnvironment,
    /// Primary structural material.
    pub material: Material,
}

impl SpaceStructuralDesigner {
    /// Creates a designer for the default LEO environment.
    pub fn new(material: Material) -> Self {
        Self {
            environment: SpaceEnvironment::default(),
            material,
        }
    }

    /// Sets the environment.
    #[must_use]
    pub fn with_environment(mut self, environment: SpaceEnvironment) -> Self {
        self.environment = environment;
        self
    }

    /// The design freedom statement: assembled in orbit, the structure
    /// escapes fairing, aerodynamic and 1-g assembly limits entirely.
    pub fn no_launch_constraint(&self) -> DesignFreedom {
        DesignFreedom {
            max_dimension_m: f64::INFINITY,
            shape_constraint: ShapeConstraint::None,
            launch_required: false,
        }
    }

    /// Hoop stress in a thin rotating ring (the habitat hull):
    ///
    /// ```text
    /// sigma = rho * omega^2 * r^2,   omega = 2 pi rpm / 60
    /// ```
    ///
    /// with `rho` the structural material density. The radial growth is the
    /// elastic strain integrated around the ring: `u = sigma * r / E`.
    pub fn rotating_habitat_stress(
        &self,
        radius_m: f64,
        rotation_rpm: f64,
        _habitat_mass_kg: f64,
    ) -> RotatingStressResult {
        let omega = 2.0 * std::f64::consts::PI * rotation_rpm / 60.0;
        let sigma_pa = self.material.density_kg_m3 * omega * omega * radius_m * radius_m;
        let sigma_mpa = sigma_pa / 1e6;
        let strain = sigma_pa / (self.material.youngs_modulus_gpa * 1e9);
        RotatingStressResult {
            hoop_stress_mpa: sigma_mpa,
            angular_velocity_rad_s: omega,
            radial_growth_m: strain * radius_m,
            utilization: sigma_mpa / self.material.yield_mpa,
        }
    }

    /// Thermal-cycling fatigue over `cycles` eclipses.
    ///
    /// Strain range `dE = alpha * (T_hot - T_cold)`; life follows the
    /// Coffin-Manson form documented in RFC 0005:
    /// `N_f = 0.5 * (dE / (3.5 * sigma_u / E))^(-1/0.12)`, with the life
    /// fraction by Miner's rule.
    pub fn thermal_cycling_fatigue(&self, cycles: u64) -> FatigueResult {
        let d_t = self.environment.thermal_cycling.hot_c - self.environment.thermal_cycling.cold_c;
        let strain_range = self.material.thermal_expansion_1_k * d_t;
        let ductility_coeff =
            3.5 * (self.material.ultimate_mpa * 1e6) / (self.material.youngs_modulus_gpa * 1e9);
        let n_f = if strain_range > 0.0 {
            0.5 * (strain_range / ductility_coeff).powf(-1.0 / 0.12)
        } else {
            f64::INFINITY
        };
        let life_fraction = cycles as f64 / n_f;
        FatigueResult {
            strain_range,
            cycles_to_failure: n_f,
            life_fraction,
            safe: life_fraction <= 1.0,
        }
    }

    /// Whipple-shield sizing for protecting against particles up to
    /// `required_protection_mm`.
    ///
    /// Adapted from NASA ship-set practices (documented in RFC 0005):
    /// bumper thickness scales with the protected diameter, standoff with
    /// the debris-cloud spread, rear wall at half the bumper scale.
    pub fn micrometeoroid_shielding(&self, required_protection_mm: f64) -> ShieldDesign {
        let d = required_protection_mm;
        let bumper = (d / 6.0).max(0.3);
        let standoff = (d / 10.0).max(0.05);
        let rear = (d / 12.0).max(0.4);
        let areal = (bumper + rear) / 1000.0 * self.material.density_kg_m3;
        ShieldDesign {
            bumper_thickness_mm: bumper,
            standoff_m: standoff,
            rear_wall_thickness_mm: rear,
            protected_diameter_mm: d,
            areal_density_kg_m2: areal,
        }
    }

    /// Expected micrometeoroid impacts per year on an area, given the
    /// environment flux.
    pub fn expected_impacts_per_year(&self, area_m2: f64) -> f64 {
        self.environment.micrometeoroid_flux * area_m2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verification: sigma = rho * omega^2 * r^2 for a thin ring — the
    /// spec's reference case (2 rpm, 100 m, aluminium).
    #[test]
    fn test_rotating_habitat_stress() {
        let designer = SpaceStructuralDesigner::new(Material::aa5083());
        let omega = 2.0 * std::f64::consts::PI * 2.0 / 60.0; // 2 rpm
        let r = 100.0;
        let rho = Material::aa5083().density_kg_m3;
        let expected = rho * omega * omega * r * r; // Pa
        let result = designer.rotating_habitat_stress(r, 2.0, 1_000_000.0);
        assert!(
            (result.hoop_stress_mpa - expected / 1e6).abs() < 1e-6,
            "sigma {} vs {}",
            result.hoop_stress_mpa,
            expected / 1e6
        );
        assert!((result.angular_velocity_rad_s - omega).abs() < 1e-12);
        // Stress grows with the square of rpm and radius.
        let faster = designer.rotating_habitat_stress(r, 4.0, 1.0e6);
        assert!((faster.hoop_stress_mpa / result.hoop_stress_mpa - 4.0).abs() < 1e-9);
        let bigger = designer.rotating_habitat_stress(200.0, 2.0, 1.0e6);
        assert!((bigger.hoop_stress_mpa / result.hoop_stress_mpa - 4.0).abs() < 1e-9);
    }

    #[test]
    fn aluminium_ring_at_one_rpm_is_comfortable() {
        let designer = SpaceStructuralDesigner::new(Material::aa5083());
        let stress = designer.rotating_habitat_stress(100.0, 1.0, 1e6);
        // ~3 MPa: trivially below AA5083 yield (215 MPa).
        assert!(stress.hoop_stress_mpa < 5.0);
        assert!(stress.utilization < 0.05);
        // Radial growth is small but measurable.
        assert!(stress.radial_growth_m > 0.0 && stress.radial_growth_m < 0.01);
    }

    #[test]
    fn steel_at_high_rpm_overstresses() {
        let designer = SpaceStructuralDesigner::new(Material::ah36());
        // 300 m at 8 rpm: sigma = 7850 * (0.8378)^2 * 9e4 ~ 494 MPa > 355.
        let stress = designer.rotating_habitat_stress(300.0, 8.0, 1e6);
        assert!(stress.utilization > 1.0, "util {}", stress.utilization);
    }

    #[test]
    fn thermal_fatigue_orders_by_material() {
        let env = SpaceEnvironment::default(); // 210 °C swing
        let alu = SpaceStructuralDesigner::new(Material::aa5083()).with_environment(env.clone());
        let steel = SpaceStructuralDesigner::new(Material::ah36()).with_environment(env.clone());
        // 2,000 eclipses (~6 months in LEO): both materials survive, but
        // the higher-expansion aluminium burns life ~2x faster.
        let f_alu = alu.thermal_cycling_fatigue(2_000);
        let f_steel = steel.thermal_cycling_fatigue(2_000);
        assert!(f_alu.strain_range > f_steel.strain_range);
        assert!(f_alu.cycles_to_failure < f_steel.cycles_to_failure);
        assert!(f_alu.safe && f_steel.safe);
        // At 100k cycles neither survives a 210 degree swing.
        assert!(!alu.thermal_cycling_fatigue(100_000).safe);
    }

    #[test]
    fn shielding_scales_with_protection() {
        let designer = SpaceStructuralDesigner::new(Material::aa5083());
        let s1 = designer.micrometeoroid_shielding(3.0); // 3 mm
        let s2 = designer.micrometeoroid_shielding(10.0);
        assert!(s2.bumper_thickness_mm > s1.bumper_thickness_mm);
        assert!(s2.standoff_m > s1.standoff_m);
        assert!(s2.areal_density_kg_m2 > s1.areal_density_kg_m2);
        // NASA-style ratios: standoff ~ d/10, rear ~ d/12.
        assert!((s2.standoff_m - 1.0).abs() < 1e-9);
        assert!((s2.rear_wall_thickness_mm - 10.0 / 12.0).abs() < 1e-9);
    }

    #[test]
    fn launch_freedom_statement() {
        let designer = SpaceStructuralDesigner::new(Material::aa5083());
        let freedom = designer.no_launch_constraint();
        assert!(!freedom.launch_required);
        assert_eq!(freedom.shape_constraint, ShapeConstraint::None);
        assert!(freedom.max_dimension_m.is_infinite());
    }

    #[test]
    fn flux_integration() {
        let designer = SpaceStructuralDesigner::new(Material::aa5083());
        // 1000 m² at 1e-3 /m²/yr means 1 impact/yr expected.
        assert!((designer.expected_impacts_per_year(1000.0) - 1.0).abs() < 1e-12);
    }
}
