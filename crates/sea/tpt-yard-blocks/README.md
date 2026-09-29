# tpt-yard-blocks

> Shared block-lifting and handling primitives: lift points and sling loads.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-blocks.svg)](https://crates.io/crates/tpt-yard-blocks)
[![Docs.rs](https://docs.rs/tpt-yard-blocks/badge.svg)](https://docs.rs/tpt-yard-blocks)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Every heavy lift in the yard asks the same statics question: given the hook position and the block's centre of gravity, what does each sling leg carry? `tpt-yard-blocks` is the canonical answer, shared by the hull-erection planner, the structural lifting checks, and the block-lifting benchmark.

Two-point picks use the exact lever rule; three or more points use an inverse-distance convention (multi-point statics are indeterminate without sling stiffnesses) — both documented in RFC 0002.

## Features

- `distribute_load_shares` — vertical load shares over lift points, summing to 1
- `sling_loads` — per-leg table: share, angle from horizontal, leg tension (`share / sin theta`)
- `sling_angles_ok` — the 30-degree practice-minimum angle check
- `cog_within_lifts` — the pick-time tipping check
- `LiftPoint` with certified working load limit for utilization math

## Installation

```toml
[dependencies]
tpt-yard-blocks = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-blocks = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_blocks::{cog_within_lifts, sling_loads};
use tpt_yard_core::Vector3;

let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(12.0, 0.0, 0.0)];
let hook = Vector3::new(6.0, 0.0, 8.0);
let loads = sling_loads(800.0 * 9.81, Vector3::new(6.0, 0.0, 0.0), &lifts, hook);

for leg in &loads {
    println!("lift {}: {:.0} kN at {:.0} deg", leg.lift_point,
             leg.leg_tension_kn, leg.angle_from_horizontal_deg);
}
assert!(cog_within_lifts(Vector3::new(6.0, 0.0, 0.0), &lifts));
```

## How it works

- Two points: shares = opposite lever arms of the CoG offset (exact).
- N >= 3 points: inverse-distance weighting of the CoG offset — deterministic and conservative for planning.
- Leg tension divides the vertical share by `sin(angle)`; shallow legs (large tension) are exactly what the angle check exists to catch.

## Verification

- Centred CoG splits evenly; 3-of-12 m offset gives the exact 75/25 lever split
- 45-degree legs: tension exactly `share * sqrt 2`
- Shallow-sling and outside-CoG rejections tested

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `lifting` `sling` `rigging` `statics` `crane` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-blocks` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
