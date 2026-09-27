//! Construction scheduling: critical path, optimisation, and resource
//! levelling.
//!
//! [`ShipyardScheduler`] takes the assembly-activity network (from
//! `tpt-yard-core` / `tpt-yard-assembly`) and answers the planner's
//! questions: which activities drive the makespan
//! ([`critical_path`](ShipyardScheduler::critical_path)), how long the
//! project takes under an objective
//! ([`optimize_sequence`](ShipyardScheduler::optimize_sequence)), and how
//! to flatten resource peaks
//! ([`resource_leveling`](ShipyardScheduler::resource_leveling)).
//!
//! # Example
//!
//! ```
//! use tpt_yard_assembly::ActivityId;
//! use tpt_yard_core::{ActivityType, AssemblyActivity};
//! use tpt_yard_scheduling::{ScheduleObjective, ShipyardScheduler};
//!
//! let acts = vec![
//!     AssemblyActivity::new(ActivityId(1), "Cut", ActivityType::CutSteel, 8.0),
//!     AssemblyActivity::new(ActivityId(2), "Weld", ActivityType::WeldBlock, 16.0)
//!         .with_dependencies(&[ActivityId(1)]),
//!     AssemblyActivity::new(ActivityId(3), "Erect", ActivityType::JoinBlock, 4.0)
//!         .with_dependencies(&[ActivityId(2)]),
//! ];
//! let scheduler = ShipyardScheduler::new(acts);
//! let path = scheduler.critical_path().unwrap();
//! assert_eq!(path, vec![ActivityId(1), ActivityId(2), ActivityId(3)]);
//! ```

use std::collections::BTreeMap;

use tpt_yard_assembly::{ActivityGraph, ActivityId, GraphError};
use tpt_yard_core::{AssemblyActivity, ResourceKind};

/// Scheduling objectives (spec §5, Domain 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleObjective {
    /// Shortest project duration (the CPM baseline).
    MinimizeDuration,
    /// Minimize cost proxy: fewer parallel resource draws.
    MinimizeCost,
    /// Flatten crane demand peaks.
    MinimizeCraneUsage,
    /// Shorten the dock-occupancy window (critical path via dock tasks).
    MinimizeDrydockTime,
    /// Maximum parallelism (earliest starts everywhere).
    MaximizeParallelism,
}

/// Result of a scheduling pass.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleResult {
    /// The objective applied.
    pub objective: ScheduleObjective,
    /// Project duration under this schedule, hours.
    pub makespan_hours: f64,
    /// Execution order (topological, priority-sorted).
    pub order: Vec<ActivityId>,
    /// Peak concurrent resource demand per kind, under this schedule.
    pub peak_resource_use: Vec<(ResourceKind, f64)>,
    /// Findings.
    pub notes: Vec<String>,
}

/// Errors from scheduling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// The activity network is malformed.
    Graph(GraphError),
    /// Empty schedules are meaningless.
    EmptySchedule,
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleError::Graph(e) => write!(f, "graph: {e}"),
            ScheduleError::EmptySchedule => f.write_str("no activities to schedule"),
        }
    }
}

impl std::error::Error for ScheduleError {}

impl From<GraphError> for ScheduleError {
    fn from(e: GraphError) -> Self {
        ScheduleError::Graph(e)
    }
}

/// One scheduled activity instance.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledActivity {
    /// Activity id.
    pub id: ActivityId,
    /// Earliest start, hours from project start.
    pub earliest_start_h: f64,
    /// Latest start without delaying the project, hours.
    pub latest_start_h: f64,
    /// Schedule float, hours (zero = critical).
    pub float_h: f64,
    /// Resources drawn while running.
    pub resources: Vec<(ResourceKind, f64)>,
}

/// The shipyard scheduler.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShipyardScheduler {
    /// Activities to schedule.
    pub activities: Vec<AssemblyActivity>,
}

impl ShipyardScheduler {
    /// Creates a scheduler over the given activities.
    pub fn new(activities: Vec<AssemblyActivity>) -> Self {
        Self { activities }
    }

    fn graph(&self) -> Result<ActivityGraph, ScheduleError> {
        let mut g = ActivityGraph::new();
        for a in &self.activities {
            if !g.contains(a.id) {
                g.add_activity(a.id, a.name.clone(), a.duration_hours, &a.dependencies)?;
            }
        }
        Ok(g)
    }

