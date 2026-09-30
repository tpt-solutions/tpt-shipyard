//! Welding simulation and distortion control for ship construction.
//!
//! The crate implements the classical analytical weld-model chain:
//!
//! 1. **Thermal cycle** — the Rosenthal quasi-stationary 3D moving point
//!    source on a semi-infinite plate, evaluated for a material point at a
//!    given transverse distance from the seam (temperature vs time, peak
//!    temperature, and the t8/5 cooling time that governs HAZ toughness).
//! 2. **Residual stress** — the longitudinal residual-stress profile after
//!    cooling: a yielding tension zone around the seam balanced by uniform
//!    compression, with the tension half-width set by the heat budget
//!    (mechanical-response-temperature energy balance).
//! 3. **Distortion** — transverse/longitudinal shrinkage and angular
//!    distortion from thermal contraction of the tension zone, including
//!    single-side groove imbalance.
//! 4. **Sequence optimisation** — relative ranking of candidate pass
//!    sequences by a heat-concentration heuristic (balanced, backstep-style
//!    sequences score better than concentrated ones).
//!
//! These are engineering-grade analytical models: fast enough for interactive
//! digital-twin use and verifiable against published experimental ranges, not
//! a replacement for thermo-mechanical FEM. RFC 0004 documents the model
//! choices and their verification.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::Material;
//! use tpt_yard_joints::{GrooveType, JointGeometry, JointKind};
//! use tpt_yard_welding::{WeldingSimulation, WeldProcess, WeldProcedure};
//!
//! let procedure = WeldProcedure {
//!     process: WeldProcess::Saw,
//!     heat_input_kj_mm: 12.0,
//!     travel_speed_mm_s: 8.0,
//!     preheat_temp_c: 50.0,
//!     interpass_temp_c: 150.0,
//!     filler_metal: "S2Si2 / SA AB1 47".into(),
//!     sequence: vec![],
//! };
//! let joint = JointGeometry::new(JointKind::Butt)
//!     .with_thickness_mm(12.0)
//!     .with_groove(GrooveType::V)
//!     .with_groove_angle_deg(60.0)
//!     .with_root_gap_mm(3.0)
//!     .with_root_face_mm(2.0);
//! let sim = WeldingSimulation::new(procedure, Material::ah36(), joint);
//!
//! let cycle = sim.thermal_cycle(10.0).unwrap(); // 10 mm from the seam
//! assert!(cycle.peak_temp_c > 200.0); // HAZ point well above ambient
//! assert!(cycle.t8_5_s.unwrap() > 0.0);
//! ```

use std::fmt;

use tpt_yard_core::{Material, Vector3};
use tpt_yard_joints::JointGeometry;

/// Arc/beam welding processes with their typical arc efficiency factors.
///
/// Efficiency η = fraction of electrical/beam power entering the plate as
/// heat; the values are classical ranges used for analytical heat-flow work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldProcess {
    /// Shielded Metal Arc Welding (stick), η ≈ 0.80.
    Smaw,
    /// Gas Metal Arc Welding (MIG/MAG), η ≈ 0.85.
    Gmaw,
    /// Gas Tungsten Arc Welding (TIG), η ≈ 0.60.
    Gtaw,
    /// Submerged Arc Welding, η ≈ 0.95.
    Saw,
    /// Flux-Cored Arc Welding, η ≈ 0.80.
    Fcaw,
    /// Electron Beam Welding (vacuum; space welds), η ≈ 0.90.
    Ebw,
    /// Laser beam welding, η ≈ 0.45 (keyhole absorption average).
    LaserWeld,
}

impl WeldProcess {
    /// Arc efficiency η (fraction of power entering the workpiece).
    pub fn efficiency(self) -> f64 {
        match self {
            WeldProcess::Smaw => 0.80,
            WeldProcess::Gmaw => 0.85,
            WeldProcess::Gtaw => 0.60,
            WeldProcess::Saw => 0.95,
            WeldProcess::Fcaw => 0.80,
            WeldProcess::Ebw => 0.90,
            WeldProcess::LaserWeld => 0.45,
        }
    }
}

impl fmt::Display for WeldProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            WeldProcess::Smaw => "SMAW",
            WeldProcess::Gmaw => "GMAW",
            WeldProcess::Gtaw => "GTAW",
            WeldProcess::Saw => "SAW",
            WeldProcess::Fcaw => "FCAW",
            WeldProcess::Ebw => "EBW",
            WeldProcess::LaserWeld => "laser",
        };
        f.write_str(s)
    }
}

/// One pass of a welding sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct WeldPass {
    /// Pass number within the sequence (1-based).
    pub id: u32,
    /// Heat input for this pass, kJ/mm.
    pub heat_input_kj_mm: f64,
    /// Travel direction along the seam; alternating directions
    /// (backstep-style sequences) spread the heat more evenly.
    pub direction: WeldDirection,
    /// Start point of the pass, mm, in the block coordinate system.
    pub start_mm: Vector3,
    /// End point of the pass, mm.
    pub end_mm: Vector3,
}

/// Travel direction of a pass along the seam axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldDirection {
    /// Along +X.
    Forward,
    /// Along −X (reverse pass).
    Reverse,
}

/// The qualified welding procedure (WPS subset used by the models).
#[derive(Debug, Clone, PartialEq)]
pub struct WeldProcedure {
    /// Welding process.
    pub process: WeldProcess,
    /// Arc energy per unit length, kJ/mm (`U·I / v`, the WPS/EN 1011
    /// convention). The net heat entering the plate is `η` times this
    /// value, with the process efficiency from [`WeldProcess::efficiency`].
    pub heat_input_kj_mm: f64,
    /// Travel speed, mm/s.
    pub travel_speed_mm_s: f64,
    /// Preheat temperature, °C.
    pub preheat_temp_c: f64,
    /// Maximum interpass temperature, °C.
    pub interpass_temp_c: f64,
    /// Filler metal designation.
    pub filler_metal: String,
    /// Pass sequence (empty for single-pass models).
    pub sequence: Vec<WeldPass>,
}

