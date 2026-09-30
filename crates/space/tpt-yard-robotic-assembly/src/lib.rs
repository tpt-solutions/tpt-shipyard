//! Robotic arm kinematics and collision-free path planning for space
//! assembly.
//!
//! A [`RoboticArm`] is a serial chain of revolute joints with parallel axes
//! (the standard planar arm used for truss-panel handling; documented
//! simplification — RFC 0003). It provides:
//!
//! - forward/inverse kinematics (closed form for two links, damped
//!   least-squares for longer chains),
//! - RRT path planning among circular obstacles in the work plane,
//! - grasp-point planning on box-like components.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::Vector3;
//! use tpt_yard_robotic_assembly::{
//!     EndEffector, Joint, JointType, Pose, RoboticArm,
//! };
//!
//! let arm = RoboticArm::new(
//!     tpt_yard_core::RobotId(1),
//!     vec![
//!         Joint::revolute(0.0, 2.0 * std::f64::consts::PI, 1.0),
//!         Joint::revolute(0.0, 2.0 * std::f64::consts::PI, 1.0),
//!     ],
//!     vec![1.0, 1.0], // link lengths, m
//!     EndEffector::Gripper { force_n: 500.0 },
//! );
//!
//! let target = Pose { position: Vector3::new(1.0, 1.0, 0.0), yaw_rad: 0.0 };
//! let ik = arm.inverse_kinematics(&target).expect("solvable").expect("reachable");
//! let fk = arm.forward_kinematics(&ik).unwrap();
//! assert!((fk.position.x - 1.0).abs() < 1e-6);
//! assert!((fk.position.y - 1.0).abs() < 1e-6);
//! ```

use std::fmt;

use tpt_yard_core::{RobotId, Vector3};

/// The pose of a body in the work plane: position plus yaw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// Position of the pose origin, m.
    pub position: Vector3,
    /// Yaw about the work-plane normal, rad.
    pub yaw_rad: f64,
}

impl Pose {
    /// Pose at the origin facing +X.
    pub fn origin() -> Self {
        Self {
            position: Vector3::ZERO,
            yaw_rad: 0.0,
        }
    }

    /// Distance between two poses' positions.
    pub fn distance(&self, other: &Pose) -> f64 {
        self.position.distance(other.position)
    }
}

/// Joint categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointType {
    /// Revolute joint.
    Revolute,
    /// Prismatic joint.
    Prismatic,
}

/// One arm joint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    /// Joint category.
    pub joint_type: JointType,
    /// Motion range, rad (revolute) or m (prismatic).
    pub range: (f64, f64),
    /// Maximum velocity, rad/s or m/s.
    pub max_velocity: f64,
    /// Maximum torque, N·m (revolute) or N (prismatic).
    pub max_torque_nm: f64,
}

impl Joint {
    /// A revolute joint with the given range and velocity limit.
    pub fn revolute(min: f64, max: f64, max_velocity: f64) -> Self {
        Self {
            joint_type: JointType::Revolute,
            range: (min, max),
            max_velocity,
            max_torque_nm: 120.0,
        }
    }
}

/// End-effectors for space assembly.
#[derive(Debug, Clone, PartialEq)]
pub enum EndEffector {
    /// Parallel gripper with a gripping force.
    Gripper {
        /// Grip force, N.
        force_n: f64,
    },
    /// Welding head.
    Welder {
        /// Welding process name (e.g. "EBW").
        process: String,
    },
    /// Drill.
    Drill,
    /// Camera / inspection head.
    Camera,
    /// Multi-tool.
    MultiTool,
}

impl fmt::Display for EndEffector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EndEffector::Gripper { force_n } => write!(f, "gripper ({force_n} N)"),
            EndEffector::Welder { process } => write!(f, "welder ({process})"),
            EndEffector::Drill => f.write_str("drill"),
            EndEffector::Camera => f.write_str("camera"),
            EndEffector::MultiTool => f.write_str("multi-tool"),
        }
    }
}

/// A circular obstacle in the work plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionObject {
    /// Centre, m.
    pub centre: Vector3,
    /// Radius, m.
    pub radius: f64,
}

/// Result of path planning.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionPlan {
    /// Cartesian waypoints from start to goal, m.
    pub waypoints: Vec<Pose>,
    /// Joint trajectory per waypoint (parallel to `waypoints`).
    pub joint_trajectories: Vec<Vec<f64>>,
    /// Motion duration at the arm's velocity limits, s.
    pub duration_s: f64,
    /// Largest joint-space step in the plan, rad.
    pub max_velocity: f64,
    /// Every segment checked collision-free.
    pub collision_free: bool,
}

