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

## Plane frames

`tpt_yard_structural::frame` adds 2-node Euler-Bernoulli beam members
alongside the truss elements: three DOF per node (axial, transverse,
rotation), full local-to-global transformation, consistent uniform-load
vectors, and member end-force recovery — solved by the same penalized
sparse CG solver. Because the Hermite-cubic shape functions are the exact
solution for end loads, one element reproduces the classical closed forms
exactly at the nodes:

```rust
use tpt_yard_structural::frame::{
    FrameElement, FrameLoad, FrameModel, FrameNode, FrameSupport,
};

// A 4 m cantilever (210 GPa, 50 cm2, 8e4 cm4) with a 50 kN tip load.
let model = FrameModel {
    nodes: vec![
        FrameNode { position: (0.0, 0.0) },
        FrameNode { position: (4.0, 0.0) },
    ],
    elements: vec![FrameElement {
        nodes: [0, 1],
        area_m2: 0.05,
        inertia_m4: 8.0e-5,
        youngs_modulus_gpa: 210.0,
        density_kg_m3: 0.0,
    }],
    supports: vec![FrameSupport::fixed(0)],
    loads: vec![FrameLoad { node: 1, fx: 0.0, fz: -50_000.0, moment_nm: 0.0 }],
    member_loads: vec![],
};
let sol = model.solve().unwrap();

// Exact against P L^3 / 3 EI = 15.87 mm.
let p = 50_000.0;
let ei = 2.1e11 * 8.0e-5;
let expected = p * 4.0_f64.powi(3) / (3.0 * ei);
assert!((sol.displacements[1].1 + expected).abs() < 1e-6 * expected);
// Root moment P L, recovered from the element end forces.
let (_n, _v, m1, _m2) = sol.member_forces[0];
assert!((m1 - p * 4.0).abs() < 1e-6 * (p * 4.0));
```

Member end forces come back as `(axial, shear, M1, M2)` with tension
positive and moments positive sagging; uniform member loads carry their
fixed-end offsets so simple supports recover zero end moments.

## Launch screening

[`launch_analysis`](tpt_yard_structural::ConstructionStructuralSolver::launch_analysis)
screens launch load cases (way pressure against the 500 kPa class screening
limit). The full launch physics — sliding, tip-up, flooding sequences — is
`tpt-yard-launch`, Phase 3.
