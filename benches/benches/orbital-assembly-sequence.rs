//! Benchmark: orbital assembly sequence planning and simulation.
//! Criterion-managed (review 7G).

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, Criterion};

use tpt_yard_core::{ComponentId, RobotId, Vector3};
use tpt_yard_orbital_assembly::{
    AssemblyAction, AssemblyState, ComponentSpec, OrbitalAssembly, OrbitalParameters,
    SpaceStructure,
};
use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose, RoboticArm};

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

fn bench_orbital(c: &mut Criterion) {
    for bays in [3u64, 8, 15] {
        c.bench_function(&format!("orbital/plan_sequence_{bays}_bays"), |b| {
            b.iter(|| {
                let mut a = truss_with(bays);
                std::hint::black_box(a.plan_sequence().len())
            })
        });
    }

    c.bench_function("orbital/simulate_full_15_bay_build", |b| {
        b.iter(|| {
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
            }
        })
    });
}

criterion_group!(benches, bench_orbital);
criterion_main!(benches);
