//! Engineering material properties (substrate: `tpt-engineering` material
//! tables, vendored as dependency-free presets).

/// Thermal/mechanical properties of a homogeneous material.
///
/// Values are room-temperature engineering averages — adequate for the
/// analytical heat-flow and structural models in this workspace; not a
/// substitute for temperature-dependent material data.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// Material name (e.g. "AH36 shipbuilding steel").
    pub name: String,
    /// Density, kg/m³.
    pub density_kg_m3: f64,
    /// Thermal conductivity, W/(m·K).
    pub conductivity_w_mk: f64,
    /// Specific heat capacity, J/(kg·K).
    pub specific_heat_j_kg_k: f64,
    /// Melting point, °C.
    pub melting_point_c: f64,
    /// Young's modulus, GPa.
    pub youngs_modulus_gpa: f64,
    /// Yield strength, MPa.
    pub yield_mpa: f64,
    /// Ultimate tensile strength, MPa.
    pub ultimate_mpa: f64,
    /// Mean thermal expansion coefficient, 1/K.
    pub thermal_expansion_1_k: f64,
    /// Poisson's ratio.
    pub poissons_ratio: f64,
}

impl Material {
    /// Thermal diffusivity α = k / (ρ·c), m²/s.
    pub fn thermal_diffusivity_m2_s(&self) -> f64 {
        self.conductivity_w_mk / (self.density_kg_m3 * self.specific_heat_j_kg_k)
    }

    /// Shear modulus G = E / (2·(1+ν)), GPa.
    pub fn shear_modulus_gpa(&self) -> f64 {
        self.youngs_modulus_gpa / (2.0 * (1.0 + self.poissons_ratio))
    }

    /// AH36 high-strength shipbuilding steel.
    pub fn ah36() -> Self {
        Self {
            name: "AH36 shipbuilding steel".into(),
            density_kg_m3: 7850.0,
            conductivity_w_mk: 50.0,
            specific_heat_j_kg_k: 490.0,
            melting_point_c: 1500.0,
            youngs_modulus_gpa: 210.0,
            yield_mpa: 355.0,
            ultimate_mpa: 490.0,
            thermal_expansion_1_k: 12.0e-6,
            poissons_ratio: 0.30,
        }
    }

    /// Mild structural steel (S235).
    pub fn s235() -> Self {
        Self {
            name: "S235 structural steel".into(),
            density_kg_m3: 7850.0,
            conductivity_w_mk: 52.0,
            specific_heat_j_kg_k: 460.0,
            melting_point_c: 1450.0,
            youngs_modulus_gpa: 210.0,
            yield_mpa: 235.0,
            ultimate_mpa: 360.0,
            thermal_expansion_1_k: 12.0e-6,
            poissons_ratio: 0.30,
        }
    }

    /// AA5083 aluminium (marine grade).
    pub fn aa5083() -> Self {
        Self {
            name: "AA5083 aluminium".into(),
            density_kg_m3: 2660.0,
            conductivity_w_mk: 120.0,
            specific_heat_j_kg_k: 900.0,
            melting_point_c: 640.0,
            youngs_modulus_gpa: 71.0,
            yield_mpa: 215.0,
            ultimate_mpa: 305.0,
            thermal_expansion_1_k: 24.0e-6,
            poissons_ratio: 0.33,
        }
    }

    /// AISI 316L stainless steel.
    pub fn aisi316l() -> Self {
        Self {
            name: "AISI 316L stainless steel".into(),
            density_kg_m3: 8000.0,
            conductivity_w_mk: 16.0,
            specific_heat_j_kg_k: 500.0,
            melting_point_c: 1400.0,
            youngs_modulus_gpa: 193.0,
            yield_mpa: 290.0,
            ultimate_mpa: 580.0,
            thermal_expansion_1_k: 16.0e-6,
            poissons_ratio: 0.30,
        }
    }

    /// Inconel 718 ( nickel superalloy, space/launch hardware).
    pub fn inconel718() -> Self {
        Self {
            name: "Inconel 718".into(),
            density_kg_m3: 8190.0,
            conductivity_w_mk: 11.4,
            specific_heat_j_kg_k: 435.0,
            melting_point_c: 1350.0,
            youngs_modulus_gpa: 200.0,
            yield_mpa: 1030.0,
            ultimate_mpa: 1240.0,
            thermal_expansion_1_k: 13.0e-6,
            poissons_ratio: 0.29,
        }
    }

    /// Ti-6Al-4V titanium.
    pub fn ti6al4v() -> Self {
        Self {
            name: "Ti-6Al-4V".into(),
            density_kg_m3: 4430.0,
            conductivity_w_mk: 6.7,
            specific_heat_j_kg_k: 526.0,
            melting_point_c: 1660.0,
            youngs_modulus_gpa: 114.0,
            yield_mpa: 880.0,
            ultimate_mpa: 950.0,
            thermal_expansion_1_k: 8.6e-6,
            poissons_ratio: 0.34,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffusivity_is_consistent() {
        let m = Material::ah36();
        let alpha = m.thermal_diffusivity_m2_s();
        // Steel: k=50, ρ=7850, c=490 → α ≈ 1.3e-5 m²/s
        assert!((alpha - 50.0 / (7850.0 * 490.0)).abs() < 1e-15);
        assert!(alpha > 1e-5 && alpha < 2e-5);
    }

    #[test]
    fn shear_modulus_of_steel() {
        let g = Material::ah36().shear_modulus_gpa();
        assert!((g - 80.77).abs() < 0.1, "G = {g}");
    }

    #[test]
    fn presets_differ_where_they_must() {
        let steel = Material::ah36();
        let alu = Material::aa5083();
        assert!(alu.density_kg_m3 < steel.density_kg_m3 / 2.0);
        assert!(alu.melting_point_c < steel.melting_point_c);
        assert!(Material::inconel718().yield_mpa > steel.yield_mpa * 2.0);
        assert!(Material::ti6al4v().density_kg_m3 < steel.density_kg_m3);
        assert!(Material::aisi316l().conductivity_w_mk < steel.conductivity_w_mk / 2.0);
    }
}
