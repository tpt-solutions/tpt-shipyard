# Changelog for tpt-yard-outfitting

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `OutfittingPlan` — outfit systems plus their 3D polyline routes
- `collision_detection` — bounding-box screening with clearance, against other routes and the hull envelope
- Typed clashes: `SystemVsSystem` and `SystemVsHull`, each with an approximate location
- `installation_sequence` — size-descending order (machinery before ducting before cable trays)
- Route bounding boxes grown by half the cross-section, so clearance is explicit

### Verification

- Crossing routes clash exactly once; parallel routes with clearance do not
- Routes poking out of the hull are flagged `SystemVsHull`
- Large-first installation order verified

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
