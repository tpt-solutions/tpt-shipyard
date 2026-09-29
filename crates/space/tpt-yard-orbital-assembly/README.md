# tpt-yard-orbital-assembly

> Orbital assembly planning and simulation for space structures.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-orbital-assembly.svg)](https://crates.io/crates/tpt-yard-orbital-assembly)
[![Docs.rs](https://docs.rs/tpt-yard-orbital-assembly/badge.svg)](https://docs.rs/tpt-yard-orbital-assembly)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Building a truss at Earth-Moon L2 looks nothing like building a ship — no gravity, but docking impulses at the growing free end, handling force limits at the grippers, and collision geometry everywhere. `tpt-yard-orbital-assembly` plans the sequence (anchored bays first, build outwards), simulates every step against constraint tags, and verifies the partially assembled cantilever against docking impulses.

Root stress under a tip impulse grows *linearly* with deployed length — the deployment risk that makes assembly order matter, and the reason the integrity verifier reports at every single step.

## Features

- `OrbitalAssembly::plan_sequence` — grasp, translate, rotate, dock, bolt, release per component, constraint-tagged
- `simulate_step` — handling force (`m a` with station keeping + manoeuvre allowance), swept bounding-sphere collision checks against installed components
- `verify_structural_integrity` — cantilever root stress at any step: `sigma = P L / (A h)`, growing with deployment
- `SpaceStructure` taxonomy: truss, station, rotating habitat, solar array, fuel depot, arbitrary mesh
- Constraint vocabulary: `NoCollision`, `ForceLimit`, `MaintainStationKeeping`, thermal and line-of-sight tags
- Collision objects exported for robot path planning

## Installation

```toml
[dependencies]
tpt-yard-orbital-assembly = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-orbital-assembly = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{ComponentId, Vector3};
use tpt_yard_orbital_assembly::{
    ComponentSpec, OrbitalAssembly, OrbitalParameters, SpaceStructure,
};

let mut assembly = OrbitalAssembly::new(
    SpaceStructure::Truss { segments: 4, length_m: 20.0 },
    OrbitalParameters::default(),
);
assembly.add_component(ComponentSpec {
    id: ComponentId(1),
    name: "bay 1".into(),
    mass_kg: 500.0,
    dimensions: Vector3::new(5.0, 3.0, 3.0),
    target_position: Vector3::new(2.5, 0.0, 0.0),
});
let steps = assembly.plan_sequence();
```

## How it works

- Collision spheres use the largest half-axis — the 3D diagonal makes adjacent same-pitch bays overlap spuriously (RFC 0003).
- The cantilever check assumes a constant cross-section (0.01 m2 chord, 3 m bay height, ISS-like); stress therefore grows linearly with length, matching the golden growth law.
- Sequence planning is doctrinal and deterministic; time/energy optimisation belongs to `tpt-yard-scheduling`.

## Verification

- `test_orbital_assembly_collision`: a mis-planned bay sharing a target with an installed bay is caught
- Golden: `orbital-assembly-sequence.json` and `iss-truss-assembly.json` (linear growth law)
- Over-mass components violate force limits; violent docking impulses fail the integrity check

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `orbital-assembly` `space` `truss` `robotics` `simulation` |
| Categories | `aerospace` `aerospace::simulation` `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-orbital-assembly` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
