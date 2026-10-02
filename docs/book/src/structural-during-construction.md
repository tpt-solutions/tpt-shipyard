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

## Pressure-hull screening

[`cylinder_shell_screen`](tpt_yard_structural::shells::cylinder_shell_screen)
gives the curved-shell slice for circular pressure hulls: thin-wall
hoop `p·d/(2t)` and longitudinal `p·d/(4t)` stresses plus the
Windenburg–Trilling elastic external-pressure buckling pressure
`2.42·E·(t/d)^2.5/(1−ν²)^0.75`, against a stress allowable and a
buckling safety factor, reporting the salt-water crush depth.

```rust
use tpt_yard_structural::shells::cylinder_shell_screen;

let screen = cylinder_shell_screen(
    2.0,   // 2 MPa external lateral pressure (200 m depth)
    8.0,   // hull diameter, m
    0.045, // wall thickness, m (t/d = 1/178)
    210_000.0, 0.3, 400.0, 1.5,
)
.unwrap();
assert!(screen.buckling_pressure_mpa > 2.0);
assert!(screen.crush_depth_m > 200.0);
```

Full curved-shell finite elements (general shells, ring stiffeners,
inelastic knockdowns) remain roadmap.

Curved members: [`add_arc`](tpt_yard_structural::frame::FrameModel::add_arc)
appends a faceted circular arc (straight members along the arc — the
standard practice for curved frames); closed rings and arcs solve via
[`solve_dense`](tpt_yard_structural::frame::FrameModel::solve_dense),
whose dense Cholesky handles the self-equilibrated load cases where the
CG free-row residual test is blind to weakly restrained rigid modes.
The membrane verification (a ring under radial pressure carrying
exactly the hoop force pR with vanishing bending) doubles as the
transformation regression for inclined members.

## Launch screening

[`launch_analysis`](tpt_yard_structural::ConstructionStructuralSolver::launch_analysis)
screens launch load cases (way pressure against the 500 kPa class screening
limit). The full launch physics — sliding, tip-up, flooding sequences — is
`tpt-yard-launch`, Phase 3.

## Plate bending elements

Two plate theories share the axis-aligned rectangular mesh machinery in
`tpt-yard-structural`:

- [`PlateModel`](tpt_yard_structural::plates::PlateModel) — thin
  (Kirchhoff) plates with the C1-conforming Bogner-Fox-Schmit element
  (16 DOF per element: `w`, both slopes and the twist per corner).
  Verified against the Timoshenko closed forms: simply supported
  `0.00406 q a^4 / D`, clamped `0.00126 q a^4 / D`.
- [`MindlinModel`](tpt_yard_structural::mindlin::MindlinModel) —
  shear-deformable plates with the MITC4 assumed-strain element of
  Bathe & Dvorkin (`w`, `theta_x`, `theta_y` per node). The transverse
  shear is interpolated from its edge-midsides values, so the element
  is locking-free and converges to the same thin-plate values on the
  same meshes while remaining valid for thick plates (span/thickness
  down to ~5, where shear flexibility adds real deflection).

```rust
use tpt_yard_structural::mindlin::{MindlinModel, MindlinSupport};
use tpt_yard_structural::plates::{PlateElement, PlateNode};

// Square plate, 2 m side, 20 mm thick, uniform pressure.
let n = 8_usize;
let mut nodes = Vec::new();
for j in 0..=n {
    for i in 0..=n {
        nodes.push(PlateNode {
            position: (2.0 * i as f64 / n as f64, 2.0 * j as f64 / n as f64),
        });
    }
}
let idx = |i: usize, j: usize| j * (n + 1) + i;
let mut elements = Vec::new();
for j in 0..n {
    for i in 0..n {
        elements.push(PlateElement {
            nodes: [idx(i, j), idx(i + 1, j), idx(i + 1, j + 1), idx(i, j + 1)],
            thickness_m: 0.02,
            youngs_modulus_gpa: 210.0,
            poissons_ratio: 0.3,
            pressure_n_m2: 10_000.0,
        });
    }
}
let mut model = MindlinModel { nodes, elements, supports: Vec::new() };
for j in 0..=n {
    for i in 0..=n {
        if i == 0 || i == n || j == 0 || j == n {
            // "Soft" simply supported: deflection fixed, rotations free.
            model.supports.push(MindlinSupport::simply_supported(idx(i, j)));
        }
    }
}
let sol = model.solve().unwrap();
let centre = sol.deflections[idx(4, 4)].abs();
assert!(centre > 0.0);
```

The MITC4 midsides operators are verified by strain-free fields (rigid
and linear-slope states exert no force; the twisting thin-limit mode
carries exactly the Kirchhoff twisting curvature and no shear energy) —
the checks that caught a transverse-shear edge-orientation error during
development.
