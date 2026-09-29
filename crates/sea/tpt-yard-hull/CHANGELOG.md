# Changelog for tpt-yard-hull

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `HullConstruction::block_division` — crane- and workshop-constrained division into tiers and longitudinal bands
- Pre-design weight model from areal steel density (deterministic, auditable)
- `erection_sequence` — bottom-tier-first, midship-outward joins with seam typing (dock joint / butt seam)
- `HullBlock` lifecycle status: Design through Cutting, Welding, Outfitting, Ready, Erected
- Total steel weight and per-block load reporting

### Verification

- Golden: `container-ship-block-division.json` — 12 blocks of 23.33 m / 142.8 t, closed form
- Constraint tests: every block under the crane limit and inside the workshop; tighter cranes yield more blocks
- Order tests: tiers never interleave, midship distance non-decreasing, first join a dock joint

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
