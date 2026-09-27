//! Launch calculations: slipway end launch, drydock float-out, shiplift,
//! side launch, and post-launch stability.
//!
//! The slipway model is the classical end-launch statics chain (RFC 0002's
//! companion treatment in `test-data/golden/sea/slipway-launch-stability.json`):
//!
//! 1. **Sliding** down the ways at slope θ with grease friction μ: launch
//!    velocity from the energy balance `v² = 2·g·s·(sinθ − μ·cosθ)`.
//! 2. **Way pressure** — while the ways are fully in contact, `P = W·cosθ
//!    / (L_way·b_way)`; after the fore poppet leaves the ways the load
//!    concentrates at the end poppet and rises.
//! 3. **Tip-up** — if the CoG passes the way end before buoyancy lifts the
//!    stern, the vessel pivots about the poppet: the launch is unsafe.
//!    The check compares the travel at which buoyancy equals weight against
//!    the travel at which the CoG crosses the way end.
//! 4. **Water entry** — entry angle equals the way slope; slamming
//!    pressure `p = ½·ρ·v²·C_imp` on the entering shell.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::{MassProperties, Vector3};
//! use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};
//!
//! let analysis = LaunchAnalysis {
//!     launch_method: LaunchMethod::Slipway { slope_deg: 3.0, ways: 2 },
//!     vessel_weight: MassProperties {
//!         mass_kg: 4_000_000.0,
//!         cog: Vector3::new(70.0, 0.0, 6.0),
//!     },
//!     way_length_m: 120.0,
//!     way_width_m: 2.0,
//!     friction_coefficient: 0.02,
//!     poppet_to_cog_m: 70.0, // CoG 70 m forward of the aft way end
//!     end_bearing_m: 20.0,   // way-end cribbing bearing length
//!     immersion_length_m: 90.0,
//!     block_coefficient: 0.8,
//!     breadth_m: 20.0,
//!     site: SiteConditions { max_sea_state: 3 },
//! };
//! let result = analysis.slipway_launch();
//! assert!(!result.tip_up_risk);
//! assert!(result.safe);
//! ```

use std::fmt;

use tpt_yard_core::MassProperties;
#[cfg(test)]
use tpt_yard_core::Vector3;
use tpt_yard_drydock::{DockedVessel, Drydock};
use tpt_yard_weight::WeightModel;

const RHO_SEA: f64 = 1025.0;
const G: f64 = 9.81;
/// Allowable way pressure at the end poppet, MPa (class screening).
pub const MAX_WAY_PRESSURE_MPA: f64 = 0.5;
/// Slamming impact coefficient (von Kármán-type, engineering screening).
const C_IMPACT: f64 = 4.0;

/// How the vessel enters the water.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LaunchMethod {
    /// End launch down `ways` greased ways at `slope_deg`.
    Slipway {
        /// Way slope from horizontal, degrees.
        slope_deg: f64,
        /// Number of ways.
        ways: u32,
    },
    /// Float-out of a building dock (see `tpt-yard-drydock`).
    DrydockFlooding,
    /// 90° side launch from a ground-level berth.
    SideLaunch,
    /// Vertical lift into the water on a platform.
    Shiplift {
        /// Platform capacity, kN.
        capacity_kn: f64,
    },
}

/// Launch-site environmental limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SiteConditions {
    /// Maximum operational sea state (Douglas scale) for the launch.
    pub max_sea_state: u32,
}

/// Result of a slipway end launch.
#[derive(Debug, Clone, PartialEq)]
pub struct SlipwayLaunchResult {
    /// Velocity at way-end / water entry, m/s.
    pub sliding_velocity_ms: f64,
    /// Way pressure at sample points along the travel, MPa.
    pub way_pressure_mpa: Vec<f64>,
    /// True if the stern would tip about the way end before buoyancy
    /// catches it.
    pub tip_up_risk: bool,
    /// Entry (trim) angle into the water, degrees.
    pub entry_angle_deg: f64,
    /// Slamming pressure on water entry, MPa.
    pub slamming_pressure_mpa: f64,
    /// All checks pass (pressure ≤ limit, no tip-up).
    pub safe: bool,
    /// Findings.
    pub notes: Vec<String>,
}

