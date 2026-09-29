# tpt-yard-launch

> Launch calculations: slipway, shiplift, drydock flooding and side launch.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-launch.svg)](https://crates.io/crates/tpt-yard-launch)
[![Docs.rs](https://docs.rs/tpt-yard-launch/badge.svg)](https://docs.rs/tpt-yard-launch)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

How does the ship get into the water? For a slipway end launch, `tpt-yard-launch` runs the classical statics chain: friction-limited sliding down the ways (energy balance), way pressures from full contact through shrinkage to the way-end cribbing, the tip-up race between float-off travel and the CoG crossing the way end, and water-entry slamming pressure.

Dock float-out delegates to `tpt-yard-drydock`'s flooding sequence, and post-launch stability is evaluated from the as-built weight model — the vessel in its partial outfitting state, not its design state.

## Features

- `slipway_launch` — sliding velocity, way-pressure progression, tip-up risk, entry angle, slamming pressure
- End-poppet bearing check against the 0.5 MPa class screening limit with explicit `end_bearing_m` cribbing length
- Sticky-ways detection (grease friction >= tan slope): the launch will not run
- `launch_stability` — GM at the launch condition from the weight model
- `drydock_flooding` — float-out sequence via the docking crate

## Installation

```toml
[dependencies]
tpt-yard-launch = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-launch = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{MassProperties, Vector3};
use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};

let analysis = LaunchAnalysis {
    launch_method: LaunchMethod::Slipway { slope_deg: 3.0, ways: 2 },
    vessel_weight: MassProperties { mass_kg: 4_000_000.0, cog: Vector3::new(70.0, 0.0, 6.0) },
    way_length_m: 120.0, way_width_m: 2.0,
    friction_coefficient: 0.02,
    poppet_to_cog_m: 70.0, end_bearing_m: 20.0,
    immersion_length_m: 90.0, block_coefficient: 0.8, breadth_m: 20.0,
    site: SiteConditions { max_sea_state: 3 },
};
let result = analysis.slipway_launch();
assert!(!result.tip_up_risk);
assert!(result.safe);
```

## How it works

- Sliding: `v^2 = 2 g s (sin theta - mu cos theta)` over the way travel.
- Tip-up: the CoG crossing the way end must come *after* float-off (`draft / sin theta` travel); otherwise the stern pivots on the poppet.
- Slamming: `p = 1/2 rho v^2 C_imp` with a von Karman-type screening coefficient (RFC 0002).

## Verification

- Golden: `slipway-launch-stability.json` — 8.73 m/s entry, 0.163 -> 0.490 MPa pressures, no tip-up
- Tip-up detection for a known-bad CoG position; sticky ways; undersized cribbing
- Capsize detection from the weight model with KG above the metacentre

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `ship-launch` `slipway` `launching` `stability` `shipyard` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-launch` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
