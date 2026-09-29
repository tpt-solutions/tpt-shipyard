# Changelog for tpt-yard-transport-link

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `plan_construction` — maritime designs become drydock projects with the five-stage skeleton; orbital spacecraft become assembly projects
- `handover_to_operations` — as-built weight, CoG, structural summary and accepted quality records from the finished twin
- The spec section 6 contract, implemented and round-trip tested

### Verification

- Round-trip test: design a ship, build all five phases through the twin, assert the handover carries exactly the installed mass and closed-form CoG
- Spacecraft designs map to `OrbitalAssembly`

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
