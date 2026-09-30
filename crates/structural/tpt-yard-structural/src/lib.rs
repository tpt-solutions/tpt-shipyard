//! Structural analysis of incomplete structures at every build phase.
//!
//! In-service analysis tools assume the vessel is complete. During
//! construction it is not: blocks are missing, welds are partial, and the
//! loads are entirely different — crane picks, temporary supports, launching
//! ways. This crate analyses the structure *as it exists at a phase*:
//!
//! - [`ConstructionStructuralSolver::analyze_at_phase`] — assembles a truss
//!   model from the elements erected so far and solves the phase load cases
//!   (see [`fem`] for the solver).
//! - [`ConstructionStructuralSolver::lifting_analysis`] — crane pick
//!   statics: sling loads from lift-point geometry, tip check against the
//!   CoG, utilization against sling and crane capacity.
//! - [`ConstructionStructuralSolver::launch_analysis`] — load-case screening
//!   for launch methods (way pressure, tipping). Detailed launch physics
//!   lives in `tpt-yard-launch` (Phase 3).
//!
//! The structural substrate (`tpt-fem`) is not yet published; this crate
//! vendors a small truss solver ([`fem::TrussModel`]) behind the same
//! concepts, so the swap is mechanical when the substrate lands.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::{PhaseId, Vector3};
//! use tpt_yard_structural::{
//!     ConstructionLoad, ConstructionStructuralSolver, PartialElement,
//!     PartialStructure, Support,
//! };
//!
//! // A two-member gallows frame; the diagonal is erected at phase 2.
//! let structure = PartialStructure {
//!     nodes: vec![
//!         Vector3::new(0.0, 0.0, 4.0), // 0: post top
//!         Vector3::new(0.0, 0.0, 0.0), // 1: post base
//!         Vector3::new(3.0, 0.0, 4.0), // 2: boom tip
//!     ],
//!     elements: vec![
//!         PartialElement { nodes: [1, 0], area_m2: 0.01,
//!             youngs_modulus_gpa: 210.0, density_kg_m3: 7850.0,
//!             erected_at: PhaseId(1) },
//!         PartialElement { nodes: [0, 2], area_m2: 0.005,
//!             youngs_modulus_gpa: 210.0, density_kg_m3: 0.0,
//!             erected_at: PhaseId(1) },
//!         PartialElement { nodes: [1, 2], area_m2: 0.004,
//!             youngs_modulus_gpa: 210.0, density_kg_m3: 0.0,
//!             erected_at: PhaseId(2) },
//!     ],
//!     supports: vec![
//!         Support { node: 1, fix_x: true, fix_y: true, fix_z: true },
//!         Support { node: 0, fix_x: true, fix_y: true, fix_z: false },
//!         Support { node: 2, fix_x: false, fix_y: true, fix_z: false },
//!     ],
//! };
//! let solver = ConstructionStructuralSolver::new(structure, 355.0);
//!
//! // At phase 1 the boom hangs as a mechanism on the post — flagged, not
//! // silently solved.
//! let r1 = solver.analyze_at_phase(PhaseId(1), &[ConstructionLoad::Gravity]);
//! assert!(r1.is_err() || !r1.unwrap().passed);
//! ```

use std::fmt;

use tpt_yard_core::{MassProperties, PhaseId, Vector3, VesselProject};

pub mod fem;

pub use fem::{Element, FemError, NodalLoad, Node, Support, TrussModel, TrussSolution};

/// One structural member with the phase at which it enters the structure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartialElement {
    /// Node indices into [`PartialStructure::nodes`].
    pub nodes: [usize; 2],
    /// Cross-section area, m².
    pub area_m2: f64,
    /// Young's modulus, GPa.
    pub youngs_modulus_gpa: f64,
    /// Density, kg/m³ (self weight; 0 to ignore).
    pub density_kg_m3: f64,
    /// The phase at/after which this member is erected and load-bearing.
    pub erected_at: PhaseId,
}

/// The evolving structure: all members, each tagged with its erection phase.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PartialStructure {
    /// Node positions, m.
    pub nodes: Vec<Vector3>,
    /// Members with erection phases.
    pub elements: Vec<PartialElement>,
    /// Supports (assumed present from phase 1 — foundations first).
    pub supports: Vec<Support>,
}

