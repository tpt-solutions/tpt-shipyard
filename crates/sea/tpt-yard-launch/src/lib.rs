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

/// One sample of the dynamic end-launch trajectory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaunchSample {
    /// Time from release, s.
    pub time_s: f64,
    /// Travel down the ways, m.
    pub travel_m: f64,
    /// Velocity along the ways, m/s.
    pub velocity_ms: f64,
    /// Buoyancy built up at this travel, kN.
    pub buoyancy_kn: f64,
    /// Total way reaction (weight component − buoyancy component), kN.
    pub way_reaction_kn: f64,
    /// Stern lift: buoyancy moment arm effect at the stern, kN·m about
    /// the way end (positive = stern being lifted).
    pub stern_lift_moment_knm: f64,
    /// Forward poppet load: the reaction concentrated at the way-end
    /// cribbing once the CoG passes it (0 while in full contact), kN.
    pub poppet_load_kn: f64,
    /// End-of-ways moment: net moment about the way end
    /// (weight side − buoyancy side), kN·m; sign flips toward float-off.
    pub end_of_ways_moment_knm: f64,
}

/// How the dynamic launch ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchEnd {
    /// The vessel ran off the way end at the recorded travel.
    Afloat,
    /// Friction stopped the vessel before it floated off.
    Stuck,
    /// The CoG passed the way end while buoyancy was still far below the
    /// weight: stern tip-up.
    TipUp,
    /// The integration budget ran out (dt budget is generous; reaching
    /// this signals a pathological configuration).
    Unresolved,
}

