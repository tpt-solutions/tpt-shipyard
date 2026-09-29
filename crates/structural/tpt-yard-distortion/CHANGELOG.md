# Changelog for tpt-yard-distortion

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `DistortionControl` — target vs measured geometry with a correction history
- `deviation_map` — per-vertex signed deviation (dominant axis, mm), max and RMS
- `correction_plan(tol)` — Accept / HeatStraighten / Reject / Rework decisions from explicit thresholds
- Systematic-distortion detection: > 20 % of vertices out of tolerance triggers rework instead of point-wise heating
- Topology mismatch (target vs measurement) is an explicit error, never silently ignored

### Verification

- Boundary tests just inside and just outside the tolerance
- Gross-deviation rejection and systematic-rework trigger tests
- Mismatched-topology rejection with a typed error

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
