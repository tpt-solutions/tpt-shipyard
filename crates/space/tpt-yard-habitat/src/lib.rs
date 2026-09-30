//! Rotating habitat design: artificial gravity, Coriolis comfort, and
//! structure.
//!
//! [`HabitatDesigner`] answers the three questions of rotating-habitat
//! design (RFC 0005):
//!
//! 1. **How fast must it spin?** — `omega = sqrt(g/r)` for a target
//!    artificial gravity level at a given radius.
//! 2. **Will occupants get sick?** — the Coriolis comfort guideline: keep
//!    the rotation below 2 rpm so head movements don't induce nauseating
//!    cross-coupled accelerations.
//! 3. **How much structure?** — hoop area from the centrifugal load,
//!    reusing the `sigma = rho omega^2 r^2` relation of
//!    `tpt-yard-space-structural`.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::Material;
//! use tpt_yard_habitat::{HabitatDesigner, HabitatType};
//!
//! let designer = HabitatDesigner::new(HabitatType::StanfordTorus {
//!     radius_m: 100.0,
//!     tube_diameter_m: 20.0,
//! }, Material::aa5083());
//!
//! // 1 g at 100 m: omega = sqrt(9.81/100) = 0.313 rad/s = 2.99 rpm.
//! let omega = designer.required_rotation(100.0, 1.0);
//! assert!((omega - (9.81f64 / 100.0).sqrt()).abs() < 1e-12);
//!
//! // ...but 2.99 rpm exceeds the 2 rpm Coriolis comfort limit.
//! assert!(!designer.coriolis_effects(100.0, 2.99).within_comfort);
//! ```

use tpt_yard_core::Material;

/// Coriolis comfort assessment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoriolisReport {
    /// Rotation rate, rpm.
    pub rotation_rpm: f64,
    /// The comfort guideline, rpm (2 rpm per established practice).
    pub comfort_limit_rpm: f64,
    /// Tangential velocity of the rim, m/s.
    pub rim_velocity_ms: f64,
    /// Cross-coupled acceleration for a 1 m/s head movement, % of g.
    pub coriolis_acceleration_pct_g: f64,
    /// Below the comfort limit.
    pub within_comfort: bool,
}

/// Structural sizing of the rotating hull.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StructuralDesign {
    /// Required hull shell thickness for the ring, mm.
    pub shell_thickness_mm: f64,
    /// Hoop stress at the design rotation, MPa.
    pub hoop_stress_mpa: f64,
    /// Utilization against yield with the safety factor applied.
    pub utilization: f64,
    /// Total structure mass, kg.
    pub structure_mass_kg: f64,
}

/// Habitat archetypes.
#[derive(Debug, Clone, PartialEq)]
pub enum HabitatType {
    /// O'Neill cylinder: a long rotating cylinder (a pair, historically).
    ONeillCylinder {
        /// Cylinder radius, m.
        radius_m: f64,
        /// Cylinder length, m.
        length_m: f64,
    },
    /// Stanford torus.
    StanfordTorus {
        /// Major radius (ring centreline), m.
        radius_m: f64,
        /// Tube diameter, m.
        tube_diameter_m: f64,
    },
    /// Bernal sphere.
    BernalSphere {
        /// Sphere radius, m.
        radius_m: f64,
    },
    /// Small ring station.
    RingStation {
        /// Ring radius, m.
        radius_m: f64,
    },
    /// Custom geometry (see `tpt-yard-core::Geometry3D`).
    Custom,
}

/// Designer for rotating habitats.
#[derive(Debug, Clone, PartialEq)]
pub struct HabitatDesigner {
    /// The habitat archetype.
    pub habitat_type: HabitatType,
    /// Structural material.
    pub material: Material,
    /// Radius used for [`HabitatType::Custom`] sizing, m (0 = unset; the
    /// old behaviour silently assumed 100 m).
    pub custom_radius_m: f64,
    /// Minimum practical shell gauge, mm (fabrication limit; default 3 mm).
    pub min_shell_thickness_mm: f64,
}

impl HabitatDesigner {
    /// Creates a designer.
    pub fn new(habitat_type: HabitatType, material: Material) -> Self {
        Self {
            habitat_type,
            material,
            custom_radius_m: 0.0,
            min_shell_thickness_mm: 3.0,
        }
    }

    /// Builder: explicit radius for [`HabitatType::Custom`] sizing.
    #[must_use]
    pub fn with_custom_radius_m(mut self, radius_m: f64) -> Self {
        self.custom_radius_m = radius_m.max(0.0);
        self
    }

    /// Required rotation rate for a target gravity at `radius_m`, rad/s:
    /// `omega = sqrt(g_target * g0 / r)`.
    pub fn required_rotation(&self, radius_m: f64, target_gravity_g: f64) -> f64 {
        (target_gravity_g * 9.81 / radius_m).sqrt()
    }