/// Post-launch stability result.
#[derive(Debug, Clone, PartialEq)]
pub struct StabilityResult {
    /// GM at the launch condition, m.
    pub gm_m: f64,
    /// Draft at the launch condition, m.
    pub draft_m: f64,
    /// `GM ≥ 0.15 m` screening pass.
    pub stable: bool,
    /// Findings.
    pub notes: Vec<String>,
}

/// Launch analysis for a vessel and site.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchAnalysis {
    /// The launch method.
    pub launch_method: LaunchMethod,
    /// Launch weight and CoG (CoG.x measured from the *aft way end*).
    pub vessel_weight: MassProperties,
    /// Way length in contact at start of launch, m.
    pub way_length_m: f64,
    /// Total way width (all ways), m.
    pub way_width_m: f64,
    /// Grease friction coefficient (0.01–0.035 typical).
    pub friction_coefficient: f64,
    /// Distance from the aft way end to the CoG, m.
    pub poppet_to_cog_m: f64,
    /// Bearing length of the way-end cribbing that carries the pivoting
    /// load, m (typ. 0.15-0.25 of the way length).
    pub end_bearing_m: f64,
    /// Waterline length available for immersion during launch, m.
    pub immersion_length_m: f64,
    /// Block coefficient at launch draft.
    pub block_coefficient: f64,
    /// Moulded breadth, m.
    pub breadth_m: f64,
    /// Site conditions.
    pub site: SiteConditions,
}

impl LaunchAnalysis {
    /// Runs the slipway end-launch calculation chain.
    ///
    /// # Panics
    ///
    /// Never panics for positive inputs; degenerate geometry yields
    /// `safe: false` with notes.
    pub fn slipway_launch(&self) -> SlipwayLaunchResult {
        let mut notes = Vec::new();
        let (slope_deg, ways) = match self.launch_method {
            LaunchMethod::Slipway { slope_deg, ways } => (slope_deg, ways),
            _ => {
                notes.push("not a slipway launch: statics computed anyway".into());
                (3.0, 2)
            }
        };
        let theta = slope_deg.to_radians();
        let w = self.vessel_weight.mass_kg;

        // --- 1. Way pressure along the travel -------------------------------
        // While in full contact: uniform P1 = W·cosθ / (L_way·b_way).
        let _full_contact_kpa =
            w * G * theta.cos() / (self.way_length_m * self.way_width_m).max(1e-6);
        // After the fore poppet passes the way end, the reaction
        // concentrates on the way-end cribbing over `end_bearing_m`.
        let pivot_kpa =
            w * G * theta.cos() / (self.end_bearing_m * ways as f64 * self.way_width_m).max(1e-6);
        // Sample the pressure: contact, 75 % contact, 50 % contact, pivot.
        let mut way_pressure_mpa = Vec::with_capacity(5);
        for frac in [1.0, 0.75, 0.5] {
            way_pressure_mpa.push(
                w * G * theta.cos()
                    / (self.way_length_m * frac * self.way_width_m).max(1e-6)
                    / 1_000_000.0,
            );
        }
        let pivot_mpa = pivot_kpa / 1_000_000.0;
        way_pressure_mpa.push(pivot_mpa);

        // --- 2. Launch velocity (energy balance over the way travel) --------
        let travel = self.way_length_m;
        let a = G * (theta.sin() - self.friction_coefficient * theta.cos());
        let sliding_velocity_ms = if a > 0.0 {
            (2.0 * a * travel).sqrt()
        } else {
            0.0
        };

        // --- 3. Tip-up check -------------------------------------------------
        // Buoyancy grows with immersion travel; the vessel floats off when
        // displaced volume equals the launch mass. Immersion length per
        // travel metre = sinθ (draft grows as the hull runs into the water).
        // Draft needed: d_f = W/(ρ Cb L B) along a length that immerses as
        // the vessel travels. Tip-up if CoG crosses the way end (travel =
        // poppet_to_cog_m) before float-off travel.
        let floating_draft =
            w / (RHO_SEA * self.block_coefficient * self.immersion_length_m * self.breadth_m);
        // Travel needed so the mean draft reaches floating_draft (stern-first
        // entry: draft ≈ travel·sinθ at the CoG, conservatively).
        let float_off_travel = floating_draft / theta.sin().max(1e-6);
        let tip_up_risk = self.poppet_to_cog_m < float_off_travel;
        if tip_up_risk {
            notes.push(format!(
                "CoG passes the way end after {:.1} m but float-off needs {:.1} m: stern tips on the poppet",
                self.poppet_to_cog_m, float_off_travel
            ));
        } else {
            notes.push(format!(
                "float-off at {:.1} m travel, CoG crosses the way end at {:.1} m: no tip-up",
                float_off_travel, self.poppet_to_cog_m
            ));
        }

        // --- 4. Water entry ---------------------------------------------------
        let entry_angle_deg = slope_deg;
        let slamming_pressure_mpa =
            0.5 * RHO_SEA * sliding_velocity_ms * sliding_velocity_ms * C_IMPACT / 1_000_000.0;

        let mut safe = !tip_up_risk;
        if pivot_mpa > MAX_WAY_PRESSURE_MPA {
            notes.push(format!(
                "end-poppet pressure {pivot_mpa:.2} MPa exceeds the {MAX_WAY_PRESSURE_MPA} MPa limit"
            ));
            safe = false;
        }
        if sliding_velocity_ms <= 0.0 {
            notes.push("friction prevents sliding: the vessel will not launch".into());
            safe = false;
        }

        SlipwayLaunchResult {
            sliding_velocity_ms,
            way_pressure_mpa,
            tip_up_risk,
            entry_angle_deg,
            slamming_pressure_mpa,
            safe,
            notes,
        }
    }

