//! In-space manufacturing and additive construction planning.
//!
//! [`InSpaceManufacturing`] plans printing and deposition work in vacuum:
//! print time from mass and deposition rate, heat-rejection planning for
//! the deposition energy (no convection up there), and the in-situ quality
//! verification plan. Feedstock can come from ISRU sources.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::Material;
//! use tpt_yard_space_manufacturing::{
//!     AdditiveTechnique, Feedstock, InSpaceManufacturing, ManufacturingProcess,
//! };
//!
//! let ism = InSpaceManufacturing::new(
//!     ManufacturingProcess::AdditiveManufacturing {
//!         technique: AdditiveTechnique::DirectedEnergyDeposition,
//!         material: "AA5083".into(),
//!     },
//!     Feedstock::earth_launched(2660.0),
//!     Material::aa5083(),
//! );
//! // 1 m^3 of aluminium at 5 kg/h deposition.
//! let hours = ism.print_time_estimate(1.0, 5.0).unwrap();
//! assert!((hours - 2660.0 / 5.0).abs() < 1e-9);
//! ```
#![allow(clippy::doc_markdown)]

use std::fmt;

use tpt_yard_core::Material;

/// Manufacturing processes available in space.
#[derive(Debug, Clone, PartialEq)]
pub enum ManufacturingProcess {
    /// Additive manufacturing of a specified material.
    AdditiveManufacturing {
        /// The additive technique.
        technique: AdditiveTechnique,
        /// Build material name.
        material: String,
    },
    /// Wire-feed deposition.
    WireFeedDeposition,
    /// Electron-beam freeform fabrication.
    ElectronBeamFreeform,
    /// Robotic assembly of truss bays (not printing — handled by the
    /// orbital-assembly crate for steps; here for time/energy bookkeeping).
    RoboticAssemblyOfTruss,
    /// In-situ resource utilisation: process local material.
    InSituResourceUtilization {
        /// The local source.
        source: IsruSource,
    },
}

/// Additive techniques.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdditiveTechnique {
    /// Powder bed fusion.
    PowderBedFusion,
    /// Directed energy deposition.
    DirectedEnergyDeposition,
    /// Wire arc additive manufacturing.
    WireArcAdditive,
    /// Electron beam freeform.
    ElectronBeamFreeform,
}

/// ISRU sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsruSource {
    /// Lunar regolith (metals, oxygen).
    LunarRegolith,
    /// Martian regolith.
    MartianRegolith,
    /// Asteroid metal.
    AsteroidMetal,
    /// Recycled debris.
    RecycledDebris,
}

impl fmt::Display for IsruSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            IsruSource::LunarRegolith => "lunar regolith",
            IsruSource::MartianRegolith => "martian regolith",
            IsruSource::AsteroidMetal => "asteroid metal",
            IsruSource::RecycledDebris => "recycled debris",
        };
        f.write_str(s)
    }
}

/// Where the feedstock comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Feedstock {
    /// Feedstock density, kg/m³.
    pub density_kg_m3: f64,
    /// Fraction of feedstock that becomes scrap/overspray (0-1).
    pub scrap_frac: f64,
    /// ISRU source, if local.
    pub isru: Option<IsruSource>,
}

impl Feedstock {
    /// Earth-launched feedstock of a material density (no scrap modelled).
    pub fn earth_launched(density_kg_m3: f64) -> Self {
        Self {
            density_kg_m3,
            scrap_frac: 0.0,
            isru: None,
        }
    }

    /// ISRU feedstock with a processing scrap fraction.
    pub fn from_isru(source: IsruSource, density_kg_m3: f64, scrap_frac: f64) -> Self {
        Self {
            density_kg_m3,
            scrap_frac: scrap_frac.clamp(0.0, 1.0),
            isru: Some(source),
        }
    }
}

/// Heat-rejection plan for the print (vacuum: radiation only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalControlPlan {
    /// Deposition power to reject, kW.
    pub deposition_power_kw: f64,
    /// Required radiator area, m².
    pub radiator_area_m2: f64,
    /// Build-plate temperature control band, °C.
    pub build_temp_band_c: (f64, f64),
    /// True when the print must pause for eclipse thermal management.
    pub eclipse_pauses: bool,
}

