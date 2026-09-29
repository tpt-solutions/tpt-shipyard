# tpt-yard-distortion

> Block distortion management: deviation maps and correction planning.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-distortion.svg)](https://crates.io/crates/tpt-yard-distortion)
[![Docs.rs](https://docs.rs/tpt-yard-distortion/badge.svg)](https://docs.rs/tpt-yard-distortion)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

After a block is welded it never matches the drawing exactly. `tpt-yard-distortion` closes the quality loop: compare the laser-measured geometry against target, quantify the deviation field, and plan the correction — accept inside tolerance, heat-straighten the moderate deviations, reject the gross ones, and rework blocks that are systematically out of tolerance.

The correction policy is explicit and threshold-tested, so the plan a planner sees is the plan the code produces: no silent acceptance beyond tolerance, no heat straightening prescribed where rework is the economical answer.

## Features

- `DistortionControl` — target vs measured geometry with a correction history
- `deviation_map` — per-vertex signed deviation (dominant axis, mm), max and RMS
- `correction_plan(tol)` — Accept / HeatStraighten / Reject / Rework decisions from explicit thresholds
- Systematic-distortion detection: > 20 % of vertices out of tolerance triggers rework instead of point-wise heating
- Topology mismatch (target vs measurement) is an explicit error, never silently ignored

## Installation

```toml
[dependencies]
tpt-yard-distortion = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-distortion = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{Geometry3D, Vector3};
use tpt_yard_distortion::{CorrectionAction, DistortionControl};

let target = Geometry3D::from_box(2000.0, 500.0, 100.0);
let mut measured = target.clone();
measured.vertices[0] = measured.vertices[0] + Vector3::new(0.0, 0.0, 4.0);

let mut dc = DistortionControl::new(target, measured);
let plan = dc.correction_plan(3.0); // +/- 3 mm tolerance
// The 4 mm bump is heat-straightened (or rejected), never accepted.
```

## How it works

- Thresholds: within tolerance accepted; up to 5x tolerance heat-straightened (heat input scaled to the excess); beyond 5x rejected.
- Point-wise straightening is suppressed when more than a fifth of the vertices are out — the block needs systematic rework.
- Every planned action is appended to `corrections`, giving the block's as-built correction history for the handover record.

## Verification

- Boundary tests just inside and just outside the tolerance
- Gross-deviation rejection and systematic-rework trigger tests
- Mismatched-topology rejection with a typed error

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `distortion` `heat-straightening` `deviation` `tolerance` `quality` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-distortion` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