impl PartialStructure {
    /// Sub-model of everything erected at or before `phase`.
    ///
    /// # Errors
    ///
    /// [`FemError::NoActiveElements`] when nothing is erected yet.
    pub fn model_at_phase(&self, phase: PhaseId) -> Result<TrussModel, FemError> {
        let active: Vec<Element> = self
            .elements
            .iter()
            .filter(|e| e.erected_at <= phase)
            .map(|e| Element {
                nodes: e.nodes,
                area_m2: e.area_m2,
                youngs_modulus_gpa: e.youngs_modulus_gpa,
                density_kg_m3: e.density_kg_m3,
            })
            .collect();
        if active.is_empty() {
            return Err(FemError::NoActiveElements);
        }
        Ok(TrussModel {
            nodes: self.nodes.iter().map(|&p| Node { position: p }).collect(),
            elements: active,
            supports: self.supports.clone(),
            loads: Vec::new(),
        })
    }
}

/// Loads applied during construction (spec §5, Domain 2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstructionLoad {
    /// Self weight of everything erected at the phase.
    Gravity,
    /// Wind pressure on the exposed structure.
    Wind {
        /// Mean wind speed, m/s.
        speed_ms: f64,
        /// Direction, degrees from +X (plan view).
        direction_deg: f64,
        /// Projected exposed area, m² (caller estimates from the geometry).
        exposed_area_m2: f64,
        /// Node that receives the resulting force.
        application_node: usize,
    },
    /// A crane load: the hook load applied at a node with a dynamic factor.
    CraneLoad {
        /// Hook load, kN.
        capacity_kn: f64,
        /// Dynamic amplification (1.0 static, 1.1–1.25 typical picks).
        dynamic_factor: f64,
        /// Loaded node.
        node: usize,
    },
    /// Hydrostatic pressure on a submerged area.
    Hydrostatic {
        /// Pressure, kPa.
        pressure_kpa: f64,
        /// Loaded area, m².
        area_m2: f64,
        /// Loaded node.
        node: usize,
    },
    /// Wave slamming impact force.
    WaveSlamming {
        /// Impact force, kN.
        force_kn: f64,
        /// Loaded node.
        node: usize,
    },
    /// Docking impulse.
    Docking {
        /// Force, kN.
        force_kn: f64,
        /// Loaded node.
        node: usize,
    },
    /// Robotic arm reaction.
    RoboticArm {
        /// Force, N.
        force_n: f64,
        /// Loaded node.
        node: usize,
    },
    /// Welding thermal load — resolved by the welding crate's distortion
    /// model, not the truss solver; kept in the enum so phase load sets are
    /// complete and auditable.
    WeldingThermal {
        /// Heat input, kJ/mm.
        heat_input_kj_mm: f64,
    },
}

/// Result of a phase structural analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuralResult {
    /// The phase analysed.
    pub phase: PhaseId,
    /// Number of members in the model.
    pub active_members: usize,
    /// Largest nodal displacement, mm.
    pub max_displacement_mm: f64,
    /// Largest absolute axial stress, MPa.
    pub max_axial_stress_mpa: f64,
    /// `max_axial_stress / allowable`.
    pub max_utilization: f64,
    /// All utilizations ≤ 1.
    pub passed: bool,
    /// Findings and any model notes.
    pub notes: Vec<String>,
}

/// A rigid body being lifted.
#[derive(Debug, Clone, PartialEq)]
pub struct LiftableBody {
    /// Mass properties (weight and CoG).
    pub mass_properties: MassProperties,
}

/// Result of a lifting analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct LiftingResult {
    /// Tension per sling leg, kN (indexed like the lift points).
    pub sling_loads_kn: Vec<f64>,
    /// `max_leg_tension / sling_capacity`.
    pub max_leg_utilization: f64,
    /// Total hook load / crane capacity.
    pub crane_utilization: f64,
    /// Angle of each sling leg from the horizontal, degrees.
    pub sling_angles_deg: Vec<f64>,
    /// True if every check passes.
    pub safe: bool,
    /// Findings.
    pub notes: Vec<String>,
}

/// Simplified launch method for load-case screening.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LaunchCase {
    /// End launch down sloping ways.
    Slipway {
        /// Way slope, degrees from horizontal.
        slope_deg: f64,
        /// Total way length in contact, m.
        way_length_m: f64,
        /// Total way width (all ways), m.
        way_width_m: f64,
    },
    /// Float-out from a building dock.
    DrydockFlooding,
    /// 90° side launch.
    SideLaunch,
    /// Shiplift / platform transfer.
    Shiplift,
}

/// Result of a launch load-case screening.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchResult {
    /// The launch case analysed.
    pub case: LaunchCase,
    /// Maximum way/ground pressure, MPa.
    pub max_pressure_mpa: f64,
    /// Longitudinal CoG margin along the ways, 0–1 (1 = centred).
    pub tipping_margin: f64,
    /// True if pressures and stability are within screening limits.
    pub within_limits: bool,
    /// Findings.
    pub notes: Vec<String>,
}

