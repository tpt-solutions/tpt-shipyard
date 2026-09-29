# Changelog for tpt-yard-habitat

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `required_rotation` / `required_rotation_rpm` — `omega = sqrt(g/r)` for any target gravity (Earth, Mars, lunar)
- `coriolis_effects` — cross-coupled acceleration for head movements and the inclusive 2 rpm comfort check
- `structural_design` — hoop area from ring tension `T = m omega^2 r / (2 pi)`, shell thickness from the circumference, with safety factor
- `HabitatType` taxonomy: O'Neill cylinder, Stanford torus, Bernal sphere, ring station, custom
- Self-consistent results: utilization <= 1 by construction, monotone in mass and safety factor (tested)

### Verification

- `omega = sqrt(g/r)` verified exactly, including the Mars-gravity `sqrt(0.38)` scaling
- The 2 rpm comfort boundary tested inclusive and exclusive
- O'Neill reference: 4 km radius at 0.47 rpm is comfortable

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
