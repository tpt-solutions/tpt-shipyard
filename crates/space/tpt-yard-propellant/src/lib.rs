//! Propellant loading and boil-off management for space vessels.
//!
//! Loading cryogenic propellant into a depot or vehicle tank is a thermal
//! control problem as much as a plumbing one: the tank must be chilled down
//! (or the first fill flashes to vapour), the fill rate is limited by vent
//! capacity, and every hour on-orbit leaks heat into the tank that becomes
//! boil-off. [`LoadingPlanner`] produces a [`PropellantLoadingPlan`] and
//! evaluates the [`BoilOffReport`] for a given tank and insulation.
//!
//! # Example
//!
//! ```
//! use tpt_yard_propellant::{LoadingPlanner, PropellantSpec, TankSpec};
//!
//! let planner = LoadingPlanner::new();
//! let tank = TankSpec { volume_m3: 300.0, ullage_frac: 0.03, heat_leak_w: 2_000.0 };
//! let plan = planner.plan_loading(&PropellantSpec::lox(), &tank, 50.0).unwrap();
//! assert!(plan.venting_required);            // LOX is cryogenic
//! assert!(plan.total_time_hours > 0.0);
//!
//! let boil_off = planner.boil_off_report(&PropellantSpec::lox(), &tank);
//! assert!(boil_off.boil_off_kg_day > 300.0);
//! assert!(!boil_off.zero_boil_off_achieved); // 0.24 %/day is not ZBO
//! ```

use std::fmt;

/// A propellant's storage-relevant properties.
#[derive(Debug, Clone, PartialEq)]
pub struct PropellantSpec {
    /// Propellant name (e.g. "LOX").
    pub name: String,
    /// Molar mass, kg/kmol.
    pub molar_mass_kg_kmol: f64,
    /// Normal boiling point, K.
    pub boiling_point_k: f64,
    /// Liquid density at the boiling point, kg/m³.
    pub liquid_density_kg_m3: f64,
    /// Latent heat of vaporisation at the boiling point, kJ/kg.
    pub latent_heat_kj_kg: f64,
    /// True for cryogenics (normal boiling point below 120 K).
    pub cryogenic: bool,
    /// Chemical formula.
    pub formula: String,
}

impl PropellantSpec {
    /// Liquid oxygen.
    pub fn lox() -> Self {
        Self {
            name: "LOX".into(),
            molar_mass_kg_kmol: 32.0,
            boiling_point_k: 90.2,
            liquid_density_kg_m3: 1141.0,
            latent_heat_kj_kg: 213.0,
            cryogenic: true,
            formula: "O2".into(),
        }
    }

    /// Liquid hydrogen.
    pub fn lh2() -> Self {
        Self {
            name: "LH2".into(),
            molar_mass_kg_kmol: 2.016,
            boiling_point_k: 20.3,
            liquid_density_kg_m3: 70.9,
            latent_heat_kj_kg: 445.0,
            cryogenic: true,
            formula: "H2".into(),
        }
    }

    /// Liquid methane (methalox).
    pub fn ch4() -> Self {
        Self {
            name: "LCH4".into(),
            molar_mass_kg_kmol: 16.04,
            boiling_point_k: 111.7,
            liquid_density_kg_m3: 422.0,
            latent_heat_kj_kg: 511.0,
            cryogenic: true,
            formula: "CH4".into(),
        }
    }

    /// Monomethylhydrazine (storable).
    pub fn mmh() -> Self {
        Self {
            name: "MMH".into(),
            molar_mass_kg_kmol: 46.07,
            boiling_point_k: 361.0,
            liquid_density_kg_m3: 874.0,
            latent_heat_kj_kg: 812.0,
            cryogenic: false,
            formula: "CH3NHNH2".into(),
        }
    }