/// Errors of the construction solver.
#[derive(Debug, Clone, PartialEq)]
pub enum StructuralError {
    /// The FEM sub-solver failed.
    Fem(FemError),
    /// Lifting needs at least two lift points.
    TooFewLiftPoints,
    /// A lift point or node index is out of range.
    OutOfRange,
    /// The load case has no screening model here — run the detailed
    /// `tpt-yard-launch` analysis for it.
    NotScreenable,
}

impl fmt::Display for StructuralError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StructuralError::Fem(e) => write!(f, "fem: {e}"),
            StructuralError::TooFewLiftPoints => {
                f.write_str("at least two lift points are required")
            }
            StructuralError::OutOfRange => f.write_str("index out of range"),
            StructuralError::NotScreenable => {
                f.write_str("not screenable here: use the detailed tpt-yard-launch analysis")
            }
        }
    }
}

impl std::error::Error for StructuralError {}

impl From<FemError> for StructuralError {
    fn from(e: FemError) -> Self {
        StructuralError::Fem(e)
    }
}

/// Analyzes incomplete structures at each build phase.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructionStructuralSolver {
    /// The evolving structure.
    pub structure: PartialStructure,
    /// Allowable axial stress, MPa (e.g. yield with safety factor applied).
    pub allowable_stress_mpa: f64,
    /// Allowable way/ground pressure for launch screening, MPa.
    pub allowable_ground_pressure_mpa: f64,
}

impl ConstructionStructuralSolver {
    /// Creates a solver with the classic 500 kPa launch-way screening limit.
    pub fn new(structure: PartialStructure, allowable_stress_mpa: f64) -> Self {
        Self {
            structure,
            allowable_stress_mpa,
            allowable_ground_pressure_mpa: 0.5,
        }
    }

    /// Analyzes the structure as it stands at the end of `phase`, under the
    /// given construction loads.
    ///
    /// # Errors
    ///
    /// [`StructuralError`] if nothing is erected yet, the staging is a
    /// mechanism, or indices are out of range.
    pub fn analyze_at_phase(
        &self,
        phase: PhaseId,
        loads: &[ConstructionLoad],
    ) -> Result<StructuralResult, StructuralError> {
        let mut model = self.structure.model_at_phase(phase)?;
        let mut notes = Vec::new();

        for load in loads {
            match *load {
                ConstructionLoad::Gravity => {} // element self weight is assembled by the model
                ConstructionLoad::Wind {
                    speed_ms,
                    direction_deg,
                    exposed_area_m2,
                    application_node,
                } => {
                    // Quasi-static drag: p = 0.613 v² (Pa) on the projected
                    // area, applied along the wind direction.
                    let p = 0.613 * speed_ms * speed_ms; // Pa
                    let f = p * exposed_area_m2;
                    let rad = direction_deg.to_radians();
                    if application_node >= self.structure.nodes.len() {
                        return Err(StructuralError::OutOfRange);
                    }
                    model.loads.push(NodalLoad {
                        node: application_node,
                        force: Vector3::new(f * rad.cos(), f * rad.sin(), 0.0),
                    });
                    notes.push(format!(
                        "wind {speed_ms} m/s → {f:.0} N applied at node {application_node}"
                    ));
                }
                ConstructionLoad::CraneLoad {
                    capacity_kn,
                    dynamic_factor,
                    node,
                } => {
                    if node >= self.structure.nodes.len() {
                        return Err(StructuralError::OutOfRange);
                    }
                    model.loads.push(NodalLoad {
                        node,
                        force: Vector3::new(0.0, 0.0, -capacity_kn * 1000.0 * dynamic_factor),
                    });
                    notes.push(format!(
                        "crane pick {:.0} kN ×{dynamic_factor} at node {node}",
                        capacity_kn
                    ));
                }
                ConstructionLoad::Hydrostatic {
                    pressure_kpa,
                    area_m2,
                    node,
                } => {
                    if node >= self.structure.nodes.len() {
                        return Err(StructuralError::OutOfRange);
                    }
                    let f = pressure_kpa * 1000.0 * area_m2;
                    model.loads.push(NodalLoad {
                        node,
                        force: Vector3::new(0.0, 0.0, f),
                    });
                    notes.push(format!("hydrostatic {f:.0} N at node {node}"));
                }
                ConstructionLoad::WaveSlamming { force_kn, node }
                | ConstructionLoad::Docking { force_kn, node } => {
                    if node >= self.structure.nodes.len() {
                        return Err(StructuralError::OutOfRange);
                    }
                    model.loads.push(NodalLoad {
                        node,
                        force: Vector3::new(-force_kn * 1000.0, 0.0, 0.0),
                    });
                    notes.push(format!("impact {force_kn:.0} kN at node {node}"));
                }
                ConstructionLoad::RoboticArm { force_n, node } => {
                    if node >= self.structure.nodes.len() {
                        return Err(StructuralError::OutOfRange);
                    }
                    model.loads.push(NodalLoad {
                        node,
                        force: Vector3::new(0.0, 0.0, -force_n),
                    });
                }
                ConstructionLoad::WeldingThermal { heat_input_kj_mm } => {
                    notes.push(format!(
                        "welding thermal {heat_input_kj_mm} kJ/mm: resolved by tpt-yard-welding, excluded from the truss model"
                    ));
                }
            }
        }

        let solution = model.solve()?;
        let max_stress = solution
            .axial_forces
            .iter()
            .zip(&model.elements)
            .map(|(n, e)| (n.abs() / e.area_m2) / 1e6) // MPa
            .fold(0.0f64, f64::max);
        let utilization = max_stress / self.allowable_stress_mpa;
        if utilization >= 1.0 {
            notes.push(format!(
                "utilization {utilization:.2} exceeds 1.0 at this phase"
            ));
        }
        Ok(StructuralResult {
            phase,
            active_members: model.elements.len(),
            max_displacement_mm: solution.max_displacement_m * 1000.0,
            max_axial_stress_mpa: max_stress,
            max_utilization: utilization,
            passed: utilization < 1.0,
            notes,
        })
    }

