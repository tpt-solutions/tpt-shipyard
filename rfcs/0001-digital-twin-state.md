# RFC 0001: Digital Twin State Model

- **Number:** 0001
- **Title:** Digital twin state model and project/twin ownership
- **Status:** Accepted
- **Authors:** TPT Solutions
- **Created:** 2026-01-12
- **Review window:** closed 2026-01-26

## Summary

Defines the state model of the construction digital twin: the twin *owns* its
`VesselProject` (breaking the cycle in the original design sketch where
`VesselProject` embedded a `DigitalTwin`), activity completion is the single
driver of state, weight items are wired to installing activities, and every
phase advance is gated by a support/tipping check of the *partial* structure.

## Motivation

The design sketch (spec §5) showed:

```rust
pub struct VesselProject { ..., pub digital_twin: DigitalTwin }
pub struct DigitalTwin { ..., pub vessel: VesselProject }
```

Both crates cannot depend on each other, and both types cannot own each other.
A decision is needed before any downstream crate can be built. Additionally,
the twin needed a precise answer to: *what makes the state advance?* The
sketch left `advance_phase(&mut self, activity)` under-specified — what
happens to weight, geometry, and structural state when one activity completes?

## Detailed Design

### Ownership

`tpt-yard-digital-twin` depends on `tpt-yard-core`, `tpt-yard-weight`, and
`tpt-yard-assembly`. `DigitalTwin` owns the `VesselProject` by value;
`VesselProject` carries **no** twin reference. Dashboards that need both take
the twin and use accessors (`twin.vessel`, `twin.weight_model()`).

### Drivers of state

The twin's state changes only through `advance_phase(&ActivityId)`:

1. **Validation (no mutation on failure).**
   - The activity must exist in the project's activity network
     (`tpt-yard-assembly::ActivityGraph`).
   - It must not already be completed.
   - Every dependency must be completed; otherwise
     `TwinError::DependenciesNotComplete` lists the blockers.
2. **Support check (dry run).** The hypothetical installed set is
   `Installed ∪ items(wired to this activity)`. The partial structure is
   treated as a rigid body on its [`SupportCondition`] (keel blocks, slipway
   ways, floating, or orbital). Longitudinal tipping is checked by rigid-body
   statics on the extreme supports: a negative reaction means the structure
   would tip, and the advance is refused with `TwinError::UnsoundStructure`.
   Transversally, the CoG must stay within the block half-track. This is the
   "half-built ship must not collapse" rule from the spec, in its simplest
   sound form; FEM-level checks belong to `tpt-yard-structural` (Phase 2).
3. **Commit.** Mark the activity completed; flip wired weight items to
   `Installed`; recompute `AssemblyState::current_mass_properties` from the
   installed subset; advance the phase pointer when the current phase's last
   activity completes; update `StructuralModel::connected_fraction`.

### Weight wiring

`WeightItem` gains one field beyond the spec sketch:
`installed_by: Option<ActivityId>`. This is the *only* mechanism by which
design mass becomes as-built mass. As-built refinement (weighing blocks)
remains explicit via `WeightModel::mark_installed`.

### Status totals

`WeightModel::total_weight()` counts every non-`Replaced` item — installed
items at as-built mass, the rest at predicted mass. This matches the
master-plan convention that the lightship estimate is complete even while
erection is in progress; the *as-built* figure is
`installed_weight()`. Deviation reporting compares both against the
contractual design weight.

### CoG tracking

`centre_of_gravity_tracking()` returns one cumulative `CoGSample` per phase
that has completed activities, in phase order: the (weight, CoG) curve a
launch officer or orbital-integration planner consumes. Phase boundaries, not
individual activities, are the natural reporting granularity (50+ activities
per phase would swamp the curve).

### Status sets

`AssemblyState` keeps `completed`/`in_progress`/`blocked` sets, matching the
sketch. `advance_phase` mutates only `completed`; the other two are managed by
planner front-ends (`start_activity`, `block_activity` manipulate the sets
directly and are intentionally cheap, non-validating operations).

## Verification

- `test_block_weight_sum` (tpt-yard-weight): item weights sum to the vessel
  total.
- `test_cog_tracking` (tpt-yard-digital-twin): 10 sequential block erections;
  the CoG marches monotonically and matches the closed-form weighted mean at
  every step.
- Milestone integration test `tests/ten_phase_tracking.rs`: 10 build phases
  end-to-end — weight growth, CoG curve, deviation, and support reactions all
  verified against analytic values.

## Drawbacks

- `installed_by` couples the weight model to the activity namespace; a twin
  without an activity plan uses `None` and manual `mark_installed`.
- Refusing to advance on `UnsoundStructure` pushes responsibility to callers
  to relocate temporary supports before retrying; we consider that the
  correct (safe) default.

## Alternatives Considered

- **Twin inside the project** (spec sketch): creates the mutual-ownership
  cycle; rejected.
- **Event sourcing** (every mutation is an event): more auditable, but
  over-engineered for Phase 1; the completed-activities set is already an
  event log in all but name. Revisit if audit requirements harden.
- **Continuous mass properties from geometry** (no weight items): attractive
  long-term, but yards run on itemised weight reports; geometry-derived mass
  is a refinement, not a replacement.

## Unresolved Questions

- None blocking Phase 1. Transverse support modelling (side blocks with
  unequal stiffness) is deferred to the `tpt-yard-structural` integration.