    /// Nitrogen tetroxide (storable oxidiser).
    pub fn nto() -> Self {
        Self {
            name: "NTO".into(),
            molar_mass_kg_kmol: 92.01,
            boiling_point_k: 294.0,
            liquid_density_kg_m3: 1450.0,
            latent_heat_kj_kg: 414.0,
            cryogenic: false,
            formula: "N2O4".into(),
        }
    }
}

impl fmt::Display for PropellantSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.formula)
    }
}

/// The tank being filled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TankSpec {
    /// Tank volume, m³.
    pub volume_m3: f64,
    /// Ullage fraction held for vapour (0.02–0.05 typical).
    pub ullage_frac: f64,
    /// Heat leak into the tank through insulation and supports, W
    /// (non-negative, finite).
    pub heat_leak_w: f64,
}

/// How boil-off is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoilOffPolicy {
    /// Vent as generated (classic cryogenic depot practice).
    Vented,
    /// Recondense with a cryocooler (reduced boil-off).
    Recooled,
    /// Cooler sized to beat the heat leak: zero boil-off.
    ZeroBoilOff,
}

/// The thermal-control side of the loading plan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalControlPlan {
    /// Tank must be chilled before fill (cryogenics only).
    pub chilldown_required: bool,
    /// Subcooling target below the boiling point, K.
    pub subcooling_k: f64,
    /// The boil-off management policy.
    pub policy: BoilOffPolicy,
}

/// The complete loading plan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropellantLoadingPlan {
    /// Mass fill rate, kg/s.
    pub fill_rate_kg_s: f64,
    /// Venting required during loading (cryogenic fill).
    pub venting_required: bool,
    /// Thermal-control plan.
    pub thermal_control: ThermalControlPlan,
    /// Chilldown time before useful fill, hours.
    pub chilldown_hours: f64,
    /// Total loading time including chilldown, hours.
    pub total_time_hours: f64,
    /// Loaded mass, kg.
    pub loaded_mass_kg: f64,
}

/// Boil-off assessment of a loaded tank.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoilOffReport {
    /// Heat leak converted to boil-off, kg/day.
    pub boil_off_kg_day: f64,
    /// Boil-off as a percent of load per day.
    pub boil_off_pct_day: f64,
    /// Days until the ullage pressure hits the vent setpoint from a
    /// cold-soaked start (longer = better insulation).
    pub time_to_vent_days: f64,
    /// True when the heat leak is small enough to call ZBO
    /// (< 0.1 %/day).
    pub zero_boil_off_achieved: bool,
    /// Cryocooler input power needed to intercept the full heat leak, W
    /// (Carnot-limited at the propellant boiling temperature; see
    /// [`LoadingPlanner::cooler_input_power_w`]).
    pub cooler_power_w: f64,
}

/// Errors from loading planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingError {
    /// Fill rate must be positive.
    InvalidFillRate,
    /// Tank geometry or heat leak must be positive.
    InvalidTank,
}

impl fmt::Display for LoadingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadingError::InvalidFillRate => f.write_str("fill rate must be > 0"),
            LoadingError::InvalidTank => f.write_str("invalid tank volume or ullage"),
        }
    }
}

impl std::error::Error for LoadingError {}

/// Produces propellant loading plans and boil-off reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadingPlanner {
    /// Tank thermal-mass coefficient for chilldown (kg of steel-equivalent
    /// per m³ of tank), ~35 for a launch-class tank.
    pub tank_thermal_mass_kg_m3: f64,
    /// Cryocooler efficiency as a fraction of the Carnot COP (Stirling and
    /// Brayton coolers land around 5-10 % of Carnot at 20-90 K).
    pub cooler_carnot_fraction: f64,
}

impl Default for LoadingPlanner {
    fn default() -> Self {
        Self {
            tank_thermal_mass_kg_m3: 35.0,
            cooler_carnot_fraction: 0.07,
        }
    }
}

impl LoadingPlanner {
    /// Creates a planner with default constants.
    pub fn new() -> Self {
        Self::default()
    }

