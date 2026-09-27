# Weight & CoG

`tpt-yard-weight` is the single source of truth for mass properties.

## The model

Every measurable mass is a [`WeightItem`](tpt_yard_weight::WeightItem): a
weight, a CoG, a group (system or zone), a growth margin, and an
[`ItemStatus`](tpt_yard_weight::ItemStatus):

```text
Design → Ordered → Received → Installed
                                 ↘ Replaced (history only, never counted)
```

The [`WeightModel`](tpt_yard_weight::WeightModel) carries the contractual
design target (`design_weight_kg`, `design_cog`) plus all items.

## Conventions (RFC 0001)

- `total_weight()` — best-estimate lightship: installed items at **as-built**
  mass, everything else at **predicted** mass. `Replaced` never counts.
- `installed_weight()` — physically in the ship to date.
- `weight_deviation()` — best estimate minus contractual design; positive
  means overweight.
- `centre_of_gravity()` / `installed_centre_of_gravity()` — weighted means
  over the respective sets.
- `weight_report()` — per-group totals, margins, outstanding item count.

## Wiring to the build

`WeightItem::installed_by: Option<ActivityId>` connects an item to the
activity that installs it. The digital twin flips `Design → Installed` when
that activity completes; as-built refinement stays explicit via
`WeightModel::mark_installed(id, as_built_kg)`.

## Verification

`test_block_weight_sum` builds a 10-block vessel (8 hull + 2 machinery
blocks) and verifies that the block weights sum exactly to the vessel total,
that installed vs design splits are correct, and that the CoG is the weighted
mean of the items.

## Milestone

Phase 1's milestone is the twin's `tests/ten_phase_tracking.rs`: a 1,800 t,
10-phase barge erected block by block — weight grows by exactly one block per
phase, the CoG matches the closed-form weighted mean at every step, and keel
block reactions stay positive throughout.
