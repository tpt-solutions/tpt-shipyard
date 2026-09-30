//! Shared assembly-activity primitives: dependency graphs and activity status
//! tracking.
//!
//! This crate is the bottom of the `tpt-yard-*` dependency stack. It defines
//! the activity network used by [`tpt_yard_core::BuildPhase`] and
//! [`tpt_yard_digital_twin::DigitalTwin`]: activities with dependencies, their
//! statuses, and graph algorithms (topological ordering, cycle detection,
//! readiness checks, and a critical-path-method forward pass).
//!
//! # Example
//!
//! ```
//! use tpt_yard_assembly::{ActivityGraph, ActivityId, ActivityStatus};
//!
//! let mut g = ActivityGraph::new();
//! g.add_activity(ActivityId(1), "Cut steel", 8.0, &[]);
//! g.add_activity(ActivityId(2), "Weld panel", 16.0, &[ActivityId(1)]);
//! g.add_activity(ActivityId(3), "Erect panel", 4.0, &[ActivityId(2)]);
//!
//! let order = g.topological_order().unwrap();
//! assert_eq!(order, vec![ActivityId(1), ActivityId(2), ActivityId(3)]);
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Stable identifier for an assembly activity within a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActivityId(pub u64);

impl fmt::Display for ActivityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "A{}", self.0)
    }
}

/// Lifecycle status of a single assembly activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityStatus {
    /// Not yet eligible: at least one dependency is incomplete.
    Pending,
    /// Deliberately held (materials missing, space conflict, weather hold).
    Blocked,
    /// Currently being executed.
    InProgress,
    /// Finished and accepted.
    Completed,
    /// Removed from the plan without being executed.
    Cancelled,
}

impl ActivityStatus {
    /// True if the activity has finished (successfully or by cancellation).
    pub fn is_terminal(self) -> bool {
        matches!(self, ActivityStatus::Completed | ActivityStatus::Cancelled)
    }
}

impl fmt::Display for ActivityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ActivityStatus::Pending => "pending",
            ActivityStatus::Blocked => "blocked",
            ActivityStatus::InProgress => "in progress",
            ActivityStatus::Completed => "completed",
            ActivityStatus::Cancelled => "cancelled",
        };
        f.write_str(s)
    }
}

/// One node of the assembly network.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityNode {
    /// Activity identifier.
    pub id: ActivityId,
    /// Human-readable name (e.g. "Erect block 212 in drydock").
    pub name: String,
    /// Planned duration in hours.
    pub duration_hours: f64,
    /// Activities that must be [`ActivityStatus::Completed`] before this one
    /// may start.
    pub dependencies: Vec<ActivityId>,
    /// Current status.
    pub status: ActivityStatus,
}

impl ActivityNode {
    /// Creates a node in the [`ActivityStatus::Pending`] state. Duplicate
    /// dependency entries are collapsed (the same edge twice is still one
    /// edge — duplicates must not create phantom indegree).
    pub fn new(
        id: ActivityId,
        name: impl Into<String>,
        duration_hours: f64,
        dependencies: &[ActivityId],
    ) -> Self {
        let mut seen = BTreeSet::new();
        let deps = dependencies
            .iter()
            .copied()
            .filter(|d| seen.insert(*d))
            .collect();
        Self {
            id,
            name: name.into(),
            duration_hours,
            dependencies: deps,
            status: ActivityStatus::Pending,
        }
    }
}

/// Errors produced by graph operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// The referenced activity does not exist.
    UnknownActivity(ActivityId),
    /// The dependency list references an activity that does not exist.
    UnknownDependency {
        /// The activity being added.
        activity: ActivityId,
        /// The missing dependency.
        dependency: ActivityId,
    },
    /// The graph contains at least one dependency cycle; the offending
    /// activities are listed.
    CycleDetected(Vec<ActivityId>),
    /// An activity with this id already exists.
    DuplicateActivity(ActivityId),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::UnknownActivity(a) => write!(f, "unknown activity {a}"),
            GraphError::UnknownDependency {
                activity,
                dependency,
            } => {
                write!(
                    f,
                    "activity {activity} depends on unknown activity {dependency}"
                )
            }
            GraphError::CycleDetected(cycle) => {
                let list: Vec<String> = cycle.iter().map(|a| a.to_string()).collect();
                write!(f, "dependency cycle detected: {}", list.join(" -> "))
            }
            GraphError::DuplicateActivity(a) => write!(f, "duplicate activity {a}"),
        }
    }
}

impl std::error::Error for GraphError {}

/// A directed acyclic graph of assembly activities.
///
/// All algorithms are deterministic (iteration order is by [`ActivityId`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActivityGraph {
    nodes: BTreeMap<ActivityId, ActivityNode>,
}

