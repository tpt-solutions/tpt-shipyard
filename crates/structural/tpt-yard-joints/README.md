# tpt-yard-joints

> Shared joint-geometry primitives for structural and welding analysis.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-joints.svg)](https://crates.io/crates/tpt-yard-joints)
[![Docs.rs](https://docs.rs/tpt-yard-joints/badge.svg)](https://docs.rs/tpt-yard-joints)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

A `JointGeometry` describes the *shape* of the connection — joint kind, groove preparation, thickness, root gap, fillet legs, penetration credit. From those it derives the quantities downstream physics needs: groove cross-section for weld-metal heat budgets, effective throat for strength, and weld volume/mass for consumable planning.

Both the welding and structural crates build on these primitives, so a 60-degree single-V means exactly the same thing everywhere in the engine.

## Features

- Joint families: butt, fillet, lap, T-joint, corner, edge
- Groove preparations: square, V, double-V, bevel, K, J, U
- Groove cross-section area (trapezoidal profiles + root gap + penetration credit)
- Effective fillet throat (`leg / sqrt 2` + penetration) for strength checks
- Weld volume and filler mass for the whole seam
- Double-side accounting (`weld_area_mm2_total`) for X/K grooves and two-toe joints

## Installation

```toml
[dependencies]
tpt-yard-joints = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-joints = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_joints::{GrooveType, JointGeometry, JointKind};

// Single-V butt weld, 60 degrees included, 12 mm plate.
let butt = JointGeometry::new(JointKind::Butt)
    .with_thickness_mm(12.0)
    .with_groove(GrooveType::V)
    .with_groove_angle_deg(60.0)
    .with_root_gap_mm(3.0)
    .with_root_face_mm(2.0);

println!("weld metal: {:.1} kg", butt.weld_mass_kg(7850.0));
```

## How it works

- V-family areas are trapezoidal: `(t - face)^2 * tan(half_angle)` plus the gap filling the root face height.
- Deep penetration (SAW-class) extends the fused zone below the root; the added area is the penetration depth continuing through the gap channel.
- T-joints are fillet-welded when a leg is given and groove-welded (full penetration) otherwise.

## Verification

- Closed-form areas for V-groove, square-groove and fillet cases
- Double-side totals for X grooves and two-toe T-joints
- Volume/mass consistency at steel density

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `welding` `joint-geometry` `groove` `fillet` `throat` |
| Categories | `science` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-joints` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
