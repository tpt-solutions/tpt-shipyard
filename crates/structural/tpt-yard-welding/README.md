# tpt-yard-welding

> Welding simulation and distortion control for ship construction.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-welding.svg)](https://crates.io/crates/tpt-yard-welding)
[![Docs.rs](https://docs.rs/tpt-yard-welding/badge.svg)](https://docs.rs/tpt-yard-welding)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

What does a weld do to the surrounding plate? `tpt-yard-welding` answers with the classical analytical chain: a Rosenthal moving point source for the thermal cycle, a yield-limited residual-stress field after cooling, calibrated contraction estimates for shrinkage and angular distortion, and a sequence ranker that prefers backstep-style, spread-out pass orders.

It is engineering-grade, not thermo-mechanical FEM: fast enough to evaluate every seam on the ship inside a digital-twin loop (microseconds per weld), verified against published ranges and a locked golden panel, and documented equation by equation in RFC 0004.

## Features

- Rosenthal quasi-stationary 3D thermal cycle: temperature history, peak temperature, and the t8/5 cooling time at any distance from the seam
- Residual stress: parabolic tension zone bounded by the T_mech isotherm, balanced by uniform compression
- Distortion: transverse and longitudinal shrinkage, angular distortion (single-side vs balanced grooves), bowing
- Seven processes with arc efficiencies (SMAW through EBW and laser)
- Multi-pass superposition and candidate-sequence optimisation
- WPS record loading from JSON (`test-data/welding-procedures/`)
- Procedure advisor: carbon equivalents (CE(IIW), CET, Pcm), Graville class, SEW 088-style preheat screening, closed-form preheat for a target t8/5, supplier t8/5 windows, and WPS-inside-PQR essential-variable checks (ISO 15614-1 / ASME IX style)

## Installation

```toml
[dependencies]
tpt-yard-welding = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-welding = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::Material;
use tpt_yard_joints::{GrooveType, JointGeometry, JointKind};
use tpt_yard_welding::{WeldProcess, WeldProcedure, WeldingSimulation};

let procedure = WeldProcedure {
    process: WeldProcess::Saw,
    heat_input_kj_mm: 12.0,
    travel_speed_mm_s: 8.0,
    preheat_temp_c: 50.0,
    interpass_temp_c: 150.0,
    filler_metal: "S2Si2 / SA AB1 47".into(),
    sequence: vec![],
};
let joint = JointGeometry::new(JointKind::Butt)
    .with_thickness_mm(12.0)
    .with_groove(GrooveType::V)
    .with_groove_angle_deg(60.0)
    .with_root_gap_mm(3.0)
    .with_root_face_mm(2.0);

let sim = WeldingSimulation::new(procedure, Material::ah36(), joint);
let cycle = sim.thermal_cycle(8.0)?;      // temperature history at 8 mm
let distortion = sim.distortion()?;       // shrinkage + angular
```

## How it works

- Rosenthal 3D: `T - T0 = Q/(2 pi k R) * exp(-v (R + xi) / (2 alpha))`; the sample window covers at least a metre of travel because the 3D tail decays like 1/R.
- t8/5 is (near) distance-independent in 3D and scales with heat input — both properties are asserted in tests.
- Distortion constants are calibrated against published AH36 panel data and locked in the golden file; see RFC 0004 for the formulas.

## Verification

- Golden: `test-data/golden/sea/welding-distortion-panel.json` at ±5 % (±10 % for t8/5)
- Peak decays monotonically with distance; heavier heat input lengthens t8/5
- Balanced X grooves halve angular distortion; sequence ranker prefers spread, alternating passes

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `welding` `rosenthal` `heat-input` `distortion` `haz` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-welding` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
