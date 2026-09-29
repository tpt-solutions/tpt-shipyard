# Changelog for tpt-yard-space-manufacturing

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `print_time_estimate` — feedstock mass (with ISRU scrap) over the deposition rate
- Process energy intensities from powder-bed (12 kWh/kg) to wire-arc (4 kWh/kg)
- `thermal_control_during_print` — deposition power and radiator area from the Stefan-Boltzmann law (eps 0.85, 350 K), eclipse pauses flagged
- `quality_verification` — 100 % layer imaging, vacuum NDT subset, witness coupons, destructive testing only if samples come home
- `Feedstock` from Earth launch or ISRU sources (lunar/martian regolith, asteroid metal, recycled debris)

### Verification

- Print time equals mass over rate, including the ISRU scrap factor
- Radiator area matches the closed-form Stefan-Boltzmann result
- ISRU builds flagged non-destructive-testing; error paths covered

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
