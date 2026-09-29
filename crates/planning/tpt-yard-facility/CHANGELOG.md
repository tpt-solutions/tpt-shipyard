# Changelog for tpt-yard-facility

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `FacilityPlan` — placed facilities with typed capacities (tonnes, m2, berth slots, cells)
- `check_capacity` — peak demand of a kind against the summed capacity
- `can_place` / `validate` — axis-aligned footprint placement with clearance; overlapping pairs named
- `FacilityKind` units documented per kind (`t`, `m2`, `slots`, `cells`)

### Verification

- Capacity pass/fail on both sides of the limit
- Overlap detection names the offending facility pair
- Clearance-respecting placement accepted

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
