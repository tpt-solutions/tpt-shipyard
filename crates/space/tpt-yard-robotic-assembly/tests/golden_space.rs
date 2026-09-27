//! Golden verification for the robotic arm RRT path planning (RFC 0003).

use std::path::PathBuf;

use tpt_yard_core::{json::Value, RobotId, Vector3};
use tpt_yard_robotic_assembly::{CollisionObject, EndEffector, Joint, Pose, RoboticArm};

#[test]
fn golden_robotic_arm_path() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test-data/golden/space/robotic-arm-path.json");
    let text = std::fs::read_to_string(&path).expect("golden readable");
    let v = Value::parse(&text).expect("parses");

    let arm_spec = v.get("arm").expect("arm");
    let links: Vec<f64> = arm_spec
        .get("links_m")
        .and_then(|l| l.as_array())
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    let range = arm_spec
        .get("joint_range_rad")
        .and_then(|r| r.as_array())
        .unwrap();
    let joints: Vec<Joint> = links
        .iter()
        .map(|_| Joint::revolute(range[0].as_f64().unwrap(), range[1].as_f64().unwrap(), 0.8))
        .collect();
    let mut arm = RoboticArm::new(
        RobotId(1),
        joints,
        links,
        EndEffector::Gripper { force_n: 400.0 },
    );
    arm.base = Pose::origin();

    let pose = |arr: &Value| -> Pose {
        let a = arr.as_array().unwrap();
        Pose {
            position: Vector3::new(a[0].as_f64().unwrap(), a[1].as_f64().unwrap(), 0.0),
            yaw_rad: 0.0,
        }
    };
    let start = pose(v.get("start_pose_m").expect("start"));
    let goal = pose(v.get("goal_pose_m").expect("goal"));
    let obstacles: Vec<CollisionObject> = v
        .get("obstacles")
        .and_then(|o| o.as_array())
        .unwrap()
        .iter()
        .map(|o| {
            let c = o.get("centre").and_then(|c| c.as_array()).unwrap();
            CollisionObject {
                centre: Vector3::new(c[0].as_f64().unwrap(), c[1].as_f64().unwrap(), 0.0),
                radius: o.get("radius").and_then(|r| r.as_f64()).unwrap(),
            }
        })
        .collect();
    let clearance = v.get("clearance_m").and_then(|c| c.as_f64()).unwrap();

    let plan = arm
        .plan_path(&start, &goal, &obstacles, clearance)
        .expect("planner ran")
        .expect("path found");

    let exp = v.get("expected").expect("expected");
    assert_eq!(
        plan.collision_free,
        exp.get("collision_free").and_then(|b| b.as_bool()).unwrap()
    );
    assert!(
        plan.waypoints.len() >= exp.get("min_waypoints").and_then(|n| n.as_u64()).unwrap() as usize
    );
    assert_eq!(
        plan.duration_s > 0.0,
        exp.get("duration_s_positive")
            .and_then(|b| b.as_bool())
            .unwrap()
    );
    let tol = exp
        .get("goal_reached_tolerance_m")
        .and_then(|t| t.as_f64())
        .unwrap();
    let end = plan.waypoints.last().unwrap();
    assert!(
        end.position.distance(goal.position) <= tol,
        "goal not reached: {} vs {}",
        end.position,
        goal.position
    );
}