/// Grasp plan for a component.
#[derive(Debug, Clone, PartialEq)]
pub struct GraspPlan {
    /// Candidate grasp poses (side faces of the bounding box).
    pub grasp_points: Vec<Pose>,
    /// Required grip force for the component mass at 0.2 m/s² handling
    /// acceleration with a 2× safety factor, N.
    pub required_force_n: f64,
    /// True if the end-effector can hold the component.
    pub feasible: bool,
}

/// Errors from arm operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArmError {
    /// Link lengths and joint count disagree.
    JointLinkMismatch,
    /// A joint value is outside its range.
    JointRangeViolation,
}

impl fmt::Display for ArmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArmError::JointLinkMismatch => f.write_str("link lengths must match the joint count"),
            ArmError::JointRangeViolation => f.write_str("joint value outside its range"),
        }
    }
}

impl std::error::Error for ArmError {}

/// A serial robotic arm with parallel revolute joints.
#[derive(Debug, Clone, PartialEq)]
pub struct RoboticArm {
    /// Arm identifier.
    pub id: RobotId,
    /// Joints, base to tip.
    pub joints: Vec<Joint>,
    /// Link lengths between joints, m (`joints.len()` entries).
    pub link_lengths_m: Vec<f64>,
    /// End-effector.
    pub end_effector: EndEffector,
    /// Friction coefficient between the gripper jaws and the typical
    /// component surface (grasp force sizing; default 0.4 for metal on
    /// textured jaws).
    pub grasp_friction_coeff: f64,
    /// Base pose in the work area.
    pub base: Pose,
}

impl RoboticArm {
    /// Creates an arm at the origin.
    pub fn new(
        id: RobotId,
        joints: Vec<Joint>,
        link_lengths_m: Vec<f64>,
        end_effector: EndEffector,
    ) -> Self {
        Self {
            id,
            joints,
            link_lengths_m,
            end_effector,
            grasp_friction_coeff: 0.4,
            base: Pose::origin(),
        }
    }

    /// Forward kinematics: pose of the end-effector for joint values
    /// `q` (yaw accumulates; Z stays in the work plane).
    ///
    /// Revolute joints rotate the heading and lay their link along it.
    /// Prismatic joints extend along the *current* heading: the paired
    /// link length is the stage's fixed offset and `q` (metres) is the
    /// extension on top of it.
    ///
    /// # Errors
    ///
    /// [`ArmError::JointLinkMismatch`] if `q.len() != joints.len()`.
    pub fn forward_kinematics(&self, q: &[f64]) -> Result<Pose, ArmError> {
        if q.len() != self.joints.len() || self.joints.len() != self.link_lengths_m.len() {
            return Err(ArmError::JointLinkMismatch);
        }
        let mut x = self.base.position.x;
        let mut y = self.base.position.y;
        let mut yaw = self.base.yaw_rad;
        for (i, &value) in q.iter().enumerate() {
            let travel = match self.joints[i].joint_type {
                JointType::Revolute => {
                    yaw += value;
                    self.link_lengths_m[i]
                }
                JointType::Prismatic => self.link_lengths_m[i] + value,
            };
            x += travel * yaw.cos();
            y += travel * yaw.sin();
        }
        Ok(Pose {
            position: Vector3::new(x, y, self.base.position.z),
            yaw_rad: yaw,
        })
    }

    /// Inverse kinematics: joint angles for a target pose.
    ///
    /// - Two-link arms: closed-form (two solutions; the elbow-down branch
    ///   is returned first).
    /// - Longer chains: damped least-squares iteration from a stretched
    ///   seed. Position error tolerance 1 mm; yaw matched within 1 mrad.
    ///
    /// Returns `None` when the target is unreachable or joint limits bind.
    ///
    /// # Errors
    ///
    /// [`ArmError::JointLinkMismatch`] if joints/links disagree.
    pub fn inverse_kinematics(&self, target: &Pose) -> Result<Option<Vec<f64>>, ArmError> {
        if self.joints.len() != self.link_lengths_m.len() {
            return Err(ArmError::JointLinkMismatch);
        }
        if self.joints.len() == 2 {
            // Standard closed-form two-link IK in base-relative coordinates.
            let dx = target.position.x - self.base.position.x;
            let dy = target.position.y - self.base.position.y;
            let (l1, l2) = (self.link_lengths_m[0], self.link_lengths_m[1]);
            let d_sq = dx * dx + dy * dy;
            let d = d_sq.sqrt();
            if d > l1 + l2 || d < (l1 - l2).abs() {
                return Ok(None); // outside the annular workspace
            }
            let cos2 = ((d_sq - l1 * l1 - l2 * l2) / (2.0 * l1 * l2)).clamp(-1.0, 1.0);
            let phi = dy.atan2(dx);
            let wrap = |a: f64, j: usize| Self::wrap_into_range(a, self.joints[j].range);
            // Elbow-down branch first, then elbow-up; each branch must fit
            // both joints' ranges or it is skipped.
            for sign in [1.0, -1.0] {
                let Some(q2) = wrap(sign * cos2.acos(), 1) else {
                    continue;
                };
                let Some(q1) =
                    wrap(phi - (l2 * q2.sin()).atan2(l1 + l2 * q2.cos()), 0)
                else {
                    continue;
                };
                return Ok(Some(vec![q1, q2]));
            }
            return Ok(None);
        }

        // Damped least squares for longer chains, from several deterministic
        // seeds with per-iteration range projection. Local minima of the
        // squared error are common; a seed sweep is the standard cure.
        let aim = (target.position.y - self.base.position.y)
            .atan2(target.position.x - self.base.position.x);
        let n = self.joints.len();
        let seeds: Vec<Vec<f64>> = vec![
            vec![aim; n],
            (0..n).map(|i| aim - 0.9 * (i as f64 + 1.0)).collect(),
            (0..n).map(|i| aim + 0.9 * (i as f64 + 1.0)).collect(),
            (0..n).map(|i| aim - 1.8 * (i as f64 + 1.0)).collect(),
            (0..n).map(|i| aim + 1.8 * (i as f64 + 1.0)).collect(),
        ];
        for seed in &seeds {
            if let Some(q) = self.dls_refine(target, seed.clone())? {
                return Ok(Some(q));
            }
        }
        Ok(None)
    }

