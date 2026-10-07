# tpt-yard-space-structural

> Structural design without launch constraints: vacuum, rotation, thermal cycling.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-space-structural.svg)](https://crates.io/crates/tpt-yard-space-structural)
[![Docs.rs](https://docs.rs/tpt-yard-space-structural/badge.svg)](https://docs.rs/tpt-yard-space-structural)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

A structure assembled in orbit never rides a rocket: no fairing diameter, no Max-Q, no ascent vibration, no 1-g handling loads. `tpt-yard-space-structural` states that freedom formally and implements the load cases that *do* govern: hoop stress in a spinning habitat ring, thermal-cycling fatigue across eclipse transitions, and Whipple-shield sizing against micrometeoroids.

The centrepiece is the spec's verification identity — hoop stress `sigma = rho omega^2 r^2` — verified to machine precision and locked as golden data.

## Features

- `rotating_habitat_stress` — hoop stress, angular velocity, radial growth, and yield utilization for a spinning ring
- `thermal_cycling_fatigue` — strain range from the orbit hot/cold swing, Coffin-Manson life, Miner's-rule life fraction
- `micrometeoroid_shielding` — Whipple bumper / standoff / rear-wall sizing with areal density
- `no_launch_constraint` — the formal design-freedom statement (`ShapeConstraint::None`, unbounded size)
- `SpaceEnvironment` — orbit, thermal cycling, flux, radiation, atomic oxygen

## Installation

```toml
[dependencies]
tpt-yard-space-structural = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-space-structural = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::Material;
use tpt_yard_space_structural::SpaceStructuralDesigner;

let designer = SpaceStructuralDesigner::new(Material::aa5083());

// The spec's reference case: 2 rpm, 100 m ring.
let stress = designer.rotating_habitat_stress(100.0, 2.0, 1.0e6);
println!("hoop: {:.2} MPa (util {:.1}%)",
         stress.hoop_stress_mpa, stress.utilization * 100.0);

let fatigue = designer.thermal_cycling_fatigue(15 * 5_660); // 15 LEO years
println!("life fraction: {:.2}", fatigue.life_fraction);
```

## How it works

- Thin-ring hoop stress with the structural material density; radial growth is the elastic strain times the radius.
- Coffin-Manson life `N_f = 0.5 (dE / (3.5 sigma_u / E))^(-1/0.12)`; aluminium's higher expansion burns life faster than steel (tested ordering).
- Whipple ratios (bumper d/8, standoff 10·d, rear wall 0.4·d) are calibrated to published ISS dual-wall sets, documented in RFC 0005.

## Verification

- Golden: `rotating-habitat-stress.json` at machine precision
- Quadratic scaling in rpm and radius asserted; steel at high rpm exceeds yield
- Shield sizing ratios and fatigue ordering by material tested

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `space-structures` `habitat` `whipple-shield` `thermal-fatigue` `aerospace` |
| Categories | `aerospace` `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-space-structural` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
