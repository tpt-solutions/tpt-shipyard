# Changelog for tpt-yard-process-link

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `PengRobinson` — single-component EOS with the classical constants, cubic root solve, vapour and liquid roots
- `vapour_density` at (T, P) from the compressibility factor
- EOS parameter table for LOX, LH2, LCH4, MMH, NTO
- `plan_propellant_loading` — the operational plan plus the EOS-derived venting decision

### Verification

- Methane Z near unity at low pressure; sub-unity near critical pressure
- LOX loading plan requires venting and chilldown, agreeing with the EOS decision
- Unknown propellants rejected with a typed error

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
