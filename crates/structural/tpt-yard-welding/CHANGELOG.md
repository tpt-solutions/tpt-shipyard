# Changelog for tpt-yard-welding

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [Unreleased]

### Added

- `advisor` module (review 7H): `SteelChemistry::cet`/`pcm`/`graville_class`,
  `preheat_for_target_t8_5` (the Rosenthal t8/5 closed form inverted for
  preheat by bisection), `advise` (end-to-end WPS screening against the
  preheat floor and a supplier t8/5 `CoolingWindow`, with a recommended
  preheat when the floor is missed), and `Pqr`/`WeldEnvelope` essential-
  variable checks (ISO 15614-1 Table 5 thickness rule 0.5·t-2·t, heat input,
  preheat/interpass, process and filler) covering ISO 15614-1 and ASME IX
  screening.

## [0.1.0] - 2026-09-28

### Added

- Rosenthal quasi-stationary 3D thermal cycle: temperature history, peak temperature, and the t8/5 cooling time at any distance from the seam
- Residual stress: parabolic tension zone bounded by the T_mech isotherm, balanced by uniform compression
- Distortion: transverse and longitudinal shrinkage, angular distortion (single-side vs balanced grooves), bowing
- Seven processes with arc efficiencies (SMAW through EBW and laser)
- Multi-pass superposition and candidate-sequence optimisation
- WPS record loading from JSON (`test-data/welding-procedures/`)

### Verification

- Golden: `test-data/golden/sea/welding-distortion-panel.json` at ±5 % (±10 % for t8/5)
- Peak decays monotonically with distance; heavier heat input lengthens t8/5
- Balanced X grooves halve angular distortion; sequence ranker prefers spread, alternating passes

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
