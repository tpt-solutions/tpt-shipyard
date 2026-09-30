# Digital Twin

`tpt-yard-digital-twin` tracks the construction state of a vessel: which
activities are complete, what mass is installed, where the centre of gravity
is — and whether the *partially built* structure can stand on its supports at
every stage.

## Ownership

The twin **owns** the `VesselProject` (RFC 0001); the project carries no
back-reference. This breaks the mutual-ownership cycle in the original design
sketch and keeps the dependency direction one-way:
`digital-twin → core, weight, assembly`.

## Advancing construction

Everything flows through `advance_phase(&ActivityId)`:

1. The activity must exist, must not be complete, and all its dependencies
   must be complete.
2. **Dry-run support check** — the twin computes the hypothetical installed
   weight set and verifies the partial structure stands on its
   [`SupportCondition`](tpt_yard_digital_twin::SupportCondition):
   - `KeelBlocks { positions }` — rigid-body statics on the extreme supports;
     a negative reaction (tip) refuses the advance.
   - `Slipway { fore_poppet_x, aft_way_x }` — CoG must stay over the ways.
   - `Floating` / `Orbital` — no tipping check (buoyancy / microgravity).
3. On success the twin commits: items wired to the activity
   (`WeightItem::installed_by`) become `Installed`, mass properties update,
   the phase pointer advances when the phase completes.

```rust,ignore
twin.advance_phase(&ActivityId(3))?; // Err(UnsoundStructure) if it would tip
```

## Weight & CoG reporting

- `weight_deviation()` — as-built installed vs best estimate vs design.
- `centre_of_gravity_tracking()` — the cumulative (weight, CoG) curve per
  phase: the data a launch officer or orbital integrator consumes.
- `structural_check_at_phase(&PhaseId)` — supports check projected at the end
  of any phase, past or future.

## Bookkeeping

`QualityRecord`s (inspection outcomes) and `SensorReading`s (metrology,
strain, temperature) attach to the twin; the Phase 5 quality crate builds its
inspection plans on top of them.

## Worked example

The [Weight & CoG] chapter walks a 10-phase barge build; the crate's
`tests/ten_phase_tracking.rs` is the executable milestone: 10 phases, weight
and CoG verified against closed-form values, support reactions positive at
every stage.
