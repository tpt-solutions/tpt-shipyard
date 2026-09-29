# tpt-yard-scheduling

> Construction scheduling: critical path, optimisation and resource levelling.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-scheduling.svg)](https://crates.io/crates/tpt-yard-scheduling)
[![Docs.rs](https://docs.rs/tpt-yard-scheduling/badge.svg)](https://docs.rs/tpt-yard-scheduling)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Which activities drive the ship's delivery date, and can the cranes actually keep up? `tpt-yard-scheduling` runs the critical-path method over the assembly network, schedules under explicit objectives, and levels resources with a real serial schedule-generation scheme — the kind that trades makespan for flat peaks and says so.

## Features

- `critical_path` — zero-float activities from a full CPM pass (earliest/latest starts, float per activity)
- `optimize_sequence` — objectives: minimum duration, cost, crane usage, drydock time, maximum parallelism
- `resource_leveling` — serial RCPSP placement: each activity at the earliest clash-free slot for its resources
- Honest levelling: may stretch the makespan to flatten peaks (golden case: 32 h -> 40 h for a crane peak of 2 -> 1)
- Peak concurrent demand per resource kind on a 1-hour grid

## Installation

```toml
[dependencies]
tpt-yard-scheduling = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-scheduling = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{ActivityType, AssemblyActivity};
use tpt_yard_scheduling::{ScheduleObjective, ShipyardScheduler};

let scheduler = ShipyardScheduler::new(vec![
    AssemblyActivity::new(ActivityId(1), "Cut", ActivityType::CutSteel, 8.0),
    AssemblyActivity::new(ActivityId(2), "Weld", ActivityType::WeldBlock, 16.0)
        .with_dependencies(&[ActivityId(1)]),
    AssemblyActivity::new(ActivityId(3), "Erect", ActivityType::JoinBlock, 4.0)
        .with_dependencies(&[ActivityId(2)]),
]);

let path = scheduler.critical_path()?;
let levelled = scheduler.resource_leveling()?;
```

## How it works

- CPM: forward pass for earliest finishes, backward pass for latest starts; float is the difference, zero float is critical.
- Levelling places activities min-float-first at the earliest time their resources conflict with nothing already placed — clamped steps, bounded scan.
- Zero-capacity resource entries do not consume: a named-but-free resource never triggers a clash.

## Verification

- Golden: `critical-path-schedule.json` (34 h network, 4 h float branch)
- Golden: `resource-leveling.json` — crane peak 2 -> 1 at a makespan cost of 32 -> 40 h
- Cycle and empty-schedule error paths typed and tested

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `scheduling` `critical-path` `resource-leveling` `project-planning` `cpm` |
| Categories | `algorithms` `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-scheduling` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
