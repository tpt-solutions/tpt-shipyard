# Changelog for tpt-yard-assembly

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `ActivityGraph` — a DAG of activities (id, name, duration, dependencies, status)
- Kahn topological sort with deterministic (BTreeSet-ordered) output
- Cycle detection via DFS colouring, returning the offending closed path
- Readiness queries (`is_ready`) against live activity statuses
- CPM forward pass: earliest finish times and project makespan
- Status lifecycle: `Pending -> InProgress -> Completed`, plus `Blocked` and `Cancelled`

### Verification

- Topological order across a diamond dependency network
- Cycle detection including a manually closed loop
- CPM forward pass against hand-computed finish times

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
