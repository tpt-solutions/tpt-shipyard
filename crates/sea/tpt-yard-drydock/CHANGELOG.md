# Changelog for tpt-yard-drydock

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [Unreleased]

### Added

- `ballast_plan` (review 7H): keel-line ballast sequencing for undocking —
  equal-increment fills until the float-off GM reaches the target, steps
  emitted pre-flood at level 0, `InsufficientBallast` when the tanks cannot
  deliver. `DockedVessel` gains `lcg_from_midship_m` and typed
  `BallastTank`s (name, capacity, longitudinal position).
- `keel_reaction_distribution` (review 7H): per-block keel reactions from
  Archimedes + the moment balance, linear pressure law over the row, the
  single-end lift-off condition (reaction centroid past L/6) flagged and
  the pressures clamped/renormalised.

## [0.1.0] - 2026-09-28

### Added

- `Drydock::flooding_sequence` — level-by-level state: draft, displacement, GM, hours of flooding
- Aground/afloat transitions with keel-block load sharing reported per level
- Rectangular-block hydrostatics: `GM = KB + BM - KG`, `BM = B^2/(12 d)`
- Fit checks: breadth, length and float-out draft against the dock
- Total flooding time from the dock volume and pump rate

### Verification

- Golden: `drydock-flooding-sequence.json` — 13 levels, 9 aground steps, final GM 7.66 m
- Capsize test: KG above the metacentre yields negative GM
- Too-wide and too-deep vessels rejected with typed errors

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
