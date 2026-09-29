# tpt-yard-habitat

> Rotating habitat design: artificial gravity, Coriolis comfort, structure.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-habitat.svg)](https://crates.io/crates/tpt-yard-habitat)
[![Docs.rs](https://docs.rs/tpt-yard-habitat/badge.svg)](https://docs.rs/tpt-yard-habitat)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

How fast must a 100 m torus spin for 1 g, and will the occupants get sick? `tpt-yard-habitat` answers the three rotating-habitat questions: spin rate (`omega = sqrt(g/r)`), the 2 rpm Coriolis comfort limit, and how much structure the spinning hull needs.

The design tension is explicit in the tests: 1 g at 100 m needs 2.99 rpm (above the comfort limit — mitigate or grow); 1 g at 4 km needs 0.47 rpm (comfortable). Radius is the knob, and the crate makes the trade computable.

## Features

- `required_rotation` / `required_rotation_rpm` — `omega = sqrt(g/r)` for any target gravity (Earth, Mars, lunar)
- `coriolis_effects` — cross-coupled acceleration for head movements and the inclusive 2 rpm comfort check
- `structural_design` — hoop area from ring tension `T = m omega^2 r / (2 pi)`, shell thickness from the circumference, with safety factor
- `HabitatType` taxonomy: O'Neill cylinder, Stanford torus, Bernal sphere, ring station, custom
- Self-consistent results: utilization <= 1 by construction, monotone in mass and safety factor (tested)

## Installation

```toml
[dependencies]
tpt-yard-habitat = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-habitat = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::Material;
use tpt_yard_habitat::{HabitatDesigner, HabitatType};

let designer = HabitatDesigner::new(
    HabitatType::StanfordTorus { radius_m: 100.0, tube_diameter_m: 20.0 },
    Material::aa5083(),
);

let rpm = designer.required_rotation_rpm(100.0, 1.0);   // ~2.99 rpm
let comfort = designer.coriolis_effects(100.0, rpm);
assert!(!comfort.within_comfort); // above the 2 rpm limit
```

## How it works

- Comfort criterion: rotation <= 2 rpm keeps cross-coupled accelerations from 1 m/s head movements (~4.3 % g at 2 rpm) below the ~10 % nausea threshold.
- Ring tension distributes the total rotating mass around the circumference; the shell thickness follows for the habitat archetype's radius.
- The 1 g design point is documented; re-run `structural_design` math for Mars-g by scaling omega.

## Verification

- `omega = sqrt(g/r)` verified exactly, including the Mars-gravity `sqrt(0.38)` scaling
- The 2 rpm comfort boundary tested inclusive and exclusive
- O'Neill reference: 4 km radius at 0.47 rpm is comfortable

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `habitat` `artificial-gravity` `coriolis` `rotating-ring` `oneill` |
| Categories | `aerospace` `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-habitat` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