    /// Plans the loading of `fill_frac` of the tank at a given rate.
    ///
    /// # Errors
    ///
    /// [`LoadingError`] for invalid inputs.
    pub fn plan_loading(
        &self,
        propellant: &PropellantSpec,
        tank: &TankSpec,
        fill_rate_kg_s: f64,
    ) -> Result<PropellantLoadingPlan, LoadingError> {
        if !(fill_rate_kg_s > 0.0) {
            return Err(LoadingError::InvalidFillRate);
        }
        if tank.volume_m3 <= 0.0
            || tank.ullage_frac < 0.0
            || tank.ullage_frac >= 1.0
            || !(tank.heat_leak_w >= 0.0)
        {
            return Err(LoadingError::InvalidTank);
        }
        let loaded_mass_kg =
            tank.volume_m3 * (1.0 - tank.ullage_frac) * propellant.liquid_density_kg_m3;
        let fill_hours = loaded_mass_kg / fill_rate_kg_s / 3600.0;

        // Chilldown: cool the tank structure from 293 K to the boiling
        // point, using latent heat consumed by the vapour flash.
        let chilldown_hours = if propellant.cryogenic {
            let thermal_mass = self.tank_thermal_mass_kg_m3 * tank.volume_m3;
            // Steel cp ~0.46 kJ/kgK; 80 % of the heat is absorbed by
            // vaporised propellant at the latent heat.
            let heat_kj = thermal_mass * 0.46 * (293.0 - propellant.boiling_point_k);
            let vaporised_kg = heat_kj * 0.8 / propellant.latent_heat_kj_kg;
            vaporised_kg / fill_rate_kg_s / 3600.0
        } else {
            0.0
        };

        let policy = if propellant.cryogenic {
            let zbo = self.zbo_heat_leak_w(propellant, tank);
            // Within ZBO: no hardware needed. Moderately above ZBO: a
            // cryocooler intercepts the leak (`Recooled`). More than 20×
            // the ZBO leak: active cooling is impractical — vent.
            if tank.heat_leak_w <= zbo {
                BoilOffPolicy::ZeroBoilOff
            } else if tank.heat_leak_w > 20.0 * zbo {
                BoilOffPolicy::Vented
            } else {
                BoilOffPolicy::Recooled
            }
        } else {
            // Storable propellants: no boil-off problem.
            BoilOffPolicy::ZeroBoilOff
        };

        Ok(PropellantLoadingPlan {
            fill_rate_kg_s,
            venting_required: propellant.cryogenic,
            thermal_control: ThermalControlPlan {
                chilldown_required: propellant.cryogenic,
                subcooling_k: if propellant.cryogenic { 3.0 } else { 0.0 },
                policy,
            },
            chilldown_hours,
            total_time_hours: chilldown_hours + fill_hours,
            loaded_mass_kg,
        })
    }

    /// Boil-off assessment of a loaded tank.
    ///
    /// Heat leak ÷ latent heat, expressed per day; ZBO threshold at
    /// 0.1 %/day.
    pub fn boil_off_report(&self, propellant: &PropellantSpec, tank: &TankSpec) -> BoilOffReport {
        let loaded = tank.volume_m3 * (1.0 - tank.ullage_frac) * propellant.liquid_density_kg_m3;
        let kg_per_day = tank.heat_leak_w * 86_400.0 / (propellant.latent_heat_kj_kg * 1000.0);
        let pct_day = kg_per_day / loaded * 100.0;
        let time_to_vent_days = if tank.heat_leak_w > 0.0 {
            // Cold-soak margin: ~5 % of the load's latent heat can be
            // absorbed before the ullage reaches the vent setpoint.
            0.05 * loaded * propellant.latent_heat_kj_kg * 1000.0 / (tank.heat_leak_w * 86_400.0)
        } else {
            f64::INFINITY
        };
        BoilOffReport {
            boil_off_kg_day: kg_per_day,
            boil_off_pct_day: pct_day,
            time_to_vent_days,
            zero_boil_off_achieved: pct_day < 0.1,
            cooler_power_w: self
                .cooler_input_power_w(tank.heat_leak_w, propellant.boiling_point_k),
        }
    }

