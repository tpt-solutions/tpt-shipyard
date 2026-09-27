# Hull Blocks

`tpt-yard-hull` turns a hull form into an erection plan. Sequencing model:
[RFC 0002](../../rfcs/0002-block-assembly-sequencing.md).

## Block division

[`HullConstruction::block_division`](tpt_yard_hull::HullConstruction::block_division)
splits the hull into blocks under the yard's real constraints:

- every block liftable: weight ≤ crane capacity − 10 % rigging allowance;
- every block fits the workshop (length, height);
- the depth splits into equal tiers (`depth_bands`).

The pre-design weight model uses an areal steel density over the moulded
surface — deterministic, auditable, and replaced by itemised weights as the
digital twin fills in.

## Erection sequence

[`erection_sequence`](tpt_yard_hull::HullConstruction::erection_sequence)
orders the joins:

1. keel tier first (the vessel stands on keel blocks),
2. within a tier from midship outwards (weight centroid stays near midship),
3. each block lands against its nearest erected neighbour; the first
   tier-0 block lands on the dock floor.

The sequence feeds the digital twin as activities — its tipping check (RFC
0001) refuses any step that would tip the partial structure.

## Lifting primitives

`tpt-yard-blocks` provides the shared statics:
[`distribute_load_shares`](tpt_yard_blocks::distribute_load_shares) (lever
rule for 2 points, inverse-distance beyond),
[`sling_loads`](tpt_yard_blocks::sling_loads) (leg tensions with angles),
and the pick-time tipping check
([`cog_within_lifts`](tpt_yard_blocks::cog_within_lifts)).

## Verification

- Golden reference: `test-data/golden/sea/container-ship-block-division.json`
  — the 140 m container ship divides into 12 blocks of 23.33 m / 142.8 t.
- Constraint tests: every block under the crane limit, inside the
  workshop; tighter cranes produce more blocks.
- Order tests: tiers never interleave; midship distance non-decreasing
  within a tier; first join is a dock joint.

The milestone example `examples/container-ship-block-assembly/` runs the
whole chain — division, twin advance, weight/CoG curve, keel reactions.
