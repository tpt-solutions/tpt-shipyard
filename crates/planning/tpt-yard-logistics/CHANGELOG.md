# Changelog for tpt-yard-logistics

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `MaterialFlow` — the manifest: items with quantity, lead time, staging footprint, and the consuming activity
- `schedule_deliveries` — order-by, arrive-by, need date and dwell per item
- Staging occupancy sweep: peak concurrent footprint checked against the available laydown area
- Negative order dates (order before project start) fall out naturally

### Verification

- Order/arrive/need dates match the hand-computed schedule for a two-activity chain
- Staging overflow detected and cleared by enlarging the laydown area
- Unknown-activity references rejected

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
