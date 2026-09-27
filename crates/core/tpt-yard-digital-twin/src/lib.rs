//! Construction state tracking and simulation: the shipyard digital twin.
//!
//! [`DigitalTwin`] owns a [`VesselProject`](tpt_yard_core::VesselProject) and
//! mirrors its real-world progress: which activities are complete, what mass
//! is physically installed, where the centre of gravity currently sits, and
//! whether the *partially built* structure can stand on its supports at every
//! stage — the question no in-service analysis tool asks.
//!
//! The twin deliberately owns the project (the project carries no
//! back-reference to the twin); see RFC 0001.
//!
//! # Example
//!
//! ```
//! use tpt_yard_assembly::ActivityId;
//! use tpt_yard_core::*;
//! use tpt_yard_digital_twin::{DigitalTwin, SupportCondition};
//! use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};
//! use tpt_yard_core::{ItemId, PhaseId, ProjectId, Vector3};
//!
//! // One-phase project with a single erection activity.
//! let mut phase = BuildPhase::new(PhaseId(1), "Erection", 5.0);
//! phase.activities.push(AssemblyActivity::new(
//!     ActivityId(1), "Erect block 211", ActivityType::JoinBlock, 8.0,
//! ));
//! let project = VesselProject::new(
//!     ProjectId(1), "Barge", VesselType::Sea(SeaVesselType::FishingVessel),
//!     ConstructionMethod::SeaDrydock, vec![phase],
//! ).unwrap();
//!
//! let mut weight = WeightModel::new(100.0, Vector3::new(0.0, 0.0, 3.0));
//! weight.add_item(WeightItem {
//!     id: ItemId(1), name: "Block 211".into(), group: "hull".into(),
//!     weight_kg: 100.0, cog: Vector3::new(2.0, 0.0, 3.0),
//!     status: ItemStatus::Design, margin_pct: 0.0,
//!     installed_by: Some(ActivityId(1)),
//! });
//!
//! let mut twin = DigitalTwin::with_weight_model(
//!     project, weight,
//!     SupportCondition::KeelBlocks {
//!         positions: vec![Vector3::new(-10.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0)],
//!     },
//! );
//!
//! twin.advance_phase(&ActivityId(1)).unwrap();
//! assert_eq!(twin.weight_model().installed_weight(), 100.0);
//! assert!(twin.structural_check().passed);
//! ```

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use tpt_yard_assembly::{ActivityGraph, ActivityId, ActivityStatus, GraphError};
use tpt_yard_core::phase::AssemblyActivity;
use tpt_yard_core::{Geometry3D, MassProperties, PhaseId, Vector3, VesselProject};
use tpt_yard_weight::{ItemStatus, WeightError, WeightModel};

/// How the incomplete structure is supported during construction.
///
/// The twin uses this to verify at every phase that the *partial* structure
/// stands safely — e.g. a half-built ship on keel blocks must not tip.
#[derive(Debug, Clone, PartialEq)]
pub enum SupportCondition {
    /// Building dock / erection on keel and side blocks. `positions` are the
    /// block support points in the plan view (x along the ship, y across),
    /// m. Longitudinal stability is checked against the extreme supports;
    /// transverse, against the mean half-track.
    KeelBlocks {
        /// Support point positions, m.
        positions: Vec<Vector3>,
    },
    /// On slipway ways before launch: `fore_poppet_x` is the way's forward
    /// end, `aft_way_x` the aft end, m.
    Slipway {
        /// Longitudinal position of the fore poppet, m.
        fore_poppet_x: f64,
        /// Longitudinal position of the aft way end, m.
        aft_way_x: f64,
    },
    /// Afloat: buoyancy supports the structure; no tipping check.
    Floating,
    /// Microgravity assembly: no gravity support loads.
    Orbital,
}

