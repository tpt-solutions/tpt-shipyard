# tpt-yard-outfitting

> Systems installation and routing for vessel outfitting.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-outfitting.svg)](https://crates.io/crates/tpt-yard-outfitting)
[![Docs.rs](https://docs.rs/tpt-yard-outfitting/badge.svg)](https://docs.rs/tpt-yard-outfitting)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Outfitting is where ships quietly go wrong: a sea-water line crossing a cable tray, a duct leaving the hull envelope. `tpt-yard-outfitting` plans the routes, detects the clashes (route vs route and route vs hull), and sequences the installation the way yards actually work — large systems first, smaller routing after.

## Features

- `OutfittingPlan` — outfit systems plus their 3D polyline routes
- `collision_detection` — bounding-box screening with clearance, against other routes and the hull envelope
- Typed clashes: `SystemVsSystem` and `SystemVsHull`, each with an approximate location
- `installation_sequence` — size-descending order (machinery before ducting before cable trays)
- Route bounding boxes grown by half the cross-section, so clearance is explicit

## Installation

```toml
[dependencies]
tpt-yard-outfitting = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-outfitting = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{FluidType, Geometry3D, OutfitSystem, Vector3};
use tpt_yard_outfitting::{OutfittingPlan, Route};

let mut plan = OutfittingPlan::new();
plan.add_system(OutfitSystem::Piping { fluid: FluidType::SeaWater, diameter_mm: 200.0 });
plan.add_system(OutfitSystem::Electrical { voltage_v: 440.0, cable_type: "FEF".into() });

plan.routes.push(Route {
    system: 0,
    waypoints: vec![Vector3::new(0.0, 0.0, 2.0), Vector3::new(20.0, 0.0, 2.0)],
    cross_section_m: 0.3,
});
let clashes = plan.collision_detection(&hull_geometry);
```

## How it works

- Screening is axis-aligned bounding boxes — fast, deterministic, and conservative; exact pipe-vs-tray contact needs a collision substrate.
- The hull check requires each route to stay inside the envelope's bounding box; anything poking out is a hull clash.
- Installation order ties break by plan order, so the sequence is stable across runs.

## Verification

- Crossing routes clash exactly once; parallel routes with clearance do not
- Routes poking out of the hull are flagged `SystemVsHull`
- Large-first installation order verified

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `outfitting` `routing` `clash-detection` `piping` `installation` |
| Categories | `science` `simulation` `algorithms` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-outfitting` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