impl ActivityGraph {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of activities in the graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True if the graph has no activities.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Adds an activity with its dependencies.
    ///
    /// Activities may be added in any order — a dependency on an
    /// activity that is added later is fine. Use [`ActivityGraph::validate`]
    /// (or run any algorithm such as [`ActivityGraph::topological_order`],
    /// which validates first) to check that every dependency resolves.
    ///
    /// # Errors
    ///
    /// [`GraphError::DuplicateActivity`] if `id` is already in the graph.
    pub fn add_activity(
        &mut self,
        id: ActivityId,
        name: impl Into<String>,
        duration_hours: f64,
        dependencies: &[ActivityId],
    ) -> Result<(), GraphError> {
        if self.nodes.contains_key(&id) {
            return Err(GraphError::DuplicateActivity(id));
        }
        self.nodes.insert(
            id,
            ActivityNode::new(id, name, duration_hours, dependencies),
        );
        Ok(())
    }

    /// Checks that every dependency of every activity resolves to an
    /// existing activity.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownDependency`] on the first dangling dependency.
    pub fn validate(&self) -> Result<(), GraphError> {
        for n in self.nodes.values() {
            for d in &n.dependencies {
                if !self.nodes.contains_key(d) {
                    return Err(GraphError::UnknownDependency {
                        activity: n.id,
                        dependency: *d,
                    });
                }
            }
        }
        Ok(())
    }

