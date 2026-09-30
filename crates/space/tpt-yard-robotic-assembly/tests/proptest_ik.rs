//! Property-based tests for arm kinematics (review 7D).
//!
//! Invariants: whenever IK returns a solution, FK(IK(target)) hits the
//! target within tolerance; FK is exact for any configuration; IK never
//! returns out-of-range joints; wrap_into_range is total (never hangs).

use proptest::prelude::*;

use tpt_yard_core::{RobotId, Vector3};
use tpt_yard_robotic_assembly::{EndEffector, Joint, JointType, Pose, RoboticArm};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// FK is exact for any joint values inside the ranges (full-turn
    /// revolute chain): the endpoint is the vector sum of link vectors.
    #[test]
    fn fk_is_the_exact_vector_sum(
        angles in proptest::collection::vec(-std::f64::consts::PI..std::f64::consts::PI, 2usize),
        l1 in 0.5f64..2.5,
        l2 in 0.5f64..2.5,
    ) {
        let arm = RoboticArm::new(
            RobotId(1),
            vec![
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 1.0),
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 1.0),
            ],
            vec![l1, l2],
            EndEffector::Gripper { force_n: 100.0 },
        );
        let fk = arm.forward_kinematics(&angles).unwrap();
        let yaw = angles[0];
        let yaw2 = angles[0] + angles[1];
        let expected_x = l1 * yaw.cos() + l2 * yaw2.cos();
        let expected_y = l1 * yaw.sin() + l2 * yaw2.sin();
        prop_assert!((fk.position.x - expected_x).abs() < 1e-9);
        prop_assert!((fk.position.y - expected_y).abs() < 1e-9);
    }

    /// Whenever the closed-form two-link IK returns a solution, FK of it
    /// reproduces the target — for ANY reachable target and ANY link pair.
    #[test]
    fn ik_solution_reproduces_target(
        tx in -2.0f64..2.0,
        ty in -2.0f64..2.0,
        l1 in 1.0f64..2.0,
        l2 in 1.0f64..2.0,
    ) {
        let arm = RoboticArm::new(
            RobotId(2),
            vec![
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 1.0),
                Joint::revolute(-std::f64::consts::PI, std::f64::consts::PI, 1.0),
            ],
            vec![l1, l2],
            EndEffector::Gripper { force_n: 100.0 },
        );
        let target = Pose {
            position: Vector3::new(tx, ty, 0.0),
            yaw_rad: 0.0,
        };
        let d = (tx * tx + ty * ty).sqrt();
        // Skip targets outside the annulus (IK legitimately returns None).
        prop_assume!(d <= l1 + l2 - 1e-3 && d >= (l1 - l2).abs() + 1e-3);
        if let Some(q) = arm.inverse_kinematics(&target).unwrap() {
            let fk = arm.forward_kinematics(&q).unwrap();
            prop_assert!((fk.position.x - tx).abs() < 1e-6, "{} vs {}", fk.position.x, tx);
            prop_assert!((fk.position.y - ty).abs() < 1e-6);
            // Solutions are inside every joint range.
            prop_assert!(arm.within_limits(&q), "{q:?}");
        }
    }

    /// Prismatic joints translate along the heading; the pose stays on the
    /// axis for any extension in range.
    #[test]
    fn prismatic_fk_stays_on_axis(extension in 0.0f64..1.0, base_len in 0.5f64..2.0) {
        let arm = RoboticArm::new(
            RobotId(3),
            vec![Joint {
                joint_type: JointType::Prismatic,
                range: (0.0, 1.0),
                max_velocity: 1.0,
                max_torque_nm: 100.0,
            }],
            vec![base_len],
            EndEffector::Gripper { force_n: 100.0 },
        );
        let fk = arm.forward_kinematics(&[extension]).unwrap();
        prop_assert!((fk.position.x - (base_len + extension)).abs() < 1e-9);
        prop_assert!(fk.position.y.abs() < 1e-9);
        prop_assert!(fk.yaw_rad.abs() < 1e-9);
    }
}
