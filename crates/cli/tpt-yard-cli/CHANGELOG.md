# Changelog

##
- `pdf-report project.json [--out file.pdf]`: the full calculation
  package (weights, by-group, phases, schedule, risk, structural check)
  as a dependency-free PDF 1.4 document — uncompressed text streams,
  Helvetica/Courier, A4 pagination with margin-aware page breaks.

- `html-report --structure partial.json`: per-phase FEM table (members,
  max deflection, max stress, utilization, verdict per erection phase)
  alongside the schedule and risk sections.

- `html-report` now carries Schedule (critical path, levelled makespan)
  and Schedule risk (Monte Carlo P50/P90/mean + most-critical activities)
  sections alongside weights and the structural check; `--out` is
  accepted in any argument position.
 0.1.0

- Initial CLI: validate, plan (end-to-end), schedule, report, new.
