# tpt-yard-core

> Fundamental shipyard domain types for the TPT Shipyard construction engine.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-core.svg)](https://crates.io/crates/tpt-yard-core)
[![Docs.rs](https://docs.rs/tpt-yard-core/badge.svg)](https://docs.rs/tpt-yard-core)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

`tpt-yard-core` defines *what* is being built: vessel projects, build phases, assembly activities, and the yard resources they consume. It is the shared vocabulary of the tpt-shipyard construction engine — every other crate consumes these types, so the dependency graph stays acyclic and the data model stays coherent from steel cutting to orbital bolt-up.

The crate also ships the primitives everything else builds on: a small 3D geometry kernel (`Vector3`, triangle meshes, mass properties), newtype identifiers, engineering material presets (AH36 to Inconel 718), and a dependency-free JSON implementation so project files round-trip without pulling in external crates.

## Features

- `VesselProject` — the complete build plan: vessel type, construction method, ordered build phases
- Sea and space vessel taxonomies (`SeaVesselType`, `SpaceVesselType`, hybrid) exactly as the master spec defines them
- `BuildPhase` / `AssemblyActivity` / `ActivityType` — the activity model with yard `Resource`s (cranes, workshops, robots, crews)
- Geometry kernel: `Vector3`, `Geometry3D` triangle meshes (box, cylinder, merge, transform, bounding box, centroid), `MassProperties`
- `Material` presets: AH36, S235, AA5083, AISI 316L, Inconel 718, Ti-6Al-4V with full thermal/mechanical property sets
- Newtype ids (`ProjectId`, `PhaseId`, `BlockId`, ...) via the exported `define_id!` macro
- Embedded JSON value/parser/writer — full grammar, surrogate pairs, zero dependencies

## Installation

```toml
[dependencies]
tpt-yard-core = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-core = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::*;

let mut phase = BuildPhase::new(PhaseId(1), "Erection", 10.0);
phase.activities.push(AssemblyActivity::new(
    ActivityId(1),
    "Erect block 212",
    ActivityType::JoinBlock,
    8.0,
));
let project = VesselProject::new(
    ProjectId(1),
    "Container ship 1400 TEU",
    VesselType::Sea(SeaVesselType::ContainerShip { teu_capacity: 1400 }),
    ConstructionMethod::SeaDrydock,
    vec![phase],
)
.unwrap();

// Round-trips through the dependency-free JSON module.
let json = project.to_json().to_string_pretty();
let same = VesselProject::from_json_str(&json).unwrap();
assert_eq!(same, project);
```

## How it works

- The project deliberately carries **no** digital-twin back-reference — the twin (in `tpt-yard-digital-twin`) owns the project. See RFC 0001 for the ownership decision.
- `VesselProject::activity_graph()` validates the cross-phase dependency network with the `tpt-yard-assembly` algorithms.
- Numbers follow shipyard convention: metres, kilograms, hours, days; vessel dimensions use LOA / breadth / depth nomenclature.

## Verification

- JSON round-trip tests over every enum variant (compact and pretty writers)
- Geometry tests: box/cylinder bounds, centroids, rotation length preservation
- Rosenthal-relevant material diffusivity and shear-modulus consistency checks

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `shipyard` `shipbuilding` `vessel` `digital-twin` `simulation` |
| Categories | `science` `simulation` `mathematics` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-core` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