    /// CPM pass: per-activity earliest/latest starts and float.
    ///
    /// # Errors
    ///
    /// [`ScheduleError`] on malformed networks or an empty schedule.
    pub fn cpm(&self) -> Result<BTreeMap<ActivityId, ScheduledActivity>, ScheduleError> {
        if self.activities.is_empty() {
            return Err(ScheduleError::EmptySchedule);
        }
        let g = self.graph()?;
        let order = g.topological_order()?;
        let finish = g.earliest_finish_times()?;
        let makespan = g.makespan_hours()?;

        // Latest start: backward pass from the makespan.
        let mut latest: BTreeMap<ActivityId, f64> = BTreeMap::new();
        for &id in order.iter().rev() {
            let node = g.activity(id).expect("exists");
            let dependents = g.dependents_of(id);
            let ls = if dependents.is_empty() {
                makespan - node.duration_hours
            } else {
                dependents
                    .iter()
                    .filter_map(|d| latest.get(d).copied())
                    .fold(f64::INFINITY, f64::min)
                    - node.duration_hours
            };
            latest.insert(id, ls);
        }

        let mut out = BTreeMap::new();
        for (&id, &ef) in &finish {
            let node = g.activity(id).expect("exists");
            let earliest_start = ef - node.duration_hours;
            let latest_start = latest[&id];
            let resources = self
                .activities
                .iter()
                .find(|a| a.id == id)
                .map(|a| a.resources.iter().map(|r| (r.kind, r.capacity)).collect())
                .unwrap_or_default();
            out.insert(
                id,
                ScheduledActivity {
                    id,
                    earliest_start_h: earliest_start,
                    latest_start_h: latest_start,
                    float_h: (latest_start - earliest_start).max(0.0),
                    resources,
                },
            );
        }
        Ok(out)
    }

    /// The critical path: zero-float activities from start to finish.
    ///
    /// # Errors
    ///
    /// [`ScheduleError`] on malformed networks.
    pub fn critical_path(&self) -> Result<Vec<ActivityId>, ScheduleError> {
        let cpm = self.cpm()?;
        let mut path: Vec<ActivityId> = cpm
            .values()
            .filter(|s| s.float_h < 1e-9)
            .map(|s| s.id)
            .collect();
        path.sort();
        Ok(path)
    }