/// Result of a structural check at a build phase.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuralCheckResult {
    /// The phase that was checked.
    pub phase_id: PhaseId,
    /// True if the partial structure passes the support/stability checks.
    pub passed: bool,
    /// Longitudinal margin as a fraction of the support span: how far the CoG
    /// is from the nearest support, divided by the span. Negative margins
    /// mean the CoG is outboard of the supports.
    pub longitudinal_margin: f64,
    /// Computed support reactions (name, kN) for discrete supports.
    pub reactions_kn: Vec<(String, f64)>,
    /// Installed weight considered, kg.
    pub weight_kg: f64,
    /// Installed CoG, m.
    pub cog: Vector3,
    /// Human-readable findings.
    pub notes: Vec<String>,
}

/// Progress snapshot of the assembly.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AssemblyState {
    /// Activities finished and accepted.
    pub completed_activities: HashSet<ActivityId>,
    /// Activities currently executing.
    pub in_progress: HashSet<ActivityId>,
    /// Activities deliberately held.
    pub blocked: HashSet<ActivityId>,
    /// Geometry of what currently exists (for rendering).
    pub current_geometry: Geometry3D,
    /// Mass properties of the currently installed material.
    pub current_mass_properties: MassProperties,
}

/// A quality record (inspection outcome) attached to an activity.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityRecord {
    /// Record id.
    pub id: u64,
    /// The activity that was inspected.
    pub activity_id: ActivityId,
    /// True if accepted.
    pub accepted: bool,
    /// Free-text description / findings.
    pub description: String,
}

/// A sensor reading (metrology, strain gauges, temperature, ...).
#[derive(Debug, Clone, PartialEq)]
pub struct SensorReading {
    /// Time since twin creation, s.
    pub timestamp_s: f64,
    /// Sensor name.
    pub sensor: String,
    /// Reading value.
    pub value: f64,
}

/// The structural model summary maintained by the twin.
///
/// Detailed finite-element analysis of the partial structure lives in
/// `tpt-yard-structural`; the twin keeps the lightweight bookkeeping that
/// stays valid in every domain.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StructuralModel {
    /// Number of structural entities (members/blocks) tracked.
    pub member_count: usize,
    /// Fraction `[0, 1]` of entities currently connected into the structure.
    pub connected_fraction: f64,
}

/// Weight/CoG deviation report comparing as-built progress against design.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightDeviation {
    /// Physically installed weight, kg.
    pub as_built_installed_kg: f64,
    /// Current best estimate (installed + predicted remainder), kg.
    pub best_estimate_kg: f64,
    /// Contractual design weight, kg.
    pub design_kg: f64,
    /// Best estimate minus design, kg (positive = overweight).
    pub deviation_kg: f64,
    /// Deviation as percent of design.
    pub deviation_pct: f64,
}

/// One sample of the CoG history: state after a given phase.
#[derive(Debug, Clone, PartialEq)]
pub struct CoGSample {
    /// Phase whose completion defines this sample.
    pub phase_id: PhaseId,
    /// Cumulative installed weight at the end of the phase, kg.
    pub installed_kg: f64,
    /// Cumulative CoG, m.
    pub cog: Vector3,
}

/// CoG evolution through construction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CoGReport {
    /// One sample per (partially) completed phase, in phase order.
    pub samples: Vec<CoGSample>,
}

/// Errors produced by twin operations.
#[derive(Debug, Clone, PartialEq)]
pub enum TwinError {
    /// The activity is not part of the project.
    UnknownActivity(ActivityId),
    /// These dependencies must complete first.
    DependenciesNotComplete(Vec<ActivityId>),
    /// The activity was already completed.
    AlreadyCompleted(ActivityId),
    /// The phase id is not part of the project.
    UnknownPhase(PhaseId),
    /// Weight bookkeeping failed.
    Weight(WeightError),
    /// The dependency network itself is malformed.
    Graph(GraphError),
    /// Completing the activity would leave the partial structure unsound; the
    /// twin refuses to advance.
    UnsoundStructure(StructuralCheckResult),
}