    /// Drydock float-out via the docking crate's flooding sequence.
    ///
    /// # Errors
    ///
    /// Returns the docking error string when the vessel cannot float out.
    pub fn drydock_flooding(
        &self,
        dock: &Drydock,
        flood_rate_m3_hr: f64,
    ) -> Result<tpt_yard_drydock::FloodingSequence, String> {
        let draft = self.launch_draft_m();
        let vessel = DockedVessel {
            launch_weight_kg: self.vessel_weight.mass_kg,
            cog_above_keel_m: self.vessel_weight.cog.z,
            length_m: self.immersion_length_m,
            breadth_m: self.breadth_m,
            block_coefficient: self.block_coefficient,
            ballast_tanks: vec![],
        };
        let _ = draft;
        dock.flooding_sequence(&vessel, flood_rate_m3_hr)
            .map_err(|e| e.to_string())
    }

    /// Launch draft, m.
    pub fn launch_draft_m(&self) -> f64 {
        self.vessel_weight.mass_kg
            / (RHO_SEA * self.block_coefficient * self.immersion_length_m * self.breadth_m)
    }

    /// Post-launch stability from the weight model: GM at the launch
    /// condition must clear the 0.15 m screening value with the vessel in
    /// its *partial outfitting* state.
    pub fn launch_stability(&self, weight_model: &WeightModel) -> StabilityResult {
        let mut notes = Vec::new();
        let mass = weight_model.total_weight();
        let cog = weight_model
            .centre_of_gravity()
            .unwrap_or(self.vessel_weight.cog);
        let draft =
            mass / (RHO_SEA * self.block_coefficient * self.immersion_length_m * self.breadth_m);
        let bm = self.breadth_m * self.breadth_m / (12.0 * draft.max(1e-6));
        let gm = draft / 2.0 + bm - cog.z;
        let stable = gm >= tpt_yard_drydock::MIN_GM_M;
        if !stable {
            notes.push(format!(
                "GM {gm:.2} m below the {tref} m minimum",
                tref = tpt_yard_drydock::MIN_GM_M
            ));
        }
        notes.push(format!(
            "launch condition: {:.0} t at draft {:.2} m, KG {:.2} m",
            mass / 1000.0,
            draft,
            cog.z
        ));
        StabilityResult {
            gm_m: gm,
            draft_m: draft,
            stable,
            notes,
        }
    }
}

