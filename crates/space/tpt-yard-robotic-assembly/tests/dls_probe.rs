use tpt_yard_core::{RobotId, Vector3};
use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose, RoboticArm};

#[test]
fn probe() {
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
    for (tx, ty, tyaw) in [(1.5, 0.0, 0.0), (1.0, 1.0, 0.5), (2.0, 0.8, 0.3)] {
        let target = Pose {
            position: Vector3::new(tx, ty, 0.0),
            yaw_rad: tyaw,
        };
        let r = arm.inverse_kinematics(&target);
        match r {
            Ok(Some(q)) => {
                let fk = arm.forward_kinematics(&q).unwrap();
                println!(
                    "target ({tx},{ty},{tyaw}) -> fk: {:.4} {:.4} yaw {:.4}",
                    fk.position.x, fk.position.y, fk.yaw_rad
                );
            }
            Ok(None) => println!("target ({tx},{ty},{tyaw}) -> None (no convergence)"),
            Err(e) => println!("target ({tx},{ty},{tyaw}) -> ik err: {e}"),
        }
    }
}