/// Result of the dynamic end-launch simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicLaunchResult {
    /// Trajectory samples at fixed dt.
    pub samples: Vec<LaunchSample>,
    /// How the run ended.
    pub end: LaunchEnd,
    /// Total run time to the end condition, s.
    pub duration_s: f64,
    /// Velocity at the end of the ways (for `Afloat`), m/s.
    pub end_velocity_ms: f64,
    /// Buoyancy share of the weight when the CoG crossed the way end, %.
    pub buoyancy_relief_at_pivot_pct: f64,
    /// End-of-ways moment about the way end at the pivot instant,
    /// kN·m (weight side positive = tipping): `W·(x_cog − s_pivot) − Δ·l_b`.
    pub pivot_moment_knm: f64,
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
        let (slope_deg, _ways) = match self.launch_method {
            LaunchMethod::Slipway { slope_deg, ways } => (slope_deg, ways),
            _ => {
                // No fake slipway statics for other launch methods — return
                // an explicitly empty, failed screening result.
                return SlipwayLaunchResult {
                    sliding_velocity_ms: 0.0,
                    way_pressure_mpa: Vec::new(),
                    tip_up_risk: false,
                    entry_angle_deg: 0.0,
                    slamming_pressure_mpa: 0.0,
                    safe: false,
                    notes: vec![format!(
                        "not a slipway launch ({:?}): run the appropriate analysis for this method",
                        self.launch_method
                    )],
                };
            }
        };
        let theta = slope_deg.to_radians();
        let w = self.vessel_weight.mass_kg;

        // --- 1. Way pressure along the travel -------------------------------
        // While in full contact: uniform P1 = W·cosθ / (L_way·b_total).
        // `way_width_m` is the *total* width over all ways — do not multiply
        // by `ways` again.
        // After the stern is afloat and the CoG reaches the way end, the
        // reaction concentrates on the way-end cribbing over
        // `end_bearing_m`, *relieved by the buoyancy built up to that
        // instant* (immersion grows parabolically with travel:
        // F_b = ½ ρ g C_b B t² sinθ for travel t past water entry).
        let t_pivot = self.poppet_to_cog_m.max(0.0);
        let buoyancy_at_pivot_n = 0.5
            * RHO_SEA
            * G
            * self.block_coefficient
            * self.breadth_m
            * t_pivot
            * t_pivot
            * theta.sin();
        let pivot_load_n = (w * G * theta.cos() - buoyancy_at_pivot_n).max(0.0);
        let pivot_kpa = pivot_load_n / (self.end_bearing_m * self.way_width_m).max(1e-6);
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
        let relief_pct = buoyancy_at_pivot_n / (w * G).max(1e-9) * 100.0;
        notes.push(format!(
            "pivot load {:.0} kN (buoyancy relieves {relief_pct:.0} % of the weight at the way end)",
            pivot_load_n / 1000.0
        ));

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
        // Slamming is driven by the *vertical* entry velocity component.
        let v_vertical = sliding_velocity_ms * theta.sin();
        let slamming_pressure_mpa =
            0.5 * RHO_SEA * v_vertical * v_vertical * C_IMPACT / 1_000_000.0;

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

    /// Dynamic end-launch simulation: time-stepped motion down the ways.
    ///
    /// Along-ways equation of motion for the sliding mass (review 7H
    /// roadmap item — the static screening in [`Self::slipway_launch`]
    /// becomes a trajectory):
    ///
    /// `a = g·sinθ − μ·g·cosθ·(1 − Δ/W) − (Δ/W)·g·sinθ − k·v²/W`
    ///
    /// with buoyancy `Δ(s) = ρ·g·Cb·B·min(s, L_imm)·s·sinθ` — the
    /// travelled length immersed to the travel draft, the *same*
    /// immersion assumption as the static float-off screen (a pure entry
    /// wedge would float the reference case ~27 m later and contradict
    /// it), a grease static-breakaway threshold before motion starts, and
    /// quadratic water drag `k = ½·ρ·Cd·A` once the stern is wet. The run
    /// ends when the vessel runs off the ways, friction stops it, or the
    /// CoG passes the way end under-insulated by buoyancy (tip-up).
    pub fn dynamic_launch(&self) -> DynamicLaunchResult {
        let mut notes = Vec::new();
        let (slope_deg, _ways) = match self.launch_method {
            LaunchMethod::Slipway { slope_deg, ways } => (slope_deg, ways),
            _ => {
                return DynamicLaunchResult {
                    samples: Vec::new(),
                    end: LaunchEnd::Unresolved,
                    duration_s: 0.0,
                    end_velocity_ms: 0.0,
                    buoyancy_relief_at_pivot_pct: 0.0,
                    pivot_moment_knm: 0.0,
                    notes: vec!["not a slipway launch: no dynamic model".into()],
                };
            }
        };
        let theta = slope_deg.to_radians();
        let w = self.vessel_weight.mass_kg;
        let weight_kn = w * G / 1000.0;
        let mu = self.friction_coefficient;
        // Quadratic water drag: the immersed transverse section grows
        // with the entry draft, A(s) = B·s·sinθ.
        let drag_area_at = |s: f64| 0.5 * RHO_SEA * 0.8 * self.breadth_m * s * theta.sin();

        let dt = 0.02;
        let max_t = 300.0;
        let mut samples = Vec::with_capacity((max_t / dt) as usize);
        let (mut s, mut v, mut t) = (0.0f64, 0.0f64, 0.0f64);
        let mut end = LaunchEnd::Unresolved;
        let mut relief_at_pivot = 0.0;
        let mut pivot_moment = 0.0;

        // Series helpers: stern-lift and end-of-ways moments about the way
        // end, and the poppet concentration once the CoG overhangs it.
        let way_end = self.way_length_m.max(1e-6);
        let series = |s: f64, buoy_kn: f64, reaction_kn: f64| {
            let weight_kn = w * G / 1000.0;
            // Effective longitudinal centres, m from the way end: the CoG
            // sits at poppet_to_cog (it crosses the end at that travel);
            // the buoyancy centroid at the mid-length of the immersed
            // wedge, which grows with travel (clamped to the hull).
            let x_cog = (self.poppet_to_cog_m - s).abs().min(way_end);
            let immersed = s.min(self.immersion_length_m);
            let x_buoy = (way_end - immersed / 2.0).max(0.0);
            let stern_lift = buoy_kn * (way_end - x_buoy);
            let moment = weight_kn * x_cog - buoy_kn * x_buoy;
            let poppet = if s >= self.poppet_to_cog_m {
                (weight_kn * (s - self.poppet_to_cog_m)
                    / (way_end - self.poppet_to_cog_m).max(1e-6))
                .min(reaction_kn.max(0.0))
            } else {
                0.0
            };
            (stern_lift, poppet, moment)
        };
        let push_sample = |samples: &mut Vec<LaunchSample>,
                           s: f64,
                           v: f64,
                           t: f64,
                           buoy_kn: f64,
                           reaction_kn: f64| {
            let (stern_lift, poppet, moment) = series(s, buoy_kn, reaction_kn);
            samples.push(LaunchSample {
                time_s: t,
                travel_m: s,
                velocity_ms: v,
                buoyancy_kn: buoy_kn,
                way_reaction_kn: reaction_kn,
                stern_lift_moment_knm: stern_lift,
                poppet_load_kn: poppet,
                end_of_ways_moment_knm: moment,
            });
        };

        while t <= max_t {
            // Static breakaway: the vessel does not move until gravity
            // overcomes the grease's static friction.
            if v <= 0.0 && theta.sin() <= mu * theta.cos() {
                end = LaunchEnd::Stuck;
                notes.push(format!(
                    "static friction holds: tan({slope_deg:.1}°) = {:.4} <= mu {mu:.4}",
                    theta.tan()
                ));
                push_sample(&mut samples, s, v, t, 0.0, weight_kn * theta.cos());
                break;
            }

            // Buoyancy growth: the travelled length immersed to the
            // travel draft (consistent with the static float-off screen).
            let buoy_n = RHO_SEA
                * G
                * self.block_coefficient
                * self.breadth_m
                * s.min(self.immersion_length_m)
                * s
                * theta.sin();
            let buoy_share = (buoy_n / (w * G)).min(1.0);

            // Pivot: CoG passes the way end while buoyancy still carries
            // less than the weight — the stern tips about the way end.
            if s >= self.poppet_to_cog_m && buoy_share < 0.98 {
                end = LaunchEnd::TipUp;
                relief_at_pivot = buoy_share * 100.0;
                // Residual tipping moment about the way end: the CoG
                // overhang times the unsupported weight share.
                let overhang = s - self.poppet_to_cog_m;
                pivot_moment = (w - buoy_n / G) * G / 1000.0 * overhang;
                notes.push(format!(
                    "tip-up at travel {s:.1} m: buoyancy carries only {relief_at_pivot:.0} % of the weight"
                ));
                push_sample(
                    &mut samples,
                    s,
                    v,
                    t,
                    buoy_n / 1000.0,
                    (w * G - buoy_n) * theta.cos() / 1000.0,
                );
                break;
            }

            // Off the way end: afloat.
            if s >= self.way_length_m {
                end = LaunchEnd::Afloat;
                notes.push(format!("afloat after {t:.1} s at {:.2} m/s", v));
                push_sample(&mut samples, s, v, t, buoy_n / 1000.0, 0.0);
                break;
            }

            // Along-ways acceleration.
            let drag = if buoy_n > 0.0 {
                drag_area_at(s) * v * v
            } else {
                0.0
            };
            let a = G * theta.sin()
                - mu * G * theta.cos() * (1.0 - buoy_share)
                - buoy_share * G * theta.sin()
                - drag / w;
            v = (v + a * dt).max(0.0);
            s += v * dt;
            t += dt;

            let buoy_kn = buoy_n / 1000.0;
            push_sample(
                &mut samples,
                s,
                v,
                t,
                buoy_kn,
                (weight_kn * theta.cos() - buoy_kn * theta.cos()).max(0.0),
            );

            if v <= 0.0 && t > 1.0 {
                // Started but lost headway (buoyancy + friction won).
                end = LaunchEnd::Stuck;
                notes.push(format!("lost headway at travel {s:.1} m after {t:.1} s"));
                break;
            }
        }
        if end == LaunchEnd::Unresolved {
            notes.push("integration budget exhausted".into());
        }
        let last = samples.last().copied().unwrap_or(LaunchSample {
            time_s: 0.0,
            travel_m: 0.0,
            velocity_ms: 0.0,
            buoyancy_kn: 0.0,
            way_reaction_kn: 0.0,
            stern_lift_moment_knm: 0.0,
            poppet_load_kn: 0.0,
            end_of_ways_moment_knm: 0.0,
        });
        DynamicLaunchResult {
            samples,
            end,
            duration_s: last.time_s,
            end_velocity_ms: last.velocity_ms,
            buoyancy_relief_at_pivot_pct: relief_at_pivot,
            pivot_moment_knm: pivot_moment,
            notes,
        }
    }

    /// Drydock float-out via the docking crate's flooding sequence. The
    /// launch draft is validated against the dock depth first so a too deep
    /// launch condition fails with a specific message instead of a generic
    /// "does not fit".
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
        if draft + 0.5 > dock.depth_m {
            return Err(format!(
                "launch draft {draft:.2} m (+0.5 m under-keel) exceeds the dock depth {:.2} m",
                dock.depth_m
            ));
        }
        let vessel = DockedVessel {
            launch_weight_kg: self.vessel_weight.mass_kg,
            cog_above_keel_m: self.vessel_weight.cog.z,
            lcg_from_midship_m: 0.0,
            length_m: self.immersion_length_m,
            breadth_m: self.breadth_m,
            block_coefficient: self.block_coefficient,
            ballast_tanks: vec![],
        };
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
    ///
    /// BM uses the fineness-corrected form `Cwp·B²/(12·Cb·T)` with
    /// `Cwp ≈ (1 + 2·Cb)/3` (standard waterplane-area approximation) — the
    /// plain box formula `B²/(12T)` ignores the block coefficient and
    /// overstates BM for full hulls.
    pub fn launch_stability(&self, weight_model: &WeightModel) -> StabilityResult {
        let mut notes = Vec::new();
        let mass = weight_model.total_weight();
        let cog = weight_model
            .centre_of_gravity()
            .unwrap_or(self.vessel_weight.cog);
        let draft =
            mass / (RHO_SEA * self.block_coefficient * self.immersion_length_m * self.breadth_m);
        let cwp = (1.0 + 2.0 * self.block_coefficient) / 3.0;
        let bm = cwp * self.breadth_m * self.breadth_m
            / (12.0 * self.block_coefficient * draft.max(1e-6));
        let gm = draft / 2.0 + bm - cog.z;
        let stable = gm >= tpt_yard_drydock::MIN_GM_M;
        if !stable {
            notes.push(format!(
                "GM {gm:.2} m below the {tref} m minimum",
                tref = tpt_yard_drydock::MIN_GM_M
            ));
        }
        notes.push(format!(
            "launch condition: {:.0} t at draft {:.2} m, KG {:.2} m (Cwp ≈ {cwp:.3})",
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
        // Review 7D: independent hand-computed value, not a re-derivation
        // of the same expression. a = g(sin 3 - 0.02 cos 3) = 0.3174844;
        // v = sqrt(2 a L) = sqrt(2 * 0.3174844 * 120) = 8.72904 m/s.
        let r = analysis(70.0).slipway_launch();
        assert!(
            (r.sliding_velocity_ms - 8.7290).abs() < 1e-3,
            "{}",
            r.sliding_velocity_ms
        );
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
        })
        .expect("valid weight item");
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

    /// Verification (review 7H): the dynamic run is consistent with the
    /// energy balance — without buoyancy or drag, v at the way end must
    /// match the closed-form sqrt(2·a·L).
    #[test]
    fn dynamic_launch_matches_energy_balance() {
        // Small buoyancy share at way end (deep ways): the run is nearly
        // pure sliding. v_end = sqrt(2 g (sin - mu cos) L). The CoG sits
        // beyond the way end so nothing overhangs (with no buoyancy, an
        // unsupported overhang genuinely tips the vessel).
        let mut a = analysis(200.0);
        a.poppet_to_cog_m = 200.0;
        // Defeat buoyancy: tiny breadth makes Delta negligible.
        a.breadth_m = 0.5;
        a.block_coefficient = 0.01;
        let r = a.dynamic_launch();
        assert_eq!(r.end, LaunchEnd::Afloat, "{:?}", r.notes);
        let theta = 3.0f64.to_radians();
        let expected =
            (2.0 * G * (theta.sin() - a.friction_coefficient * theta.cos()) * a.way_length_m)
                .sqrt();
        assert!(
            (r.end_velocity_ms - expected).abs() < 0.05 * expected,
            "sim {} vs energy {}",
            r.end_velocity_ms,
            expected
        );
        // The trajectory is monotone in travel and time (the terminal
        // sample restates the end condition, so ties are allowed).
        for w in r.samples.windows(2) {
            assert!(w[1].travel_m >= w[0].travel_m);
            assert!(w[1].time_s >= w[0].time_s);
        }
    }

    /// Sticky ways never release the vessel; the dynamic result says Stuck.
    #[test]
    fn dynamic_launch_detects_stuck_ways() {
        let mut a = analysis(70.0);
        a.friction_coefficient = 0.20; // tan(3 deg) ~ 0.052
        let r = a.dynamic_launch();
        assert_eq!(r.end, LaunchEnd::Stuck);
        assert!(r.samples.len() == 1, "no motion samples expected");
        assert!(r.notes.iter().any(|n| n.contains("static friction")));
    }

    /// The healthy reference launch floats off. The classical "checking"
    /// phase is visible: buoyancy's along-ways component decelerates the
    /// vessel hard near float-off, so the end velocity is modest even
    /// though mid-run speed is high.
    #[test]
    fn dynamic_launch_floats_the_reference_case() {
        let r = analysis(70.0).dynamic_launch();
        assert_eq!(r.end, LaunchEnd::Afloat, "{:?}", r.notes);
        // Checking phase: the vessel arrives gently.
        assert!(
            r.end_velocity_ms < 3.0 && r.end_velocity_ms >= 0.0,
            "end velocity {}",
            r.end_velocity_ms
        );
        // But it was moving fast mid-run.
        let mid = &r.samples[r.samples.len() / 2];
        assert!(mid.velocity_ms > 3.0, "mid-run {}", mid.velocity_ms);
        // Peak way reaction decreases along the run (buoyancy grows).
        let first = r.samples.first().unwrap().way_reaction_kn;
        assert!(
            mid.way_reaction_kn < first,
            "reaction {} at mid-run vs {} at start",
            mid.way_reaction_kn,
            first
        );
        assert!(r.duration_s > 10.0);
    }

    /// A far-aft CoG tips before buoyancy can catch it — matching the
    /// static tip-up verdict, plus a pivot moment figure.
    #[test]
    fn dynamic_launch_detects_tip_up() {
        let mut a = analysis(20.0); // CoG 20 m from the way end
        a.poppet_to_cog_m = 20.0;
        let r = a.dynamic_launch();
        assert_eq!(r.end, LaunchEnd::TipUp, "{:?}", r.notes);
        assert!(r.pivot_moment_knm > 0.0);
        assert!(r.buoyancy_relief_at_pivot_pct < 98.0);
    }

    /// Verification (review 7H launch partial): the time series carries
    /// the stern-lift, poppet-load and end-of-ways moment curves — poppet
    /// load concentrates only after the CoG passes the way end, the
    /// end-of-ways moment trends from weight-dominated toward buoyancy-
    /// dominated, and stern lift grows monotonically with buoyancy.
    #[test]
    fn dynamic_series_tracks_stern_lift_poppet_and_moment() {
        let r = analysis(70.0).dynamic_launch();
        assert_eq!(r.end, LaunchEnd::Afloat);
        // Poppet load stays zero in the healthy case: the vessel floats
        // off before (or just as) the CoG reaches the way end, so nothing
        // concentrates on the cribbing.
        assert!(
            r.samples.iter().all(|s| s.poppet_load_kn == 0.0),
            "healthy launch must not load the poppet"
        );
        // In a tip-up run the story is different: the CoG passes the way
        // end while the blocks still carry load, and the poppet share
        // appears on the final sample.
        let tip = analysis(20.0).dynamic_launch();
        assert_eq!(tip.end, LaunchEnd::TipUp);
        assert!(tip.samples.last().unwrap().poppet_load_kn > 0.0);
        // Stern lift grows as buoyancy builds.
        let lifts: Vec<f64> = r.samples.iter().map(|s| s.stern_lift_moment_knm).collect();
        assert!(lifts.last().unwrap() > lifts.first().unwrap());
        // The moment curve is finite everywhere, starts near zero (free
        // aft end), and is bounded by W x way length.
        let mags: Vec<f64> = r
            .samples
            .iter()
            .map(|s| s.end_of_ways_moment_knm.abs())
            .collect();
        let peak = mags.iter().cloned().fold(0.0, f64::max);
        assert!(peak > 0.0);
        let weight_knm = 2.0 * 4_000_000.0 * 9.81 / 1000.0 * 120.0; // x2: both sides of the way end contribute
        assert!(peak < weight_knm, "{peak} vs bound {weight_knm}");
        assert!(mags.first().unwrap() < &peak);
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
