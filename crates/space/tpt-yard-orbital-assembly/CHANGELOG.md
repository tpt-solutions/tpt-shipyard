# Changelog for tpt-yard-orbital-assembly

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `OrbitalAssembly::plan_sequence` — grasp, translate, rotate, dock, bolt, release per component, constraint-tagged
- `simulate_step` — handling force (`m a` with station keeping + manoeuvre allowance), swept bounding-sphere collision checks against installed components
- `verify_structural_integrity` — cantilever root stress at any step: `sigma = P L / (A h)`, growing with deployment
- `SpaceStructure` taxonomy: truss, station, rotating habitat, solar array, fuel depot, arbitrary mesh
- Constraint vocabulary: `NoCollision`, `ForceLimit`, `MaintainStationKeeping`, thermal and line-of-sight tags
- Collision objects exported for robot path planning

### Verification

- `test_orbital_assembly_collision`: a mis-planned bay sharing a target with an installed bay is caught
- Golden: `orbital-assembly-sequence.json` and `iss-truss-assembly.json` (linear growth law)
- Over-mass components violate force limits; violent docking impulses fail the integrity check

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