    /// Crane pick statics for a block.
    ///
    /// The hook sits directly above the **CoG projection** (a statically
    /// correct pick); legs run from each lift point up to the hook at
    /// `hook_height_above_lifts_m`. Load shares follow the exact lever rule
    /// for 2 points (scalar projection of the CoG onto the lift line) and an
    /// inverse-distance weighted statics approximation above that; leg
    /// tension = share / sin(angle from horizontal).
    ///
    /// Checks (any failure makes `safe = false`): CoG projection inside the
    /// lift-point **convex hull** (tip), leg utilization, crane utilization
    /// (hook load + 5 % rigging against `crane_capacity_kn`), and the 30°
    /// minimum sling-angle practice guideline.
    ///
    /// # Errors
    ///
    /// [`StructuralError::TooFewLiftPoints`] under two points;
    /// [`StructuralError::OutOfRange`] on bad `body`/point input.
    pub fn lifting_analysis(
        &self,
        body: &LiftableBody,
        sling_capacity_kn: f64,
        crane_capacity_kn: f64,
        lift_points: &[Vector3],
        hook_height_above_lifts_m: f64,
    ) -> Result<LiftingResult, StructuralError> {
        if lift_points.len() < 2 {
            return Err(StructuralError::TooFewLiftPoints);
        }
        if body.mass_properties.mass_kg <= 0.0
            || !body.mass_properties.mass_kg.is_finite()
            || sling_capacity_kn <= 0.0
            || !sling_capacity_kn.is_finite()
            || crane_capacity_kn <= 0.0
            || !crane_capacity_kn.is_finite()
            || hook_height_above_lifts_m <= 0.0
            || !hook_height_above_lifts_m.is_finite()
        {
            return Err(StructuralError::OutOfRange);
        }
        let weight_n = body.mass_properties.mass_kg * 9.81;
        let cog = body.mass_properties.cog;
        let n = lift_points.len();

        // Statically correct pick: hook over the CoG projection.
        let hook_xy = Vector3::new(cog.x, cog.y, 0.0);

        // Tip check: the CoG projection must lie inside the convex hull of
        // the lift points (the AABB is a superset and would let corners
        // pass that would actually tip).
        let mut notes = Vec::new();
        let mut safe = true;
        if !cog_in_convex_hull(cog, lift_points) {
            safe = false;
            notes.push(
                "CoG projects outside the lift points: the block would tip on the pick".to_string(),
            );
        }

        // Load shares: exact lever rule for 2 points, inverse-distance
        // weighting beyond (documented approximation; statically
        // indeterminate without sling stiffnesses).
        let mut shares = vec![1.0 / n as f64; n];
        if n == 2 {
            let u = lift_points[1] - lift_points[0];
            let span_sq = u.dot(u);
            if span_sq > 1e-9 {
                let t = ((cog - lift_points[0]).dot(u) / span_sq).clamp(0.0, 1.0);
                shares[0] = 1.0 - t;
                shares[1] = t;
            }
        } else {
            let weights: Vec<f64> = lift_points
                .iter()
                .map(|p| {
                    let d = ((p.x - cog.x) * (p.x - cog.x) + (p.y - cog.y) * (p.y - cog.y))
                        .sqrt()
                        .max(1e-3);
                    1.0 / d
                })
                .collect();
            let total: f64 = weights.iter().sum();
            shares = weights.iter().map(|w| w / total).collect();
        }

        let mut sling_loads_kn = Vec::with_capacity(n);
        let mut angles = Vec::with_capacity(n);
        let mut max_leg = 0.0f64;
        for (i, p) in lift_points.iter().enumerate() {
            let dx = (p.x - hook_xy.x).hypot(p.y - hook_xy.y);
            let theta = (hook_height_above_lifts_m.max(1e-6)).atan2(dx.max(1e-6));
            angles.push(theta.to_degrees());
            let leg_n = weight_n * shares[i] / theta.sin();
            let leg_kn = leg_n / 1000.0;
            max_leg = max_leg.max(leg_kn / sling_capacity_kn);
            sling_loads_kn.push(leg_kn);
        }
        if max_leg > 1.0 {
            safe = false;
            notes.push(format!("sling utilization {max_leg:.2} exceeds 1.0"));
        }
        // Crane utilization: hook load + a 5% rigging allowance.
        let hook_kn = weight_n / 1000.0 * 1.05;
        let crane_util = hook_kn / crane_capacity_kn;
        if crane_util > 1.0 {
            safe = false;
            notes.push(format!(
                "crane utilization {crane_util:.2} exceeds 1.0 (hook {hook_kn:.0} kN vs crane {crane_capacity_kn:.0} kN)"
            ));
        }
        for (i, a) in angles.iter().enumerate() {
            if *a < 30.0 {
                safe = false;
                notes.push(format!(
                    "sling {i} at {a:.0}° from horizontal: below the 30° practice guideline"
                ));
            }
        }

        Ok(LiftingResult {
            sling_loads_kn,
            max_leg_utilization: max_leg,
            crane_utilization: crane_util,
            sling_angles_deg: angles,
            safe,
            notes,
        })
    }

