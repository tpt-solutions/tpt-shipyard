# Structural Analysis During Construction

`tpt-yard-structural` analyses the structure *as it exists at a build phase* —
the load cases no in-service tool sees: partial welds, crane picks, temporary
supports, launching ways.

## The staging model

A [`PartialStructure`](tpt_yard_structural::PartialStructure) lists every
member with the phase at which it becomes load-bearing
([`PartialElement::erected_at`](tpt_yard_structural::PartialElement)). At
analysis time the solver assembles the sub-model of members erected at or
before the requested phase. Missing members are not approximations — a
phase whose staging is a *mechanism* is refused outright:

```rust,ignore
let r = solver.analyze_at_phase(PhaseId(1), &[ConstructionLoad::Gravity]);
// Err(SingularSystem): the boom has no diagonal yet
```

## Loads

[`ConstructionLoad`](tpt_yard_structural::ConstructionLoad) covers the
construction-phase set from the spec: `Gravity` (member self weight),
`Wind`, `CraneLoad` (with dynamic factor), `Hydrostatic`, `WaveSlamming`,
`Docking`, `RoboticArm`, and `WeldingThermal` (bookkept here, resolved by
the welding crate).

Results ([`StructuralResult`](tpt_yard_structural::StructuralResult)) report
max displacement, max axial stress, and utilization against the allowable.

## The FEM substrate

The crate vendors a minimal 3D truss solver
([`fem::TrussModel`](tpt_yard_structural::fem::TrussModel)): direct stiffness
method, penalty boundary conditions, dense elimination. It is verified
against closed forms:

- single axial bar: `δ = FL/EA` (with the documented penalty compliance),
- two-bar truss: `N = F/(2 sinθ)`,
- vertical bar self weight: `δ = WL/(2EA)`.

When the `tpt-fem` substrate publishes, the swap is mechanical — the staging
concepts (`PartialStructure`, phase-gated members) are independent of the
solver.

## Lifting analysis

[`lifting_analysis`](tpt_yard_structural::ConstructionStructuralSolver::lifting_analysis)
computes crane-pick statics: sling leg loads from lift-point geometry
(exact lever rule for two-point picks), angles from the hook height, tip
check of the CoG projection against the lift points, and utilization against
sling and crane capacity. A pick that would tip is `safe: false` with the
reason in `notes`.

## Launch screening

[`launch_analysis`](tpt_yard_structural::ConstructionStructuralSolver::launch_analysis)
screens launch load cases (way pressure against the 500 kPa class screening
limit). The full launch physics — sliding, tip-up, flooding sequences — is
`tpt-yard-launch`, Phase 3.