impl fmt::Display for LaunchMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaunchMethod::Slipway { slope_deg, ways } => {
                write!(f, "slipway ({slope_deg}°, {ways} ways)")
            }
            LaunchMethod::DrydockFlooding => f.write_str("drydock flooding"),
            LaunchMethod::SideLaunch => f.write_str("side launch"),
            LaunchMethod::Shiplift { capacity_kn } => write!(f, "shiplift ({capacity_kn} kN)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analysis(cog_x: f64) -> LaunchAnalysis {
        LaunchAnalysis {
            launch_method: LaunchMethod::Slipway {
                slope_deg: 3.0,
                ways: 2,
            },
            vessel_weight: MassProperties {
                mass_kg: 4_000_000.0,
                cog: Vector3::new(cog_x, 0.0, 6.0),
            },
            way_length_m: 120.0,
            way_width_m: 2.0,
            friction_coefficient: 0.02,
            poppet_to_cog_m: cog_x,
            end_bearing_m: 20.0,
            immersion_length_m: 90.0,
            block_coefficient: 0.8,
            breadth_m: 20.0,
            site: SiteConditions { max_sea_state: 3 },
        }
    }

    /// Verification: tip-up detection for known launch configurations.
    #[test]
    fn test_slipway_tip_up() {
        // Healthy launch: CoG well forward of the way end; float-off travel
        // for these numbers: draft = 4e6/(1025·0.8·90·20) = 2.713 m;
        // travel = 2.713/sin(3°) = 51.8 m < 70 m → no tip-up.
        let ok = analysis(70.0).slipway_launch();
        assert!(!ok.tip_up_risk);
        assert!(ok.safe, "notes: {:?}", ok.notes);
        assert!(ok.sliding_velocity_ms > 3.0);
        assert_eq!(ok.entry_angle_deg, 3.0);

        // Degenerate launch: CoG 40 m from the way end tips before floating.
        let bad = analysis(40.0).slipway_launch();
        assert!(bad.tip_up_risk);
        assert!(!bad.safe);
        assert!(bad.notes.iter().any(|n| n.contains("tip")));
    }

    #[test]
    fn undersized_end_cribbing_is_flagged() {
        let mut a = analysis(70.0);
        a.end_bearing_m = 2.0; // far too small: 39.19e6/(2*2*2) = 4.9 MPa
        let r = a.slipway_launch();
        assert!(!r.safe);
        assert!(r.notes.iter().any(|n| n.contains("exceeds")));
    }

    #[test]
    fn way_pressure_rises_to_the_poppet() {
        let r = analysis(70.0).slipway_launch();
        // Monotone rise along the samples; final sample is the pivot value.
        for w in r.way_pressure_mpa.windows(2) {
            assert!(w[1] >= w[0], "pressures: {:?}", r.way_pressure_mpa);
        }
        // Full-contact pressure: 4e6·9.81·cos3°/(120·2) = 163.4 kPa.
        assert!(
            (r.way_pressure_mpa[0] - 0.1634).abs() < 0.001,
            "{}",
            r.way_pressure_mpa[0]
        );
    }

    #[test]
    fn sliding_velocity_matches_energy_balance() {
        let a = analysis(70.0);
        let r = a.slipway_launch();
        let theta = 3.0f64.to_radians();
        let expected = (2.0 * 9.81 * a.way_length_m * (theta.sin() - 0.02 * theta.cos())).sqrt();
        assert!((r.sliding_velocity_ms - expected).abs() < 1e-9);
    }

    #[test]
    fn sticky_ways_prevent_launch() {
        let mut a = analysis(70.0);
        a.friction_coefficient = 0.20; // tan(3°) ≈ 0.052: stuck
        let r = a.slipway_launch();
        assert_eq!(r.sliding_velocity_ms, 0.0);
        assert!(!r.safe);
    }

    #[test]
    fn stability_uses_the_weight_model() {
        let a = analysis(70.0);
        let mut wm = WeightModel::new(4_000_000.0, tpt_yard_core::Vector3::new(70.0, 0.0, 6.0));
        wm.add_item(tpt_yard_weight::WeightItem {
            id: tpt_yard_core::ItemId(1),
            name: "hull".into(),
            group: "hull".into(),
            weight_kg: 4_000_000.0,
            cog: tpt_yard_core::Vector3::new(70.0, 0.0, 6.0),
            status: tpt_yard_weight::ItemStatus::Installed,
            margin_pct: 0.0,
            installed_by: None,
        });
        let s = a.launch_stability(&wm);
        // Beamy hull at modest KG: GM comfortably positive.
        assert!(s.stable, "{:?}", s.notes);
        assert!(s.gm_m > 1.0);

        // Heavy topside: capsized.
        let mut wm2 = wm.clone();
        wm2.items[0].cog.z = 30.0;
        let s2 = a.launch_stability(&wm2);
        assert!(!s2.stable);
    }

    #[test]
    fn drydock_delegation() {
        let a = analysis(70.0);
        let dock = Drydock {
            length_m: 200.0,
            width_m: 30.0,
            depth_m: 10.0,
        };
        let seq = a.drydock_flooding(&dock, 5_000.0).unwrap();
        assert!(seq.stable_at_every_level);
    }
}