    /// Wraps `a` by multiples of 2π into the given range; `None` when no
    /// representative of the angle fits (e.g. a narrow joint range that no
    /// full turn can satisfy). Total in the angle: no loops can spin.
    fn wrap_into_range(a: f64, range: (f64, f64)) -> Option<f64> {
        let (lo, hi) = range;
        let two_pi = 2.0 * std::f64::consts::PI;
        let v = a.rem_euclid(two_pi); // in [0, 2π)
        let k_min = ((lo - v) / two_pi).ceil();
        let k_max = ((hi - v) / two_pi).floor();
        if k_max >= k_min {
            Some(v + k_min * two_pi)
        } else {
            None
        }
    }

    /// One damped least-squares refinement run from `seed`.
    ///
    /// Solves `dq = (JᵀJ + λI)⁻¹ Jᵀ e` in joint space (n equations for n
    /// joints), with the task vector `e = [ex, ey, eyaw]`.
    fn dls_refine(&self, target: &Pose, mut q: Vec<f64>) -> Result<Option<Vec<f64>>, ArmError> {
        // Levenberg-Marquardt damping: high early (robust gradient-ish
        // steps from bad seeds), relaxed as the solution approaches.
        let mut lambda = 0.2;
        let clamp = |v: f64, i: usize| -> f64 {
            let (lo, hi) = self.joints[i].range;
            if matches!(self.joints[i].joint_type, JointType::Revolute) && (hi - lo) >= 2.0 * std::f64::consts::PI {
                // Full-turn revolute joints wrap instead of clamping
                // (a solution at -4.18 rad is the same joint as +2.10 rad).
                Self::wrap_into_range(v, (lo, hi)).unwrap_or_else(|| v.clamp(lo, hi))
            } else {
                v.clamp(lo, hi)
            }
        };
        for (i, v) in q.iter_mut().enumerate() {
            *v = clamp(*v, i);
        }
        for _ in 0..400 {
            let current = self.forward_kinematics(&q)?;
            let ex = target.position.x - current.position.x;
            let ey = target.position.y - current.position.y;
            let eyaw = yaw_diff(target.yaw_rad, current.yaw_rad);
            let err_norm = (ex * ex + ey * ey + eyaw * eyaw).sqrt();
            if err_norm < 1e-3 {
                return Ok(Some(q));
            }
            let n = q.len();
            let eps = 1e-6;
            let err = [ex, ey, eyaw];
            // Jacobian columns (task = [x, y, yaw]).
            let mut jac = vec![[0.0f64; 3]; n];
            for (i, col) in jac.iter_mut().enumerate() {
                let mut perturbed = q.clone();
                perturbed[i] += eps;
                let p = self.forward_kinematics(&perturbed)?;
                *col = [
                    (p.position.x - current.position.x) / eps,
                    (p.position.y - current.position.y) / eps,
                    yaw_diff(p.yaw_rad, current.yaw_rad) / eps,
                ];
            }
            // A = JᵀJ + λI (n×n), b = Jᵀe (n).
            let mut a = vec![vec![0.0f64; n]; n];
            let mut b = vec![0.0f64; n];
            for r in 0..n {
                for c in 0..n {
                    a[r][c] = jac[r][0] * jac[c][0]
                        + jac[r][1] * jac[c][1]
                        + jac[r][2] * jac[c][2];
                }
                a[r][r] += lambda;
                b[r] = jac[r][0] * err[0] + jac[r][1] * err[1] + jac[r][2] * err[2];
            }
            let Some(mut dq) = solve_dense(&mut a, &mut b) else {
                return Ok(None); // singular even with damping: give up
            };
            // Clamp the step magnitude.
            let norm = dq.iter().map(|d| d * d).sum::<f64>().sqrt();
            if norm > 0.4 {
                for d in &mut dq {
                    *d *= 0.4 / norm;
                }
            }
            // Accept only error-reducing steps; otherwise stiffen.
            let candidate: Vec<f64> = q
                .iter()
                .zip(&dq)
                .enumerate()
                .map(|(i, (v, d))| clamp(v + d, i))
                .collect();
            let c = self.forward_kinematics(&candidate)?;
            let c_err = (
                target.position.x - c.position.x,
                target.position.y - c.position.y,
                yaw_diff(target.yaw_rad, c.yaw_rad),
            );
            let c_norm = (c_err.0 * c_err.0 + c_err.1 * c_err.1 + c_err.2 * c_err.2).sqrt();
            if c_norm < err_norm {
                q = candidate;
                lambda = (lambda * 0.7).max(1e-3);
            } else {
                lambda = (lambda * 2.5).min(5.0);
            }
        }
        Ok(None)
    }

