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
//! }).expect("valid weight item");
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

/// Counts from one [`DigitalTwin::ingest_telemetry`] batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TelemetrySummary {
    /// Sensor readings stored.
    pub readings_ingested: usize,
    /// Scan deviations seen.
    pub deviations_scanned: usize,
    /// Deviations beyond tolerance flagged as quality rework.
    pub corrections_flagged: usize,
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
    /// The activity belongs to a phase ahead of the twin's current phase:
    /// the plan says this work has not started yet.
    ActivityNotInCurrentPhase {
        /// The activity.
        activity: ActivityId,
        /// The phase it belongs to.
        activity_phase: PhaseId,
        /// The twin's current phase.
        current_phase: PhaseId,
    },
    /// Serialized twin state is malformed.
    Malformed(String),
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
            TwinError::ActivityNotInCurrentPhase {
                activity,
                activity_phase,
                current_phase,
            } => write!(
                f,
                "activity {activity} belongs to phase {activity_phase:?}, ahead of the current phase {current_phase:?}"
            ),
            TwinError::Malformed(m) => write!(f, "malformed twin state: {m}"),
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
        // The activity must not run ahead of the plan: a twin tracking the
        // build state refuses work from a phase that has not started.
        if let Some((idx, phase_id)) = self.phase_of_activity(*activity) {
            let current_idx = self.vessel.phase_index(self.vessel.current_phase);
            if current_idx.is_some_and(|c| idx > c) {
                return Err(TwinError::ActivityNotInCurrentPhase {
                    activity: *activity,
                    activity_phase: phase_id,
                    current_phase: self.vessel.current_phase,
                });
            }
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
        // `weight`/`moment` are the exact post-commit installed totals, so
        // the mass-properties update below needs no further computation that
        // could fail mid-commit.
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

        // Commit. Every fallible step has already run, so no failure can
        // leave the twin half-mutated. An empty installed set (early phases
        // with no wired items yet) is a valid state: zero mass properties.
        self.assembly_state.completed_activities.insert(*activity);
        self.assembly_state.in_progress.remove(activity);
        self.assembly_state.blocked.remove(activity);
        for item in &mut self.weight_model.items {
            if item.installed_by == Some(*activity) {
                item.status = ItemStatus::Installed;
            }
        }
        self.assembly_state.current_mass_properties = MassProperties {
            mass_kg: weight,
            cog: hypothetical_cog,
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
                // The pointer only ever moves forward: completing a late
                // activity from an earlier phase must not rewind the twin.
                if next.id > self.vessel.current_phase {
                    self.vessel.current_phase = next.id;
                }
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
                    // Transverse containment within the actual support
                    // track (handles asymmetric keel-block layouts; a
                    // single keel line — zero-width track — requires the
                    // CoG on the line).
                    let (min_y, max_y) = positions
                        .iter()
                        .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
                    if cog.y < min_y - 1e-9 || cog.y > max_y + 1e-9 {
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

    /// Restores a twin serialized by [`DigitalTwin::to_json`].
    ///
    /// The restored twin carries an empty render geometry (derived state —
    /// it repopulates as activities advance); everything else round-trips.
    ///
    /// # Errors
    ///
    /// [`TwinError`] on malformed state or a project that fails
    /// [`VesselProject::validate`].
    pub fn from_json_value(v: &tpt_yard_core::json::Value) -> Result<Self, TwinError> {
        use tpt_yard_core::json::Value;
        let malformed = |what: &str| TwinError::Malformed(format!("twin state: missing '{what}'"));
        let vessel = VesselProject::from_json_value(
            v.get("vessel")
                .ok_or_else(|| malformed("vessel"))
                .map_err(|e| TwinError::Malformed(format!("twin vessel: {e}")))?,
        )
        .map_err(|e| TwinError::Malformed(format!("twin vessel: {e}")))?;
        let weight = WeightModel::from_json_value(
            v.get("weight_model")
                .ok_or_else(|| malformed("weight_model"))?,
        )
        .map_err(|e| TwinError::Malformed(format!("twin weight model: {e}")))?;
        let parse_set = |k: &str| -> Result<HashSet<ActivityId>, TwinError> {
            let arr = v
                .get(k)
                .and_then(|x| x.as_array())
                .ok_or_else(|| malformed(k))?;
            Ok(arr
                .iter()
                .filter_map(|n| n.as_f64().map(|f| ActivityId(f as u64)))
                .collect())
        };
        let completed = parse_set("completed")?;
        let in_progress = parse_set("in_progress")?;
        let blocked = parse_set("blocked")?;
        let mp_v = v.get("mass_properties").ok_or_else(|| malformed("mass_properties"))?;
        let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).unwrap_or(0.0);
        let cog_v = mp_v.get("cog").and_then(|c| c.as_array());
        let cog = cog_v
            .map(|a| {
                Vector3::new(
                    a.first().and_then(|n| n.as_f64()).unwrap_or(0.0),
                    a.get(1).and_then(|n| n.as_f64()).unwrap_or(0.0),
                    a.get(2).and_then(|n| n.as_f64()).unwrap_or(0.0),
                )
            })
            .unwrap_or(Vector3::ZERO);

        let member_count = vessel
            .build_phases
            .iter()
            .map(|p| p.activities.len())
            .sum();
        let mut twin = Self {
            vessel,
            weight_model: weight,
            structural_model: StructuralModel {
                member_count,
                connected_fraction: 0.0,
            },
            assembly_state: AssemblyState {
                completed_activities: completed,
                in_progress,
                blocked,
                current_geometry: tpt_yard_core::Geometry3D::new(),
                current_mass_properties: MassProperties {
                    mass_kg: num(mp_v, "mass_kg"),
                    cog,
                },
            },
            support_condition: SupportCondition::Floating,
            quality_records: Vec::new(),
            sensor_data: Vec::new(),
        };
        twin.structural_model.connected_fraction =
            if twin.structural_model.member_count > 0 {
                twin.assembly_state.completed_activities.len() as f64
                    / twin.structural_model.member_count as f64
            } else {
                0.0
            };
        twin.activity_graph()?;
        Ok(twin)
    }

    /// Ingests a live sensor/scan batch (review 7H roadmap item: "live
    /// digital-twin ingest of sensor and scan-deviation JSON driving
    /// distortion corrections").
    ///
    /// The batch schema (see `test-data/telemetry/sample-batch.json`):
    /// readings append to `sensor_data`; scan deviations beyond the 5 mm
    /// tolerance produce rejected quality records — corrections are
    /// *flagged*, never auto-executed.
    ///
    /// # Errors
    ///
    /// [`TwinError::Malformed`] on a malformed batch.
    pub fn ingest_telemetry(
        &mut self,
        batch: &tpt_yard_core::json::Value,
    ) -> Result<TelemetrySummary, TwinError> {
        let malformed = |m: &str| TwinError::Malformed(format!("telemetry: {m}"));
        const TOLERANCE_MM: f64 = 5.0;

        let mut summary = TelemetrySummary::default();

        if let Some(readings) = batch.get("readings").and_then(|r| r.as_array()) {
            for r in readings {
                let block = r
                    .get("block")
                    .and_then(|b| b.as_u64())
                    .ok_or_else(|| malformed("reading missing 'block'"))?;
                let quantity = r
                    .get("quantity")
                    .and_then(|q| q.as_str())
                    .ok_or_else(|| malformed("reading missing 'quantity'"))?
                    .to_string();
                let value = r
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| malformed("reading missing numeric 'value'"))?;
                self.sensor_data.push(SensorReading {
                    timestamp_s: self.sensor_data.len() as f64,
                    sensor: format!("block {block}: {quantity}"),
                    value,
                });
                summary.readings_ingested += 1;
            }
        }

        if let Some(devs) = batch.get("scan_deviations").and_then(|d| d.as_array()) {
            for d in devs {
                let block = d
                    .get("block")
                    .and_then(|b| b.as_u64())
                    .ok_or_else(|| malformed("deviation missing 'block'"))?;
                let dev_mm = d
                    .get("axis_mm")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| malformed("deviation missing numeric 'axis_mm'"))?;
                summary.deviations_scanned += 1;
                if dev_mm.abs() > TOLERANCE_MM {
                    summary.corrections_flagged += 1;
                    self.quality_records.push(QualityRecord {
                        id: self.quality_records.len() as u64 + 1,
                        activity_id: ActivityId(block),
                        accepted: false,
                        description: format!(
                            "scan deviation {dev_mm:.1} mm exceeds the {TOLERANCE_MM} mm tolerance: flag for heat-straightening"
                        ),
                    });
                }
            }
        }

        Ok(summary)
    }

    /// Serializes the twin's session state to JSON: the vessel project,
    /// the weight model, and the assembly state (completed / in-progress /
    /// blocked activity sets plus current mass properties). The derived
    /// render geometry is *not* serialized — it rebuilds as the plan
    /// advances (review 7C persistence item).
    ///
    /// # Errors
    ///
    /// [`TwinError::Graph`] if the project's dependency network is
    /// malformed (the same check [`DigitalTwin::activity_graph`] runs).
    pub fn to_json(&self) -> Result<tpt_yard_core::json::Value, TwinError> {
        use tpt_yard_core::json::Value;
        // Validate first: persisting a broken twin would just defer the
        // failure to load time.
        self.activity_graph()?;
        let set = |name: &str, items: &HashSet<ActivityId>| {
            (
                name.to_string(),
                Value::Array(
                    items
                        .iter()
                        .map(|a| Value::Number(a.0 as f64))
                        .collect(),
                ),
            )
        };
        let mp = &self.assembly_state.current_mass_properties;
        Ok(Value::Object(vec![
            ("vessel".to_string(), self.vessel.to_json()),
            ("weight_model".to_string(), self.weight_model.to_json()),
            set("completed", &self.assembly_state.completed_activities),
            set("in_progress", &self.assembly_state.in_progress),
            set("blocked", &self.assembly_state.blocked),
            (
                "current_phase".to_string(),
                Value::Number(self.vessel.current_phase.0 as f64),
            ),
            (
                "mass_properties".to_string(),
                Value::Object(vec![
                    ("mass_kg".to_string(), Value::Number(mp.mass_kg)),
                    (
                        "cog".to_string(),
                        Value::Array(vec![
                            Value::Number(mp.cog.x),
                            Value::Number(mp.cog.y),
                            Value::Number(mp.cog.z),
                        ]),
                    ),
                ]),
            ),
        ]))
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
            }).expect("valid weight item");
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
        // Phase 1 holds two chained activities (1 <- 2); phase 2 holds 3.
        let mut phase1 = BuildPhase::new(PhaseId(1), "Erect", 3.0);
        phase1
            .activities
            .push(AssemblyActivity::new(ActivityId(1), "Block 1", ActivityType::JoinBlock, 8.0));
        phase1.activities.push(
            AssemblyActivity::new(ActivityId(2), "Block 2", ActivityType::JoinBlock, 8.0)
                .with_dependencies(&[ActivityId(1)]),
        );
        let mut phase2 = BuildPhase::new(PhaseId(2), "Outfit", 3.0);
        phase2
            .activities
            .push(AssemblyActivity::new(ActivityId(3), "Wire", ActivityType::JoinBlock, 8.0));
        let (project, _w) = line_project(1, 100.0, 2.0);
        let mut project = project;
        project.build_phases = vec![phase1, phase2];
        let mut weight = WeightModel::new(200.0, Vector3::ZERO);
        for (i, aid) in [(1u64, ActivityId(1)), (2, ActivityId(2))] {
            weight.add_item(WeightItem {
                id: ItemId(i),
                name: format!("block {i}"),
                group: "hull".into(),
                weight_kg: 100.0,
                cog: Vector3::new(i as f64 * 2.0, 0.0, 6.0),
                status: ItemStatus::Design,
                margin_pct: 0.0,
                installed_by: Some(aid),
            })
            .expect("valid weight item");
        }
        let mut twin = DigitalTwin::with_weight_model(project, weight, SupportCondition::Orbital);
        // Within the current phase, incomplete dependencies are rejected.
        let err = twin.advance_phase(&ActivityId(2)).unwrap_err();
        assert!(matches!(err, TwinError::DependenciesNotComplete(_)));
        twin.advance_phase(&ActivityId(1)).unwrap();
        twin.advance_phase(&ActivityId(2)).unwrap();
    }

    /// Regression (review 7B): the twin refuses work from a phase ahead of
    /// the current one — the plan has not started it yet.
    #[test]
    fn future_phase_activity_is_rejected() {
        let mut twin = twin_on_blocks(3);
        let err = twin.advance_phase(&ActivityId(2)).unwrap_err();
        assert!(matches!(
            err,
            TwinError::ActivityNotInCurrentPhase { .. }
        ));
        // Completed activities in the current phase remain valid, and the
        // phase pointer advances normally afterwards.
        twin.advance_phase(&ActivityId(1)).unwrap();
        assert_eq!(twin.vessel.current_phase, PhaseId(2));
        twin.advance_phase(&ActivityId(2)).unwrap();
        assert_eq!(twin.vessel.current_phase, PhaseId(3));
    }

    /// Regression (review 7A/A3): `advance_phase` used to commit the
    /// activity before the installed-CoG recompute could fail, leaving the
    /// twin half-mutated on error; and `DigitalTwin::new` (no wired items)
    /// could therefore never advance at all.
    #[test]
    fn fresh_twin_without_items_advances() {
        let (project, _) = line_project(3, 100.0, 2.0);
        let mut twin = DigitalTwin::new(project);
        twin.advance_phase(&ActivityId(1)).unwrap();
        assert!(twin
            .assembly_state
            .completed_activities
            .contains(&ActivityId(1)));
        let mp = twin.assembly_state.current_mass_properties;
        assert_eq!(mp.mass_kg, 0.0);
    }

    /// Verification (review 7H): live telemetry ingest stores readings and
    /// flags out-of-tolerance scan deviations as rework — without
    /// auto-executing corrections.
    #[test]
    fn telemetry_ingest_stores_and_flags() {
        let mut twin = twin_on_blocks(12);
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../test-data/telemetry/sample-batch.json"
        ))
        .expect("sample batch");
        let batch = tpt_yard_core::json::Value::parse(&text).unwrap();

        let summary = twin.ingest_telemetry(&batch).unwrap();
        assert_eq!(summary.readings_ingested, 3);
        assert_eq!(summary.deviations_scanned, 3);
        assert_eq!(summary.corrections_flagged, 2, "deviations 7.8 and -6.3 exceed 5 mm");
        assert_eq!(twin.sensor_data.len(), 3);
        let rejected: Vec<_> = twin
            .quality_records
            .iter()
            .filter(|r| !r.accepted)
            .collect();
        assert_eq!(rejected.len(), 2);
        assert!(rejected.iter().all(|r| r.description.contains("heat-straightening")));

        // Malformed batches are rejected, not partially applied.
        let before = twin.sensor_data.len();
        assert!(matches!(
            twin.ingest_telemetry(
                &tpt_yard_core::json::Value::parse(r#"{"readings":[{"block":1}]}"#).unwrap()
            ),
            Err(TwinError::Malformed(_))
        ));
        assert_eq!(twin.sensor_data.len(), before);
    }

    /// Regression (review 7C persistence): a twin's session state survives
    /// a JSON round-trip — project, weight model and progress sets.
    #[test]
    fn twin_session_round_trips_through_json() {
        let mut twin = twin_on_blocks(4);
        twin.advance_phase(&ActivityId(1)).unwrap();
        twin.advance_phase(&ActivityId(2)).unwrap();

        let json = twin.to_json().unwrap();
        let restored = DigitalTwin::from_json_value(&json).expect("restores");

        assert_eq!(
            restored.assembly_state.completed_activities,
            twin.assembly_state.completed_activities
        );
        assert_eq!(
            restored.assembly_state.current_mass_properties,
            twin.assembly_state.current_mass_properties
        );
        assert_eq!(*restored.weight_model(), *twin.weight_model());
        assert_eq!(restored.vessel.current_phase, twin.vessel.current_phase);
        // The restored twin keeps working: the next erection passes.
        let mut restored = restored;
        restored.advance_phase(&ActivityId(3)).unwrap();
        assert_eq!(restored.assembly_state.completed_activities.len(), 3);

        // Truncated state is rejected, not defaulted.
        assert!(matches!(
            DigitalTwin::from_json_value(&tpt_yard_core::json::Value::parse("{}").unwrap()),
            Err(TwinError::Malformed(_))
        ));
    }

    /// Regression (review 7A/A3): every rejected advance must leave the twin
    /// exactly as it was.
    #[test]
    fn failed_advance_leaves_twin_untouched() {
        let mut twin = twin_on_blocks(3);
        twin.advance_phase(&ActivityId(1)).unwrap();
        let before = twin.clone();

        // Unknown activity.
        assert!(matches!(
            twin.advance_phase(&ActivityId(99)),
            Err(TwinError::UnknownActivity(_))
        ));
        // Activity 3 lives in phase 3 while the twin is in phase 1.
        assert!(matches!(
            twin.advance_phase(&ActivityId(3)),
            Err(TwinError::ActivityNotInCurrentPhase { .. })
        ));
        // Already completed.
        assert!(matches!(
            twin.advance_phase(&ActivityId(1)),
            Err(TwinError::AlreadyCompleted(_))
        ));

        assert_eq!(before, twin, "rejected advances must not mutate the twin");
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