impl WeldProcedure {
    /// Loads a procedure from a JSON WPS record (see
    /// `test-data/welding-procedures/` for the schema).
    ///
    /// # Errors
    ///
    /// A JSON error string on malformed records.
    pub fn from_json(v: &tpt_yard_core::json::Value) -> Result<Self, String> {
        let get = |k: &str| -> Result<&tpt_yard_core::json::Value, String> {
            v.get(k).ok_or_else(|| format!("wps missing field '{k}'"))
        };
        let num = |k: &str| -> Result<f64, String> {
            get(k)?
                .as_f64()
                .ok_or_else(|| format!("wps field '{k}' must be a number"))
        };
        let process = match get("process")?.as_str().unwrap_or("") {
            "SMAW" => WeldProcess::Smaw,
            "GMAW" => WeldProcess::Gmaw,
            "GTAW" => WeldProcess::Gtaw,
            "SAW" => WeldProcess::Saw,
            "FCAW" => WeldProcess::Fcaw,
            "EBW" => WeldProcess::Ebw,
            "LaserWeld" => WeldProcess::LaserWeld,
            other => return Err(format!("unknown weld process '{other}'")),
        };
        Ok(Self {
            process,
            heat_input_kj_mm: num("heat_input_kj_mm")?,
            travel_speed_mm_s: num("travel_speed_mm_s")?,
            preheat_temp_c: num("preheat_temp_c")?,
            interpass_temp_c: num("interpass_temp_c")?,
            filler_metal: get("filler_metal")?
                .as_str()
                .unwrap_or_default()
                .to_string(),
            sequence: Vec::new(), // passes are loaded by the simulation builder
        })
    }
}

/// Result of the thermal-cycle analysis at one point.
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalCycle {
    /// Time (s, relative to the source passing the point) and temperature
    /// (°C) samples.
    pub samples: Vec<(f64, f64)>,
    /// Peak temperature reached, °C.
    pub peak_temp_c: f64,
    /// Cooling time from 800 °C to 500 °C (t8/5), s — governs HAZ micro-
    /// structure and toughness. `None` if the peak stays below 800 °C.
    pub t8_5_s: Option<f64>,
    /// Distance from the seam the cycle was evaluated at, mm.
    pub distance_from_weld_mm: f64,
}

/// Residual stress field after cooling (longitudinal-dominant model).
#[derive(Debug, Clone, PartialEq)]
pub struct ResidualStressField {
    /// (transverse distance y in mm, longitudinal stress σx in MPa) pairs.
    /// Positive = tension.
    pub profile: Vec<(f64, f64)>,
    /// Half-width of the yielding tension zone, mm.
    pub tension_half_width_mm: f64,
    /// Balancing compressive stress outside the tension zone, MPa.
    pub compressive_stress_mpa: f64,
    /// Peak tensile stress at the seam (= material yield), MPa.
    pub peak_tensile_mpa: f64,
}

/// Predicted distortion from one weld (or one sequence).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DistortionResult {
    /// Transverse (across the seam) shrinkage, mm.
    pub transverse_shrinkage_mm: f64,
    /// Longitudinal (along the seam) shrinkage, mm.
    pub longitudinal_shrinkage_mm: f64,
    /// Angular distortion (butterfly opening), degrees. Non-zero for
    /// single-side groove imbalance.
    pub angular_distortion_deg: f64,
    /// Out-of-plane bowing (longitudinal hogging/sagging) amplitude, mm.
    pub bowing_mm: f64,
    /// Model notes (assumptions in effect).
    pub notes: Vec<String>,
}

/// Errors produced by welding simulations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeldingError {
    /// Heat input must be positive.
    NonPositiveHeatInput,
    /// Travel speed must be positive for time-domain evaluation.
    NonPositiveTravelSpeed,
    /// The distance must be non-negative.
    NegativeDistance,
    /// A candidate sequence is empty.
    EmptySequence,
}

impl fmt::Display for WeldingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WeldingError::NonPositiveHeatInput => f.write_str("heat input must be > 0"),
            WeldingError::NonPositiveTravelSpeed => f.write_str("travel speed must be > 0"),
            WeldingError::NegativeDistance => f.write_str("distance must be >= 0"),
            WeldingError::EmptySequence => f.write_str("candidate sequence is empty"),
        }
    }
}

impl std::error::Error for WeldingError {}

/// A welding simulation: procedure + material + joint geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct WeldingSimulation {
    /// The qualified procedure.
    pub weld_procedure: WeldProcedure,
    /// Base material.
    pub material: Material,
    /// Joint geometry.
    pub joint_geometry: JointGeometry,
    /// Plate thickness for heat spreading, mm (defaults to the joint
    /// thickness).
    pub plate_thickness_mm: f64,
    /// Transverse width of the reference panel carrying the balancing
    /// compression and the bowing span, mm (default 2000 mm). A bare
    /// hard-coded value gave results that silently ignored the real panel.
    pub panel_width_mm: f64,
    /// Fraction of the melting temperature (Kelvin) above which the
    /// material has negligible mechanical response (default 0.8, the
    /// classical 0.8·T_m assumption — configurable because high-strength
    /// steels lose strength earlier).
    pub mech_response_frac: f64,
}

impl WeldingSimulation {
    /// Creates a simulation on a plate as thick as the joint.
    pub fn new(procedure: WeldProcedure, material: Material, joint: JointGeometry) -> Self {
        let plate = joint.thickness_mm;
        Self {
            weld_procedure: procedure,
            material,
            joint_geometry: joint,
            plate_thickness_mm: plate,
            panel_width_mm: 2000.0,
            mech_response_frac: 0.8,
        }
    }

    /// Builder: override the heat-spreading plate thickness, mm.
    #[must_use]
    pub fn with_plate_thickness_mm(mut self, t: f64) -> Self {
        self.plate_thickness_mm = t;
        self
    }

    /// Builder: override the reference panel width, mm.
    #[must_use]
    pub fn with_panel_width_mm(mut self, w: f64) -> Self {
        self.panel_width_mm = w.max(10.0);
        self
    }

    /// Builder: override the mechanical-response temperature fraction of
    /// the melting point (clamped to (0, 1]).
    #[must_use]
    pub fn with_mech_response_frac(mut self, frac: f64) -> Self {
        self.mech_response_frac = frac.clamp(1e-3, 1.0);
        self
    }

    /// Total heat input of the whole sequence (or the single procedure pass),
    /// kJ/mm.
    pub fn total_heat_input_kj_mm(&self) -> f64 {
        if self.weld_procedure.sequence.is_empty() {
            self.weld_procedure.heat_input_kj_mm
        } else {
            self.weld_procedure
                .sequence
                .iter()
                .map(|p| p.heat_input_kj_mm)
                .sum()
        }
    }

