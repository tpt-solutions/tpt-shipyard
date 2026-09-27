# Orbital Assembly

`tpt-yard-orbital-assembly` plans and simulates building structures in
orbit. Model: [RFC 0003](../../rfcs/0003-orbital-assembly-planning.md).

## Planning

[`plan_sequence`](tpt_yard_orbital_assembly::OrbitalAssembly::plan_sequence)
orders components by target radius — anchored geometry first — and emits the
handling cycle per component: grasp, translate, rotate, dock, bolt, release.
Every step carries constraint tags (`NoCollision`, `ForceLimit`,
`MaintainStationKeeping`).

## Simulation

[`simulate_step`](tpt_yard_orbital_assembly::OrbitalAssembly::simulate_step)
checks, per step:

- **handling force** — `m·a` with station keeping + 0.05 m/s² allowance,
  against the step's force limit;
- **collision** — the moved component's bounding sphere (largest half-axis)
  swept along its approach path against installed components.

## Partial-structure integrity

The governing in-orbit load case for a deploying cantilever is a docking
impulse at the free end:

```text
sigma = P * L / (A_chord * h)
```

Root stress grows linearly with deployed length —
[`verify_structural_integrity`](tpt_yard_orbital_assembly::OrbitalAssembly::verify_structural_integrity)
reports it at every step, and the milestone example shows the growth across
a 6-bay build.

## Robots

Robotic arms come from `tpt-yard-robotic-assembly`:
[`RoboticArm`](tpt_yard_robotic_assembly::RoboticArm) provides forward and
inverse kinematics (closed-form two-link; Levenberg–Marquardt with seed
sweep for longer chains), RRT path planning around obstacles, and
grasp-point planning.

## Verification

- Collision verification: `test_orbital_assembly_collision`.
- Golden: `orbital-assembly-sequence.json`, `iss-truss-assembly.json`,
  `robotic-arm-path.json`.
- Milestone example: `examples/orbital-station-truss/` — 36 steps, all
  constraints green, integrity reported per bay.
