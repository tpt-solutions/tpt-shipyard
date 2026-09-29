# Changelog for tpt-yard-launch

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `slipway_launch` — sliding velocity, way-pressure progression, tip-up risk, entry angle, slamming pressure
- End-poppet bearing check against the 0.5 MPa class screening limit with explicit `end_bearing_m` cribbing length
- Sticky-ways detection (grease friction >= tan slope): the launch will not run
- `launch_stability` — GM at the launch condition from the weight model
- `drydock_flooding` — float-out sequence via the docking crate

### Verification

- Golden: `slipway-launch-stability.json` — 8.73 m/s entry, 0.163 -> 0.490 MPa pressures, no tip-up
- Tip-up detection for a known-bad CoG position; sticky ways; undersized cribbing
- Capsize detection from the weight model with KG above the metacentre

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
