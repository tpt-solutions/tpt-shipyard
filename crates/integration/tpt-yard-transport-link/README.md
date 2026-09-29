# tpt-yard-transport-link

> Bridge between vehicle design (tpt-transport) and shipyard construction.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-transport-link.svg)](https://crates.io/crates/tpt-yard-transport-link)
[![Docs.rs](https://docs.rs/tpt-yard-transport-link/badge.svg)](https://docs.rs/tpt-yard-transport-link)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

The lifecycle reads: design, build, operate. `tpt-yard-transport-link` is the first and last mile — mapping a vehicle design to a construction plan, and handing the finished twin's as-built truth back to the operational model.

The `tpt-transport` substrate is not published yet, so the design-side types (`VehicleDesign`, `AsBuiltProperties`) are vendored behind the same shapes the substrate will use; the swap is mechanical when it lands.

## Features

- `plan_construction` — maritime designs become drydock projects with the five-stage skeleton; orbital spacecraft become assembly projects
- `handover_to_operations` — as-built weight, CoG, structural summary and accepted quality records from the finished twin
- The spec section 6 contract, implemented and round-trip tested

## Installation

```toml
[dependencies]
tpt-yard-transport-link = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-transport-link = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{ConstructionMethod, VesselType};
use tpt_yard_transport_link::{plan_construction, DesignKind, VehicleDesign};

let design = VehicleDesign {
    name: "Container ship 1400 TEU".into(),
    kind: DesignKind::Maritime { loa_m: 140.0, deadweight_t: 18_500.0 },
};
let project = plan_construction(&design);
assert_eq!(project.construction_method, ConstructionMethod::SeaDrydock);
```

## How it works

- Construction method follows the design family: maritime -> `SeaDrydock`, orbit-assembled spacecraft -> `OrbitalAssembly`.
- The handover carries *installed* (as-built) numbers — the operational model gets reality, not the design brochure.

## Verification

- Round-trip test: design a ship, build all five phases through the twin, assert the handover carries exactly the installed mass and closed-form CoG
- Spacecraft designs map to `OrbitalAssembly`

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `integration` `as-built` `handover` `vessel` `lifecycle` |
| Categories | `science` `simulation` `development-tools` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-transport-link` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