    /// Effective net power entering the plate, W:
    /// `η · (heat input per length) × travel speed`, with the process arc
    /// efficiency η from [`WeldProcess::efficiency`].
    pub fn net_power_w(&self) -> Result<f64, WeldingError> {
        let v = self.weld_procedure.travel_speed_mm_s;
        if !(v > 0.0) {
            return Err(WeldingError::NonPositiveTravelSpeed);
        }
        if !(self.weld_procedure.heat_input_kj_mm > 0.0) {
            return Err(WeldingError::NonPositiveHeatInput);
        }
        let eta = self.weld_procedure.process.efficiency();
        Ok(eta * self.weld_procedure.heat_input_kj_mm * 1000.0 * v)
    }

    /// Temperature history at a material point `distance_from_weld_mm` off
    /// the seam, using the Rosenthal quasi-stationary 3D moving point source:
    ///
    /// ```text
    /// T - T0 = Q / (2π k R) · exp(−v (R + ξ) / (2α)),   ξ = −v t,  R = √(ξ² + d²)
    /// ```
    ///
    /// where Q is net power (W, after the arc efficiency η), k conductivity
    /// (W/mK), α diffusivity (m²/s), v travel speed (mm/s → m/s), d the
    /// transverse distance (mm → m). The window spans −t_span to +t_span
    /// around the source transit and widens automatically until the tail has
    /// cooled below 450 °C, so the 800→500 °C leg is fully captured.
    ///
    /// # Errors
    ///
    /// [`WeldingError`] on non-positive (or NaN) heat input/speed or
    /// negative distance.
    pub fn thermal_cycle(&self, distance_from_weld_mm: f64) -> Result<ThermalCycle, WeldingError> {
        if !(self.weld_procedure.heat_input_kj_mm > 0.0) {
            return Err(WeldingError::NonPositiveHeatInput);
        }
        if !(self.weld_procedure.travel_speed_mm_s > 0.0) {
            return Err(WeldingError::NonPositiveTravelSpeed);
        }
        if !(distance_from_weld_mm >= 0.0) {
            return Err(WeldingError::NegativeDistance);
        }

        let q = self.net_power_w()?;
        let k = self.material.conductivity_w_mk;
        let alpha = self.material.thermal_diffusivity_m2_s();
        let v = self.weld_procedure.travel_speed_mm_s / 1000.0; // m/s
        // The 3D point source is singular at the seam; keep the evaluation
        // point a hair off it so temperatures stay finite.
        let d = (distance_from_weld_mm.max(1e-3)) / 1000.0;
        let t0_c = self.weld_procedure.preheat_temp_c;

        // Sample window: the Rosenthal 3D tail decays like 1/R, so the
        // 800->500 C cooling leg runs for `~Q / (2 pi k · 480 C)` metres of
        // source travel behind the weld (longer for heavy heat input). Start
        // from the local transit time and keep doubling until the whole
        // cooling leg fits inside the window (or the extension cap trips).
        let base_span = (20.0 * d / v).max(1.0 / v);
        let leg_limit = (q / (2.0 * std::f64::consts::PI * k * 480.0) / v) * 1.5;
        let n = 400;
        let sample = |t_span: f64| -> Vec<(f64, f64)> {
            (0..=n)
                .map(|i| {
                    let t = -t_span + 2.0 * t_span * i as f64 / n as f64;
                    let xi = -v * t; // source passes at t = 0
                    let r = (xi * xi + d * d).sqrt();
                    let temperature = t0_c
                        + (q / (2.0 * std::f64::consts::PI * k * r))
                            * (-v * (r + xi) / (2.0 * alpha)).exp();
                    (t, temperature)
                })
                .collect()
        };
        // Double the window while the tail end is still hot: the point must
        // cool below the 500 C t8/5 threshold (with margin) inside the
        // sampled span, else the crossing detection would silently truncate.
        let mut t_span = base_span.max(leg_limit);
        let mut samples = sample(t_span);
        for _ in 0..24 {
            if samples.last().is_none_or(|s| s.1 < 450.0) {
                break;
            }
            t_span *= 2.0;
            samples = sample(t_span);
        }
        let peak = samples.iter().map(|s| s.1).fold(f64::MIN, f64::max);

        // t8/5 from the samples (cooling branch).
        let t8_5 = crossing_time_after(&samples, 800.0)
            .zip(crossing_time_after(&samples, 500.0))
            .map(|(t8, t5)| t5 - t8)
            .filter(|dt| *dt > 0.0);

        Ok(ThermalCycle {
            samples,
            peak_temp_c: peak,
            t8_5_s: t8_5,
            distance_from_weld_mm,
        })
    }

    /// Peak temperature at a distance from the seam, °C (same Rosenthal
    /// solution evaluated at the source transit).
    ///
    /// # Errors
    ///
    /// [`WeldingError::NegativeDistance`] on a negative or NaN distance,
    /// [`WeldingError::NonPositiveHeatInput`] / [`WeldingError::NonPositiveTravelSpeed`]
    /// on an invalid procedure.
    pub fn peak_temperature_at(&self, distance_from_weld_mm: f64) -> Result<f64, WeldingError> {
        if !(distance_from_weld_mm >= 0.0) {
            return Err(WeldingError::NegativeDistance);
        }
        let q = self.net_power_w()?;
        let k = self.material.conductivity_w_mk;
        let alpha = self.material.thermal_diffusivity_m2_s();
        let v = self.weld_procedure.travel_speed_mm_s / 1000.0;
        // Clamp off the singular seam point (see `thermal_cycle`).
        let d = distance_from_weld_mm.max(1e-3) / 1000.0;
        let exponent = -v * d / (2.0 * alpha);
        Ok(self.weld_procedure.preheat_temp_c
            + q / (2.0 * std::f64::consts::PI * k * d) * exponent.exp())
    }

