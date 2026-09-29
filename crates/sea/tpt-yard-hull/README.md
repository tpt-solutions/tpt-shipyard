# tpt-yard-hull

> Hull construction and block management for sea shipyards.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-hull.svg)](https://crates.io/crates/tpt-yard-hull)
[![Docs.rs](https://docs.rs/tpt-yard-hull/badge.svg)](https://docs.rs/tpt-yard-hull)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

`tpt-yard-hull` turns a hull form into an erection plan. Block division splits the hull under the yard's real constraints — every block liftable by the crane (with a rigging allowance), every block fitting the workshop — and the erection sequence builds the ship the way yards actually do: keel tier first, from midship outwards, each block landing against its nearest erected neighbour.

The sequence feeds straight into the digital twin as activities, so the tipping gate (RFC 0001) guards every erection step and the weight/CoG curve grows with the ship.

## Features

- `HullConstruction::block_division` — crane- and workshop-constrained division into tiers and longitudinal bands
- Pre-design weight model from areal steel density (deterministic, auditable)
- `erection_sequence` — bottom-tier-first, midship-outward joins with seam typing (dock joint / butt seam)
- `HullBlock` lifecycle status: Design through Cutting, Welding, Outfitting, Ready, Erected
- Total steel weight and per-block load reporting

## Installation

```toml
[dependencies]
tpt-yard-hull = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-hull = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::Dimensions;
use tpt_yard_hull::{HullConstruction, HullGeometry};

let hull = HullConstruction::new(HullGeometry {
    loa_m: 140.0,
    boa_m: 22.0,
    depth_m: 12.0,
    areal_density_kg_m2: 180.0,
    depth_bands: 2,
});

let blocks = hull.block_division(40_000.0, Dimensions::new(24.0, 30.0, 14.0));
let joins = hull.erection_sequence(&blocks);
assert!(joins[0].z_band == 0); // keel tier first
```

## How it works

- Pre-design weight: areal density over `2*(B + tier height)` per metre — the classic estimate, replaced by itemised weights as the twin fills in.
- Crane limit includes a 10 % rigging allowance; the workshop bounds block length and height independently.
- Midship-outward keeps the growing centroid near midship, maximising keel-block reactions and confining shrinkage away from end joints (RFC 0002).

## Verification

- Golden: `container-ship-block-division.json` — 12 blocks of 23.33 m / 142.8 t, closed form
- Constraint tests: every block under the crane limit and inside the workshop; tighter cranes yield more blocks
- Order tests: tiers never interleave, midship distance non-decreasing, first join a dock joint

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `hull` `block-division` `erection` `shipbuilding` `shipyard` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-hull` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
