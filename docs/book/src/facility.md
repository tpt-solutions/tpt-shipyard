# Facility Planning

`tpt-yard-facility` models the yard itself: cranes, workshops, docks,
quays, orbital bays and manufacturing cells as placed facilities with
capacities and footprints. Two checks drive the layout work:

- `check_capacity` — does peak demand fit? Crane capacities do **not**
  aggregate (a 1200 t and a 50 t crane cannot jointly lift 600 t); workshop
  area and berth slots do pool.
- `can_place` / `validate` — footprint collision checks with clearance,
  including vertical separation so elevated platforms do not clash with the
  pads beneath them.

> Status: plan-view layout with capacity screening; see the crate docs for
> the `Facility` schema.
