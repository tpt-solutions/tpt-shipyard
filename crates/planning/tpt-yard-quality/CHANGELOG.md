# Changelog for tpt-yard-quality

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `generate_inspection_plan` — activity type drives the method: structural butts get UT at criticality I, piping gets PT at II, pressure tests get PRESS, everything else visual at III
- Coverage per point (100 % for pressure boundaries and full-penetration welds)
- `defect_tracking` — counts by type, dominant defect, repair rate, rejections to engineering
- `NdtMethod` with standard codes (UT, RT, MT, PT, ET, VT, VAC, PRESS)
- `AcceptanceCriteria` referencing the governing standard and maximum indication

### Verification

- Golden: `ndt-inspection-plan.json` — method/criticality/coverage per activity
- Golden: `weld-defect-tracking.json` — aggregation, dominant defect, repair rate
- Empty-tracking and boundary behaviours tested

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
