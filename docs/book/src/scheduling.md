# Scheduling

`tpt-yard-scheduling` answers the planner's three questions on the activity
network.

## Critical path

The CPM pass computes earliest/latest starts and float per activity;
[`critical_path`](tpt_yard_scheduling::ShipyardScheduler::critical_path)
returns the zero-float chain that drives the makespan. Golden reference:
`test-data/golden/planning/critical-path-schedule.json` (34 h network with
a 4 h float branch).

## Objectives

[`optimize_sequence`](tpt_yard_scheduling::ShipyardScheduler::optimize_sequence)
schedules under a [`ScheduleObjective`](tpt_yard_scheduling::ScheduleObjective):
earliest starts for duration/parallelism objectives; a serial
schedule-generation scheme (priority = minimum float first) for the
cost/crane-usage objectives.

## Resource levelling

[`resource_leveling`](tpt_yard_scheduling::ShipyardScheduler::resource_leveling)
places each activity at the earliest time where its resources clash with
nothing already placed — a true serial RCPSP pass. Levelling may legitimately
stretch the makespan to flatten peaks; the golden case trades 32 h -> 40 h
for a crane peak of 2 -> 1 (`resource-leveling.json`).

## Capacity-aware levelling

The legacy levelling treats each shared resource kind as one-at-a-time.
Yards usually have *several* of a resource: 200 t of crane split over two
machines, 40 welders in three crews.
[`resource_leveling_with_limits`](tpt_yard_scheduling::ShipyardScheduler::resource_leveling_with_limits)
takes those yard-wide capacities (in the same units as the activities'
resource demands) and lets activities share a kind while the summed
concurrent demand stays at or under the limit; kinds without a limit are
unconstrained, and an activity that alone exceeds a limit is a typed error
(no feasible schedule exists). The feasibility test covers the whole
placement window — demand that rises inside the activity's own duration
(a later activity starting) defers the placement exactly as an overrun at
the start instant would.

```rust
use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{ActivityType, AssemblyActivity, Resource, ResourceKind};
use tpt_yard_scheduling::ShipyardScheduler;
use std::collections::BTreeMap;

let lift = |id: u64, h: f64| {
    AssemblyActivity::new(ActivityId(id), "lift", ActivityType::JoinBlock, h).with_resources(
        vec![Resource {
            name: "crane".into(),
            kind: ResourceKind::Crane,
            capacity: 100.0,
        }],
    )
};
let s = ShipyardScheduler::new(vec![lift(1, 16.0), lift(2, 8.0)]);
let mut limits = BTreeMap::new();
limits.insert(ResourceKind::Crane, 200.0); // two 100 t cranes
let r = s.resource_leveling_with_limits(&limits).unwrap();
assert_eq!(r.makespan_hours, 16.0); // both lifts run in parallel
```

The CLI exposes it as `schedule project.json --limit crane=200 --limit crew=40`.