impl fmt::Display for TwinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwinError::UnknownActivity(a) => write!(f, "unknown activity {a}"),
            TwinError::AlreadyCompleted(a) => write!(f, "activity {a} already completed"),
            TwinError::DependenciesNotComplete(deps) => {
                let list: Vec<String> = deps.iter().map(|d| d.to_string()).collect();
                write!(f, "dependencies not complete: {}", list.join(", "))
            }
            TwinError::UnknownPhase(p) => write!(f, "unknown phase {p}"),
            TwinError::Weight(e) => write!(f, "weight model: {e}"),
            TwinError::Graph(e) => write!(f, "activity graph: {e}"),
            TwinError::UnsoundStructure(check) => {
                write!(
                    f,
                    "structure unsound at this state: {}",
                    check.notes.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for TwinError {}

impl From<WeightError> for TwinError {
    fn from(e: WeightError) -> Self {
        TwinError::Weight(e)
    }
}

impl From<GraphError> for TwinError {
    fn from(e: GraphError) -> Self {
        TwinError::Graph(e)
    }
}

/// The construction digital twin.
#[derive(Debug, Clone, PartialEq)]
pub struct DigitalTwin {
    /// The vessel under construction (owned by the twin).
    pub vessel: VesselProject,
    /// Weight and CoG bookkeeping.
    pub weight_model: WeightModel,
    /// Structural bookkeeping summary.
    pub structural_model: StructuralModel,
    /// Progress of the assembly network.
    pub assembly_state: AssemblyState,
    /// How the structure is supported during construction.
    pub support_condition: SupportCondition,
    /// Inspection records.
    pub quality_records: Vec<QualityRecord>,
    /// Sensor stream.
    pub sensor_data: Vec<SensorReading>,
}

impl DigitalTwin {
    /// Creates a twin with an empty weight model whose design target is the
    /// sum of the phase design weights.
    pub fn new(vessel: VesselProject) -> Self {
        let design = vessel
            .build_phases
            .iter()
            .map(|p| p.weight_state.design_kg)
            .sum();
        let weight = WeightModel::new(design, Vector3::ZERO);
        Self::with_weight_model(vessel, weight, SupportCondition::Floating)
    }

    /// Creates a twin with an explicit weight model and support condition.
    pub fn with_weight_model(
        vessel: VesselProject,
        weight_model: WeightModel,
        support_condition: SupportCondition,
    ) -> Self {
        let member_count = vessel.build_phases.iter().map(|p| p.activities.len()).sum();
        Self {
            vessel,
            weight_model,
            structural_model: StructuralModel {
                member_count,
                connected_fraction: 0.0,
            },
            assembly_state: AssemblyState::default(),
            support_condition,
            quality_records: Vec::new(),
            sensor_data: Vec::new(),
        }
    }

    /// Read access to the weight model.
    pub fn weight_model(&self) -> &WeightModel {
        &self.weight_model
    }

    /// The activity-level dependency graph of the project.
    ///
    /// # Errors
    ///
    /// [`TwinError::Graph`] if the project's dependency network is malformed.
    pub fn activity_graph(&self) -> Result<ActivityGraph, TwinError> {
        Ok(self.vessel.activity_graph()?)
    }

    /// Maps activity -> phase index for quick lookups.
    fn phase_of_activity(&self, activity: ActivityId) -> Option<(usize, PhaseId)> {
        for (idx, phase) in self.vessel.build_phases.iter().enumerate() {
            if phase.activities.iter().any(|a| a.id == activity) {
                return Some((idx, phase.id));
            }
        }
        None
    }

    /// Completes an activity.
    ///
    /// The twin validates that every dependency is complete, installs any
    /// weight items wired to this activity ([`tpt_yard_weight::WeightItem::installed_by`]),
    /// recomputes the current mass properties, advances the phase pointer when
    /// the phase finishes, and — critically — verifies that the *now more
    /// complete* structure still stands on its supports before committing.
    ///
    /// On error the twin is left unmodified.
    ///
    /// # Errors
    ///
    /// See [`TwinError`].
    pub fn advance_phase(&mut self, activity: &ActivityId) -> Result<(), TwinError> {
        if self.assembly_state.completed_activities.contains(activity) {
            return Err(TwinError::AlreadyCompleted(*activity));
        }
        // Resolve the graph once; dependency check comes from the *graph
        // statuses*, which mirror assembly_state.
        let graph = self.status_graph()?;
        let node = graph
            .activity(*activity)
            .ok_or(TwinError::UnknownActivity(*activity))?;
        let incomplete: Vec<ActivityId> = node
            .dependencies
            .iter()
            .filter(|d| !self.assembly_state.completed_activities.contains(d))
            .copied()
            .collect();
        if !incomplete.is_empty() {
            return Err(TwinError::DependenciesNotComplete(incomplete));
        }

        // Dry run: what will the installed set look like after this activity?
        let mut weight = self.weight_model.installed_weight();
        let mut moment = Vector3::ZERO;
        for i in &self.weight_model.items {
            let counts = i.status.is_installed()
                || (i.installed_by == Some(*activity) && !matches!(i.status, ItemStatus::Replaced));
            if counts {
                moment = moment + i.cog * i.weight_kg;
                weight += if i.status.is_installed() {
                    0.0
                } else {
                    i.weight_kg
                };
            }
        }
        let hypothetical_cog = if weight > 0.0 {
            moment / weight
        } else {
            Vector3::ZERO
        };
        let check = self.check_supports_for(weight, hypothetical_cog);
        if !check.passed {
            return Err(TwinError::UnsoundStructure(check));
        }

        // Commit.
        self.assembly_state.completed_activities.insert(*activity);
        self.assembly_state.in_progress.remove(activity);
        self.assembly_state.blocked.remove(activity);
        for item in &mut self.weight_model.items {
            if item.installed_by == Some(*activity) {
                item.status = ItemStatus::Installed;
            }
        }
        let installed_cog = self.weight_model.installed_centre_of_gravity()?;
        self.assembly_state.current_mass_properties = MassProperties {
            mass_kg: self.weight_model.installed_weight(),
            cog: installed_cog,
        };
        self.structural_model.connected_fraction = if self.structural_model.member_count > 0 {
            self.assembly_state.completed_activities.len() as f64
                / self.structural_model.member_count as f64
        } else {
            0.0
        };

        // Advance the phase pointer when the current phase is complete.
        let Some((idx, _)) = self.phase_of_activity(*activity) else {
            return Ok(());
        };
        let current_done = self
            .vessel
            .build_phases
            .get(idx)
            .map(|p| {
                p.activities
                    .iter()
                    .all(|a| self.assembly_state.completed_activities.contains(&a.id))
            })
            .unwrap_or(false);
        if current_done {
            if let Some(next) = self.vessel.build_phases.get(idx + 1) {
                self.vessel.current_phase = next.id;
            }
        }
        Ok(())
    }

    /// Builds the project graph with statuses from the assembly state.
    fn status_graph(&self) -> Result<ActivityGraph, TwinError> {
        let mut g = self.vessel.activity_graph()?;
        let ids: Vec<ActivityId> = g.activities().map(|node| node.id).collect();
        for id in ids {
            let status = if self.assembly_state.completed_activities.contains(&id) {
                ActivityStatus::Completed
            } else if self.assembly_state.in_progress.contains(&id) {
                ActivityStatus::InProgress
            } else if self.assembly_state.blocked.contains(&id) {
                ActivityStatus::Blocked
            } else {
                ActivityStatus::Pending
            };
            if let Some(n) = g.activity_mut(id) {
                n.status = status;
            }
        }
        Ok(g)
    }

    /// Structural check at the twin's current state (installed weight only).
    pub fn structural_check(&self) -> StructuralCheckResult {
        let installed = self.weight_model.installed_weight();
        let cog = self
            .weight_model
            .installed_centre_of_gravity()
            .unwrap_or(Vector3::ZERO);
        self.check_supports_for(installed, cog)
    }

    /// Structural check at the end of a given phase: assumes every activity
    /// of phases 1..=phase completed.
    ///
    /// # Errors
    ///
    /// [`TwinError::UnknownPhase`] if the phase does not exist.
    pub fn structural_check_at_phase(
        &self,
        phase: &PhaseId,
    ) -> Result<StructuralCheckResult, TwinError> {
        let upto = self
            .vessel
            .phase_index(*phase)
            .ok_or(TwinError::UnknownPhase(*phase))?;
        let activities: HashSet<ActivityId> = self.vessel.build_phases[..=upto]
            .iter()
            .flat_map(|p| p.activities.iter().map(|a| a.id))
            .collect();
        let mut weight = 0.0;
        let mut moment = Vector3::ZERO;
        for i in &self.weight_model.items {
            let counts = i.status.is_installed()
                || (i.installed_by.is_some_and(|a| activities.contains(&a))
                    && !matches!(i.status, ItemStatus::Replaced));
            if counts {
                weight += i.weight_kg;
                moment = moment + i.cog * i.weight_kg;
            }
        }
        let cog = if weight > 0.0 {
            moment / weight
        } else {
            Vector3::ZERO
        };
        let mut result = self.check_supports_for(weight, cog);
        result.phase_id = *phase;
        Ok(result)
    }

    /// Core support/reaction check for a hypothetical installed weight.
    ///
    /// The structure is treated as a rigid body on its supports; longitudinal
    /// tipping is the dominant construction-phase risk. Transverse stability
    /// is checked as |CoG_y| against the block half-track.
    fn check_supports_for(&self, weight_kg: f64, cog: Vector3) -> StructuralCheckResult {
        let mut notes = Vec::new();
        let mut passed = true;
        let mut reactions: Vec<(String, f64)> = Vec::new();
        let mut margin = 0.0;

        match &self.support_condition {
            SupportCondition::KeelBlocks { positions } if positions.len() >= 2 => {
                let mut xs: Vec<f64> = positions.iter().map(|p| p.x).collect();
                xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let aft = xs[0];
                let fore = xs[xs.len() - 1];
                let span = fore - aft;
                if span <= 0.0 {
                    notes.push("degenerate support span".into());
                    passed = false;
                } else if weight_kg > 0.0 {
                    // Rigid beam on end supports: reactions from statics.
                    let r_fore = weight_kg * (cog.x - aft) / span;
                    let r_aft = weight_kg - r_fore;
                    let g = 9.81;
                    reactions.push(("aft".into(), r_aft * g / 1000.0));
                    reactions.push(("fore".into(), r_fore * g / 1000.0));
                    margin = (cog.x - aft).min(fore - cog.x) / span;
                    if r_fore < 0.0 || r_aft < 0.0 {
                        passed = false;
                        notes.push("CoG outboard of end supports: structure would tip".into());
                    } else if margin < 0.05 {
                        notes.push(format!(
                            "longitudinal margin {:.1}% below the 5% guideline",
                            margin * 100.0
                        ));
                    }
                    let half_track = positions.iter().map(|p| p.y.abs()).fold(0.0f64, f64::max);
                    if half_track > 0.0 && cog.y.abs() > half_track {
                        passed = false;
                        notes.push("CoG outside the block track transversally".into());
                    }
                }
            }
            SupportCondition::KeelBlocks { .. } => {
                notes.push("fewer than two keel supports: check skipped".into());
            }
            SupportCondition::Slipway {
                fore_poppet_x,
                aft_way_x,
            } => {
                let (aft, fore) = if aft_way_x < fore_poppet_x {
                    (*aft_way_x, *fore_poppet_x)
                } else {
                    (*fore_poppet_x, *aft_way_x)
                };
                let span = fore - aft;
                if span > 0.0 && weight_kg > 0.0 {
                    margin = (cog.x - aft).min(fore - cog.x) / span;
                    if cog.x < aft || cog.x > fore {
                        passed = false;
                        notes.push("CoG beyond the ways: tipping risk during launch".into());
                    }
                }
            }
            SupportCondition::Floating => {
                notes.push("afloat: buoyancy assumed, no tipping check".into());
            }
            SupportCondition::Orbital => {
                notes.push("microgravity: no gravity support loads".into());
            }
        }

        StructuralCheckResult {
            phase_id: self.vessel.current_phase,
            passed,
            longitudinal_margin: margin,
            reactions_kn: reactions,
            weight_kg,
            cog,
            notes,
        }
    }

    /// As-built vs design weight deviation.
    pub fn weight_deviation(&self) -> WeightDeviation {
        let best = self.weight_model.total_weight();
        let design = self.weight_model.design_weight_kg;
        WeightDeviation {
            as_built_installed_kg: self.weight_model.installed_weight(),
            best_estimate_kg: best,
            design_kg: design,
            deviation_kg: best - design,
            deviation_pct: if design > 0.0 {
                (best - design) / design * 100.0
            } else {
                0.0
            },
        }
    }

    /// CoG evolution through the phases that have any completed activity.
    ///
    /// Each sample is the cumulative state after that phase's completed
    /// activities, in phase order — the curve a launch officer or orbital
    /// integrator needs.
    pub fn centre_of_gravity_tracking(&self) -> Result<CoGReport, TwinError> {
        // Activities to phase index.
        let mut activity_phase: BTreeMap<ActivityId, usize> = BTreeMap::new();
        for (idx, phase) in self.vessel.build_phases.iter().enumerate() {
            for a in &phase.activities {
                activity_phase.insert(a.id, idx);
            }
        }
        // Items grouped by the phase that installs them.
        let mut samples = CoGReport::default();
        let mut moment = Vector3::ZERO;
        let mut weight = 0.0;
        for (idx, phase) in self.vessel.build_phases.iter().enumerate() {
            for item in &self.weight_model.items {
                let Some(a) = item.installed_by else { continue };
                if activity_phase.get(&a) == Some(&idx)
                    && self.assembly_state.completed_activities.contains(&a)
                    && !matches!(item.status, ItemStatus::Replaced)
                {
                    moment = moment + item.cog * item.weight_kg;
                    weight += item.weight_kg;
                }
            }
            if weight > 0.0 {
                samples.samples.push(CoGSample {
                    phase_id: phase.id,
                    installed_kg: weight,
                    cog: moment / weight,
                });
            }
        }
        Ok(samples)
    }

    /// Records a quality outcome for an activity.
    pub fn record_quality(&mut self, record: QualityRecord) {
        self.quality_records.push(record);
    }

    /// Pushes a sensor reading.
    pub fn push_sensor_reading(&mut self, reading: SensorReading) {
        self.sensor_data.push(reading);
    }

    /// The activity that a weight item's installation is wired to, with the
    /// activity's phase (helper for dashboards).
    pub fn activity(&self, id: ActivityId) -> Option<&AssemblyActivity> {
        self.vessel
            .build_phases
            .iter()
            .flat_map(|p| p.activities.iter())
            .find(|a| a.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_core::{
        ActivityType, BuildPhase, ConstructionMethod, ItemId, PhaseId, ProjectId, SeaVesselType,
        VesselType,
    };
    use tpt_yard_weight::WeightItem;

    /// Builds a straight-line project: `n` phases, one activity each, each
    /// installing one block of `kg` at position `x`.
    fn line_project(n: usize, kg: f64, dx: f64) -> (VesselProject, WeightModel) {
        let mut phases = Vec::new();
        let mut weight = WeightModel::new(n as f64 * kg, Vector3::ZERO);
        for i in 0..n {
            let mut phase = BuildPhase::new(PhaseId(i as u64 + 1), format!("Phase {}", i + 1), 3.0);
            let id = ActivityId(i as u64 + 1);
            let deps: Vec<ActivityId> = if i == 0 {
                vec![]
            } else {
                vec![ActivityId(i as u64)]
            };
            phase.activities.push(
                AssemblyActivity::new(
                    id,
                    format!("Erect block {}", i + 1),
                    ActivityType::JoinBlock,
                    8.0,
                )
                .with_dependencies(&deps),
            );
            weight.add_item(WeightItem {
                id: ItemId(i as u64 + 1),
                name: format!("Block {}", i + 1),
                group: "hull".into(),
                weight_kg: kg,
                cog: Vector3::new(i as f64 * dx, 0.0, 6.0),
                status: ItemStatus::Design,
                margin_pct: 0.0,
                installed_by: Some(id),
            });
            phases.push(phase);
        }
        let project = VesselProject::new(
            ProjectId(1),
            "Line test",
            VesselType::Sea(SeaVesselType::FishingVessel),
            ConstructionMethod::SeaDrydock,
            phases,
        )
        .unwrap();
        (project, weight)
    }

    fn twin_on_blocks(n: usize) -> DigitalTwin {
        let (project, weight) = line_project(n, 100.0, 2.0);
        DigitalTwin::with_weight_model(
            project,
            weight,
            SupportCondition::KeelBlocks {
                positions: vec![
                    Vector3::new(-5.0, -3.0, 0.0),
                    Vector3::new(-5.0, 3.0, 0.0),
                    Vector3::new(25.0, -3.0, 0.0),
                    Vector3::new(25.0, 3.0, 0.0),
                ],
            },
        )
    }

    /// Verification: CoG shifts predictably as blocks are added.
    #[test]
    fn test_cog_tracking() {
        let mut twin = twin_on_blocks(10);
        // Closed form: n blocks of 100 kg at x = 0, 2, ..., 18.
        let expected_final_x = (0..10).map(|i| i as f64 * 2.0).sum::<f64>() / 10.0; // = 9.0
        let mut last_x = f64::MIN;
        for i in 0..10u64 {
            twin.advance_phase(&ActivityId(i + 1)).unwrap();
            let report = twin.centre_of_gravity_tracking().unwrap();
            let sample = report.samples.last().unwrap();
            // CoG marches forward monotonically.
            assert!(
                sample.cog.x > last_x,
                "CoG must move aft->fore monotonically"
            );
            last_x = sample.cog.x;
            // And matches the closed form at each step.
            let n = (i + 1) as f64;
            let expected = (0..(i as i64 + 1)).map(|k| k as f64 * 2.0).sum::<f64>() / n;
            assert!((sample.cog.x - expected).abs() < 1e-9);
            assert_eq!(sample.installed_kg, 100.0 * n);
        }
        assert!((last_x - expected_final_x).abs() < 1e-9);
    }

    #[test]
    fn weight_and_phase_pointer_advance() {
        let mut twin = twin_on_blocks(4);
        assert_eq!(twin.vessel.current_phase, PhaseId(1));
        twin.advance_phase(&ActivityId(1)).unwrap();
        assert_eq!(twin.weight_model().installed_weight(), 100.0);
        assert_eq!(twin.vessel.current_phase, PhaseId(2));
        let dev = twin.weight_deviation();
        assert_eq!(dev.as_built_installed_kg, 100.0);
        assert_eq!(dev.best_estimate_kg, 400.0);
    }

    #[test]
    fn dependencies_are_enforced() {
        let mut twin = twin_on_blocks(3);
        let err = twin.advance_phase(&ActivityId(2)).unwrap_err();
        assert!(matches!(err, TwinError::DependenciesNotComplete(_)));
        twin.advance_phase(&ActivityId(1)).unwrap();
        twin.advance_phase(&ActivityId(2)).unwrap();
    }

    #[test]
    fn double_completion_is_rejected() {
        let mut twin = twin_on_blocks(2);
        twin.advance_phase(&ActivityId(1)).unwrap();
        assert!(matches!(
            twin.advance_phase(&ActivityId(1)),
            Err(TwinError::AlreadyCompleted(_))
        ));
    }

    #[test]
    fn tipping_is_detected_and_blocks_advance() {
        let (project, weight) = line_project(3, 100.0, 2.0);
        let mut twin = DigitalTwin::with_weight_model(
            project,
            weight,
            // Supports around x in [-1, 0.5]: block 1 (CoG 0) stands, but the
            // second block pulls the CoG to x = 1, outboard of the fore
            // support -> the twin must refuse the advance.
            SupportCondition::KeelBlocks {
                positions: vec![Vector3::new(-1.0, 0.0, 0.0), Vector3::new(0.5, 0.0, 0.0)],
            },
        );
        twin.advance_phase(&ActivityId(1)).unwrap();
        let err = twin.advance_phase(&ActivityId(2)).unwrap_err();
        match err {
            TwinError::UnsoundStructure(check) => {
                assert!(!check.passed);
                assert!(!check.notes.is_empty());
            }
            other => panic!("expected unsound structure, got {other:?}"),
        }
        // Refused advance leaves state untouched.
        assert!(!twin
            .assembly_state
            .completed_activities
            .contains(&ActivityId(2)));
        assert_eq!(twin.weight_model().installed_weight(), 100.0);
    }

    #[test]
    fn structural_check_at_phase_projects_end_state() {
        let twin = twin_on_blocks(5);
        let check = twin.structural_check_at_phase(&PhaseId(5)).unwrap();
        assert_eq!(check.weight_kg, 500.0);
        assert!(check.passed);
        assert!(check.reactions_kn.iter().all(|(_, r)| *r > 0.0));
        assert!(matches!(
            twin.structural_check_at_phase(&PhaseId(99)),
            Err(TwinError::UnknownPhase(_))
        ));
    }

    #[test]
    fn orbital_support_always_passes() {
        let (project, weight) = line_project(2, 100.0, 2.0);
        let mut twin = DigitalTwin::with_weight_model(project, weight, SupportCondition::Orbital);
        twin.advance_phase(&ActivityId(1)).unwrap();
        let check = twin.structural_check();
        assert!(check.passed);
        assert!(check.notes.iter().any(|n| n.contains("microgravity")));
    }

    #[test]
    fn slipway_cog_must_stay_on_the_ways() {
        let (project, weight) = line_project(2, 100.0, 200.0); // block 2 far forward
        let mut twin = DigitalTwin::with_weight_model(
            project,
            weight,
            SupportCondition::Slipway {
                fore_poppet_x: 100.0,
                aft_way_x: -50.0,
            },
        );
        twin.advance_phase(&ActivityId(1)).unwrap(); // CoG at 0: fine
        assert!(twin.structural_check().passed);
        twin.advance_phase(&ActivityId(2)).unwrap(); // CoG moves to ~100
        let check = twin.structural_check();
        // CoG at x=100.0 == fore poppet edge; margin 0 -> passes with note
        // (tipping margin below guideline is a note, not a failure).
        assert!(check.passed);
    }

    #[test]
    fn quality_and_sensor_bookkeeping() {
        let mut twin = twin_on_blocks(1);
        twin.record_quality(QualityRecord {
            id: 1,
            activity_id: ActivityId(1),
            accepted: true,
            description: "visual weld inspection passed".into(),
        });
        twin.push_sensor_reading(SensorReading {
            timestamp_s: 0.0,
            sensor: "block-1-strain".into(),
            value: 12.5,
        });
        assert_eq!(twin.quality_records.len(), 1);
        assert_eq!(twin.sensor_data.len(), 1);
    }
}
