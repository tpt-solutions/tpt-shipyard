# Changelog for tpt-yard-core

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `VesselProject` — the complete build plan: vessel type, construction method, ordered build phases
- Sea and space vessel taxonomies (`SeaVesselType`, `SpaceVesselType`, hybrid) exactly as the master spec defines them
- `BuildPhase` / `AssemblyActivity` / `ActivityType` — the activity model with yard `Resource`s (cranes, workshops, robots, crews)
- Geometry kernel: `Vector3`, `Geometry3D` triangle meshes (box, cylinder, merge, transform, bounding box, centroid), `MassProperties`
- `Material` presets: AH36, S235, AA5083, AISI 316L, Inconel 718, Ti-6Al-4V with full thermal/mechanical property sets
- Newtype ids (`ProjectId`, `PhaseId`, `BlockId`, ...) via the exported `define_id!` macro
- Embedded JSON value/parser/writer — full grammar, surrogate pairs, zero dependencies

### Verification

- JSON round-trip tests over every enum variant (compact and pretty writers)
- Geometry tests: box/cylinder bounds, centroids, rotation length preservation
- Rosenthal-relevant material diffusivity and shear-modulus consistency checks

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