    /// Schedules under an objective.
    ///
    /// - [`ScheduleObjective::MinimizeDuration`], `MaximizeParallelism`,
    ///   `MinimizeDrydockTime`: earliest starts (pure CPM).
    /// - `MinimizeCraneUsage` / `MinimizeCost`: serial schedule, activities
    ///   priority-sorted by float (minimum slack first) — non-critical work
    ///   is pushed into its float, flattening resource peaks.
    /// # Errors
    ///
    /// [`ScheduleError`] on malformed networks.
    pub fn optimize_sequence(
        &self,
        objective: ScheduleObjective,
    ) -> Result<ScheduleResult, ScheduleError> {
        if self.activities.is_empty() {
            return Err(ScheduleError::EmptySchedule);
        }
        let g = self.graph()?;
        let cpm = self.cpm()?;
        let duration_mode = matches!(
            objective,
            ScheduleObjective::MinimizeDuration
                | ScheduleObjective::MaximizeParallelism
                | ScheduleObjective::MinimizeDrydockTime
        );

        let mut order = g.topological_order()?;
        if !duration_mode {
            // Min-float-first priority (a classic levelling heuristic).
            order.sort_by(|&a, &b| {
                let fa = cpm[&a].float_h;
                let fb = cpm[&b].float_h;
                fa.total_cmp(&fb).then(a.cmp(&b))
            });
        }

        // Serial schedule: place each activity after its dependencies; in
        // levelling mode respect the priority order when contention occurs.
        let mut start: BTreeMap<ActivityId, f64> = BTreeMap::new();
        // Placed activities for the resource-conflict scan.
        let mut placed: Vec<(ActivityId, f64, f64)> = Vec::new(); // (id, start, end)
        for &id in &order {
            let node = g.activity(id).expect("exists");
            let dep_finish = node
                .dependencies
                .iter()
                .map(|d| {
                    let dur = g.activity(*d).expect("exists").duration_hours;
                    start.get(d).copied().unwrap_or(0.0) + dur
                })
                .fold(0.0f64, f64::max);
            let earliest = dep_finish.max(cpm[&id].earliest_start_h);
            let s = if duration_mode {
                earliest
            } else {
                // Levelling: serial schedule generation — place at the
                // earliest clash-free time for this activity's resources,
                // scanning past its CPM float if needed (levelling may
                // legitimately stretch the makespan to flatten peaks).
                let dur = node.duration_hours;
                let resources = &self
                    .activities
                    .iter()
                    .find(|a| a.id == id)
                    .map(|a| {
                        a.resources
                            .iter()
                            .filter(|r| r.capacity > 0.0)
                            .map(|r| r.kind)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let mut t = earliest;
                let mut found = None;
                // Bound: one full pass of the schedule horizon.
                while t <= earliest + 10_000.0 {
                    let clash = placed.iter().any(|(pid, ps, pe)| {
                        let other = self
                            .activities
                            .iter()
                            .find(|a| a.id == *pid)
                            .expect("placed");
                        let overlaps_time = t < *pe && *ps < t + dur;
                        overlaps_time
                            && other
                                .resources
                                .iter()
                                .filter(|r| r.capacity > 0.0)
                                .any(|r| resources.contains(&r.kind))
                    });
                    if !clash {
                        found = Some(t);
                        break;
                    }
                    t += 1.0;
                }
                found.unwrap_or(earliest)
            };
            start.insert(id, s);
            placed.push((id, s, s + node.duration_hours));
        }
        let makespan = order
            .iter()
            .map(|&id| start[&id] + g.activity(id).expect("exists").duration_hours)
            .fold(0.0f64, f64::max);

        // Peak concurrent demand per resource kind on a 1-hour grid.
        let mut peaks: BTreeMap<ResourceKind, f64> = BTreeMap::new();
        let grid_end = makespan.ceil() as i64;
        for t in 0..grid_end {
            let tf = t as f64;
            let mut active: BTreeMap<ResourceKind, f64> = BTreeMap::new();
            for a in &self.activities {
                let s = start[&a.id];
                if s <= tf && tf < s + a.duration_hours {
                    for r in &a.resources {
                        *active.entry(r.kind).or_insert(0.0) += r.capacity;
                    }
                }
            }
            for (k, v) in active {
                let e = peaks.entry(k).or_insert(0.0);
                if v > *e {
                    *e = v;
                }
            }
        }
        let mut peak_resource_use: Vec<(ResourceKind, f64)> = peaks.into_iter().collect();
        peak_resource_use.sort_by(|a, b| resource_kind_name(a.0).cmp(resource_kind_name(b.0)));

        let notes = vec![format!("objective {objective:?}: makespan {makespan:.1} h")];
        Ok(ScheduleResult {
            objective,
            makespan_hours: makespan,
            order,
            peak_resource_use,
            notes,
        })
    }

    /// Resource levelling: the levelling schedule's peak demand against the
    /// earliest-start schedule's.
    ///
    /// # Errors
    ///
    /// [`ScheduleError`] on malformed networks.
    pub fn resource_leveling(&self) -> Result<ScheduleResult, ScheduleError> {
        let mut levelled = self.optimize_sequence(ScheduleObjective::MinimizeCraneUsage)?;
        let early = self.optimize_sequence(ScheduleObjective::MinimizeDuration)?;
        levelled.notes.push(format!(
            "earliest-start makespan {:.1} h vs levelled {:.1} h",
            early.makespan_hours, levelled.makespan_hours
        ));
        Ok(levelled)
    }
}

/// Stable name of a resource kind for reports.
pub fn resource_kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Crane => "crane",
        ResourceKind::Workshop => "workshop",
        ResourceKind::Drydock => "drydock",
        ResourceKind::WeldingStation => "welding",
        ResourceKind::Robot => "robot",
        ResourceKind::Crew => "crew",
        ResourceKind::Transport => "transport",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_core::{ActivityType, Resource};

    fn network() -> ShipyardScheduler {
        //       +--> B (16h) --+
        //   A(8)+              +--> D (10h)
        //       +--> C (12h) --+
        // E (20h) independent
        ShipyardScheduler::new(vec![
            AssemblyActivity::new(ActivityId(1), "A", ActivityType::CutSteel, 8.0),
            AssemblyActivity::new(ActivityId(2), "B", ActivityType::WeldBlock, 16.0)
                .with_dependencies(&[ActivityId(1)]),
            AssemblyActivity::new(ActivityId(3), "C", ActivityType::WeldBlock, 12.0)
                .with_dependencies(&[ActivityId(1)]),
            AssemblyActivity::new(ActivityId(4), "D", ActivityType::JoinBlock, 10.0)
                .with_dependencies(&[ActivityId(2), ActivityId(3)]),
            AssemblyActivity::new(ActivityId(5), "E", ActivityType::Paint, 20.0),
        ])
    }

    /// Verification: critical path of the reference network.
    #[test]
    fn critical_path_of_reference_network() {
        let s = network();
        let cp = s.critical_path().unwrap();
        // A(8) -> C(12) -> D(10) = 30 h vs A->B->D = 34 h. B is critical!
        // A(8)+B(16)+D(10) = 34 h.
        assert_eq!(cp, vec![ActivityId(1), ActivityId(2), ActivityId(4)]);
        let cpm = s.cpm().unwrap();
        assert!((cpm[&ActivityId(3)].float_h - 4.0).abs() < 1e-9); // C has 4 h float
        assert!(cpm[&ActivityId(2)].float_h < 1e-9);
        assert!(cpm[&ActivityId(5)].float_h > 0.0); // E floats freely
    }

    #[test]
    fn makespan_matches_longest_path() {
        let s = network();
        let r = s
            .optimize_sequence(ScheduleObjective::MinimizeDuration)
            .unwrap();
        assert!((r.makespan_hours - 34.0).abs() < 1e-9);
    }

    #[test]
    fn leveling_flattens_peaks_at_makespan_cost() {
        // A(8) -> B(16) -> E(8);  A -> C(8) -> F(8), each of B and C taking
        // the single crane. Earliest start runs them together (peak 2);
        // levelling pushes C past B (peak 1) and stretches the makespan.
        let mut acts = vec![
            AssemblyActivity::new(ActivityId(1), "A", ActivityType::CutSteel, 8.0),
            AssemblyActivity::new(ActivityId(2), "B", ActivityType::WeldBlock, 16.0)
                .with_dependencies(&[ActivityId(1)]),
            AssemblyActivity::new(ActivityId(3), "C", ActivityType::WeldBlock, 8.0)
                .with_dependencies(&[ActivityId(1)]),
            AssemblyActivity::new(ActivityId(4), "E", ActivityType::JoinBlock, 8.0)
                .with_dependencies(&[ActivityId(2)]),
            AssemblyActivity::new(ActivityId(5), "F", ActivityType::JoinBlock, 8.0)
                .with_dependencies(&[ActivityId(3)]),
        ];
        for a in &mut acts {
            a.resources.push(Resource {
                name: "crane".into(),
                kind: ResourceKind::Crane,
                capacity: 1.0,
            });
        }
        let s = ShipyardScheduler::new(acts);
        let early = s
            .optimize_sequence(ScheduleObjective::MinimizeDuration)
            .unwrap();
        let levelled = s.resource_leveling().unwrap();
        let peak_of = |r: &ScheduleResult| {
            r.peak_resource_use
                .iter()
                .find(|(k, _)| *k == ResourceKind::Crane)
                .map(|(_, v)| *v)
                .unwrap_or(0.0)
        };
        assert_eq!(peak_of(&early), 2.0);
        assert_eq!(
            peak_of(&levelled),
            1.0,
            "levelling must flatten the crane peak"
        );
        // ...at the cost of a longer makespan.
        assert!(levelled.makespan_hours > early.makespan_hours);
    }

    #[test]
    fn empty_schedule_rejected() {
        let s = ShipyardScheduler::new(vec![]);
        assert_eq!(s.critical_path(), Err(ScheduleError::EmptySchedule));
        assert_eq!(
            s.optimize_sequence(ScheduleObjective::MinimizeDuration),
            Err(ScheduleError::EmptySchedule)
        );
    }

    #[test]
    fn cycle_rejected() {
        let s = ShipyardScheduler::new(vec![
            AssemblyActivity::new(ActivityId(1), "a", ActivityType::CutSteel, 1.0)
                .with_dependencies(&[ActivityId(2)]),
            AssemblyActivity::new(ActivityId(2), "b", ActivityType::CutSteel, 1.0),
        ]);
        // Building the graph fails: dependency 2 unknown when adding 1.
        assert!(matches!(s.critical_path(), Err(ScheduleError::Graph(_))));
    }
}