    /// True if the activity exists in the graph.
    pub fn contains(&self, id: ActivityId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Read access to a node.
    pub fn activity(&self, id: ActivityId) -> Option<&ActivityNode> {
        self.nodes.get(&id)
    }

    /// Mutable access to a node (to change status, duration, or dependencies).
    pub fn activity_mut(&mut self, id: ActivityId) -> Option<&mut ActivityNode> {
        self.nodes.get_mut(&id)
    }

    /// All nodes ordered by id.
    pub fn activities(&self) -> impl Iterator<Item = &ActivityNode> {
        self.nodes.values()
    }

    /// Direct dependencies of an activity.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownActivity`] if the node does not exist.
    pub fn dependencies_of(&self, id: ActivityId) -> Result<Vec<ActivityId>, GraphError> {
        let node = self.nodes.get(&id).ok_or(GraphError::UnknownActivity(id))?;
        Ok(node.dependencies.clone())
    }

    /// Activities that directly depend on `id` (the reverse edges), ordered by
    /// id.
    pub fn dependents_of(&self, id: ActivityId) -> Vec<ActivityId> {
        self.nodes
            .values()
            .filter(|n| n.dependencies.contains(&id))
            .map(|n| n.id)
            .collect()
    }

    /// True if every dependency of `id` is [`ActivityStatus::Completed`].
    /// An activity with no dependencies is always ready.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownActivity`] if the node does not exist.
    pub fn is_ready(&self, id: ActivityId) -> Result<bool, GraphError> {
        let node = self.nodes.get(&id).ok_or(GraphError::UnknownActivity(id))?;
        Ok(node.dependencies.iter().all(|d| {
            self.nodes
                .get(d)
                .is_some_and(|n| n.status == ActivityStatus::Completed)
        }))
    }

    /// Kahn topological sort of the whole graph.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownDependency`] if any dependency does not resolve,
    /// [`GraphError::CycleDetected`] listing the activities still unsorted
    /// when the queue drains (these participate in, or depend on, a cycle).
    pub fn topological_order(&self) -> Result<Vec<ActivityId>, GraphError> {
        self.validate()?;
        let mut indegree: BTreeMap<ActivityId, usize> = BTreeMap::new();
        for n in self.nodes.values() {
            indegree.entry(n.id).or_insert(0);
            // Count each distinct edge once (duplicate dependency entries
            // must not inflate the indegree — `dependents_of` yields the
            // node once, so the decrement would never balance).
            let mut seen = BTreeSet::new();
            for d in &n.dependencies {
                if seen.insert(*d) {
                    *indegree.entry(n.id).or_insert(0) += 1;
                    indegree.entry(*d).or_insert(0);
                }
            }
        }
        // BTreeSet as the queue keeps the output deterministic.
        let mut queue: BTreeSet<ActivityId> = indegree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(id) = queue.pop_first() {
            order.push(id);
            for dep_id in self.dependents_of(id) {
                let deg = indegree.get_mut(&dep_id).expect("dependent registered");
                *deg -= 1;
                if *deg == 0 {
                    queue.insert(dep_id);
                }
            }
        }
        if order.len() != self.nodes.len() {
            let sorted: BTreeSet<ActivityId> = order.iter().copied().collect();
            let cyclic: Vec<ActivityId> = self
                .nodes
                .keys()
                .filter(|id| !sorted.contains(id))
                .copied()
                .collect();
            return Err(GraphError::CycleDetected(cyclic));
        }
        Ok(order)
    }

    /// Returns one dependency cycle (as a closed path), if the graph is cyclic.
    ///
    /// Iterative depth-first search with node colouring; the returned path
    /// starts and ends at the same activity.
    pub fn find_cycle(&self) -> Option<Vec<ActivityId>> {
        const WHITE: u8 = 0;
        const GREY: u8 = 1;
        const BLACK: u8 = 2;
        let mut colour: BTreeMap<ActivityId, u8> =
            self.nodes.keys().map(|&id| (id, WHITE)).collect();

        for start in self.nodes.keys().copied().collect::<Vec<_>>() {
            if colour[&start] != WHITE {
                continue;
            }
            // Stack of (node, next-dependency-index-to-explore).
            let mut stack: Vec<(ActivityId, usize)> = vec![(start, 0)];
            colour.insert(start, GREY);
            while let Some(top) = stack.last_mut() {
                let deps = self.nodes[&top.0].dependencies.clone();
                if top.1 < deps.len() {
                    let next = deps[top.1];
                    top.1 += 1;
                    match colour[&next] {
                        WHITE => {
                            colour.insert(next, GREY);
                            stack.push((next, 0));
                        }
                        GREY => {
                            // Found a back edge; trim the stack to the cycle.
                            let mut path: Vec<ActivityId> =
                                stack.iter().map(|(id, _)| *id).collect();
                            while path.first() != Some(&next) {
                                path.remove(0);
                            }
                            path.push(next);
                            return Some(path);
                        }
                        _ => {}
                    }
                } else {
                    let node = top.0;
                    colour.insert(node, BLACK);
                    stack.pop();
                }
            }
        }
        None
    }

    /// Forward pass of the critical path method (CPM): `id ->
    /// earliest_finish_hours` assuming every activity starts as early as its
    /// dependencies allow.
    ///
    /// # Errors
    ///
    /// [`GraphError::CycleDetected`] if the graph is cyclic.
    pub fn earliest_finish_times(&self) -> Result<BTreeMap<ActivityId, f64>, GraphError> {
        let order = self.topological_order()?;
        let mut finish = BTreeMap::new();
        for id in order {
            let node = &self.nodes[&id];
            let start = node
                .dependencies
                .iter()
                .filter_map(|d| finish.get(d).copied())
                .fold(0.0, f64::max);
            finish.insert(id, start + node.duration_hours);
        }
        Ok(finish)
    }

    /// Total project duration (longest path length) in hours.
    ///
    /// # Errors
    ///
    /// [`GraphError::CycleDetected`] if the graph is cyclic.
    pub fn makespan_hours(&self) -> Result<f64, GraphError> {
        Ok(self
            .earliest_finish_times()?
            .values()
            .copied()
            .fold(0.0, f64::max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> ActivityGraph {
        let mut g = ActivityGraph::new();
        g.add_activity(ActivityId(1), "Cut steel", 8.0, &[])
            .unwrap();
        g.add_activity(ActivityId(2), "Form frames", 12.0, &[])
            .unwrap();
        g.add_activity(
            ActivityId(3),
            "Weld panel",
            16.0,
            &[ActivityId(1), ActivityId(2)],
        )
        .unwrap();
        g.add_activity(ActivityId(4), "Outfit panel", 10.0, &[ActivityId(3)])
            .unwrap();
        g
    }

    #[test]
    fn topological_order_respects_dependencies() {
        let g = sample_graph();
        let order = g.topological_order().unwrap();
        let pos = |a: ActivityId| order.iter().position(|&x| x == a).unwrap();
        assert!(pos(ActivityId(1)) < pos(ActivityId(3)));
        assert!(pos(ActivityId(2)) < pos(ActivityId(3)));
        assert!(pos(ActivityId(3)) < pos(ActivityId(4)));
    }

    #[test]
    fn cycle_is_detected() {
        let mut g = ActivityGraph::new();
        g.add_activity(ActivityId(1), "a", 1.0, &[ActivityId(2)])
            .unwrap();
        g.add_activity(ActivityId(2), "b", 1.0, &[ActivityId(1)])
            .unwrap();
        assert!(g.find_cycle().is_some());
        assert!(matches!(
            g.topological_order(),
            Err(GraphError::CycleDetected(_))
        ));
    }

    #[test]
    fn unknown_dependency_is_rejected_at_validation() {
        let mut g = ActivityGraph::new();
        // Adding with a not-yet-existing dependency is allowed (out-of-order
        // construction); the dangling edge is caught at validation.
        g.add_activity(ActivityId(1), "a", 1.0, &[ActivityId(9)])
            .unwrap();
        assert_eq!(
            g.validate(),
            Err(GraphError::UnknownDependency {
                activity: ActivityId(1),
                dependency: ActivityId(9)
            })
        );
        assert!(matches!(
            g.topological_order(),
            Err(GraphError::UnknownDependency { .. })
        ));
    }

    /// Regression (review 7A/A11): a valid network built out of order must
    /// work — dependencies may reference activities added later.
    #[test]
    fn out_of_order_construction_works() {
        let mut g = ActivityGraph::new();
        g.add_activity(ActivityId(3), "Erect", 4.0, &[ActivityId(2)])
            .unwrap();
        g.add_activity(ActivityId(2), "Weld", 16.0, &[ActivityId(1)])
            .unwrap();
        g.add_activity(ActivityId(1), "Cut", 8.0, &[]).unwrap();
        g.validate().unwrap();
        assert_eq!(
            g.topological_order().unwrap(),
            vec![ActivityId(1), ActivityId(2), ActivityId(3)]
        );
        assert_eq!(g.makespan_hours().unwrap(), 28.0);
    }

    /// Regression (review 7A/A11): a duplicate dependency edge must not
    /// inflate the indegree and report a false cycle.
    #[test]
    fn duplicate_dependency_is_not_a_cycle() {
        let mut g = ActivityGraph::new();
        g.add_activity(ActivityId(1), "a", 1.0, &[]).unwrap();
        g.add_activity(ActivityId(2), "b", 1.0, &[ActivityId(1), ActivityId(1)])
            .unwrap();
        g.topological_order()
            .expect("duplicate dependency must not create a false cycle");
        let mut with_dup = ActivityGraph::new();
        with_dup
            .add_activity(ActivityId(1), "a", 1.0, &[])
            .unwrap();
        with_dup
            .add_activity(ActivityId(2), "b", 1.0, &[ActivityId(1)])
            .unwrap();
        // Duplicating via mutation is still just one edge.
        with_dup
            .activity_mut(ActivityId(2))
            .unwrap()
            .dependencies
            .push(ActivityId(1));
        with_dup
            .topological_order()
            .expect("no false cycle from a mutated duplicate edge");
    }

    /// Regression (review 7A/A11): a duplicate activity id must be rejected,
    /// not silently overwrite the node (which would orphan its dependents).
    #[test]
    fn duplicate_activity_id_rejected() {
        let mut g = ActivityGraph::new();
        g.add_activity(ActivityId(1), "a", 1.0, &[]).unwrap();
        g.add_activity(ActivityId(2), "b", 1.0, &[ActivityId(1)])
            .unwrap();
        assert_eq!(
            g.add_activity(ActivityId(1), "a again", 2.0, &[]),
            Err(GraphError::DuplicateActivity(ActivityId(1)))
        );
        // The original node is intact.
        assert_eq!(g.activity(ActivityId(1)).unwrap().duration_hours, 1.0);
    }

    /// Regression (review 7A/A11): a dangling dependency introduced through
    /// `activity_mut` must be reported as an error by graph algorithms,
    /// not produce a phantom node that panics later passes.
    #[test]
    fn dangling_dependency_from_mutation_is_an_error() {
        let mut g = ActivityGraph::new();
        g.add_activity(ActivityId(1), "a", 1.0, &[]).unwrap();
        g.add_activity(ActivityId(2), "b", 1.0, &[ActivityId(1)])
            .unwrap();
        g.activity_mut(ActivityId(1))
            .unwrap()
            .dependencies
            .push(ActivityId(99));
        assert!(matches!(
            g.topological_order(),
            Err(GraphError::UnknownDependency { .. })
        ));
        assert!(g.earliest_finish_times().is_err());
    }

    #[test]
    fn readiness_tracks_completed_dependencies() {
        let mut g = sample_graph();
        assert!(!g.is_ready(ActivityId(3)).unwrap());
        g.activity_mut(ActivityId(1)).unwrap().status = ActivityStatus::Completed;
        g.activity_mut(ActivityId(2)).unwrap().status = ActivityStatus::Completed;
        assert!(g.is_ready(ActivityId(3)).unwrap());
        assert!(g.is_ready(ActivityId(1)).unwrap()); // no dependencies
    }

    #[test]
    fn dependents_are_the_reverse_edges() {
        let g = sample_graph();
        assert_eq!(g.dependents_of(ActivityId(3)), vec![ActivityId(4)]);
    }

    #[test]
    fn cpm_forward_pass() {
        let g = sample_graph();
        let finish = g.earliest_finish_times().unwrap();
        assert_eq!(finish[&ActivityId(3)], 28.0); // max(8, 12) + 16
        assert_eq!(finish[&ActivityId(4)], 38.0);
        assert_eq!(g.makespan_hours().unwrap(), 38.0);
    }
}
