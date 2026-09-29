# tpt-yard-facility

> Shipyard and orbital-facility layout planning with capacity constraints.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-facility.svg)](https://crates.io/crates/tpt-yard-facility)
[![Docs.rs](https://docs.rs/tpt-yard-facility/badge.svg)](https://docs.rs/tpt-yard-facility)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Can the yard actually build what the schedule promises? `tpt-yard-facility` models the yard's physical plant — cranes, workshops, docks, orbital bays — as placed facilities with capacities, checks peak demand against them, and keeps the layout honest with clearance-aware placement validation.

## Features

- `FacilityPlan` — placed facilities with typed capacities (tonnes, m2, berth slots, cells)
- `check_capacity` — peak demand of a kind against the summed capacity
- `can_place` / `validate` — axis-aligned footprint placement with clearance; overlapping pairs named
- `FacilityKind` units documented per kind (`t`, `m2`, `slots`, `cells`)

## Installation

```toml
[dependencies]
tpt-yard-facility = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-facility = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::Vector3;
use tpt_yard_facility::{Facility, FacilityKind, FacilityPlan};

let mut plan = FacilityPlan::new();
plan.add(Facility {
    name: "Goliath crane".into(),
    kind: FacilityKind::Crane,
    capacity: 1_200.0,
    position: Vector3::new(0.0, 0.0, 0.0),
    footprint_m: (30.0, 30.0),
});

assert!(plan.check_capacity(FacilityKind::Crane, 900.0));
assert!(!plan.check_capacity(FacilityKind::Crane, 1_500.0));
```

## How it works

- Placement overlap is axis-aligned footprint rectangles grown by the clearance — conservative and deterministic.
- Capacity checks sum over all facilities of a kind, so adding a second crane simply raises the ceiling.

## Verification

- Capacity pass/fail on both sides of the limit
- Overlap detection names the offending facility pair
- Clearance-respecting placement accepted

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `facility-layout` `capacity` `shipyard` `crane` `planning` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-facility` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
