# Changelog for tpt-yard-digital-twin

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `DigitalTwin` — owns the `VesselProject`, the weight model, and the assembly state (RFC 0001)
- `advance_phase` — dependency-validated, support-checked activity completion; failures leave the twin untouched
- `SupportCondition` — keel blocks, slipway ways, floating, or orbital (microgravity)
- `structural_check_at_phase` — the same tipping analysis projected at any past or future phase end
- `centre_of_gravity_tracking` — the cumulative (weight, CoG) curve per phase for launch officers and orbital integrators
- Quality records and sensor readings attached to the build

### Verification

- Milestone test `ten_phase_tracking`: 10 phases, weight and CoG match the closed form at every step, reactions positive throughout
- Tipping refusal test: a CoG marched outboard of the cribbing is rejected and rolled back
- Dependency, double-completion and unknown-phase error paths covered

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