/// In-situ quality plan.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityPlan {
    /// Layer-wise imaging coverage, percent.
    pub layer_imaging_pct: f64,
    /// In-situ NDT methods.
    pub in_situ_ndt: Vec<String>,
    /// Witness coupons per N kg of deposition.
    pub witness_coupon_per_kg: f64,
    /// Destructive testing budget (returns to Earth or sampled locally).
    pub destructive_testing: bool,
}

/// Errors from manufacturing planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManufacturingError {
    /// Deposition rate must be positive.
    InvalidRate,
    /// Volume must be positive.
    InvalidVolume,
}

impl fmt::Display for ManufacturingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManufacturingError::InvalidRate => f.write_str("deposition rate must be > 0"),
            ManufacturingError::InvalidVolume => f.write_str("volume must be > 0"),
        }
    }
}

impl std::error::Error for ManufacturingError {}

/// The in-space manufacturing planner.
#[derive(Debug, Clone, PartialEq)]
pub struct InSpaceManufacturing {
    /// The process.
    pub process: ManufacturingProcess,
    /// Feedstock.
    pub material_feedstock: Feedstock,
    /// The build material.
    pub material: Material,
    /// Deposition energy per kg (process-specific), kWh/kg.
    pub energy_kwh_per_kg: f64,
}

impl InSpaceManufacturing {
    /// Creates a planner with a process-typical energy intensity.
    pub fn new(
        process: ManufacturingProcess,
        material_feedstock: Feedstock,
        material: Material,
    ) -> Self {
        let energy = match &process {
            ManufacturingProcess::AdditiveManufacturing { technique, .. } => match technique {
                AdditiveTechnique::PowderBedFusion => 12.0,
                AdditiveTechnique::DirectedEnergyDeposition => 8.0,
                AdditiveTechnique::WireArcAdditive => 4.0,
                AdditiveTechnique::ElectronBeamFreeform => 6.0,
            },
            ManufacturingProcess::WireFeedDeposition => 4.0,
            ManufacturingProcess::ElectronBeamFreeform => 6.0,
            ManufacturingProcess::RoboticAssemblyOfTruss => 0.5,
            ManufacturingProcess::InSituResourceUtilization { .. } => 15.0,
        };
        Self {
            process,
            material_feedstock,
            material,
            energy_kwh_per_kg: energy,
        }
    }

    /// Print time estimate for a part of `volume_m3` at a deposition rate,
    /// hours: `t = rho * V * (1 + scrap) / rate`.
    ///
    /// # Errors
    ///
    /// [`ManufacturingError`] for non-positive inputs.
    pub fn print_time_estimate(
        &self,
        volume_m3: f64,
        deposition_rate_kg_hr: f64,
    ) -> Result<f64, ManufacturingError> {
        if volume_m3 <= 0.0 {
            return Err(ManufacturingError::InvalidVolume);
        }
        if deposition_rate_kg_hr <= 0.0 {
            return Err(ManufacturingError::InvalidRate);
        }
        let mass = volume_m3 * self.material_feedstock.density_kg_m3;
        let with_scrap = mass * (1.0 + self.material_feedstock.scrap_frac);
        Ok(with_scrap / deposition_rate_kg_hr)
    }

    /// Feedstock mass (including scrap) for a finished volume, kg.
    pub fn feedstock_mass_kg(&self, volume_m3: f64) -> f64 {
        volume_m3
            * self.material_feedstock.density_kg_m3
            * (1.0 + self.material_feedstock.scrap_frac)
    }

    /// Total energy for a print, kWh.
    pub fn energy_kwh(&self, volume_m3: f64) -> Result<f64, ManufacturingError> {
        Ok(self.feedstock_mass_kg(volume_m3) * self.energy_kwh_per_kg)
    }

    /// Heat-rejection plan for a print at a given deposition rate.
    ///
    /// Deposition power = `rate * energy intensity`; vacuum rejects heat by
    /// radiation only: `A = P / (eps * sigma * T^4)` with eps 0.85 and a
    /// radiator at 350 K.
    ///
    /// # Errors
    ///
    /// [`ManufacturingError::InvalidRate`] for non-positive rates.
    pub fn thermal_control_during_print(
        &self,
        deposition_rate_kg_hr: f64,
    ) -> Result<ThermalControlPlan, ManufacturingError> {
        if deposition_rate_kg_hr <= 0.0 {
            return Err(ManufacturingError::InvalidRate);
        }
        let power_kw = deposition_rate_kg_hr * self.energy_kwh_per_kg; // kWh/kg * kg/h = kW
        const EPS: f64 = 0.85;
        const SIGMA: f64 = 5.670374419e-8;
        const T_RAD: f64 = 350.0; // K
        let area = power_kw * 1000.0 / (EPS * SIGMA * T_RAD.powi(4));
        Ok(ThermalControlPlan {
            deposition_power_kw: power_kw,
            radiator_area_m2: area,
            build_temp_band_c: (20.0, 180.0),
            eclipse_pauses: true,
        })
    }