    /// Launch load-case screening (detailed physics in `tpt-yard-launch`).
    ///
    /// Slipway: normal way pressure `P = W·cos(slope)/(L_way · b_way)`
    /// against the screening limit, with the CoG-centred resting tipping
    /// assumption stated in the notes. Other launch cases have no
    /// screening model here — they return
    /// [`StructuralError::NotScreenable`] rather than a made-up pass.
    ///
    /// # Errors
    ///
    /// [`StructuralError::OutOfRange`] for degenerate way geometry,
    /// [`StructuralError::NotScreenable`] for cases this screening pass
    /// does not model.
    pub fn launch_analysis(
        &self,
        vessel: &VesselProject,
        weight: &MassProperties,
        case: LaunchCase,
    ) -> Result<LaunchResult, StructuralError> {
        let mut notes = Vec::new();
        match case {
            LaunchCase::Slipway {
                slope_deg,
                way_length_m,
                way_width_m,
            } => {
                if way_length_m <= 0.0
                    || !way_length_m.is_finite()
                    || way_width_m <= 0.0
                    || !way_width_m.is_finite()
                {
                    return Err(StructuralError::OutOfRange);
                }
                let weight_kn = weight.mass_kg * 9.81 / 1000.0;
                // Only the normal component loads the ways.
                let slope_rad = slope_deg.to_radians();
                let normal_kn = weight_kn * slope_rad.cos();
                let pressure_kpa = normal_kn / (way_length_m * way_width_m);
                let pressure_mpa = pressure_kpa / 1000.0;
                notes.push(format!(
                    "'{}' on ways at {slope_deg:.1}°: {normal_kn:.0} kN normal over {:.0} m² → {pressure_kpa:.0} kPa",
                    vessel.name,
                    way_length_m * way_width_m
                ));
                notes.push(
                    "screening assumes the CoG centred at rest; end-of-ways tipping needs the launch simulation".into(),
                );
                let within = pressure_mpa <= self.allowable_ground_pressure_mpa;
                Ok(LaunchResult {
                    case,
                    max_pressure_mpa: pressure_mpa,
                    tipping_margin: 1.0, // CoG centred assumption at rest
                    within_limits: within,
                    notes,
                })
            }
            LaunchCase::DrydockFlooding | LaunchCase::SideLaunch | LaunchCase::Shiplift => {
                Err(StructuralError::NotScreenable)
            }
        }
    }
}

