# tpt-yard-drydock

> Drydock flooding and ballast sequencing with stability at every water level.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-drydock.svg)](https://crates.io/crates/tpt-yard-drydock)
[![Docs.rs](https://docs.rs/tpt-yard-drydock/badge.svg)](https://docs.rs/tpt-yard-drydock)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Flooding a building dock is a controlled stability exercise: as the water rises the vessel takes load off the keel blocks, starts to float, and must be upright at *every* intermediate level before the caisson opens.

`Drydock::flooding_sequence` walks the water level bottom-up and reports draft, displaced mass, and metacentric height at each step — flagging any level where the GM drops below the 0.15 m screening minimum, and any vessel that simply cannot float out of the dock.

## Features

- `Drydock::flooding_sequence` — level-by-level state: draft, displacement, GM, hours of flooding
- Aground/afloat transitions with keel-block load sharing reported per level
- Rectangular-block hydrostatics: `GM = KB + BM - KG`, `BM = B^2/(12 d)`
- Fit checks: breadth, length and float-out draft against the dock
- Total flooding time from the dock volume and pump rate

## Installation

```toml
[dependencies]
tpt-yard-drydock = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-drydock = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_drydock::{DockedVessel, Drydock};

let dock = Drydock { length_m: 200.0, width_m: 30.0, depth_m: 10.0 };
let vessel = DockedVessel {
    launch_weight_kg: 6_000_000.0,
    cog_above_keel_m: 5.5,
    length_m: 140.0,
    breadth_m: 22.0,
    block_coefficient: 0.85,
    ballast_tanks: vec![],
};

let seq = dock.flooding_sequence(&vessel, 5_000.0)?; // m3/h
assert!(seq.stable_at_every_level);
```

## How it works

- Displacement: rectangular block, `rho * Cb * L * B * d` with sea water at 1025 kg/m3.
- The float-off draft solves the displacement equation for the launch weight; levels below it are aground.
- Screening-grade hydrostatics (documented in RFC 0002) — fine forms overestimate BM slightly; full hydrostatics belongs to a hull-mesh substrate.

## Verification

- Golden: `drydock-flooding-sequence.json` — 13 levels, 9 aground steps, final GM 7.66 m
- Capsize test: KG above the metacentre yields negative GM
- Too-wide and too-deep vessels rejected with typed errors

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `drydock` `flooding` `stability` `metacentric` `ballast` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-drydock` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
