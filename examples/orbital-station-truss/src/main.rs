//! Milestone example (Phase 4): simulate an ISS-class truss assembly with a
//! robotic arm end-to-end — manifest, sequence plan, step simulation with
//! collision and force checks, and partial-structure integrity at every bay.

use tpt_yard_core::{ComponentId, StepId, Vector3};
use tpt_yard_orbital_assembly::{
    AssemblyAction, AssemblyState, ComponentSpec, OrbitalAssembly, OrbitalParameters,
    SpaceStructure,
};
use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose, RoboticArm};

const BAYS: u64 = 6;
const PITCH: f64 = 5.0;

fn main() {
    // 1. The truss manifest and the assembly robot.
    let mut assembly = OrbitalAssembly::new(
        SpaceStructure::Truss {
            segments: BAYS as u32,
            length_m: BAYS as f64 * PITCH,
        },
        OrbitalParameters::default(),
    );
    for i in 1..=BAYS {
        assembly.add_component(ComponentSpec {
            id: ComponentId(i),
            name: format!("bay {i}"),
            mass_kg: 500.0,
            dimensions: Vector3::new(PITCH, 3.0, 3.0),
            target_position: Vector3::new(PITCH * i as f64 - PITCH / 2.0, 0.0, 0.0),
        });
    }
    let mut robot = RoboticArm::new(
        tpt_yard_core::RobotId(1),
        vec![
            Joint::revolute(-3.0, 3.0, 0.5),
            Joint::revolute(-3.0, 3.0, 0.5),
        ],
        vec![4.0, 4.0],
        EndEffector::Gripper { force_n: 400.0 },
    );
    robot.base = Pose::origin();
    assembly.add_robot(robot);

    // 2. Plan the sequence: anchor bay first, build outwards.
    let steps = assembly.plan_sequence();
    println!(
        "Truss: {BAYS} bays x {PITCH} m, {} assembly steps",
        steps.len()
    );

    // 3. Simulate the whole build; check the partial structure at every
    //    bay release.
    let mut state = AssemblyState::default();
    let mut bays_done = 0;
    println!("\n step | action           | bay    | force [N] | ok | root stress [MPa]");
    for step in &steps {
        let result = assembly.simulate_step(step, &state).expect("valid step");
        let bay = step.component.0;
        let action = match &step.action {
            AssemblyAction::GraspComponent => "grasp".to_string(),
            AssemblyAction::Translate { .. } => "translate".to_string(),
            AssemblyAction::Rotate { .. } => "rotate".to_string(),
            AssemblyAction::Dock => "dock".to_string(),
            AssemblyAction::Bolt { torque_nm } => format!("bolt {torque_nm} N·m"),
            AssemblyAction::Weld { process } => format!("weld {process}"),
            AssemblyAction::ConnectFluid { fluid } => format!("fluid {fluid}"),
            AssemblyAction::ConnectElectrical { connector } => format!("elec {connector}"),
            AssemblyAction::Release => "release".to_string(),
        };

        if matches!(step.action, AssemblyAction::Release) {
            bays_done += 1;
        }
        let check = assembly
            .verify_structural_integrity(&step.id)
            .expect("step in sequence");

        println!(
            "{:>5} | {:<16} | bay {:>2} | {:>9.2} | {:<2} | {:.4}",
            step.id.0,
            action,
            bay,
            result.interface_force_n,
            if result.ok { "Y" } else { "N" },
            check.root_stress_mpa
        );

        assert!(
            result.ok,
            "step {} violated constraints: {:?}",
            step.id.0, result.notes
        );
        assert!(
            check.passed,
            "partial structure unsound at step {}: {:?}",
            step.id.0, check.notes
        );
        if matches!(step.action, AssemblyAction::Release) {
            state.installed_components.push(step.component);
        }
        state.completed_steps.push(step.id);
    }

    // 4. Final integrity report.
    let final_check = assembly
        .verify_structural_integrity(&StepId(steps.len() as u64))
        .unwrap();
    println!(
        "\nComplete: {bays_done} bays erected, root stress {:.3} MPa (util {:.2}%), \
{} m deployed cantilever under {} N docking impulse",
        final_check.root_stress_mpa,
        final_check.utilization * 100.0,
        BAYS as f64 * PITCH - PITCH / 2.0,
        assembly.docking_impulse_n
    );
    assert_eq!(state.installed_components.len(), BAYS as usize);
    assert!(final_check.passed);
}