    /// Longitudinal residual stress field after cooling.
    ///
    /// Model (documented in RFC 0004): the tension zone (`|y| ≤ b`) yields at
    /// ±σ_yield with the classical parabolic roll-off; the remainder of the
    /// plate carries uniform compression balancing the tension zone's force.
    /// The half-width `b` is the transverse isotherm of the mechanical
    /// response temperature `T_mech ≈ 0.8·T_melt` (Kelvin) taken from the
    /// Rosenthal peak-temperature solution — material beyond that isotherm
    /// never yields, so it carries no residual tension.
    ///
    /// # Errors
    ///
    /// [`WeldingError::NonPositiveHeatInput`] when there is no heat.
    pub fn residual_stress(&self) -> Result<ResidualStressField, WeldingError> {
        if self.weld_procedure.heat_input_kj_mm <= 0.0 {
            return Err(WeldingError::NonPositiveHeatInput);
        }
        let t_mech_k = self.mechanical_response_temp_k();
        let b_mm = self.isotherm_half_width_mm(t_mech_k);

        let sigma_y = self.material.yield_mpa;
        // Profile: σ(y) = σy·(1 − (y/b)²) for |y| ≤ b; uniform compression
        // outside balancing the tension force (parabola mean = 2/3 σy).
        let tension_force = sigma_y * (2.0 / 3.0) * (2.0 * b_mm);
        let panel = self.panel_width_mm;
        let comp = -(tension_force / (panel - 2.0 * b_mm).max(1.0));

        let mut profile = Vec::with_capacity(81);
        let half = panel / 2.0;
        for i in 0..=80 {
            let y = -half + panel * i as f64 / 80.0;
            let s = if y.abs() <= b_mm {
                sigma_y * (1.0 - (y / b_mm).powi(2))
            } else {
                comp
            };
            profile.push((y, s));
        }
        Ok(ResidualStressField {
            profile,
            tension_half_width_mm: b_mm,
            compressive_stress_mpa: comp,
            peak_tensile_mpa: sigma_y,
        })
    }

    /// Predicted distortion for the current procedure/sequence.
    ///
    /// Calibrated contraction model (constants and verification in RFC 0004):
    ///
    /// - **Transverse shrinkage** — heat injected per metre of seam, spread
    ///   through the plate thickness, drives the contraction:
    ///   `dT = 0.5 * alpha * Qp / (rho c t)` (Qp in J/m, t in m).
    /// - **Longitudinal shrinkage** — the weld cross-section contracting by
    ///   `alpha * dT_mech` against the panel cross-section:
    ///   `dL = alpha * dT_mech * (A_weld / A_panel) * L`.
    /// - **Angular distortion** — weld metal deposited off the mid-plane
    ///   rotates the joint: `beta = 2 * alpha * dT_mech * (A_off / t^2) * e`,
    ///   with eccentricity `e` = 1.0 for single-side grooves, 0.25 for
    ///   balanced double-side grooves.
    /// - **Bowing** — longitudinal shrinkage acting at mid-thickness of the
    ///   2 m reference panel.
    ///
    /// # Errors
    ///
    /// [`WeldingError::NonPositiveHeatInput`] when there is no heat.
    pub fn distortion(&self) -> Result<DistortionResult, WeldingError> {
        if self.total_heat_input_kj_mm() <= 0.0 {
            return Err(WeldingError::NonPositiveHeatInput);
        }
        let t0_k = self.weld_procedure.preheat_temp_c + 273.15;
        let t_mech_k = self.mechanical_response_temp_k();
        let d_t = t_mech_k - t0_k;
        let alpha = self.material.thermal_expansion_1_k;

        // Transverse: net heat per metre of seam (J/m); t in m.
        let eta = self.weld_procedure.process.efficiency();
        let q_per_m = eta * self.weld_procedure.heat_input_kj_mm * 1e6; // kJ/mm -> net J/m
        let t_m = self.plate_thickness_mm / 1000.0;
        let transverse_m = 0.5 * alpha * q_per_m
            / (self.material.density_kg_m3 * self.material.specific_heat_j_kg_k * t_m);

        // Longitudinal: weld area contracting against the panel section.
        let a_weld = self.joint_geometry.weld_area_mm2_total(); // mm^2
        let a_panel = self.panel_width_mm * self.plate_thickness_mm; // panel section, mm^2
        let length_mm = self.joint_geometry.length_mm;
        let longitudinal = alpha * d_t * (a_weld / a_panel) * length_mm;

        // Angular: off-mid-plane weld area rotates the joint.
        let single_side = matches!(
            self.joint_geometry.groove,
            tpt_yard_joints::GrooveType::V
                | tpt_yard_joints::GrooveType::Bevel
                | tpt_yard_joints::GrooveType::J
                | tpt_yard_joints::GrooveType::U
        );
        let a_off = a_weld.min(self.joint_geometry.thickness_mm * self.plate_thickness_mm);
        let ecc = if single_side { 1.0 } else { 0.25 };
        let t_sq = self.plate_thickness_mm * self.plate_thickness_mm;
        let angular_rad = 2.0 * alpha * d_t * (a_off / t_sq) * ecc;

        // Bowing: longitudinal shrinkage eccentricity over the panel span
        // (the reference bow arm is a quarter of the panel width).
        let bow = longitudinal * (self.plate_thickness_mm / 2.0) / (self.panel_width_mm / 4.0);

        let mut notes = vec![
            "calibrated contraction model (RFC 0004)".to_string(),
            format!(
                "tension half-width {:.1} mm at T_mech = {:.0} K",
                self.isotherm_half_width_mm(t_mech_k),
                t_mech_k
            ),
        ];
        if single_side {
            notes.push("single-side groove: angular distortion dominant".to_string());
        }
        Ok(DistortionResult {
            transverse_shrinkage_mm: transverse_m * 1000.0,
            longitudinal_shrinkage_mm: longitudinal.abs(),
            angular_distortion_deg: angular_rad.to_degrees(),
            bowing_mm: bow.abs(),
            notes,
        })
    }

    /// Mechanical response temperature (Kelvin): above ~0.8 of the melting
    /// point the material has negligible strength, so cooling from there
    /// produces residual effects.
    fn mechanical_response_temp_k(&self) -> f64 {
        self.mech_response_frac.clamp(1e-3, 1.0)
            * (self.material.melting_point_c + 273.15)
    }

