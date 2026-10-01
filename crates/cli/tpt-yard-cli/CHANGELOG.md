# Changelog

##
- `html-report --structure partial.json`: per-phase FEM table (members,
  max deflection, max stress, utilization, verdict per erection phase)
  alongside the schedule and risk sections.

- `html-report` now carries Schedule (critical path, levelled makespan)
  and Schedule risk (Monte Carlo P50/P90/mean + most-critical activities)
  sections alongside weights and the structural check; `--out` is
  accepted in any argument position.
 0.1.0

- Initial CLI: validate, plan (end-to-end), schedule, report, new.
