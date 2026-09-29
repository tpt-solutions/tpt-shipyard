# tpt-yard-logistics

> Material and resource logistics: delivery, staging and transport scheduling.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-logistics.svg)](https://crates.io/crates/tpt-yard-logistics)
[![Docs.rs](https://docs.rs/tpt-yard-logistics/badge.svg)](https://docs.rs/tpt-yard-logistics)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Yard logistics is a pull system: every steel delivery exists because an assembly activity needs it on a computable date. `tpt-yard-logistics` computes order-by and arrive-by dates from lead times and the CPM schedule, enforces a staging dwell limit, and refuses plans that would  bury the laydown area.

## Features

- `MaterialFlow` — the manifest: items with quantity, lead time, staging footprint, and the consuming activity
- `schedule_deliveries` — order-by, arrive-by, need date and dwell per item
- Staging occupancy sweep: peak concurrent footprint checked against the available laydown area
- Negative order dates (order before project start) fall out naturally

## Installation

```toml
[dependencies]
tpt-yard-logistics = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-logistics = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_assembly::ActivityId;
use tpt_yard_core::AssemblyActivity;
use tpt_yard_logistics::{MaterialFlow, MaterialItem};

let mut flow = MaterialFlow::new();
flow.add_item(MaterialItem {
    id: 1,
    name: "Block 212 steel".into(),
    quantity_t: 142.0,
    needed_for: ActivityId(1),
    lead_time_days: 30.0,
    footprint_m2: 200.0,
});
let deliveries = flow.schedule_deliveries(&activities, 5.0, 1_000.0)?;
```

## How it works

- Need dates come from the CPM earliest start (8-hour days); orders go `lead_time + buffer` days ahead.
- Staging occupancy is a +footprint/-footprint event sweep between arrival and consumption — arrivals before need are the dwell.
- Overflow is a typed error naming the required and available areas, not a warning.

## Verification

- Order/arrive/need dates match the hand-computed schedule for a two-activity chain
- Staging overflow detected and cleared by enlarging the laydown area
- Unknown-activity references rejected

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `logistics` `material-flow` `staging` `supply` `delivery` |
| Categories | `science` `simulation` `algorithms` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-logistics` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