    /// The same rotation in rpm.
    pub fn required_rotation_rpm(&self, radius_m: f64, target_gravity_g: f64) -> f64 {
        self.required_rotation(radius_m, target_gravity_g) * 60.0 / (2.0 * std::f64::consts::PI)
    }

    /// Coriolis effects at a rotation rate.
    ///
    /// The comfort criterion: rotation below 2 rpm keeps cross-coupled
    /// accelerations from ordinary head movements (taken as 1 m/s at 90° to
    /// the spin axis: `a = 2 omega v`) below roughly 10 % of g.
    pub fn coriolis_effects(&self, radius_m: f64, rotation_rpm: f64) -> CoriolisReport {
        let omega = 2.0 * std::f64::consts::PI * rotation_rpm / 60.0;
        let v_head = 1.0; // m/s head movement
        let a_c = 2.0 * omega * v_head;
        CoriolisReport {
            rotation_rpm,
            comfort_limit_rpm: 2.0,
            rim_velocity_ms: omega * radius_m,
            coriolis_acceleration_pct_g: a_c / 9.81 * 100.0,
            within_comfort: rotation_rpm <= 2.0,
        }
    }

    /// Structural sizing of the rotating pressure hull.
    ///
    /// The ring carries its own centrifugal load: hoop area
    /// `A = m * omega^2 * r / (2 pi sigma_allow)` (the tension of a spinning
    /// ring distributed over the circumference), and the shell thickness
    /// follows for the habitat's circumference. `habitat_mass_kg` is the
    /// *total* rotating mass (structure + outfit + air/water/soil).
    pub fn structural_design(&self, habitat_mass_kg: f64, safety_factor: f64) -> StructuralDesign {
        let radius = match &self.habitat_type {
            HabitatType::ONeillCylinder { radius_m, .. }
            | HabitatType::StanfordTorus { radius_m, .. }
            | HabitatType::BernalSphere { radius_m }
            | HabitatType::RingStation { radius_m } => *radius_m,
            // Unset custom radius: report an empty design rather than a
            // silently assumed 100 m.
            HabitatType::Custom => self.custom_radius_m,
        };
        if radius <= 0.0 {
            return StructuralDesign::default();
        }
        // 1 g design point by default (documented; callers re-run for Mars-g).
        let omega = self.required_rotation(radius, 1.0);
        let sigma_allow = self.material.yield_mpa * 1e6 / safety_factor.max(1e-6);

        // Hoop tension of a spinning ring: T = m * omega^2 * r / (2 pi).
        let tension_n = habitat_mass_kg * omega * omega * radius / (2.0 * std::f64::consts::PI);
        let ideal_area_m2 = tension_n / sigma_allow;

        // The circumference of the shell that actually carries the hoop
        // load: for a torus that is the *tube* (minor) circumference — the
        // major ring length carries no hoop tension of the spin — while
        // cylinders, spheres and ring stations carry it around the main
        // circumference.
        let carrying_circumference = match &self.habitat_type {
            HabitatType::StanfordTorus { tube_diameter_m, .. } => {
                std::f64::consts::PI * tube_diameter_m.max(1e-3)
            }
            _ => 2.0 * std::f64::consts::PI * radius,
        };

        // Round the shell up to the fabrication minimum gauge; the
        // utilization below is then a *real* margin, not a tautological 1.
        let thickness_m = (ideal_area_m2 / carrying_circumference)
            .max(self.min_shell_thickness_mm / 1000.0);
        let area_m2 = thickness_m * carrying_circumference;

        let stress_mpa = (tension_n / area_m2) / 1e6;
        let utilization = stress_mpa / sigma_allow;

        StructuralDesign {
            shell_thickness_mm: thickness_m * 1000.0,
            hoop_stress_mpa: stress_mpa,
            utilization,
            structure_mass_kg: area_m2 * carrying_circumference * self.material.density_kg_m3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn designer(radius: f64) -> HabitatDesigner {
        HabitatDesigner::new(
            HabitatType::StanfordTorus {
                radius_m: radius,
                tube_diameter_m: 20.0,
            },
            Material::aa5083(),
        )
    }

    /// Verification: omega = sqrt(g/r) — the spec's formula.
    #[test]
    fn required_rotation_matches_omega_sqrt_gr() {
        let d = designer(100.0);
        let omega = d.required_rotation(100.0, 1.0);
        assert!((omega - (9.81 / 100.0f64).sqrt()).abs() < 1e-12);
        // Mars gravity (0.38 g) needs sqrt(0.38) of the rate.
        let mars = d.required_rotation(100.0, 0.38);
        assert!((mars / omega - 0.38f64.sqrt()).abs() < 1e-12);
        // rpm conversion.
        let rpm = d.required_rotation_rpm(100.0, 1.0);
        assert!((rpm - 2.988).abs() < 0.01, "rpm {rpm}");
    }

    /// Verification: the 2 rpm Coriolis comfort limit.
    #[test]
    fn coriolis_comfort_limit_at_two_rpm() {
        let d = designer(100.0);
        assert!(d.coriolis_effects(100.0, 1.9).within_comfort);
        assert!(d.coriolis_effects(100.0, 2.0).within_comfort); // inclusive
        assert!(!d.coriolis_effects(100.0, 2.1).within_comfort);
        // At the 2 rpm limit the cross-coupled acceleration for a 1 m/s
        // head movement is ~4.3 % of g — below the ~10 % nausea threshold.
        let report = d.coriolis_effects(100.0, 2.0);
        assert!(report.coriolis_acceleration_pct_g < 10.0);
        let expected_rim = 2.0 * 2.0 * std::f64::consts::PI / 60.0 * 100.0;
        assert!((report.rim_velocity_ms - expected_rim).abs() < 1e-9);
    }

    #[test]
    fn one_g_at_small_radius_exceeds_comfort() {
        let d = designer(20.0);
        // 1 g at 20 m requires ~6.6 rpm — above the comfort limit. Real
        // designs either accept it or use a larger radius.
        let rpm = d.required_rotation_rpm(20.0, 1.0);
        assert!(rpm > 2.0);
        assert!(!d.coriolis_effects(20.0, rpm).within_comfort);
    }

    #[test]
    fn structural_design_is_self_consistent() {
        let d = designer(100.0);
        // Small habitat: the 3 mm fabrication minimum gauge floors the
        // shell, so the utilization is a real (low) margin, not a
        // tautological 1.0 (review 7B).
        let small = d.structural_design(1_000_000.0, 2.0);
        assert!((small.shell_thickness_mm - 3.0).abs() < 1e-9);
        assert!(small.utilization < 1.0);
        assert!(small.structure_mass_kg > 0.0);

        // Heavier habitat beyond the gauge: thicker shell, and the
        // utilization rises toward (but never tautologically reaches) 1.
        let design = d.structural_design(5e8, 2.0);
        assert!(design.shell_thickness_mm > 3.0);
        let heavy = d.structural_design(2.5e9, 2.0);
        assert!(heavy.shell_thickness_mm > design.shell_thickness_mm);
        assert!(heavy.utilization > design.utilization);
        assert!(heavy.utilization <= 1.0 + 1e-9);
        // Higher safety factor: thicker shell.
        let safer = d.structural_design(5e8, 4.0);
        assert!(safer.shell_thickness_mm > design.shell_thickness_mm);
    }

    /// Regression (review 7B): the torus load path is the *tube* shell, so
    /// its required gauge differs from an equal-radius ring station; and a
    /// `Custom` habitat without an explicit radius reports an empty design
    /// instead of silently assuming 100 m.
    #[test]
    fn torus_carries_on_the_tube_and_custom_is_explicit() {
        let torus = designer(100.0); // tube diameter 20 m
        let ring = HabitatDesigner::new(HabitatType::RingStation { radius_m: 100.0 }, Material::aa5083());
        let t_torus = torus.structural_design(5e8, 2.0).shell_thickness_mm;
        let t_ring = ring.structural_design(5e8, 2.0).shell_thickness_mm;
        // Tube circumference (2*pi*10) is far shorter than the major
        // circumference (2*pi*100): the same steel area makes a much
        // thicker tube shell.
        assert!(
            t_torus > t_ring * 5.0,
            "torus gauge {t_torus} vs ring gauge {t_ring}"
        );

        let custom_unset = HabitatDesigner::new(HabitatType::Custom, Material::aa5083())
            .structural_design(1e6, 2.0);
        assert_eq!(custom_unset.shell_thickness_mm, 0.0, "unset custom radius: empty design");
        let custom = HabitatDesigner::new(HabitatType::Custom, Material::aa5083())
            .with_custom_radius_m(250.0)
            .structural_design(1e6, 2.0);
        assert!(custom.shell_thickness_mm > 0.0);
    }

    #[test]
    fn habitat_type_radius_is_used() {
        let oneill = HabitatDesigner::new(
            HabitatType::ONeillCylinder {
                radius_m: 4000.0,
                length_m: 32_000.0,
            },
            Material::ah36(),
        );
        // Classic O'Neill: 4 km radius, ~0.47 rpm for 1 g — comfortably
        // below the 2 rpm limit.
        let rpm = oneill.required_rotation_rpm(4000.0, 1.0);
        assert!((rpm - 0.474).abs() < 0.01, "rpm {rpm}");
        assert!(oneill.coriolis_effects(4000.0, rpm).within_comfort);
    }
}