    /// True when every joint value is inside its range (1e-6 tolerance).
    pub fn within_limits(&self, q: &[f64]) -> bool {
        q.iter()
            .zip(&self.joints)
            .all(|(&v, j)| v >= j.range.0 - 1e-6 && v <= j.range.1 + 1e-6)
    }

    /// Plans a collision-free path from `start` to `goal` with RRT.
    ///
    /// Obstacles are circles in the work plane; a configuration collides if
    /// any joint-frame point of the arm (sampled along the links) is inside
    /// an obstacle inflated by `clearance_m`. Returns `None` if no path is
    /// found within the iteration budget.
    ///
    /// # Errors
    ///
    /// [`ArmError::JointLinkMismatch`] if start/goal dimensions disagree.
    pub fn plan_path(
        &self,
        start: &Pose,
        goal: &Pose,
        obstacles: &[CollisionObject],
        clearance_m: f64,
    ) -> Result<Option<MotionPlan>, ArmError> {
        let q_start = self.inverse_kinematics(start)?;
        let q_goal = self.inverse_kinematics(goal)?;
        let (Some(q_start), Some(q_goal)) = (q_start, q_goal) else {
            return Ok(None); // endpoint unreachable
        };

        // Joint wrap-around: the IK goal may sit on the other 2-pi branch
        // from the start (e.g. start 3.1, goal -3.1). Take the equivalent
        // goal representation closest to the start so the path does not
        // spin joints the long way round.
        let mut q_goal = q_goal;
        for (idx, g) in q_goal.iter_mut().enumerate() {
            let s0 = q_start[idx];
            let two_pi = 2.0 * std::f64::consts::PI;
            while *g - s0 > std::f64::consts::PI {
                *g -= two_pi;
            }
            while s0 - *g > std::f64::consts::PI {
                *g += two_pi;
            }
            // Keep the representation inside the joint range if possible.
            let (lo, hi) = self.joints[idx].range;
            if (*g < lo || *g > hi) && hi - lo >= two_pi {
                if let Some(wrapped) = Self::wrap_into_range(*g, (lo, hi)) {
                    *g = wrapped;
                }
            }
        }

        // Work-plane bounding region from the arm's reach.
        let _reach: f64 = self.link_lengths_m.iter().sum();
        let mut rng_state = 0x853c49e6748fea9bu64;
        let mut rand = move || {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            (rng_state >> 11) as f64 / (1u64 << 53) as f64
        };

        let mut nodes: Vec<Vec<f64>> = vec![q_start.clone()];
        let mut parents: Vec<usize> = vec![usize::MAX];
        let max_iter = 4000;
        let mut goal_reached: Option<usize> = None;
        if !Self::segment_collides(self, &q_start, &q_goal, obstacles, clearance_m) {
            // Direct connect.
            nodes.push(q_goal.clone());
            parents.push(0);
            goal_reached = Some(1);
        }
        for _ in 0..max_iter {
            if goal_reached.is_some() {
                break;
            }
            // Sample around the goal 20 % of the time.
            let sample: Vec<f64> = if rand() < 0.2 {
                q_goal.iter().map(|g| g + (rand() - 0.5) * 0.4).collect()
            } else {
                (0..self.joints.len())
                    .map(|i| {
                        let (lo, hi) = self.joints[i].range;
                        lo + rand() * (hi - lo)
                    })
                    .collect()
            };
            // Nearest node in joint space.
            let nearest = nodes
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let da = joint_dist(a, &sample);
                    let db = joint_dist(b, &sample);
                    da.total_cmp(&db)
                })
                .map(|(i, _)| i)
                .unwrap();
            // Step toward the sample.
            let step_len = 0.5;
            let dist = joint_dist(&nodes[nearest], &sample);
            let new_q: Vec<f64> = if dist <= step_len {
                sample.clone()
            } else {
                nodes[nearest]
                    .iter()
                    .zip(&sample)
                    .map(|(a, b)| a + (b - a) / dist * step_len)
                    .collect()
            };
            if !self.within_limits(&new_q) {
                continue;
            }
            if Self::segment_collides(self, &nodes[nearest], &new_q, obstacles, clearance_m) {
                continue;
            }
            nodes.push(new_q.clone());
            parents.push(nearest);
            if joint_dist(&new_q, &q_goal) < 0.25
                && !Self::segment_collides(self, &new_q, &q_goal, obstacles, clearance_m)
            {
                nodes.push(q_goal.clone());
                parents.push(nodes.len() - 2);
                goal_reached = Some(nodes.len() - 1);
                break;
            }
        }

        let Some(end) = goal_reached else {
            return Ok(None);
        };
        // Reconstruct.
        let mut chain = Vec::new();
        let mut cur = end;
        while cur != usize::MAX {
            chain.push(nodes[cur].clone());
            cur = parents[cur];
        }
        chain.reverse();

        // Cartesian waypoints through FK.
        let mut waypoints = Vec::with_capacity(chain.len());
        for q in &chain {
            waypoints.push(self.forward_kinematics(q)?);
        }
        // Duration: sum of worst joint step / velocity limit, with a small
        // per-waypoint settle time.
        let mut duration = 0.0;
        let mut max_step = 0.0f64;
        for w in chain.windows(2) {
            let step = joint_dist(&w[0], &w[1]);
            max_step = max_step.max(step);
            let slowest = self
                .joints
                .iter()
                .zip(w[0].iter().zip(&w[1]))
                .map(|(j, (a, b))| (a - b).abs() / j.max_velocity.max(1e-6))
                .fold(0.0f64, f64::max);
            duration += slowest + 0.5; // 0.5 s settle per waypoint
        }
        // Verify every chain edge rather than asserting freedom by
        // construction (the flag then tells the truth even if a later
        // refactor weakens a check).
        let collision_free = !chain.windows(2).any(|w| {
            Self::segment_collides(self, &w[0], &w[1], obstacles, clearance_m)
        });
        Ok(Some(MotionPlan {
            waypoints,
            joint_trajectories: chain,
            duration_s: duration,
            max_velocity: max_step,
            collision_free,
        }))
    }

    /// True if moving between two configurations sweeps through an obstacle.
    fn segment_collides(
        arm: &RoboticArm,
        a: &[f64],
        b: &[f64],
        obstacles: &[CollisionObject],
        clearance: f64,
    ) -> bool {
        const STEPS: usize = 8;
        for k in 0..=STEPS {
            let t = k as f64 / STEPS as f64;
            let q: Vec<f64> = a.iter().zip(b).map(|(x, y)| x + (y - x) * t).collect();
            if Self::configuration_collides(arm, &q, obstacles, clearance) {
                return true;
            }
        }
        false
    }

    fn configuration_collides(
        arm: &RoboticArm,
        q: &[f64],
        obstacles: &[CollisionObject],
        clearance: f64,
    ) -> bool {
        // Sample points along each link and the end-effector.
        let mut x = arm.base.position.x;
        let mut y = arm.base.position.y;
        let mut yaw = arm.base.yaw_rad;
        for (i, &value) in q.iter().enumerate() {
            let l = arm.link_lengths_m[i];
            let steps = 4;
            for s in 1..=steps {
                // Prismatic joints extend along the current heading on top
                // of the stage offset (matching `forward_kinematics`).
                let f = match arm.joints[i].joint_type {
                    JointType::Revolute => l * s as f64 / steps as f64,
                    JointType::Prismatic => (l + value) * s as f64 / steps as f64,
                };
                let px = x + yaw.cos() * f;
                let py = y + yaw.sin() * f;
                for o in obstacles {
                    let dx = px - o.centre.x;
                    let dy = py - o.centre.y;
                    if dx.hypot(dy) < o.radius + clearance {
                        return true;
                    }
                }
            }
            if matches!(arm.joints[i].joint_type, JointType::Revolute) {
                yaw += value;
            }
            x += l * yaw.cos();
            y += l * yaw.sin();
        }
        false
    }

    /// Grasp-point planning for a box-like component: approach poses on the
    /// two long side faces, grip force from the handling acceleration.
    pub fn grasp_planning(&self, component: &ComponentSpec) -> GraspPlan {
        let cx = component.centre.x;
        let cy = component.centre.y;
        // Grasp on the faces of the *long* axis: the jaws close on the
        // wide side (a stable, low-tension pick), not blindly on x.
        let (half_long, long_is_x) = if component.dimensions.x >= component.dimensions.y {
            (component.dimensions.x / 2.0, true)
        } else {
            (component.dimensions.y / 2.0, false)
        };
        let all_points = if long_is_x {
            vec![
                Pose {
                    position: Vector3::new(cx - half_long, cy, component.centre.z),
                    yaw_rad: -std::f64::consts::FRAC_PI_2,
                },
                Pose {
                    position: Vector3::new(cx + half_long, cy, component.centre.z),
                    yaw_rad: std::f64::consts::FRAC_PI_2,
                },
            ]
        } else {
            vec![
                Pose {
                    position: Vector3::new(cx, cy - half_long, component.centre.z),
                    yaw_rad: std::f64::consts::PI,
                },
                Pose {
                    position: Vector3::new(cx, cy + half_long, component.centre.z),
                    yaw_rad: 0.0,
                },
            ]
        };
        // Reachability: a grasp point beyond the arm's reach cannot be
        // used, whatever the gripper can squeeze.
        let reach: f64 = self.link_lengths_m.iter().sum();
        let grasp_points: Vec<Pose> = all_points
            .into_iter()
            .filter(|p| {
                self.base.position.distance(p.position) <= reach + 1e-9
            })
            .collect();
        // Friction-limited grip: the jaws must transmit the handling force
        // (0.2 m/s^2 with a 2x factor) through friction on two faces.
        let mu = self.grasp_friction_coeff.max(1e-3);
        let required_force_n = component.mass_kg * 0.2 * 2.0 / (2.0 * mu);
        let force_ok = match &self.end_effector {
            EndEffector::Gripper { force_n } => *force_n >= required_force_n,
            EndEffector::MultiTool => true,
            _ => false,
        };
        let feasible = force_ok && !grasp_points.is_empty();
        GraspPlan {
            grasp_points,
            required_force_n,
            feasible,
        }
    }
}

