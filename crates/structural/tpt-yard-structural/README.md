# tpt-yard-structural

> Structural analysis of incomplete structures at every build phase.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-structural.svg)](https://crates.io/crates/tpt-yard-structural)
[![Docs.rs](https://docs.rs/tpt-yard-structural/badge.svg)](https://docs.rs/tpt-yard-structural)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

In-service analysis tools assume the vessel is complete. During construction it is not: blocks are missing, welds are partial, and the loads are entirely different — crane picks, temporary supports, launching ways. `tpt-yard-structural` analyses the structure *as it exists at a build phase*.

Members carry the phase at which they become load-bearing; the solver assembles the sub-model erected so far and solves the construction load cases. A phase whose staging is a mechanism is refused outright rather than silently solved — a missing diagonal is a finding, not a NaN.

## Features

- `PartialStructure` — every member tagged with its erection phase
- `analyze_at_phase` — staged truss analysis under construction loads: gravity, wind, crane picks (with dynamic factor), hydrostatic, slamming, docking, robotic reactions
- `fem::TrussModel` — dependency-free 3D truss solver: direct stiffness, penalty BCs, dense elimination; swap-ready for the `tpt-fem` substrate
- `lifting_analysis` — crane-pick statics: sling loads by lever rule, angles, tip check, sling and crane utilization
- `launch_analysis` — way-pressure screening against the 500 kPa class limit
- Refuses mechanisms with `FemError::SingularSystem` instead of returning nonsense

## Installation

```toml
[dependencies]
tpt-yard-structural = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-structural = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{PhaseId, Vector3};
use tpt_yard_structural::{
    ConstructionLoad, ConstructionStructuralSolver, PartialElement,
    PartialStructure, Support,
};

let structure = PartialStructure {
    nodes: vec![
        Vector3::new(0.0, 0.0, 4.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(3.0, 0.0, 4.0),
    ],
    elements: vec![
        PartialElement { nodes: [1, 0], area_m2: 0.01, youngs_modulus_gpa: 210.0,
            density_kg_m3: 7850.0, erected_at: PhaseId(1) },
        // ... more members, each with its erection phase
    ],
    supports: vec![Support { node: 1, fix_x: true, fix_y: true, fix_z: true }],
};
let solver = ConstructionStructuralSolver::new(structure, 355.0);
let result = solver.analyze_at_phase(PhaseId(2), &[ConstructionLoad::Gravity])?;
```

## How it works

- Direct stiffness method on 2-node bar elements; penalty stiffness 1e13 gives ~1e-4 relative compliance (documented in the test tolerances).
- Lifting statics: exact lever rule for two-point picks, inverse-distance sharing beyond; a CoG outside the lift points is a failed pick.
- The substrate `tpt-fem` is not published yet; the staging concepts are solver-independent so the swap is mechanical.

## Verification

- Single axial bar vs `delta = FL/EA`; two-bar truss vs `N = F/(2 sin theta)`; vertical bar self weight vs `delta = WL/(2EA)`
- Staged erection test: the open frame is a mechanism at phase 1, the closed triangle solves at phase 2
- Crane overload and slipway over-pressure paths flagged as `passed: false`

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `fem` `truss` `structural-analysis` `construction` `lifting` |
| Categories | `science` `simulation` `mathematics` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-structural` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