    /// In-situ quality verification plan (vacuum NDT subset).
    pub fn quality_verification(&self) -> QualityPlan {
        QualityPlan {
            layer_imaging_pct: 100.0,
            in_situ_ndt: vec!["thermography".into(), "eddy current".into()],
            witness_coupon_per_kg: 0.01,
            destructive_testing: self.material_feedstock.isru.is_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ded() -> InSpaceManufacturing {
        InSpaceManufacturing::new(
            ManufacturingProcess::AdditiveManufacturing {
                technique: AdditiveTechnique::DirectedEnergyDeposition,
                material: "AA5083".into(),
            },
            Feedstock::earth_launched(2660.0),
            Material::aa5083(),
        )
    }

    /// Verification: print time = mass / rate.
    #[test]
    fn print_time_is_mass_over_rate() {
        let ism = ded();
        let hours = ism.print_time_estimate(1.0, 5.0).unwrap();
        assert!((hours - 532.0).abs() < 1e-9); // 2660 kg / 5 kg/h
                                               // ISRU scrap increases the feedstock and the time.
        let isru = InSpaceManufacturing::new(
            ManufacturingProcess::InSituResourceUtilization {
                source: IsruSource::LunarRegolith,
            },
            Feedstock::from_isru(IsruSource::LunarRegolith, 2660.0, 0.25),
            Material::aa5083(),
        );
        let t_isru = isru.print_time_estimate(1.0, 5.0).unwrap();
        assert!((t_isru - hours * 1.25).abs() < 1e-9);
    }

    #[test]
    fn energy_follows_mass_and_intensity() {
        let ism = ded(); // DED: 8 kWh/kg
        let e = ism.energy_kwh(1.0).unwrap();
        assert!((e - 2660.0 * 8.0).abs() < 1e-9);
    }

    #[test]
    fn radiator_rejects_deposition_power() {
        let ism = ded();
        let plan = ism.thermal_control_during_print(5.0).unwrap();
        // 5 kg/h * 8 kWh/kg = 40 kW deposition power.
        assert!((plan.deposition_power_kw - 40.0).abs() < 1e-9);
        // Radiator area from Stefan-Boltzmann at 350 K, eps 0.85.
        let expected = 40_000.0 / (0.85 * 5.670374419e-8 * 350.0f64.powi(4));
        assert!((plan.radiator_area_m2 - expected).abs() < 1e-9);
        assert!(plan.radiator_area_m2 > 5.0);
        assert!(plan.eclipse_pauses);
    }

    #[test]
    fn quality_plan_checks_out() {
        let ism = ded();
        let q = ism.quality_verification();
        assert_eq!(q.layer_imaging_pct, 100.0);
        assert!(!q.in_situ_ndt.is_empty());
        assert!(q.destructive_testing);
        // ISRU print: samples are not returned to Earth.
        let isru = InSpaceManufacturing::new(
            ManufacturingProcess::InSituResourceUtilization {
                source: IsruSource::AsteroidMetal,
            },
            Feedstock::from_isru(IsruSource::AsteroidMetal, 7800.0, 0.3),
            Material::ah36(),
        );
        assert!(!isru.quality_verification().destructive_testing);
    }

    #[test]
    fn invalid_inputs_rejected() {
        let ism = ded();
        assert_eq!(
            ism.print_time_estimate(0.0, 5.0),
            Err(ManufacturingError::InvalidVolume)
        );
        assert_eq!(
            ism.print_time_estimate(1.0, 0.0),
            Err(ManufacturingError::InvalidRate)
        );
        assert!(matches!(
            ism.thermal_control_during_print(0.0),
            Err(ManufacturingError::InvalidRate)
        ));
    }
}
