//! Benchmark: orbital assembly sequence planning and simulation.

use std::time::Instant;

use tpt_yard_core::{ComponentId, RobotId, Vector3};
use tpt_yard_orbital_assembly::{
    AssemblyAction, AssemblyState, ComponentSpec, OrbitalAssembly, OrbitalParameters,
    SpaceStructure,
};
use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose, RoboticArm};

fn bench(name: &str, iterations: u32, mut f: impl FnMut()) {
    f();
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    println!(
        "{name:<44} {iterations:>8} iters  {:>10.1?} total  {:>10.3?} / iter",
        elapsed,
        elapsed / iterations
    );
}

fn truss_with(bays: u64) -> OrbitalAssembly {
    let mut a = OrbitalAssembly::new(
        SpaceStructure::Truss {
            segments: bays as u32,
            length_m: bays as f64 * 5.0,
        },
        OrbitalParameters::default(),
    );
    for i in 1..=bays {
        a.add_component(ComponentSpec {
            id: ComponentId(i),
            name: format!("bay {i}"),
            mass_kg: 500.0,
            dimensions: Vector3::new(5.0, 3.0, 3.0),
            target_position: Vector3::new(5.0 * i as f64 - 2.5, 0.0, 0.0),
        });
    }
    let mut robot = RoboticArm::new(
        RobotId(1),
        vec![
            Joint::revolute(-3.0, 3.0, 0.5),
            Joint::revolute(-3.0, 3.0, 0.5),
        ],
        vec![4.0, 4.0],
        EndEffector::Gripper { force_n: 400.0 },
    );
    robot.base = Pose::origin();
    a.add_robot(robot);
    a
}

fn main() {
    println!("== tpt-shipyard: orbital-assembly-sequence benchmark ==");

    for bays in [3, 8, 15] {
        let name = format!("plan_sequence ({bays} bays)");
        bench(&name, 500, || {
            let mut a = truss_with(bays);
            let s = a.plan_sequence();
            assert_eq!(s.len(), bays as usize * 6);
        });
    }

    // Full simulation sweep of the 15-bay build.
    bench("simulate full build (15 bays x 6 steps)", 100, || {
        let mut a = truss_with(15);
        let steps = a.plan_sequence();
        let mut state = AssemblyState::default();
        for step in &steps {
            let r = a.simulate_step(step, &state).unwrap();
            assert!(r.ok);
            if matches!(step.action, AssemblyAction::Release) {
                state.installed_components.push(step.component);
            }
            state.completed_steps.push(step.id);
            let _ = a.verify_structural_integrity(&step.id).unwrap();
        }
    });

    // Robot path planning around installed structure.
    bench("plan_path around installed stack", 20, || {
        let mut a = truss_with(4);
        a.plan_sequence();
        let arm = RoboticArm::new(
            RobotId(9),
            vec![
                Joint::revolute(0.0, 6.3, 0.8),
                Joint::revolute(0.0, 6.3, 0.8),
            ],
            vec![5.0, 5.0],
            EndEffector::Gripper { force_n: 500.0 },
        );
        let start = Pose {
            position: Vector3::new(9.0, 0.5, 0.0),
            yaw_rad: 0.0,
        };
        let goal = Pose {
            position: Vector3::new(0.6, 9.0, 0.0),
            yaw_rad: 0.0,
        };
        let obstacles = vec![tpt_yard_robotic_assembly::CollisionObject {
            centre: Vector3::new(5.0, 2.5, 0.0),
            radius: 1.5,
        }];
        let plan = arm.plan_path(&start, &goal, &obstacles, 0.1).unwrap();
        assert!(plan.is_some());
    });
}
