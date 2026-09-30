# Changelog for tpt-yard-blocks

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [Unreleased]

### Added

- `pin_bending_check` (review 7H): the shackle pin as a simply supported
  beam spanning the clevis with the lug bearing as a distributed load —
  midspan moment `P(L-t)/4`, section `pi d^3/32`, double shear at the
  supports — against sigma_y/1.5 and 0.6 sigma_y/1.5.

## [0.1.0] - 2026-09-28

### Added

- `distribute_load_shares` — vertical load shares over lift points, summing to 1
- `sling_loads` — per-leg table: share, angle from horizontal, leg tension (`share / sin theta`)
- `sling_angles_ok` — the 30-degree practice-minimum angle check
- `cog_within_lifts` — the pick-time tipping check
- `LiftPoint` with certified working load limit for utilization math

### Verification

- Centred CoG splits evenly; 3-of-12 m offset gives the exact 75/25 lever split
- 45-degree legs: tension exactly `share * sqrt 2`
- Shallow-sling and outside-CoG rejections tested

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