    /// Transverse half-width (mm) at which the Rosenthal peak temperature
    /// falls below `temp_k` — the isotherm of `temp_k`. Bisection on the
    /// monotone peak-temperature profile; clamped to [0.01, 1000] mm.
    fn isotherm_half_width_mm(&self, temp_k: f64) -> f64 {
        let below =
            |y_mm: f64| self.peak_temperature_at(y_mm).unwrap_or(f64::MIN) < temp_k - 273.15;
        if below(0.01) {
            return 0.01;
        }
        let (mut lo, mut hi) = (0.01_f64, 1000.0_f64);
        if !below(hi) {
            return hi; // enormous heat input: isotherm beyond the bound
        }
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if below(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        0.5 * (lo + hi)
    }

    /// Distortion including every pass of the procedure's sequence.
    ///
    /// Superposition model: each pass adds its own heat and its own weld
    /// area. Transverse shrinkage is linear in heat, so it scales with the
    /// total-to-base *heat* ratio; longitudinal shrinkage and angular
    /// distortion scale with the number of deposited weld areas (pass
    /// count), not with the heat ratio — the previous version scaled every
    /// term by the heat ratio, which double-counted the base pass's heat in
    /// the non-linear terms.
    ///
    /// # Errors
    ///
    /// [`WeldingError::NonPositiveHeatInput`] when the sequence has no heat.
    pub fn distortion_of_sequence(&self) -> Result<DistortionResult, WeldingError> {
        let base = self.distortion()?;
        if self.weld_procedure.sequence.is_empty() {
            return Ok(base);
        }
        let n = self.weld_procedure.sequence.len() as f64;
        let base_heat = self.weld_procedure.heat_input_kj_mm.max(1e-9);
        let total_heat: f64 = self
            .weld_procedure
            .sequence
            .iter()
            .map(|p| p.heat_input_kj_mm)
            .sum();
        let heat_scale = total_heat / base_heat;
        Ok(DistortionResult {
            transverse_shrinkage_mm: base.transverse_shrinkage_mm * heat_scale,
            longitudinal_shrinkage_mm: base.longitudinal_shrinkage_mm * n,
            angular_distortion_deg: base.angular_distortion_deg * n,
            bowing_mm: base.bowing_mm * n,
            notes: base
                .notes
                .into_iter()
                .chain(std::iter::once(format!(
                    "{n:.0} passes, {total_heat:.1} kJ/mm total (transverse scales with heat, shrinkage/rotation with deposited area)"
                )))
                .collect(),
        })
    }

    /// Plate-regime check for the 3-D point-source model.
    ///
    /// The Rosenthal semi-infinite solution is valid while the melting
    /// isotherm fits inside the plate thickness; when it reaches the far
    /// surface the plate is thermally *thin* (2-D spreading, adiabatic
    /// bottom) and the model over-predicts peak temperature. Returns a
    /// warning string in that regime, `None` for a thermally thick plate.
    ///
    /// # Errors
    ///
    /// [`WeldingError::NonPositiveHeatInput`] when there is no heat.
    pub fn thin_plate_warning(&self) -> Result<Option<String>, WeldingError> {
        if !(self.weld_procedure.heat_input_kj_mm > 0.0) {
            return Err(WeldingError::NonPositiveHeatInput);
        }
        let t_melt_k = self.material.melting_point_c + 273.15;
        let melt_isotherm_mm = self.isotherm_half_width_mm(t_melt_k);
        if melt_isotherm_mm >= self.plate_thickness_mm {
            Ok(Some(format!(
                "melting isotherm ({melt_isotherm_mm:.1} mm) reaches the {:.1} mm plate far side: thermally thin plate — the 3-D point source over-predicts temperature; use a 2-D solution",
                self.plate_thickness_mm
            )))
        } else {
            Ok(None)
        }
    }

    /// Ranks candidate sequences and returns the one with the lowest
    /// estimated distortion.
    ///
    /// Scoring heuristic (RFC 0004): concentrated heat builds up local
    /// distortion — each pair of passes that runs near each other
    /// (longitudinally, in sequence order) with the *same* direction adds a
    /// coupling penalty; alternating-direction, spread-out sequences score
    /// best. This ranks sequences relative to each other, exactly what
    /// sequence selection needs; absolute distortion comes from `distortion`.
    ///
    /// # Errors
    ///
    /// [`WeldingError::EmptySequence`] if `possible_sequences` is empty.
    pub fn welding_sequence_optimization(
        &self,
        possible_sequences: &[Vec<WeldPass>],
    ) -> Result<Vec<WeldPass>, WeldingError> {
        if possible_sequences.is_empty() {
            return Err(WeldingError::EmptySequence);
        }
        let mut best: Option<(f64, &Vec<WeldPass>)> = None;
        for seq in possible_sequences {
            let score = self.sequence_score(seq);
            if best.is_none_or(|(bs, _)| score < bs) {
                best = Some((score, seq));
            }
        }
        Ok(best
            .map(|(_, s)| s.clone())
            .expect("sequences is non-empty"))
    }

    fn sequence_score(&self, seq: &[WeldPass]) -> f64 {
        let mut score = 0.0;
        for (i, a) in seq.iter().enumerate() {
            for b in &seq[..i] {
                // Distance between pass centres along the seam.
                let ca = (a.start_mm.x + a.end_mm.x) * 0.5;
                let cb = (b.start_mm.x + b.end_mm.x) * 0.5;
                let gap = (ca - cb).abs().max(1.0);
                // Same direction & nearby = heat concentrates.
                let same_dir = a.direction == b.direction;
                score += a.heat_input_kj_mm
                    * b.heat_input_kj_mm
                    * (-gap / 500.0).exp()
                    * if same_dir { 1.0 } else { 0.5 };
            }
            // Later passes at high heat add angular distortion on single-side
            // grooves.
        }
        // Alternating directions earn a small structural bonus.
        let alternations = seq
            .windows(2)
            .filter(|w| w[0].direction != w[1].direction)
            .count();
        score * (1.0 - 0.05 * alternations as f64 / seq.len().max(1) as f64)
    }
}

/// First time (s) after the peak where the temperature drops below `temp_c`.
fn crossing_time_after(samples: &[(f64, f64)], temp_c: f64) -> Option<f64> {
    let peak_idx = samples
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)?;
    for w in samples[peak_idx..].windows(2) {
        let (t0, t1) = (w[0].1, w[1].1);
        if t0 >= temp_c && t1 < temp_c {
            let frac = (t0 - temp_c) / (t0 - t1).max(f64::MIN_POSITIVE);
            return Some(w[0].0 + frac * (w[1].0 - w[0].0));
        }
    }
    None
}

/// Base-metal chemistry for weldability assessment, weight percent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SteelChemistry {
    /// Carbon, wt %.
    pub c: f64,
    /// Silicon, wt %.
    pub si: f64,
    /// Manganese, wt %.
    pub mn: f64,
    /// Chromium, wt %.
    pub cr: f64,
    /// Molybdenum, wt %.
    pub mo: f64,
    /// Nickel, wt %.
    pub ni: f64,
    /// Copper, wt %.
    pub cu: f64,
    /// Vanadium, wt %.
    pub v: f64,
}

