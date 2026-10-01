# Changelog


## [0.1.0](https://github.com/tpt-solutions/tpt-shipyard/releases/tag/tpt-yard-cli-v0.1.0) - 2026-10-01

### Added

- schedule and risk sections in html-report; frame module in the book
##
- `html-report` now carries Schedule (critical path, levelled makespan)
  and Schedule risk (Monte Carlo P50/P90/mean + most-critical activities)
  sections alongside weights and the structural check; `--out` is
  accepted in any argument position.
 0.1.0

- Initial CLI: validate, plan (end-to-end), schedule, report, new.
