# Assembly Activities

`tpt-yard-assembly` is the bottom of the stack: the dependency-graph
primitives every planner shares.

## The graph

[`ActivityGraph`](tpt_yard_assembly::ActivityGraph) is a DAG of
[`ActivityNode`](tpt_yard_assembly::ActivityNode)s (id, name, duration,
dependencies, [`ActivityStatus`](tpt_yard_assembly::ActivityStatus)).
Dependencies must reference existing activities, keeping the graph well-formed
as it is built.

## Algorithms

- `topological_order()` — Kahn's algorithm; deterministic (BTreeSet queue).
- `find_cycle()` — DFS colouring; returns the closed path of a cycle.
- `is_ready(id)` — all dependencies `Completed`.
- `earliest_finish_times()` / `makespan_hours()` — CPM forward pass; the
  scheduling crate builds backward passes and levelling on top of these.

## Statuses

`Pending → InProgress → Completed`, with `Blocked` (held) and `Cancelled`
(removed) as terminal-adjacent states. The digital twin mirrors this set in
its `AssemblyState`.

## Example

```rust
use tpt_yard_assembly::{ActivityGraph, ActivityId};

let mut g = ActivityGraph::new();
g.add_activity(ActivityId(1), "Cut steel", 8.0, &[])?;
g.add_activity(ActivityId(2), "Form frames", 12.0, &[])?;
g.add_activity(ActivityId(3), "Weld panel", 16.0, &[ActivityId(1), ActivityId(2)])?;
g.add_activity(ActivityId(4), "Outfit panel", 10.0, &[ActivityId(3)])?;

assert_eq!(g.makespan_hours()?, 38.0); // max(8, 12) + 16 + 10
```

## Verification

Unit tests cover topological ordering across a diamond dependency, cycle
detection (including a manually closed loop), readiness transitions,
reverse-edge queries, and the CPM forward pass against hand-computed values.
