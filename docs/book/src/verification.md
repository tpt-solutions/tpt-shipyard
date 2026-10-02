# Verification & Golden Data

Every numerical claim in this workspace is verified — against closed forms
where they exist, and against locked golden data otherwise.

## Golden data

`test-data/golden/` holds the locked reference cases; each has an
integration test that recomputes and compares within the file's tolerance:

| File | Verifies | Tolerance |
|---|---|---|
| `sea/welding-distortion-panel.json` | Rosenthal + residual stress + distortion chain | ±5 % |
| `sea/container-ship-block-division.json` | Crane/workshop-constrained division | closed form |
| `sea/slipway-launch-stability.json` | End-launch statics, tip-up, slamming | closed form |
| `sea/drydock-flooding-sequence.json` | Level-by-level GM stability | closed form |
| `space/rotating-habitat-stress.json` | `sigma = rho omega^2 r^2` | machine precision |
| `space/orbital-assembly-sequence.json` | Sequence + cantilever integrity | closed form |
| `space/iss-truss-assembly.json` | Linear root-stress growth law | closed form |
| `space/robotic-arm-path.json` | RRT path finding | feasibility |
| `planning/critical-path-schedule.json` | CPM makespan, float | closed form |
| `planning/resource-leveling.json` | Peak-flattening trade | discrete-exact |
| `quality/ndt-inspection-plan.json` | Criticality → method mapping | exact |
| `quality/weld-defect-tracking.json` | Defect aggregation | exact |

## Verification tests

Beyond golden files, the crates carry closed-form verification tests:

- single-bar FEM: `delta = FL/EA`; two-bar truss: `N = F/(2 sin theta)`;
  vertical bar self weight: `delta = WL/(2EA)`;
- CoG tracking through 10 phases against the weighted mean (RFC 0001
  milestone);
- `omega = sqrt(g/r)` and the 2 rpm comfort limit (RFC 0005);
- t8/5 scaling with heat input; distance independence of the 3D solution;
- plate FEM: Timoshenko 0.00406/0.00126 q a^4/D (BFS and MITC4 Mindlin on
  identical meshes), strain-free rigid/linear fields, and the twisting
  thin-limit mode carrying exactly the Kirchhoff curvature;
- capacity-aware levelling: parallel sharing vs deferral at exact
  breakpoints, and the whole-window regression (a start-only check would
  overrun the limit mid-activity);
- SOLAS probabilistic damage stability: the bi-linear damage-length
  density against exact hand fractions, `p` against Simpson integration of
  the contained-damage integral in both branches, `p(Jm) = Jm - E[J]`, and
  s-factor hand values;
- local scantlings: the 22.4 slab constant round-trips the clamped-plate
  relation, Euler `sigma_E(t_req) = sigma_applied` exactly, and the
  stiffener closed forms (p s l^2/12, p s l/2);
- IFC export: STEP well-formedness plus an exact (bit-identical)
  geometry round-trip through the emitted reals;
- SOLAS r/multi-zone/v/intermediate: r monotone in penetration depth
  and exactly 1 at B/2, group factors bounded by the union span, the
  s-factors monotone in GZmax/range and gated by heel, the v factor
  monotone in deck height, and the cross-flooding closed form against
  a brute-force ODE integration (proptest_damage, 128 cases each);
- the hull-stability-screen example exercises the full pipeline on the
  container-ship manifest: tank-plan damage screen, staged flooding
  with per-stage intermediate s, cross-flooding equalization, the
  multi-zone p·r group factor, and the plate/frame/EC3 buckling
  checks.

## Benchmarks

`cargo bench -p tpt-shipyard-benches` runs four benches: block lifting,
welding distortion, orbital assembly sequences, and launch stability. All
hot paths are microsecond-scale — interactive digital-twin budgets.

## Sample data

- `test-data/hull-blocks/` — reference hull definitions,
- `test-data/orbital-structures/` — truss and habitat manifests,
- `test-data/welding-procedures/` — sample WPS records (validated by test).
