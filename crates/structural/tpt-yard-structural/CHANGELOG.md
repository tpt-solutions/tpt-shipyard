# Changelog for
- `plates` module (review 7H roadmap): Bogner-Fox-Schmit 16-DOF
  rectangular plate bending elements — C1 conforming via Hermite tensor
  products, consistent pressure vectors, penalty BCs over a dense
  Cholesky solve (mixed w/slope/twist scales exceed CG's iteration
  budget). Verified against the Timoshenko closed forms (SS 0.00406,
  clamped 0.00126 q a^4/D) and an SPD probe. The SPD probe caught a real
  element bug before any plate solved: the twist-derivative (N_xy) terms
  of the wx/wy/wxy DOFs initially used the value-function first
  derivatives instead of the slope-function ones.

- Strict JSON loading for staged structures: `from_json_with_loads` parses
  nodes/elements/supports plus a `loads` block (gravity flag defaulting
  true, wind and crane specs) with the review-7C loader standards — no
  silent defaults, dangling node indices rejected, wrong types reported.

- `frame` module (review 7H roadmap): plane frame elements — 2-node
  Euler-Bernoulli members with axial + bending stiffness, full local-to-
  global transformation, consistent uniform-load vectors, and member end
  force recovery (axial, shear, sagging-positive end moments) over the
  shared penalized sparse CG solver. Verified against the classical
  closed forms (cantilever, simply supported, uniform load, axial,
  slender inclined triangle).
 tpt-yard-structural

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `PartialStructure` — every member tagged with its erection phase
- `analyze_at_phase` — staged truss analysis under construction loads: gravity, wind, crane picks (with dynamic factor), hydrostatic, slamming, docking, robotic reactions
- `fem::TrussModel` — dependency-free 3D truss solver: direct stiffness, penalty BCs, dense elimination; swap-ready for the `tpt-fem` substrate
- `lifting_analysis` — crane-pick statics: sling loads by lever rule, angles, tip check, sling and crane utilization
- `launch_analysis` — way-pressure screening against the 500 kPa class limit
- Refuses mechanisms with `FemError::SingularSystem` instead of returning nonsense

### Verification

- Single axial bar vs `delta = FL/EA`; two-bar truss vs `N = F/(2 sin theta)`; vertical bar self weight vs `delta = WL/(2EA)`
- Staged erection test: the open frame is a mechanism at phase 1, the closed triangle solves at phase 2
- Crane overload and slipway over-pressure paths flagged as `passed: false`

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