/// A component awaiting assembly (input to planning/grasping).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentSpec {
    /// Component identifier (from `tpt-yard-core::ComponentId.0`).
    pub id: u64,
    /// Name.
    pub name: String,
    /// Mass, kg.
    pub mass_kg: f64,
    /// Centre position in the work area, m.
    pub centre: Vector3,
    /// Bounding dimensions, m.
    pub dimensions: Vector3,
}

fn joint_dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Smallest signed difference between two yaw angles, in `(-π, π]`.
fn yaw_diff(target: f64, current: f64) -> f64 {
    let mut e = target - current;
    while e > std::f64::consts::PI {
        e -= 2.0 * std::f64::consts::PI;
    }
    while e < -std::f64::consts::PI {
        e += 2.0 * std::f64::consts::PI;
    }
    e
}

/// Solves the dense `n × n` system `a·x = b` in place by Gaussian
/// elimination with partial pivoting. Returns `None` when singular.
fn solve_dense(a: &mut Vec<Vec<f64>>, b: &mut Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&r1, &r2| {
                a[r1][col]
                    .abs()
                    .partial_cmp(&a[r2][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?;
        a.swap(col, pivot);
        b.swap(col, pivot);
        if a[col][col].abs() < 1e-12 {
            return None;
        }
        for r in (col + 1)..n {
            let f = a[r][col] / a[col][col];
            for c in col..n {
                a[r][c] -= f * a[col][c];
            }
            b[r] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let s: f64 = ((r + 1)..n).map(|c| a[r][c] * x[c]).sum();
        x[r] = (b[r] - s) / a[r][r];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_link() -> RoboticArm {
        let mut arm = RoboticArm::new(
            RobotId(1),
            vec![
                Joint::revolute(0.0, 2.0 * std::f64::consts::PI, 1.0),
                Joint::revolute(0.0, 2.0 * std::f64::consts::PI, 1.0),
            ],
            vec![1.0, 1.0],
            EndEffector::Gripper { force_n: 500.0 },
        );
        arm.base = Pose::origin();
        arm
    }

    #[test]
    fn two_link_ik_matches_fk() {
        let arm = two_link();
        for target in [
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(1.5, 0.5, 0.0),
            Vector3::new(0.5, -1.2, 0.0),
        ] {
            let pose = Pose {
                position: target,
                yaw_rad: 0.0,
            };
            let q = arm
                .inverse_kinematics(&pose)
                .expect("reachable")
                .expect("solution");
            let fk = arm.forward_kinematics(&q).unwrap();
            assert!(
                (fk.position.x - target.x).abs() < 1e-6,
                "x: {} vs {}",
                fk.position.x,
                target.x
            );
            assert!((fk.position.y - target.y).abs() < 1e-6);
        }
    }

    #[test]
    fn unreachable_target_is_none() {
        let arm = two_link();
        let pose = Pose {
            position: Vector3::new(5.0, 0.0, 0.0),
            yaw_rad: 0.0,
        };
        assert_eq!(arm.inverse_kinematics(&pose).unwrap(), None);
    }

    #[test]
    fn three_link_dls_converges() {
        let mut arm = RoboticArm::new(
            RobotId(2),
            vec![
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.8),
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.8),
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.8),
            ],
            vec![1.2, 1.0, 0.8],
            EndEffector::MultiTool,
        );
        arm.base = Pose::origin();
        let target = Pose {
            position: Vector3::new(2.0, 0.8, 0.0),
            yaw_rad: 0.3,
        };
        let q = arm
            .inverse_kinematics(&target)
            .expect("solvable")
            .expect("solution");
        let fk = arm.forward_kinematics(&q).unwrap();
        assert!((fk.position.x - 2.0).abs() < 1e-3, "{}", fk.position.x);
        assert!((fk.position.y - 0.8).abs() < 1e-3);
    }

    /// Regression (review 7A/A5): the two-link angle normalisation used to
    /// wrap by ±2π in a `while` loop, which never terminates when the joint
    /// range is narrower than a full turn and no representative fits.
    #[test]
    fn narrow_joint_ranges_terminate() {
        let arm = RoboticArm::new(
            RobotId(4),
            vec![
                Joint::revolute(0.2, 0.6, 0.5),
                Joint::revolute(0.2, 0.6, 0.5),
            ],
            vec![1.0, 1.0],
            EndEffector::Gripper { force_n: 100.0 },
        );
        // Any target: the answer (Some or None) must come back immediately
        // — a hang here fails the test-runner timeout.
        let target = Pose {
            position: Vector3::new(1.5, 0.5, 0.0),
            yaw_rad: 0.4,
        };
        let q = arm.inverse_kinematics(&target).expect("runs");
        if let Some(q) = q {
            // Whatever it returns must actually solve the pose.
            let fk = arm.forward_kinematics(&q).unwrap();
            assert!((fk.position.x - 1.5).abs() < 1e-3);
            assert!((fk.position.y - 0.5).abs() < 1e-3);
        }
    }

    /// Regression (review 7A/A5): the DLS step used a hard-coded 3×3 system
    /// regardless of joint count, so chains with 4+ joints never converged.
    #[test]
    fn four_link_dls_converges() {
        let mut arm = RoboticArm::new(
            RobotId(5),
            vec![
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.5),
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.5),
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.5),
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.5),
            ],
            vec![0.8, 0.7, 0.6, 0.5],
            EndEffector::Camera,
        );
        arm.base = Pose::origin();
        let target = Pose {
            position: Vector3::new(1.8, 0.5, 0.0),
            yaw_rad: 0.2,
        };
        let q = arm
            .inverse_kinematics(&target)
            .expect("solvable")
            .expect("4-link solution");
        let fk = arm.forward_kinematics(&q).unwrap();
        assert!((fk.position.x - 1.8).abs() < 1e-3, "{}", fk.position.x);
        assert!((fk.position.y - 0.5).abs() < 1e-3);
        assert!(yaw_diff(0.2, fk.yaw_rad).abs() < 1e-3);
    }

    /// A target behind the arm's base must be reachable by folding the
    /// chain back on itself.
    #[test]
    fn behind_the_arm_target_reachable() {
        let mut arm = RoboticArm::new(
            RobotId(6),
            vec![
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.8),
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.8),
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 0.8),
            ],
            vec![1.0, 0.9, 0.8],
            EndEffector::MultiTool,
        );
        arm.base = Pose::origin();
        let target = Pose {
            position: Vector3::new(-1.8, 0.3, 0.0),
            yaw_rad: 2.5,
        };
        let q = arm
            .inverse_kinematics(&target)
            .expect("solvable")
            .expect("behind-arm solution");
        let fk = arm.forward_kinematics(&q).unwrap();
        assert!((fk.position.x - -1.8).abs() < 1e-3, "{}", fk.position.x);
        assert!((fk.position.y - 0.3).abs() < 1e-3);
    }

    /// Prismatic joints extend along the current heading instead of
    /// rotating it.
    #[test]
    fn prismatic_joint_extends_fk() {
        let arm = RoboticArm::new(
            RobotId(7),
            vec![
                Joint::revolute(0.0, 0.0, 1.0),
                Joint {
                    joint_type: JointType::Prismatic,
                    range: (0.0, 1.0),
                    max_velocity: 0.2,
                    max_torque_nm: 2000.0,
                },
            ],
            vec![1.0, 0.5],
            EndEffector::Gripper { force_n: 200.0 },
        );
        // Heading stays 0; travel = link 1 + extension.
        let fk = arm.forward_kinematics(&[0.0, 0.3]).unwrap();
        assert!((fk.position.x - (1.0 + 0.5 + 0.3)).abs() < 1e-9, "{fk:?}");
        assert!((fk.position.y).abs() < 1e-9);
        assert!((fk.yaw_rad).abs() < 1e-9);
    }

    #[test]
    fn rrt_paths_around_obstacle() {
        let arm = two_link();
        // Start right of the obstacle, goal above-left of it: the direct
        // joint-space move sweeps the arm through the obstacle; a detour
        // exists (swing the base joint under/around).
        let start = Pose {
            position: Vector3::new(1.9, 0.1, 0.0),
            yaw_rad: 0.0,
        };
        let goal = Pose {
            position: Vector3::new(0.5, 1.85, 0.0),
            yaw_rad: 0.0,
        };
        let obstacles = [CollisionObject {
            centre: Vector3::new(1.0, 0.6, 0.0),
            radius: 0.5,
        }];
        let plan = arm
            .plan_path(&start, &goal, &obstacles, 0.05)
            .expect("planner ran")
            .expect("path exists around the obstacle");
        assert!(plan.collision_free);
        assert!(plan.waypoints.len() >= 2);
        assert!(plan.duration_s > 0.0);
        // Bounded length (review 7D): the detour must be a real but sane
        // path — Cartesian path length within 3x the straight-line span,
        // and the joint trajectory stays inside every joint range.
        let straight = start.position.distance(goal.position);
        let mut path_len = 0.0;
        for w in plan.waypoints.windows(2) {
            path_len += w[0].position.distance(w[1].position);
        }
        assert!(
            path_len < 3.0 * straight,
            "path length {path_len} vs straight {straight}"
        );
        for q in &plan.joint_trajectories {
            assert!(arm.within_limits(q), "trajectory {q:?} out of range");
        }
    }

    /// Grasp analysis (review 7B: friction-limited force, long-side faces,
    /// reachability). Replaces two earlier near-duplicate tests.
    #[test]
    fn grasp_planning_is_friction_and_reach_aware() {
        let arm = two_link(); // 500 N gripper, reach 2 m, mu = 0.4
        let panel = ComponentSpec {
            id: 1,
            name: "truss panel".into(),
            mass_kg: 1000.0,
            centre: Vector3::new(1.0, 0.0, 0.0),
            dimensions: Vector3::new(2.0, 1.0, 0.2), // long in x
        };
        let plan = arm.grasp_planning(&panel);
        // Friction-limited: F = m*a*SF / (2 mu) = 1000*0.2*2 / 0.8 = 500 N.
        assert!((plan.required_force_n - 500.0).abs() < 1e-9);
        // Faces of the long axis (x): at cx +/- 1 m, both within reach.
        assert_eq!(plan.grasp_points.len(), 2);
        assert!((plan.grasp_points[0].position.x - 0.0).abs() < 1e-9);
        assert!((plan.grasp_points[1].position.x - 2.0).abs() < 1e-9);
        assert!(plan.feasible, "500 N gripper exactly meets the 500 N need");

        // A y-long part is grasped on its y faces, not blindly on x.
        let y_long = ComponentSpec {
            id: 2,
            name: "beam".into(),
            mass_kg: 200.0,
            centre: Vector3::new(1.0, 0.0, 0.0),
            dimensions: Vector3::new(0.5, 1.5, 0.3),
        };
        let plan_y = arm.grasp_planning(&y_long);
        assert!((plan_y.grasp_points[0].position.y - -0.75).abs() < 1e-9);
        assert!((plan_y.grasp_points[1].position.y - 0.75).abs() < 1e-9);

        // Beyond the reach: no usable grasp points, infeasible whatever
        // the force.
        let far = ComponentSpec {
            id: 3,
            name: "far panel".into(),
            mass_kg: 100.0,
            centre: Vector3::new(5.0, 0.0, 0.0),
            dimensions: Vector3::new(2.0, 1.0, 0.2),
        };
        let plan_far = arm.grasp_planning(&far);
        assert!(plan_far.grasp_points.is_empty());
        assert!(!plan_far.feasible);

        // Slippery jaws raise the force need above a 500 N gripper.
        let mut slick = two_link();
        slick.grasp_friction_coeff = 0.2;
        assert!(!slick.grasp_planning(&panel).feasible);
    }

    #[test]
    fn joint_mismatch_errors() {
        // Two joints, one link length: dimension mismatch.
        let arm = RoboticArm::new(
            RobotId(3),
            vec![
                Joint::revolute(0.0, 1.0, 1.0),
                Joint::revolute(0.0, 1.0, 1.0),
            ],
            vec![1.0],
            EndEffector::Camera,
        );
        assert_eq!(
            arm.forward_kinematics(&[0.1, 0.2]),
            Err(ArmError::JointLinkMismatch)
        );
        assert_eq!(
            arm.inverse_kinematics(&Pose::origin()),
            Err(ArmError::JointLinkMismatch)
        );
    }
}
