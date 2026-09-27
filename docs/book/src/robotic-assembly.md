# Robotic Assembly

`tpt-yard-robotic-assembly` provides the kinematics and motion planning for
assembly robots (documented simplification: serial revolute arms with
parallel axes in a planar work area — RFC 0003).

## Kinematics

- [`forward_kinematics`](tpt_yard_robotic_assembly::RoboticArm::forward_kinematics) —
  pose of the end-effector for joint angles.
- [`inverse_kinematics`](tpt_yard_robotic_assembly::RoboticArm::inverse_kinematics) —
  closed form for two-link arms; for longer chains, damped least squares
  (Levenberg–Marquardt): step clamping, full-turn range wrapping, and a
  deterministic seed sweep to escape local minima.

## Path planning

[`plan_path`](tpt_yard_robotic_assembly::RoboticArm::plan_path) runs RRT in
joint space with circle obstacles in the work plane; every segment is
sample-checked against obstacles inflated by the requested clearance. The
resulting [`MotionPlan`](tpt_yard_robotic_assembly::MotionPlan) carries
Cartesian waypoints, the joint trajectory, and the duration at the arm's
velocity limits.

## Grasping

[`grasp_planning`](tpt_yard_robotic_assembly::RoboticArm::grasp_planning)
picks side-face approach poses from the component's bounding box and computes
the required grip force from the handling acceleration; feasibility is
checked against the end-effector's capability.

## Verification

- IK ↔ FK round trips on multiple targets; unreachable targets return
  `None`; joint-range violations are rejected.
- RRT paths around obstacles are collision-free by construction and
  verified by the golden case (`robotic-arm-path.json`).
