# Changelog for tpt-yard-wasm

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `WasmDigitalTwin` — load a `VesselProject` from JSON, advance phases, dependency-gated
- `get_geometry` / `get_geometry_indices` — flat triangle-soup vertex and index buffers, upload-ready
- `get_weight_report` / `structural_check` — JSON reports (installed vs design weight, CoG, keel reactions, margin)
- `WasmOrbitalAssembly` — step through a truss build (`simulate_next_step`), robot pose and installed components for rendering
- Compiles and unit-tests on native targets (wasm-bindgen degrades gracefully)

### Verification

- Native tests: JSON round-trip, dependency-gated advance, geometry buffer sizes
- Orbital façade: full 18-step sequence simulated step by step
- Reference dashboard page in `www/` (2-D plan view); the interactive 3-D WASM mesh demo is on the roadmap

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
