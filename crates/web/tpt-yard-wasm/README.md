# tpt-yard-wasm

> WebAssembly bindings for interactive shipyard construction dashboards.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-wasm.svg)](https://crates.io/crates/tpt-yard-wasm)
[![Docs.rs](https://docs.rs/tpt-yard-wasm/badge.svg)](https://docs.rs/tpt-yard-wasm)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

`tpt-yard-wasm` compiles the construction engine to WebAssembly so a browser can drive a build: advance erection phases, pull geometry for WebGL buffers, and read weight/CoG and structural-check reports as JSON — all computed by the same Rust engine the server-side tools use.

The façades hold no rendering state: the browser draws, the engine computes. The workspace ships a working reference dashboard in `www/` that erects a 12-block ship on a plain canvas with zero runtime dependencies.

## Features

- `WasmDigitalTwin` — load a `VesselProject` from JSON, advance phases, dependency-gated
- `get_geometry` / `get_geometry_indices` — flat triangle-soup vertex and index buffers, upload-ready
- `get_weight_report` / `structural_check` — JSON reports (installed vs design weight, CoG, keel reactions, margin)
- `WasmOrbitalAssembly` — step through a truss build (`simulate_next_step`), robot pose and installed components for rendering
- Compiles and unit-tests on native targets (wasm-bindgen degrades gracefully)

## Installation

```toml
[dependencies]
tpt-yard-wasm = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-wasm = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
import init, { WasmDigitalTwin } from "./pkg/tpt_yard_wasm.js";

await init();
const twin = new WasmDigitalTwin(projectJson);

twin.advance_next();                  // erect the next block
const verts = twin.get_geometry();    // Float32Array triples for WebGL
const report = JSON.parse(twin.get_weight_report());
console.log(report.installed_kg, report.cog);
```

## How it works

- One weight item per activity is synthesised from the phase design weights, laid out along +X; keel-block supports span the plan automatically.
- Geometry is a box per completed erection step in plan order — dashboards that carry real block meshes swap the buffer source, the report path is unchanged.

## Verification

- Native tests: JSON round-trip, dependency-gated advance, geometry buffer sizes
- Orbital façade: full 18-step sequence simulated step by step
- Reference dashboard page in `www/` (2-D plan view); the interactive 3-D WASM mesh demo is on the roadmap

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `wasm` `webassembly` `digital-twin` `dashboard` `shipyard` |
| Categories | `web-programming::wasm` `science` `simulation` `graphics` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-wasm` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
