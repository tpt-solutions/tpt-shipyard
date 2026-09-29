# tpt-yard-digital-twin

> Construction state tracking and simulation: the shipyard digital twin.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-digital-twin.svg)](https://crates.io/crates/tpt-yard-digital-twin)
[![Docs.rs](https://docs.rs/tpt-yard-digital-twin/badge.svg)](https://docs.rs/tpt-yard-digital-twin)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

The digital twin mirrors a build in progress: which activities are complete, what mass is physically installed, where the centre of gravity sits — and, the question no in-service analysis tool asks, whether the *partially built* structure can still stand on its supports at every stage.

Everything flows through `advance_phase`: the twin validates dependencies, dry-runs the support check with the activity's mass added, and only then commits — installing wired weight items, recomputing mass properties, and advancing the phase pointer. An erection step that would tip the hull is refused outright.

## Features

- `DigitalTwin` — owns the `VesselProject`, the weight model, and the assembly state (RFC 0001)
- `advance_phase` — dependency-validated, support-checked activity completion; failures leave the twin untouched
- `SupportCondition` — keel blocks, slipway ways, floating, or orbital (microgravity)
- `structural_check_at_phase` — the same tipping analysis projected at any past or future phase end
- `centre_of_gravity_tracking` — the cumulative (weight, CoG) curve per phase for launch officers and orbital integrators
- Quality records and sensor readings attached to the build

## Installation

```toml
[dependencies]
tpt-yard-digital-twin = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-digital-twin = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{ItemId, PhaseId, ProjectId, Vector3};
use tpt_yard_digital_twin::{DigitalTwin, SupportCondition};
use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

let mut twin = DigitalTwin::with_weight_model(project, weight_model,
    SupportCondition::KeelBlocks {
        positions: vec![
            Vector3::new(-10.0, -4.0, 0.0), Vector3::new(-10.0, 4.0, 0.0),
            Vector3::new(130.0, -4.0, 0.0), Vector3::new(130.0, 4.0, 0.0),
        ],
    },
);

twin.advance_phase(&ActivityId(1))?; // Err(UnsoundStructure) if it would tip
let report = twin.centre_of_gravity_tracking()?;
```

## How it works

- Support statics treat the partial structure as a rigid body: longitudinal tipping against the extreme keel supports (negative reaction = refuse), transverse CoG against the block half-track.
- The dry run includes the completing activity's wired mass *before* committing, so an unsound advance never mutates state.
- Phase 2's `tpt-yard-structural` adds FEM-level checks on top; the twin's rigid-body gate stays valid in every domain.

## Verification

- Milestone test `ten_phase_tracking`: 10 phases, weight and CoG match the closed form at every step, reactions positive throughout
- Tipping refusal test: a CoG marched outboard of the cribbing is rejected and rolled back
- Dependency, double-completion and unknown-phase error paths covered

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `digital-twin` `construction` `simulation` `monitoring` `shipbuilding` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-digital-twin` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
