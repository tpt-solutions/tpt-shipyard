# Changelog for tpt-yard-weight

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [Unreleased]

### Added

- `WeightModel::monte_carlo_risk` (review 7H): weight-risk roll-up —
  triangular item-weight sampling, P50/P90/mean totals and P50/P90 CoG
  shifts, seed-deterministic.

## [0.1.0] - 2026-09-28

### Added

- `WeightModel` — all items plus the contractual design weight and design CoG
- Best-estimate vs installed vs design weight, with deviation in kg and %
- Best-estimate and installed-only centres of gravity (weighted means)
- Growth margins per item, aggregated for the weight report
- Group-by reports (by system or by zone), outstanding item counts
- `installed_by: ActivityId` wiring — the only path from predicted to as-built mass

### Verification

- `test_block_weight_sum`: ten blocks sum exactly to the vessel total
- CoG equals the closed-form weighted mean; replaced items never count
- Deviation, margin aggregation and report grouping unit-tested

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