impl SteelChemistry {
    /// Carbon equivalent per the IIW relation (EN 1011-2):
    /// `CE = C + Mn/6 + (Cr + Mo + V)/5 + (Ni + Cu)/15`.
    pub fn carbon_equivalent_iiw(&self) -> f64 {
        self.c + self.mn / 6.0 + (self.cr + self.mo + self.v) / 5.0
            + (self.ni + self.cu) / 15.0
    }

    /// AH36-like shipbuilding steel: C 0.16, Mn 1.4, Si 0.40, traces.
    pub fn ah36_like() -> Self {
        Self {
            c: 0.16,
            si: 0.40,
            mn: 1.40,
            cr: 0.02,
            mo: 0.0,
            ni: 0.02,
            cu: 0.02,
            v: 0.005,
        }
    }
}

/// Preheat recommendation from the SEW 088-style screening table
/// (carbon equivalent vs plate thickness), as referenced by EN 1011-2
/// guidance for hardness-limited weldable structural steels with
/// low-hydrogen consumables.
#[derive(Debug, Clone, PartialEq)]
pub struct PreheatAdvice {
    /// The IIW carbon equivalent used.
    pub carbon_equivalent: f64,
    /// Recommended minimum preheat/interpass temperature, °C.
    pub recommended_preheat_c: f64,
    /// How the figure was reached (table cell, hydrogen adjustment).
    pub rationale: String,
    /// True when the WPS preheat (if a procedure was supplied) meets the
    /// recommendation.
    pub procedure_adequate: Option<bool>,
}

/// Recommends a minimum preheat for a carbon-manganese steel plate.
///
/// The SEW 088-style screening table (°C):
///
/// ```text
/// CEV \ t (mm):  <=10  10-20  20-30  30-40  40-50   >50
/// < 0.39:          20     20    100    100    100    150
/// 0.39 - 0.44:     20    100    100    100    150    200
/// 0.44 - 0.49:    100    100    150    150    200    200
/// >= 0.49:        150    150    200    200    200    200
/// ```
///
/// A non-low-hydrogen process (not GTAW/GMAW/SAW with basic consumables —
/// modelled here by `low_hydrogen = false`) moves one row up: hydrogen
/// cracking risk rises with diffusible hydrogen, so the table's next-higher
/// CEV band applies.
pub fn advise_preheat(
    chemistry: &SteelChemistry,
    thickness_mm: f64,
    low_hydrogen: bool,
) -> PreheatAdvice {
    let ce = chemistry.carbon_equivalent_iiw();
    // Table row: 0 = <0.39, 1 = 0.39-0.44, 2 = 0.44-0.49, 3 = >=0.49.
    let mut row = if ce < 0.39 {
        0
    } else if ce < 0.44 {
        1
    } else if ce < 0.49 {
        2
    } else {
        3
    };
    let mut rationale = format!("CEV {ce:.3}");
    if !low_hydrogen {
        row = (row + 1).min(3);
        rationale.push_str(", non-low-hydrogen process: one CEV band worse");
    }
    // Column by thickness.
    let col = match thickness_mm {
        t if t <= 10.0 => 0,
        t if t <= 20.0 => 1,
        t if t <= 30.0 => 2,
        t if t <= 40.0 => 3,
        t if t <= 50.0 => 4,
        _ => 5,
    };
    const TABLE: [[f64; 6]; 4] = [
        [20.0, 20.0, 100.0, 100.0, 100.0, 150.0],
        [20.0, 100.0, 100.0, 100.0, 150.0, 200.0],
        [100.0, 100.0, 150.0, 150.0, 200.0, 200.0],
        [150.0, 150.0, 200.0, 200.0, 200.0, 200.0],
    ];
    rationale.push_str(&format!(
        ", t {thickness_mm:.0} mm -> SEW 088-style band"
    ));
    PreheatAdvice {
        carbon_equivalent: ce,
        recommended_preheat_c: TABLE[row][col],
        rationale,
        procedure_adequate: None,
    }
}