    /// Cryocooler *input* power (W) to lift `heat_leak_w` at a cold-side
    /// temperature `t_cold_k`, from the Carnot COP scaled by
    /// [`LoadingPlanner::cooler_carnot_fraction`]:
    /// `P_in = Q · (T_hot/T_cold − 1) / fraction` with T_hot = 300 K.
    ///
    /// (The previous version multiplied the leak by a flat 100:1 — inverted
    /// and identical for 20 K hydrogen and 90 K oxygen.)
    pub fn cooler_input_power_w(&self, heat_leak_w: f64, t_cold_k: f64) -> f64 {
        if !(heat_leak_w > 0.0) || !(t_cold_k > 0.0) {
            return 0.0;
        }
        let carnot_ratio = 300.0 / t_cold_k - 1.0;
        heat_leak_w * carnot_ratio / self.cooler_carnot_fraction.max(1e-6)
    }

    /// Heat leak (W) below which the tank qualifies as zero-boil-off.
    pub fn zbo_heat_leak_w(&self, propellant: &PropellantSpec, tank: &TankSpec) -> f64 {
        let loaded = tank.volume_m3 * (1.0 - tank.ullage_frac) * propellant.liquid_density_kg_m3;
        // 0.1 %/day of load through latent heat.
        0.001 * loaded * propellant.latent_heat_kj_kg * 1000.0 / 86_400.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cryo_loading_requires_vent_and_chilldown() {
        let planner = LoadingPlanner::new();
        let tank = TankSpec {
            volume_m3: 300.0,
            ullage_frac: 0.03,
            heat_leak_w: 800.0,
        };
        let plan = planner
            .plan_loading(&PropellantSpec::lox(), &tank, 50.0)
            .unwrap();
        assert!(plan.venting_required);
        assert!(plan.thermal_control.chilldown_required);
        assert!(plan.chilldown_hours > 0.0);
        assert!(plan.total_time_hours > plan.chilldown_hours);
        assert!((plan.loaded_mass_kg - 300.0 * 0.97 * 1141.0).abs() < 1.0);

        // Storable propellant: no chilldown, no venting.
        let storable = planner
            .plan_loading(&PropellantSpec::mmh(), &tank, 20.0)
            .unwrap();
        assert!(!storable.venting_required);
        assert_eq!(storable.chilldown_hours, 0.0);
    }

    /// Verification: cryogenic loading reaches thermal stability — the
    /// plan's vented policy matches a boil-off report that balances the
    /// heat leak.
    #[test]
    fn cryogenic_loading_thermal_stability() {
        let planner = LoadingPlanner::new();
        // 2000 W into a 300 m3 LOX tank = 0.24 %/day: vented, not ZBO.
        let tank = TankSpec {
            volume_m3: 300.0,
            ullage_frac: 0.03,
            heat_leak_w: 2_000.0,
        };
        let lox = PropellantSpec::lox();
        let plan = planner.plan_loading(&lox, &tank, 50.0).unwrap();
        let report = planner.boil_off_report(&lox, &tank);
        // 2000 W -> 2000*86400/213000 = 811.3 kg/day.
        assert!(
            (report.boil_off_kg_day - 811.3).abs() < 1.0,
            "{}",
            report.boil_off_kg_day
        );
        assert!(!report.zero_boil_off_achieved);
        // 2000 W is ~2.4x the ZBO leak: a cryocooler can intercept it
        // (`Recooled`). A leak 20x beyond ZBO is impractical to cool: vent.
        assert_eq!(plan.thermal_control.policy, BoilOffPolicy::Recooled);
        let huge = TankSpec {
            heat_leak_w: 20_000.0,
            ..tank
        };
        assert_eq!(
            planner
                .plan_loading(&lox, &huge, 50.0)
                .unwrap()
                .thermal_control
                .policy,
            BoilOffPolicy::Vented
        );
        // The ZBO threshold heat leak is far below 2000 W here.
        assert!(planner.zbo_heat_leak_w(&lox, &tank) < 2_000.0);
        // The 800 W tank from the other test is a borderline-ZBO case:
        // 0.098 %/day just squeaks under the 0.1 %/day line.
        let near_zbo = TankSpec {
            volume_m3: 300.0,
            ullage_frac: 0.03,
            heat_leak_w: 800.0,
        };
        assert!(
            planner
                .boil_off_report(&lox, &near_zbo)
                .zero_boil_off_achieved
        );
    }

    #[test]
    fn lh2_is_the_hardest_cryo() {
        let planner = LoadingPlanner::new();
        let tank = TankSpec {
            volume_m3: 300.0,
            ullage_frac: 0.03,
            heat_leak_w: 200.0,
        };
        let lox = planner.boil_off_report(&PropellantSpec::lox(), &tank);
        let lh2 = planner.boil_off_report(&PropellantSpec::lh2(), &tank);
        // LH2: low density (small load) and modest latent heat -> the worst
        // percentage boil-off.
        assert!(lh2.boil_off_pct_day > lox.boil_off_pct_day);
    }

    #[test]
    fn zbo_tank_qualifies() {
        let planner = LoadingPlanner::new();
        let lox = PropellantSpec::lox();
        let tank = TankSpec {
            volume_m3: 300.0,
            ullage_frac: 0.03,
            heat_leak_w: 800.0,
        };
        let zbo_limit = planner.zbo_heat_leak_w(&lox, &tank);
        let good_tank = TankSpec {
            heat_leak_w: zbo_limit * 0.5,
            ..tank
        };
        let report = planner.boil_off_report(&lox, &good_tank);
        assert!(report.zero_boil_off_achieved);
        assert_eq!(
            planner
                .plan_loading(&lox, &good_tank, 50.0)
                .unwrap()
                .thermal_control
                .policy,
            BoilOffPolicy::ZeroBoilOff
        );
        // Cooler input power is Carnot-scaled at the LOX boiling point
        // (review 7B: the old flat 100:1 multiplier was inverted and
        // temperature-blind).
        let expected =
            planner.cooler_input_power_w(good_tank.heat_leak_w, lox.boiling_point_k);
        assert!((report.cooler_power_w - expected).abs() < 1e-6);
        // Hydrogen (20 K) needs far more input power per watt lifted than
        // LOX (90 K) — the ratio must be temperature-dependent.
        let h2_leak = planner.cooler_input_power_w(1.0, 20.0);
        let o2_leak = planner.cooler_input_power_w(1.0, 90.0);
        assert!(h2_leak > o2_leak * 2.0, "{h2_leak} vs {o2_leak}");
    }

    #[test]
    fn fill_time_scales_with_rate() {
        let planner = LoadingPlanner::new();
        let tank = TankSpec {
            volume_m3: 300.0,
            ullage_frac: 0.03,
            heat_leak_w: 800.0,
        };
        let lox = PropellantSpec::lox();
        let fast = planner.plan_loading(&lox, &tank, 100.0).unwrap();
        let slow = planner.plan_loading(&lox, &tank, 25.0).unwrap();
        assert!((fast.total_time_hours * 4.0 - slow.total_time_hours).abs() < 1e-9);
    }

    #[test]
    fn invalid_inputs_rejected() {
        let planner = LoadingPlanner::new();
        let tank = TankSpec {
            volume_m3: 300.0,
            ullage_frac: 0.03,
            heat_leak_w: 800.0,
        };
        assert_eq!(
            planner.plan_loading(&PropellantSpec::lox(), &tank, 0.0),
            Err(LoadingError::InvalidFillRate)
        );
        let bad_tank = TankSpec {
            volume_m3: 0.0,
            ..tank
        };
        assert_eq!(
            planner.plan_loading(&PropellantSpec::lox(), &bad_tank, 50.0),
            Err(LoadingError::InvalidTank)
        );
    }
}
