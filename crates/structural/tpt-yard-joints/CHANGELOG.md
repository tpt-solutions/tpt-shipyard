# Changelog for tpt-yard-joints

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- Joint families: butt, fillet, lap, T-joint, corner, edge
- Groove preparations: square, V, double-V, bevel, K, J, U
- Groove cross-section area (trapezoidal profiles + root gap + penetration credit)
- Effective fillet throat (`leg / sqrt 2` + penetration) for strength checks
- Weld volume and filler mass for the whole seam
- Double-side accounting (`weld_area_mm2_total`) for X/K grooves and two-toe joints

### Verification

- Closed-form areas for V-groove, square-groove and fillet cases
- Double-side totals for X grooves and two-toe T-joints
- Volume/mass consistency at steel density

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