/// Checks a qualified WPS against the preheat recommendation for its
/// material and joint: `Some(false)` when the procedure's preheat is below
/// the recommendation, `Some(true)` when it meets it.
pub fn check_wps_preheat(
    chemistry: &SteelChemistry,
    procedure: &WeldProcedure,
    thickness_mm: f64,
    low_hydrogen: bool,
) -> PreheatAdvice {
    let mut advice = advise_preheat(chemistry, thickness_mm, low_hydrogen);
    let adequate = procedure.preheat_temp_c + 1e-9 >= advice.recommended_preheat_c;
    advice.procedure_adequate = Some(adequate);
    if !adequate {
        advice.rationale.push_str(&format!(
            "; WPS preheat {:.0} °C is BELOW the recommendation",
            procedure.preheat_temp_c
        ));
    } else {
        advice.rationale.push_str(&format!(
            "; WPS preheat {:.0} °C meets it",
            procedure.preheat_temp_c
        ));
    }
    advice
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_joints::{GrooveType, JointKind};

    fn saw_procedure(heat_kj_mm: f64, thickness_mm: f64) -> WeldingSimulation {
        let procedure = WeldProcedure {
            process: WeldProcess::Saw,
            heat_input_kj_mm: heat_kj_mm,
            travel_speed_mm_s: 8.0,
            preheat_temp_c: 20.0,
            interpass_temp_c: 150.0,
            filler_metal: "S2Si2".into(),
            sequence: vec![],
        };
        let joint = JointGeometry::new(JointKind::Butt)
            .with_thickness_mm(thickness_mm)
            .with_groove(GrooveType::V)
            .with_groove_angle_deg(60.0)
            .with_root_gap_mm(3.0)
            .with_root_face_mm(2.0);
        WeldingSimulation::new(procedure, Material::ah36(), joint)
    }

    #[test]
    fn rosenthal_peak_decays_with_distance() {
        let sim = saw_procedure(12.0, 12.0);
        let at_weld = sim.peak_temperature_at(1.0).unwrap();
        let near = sim.peak_temperature_at(10.0).unwrap();
        let far = sim.peak_temperature_at(40.0).unwrap();
        assert!(at_weld > near && near > far, "{at_weld} {near} {far}");
        // At the weld line the plate must melt (Rosenthal point source is
        // singular at d=0; at 1 mm it should exceed melting for SAW input).
        assert!(at_weld > 1500.0);
        // Far enough away, barely warm.
        assert!(far < 300.0, "far field {far}");
    }

    #[test]
    fn thermal_cycle_peak_and_t85() {
        let sim = saw_procedure(12.0, 12.0);
        let cycle = sim.thermal_cycle(8.0).unwrap();
        assert!(cycle.peak_temp_c > 800.0, "HAZ at 8 mm should exceed 800 C");
        let t85 = cycle.t8_5_s.expect("crosses both 800 and 500");
        assert!(t85 > 0.0);
        // Known property of the 3D point source: t8/5 is (nearly) independent
        // of the distance — it scales with heat input instead. Verify the
        // heat-input scaling: more heat per mm, longer cooling leg.
        let heavy = saw_procedure(20.0, 12.0);
        let t85_heavy = heavy.thermal_cycle(8.0).unwrap().t8_5_s.unwrap();
        assert!(
            t85_heavy > t85,
            "heat input must lengthen t8/5: {t85_heavy} vs {t85}"
        );
    }

    #[test]
    fn residual_stress_profile_is_physical() {
        let sim = saw_procedure(12.0, 12.0);
        let field = sim.residual_stress().unwrap();
        // Peak tensile = yield at the seam.
        assert!((field.peak_tensile_mpa - Material::ah36().yield_mpa).abs() < 1e-9);
        // Compression is negative and smaller in magnitude than yield.
        assert!(field.compressive_stress_mpa < 0.0);
        assert!(field.compressive_stress_mpa.abs() < Material::ah36().yield_mpa);
        // Tension zone is finite and positive.
        assert!(field.tension_half_width_mm > 0.0);
        // Profile: tension inside b, compression far away, zero crossing.
        let at_seam = field.profile.iter().find(|(y, _)| y.abs() < 1.0).unwrap().1;
        assert!(at_seam > 0.9 * field.peak_tensile_mpa);
        let far = field.profile.last().unwrap().1;
        assert!(far < 0.0);
        // More heat → wider tension zone.
        let hot = saw_procedure(20.0, 12.0).residual_stress().unwrap();
        assert!(hot.tension_half_width_mm > field.tension_half_width_mm);
    }

    /// Verification (review 7H): the IIW carbon equivalent is exact.
    #[test]
    fn carbon_equivalent_is_exact() {
        let chem = SteelChemistry {
            c: 0.16,
            si: 0.40,
            mn: 1.40,
            cr: 0.02,
            mo: 0.00,
            ni: 0.02,
            cu: 0.02,
            v: 0.005,
        };
        // CE = 0.16 + 1.4/6 + (0.02+0+0.005)/5 + (0.02+0.02)/15.
        let expected = 0.16 + 1.40 / 6.0 + 0.025 / 5.0 + 0.04 / 15.0;
        assert!((chem.carbon_equivalent_iiw() - expected).abs() < 1e-12);
        assert!((chem.carbon_equivalent_iiw() - 0.4010).abs() < 1e-3);
    }

    /// Verification (review 7H): the SEW 088-style preheat table — thin
    /// AH36-like plate needs no preheat, thick plate or a dirtier heat does,
    /// and non-low-hydrogen processes move a band worse.
    #[test]
    fn preheat_table_follows_ce_and_thickness() {
        let mild = SteelChemistry::ah36_like(); // CEV ~ 0.405
        // 12 mm, low hydrogen: row 1 (0.39-0.44), col 1 -> 100 C.
        assert_eq!(advise_preheat(&mild, 12.0, true).recommended_preheat_c, 100.0);
        // 8 mm: col 0 -> 20 C (no preheat).
        assert_eq!(advise_preheat(&mild, 8.0, true).recommended_preheat_c, 20.0);
        // 35 mm: col 3 -> 100 C.
        assert_eq!(advise_preheat(&mild, 35.0, true).recommended_preheat_c, 100.0);
        // Non-low-hydrogen: bumped to row 2 -> 150 C at 35 mm.
        assert_eq!(advise_preheat(&mild, 35.0, false).recommended_preheat_c, 150.0);
        // A hot heat (CEV >= 0.49): top row everywhere.
        let hot = SteelChemistry { c: 0.25, mn: 1.65, si: 0.5, cr: 0.05, mo: 0.02, ni: 0.0, cu: 0.0, v: 0.01 };
        assert!(hot.carbon_equivalent_iiw() >= 0.49, "{}", hot.carbon_equivalent_iiw());
        assert_eq!(advise_preheat(&hot, 12.0, true).recommended_preheat_c, 150.0);
        assert_eq!(advise_preheat(&hot, 45.0, true).recommended_preheat_c, 200.0);
    }

    /// The WPS preheat check flags an underheated procedure and passes an
    /// adequate one.
    #[test]
    fn wps_preheat_check_flags_underheated_procedure() {
        let chem = SteelChemistry::ah36_like();
        let mut procedure = saw_procedure(12.0, 12.0).weld_procedure;
        procedure.preheat_temp_c = 20.0;
        let advice = check_wps_preheat(&chem, &procedure, 12.0, true);
        assert_eq!(advice.procedure_adequate, Some(false), "{}", advice.rationale);
        procedure.preheat_temp_c = 100.0;
        let advice = check_wps_preheat(&chem, &procedure, 12.0, true);
        assert_eq!(advice.procedure_adequate, Some(true));
    }

    /// Verification against published experimental ranges for a reference
    /// AH36 panel (see test-data/golden/sea/welding-distortion-panel.json
    /// and RFC 0004): single-V SAW butt weld, 12 mm plate.
    #[test]
    fn test_welding_distortion() {
        let sim = saw_procedure(12.0, 12.0);
        let d = sim.distortion().unwrap();
        // Transverse shrinkage for a 12 mm single-V butt: ~0.5–2 mm.
        assert!(
            d.transverse_shrinkage_mm > 0.2 && d.transverse_shrinkage_mm < 2.5,
            "transverse {}",
            d.transverse_shrinkage_mm
        );
        // Angular distortion for single-side V: ~0.5–4 degrees.
        assert!(
            d.angular_distortion_deg > 0.2 && d.angular_distortion_deg < 4.0,
            "angular {}",
            d.angular_distortion_deg
        );
        // Double-sided (balanced X groove) should halve the angular change.
        let mut x_sim = sim.clone();
        x_sim.joint_geometry.groove = GrooveType::DoubleV;
        let dx = x_sim.distortion().unwrap();
        assert!(dx.angular_distortion_deg < d.angular_distortion_deg);
    }

    #[test]
    fn sequence_optimization_prefers_balanced_heat() {
        let sim = saw_procedure(12.0, 12.0);
        let mk = |i: u32, x: f64, dir| WeldPass {
            id: i,
            heat_input_kj_mm: 6.0,
            direction: dir,
            start_mm: Vector3::new(x, 0.0, 0.0),
            end_mm: Vector3::new(x + 500.0, 0.0, 0.0),
        };
        // Concentrated: all passes same place, same direction.
        let concentrated = vec![
            mk(1, 0.0, WeldDirection::Forward),
            mk(2, 0.0, WeldDirection::Forward),
            mk(3, 0.0, WeldDirection::Forward),
        ];
        // Balanced: spread out, alternating directions.
        let balanced = vec![
            mk(1, 0.0, WeldDirection::Forward),
            mk(2, 600.0, WeldDirection::Reverse),
            mk(3, 1200.0, WeldDirection::Forward),
        ];
        let chosen = sim
            .welding_sequence_optimization(&[concentrated.clone(), balanced.clone()])
            .unwrap();
        assert_eq!(chosen, balanced);
        assert!(sim.sequence_score(&balanced) < sim.sequence_score(&concentrated));
    }

    #[test]
    fn multi_pass_scales_distortion() {
        let mut sim = saw_procedure(6.0, 20.0);
        let mk = |i: u32| WeldPass {
            id: i,
            heat_input_kj_mm: 6.0,
            direction: if i.is_multiple_of(2) {
                WeldDirection::Reverse
            } else {
                WeldDirection::Forward
            },
            start_mm: Vector3::ZERO,
            end_mm: Vector3::new(1000.0, 0.0, 0.0),
        };
        sim.weld_procedure.sequence = vec![mk(1), mk(2), mk(3)];
        let multi = sim.distortion_of_sequence().unwrap();
        sim.weld_procedure.sequence.clear();
        let single = sim.distortion().unwrap();
        assert!(
            (multi.transverse_shrinkage_mm - single.transverse_shrinkage_mm * 3.0).abs() < 1e-9
        );
        // Equal heats: area-driven terms also scale by pass count.
        assert!(
            (multi.longitudinal_shrinkage_mm - single.longitudinal_shrinkage_mm * 3.0).abs()
                < 1e-9
        );
    }

    /// Regression (review 7B): the shrinkage/rotation terms scale with the
    /// number of deposited weld areas (pass count), NOT with the heat ratio
    /// — two low-heat passes are not one high-heat pass for angular
    /// distortion, even at the same total heat.
    #[test]
    fn sequence_superposition_distinguishes_heat_from_area() {
        // One 12 kJ/mm pass vs two 6 kJ/mm passes: same total heat.
        let single = saw_procedure(12.0, 12.0);
        let mut doubled = saw_procedure(6.0, 12.0);
        let mk = |i: u32| WeldPass {
            id: i,
            heat_input_kj_mm: 6.0,
            direction: WeldDirection::Forward,
            start_mm: Vector3::ZERO,
            end_mm: Vector3::new(1000.0, 0.0, 0.0),
        };
        doubled.weld_procedure.sequence = vec![mk(1), mk(2)];
        let d1 = single.distortion_of_sequence().unwrap();
        let d2 = doubled.distortion_of_sequence().unwrap();
        // Same heat in: same transverse shrinkage.
        assert!(
            (d1.transverse_shrinkage_mm - d2.transverse_shrinkage_mm).abs() < 1e-9,
            "{:?} vs {:?}",
            d1.transverse_shrinkage_mm,
            d2.transverse_shrinkage_mm
        );
        // Two weld areas deposited: double the rotation, not equal.
        assert!(
            (d2.angular_distortion_deg - 2.0 * d1.angular_distortion_deg).abs() < 1e-9,
            "two passes at the same total heat must rotate twice: {} vs {}",
            d2.angular_distortion_deg,
            d1.angular_distortion_deg
        );
    }

    /// Regression (review 7B): the plate-regime check flags thermally thin
    /// plates where the melting isotherm reaches the far surface.
    #[test]
    fn thin_plate_warning_fires() {
        // 12 mm reference plate at 12 kJ/mm: melting isotherm well inside.
        let thick = saw_procedure(12.0, 12.0);
        assert!(thick.thin_plate_warning().unwrap().is_none());
        // 2 mm sheet with the same line heat: isotherm saturates the plate.
        let thin = saw_procedure(12.0, 2.0);
        let warn = thin.thin_plate_warning().unwrap();
        assert!(warn.is_some(), "2 mm sheet at 12 kJ/mm must warn");
        assert!(warn.unwrap().contains("thermally thin"));
        // The builder parameters are honoured: a *higher* response
        // fraction (isotherm closer to the melt) narrows the tension zone.
        let custom = saw_procedure(12.0, 12.0).with_mech_response_frac(0.9);
        let field = custom.residual_stress().unwrap();
        assert!(
            field.tension_half_width_mm < thick.residual_stress().unwrap().tension_half_width_mm,
            "a higher response fraction narrows the tension zone"
        );
    }

    #[test]
    fn errors_are_reported() {
        let mut sim = saw_procedure(0.0, 12.0);
        assert_eq!(
            sim.thermal_cycle(5.0),
            Err(WeldingError::NonPositiveHeatInput)
        );
        assert_eq!(sim.distortion(), Err(WeldingError::NonPositiveHeatInput));
        sim.weld_procedure.heat_input_kj_mm = 12.0;
        sim.weld_procedure.travel_speed_mm_s = 0.0;
        assert_eq!(sim.net_power_w(), Err(WeldingError::NonPositiveTravelSpeed));
        // travel-speed validation runs before distance validation
        assert_eq!(
            sim.thermal_cycle(-1.0),
            Err(WeldingError::NonPositiveTravelSpeed)
        );
        sim.weld_procedure.travel_speed_mm_s = 8.0;
        assert_eq!(sim.thermal_cycle(-1.0), Err(WeldingError::NegativeDistance));
        assert_eq!(
            sim.welding_sequence_optimization(&[]),
            Err(WeldingError::EmptySequence)
        );
    }
}
