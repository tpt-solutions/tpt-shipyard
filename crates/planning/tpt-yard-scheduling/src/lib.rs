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
    /// A release gate names an activity that is not in the schedule.
    UnknownGateActivity(ActivityId),
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleError::Graph(e) => write!(f, "graph: {e}"),
            ScheduleError::UnknownGateActivity(a) => {
                write!(f, "release gate names unknown activity {a}")
            }
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
            g.add_activity(a.id, a.name.clone(), a.duration_hours, &a.dependencies)?;
        }
        g.validate()?;
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

    /// CPM with release gates (review 7H Monte Carlo leftover: the
    /// delivery-slippage coupling). `releases` map an activity id to the
    /// earliest hour its material (or weather window, or anything
    /// upstream) allows it to start; the forward pass takes
    /// `ES_i = max(release_i, latest predecessor finish)`.
    ///
    /// # Errors
    ///
    /// [`ScheduleError::UnknownGateActivity`] when a gate names an
    /// activity outside the schedule, [`ScheduleError::Graph`] on a
    /// malformed network.
    pub fn cpm_with_releases(
        &self,
        releases: &BTreeMap<ActivityId, f64>,
    ) -> Result<BTreeMap<ActivityId, ScheduledActivity>, ScheduleError> {
        if self.activities.is_empty() {
            return Err(ScheduleError::EmptySchedule);
        }
        for &id in releases.keys() {
            if !self.activities.iter().any(|a| a.id == id) {
                return Err(ScheduleError::UnknownGateActivity(id));
            }
        }
        let g = self.graph()?;
        let order = g.topological_order()?;

        // Forward pass with gates.
        let mut es: BTreeMap<ActivityId, f64> = BTreeMap::new();
        let mut ef: BTreeMap<ActivityId, f64> = BTreeMap::new();
        let mut makespan = 0.0_f64;
        for &id in &order {
            let node = g.activity(id).expect("exists");
            let mut start = releases.get(&id).copied().unwrap_or(0.0).max(0.0);
            for dep in g.dependencies_of(id)? {
                start = start.max(ef[&dep]);
            }
            let finish = start + node.duration_hours;
            makespan = makespan.max(finish);
            es.insert(id, start);
            ef.insert(id, finish);
        }

        // Backward pass from the gated makespan.
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
        for id in order {
            let _node = g.activity(id).expect("exists");
            let earliest_start = es[&id];
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

    /// Project makespan under release gates: the largest gated
    /// earliest-finish across all activities.
    ///
    /// # Errors
    ///
    /// [`ScheduleError`] on malformed networks or unknown gate activities.
    pub fn makespan_with_releases(
        &self,
        releases: &BTreeMap<ActivityId, f64>,
    ) -> Result<f64, ScheduleError> {
        let cpm = self.cpm_with_releases(releases)?;
        Ok(cpm
            .values()
            .map(|s| {
                let dur = self
                    .activities
                    .iter()
                    .find(|a| a.id == s.id)
                    .map(|a| a.duration_hours)
                    .unwrap_or(0.0);
                s.earliest_start_h + dur
            })
            .fold(0.0_f64, f64::max))
    }

    /// The critical path: one zero-float chain from a network start to a
    /// network end, in execution order.
    ///
    /// # Errors
    ///
    /// [`ScheduleError`] on malformed networks.
    pub fn critical_path(&self) -> Result<Vec<ActivityId>, ScheduleError> {
        let g = self.graph()?;
        let cpm = self.cpm()?;
        let critical: std::collections::BTreeSet<ActivityId> = cpm
            .values()
            .filter(|s| s.float_h < 1e-9)
            .map(|s| s.id)
            .collect();
        // Walk a chain: start at the critical activity with the earliest
        // start, then repeatedly take its earliest-finishing critical
        // dependent. Ties break by id, so the result is deterministic.
        let start = critical
            .iter()
            .copied()
            .min_by(|&a, &b| {
                cpm[&a]
                    .earliest_start_h
                    .total_cmp(&cpm[&b].earliest_start_h)
                    .then(a.cmp(&b))
            })
            .ok_or(ScheduleError::EmptySchedule)?;
        let mut path = vec![start];
        loop {
            let current = *path.last().expect("non-empty");
            let next = g
                .dependents_of(current)
                .into_iter()
                .filter(|d| critical.contains(d))
                .min_by(|&a, &b| {
                    cpm[&a]
                        .earliest_start_h
                        .total_cmp(&cpm[&b].earliest_start_h)
                        .then(a.cmp(&b))
                });
            match next {
                Some(n) => path.push(n),
                None => break,
            }
        }
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

        // Serial schedule generation. In duration mode activities run in
        // plain topological order at their earliest starts. In levelling
        // mode the *priority* (min float first) decides which eligible
        // activity (all dependencies already placed) is placed next — the
        // placement itself is always precedence-feasible, unlike a
        // priority-sorted flat order, which can put a successor in front of
        // its predecessor.
        let topo = g.topological_order()?;
        let mut start: BTreeMap<ActivityId, f64> = BTreeMap::new();
        // Placed activities for the resource-conflict scan.
        let mut placed: Vec<(ActivityId, f64, f64)> = Vec::new(); // (id, start, end)
        let mut order: Vec<ActivityId> = Vec::with_capacity(topo.len());
        let mut remaining: std::collections::BTreeSet<ActivityId> = topo.iter().copied().collect();
        while !remaining.is_empty() {
            let next = if duration_mode {
                // First in topological order.
                *topo
                    .iter()
                    .find(|id| remaining.contains(id))
                    .expect("non-empty")
            } else {
                // Levelling priority among eligible activities.
                *remaining
                    .iter()
                    .filter(|&&id| {
                        g.activity(id)
                            .expect("exists")
                            .dependencies
                            .iter()
                            .all(|d| start.contains_key(d))
                    })
                    .min_by(|x, y| cpm[*x].float_h.total_cmp(&cpm[*y].float_h).then(x.cmp(y)))
                    .expect("acyclic network always has an eligible activity")
            };
            remaining.remove(&next);
            order.push(next);
            let node = g.activity(next).expect("exists");
            let dep_finish = node
                .dependencies
                .iter()
                .map(|d| {
                    let dur = g.activity(*d).expect("exists").duration_hours;
                    start.get(d).expect("dependencies placed first") + dur
                })
                .fold(0.0f64, f64::max);
            let earliest = dep_finish.max(cpm[&next].earliest_start_h);
            let s = if !duration_mode {
                // Levelling: place at the earliest clash-free time for this
                // activity's resources. Interval scheduling: a clash-free
                // start exists at `earliest` or at the finish of some placed
                // activity — checking those breakpoints is exact, with no
                // arbitrary horizon cap.
                let dur = node.duration_hours;
                let resources = &self
                    .activities
                    .iter()
                    .find(|a| a.id == next)
                    .map(|a| {
                        a.resources
                            .iter()
                            .filter(|r| r.capacity > 0.0)
                            .map(|r| r.kind)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let mut candidates: Vec<f64> = vec![earliest];
                for &(_, _, pe) in &placed {
                    if pe > earliest {
                        candidates.push(pe);
                    }
                }
                candidates.sort_by(f64::total_cmp);
                candidates.dedup();
                candidates
                    .into_iter()
                    .find(|&t| {
                        !placed.iter().any(|(pid, ps, pe)| {
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
                        })
                    })
                    .unwrap_or(earliest) // unreachable: the last breakpoint is always free
            } else {
                earliest
            };
            start.insert(next, s);
            placed.push((next, s, s + node.duration_hours));
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

/// Result of the Monte Carlo schedule-risk pass (review 7H roadmap item).
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleRisk {
    /// Sample count.
    pub samples: u32,
    /// Median (P50) makespan, hours.
    pub p50_makespan_h: f64,
    /// 90th-percentile makespan, hours — the number to promise against.
    pub p90_makespan_h: f64,
    /// Mean makespan, hours.
    pub mean_makespan_h: f64,
    /// Fraction of samples in which each activity sat on the critical
    /// path, descending — where the schedule is actually fragile.
    pub criticality_frequency: Vec<(ActivityId, f64)>,
}

/// Deterministic xorshift64* RNG (reproducible risk runs).
struct XorShift(u64);

impl XorShift {
    fn next_f64(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Triangular sample on [min, mode, max] by inverse CDF.
    fn triangular(&mut self, min: f64, mode: f64, max: f64) -> f64 {
        let r = self.next_f64();
        let f = (mode - min) / (max - min).max(1e-12);
        if r < f {
            min + (r * (max - min) * (mode - min)).sqrt()
        } else {
            max - ((1.0 - r) * (max - min) * (max - mode)).sqrt()
        }
    }
}

impl ShipyardScheduler {
    /// Monte Carlo schedule risk: samples each activity's duration from a
    /// triangular distribution — `(1-u)·d … d … (1+u)·d` for the
    /// uncertainty fraction `u` — runs the CPM forward pass per sample,
    /// and reports the makespan percentiles plus how often each activity
    /// was critical.
    ///
    /// The run is deterministic for a given `seed` (same numbers every
    /// time; review requirement: reproducible planning outputs).
    ///
    /// # Errors
    ///
    /// [`ScheduleError`] on malformed networks or an empty schedule.
    pub fn monte_carlo_risk(
        &self,
        uncertainty_frac: f64,
        n_samples: u32,
        seed: u64,
    ) -> Result<ScheduleRisk, ScheduleError> {
        if self.activities.is_empty() {
            return Err(ScheduleError::EmptySchedule);
        }
        let u = uncertainty_frac.clamp(0.0, 10.0);
        let n = n_samples.max(1);
        let mut rng = XorShift(if seed == 0 { 0x853c49e6748fea9b } else { seed });

        let mut makespans = Vec::with_capacity(n as usize);
        let mut critical_counts: BTreeMap<ActivityId, u32> =
            self.activities.iter().map(|a| (a.id, 0)).collect();

        for _ in 0..n {
            let sampled: Vec<AssemblyActivity> = self
                .activities
                .iter()
                .map(|a| {
                    let mut copy = a.clone();
                    copy.duration_hours = rng.triangular(
                        a.duration_hours * (1.0 - u),
                        a.duration_hours,
                        a.duration_hours * (1.0 + u),
                    );
                    copy
                })
                .collect();
            let sample_net = ShipyardScheduler::new(sampled);
            let cpm = sample_net.cpm()?;
            let finish = sample_net.graph()?.earliest_finish_times()?;
            let makespan = finish.values().copied().fold(0.0, f64::max);
            makespans.push(makespan);
            for s in cpm.values() {
                if s.float_h < 1e-9 {
                    if let Some(c) = critical_counts.get_mut(&s.id) {
                        *c += 1;
                    }
                }
            }
        }

        makespans.sort_by(f64::total_cmp);
        let p = |q: f64| -> f64 {
            let idx = ((q * (makespans.len() as f64 - 1.0)).round()) as usize;
            makespans[idx.min(makespans.len() - 1)]
        };
        let mean = makespans.iter().sum::<f64>() / makespans.len() as f64;
        let mut criticality: Vec<(ActivityId, f64)> = critical_counts
            .into_iter()
            .map(|(id, c)| (id, c as f64 / n as f64))
            .collect();
        criticality.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));

        Ok(ScheduleRisk {
            samples: n,
            p50_makespan_h: p(0.50),
            p90_makespan_h: p(0.90),
            mean_makespan_h: mean,
            criticality_frequency: criticality,
        })
    }
}

/// A delivery gate for the risk simulation: the material (or anything
/// upstream) for `activity` is expected at `expected_available_h` from
/// project start, with lead-time slippage sampled triangularly at
/// `+/- slippage_frac` around the expectation (review 7H Monte Carlo
/// leftover: the delivery-slippage coupling).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeliveryGate {
    /// The gated activity: it cannot start before the material arrives.
    pub activity: ActivityId,
    /// Expected availability of the delivery, hours from project start.
    pub expected_available_h: f64,
    /// Lead-time slippage fraction (0.25 = +/- 25 % triangular).
    pub slippage_frac: f64,
}

impl ShipyardScheduler {
    /// Monte Carlo schedule risk with delivery gates: as
    /// [`Self::monte_carlo_risk`], plus each gate's availability is
    /// sampled per run (triangular around the expectation, scaled by
    /// `slippage_frac`) and fed into the release-gated CPM. Seed
    /// deterministic.
    ///
    /// # Errors
    ///
    /// [`ScheduleError`] on malformed networks, empty schedules, unknown
    /// gate activities, or non-finite gate inputs.
    pub fn monte_carlo_risk_with_gates(
        &self,
        gates: &[DeliveryGate],
        uncertainty_frac: f64,
        n_samples: u32,
        seed: u64,
    ) -> Result<ScheduleRisk, ScheduleError> {
        if self.activities.is_empty() {
            return Err(ScheduleError::EmptySchedule);
        }
        for g in gates {
            if !self.activities.iter().any(|a| a.id == g.activity) {
                return Err(ScheduleError::UnknownGateActivity(g.activity));
            }
            if !(g.expected_available_h.is_finite() && g.expected_available_h >= 0.0)
                || !(g.slippage_frac.is_finite() && g.slippage_frac >= 0.0)
            {
                return Err(ScheduleError::UnknownGateActivity(g.activity));
            }
        }
        let u = uncertainty_frac.clamp(0.0, 10.0);
        let n = n_samples.max(1);
        let mut rng = XorShift(if seed == 0 { 0x853c49e6748fea9b } else { seed });

        let mut makespans = Vec::with_capacity(n as usize);
        let mut critical_counts: BTreeMap<ActivityId, u32> =
            self.activities.iter().map(|a| (a.id, 0)).collect();

        for _ in 0..n {
            let sampled: Vec<AssemblyActivity> = self
                .activities
                .iter()
                .map(|a| {
                    let mut copy = a.clone();
                    copy.duration_hours = rng.triangular(
                        a.duration_hours * (1.0 - u),
                        a.duration_hours,
                        a.duration_hours * (1.0 + u),
                    );
                    copy
                })
                .collect();
            let sample_net = ShipyardScheduler::new(sampled);
            let mut releases: BTreeMap<ActivityId, f64> = BTreeMap::new();
            for g in gates {
                let avail = rng.triangular(
                    g.expected_available_h * (1.0 - g.slippage_frac),
                    g.expected_available_h,
                    g.expected_available_h * (1.0 + g.slippage_frac),
                );
                releases.insert(g.activity, avail);
            }
            let cpm = sample_net.cpm_with_releases(&releases)?;
            let sample_makespan = sample_net.makespan_with_releases(&releases)?;
            makespans.push(sample_makespan);
            for s in cpm.values() {
                if s.float_h < 1e-9 {
                    if let Some(c) = critical_counts.get_mut(&s.id) {
                        *c += 1;
                    }
                }
            }
        }

        makespans.sort_by(f64::total_cmp);
        let p = |q: f64| -> f64 {
            let idx = ((q * (makespans.len() as f64 - 1.0)).round()) as usize;
            makespans[idx.min(makespans.len() - 1)]
        };
        let mean = makespans.iter().sum::<f64>() / makespans.len() as f64;
        let mut criticality: Vec<(ActivityId, f64)> = critical_counts
            .into_iter()
            .map(|(id, c)| (id, c as f64 / n as f64))
            .collect();
        criticality.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));

        Ok(ScheduleRisk {
            samples: n,
            p50_makespan_h: p(0.50),
            p90_makespan_h: p(0.90),
            mean_makespan_h: mean,
            criticality_frequency: criticality,
        })
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
        let s = ShipyardScheduler::new(acts.clone());
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

    /// Verification (review 7H): zero uncertainty reproduces the
    /// deterministic makespan exactly; uncertainty spreads the percentiles
    /// in the right order; the run is seed-deterministic; criticality
    /// frequencies are sane.
    /// Review 7H Monte Carlo leftover: delivery gates. A gate later than
    /// the dependency finish delays the activity and makes it critical;
    /// a gate earlier than the network allows changes nothing.
    #[test]
    fn release_gates_delay_and_gate_criticality() {
        let net = ShipyardScheduler::new(vec![AssemblyActivity::new(
            ActivityId(1),
            "A",
            ActivityType::CutSteel,
            8.0,
        )]);
        // No gate: start at 0, makespan 8.
        assert_eq!(net.makespan_with_releases(&BTreeMap::new()).unwrap(), 8.0);

        // Gate at 100 h: the activity cannot start before 100.
        let mut gates = BTreeMap::new();
        gates.insert(ActivityId(1), 100.0);
        let cpm = net.cpm_with_releases(&gates).unwrap();
        assert_eq!(cpm[&ActivityId(1)].earliest_start_h, 100.0);
        assert_eq!(
            cpm[&ActivityId(1)].float_h,
            0.0,
            "gated-in activity is critical"
        );
        assert_eq!(net.makespan_with_releases(&gates).unwrap(), 108.0);

        // A 5 h gate is a hard availability constraint: the activity
        // starts at 5 h (not 0) and the makespan grows to 13 h — the
        // gated activity becomes critical.
        let mut early = BTreeMap::new();
        early.insert(ActivityId(1), 5.0);
        assert_eq!(net.makespan_with_releases(&early).unwrap(), 13.0);
        let cpm = net.cpm_with_releases(&early).unwrap();
        assert_eq!(cpm[&ActivityId(1)].earliest_start_h, 5.0);
        assert_eq!(cpm[&ActivityId(1)].float_h, 0.0);

        // A zero-hour gate is the no-op.
        let mut zero = BTreeMap::new();
        zero.insert(ActivityId(1), 0.0);
        assert_eq!(net.makespan_with_releases(&zero).unwrap(), 8.0);

        // A negative gate clamps to 0.
        let mut neg = BTreeMap::new();
        neg.insert(ActivityId(1), -50.0);
        assert_eq!(net.makespan_with_releases(&neg).unwrap(), 8.0);

        // Two-activity chain: the gate propagates through the dependency.
        let chain = ShipyardScheduler::new(vec![
            AssemblyActivity::new(ActivityId(1), "steel", ActivityType::CutSteel, 10.0),
            AssemblyActivity::new(ActivityId(2), "erect", ActivityType::JoinBlock, 5.0)
                .with_dependencies(&[ActivityId(1)]),
        ]);
        let mut gate2 = BTreeMap::new();
        gate2.insert(ActivityId(2), 40.0);
        assert_eq!(chain.makespan_with_releases(&gate2).unwrap(), 45.0);
        // The gated activity is critical, the predecessor gains float.
        let cpm = chain.cpm_with_releases(&gate2).unwrap();
        assert_eq!(cpm[&ActivityId(2)].float_h, 0.0);
        assert!(cpm[&ActivityId(1)].float_h > 0.0);

        // Unknown gate activity is an error.
        let mut bad = BTreeMap::new();
        bad.insert(ActivityId(99), 1.0);
        assert_eq!(
            net.cpm_with_releases(&bad),
            Err(ScheduleError::UnknownGateActivity(ActivityId(99)))
        );
    }

    /// Review 7H Monte Carlo leftover: sampled delivery slippage. Zero
    /// slippage reduces to the deterministic gated CPM; with slippage the
    /// P90 sits at or above the P50 and the deterministic value sits at or
    /// below the P50 (the triangular mode).
    #[test]
    fn gated_monte_carlo_couples_delivery_slippage() {
        let net = ShipyardScheduler::new(vec![AssemblyActivity::new(
            ActivityId(1),
            "erect",
            ActivityType::JoinBlock,
            20.0,
        )]);
        let gates = vec![DeliveryGate {
            activity: ActivityId(1),
            expected_available_h: 100.0,
            slippage_frac: 0.25,
        }];

        // Zero duration uncertainty keeps the durations exact; the
        // delivery still slips +/- 25 %: the deterministic 120 h is the
        // triangular MODE of the sample distribution.
        let risk = net
            .monte_carlo_risk_with_gates(&gates, 0.0, 4_000, 11)
            .unwrap();
        assert!(risk.p50_makespan_h <= 120.0 + 1e-9);
        assert!(risk.p90_makespan_h >= 120.0 - 1e-9);
        assert!(risk.p90_makespan_h >= risk.p50_makespan_h);
        // With real slippage the samples span the 95-125 h gate window
        // plus the 20 h duration: P90 well above the mode.
        assert!(risk.p90_makespan_h >= 125.0, "p90 {}", risk.p90_makespan_h);
        assert!(
            risk.p50_makespan_h >= 115.0,
            "the mode region must dominate the median: {}",
            risk.p50_makespan_h
        );

        // Seed determinism.
        let a = net
            .monte_carlo_risk_with_gates(&gates, 0.1, 200, 5)
            .unwrap();
        let b = net
            .monte_carlo_risk_with_gates(&gates, 0.1, 200, 5)
            .unwrap();
        assert_eq!(a, b);

        // Unknown gate activity is refused.
        let bad = vec![DeliveryGate {
            activity: ActivityId(77),
            expected_available_h: 1.0,
            slippage_frac: 0.0,
        }];
        assert_eq!(
            net.monte_carlo_risk_with_gates(&bad, 0.1, 10, 1),
            Err(ScheduleError::UnknownGateActivity(ActivityId(77)))
        );
    }

    /// Verification (review 7H): zero uncertainty reproduces the
    /// deterministic makespan exactly; uncertainty spreads the percentiles
    /// in the right order; the run is seed-deterministic; criticality
    /// frequencies are sane.
    #[test]
    fn monte_carlo_risk_is_sound() {
        let s = network(); // makespan 34 h (A -> B -> D)
        let zero = s.monte_carlo_risk(0.0, 500, 42).unwrap();
        assert!((zero.p50_makespan_h - 34.0).abs() < 1e-9);
        assert!((zero.p90_makespan_h - 34.0).abs() < 1e-9);
        assert!((zero.mean_makespan_h - 34.0).abs() < 1e-9);

        let risky = s.monte_carlo_risk(0.3, 2_000, 42).unwrap();
        assert!(risky.p90_makespan_h > risky.p50_makespan_h);
        assert!(risky.p50_makespan_h > 0.0);
        // P90 must stay inside the triangular envelope: 34 * 1.3 at the
        // very worst (all-critical chain at max simultaneously).
        assert!(
            risky.p90_makespan_h <= 34.0 * 1.3 + 1e-6,
            "{}",
            risky.p90_makespan_h
        );
        // Frequencies: between 0 and 1, and the truly-critical B dominates.
        for (id, f) in &risky.criticality_frequency {
            assert!((0.0..=1.0).contains(f), "{id:?} -> {f}");
        }
        let freq_of = |id: u64| {
            risky
                .criticality_frequency
                .iter()
                .find(|(a, _)| a.0 == id)
                .map(|(_, f)| *f)
                .unwrap()
        };
        assert!(freq_of(2) > freq_of(3), "B (critical) more often than C");
        assert!(freq_of(1) > 0.5, "A is on nearly every critical path");

        // Seed determinism.
        let again = s.monte_carlo_risk(0.3, 2_000, 42).unwrap();
        assert_eq!(again, risky);
        // A different seed moves the percentiles (with overwhelming
        // probability for 2000 samples).
        let other = s.monte_carlo_risk(0.3, 2_000, 7).unwrap();
        assert!(
            other.p50_makespan_h != risky.p50_makespan_h
                || other.mean_makespan_h != risky.mean_makespan_h
        );
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
        // Regression (review 7A/A6): a *real* cycle — both activities exist,
        // each depends on the other.
        let s = ShipyardScheduler::new(vec![
            AssemblyActivity::new(ActivityId(1), "a", ActivityType::CutSteel, 1.0)
                .with_dependencies(&[ActivityId(2)]),
            AssemblyActivity::new(ActivityId(2), "b", ActivityType::CutSteel, 1.0)
                .with_dependencies(&[ActivityId(1)]),
        ]);
        assert!(matches!(s.critical_path(), Err(ScheduleError::Graph(_))));
        assert!(matches!(
            s.optimize_sequence(ScheduleObjective::MinimizeCraneUsage),
            Err(ScheduleError::Graph(_))
        ));
        assert!(matches!(
            s.resource_leveling(),
            Err(ScheduleError::Graph(_))
        ));
    }

    #[test]
    fn unknown_dependency_rejected() {
        let s = ShipyardScheduler::new(vec![
            AssemblyActivity::new(ActivityId(1), "a", ActivityType::CutSteel, 1.0),
            AssemblyActivity::new(ActivityId(2), "b", ActivityType::CutSteel, 1.0)
                .with_dependencies(&[ActivityId(9)]),
        ]);
        assert!(matches!(s.critical_path(), Err(ScheduleError::Graph(_))));
    }

    /// Regression (review 7A/A6): in levelling mode a successor with less
    /// float than its predecessor used to be *placed* first — its start came
    /// out before the predecessor even ran. Every returned order must be a
    /// topological order of the network regardless of objective.
    #[test]
    fn leveled_schedule_respects_precedence() {
        // Long low-priority predecessor chain: A(20h) -> B(1h); B has far
        // less float than A once the independent 40 h activity C runs, so
        // min-float-first priority puts B before A unless precedence
        // constrains the placement loop itself.
        let acts = vec![
            AssemblyActivity::new(ActivityId(1), "A", ActivityType::CutSteel, 20.0),
            AssemblyActivity::new(ActivityId(2), "B", ActivityType::JoinBlock, 1.0)
                .with_dependencies(&[ActivityId(1)]),
            AssemblyActivity::new(ActivityId(3), "C", ActivityType::Paint, 40.0),
        ];
        let s = ShipyardScheduler::new(acts.clone());
        let objectives = [
            ScheduleObjective::MinimizeDuration,
            ScheduleObjective::MinimizeCraneUsage,
            ScheduleObjective::MinimizeCost,
            ScheduleObjective::MinimizeDrydockTime,
            ScheduleObjective::MaximizeParallelism,
        ];
        for objective in objectives {
            let r = s.optimize_sequence(objective).unwrap();
            assert_topological(&acts, &r.order, objective);
        }
        // The reference network too: D depends on B and C.
        let s = network();
        for objective in objectives {
            let r = s.optimize_sequence(objective).unwrap();
            assert_topological(&s.activities, &r.order, objective);
        }
    }

    /// Asserts `order` contains every activity exactly once with every
    /// dependency placed earlier.
    fn assert_topological(
        acts: &[AssemblyActivity],
        order: &[ActivityId],
        objective: ScheduleObjective,
    ) {
        assert_eq!(
            order.len(),
            acts.len(),
            "{objective:?}: every activity placed"
        );
        let mut seen = std::collections::BTreeSet::new();
        for id in order {
            let act = acts.iter().find(|a| a.id == *id).unwrap();
            for d in &act.dependencies {
                assert!(
                    seen.contains(d),
                    "{objective:?}: activity {id:?} placed before its dependency {d:?}"
                );
            }
            assert!(seen.insert(*id), "{objective:?}: duplicate placement");
        }
    }
}
