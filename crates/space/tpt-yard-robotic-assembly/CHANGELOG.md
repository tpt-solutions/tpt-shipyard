# Changelog for tpt-yard-robotic-assembly

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `forward_kinematics` / `inverse_kinematics` — closed form for two links, DLS refinement for longer chains, `None` for unreachable targets
- Range-aware solvers: revolute joints wrap modulo 2-pi, others clamp
- `plan_path` — RRT in joint space with circular obstacles, clearance inflation, and swept segment checks
- `MotionPlan` — Cartesian waypoints, joint trajectory, duration at the arm's velocity limits
- `grasp_planning` — side-face grasp poses and required grip force from the handling acceleration
- Typed `ArmError` for joint/link count mismatches

### Verification

- IK-to-FK round trips on multiple targets; unreachable and mismatched cases typed
- Golden: `robotic-arm-path.json` — RRT finds a collision-free path reaching the goal within tolerance
- Grasp force vs gripper capability tested both ways

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
