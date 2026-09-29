# Changelog for tpt-yard-scheduling

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `critical_path` — zero-float activities from a full CPM pass (earliest/latest starts, float per activity)
- `optimize_sequence` — objectives: minimum duration, cost, crane usage, drydock time, maximum parallelism
- `resource_leveling` — serial RCPSP placement: each activity at the earliest clash-free slot for its resources
- Honest levelling: may stretch the makespan to flatten peaks (golden case: 32 h -> 40 h for a crane peak of 2 -> 1)
- Peak concurrent demand per resource kind on a 1-hour grid

### Verification

- Golden: `critical-path-schedule.json` (34 h network, 4 h float branch)
- Golden: `resource-leveling.json` — crane peak 2 -> 1 at a makespan cost of 32 -> 40 h
- Cycle and empty-schedule error paths typed and tested

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
