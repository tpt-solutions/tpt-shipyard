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
- t8/5 scaling with heat input; distance independence of the 3D solution.

## Benchmarks

`cargo bench -p tpt-shipyard-benches` runs four benches: block lifting,
welding distortion, orbital assembly sequences, and launch stability. All
hot paths are microsecond-scale — interactive digital-twin budgets.

## Sample data

- `test-data/hull-blocks/` — reference hull definitions,
- `test-data/orbital-structures/` — truss and habitat manifests,
- `test-data/welding-procedures/` — sample WPS records (validated by test).
