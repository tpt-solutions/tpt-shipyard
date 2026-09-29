# Changelog for tpt-yard-propellant

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `PropellantSpec` presets: LOX, LH2, LCH4, MMH, NTO with density, latent heat, boiling point
- `LoadingPlanner::plan_loading` — fill rate, chilldown time (tank thermal mass vs latent heat), venting requirement, subcooling target
- `boil_off_report` — kg/day and %/day boil-off, time-to-vent, cryocooler power for ZBO
- `BoilOffPolicy` — Vented, Recooled, or ZeroBoilOff, assigned by qualification
- Storable propellants: no chilldown, no venting, ZBO by construction

### Verification

- Cryogenic stability test balances 2000 W against LOX latent heat (811 kg/day, vented policy)
- LH2 verified as the worst percentage boil-off (low density, modest latent heat)
- ZBO qualification tested both sides of the threshold

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
