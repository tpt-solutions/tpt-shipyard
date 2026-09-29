# tpt-yard-assembly

> Shared assembly-activity primitives: dependency graphs and activity status tracking.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-assembly.svg)](https://crates.io/crates/tpt-yard-assembly)
[![Docs.rs](https://docs.rs/tpt-yard-assembly/badge.svg)](https://docs.rs/tpt-yard-assembly)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Every construction plan is a directed acyclic graph of activities: cut steel before welding, weld before erection. `tpt-yard-assembly` is that graph — a small, deterministic, dependency-free DAG with the algorithms planners actually ask for: topological ordering, cycle detection, readiness checks, and a critical-path-method forward pass.

It sits at the very bottom of the `tpt-yard-*` dependency stack, so the digital twin, the scheduler, and the logistics planner all share one well-tested implementation instead of three divergent ones.

## Features

- `ActivityGraph` — a DAG of activities (id, name, duration, dependencies, status)
- Kahn topological sort with deterministic (BTreeSet-ordered) output
- Cycle detection via DFS colouring, returning the offending closed path
- Readiness queries (`is_ready`) against live activity statuses
- CPM forward pass: earliest finish times and project makespan
- Status lifecycle: `Pending -> InProgress -> Completed`, plus `Blocked` and `Cancelled`

## Installation

```toml
[dependencies]
tpt-yard-assembly = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-assembly = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_assembly::{ActivityGraph, ActivityId};

let mut g = ActivityGraph::new();
g.add_activity(ActivityId(1), "Cut steel", 8.0, &[])?;
g.add_activity(ActivityId(2), "Form frames", 12.0, &[])?;
g.add_activity(ActivityId(3), "Weld panel", 16.0, &[ActivityId(1), ActivityId(2)])?;
g.add_activity(ActivityId(4), "Outfit panel", 10.0, &[ActivityId(3)])?;

assert_eq!(g.makespan_hours()?, 38.0); // max(8, 12) + 16 + 10
```

## How it works

- Dependencies must reference already-existing activities, so a well-formed graph is guaranteed while it is being built.
- All iteration is id-ordered: identical graphs produce identical orderings on every platform.
- The forward pass assumes every activity starts as early as its dependencies allow; backward passes and float live in `tpt-yard-scheduling`.

## Verification

- Topological order across a diamond dependency network
- Cycle detection including a manually closed loop
- CPM forward pass against hand-computed finish times

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `dependency-graph` `dag` `topological-sort` `cpm` `scheduling` |
| Categories | `algorithms` `data-structures` `science` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-assembly` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
