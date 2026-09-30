# tpt-yard-weight

> Weight and centre-of-gravity management for vessels under construction.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-weight.svg)](https://crates.io/crates/tpt-yard-weight)
[![Docs.rs](https://docs.rs/tpt-yard-weight/badge.svg)](https://docs.rs/tpt-yard-weight)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Shipyards live and die by two numbers: how much does it weigh now, and where is the centre of gravity? `tpt-yard-weight` is the single source of truth for both. Every measurable mass is a `WeightItem` with a lifecycle (`Design -> Ordered -> Received -> Installed`, with `Replaced` kept for history), a growth margin, and — crucially — the assembly activity that installs it.

Because items are wired to activities, the model can answer the master-plan question (what will it weigh?) and the as-built question (what is in the ship *right now*?) from the same data, and the digital twin can flip items to installed automatically as erection proceeds.

## Features

- `WeightModel` — all items plus the contractual design weight and design CoG
- Best-estimate vs installed vs design weight, with deviation in kg and %
- Best-estimate and installed-only centres of gravity (weighted means)
- Growth margins per item, aggregated for the weight report
- Group-by reports (by system or by zone), outstanding item counts
- `installed_by: ActivityId` wiring — the only path from predicted to as-built mass

## Installation

```toml
[dependencies]
tpt-yard-weight = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-weight = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{ItemId, Vector3};
use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

let mut model = WeightModel::new(190_000.0, Vector3::new(70.0, 0.0, 7.0));
model.add_item(WeightItem {
    id: ItemId(1),
    name: "Block 211".into(),
    group: "hull".into(),
    weight_kg: 15_000.0,
    cog: Vector3::new(10.0, 0.0, 6.0),
    status: ItemStatus::Installed,
    margin_pct: 2.0,
    installed_by: None,
})
.expect("valid weight item");

assert_eq!(model.installed_weight(), 15_000.0);
assert_eq!(model.weight_deviation(), -175_000.0); // vs the 190 t design
```

## How it works

- `total_weight()` counts every non-replaced item: installed items at as-built mass, the rest at predicted mass (the lightship best estimate).
- `installed_weight()` counts only what is physically aboard — the launch officer's number.
- Replaced items are kept for audit but never counted.

## Verification

- `test_block_weight_sum`: ten blocks sum exactly to the vessel total
- CoG equals the closed-form weighted mean; replaced items never count
- Deviation, margin aggregation and report grouping unit-tested

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `weight` `centre-of-gravity` `mass-properties` `shipbuilding` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-weight` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
