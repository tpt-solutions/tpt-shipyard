# Changelog for tpt-yard-sea-trials

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `TrialProgram` — the ordered trial list with categories from speed to class acceptance
- `Acceptance` windows on typed metrics (knots, turning-circle lengths, dB(A), GM, ...) with inclusive bounds
- `evaluate` — per-trial outcomes: passed, failed with reason, or *not performed*
- `TrialReport` — delivery-facing summary with passed/total counts

### Verification

- Passing, failing and unperformed trials each tested
- Inclusive-bound edge values accepted exactly at the limit
- Inclining-experiment GM gate covered

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
