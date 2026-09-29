# Changelog for tpt-yard-space-structural

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `rotating_habitat_stress` — hoop stress, angular velocity, radial growth, and yield utilization for a spinning ring
- `thermal_cycling_fatigue` — strain range from the orbit hot/cold swing, Coffin-Manson life, Miner's-rule life fraction
- `micrometeoroid_shielding` — Whipple bumper / standoff / rear-wall sizing with areal density
- `no_launch_constraint` — the formal design-freedom statement (`ShapeConstraint::None`, unbounded size)
- `SpaceEnvironment` — orbit, thermal cycling, flux, radiation, atomic oxygen

### Verification

- Golden: `rotating-habitat-stress.json` at machine precision
- Quadratic scaling in rpm and radius asserted; steel at high rpm exceeds yield
- Shield sizing ratios and fatigue ordering by material tested

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
