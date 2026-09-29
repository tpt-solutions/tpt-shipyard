# tpt-yard-robotic-assembly

> Robotic arm kinematics and collision-free path planning for space assembly.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-robotic-assembly.svg)](https://crates.io/crates/tpt-yard-robotic-assembly)
[![Docs.rs](https://docs.rs/tpt-yard-robotic-assembly/badge.svg)](https://docs.rs/tpt-yard-robotic-assembly)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Free-flying manipulators need three competences: solve for joint angles, find a collision-free path, and plan the grasp. `tpt-yard-robotic-assembly` implements all three for serial revolute arms in a planar work area — the documented simplification (RFC 0003) that already validates the constraint machinery end to end.

Two-link arms get closed-form inverse kinematics; longer chains get Levenberg-Marquardt damped least squares with step clamping, full-turn range wrapping, and a deterministic seed sweep to escape local minima. Paths come from RRT with swept collision sampling.

## Features

- `forward_kinematics` / `inverse_kinematics` — closed form for two links, DLS refinement for longer chains, `None` for unreachable targets
- Range-aware solvers: revolute joints wrap modulo 2-pi, others clamp
- `plan_path` — RRT in joint space with circular obstacles, clearance inflation, and swept segment checks
- `MotionPlan` — Cartesian waypoints, joint trajectory, duration at the arm's velocity limits
- `grasp_planning` — side-face grasp poses and required grip force from the handling acceleration
- Typed `ArmError` for joint/link count mismatches

## Installation

```toml
[dependencies]
tpt-yard-robotic-assembly = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-robotic-assembly = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{RobotId, Vector3};
use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose, RoboticArm};

let arm = RoboticArm::new(
    RobotId(1),
    vec![
        Joint::revolute(0.0, 2.0 * std::f64::consts::PI, 1.0),
        Joint::revolute(0.0, 2.0 * std::f64::consts::PI, 1.0),
    ],
    vec![1.0, 1.0],
    EndEffector::Gripper { force_n: 500.0 },
);
let target = Pose { position: Vector3::new(1.0, 1.0, 0.0), yaw_rad: 0.0 };
let q = arm.inverse_kinematics(&target)?.expect("reachable");
let fk = arm.forward_kinematics(&q)?;
```

## How it works

- DLS damping is adaptive: high early for robust steps from bad seeds, relaxed near the solution; steps are clamped so Newton cannot throw the chain across the workspace.
- Local minima are a property of the problem, not a bug — the multi-seed sweep is the standard cure and is deterministic.
- The obstacle check samples points along every link, so swept motion cannot tunnel through a circle between waypoints.

## Verification

- IK-to-FK round trips on multiple targets; unreachable and mismatched cases typed
- Golden: `robotic-arm-path.json` — RRT finds a collision-free path reaching the goal within tolerance
- Grasp force vs gripper capability tested both ways

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `robotics` `inverse-kinematics` `rrt` `path-planning` `motion-planning` |
| Categories | `science::robotics` `algorithms` `aerospace` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-robotic-assembly` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
