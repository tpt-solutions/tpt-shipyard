# Outfitting

`tpt-yard-outfitting` plans the installation of the vessel's systems —
piping, electrical, HVAC, machinery — and checks the routes for clashes.

## Collision detection

[`OutfittingPlan::collision_detection`](tpt_yard_outfitting::OutfittingPlan::collision_detection)
takes each route's bounding box (grown by half the cross-section) and
reports:

- **route vs route** overlaps (system vs system clash),
- **route vs hull** breaches (any part of a route outside the hull
  envelope).

Each [`Collision`](tpt_yard_outfitting::Collision) names the participants,
an approximate location, and the kind.

## Installation sequence

[`installation_sequence`](tpt_yard_outfitting::OutfittingPlan::installation_sequence)
orders systems by size, descending — the "big items in first" doctrine of
pre-outfitted block construction: machinery before ducting, ducting before
cable trays, cable trays before small piping.

## Systems

`OutfitSystem` is defined in `tpt-yard-core` (it tags outfit activities);
the outfitting crate adds the routing and clash layer on top.