/// True when the CoG's XY projection lies inside the convex hull of the
/// lift points (the pick-tipping test). Andrew's monotone chain hull, then
/// a ray-crossing point test; a degenerate (collinear) point set falls back
/// to the bounding box, which is exact for a line segment.
fn cog_in_convex_hull(cog: Vector3, points: &[Vector3]) -> bool {
    if points.is_empty() {
        return false;
    }
    // Monotone chain on (x, y); duplicates collapse naturally.
    let mut pts: Vec<(f64, f64)> = points.iter().map(|p| (p.x, p.y)).collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    pts.dedup();
    if pts.len() < 3 {
        // Collinear or duplicated lifts: the tip region is the segment's
        // bounding box.
        let (min_x, max_x) = pts
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
        let (min_y, max_y) = pts
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
        return cog.x >= min_x && cog.x <= max_x && cog.y >= min_y && cog.y <= max_y;
    }
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut lower: Vec<(f64, f64)> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<(f64, f64)> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    let hull: Vec<(f64, f64)> = lower.into_iter().chain(upper).collect();
    if hull.len() < 3 {
        return false;
    }
    // Ray-crossing point-in-polygon (the guard condition guarantees
    // yi != yj, so the division is safe).
    let (px, py) = (cog.x, cog.y);
    let mut inside = false;
    let mut j = hull.len() - 1;
    for i in 0..hull.len() {
        let (xi, yi) = hull[i];
        let (xj, yj) = hull[j];
        if ((yi > py) != (yj > py)) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_core::ConstructionMethod;

    fn gallows() -> PartialStructure {
        PartialStructure {
            nodes: vec![
                Vector3::new(0.0, 0.0, 4.0), // 0 post top
                Vector3::new(0.0, 0.0, 0.0), // 1 post base
                Vector3::new(3.0, 0.0, 4.0), // 2 boom tip
            ],
            elements: vec![
                PartialElement {
                    nodes: [1, 0],
                    area_m2: 0.01,
                    youngs_modulus_gpa: 210.0,
                    density_kg_m3: 7850.0,
                    erected_at: PhaseId(1),
                },
                PartialElement {
                    nodes: [0, 2],
                    area_m2: 0.005,
                    youngs_modulus_gpa: 210.0,
                    density_kg_m3: 0.0,
                    erected_at: PhaseId(1),
                },
                PartialElement {
                    nodes: [1, 2],
                    area_m2: 0.004,
                    youngs_modulus_gpa: 210.0,
                    density_kg_m3: 0.0,
                    erected_at: PhaseId(2),
                },
            ],
            // Erection-fixture restraints: the base is pinned; the post top
            // and boom tip are held laterally by temporary guy/roller
            // restraints (a planar triangle on one pin is a rigid-body
            // mechanism in 3D truss-land).
            supports: vec![
                Support::pinned(1),
                Support {
                    node: 0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: false,
                },
                Support {
                    node: 2,
                    fix_x: false,
                    fix_y: true,
                    fix_z: false,
                },
            ],
        }
    }

    /// Verification: partial-structure analysis reflects erection staging.
    #[test]
    fn staged_structure_changes_load_path() {
        let solver = ConstructionStructuralSolver::new(gallows(), 355.0);

        // Phase 1: post + boom = a mechanism (boom tip can swing) — the
        // solver must refuse rather than return nonsense.
        let r1 = solver.analyze_at_phase(PhaseId(1), &[ConstructionLoad::Gravity]);
        assert!(matches!(
            r1,
            Err(StructuralError::Fem(FemError::SingularSystem))
        ));

        // Phase 2: diagonal closes the triangle — solvable, members = 3.
        let r2 = solver
            .analyze_at_phase(PhaseId(2), &[ConstructionLoad::Gravity])
            .expect("closed triangle must solve");
        assert_eq!(r2.active_members, 3);
        assert!(r2.max_axial_stress_mpa > 0.0);
        assert!(r2.passed, "self weight of 4 m post is far below 355 MPa");
    }

    #[test]
    fn crane_load_overstresses_when_allowable_is_tight() {
        let mut structure = gallows();
        // Steel already erected; drop the self weight to isolate the pick.
        for e in &mut structure.elements {
            e.density_kg_m3 = 0.0;
        }
        let solver = ConstructionStructuralSolver::new(structure, 100.0);
        let r = solver
            .analyze_at_phase(
                PhaseId(2),
                &[ConstructionLoad::CraneLoad {
                    capacity_kn: 500.0,
                    dynamic_factor: 1.25,
                    node: 2,
                }],
            )
            .unwrap();
        // Diagonal (A = 0.004 m²) carries a large share of 625 kN.
        assert!(r.max_axial_stress_mpa > 50.0);
        assert!(
            !r.passed,
            "625 kN on a 40 cm² diagonal is over 100 MPa allowable"
        );
        assert!(r.max_utilization > 1.0);
    }

    #[test]
    fn wind_load_is_directional() {
        let solver = ConstructionStructuralSolver::new(gallows(), 355.0);
        let r = solver
            .analyze_at_phase(
                PhaseId(2),
                &[ConstructionLoad::Wind {
                    speed_ms: 20.0,
                    direction_deg: 0.0,
                    exposed_area_m2: 10.0,
                    application_node: 0,
                }],
            )
            .unwrap();
        assert!(r.max_axial_stress_mpa > 0.0);
        assert_eq!(r.notes.len(), 1); // wind note
    }

    /// Verification: lifting a symmetric block loads both slings equally;
    /// an offset CoG shifts load to the nearer point.
    #[test]
    fn lifting_sling_loads_follow_lever_rule() {
        let solver = ConstructionStructuralSolver::new(PartialStructure::default(), 355.0);

        // 100 t block, 10 m wide, CoG centred, lifts at the ends.
        let body = LiftableBody {
            mass_properties: MassProperties {
                mass_kg: 100_000.0,
                cog: Vector3::new(5.0, 0.0, 0.0),
            },
        };
        let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0)];
        let r = solver
            .lifting_analysis(&body, 1000.0, 2000.0, &lifts, 4.0)
            .unwrap();
        assert!(r.safe);
        let w_kn = 100_000.0 * 9.81 / 1000.0;
        // Legs at atan(4/5) from horizontal.
        let expected = w_kn / 2.0 / 4.0f64.atan2(5.0).sin();
        assert!(
            (r.sling_loads_kn[0] - expected).abs() < 1.0,
            "{} vs {expected}",
            r.sling_loads_kn[0]
        );
        assert!((r.sling_loads_kn[1] - expected).abs() < 1.0);
        // Crane utilization against the crane capacity parameter.
        let hook_kn = w_kn * 1.05;
        assert!((r.crane_utilization - hook_kn / 2000.0).abs() < 1e-9);

        // Offset CoG: nearer point takes more (lever rule, 80/20).
        let offset = LiftableBody {
            mass_properties: MassProperties {
                mass_kg: 100_000.0,
                cog: Vector3::new(8.0, 0.0, 0.0),
            },
        };
        let r2 = solver
            .lifting_analysis(&offset, 1000.0, 2000.0, &lifts, 4.0)
            .unwrap();
        assert!(r2.sling_loads_kn[1] > r2.sling_loads_kn[0]);
        // Vertical components of the leg tensions are the 20/80 shares
        // (tension differs per leg because the angles differ when the hook
        // rides over the CoG).
        let th0 = 4.0f64.atan2(8.0).sin();
        let th1 = 4.0f64.atan2(2.0).sin();
        assert!(
            (r2.sling_loads_kn[0] * th0 / (r2.sling_loads_kn[1] * th1) - 0.25).abs() < 1e-9,
            "vertical shares must be 20/80"
        );
    }

    /// Regression (review 7A/A9): crane overload, shallow slings and an
    /// out-of-hull CoG must all fail the pick, and the tip check must use
    /// the convex hull (an AABB would pass a CoG outside the hull corner).
    #[test]
    fn lifting_failures_fail_safe() {
        let solver = ConstructionStructuralSolver::new(PartialStructure::default(), 355.0);
        let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0)];
        let body = LiftableBody {
            mass_properties: MassProperties {
                mass_kg: 100_000.0,
                cog: Vector3::new(5.0, 0.0, 0.0),
            },
        };

        // Crane overload: 500 t hook on a 300 t crane.
        let heavy = LiftableBody {
            mass_properties: MassProperties {
                mass_kg: 500_000.0,
                cog: Vector3::new(5.0, 0.0, 0.0),
            },
        };
        let r = solver
            .lifting_analysis(&heavy, 5000.0, 3000.0, &lifts, 4.0)
            .unwrap();
        assert!(!r.safe, "crane overload must fail the pick");
        assert!(r.crane_utilization > 1.0);
        assert!(r.notes.iter().any(|n| n.contains("crane utilization")));

        // Shallow slings: 1 m hook height on a 10 m span → ~11°.
        let r = solver
            .lifting_analysis(&body, 1000.0, 2000.0, &lifts, 1.0)
            .unwrap();
        assert!(!r.safe, "below-guideline sling angles must fail the pick");
        assert!(r.notes.iter().any(|n| n.contains("30°")));

        // Convex hull, not AABB: an L of lifts (three points) leaves a notch
        // that the AABB covers but the hull does not; a CoG in the notch tips.
        let l_lifts = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
            Vector3::new(0.0, 10.0, 0.0),
        ];
        // (9, 9) is inside the AABB [0,10]² but outside the triangle hull.
        let notch = LiftableBody {
            mass_properties: MassProperties {
                mass_kg: 10_000.0,
                cog: Vector3::new(9.0, 9.0, 0.0),
            },
        };
        let r = solver
            .lifting_analysis(&notch, 1000.0, 2000.0, &l_lifts, 8.0)
            .unwrap();
        assert!(!r.safe, "CoG outside the convex hull must tip the pick");
        assert!(r.notes.iter().any(|n| n.contains("tip")));

        // And a CoG inside the triangle passes the tip check.
        let inside = LiftableBody {
            mass_properties: MassProperties {
                mass_kg: 10_000.0,
                cog: Vector3::new(2.0, 2.0, 0.0),
            },
        };
        let r = solver
            .lifting_analysis(&inside, 1000.0, 2000.0, &l_lifts, 8.0)
            .unwrap();
        assert!(!r.notes.iter().any(|n| n.contains("tip")));
    }

    #[test]
    fn lifting_detects_tipping_and_slack_angles() {
        let solver = ConstructionStructuralSolver::new(PartialStructure::default(), 355.0);
        // CoG beyond the lift points → tip.
        let body = LiftableBody {
            mass_properties: MassProperties {
                mass_kg: 50_000.0,
                cog: Vector3::new(20.0, 0.0, 0.0),
            },
        };
        let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0)];
        let r = solver
            .lifting_analysis(&body, 1000.0, 2000.0, &lifts, 8.0)
            .unwrap();
        assert!(!r.safe);
        assert!(r.notes.iter().any(|n| n.contains("tip")));

        // Too few lift points.
        assert_eq!(
            solver.lifting_analysis(
                &LiftableBody {
                    mass_properties: MassProperties {
                        mass_kg: 1.0,
                        cog: Vector3::ZERO
                    }
                },
                100.0,
                200.0,
                &[Vector3::ZERO],
                4.0
            ),
            Err(StructuralError::TooFewLiftPoints)
        );
    }

    #[test]
    fn slipway_pressure_screening() {
        let solver = ConstructionStructuralSolver::new(PartialStructure::default(), 355.0);
        let vessel = VesselProject::new(
            tpt_yard_core::ProjectId(1),
            "Barge",
            tpt_yard_core::VesselType::Sea(tpt_yard_core::SeaVesselType::FishingVessel),
            ConstructionMethod::SeaSlipwayLaunch,
            vec![tpt_yard_core::BuildPhase::new(
                tpt_yard_core::PhaseId(1),
                "Complete",
                1.0,
            )],
        )
        .unwrap();
        let weight = MassProperties {
            mass_kg: 5_000_000.0, // 5000 t
            cog: Vector3::new(60.0, 0.0, 6.0),
        };
        // Ways: 100 m × 4 m = 400 m² → 49050 kN / 400 = ~122 kPa. Fine.
        let ok = solver
            .launch_analysis(
                &vessel,
                &weight,
                LaunchCase::Slipway {
                    slope_deg: 3.0,
                    way_length_m: 100.0,
                    way_width_m: 4.0,
                },
            )
            .unwrap();
        assert!(ok.within_limits);
        assert!((ok.max_pressure_mpa - 0.1226).abs() < 0.001);

        // Tiny ways → over-pressure, flagged.
        let bad = solver
            .launch_analysis(
                &vessel,
                &weight,
                LaunchCase::Slipway {
                    slope_deg: 3.0,
                    way_length_m: 10.0,
                    way_width_m: 1.0,
                },
            )
            .unwrap();
        assert!(!bad.within_limits);
        assert!(bad.max_pressure_mpa > solver.allowable_ground_pressure_mpa);
    }

    #[test]
    fn no_steel_yet_is_reported() {
        let solver = ConstructionStructuralSolver::new(gallows(), 355.0);
        assert!(matches!(
            solver.analyze_at_phase(PhaseId(0), &[ConstructionLoad::Gravity]),
            Err(StructuralError::Fem(FemError::NoActiveElements))
        ));
    }
}
