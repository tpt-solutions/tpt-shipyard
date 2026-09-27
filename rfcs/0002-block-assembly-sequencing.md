# RFC 0002: Block Assembly Sequencing

- **Number:** 0002
- **Title:** Hull block division and erection sequencing model
- **Status:** Accepted
- **Authors:** TPT Solutions
- **Created:** 2026-05-11
- **Review window:** closed 2026-05-25

## Summary

Defines how `tpt-yard-hull` divides a hull into erection blocks under yard
constraints, and how `tpt-yard-hull`, `tpt-yard-blocks`, `tpt-yard-launch`
and `tpt-yard-drydock` cooperate to plan the build: bottom-tier-first,
midship-outward erection on keel blocks with a live tipping check, and
launch screening (slipway statics, dock flooding) against golden reference
cases.

## Detailed Design

### Block division

The hull envelope is approximated pre-design as an areal steel density
`ρ_A` (kg/m²) over `2·(B + tier height)` per metre — the classic weight
estimate. The depth splits into `depth_bands` tiers of equal height. Block
length is the minimum of:

- the **crane limit**: `L ≤ 0.9·C_kn·1000/g / (ρ_A·(B + 2·h_tier))`
  (10 % rigging allowance);
- the **workshop length**.

`n_x = ceil(LOA / L_max)`; blocks get ids, tier/band indices, centre
positions, and design weights. Every block must satisfy both constraints —
enforced by tests and by the golden case
(`test-data/golden/sea/container-ship-block-division.json`).

### Erection sequence

Doctrine (bottom-up, centre-out):

1. **Keel tier first** — the vessel stands on keel blocks; no block is
   erected above an unsupported volume.
2. **Within a tier, from midship outwards** — keeps the growing weight
   centroid near midship (maximising keel-block reactions), and confines
   cumulative weld shrinkage away from block joints at the ends.
3. Each block lands against its nearest already-erected neighbour in the
   tier (butt seam); tier-0 first-of-tier lands on the dock floor
   (`DockJoint`).

The digital twin consumes this sequence as activities; the support check
(RFC 0001) rejects any erection step whose partial structure would tip.

### Lifting statics

`tpt-yard-blocks` owns the shared convention: two-point picks use the exact
lever rule; ≥3 points use inverse-distance weighting of the CoG offset
(statically indeterminate otherwise — sling stiffness data is not modelled
at this phase). Leg tension = share / sin(angle); angles below the 30°
practice guideline are flagged; the CoG projection must lie inside the lift
points.

### Launch screening

`tpt-yard-launch` implements classical end-launch statics: friction-limited
sliding (energy balance), way pressures (full contact → shrinkage →
way-end cribbing bearing `end_bearing_m`), tip-up as the race between
float-off travel and CoG crossing the way end, and water-entry slamming
(½ρv²·C_imp). Dock float-out delegates to `tpt-yard-drydock`'s
level-by-level GM sequence (`GM ≥ 0.15 m` screening, rectangular-block
hydrostatics: KB = d/2, BM = B²/12d).

## Verification

- Golden: `container-ship-block-division.json` (12 blocks, closed-form),
  `slipway-launch-stability.json` (velocity 8.729 m/s, pivot pressure
  0.49 MPa, no tip-up), `drydock-flooding-sequence.json` (13 levels, GM
  7.66 m afloat, 9 aground steps).
- Unit tests enforce the crane/workshop constraints, tier ordering,
  lever-rule shares, 45° sling tensions (50·√2 kN), tip-up detection, and
  capsize detection (KG above the metacentre).
- Milestone example: `examples/container-ship-block-assembly/` plans the
  full erection of a 1,400 TEU ship through the digital twin — 12 blocks,
  weight and CoG tracked, keel reactions positive at every step.

## Drawbacks

- Pre-design weight model: real block weights need the block-level weight
  items; division should re-run once weights firm up (supported: division is
  deterministic given geometry).
- Midship-outward is one of several valid doctrines (some yards build
  two-bow-first); the sequencer is doctrine-parameterised internally but
  exposes only the default now.
- Rectangular-block hydrostatics overestimate BM for fine forms; screening
  only.

## Alternatives Considered

- Optimisation-based sequencing (genetic/CP over join orders): deferred —
  the doctrine sequence is already within a few percent of optimal for
  conventional layouts, and the twin's tipping check guards correctness.
- Full hydrostatics from hull mesh: deferred to `tpt-earth`/`tpt-fem`
  integration.

## Unresolved Questions

- None blocking Phase 3. Side-launch transverse tipping physics is
  screening-only until a side-launch yard case demands more.
