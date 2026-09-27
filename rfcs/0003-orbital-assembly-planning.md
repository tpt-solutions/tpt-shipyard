# RFC 0003: Orbital Assembly Planning

- **Number:** 0003
- **Title:** Orbital assembly sequence planning, simulation, and integrity
- **Status:** Accepted
- **Authors:** TPT Solutions
- **Created:** 2026-08-17
- **Review window:** closed 2026-08-31

## Summary

`tpt-yard-orbital-assembly` plans a component-by-component assembly sequence
(anchor first, build outwards), simulates every step against constraint
tags (collision, handling force, station keeping), and verifies the
*partially assembled* structure against docking impulses — the governing
in-orbit load case for a deploying cantilever. Robot kinematics and
collision-free motion come from `tpt-yard-robotic-assembly`.

## Detailed Design

### Sequence planning

`plan_sequence()` orders components by target-position radius (anchored
geometry first), then emits the standard handling cycle per component:

```text
GraspComponent → Translate → Rotate → Dock → Bolt(75 N·m) → Release
```

Each step carries constraint tags (`NoCollision`, `ForceLimit(400 N)`,
`MaintainStationKeeping`). The planner is deterministic; time/energy
optimisation across sequences is delegated to Phase 5 scheduling.

### Step simulation

- **Handling force**: microgravity handling accelerates the component at
  station keeping + 0.05 m/s² manoeuvre allowance; the gripper force is
  `m·a` and must respect the step's force limit.
- **Collision**: the moved component's bounding sphere (largest half-axis)
  is swept along its approach path against installed components' spheres.
  Half-axis rather than half-diagonal — the diagonal makes adjacent
  same-pitch bays overlap spuriously.
- **Docking**: the impulse passes through shock absorbers; the force tag is
  informational at this level, structural consequences are checked by the
  integrity verifier below.

### Partial-structure integrity

No gravity in orbit — but the stack must survive docking impulses at the
free end. The governing screening case is a tip load `P` on a cantilever of
deployed length `L` with constant cross-section (chord area `A`, bay height
`h`):

```text
sigma = P * L / (A * h)
```

Root stress grows **linearly with deployed length** — the real deployment
risk and the reason assembly order (anchored bays first) matters. Golden
reference: `test-data/golden/space/iss-truss-assembly.json` (15 bays,
5 m pitch: 48.3 MPa at 72.5 m, util 19.3%).

### Robot kinematics

`tpt-yard-robotic-assembly` models a serial revolute arm with parallel
axes: closed-form IK for two links, damped least-squares (Levenberg–
Marquardt with step clamping, range wrapping, and deterministic seed sweep)
for longer chains, RRT path planning with circle obstacles, and grasp-point
planning from component geometry. Documented simplification: planar
work-area analysis per arm; full 6-DOF kinematics is future work.

## Verification

- `test_orbital_assembly_collision`: a mis-planned bay sharing a target
  with an installed bay is caught by the sweep.
- Golden: `orbital-assembly-sequence.json` (18 steps, 12.3 h, 16.67 MPa
  root stress) and `iss-truss-assembly.json` (linear growth law).
- Golden: `robotic-arm-path.json` (RRT finds a collision-free path; the
  goal is reached within tolerance).
- Milestone example: `examples/orbital-station-truss/` simulates a 6-bay
  build end-to-end — 36 steps, every constraint green, root stress
  reported at every bay release.

## Drawbacks

- Bounding-sphere collision screening; exact mesh contact is out of scope
  without a collision library.
- Planar arm model; real free-flying manipulators are 6-7 DOF.
- Sequence planning is doctrinal, not optimised.

## Alternatives Considered

- Full 6-DOF kinematics now (fetches heavy math); rejected — the staging
  logic is independent and the planar model already validates the
  constraint machinery.
- Sampling-based task planning (PDDL-like); deferred to Phase 5.

## Unresolved Questions

- Multi-arm coordination (shared workspace, scheduling) — Phase 5.
