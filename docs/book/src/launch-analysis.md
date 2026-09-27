# Launch Analysis

`tpt-yard-launch` computes how the vessel gets into the water, and
`tpt-yard-drydock` plans the dock flooding that precedes a float-out.
Model: RFC 0002.

## Slipway end launch

[`slipway_launch`](tpt_yard_launch::LaunchAnalysis::slipway_launch)
implements the classical statics chain:

1. **Sliding** — energy balance down the ways at slope θ with grease
   friction μ: `v² = 2 g s (sinθ − μ cosθ)`; sticky ways (μ ≥ tanθ) are
   flagged as "will not launch".
2. **Way pressures** — full contact `P = W cosθ/(L·b)`, rising as contact
   shrinks, ending at the way-end cribbing pressure
   `W cosθ/(end_bearing·ways·b)` checked against the 0.5 MPa class
   screening limit.
3. **Tip-up** — the race between float-off travel (draft ÷ sinθ) and the
   CoG crossing the way end; a tip-up refuses the launch.
4. **Water entry** — entry angle = way slope; slamming
   `p = ½ρv²·C_imp`.

## Drydock flooding

[`Drydock::flooding_sequence`](tpt_yard_drydock::Drydock::flooding_sequence)
steps the dock water level from floor to float-out, and at every level
reports draft, displaced mass, and GM (rectangular-block hydrostatics:
`GM = KB + BM − KG`, `BM = B²/12d`) against the 0.15 m minimum. A vessel
that is unstable at *any* intermediate level fails the plan.

## Post-launch stability

[`launch_stability`](tpt_yard_launch::LaunchAnalysis::launch_stability)
checks GM at the launch condition using the as-built weight model — the
vessel in its partial outfitting state, not its design state.

## Verification

- Golden: `test-data/golden/sea/slipway-launch-stability.json`
  (8.729 m/s entry, 0.163 → 0.490 MPa way pressures, no tip-up) and
  `drydock-flooding-sequence.json` (13 levels, 9 aground steps, final GM
  7.66 m).
- Unit tests: tip-up detection (CoG 40 m case), sticky ways, undersized
  cribbing, capsize with KG above the metacentre.
- Example: `examples/drydock-flooding-sequence/` prints the full
  level-by-level table.
