//! Milestone example (Phase 4): simulate an ISS-class truss assembly with a
//! robotic arm end-to-end — manifest, sequence plan, step simulation with
//! collision and force checks, and partial-structure integrity at every bay.

use tpt_yard_core::{json::Value, ComponentId, StepId, Vector3};
use tpt_yard_orbital_assembly::{
    AssemblyAction, AssemblyState, ComponentSpec, OrbitalAssembly, OrbitalParameters,
    SpaceStructure,
};
use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose, RoboticArm};

/// Reports a bad path or manifest as one `error:` line and a nonzero
/// exit, instead of the default panic message and backtrace hint.
fn install_error_hook() {
    std::panic::set_hook(Box::new(|info| {
        let payload = info.payload();
        let msg = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("unexpected failure");
        eprintln!("error: {msg} (check the manifest path and fields)");
    }));
}

fn main() {
    install_error_hook();
    // 1. The truss manifest (review 7F: examples read their test data and
    //    accept a path argument). Defaults to the repo's ISS reference
    //    manifest; pass another manifest path to plan a different truss.
    let manifest_path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-data/orbital-structures/iss-truss-manifest.json"
        )
        .to_string()
    });
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading {manifest_path}: {e}"));
    let manifest = Value::parse(&text).expect("manifest parses");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);

    let bays = manifest
        .get("segments")
        .and_then(|n| n.as_u64())
        .expect("segments");
    let pitch = num(&manifest, "bay_pitch_m");
    let bay = manifest.get("bay").expect("bay");
    let (bay_mass, dims) = {
        let d = bay
            .get("dimensions_m")
            .and_then(|d| d.as_array())
            .expect("dims");
        (
            num(bay, "mass_kg"),
            Vector3::new(
                d[0].as_f64().unwrap(),
                d[1].as_f64().unwrap(),
                d[2].as_f64().unwrap(),
            ),
        )
    };
    let mut orbit = OrbitalParameters::default();
    if let Some(o) = manifest.get("orbit") {
        if let Some(a) = o.get("station_keeping_accel_ms2").and_then(|n| n.as_f64()) {
            orbit.station_keeping_accel_ms2 = a;
        }
    }

    let mut assembly = OrbitalAssembly::new(
        SpaceStructure::Truss {
            segments: bays as u32,
            length_m: bays as f64 * pitch,
        },
        orbit,
    );
    for i in 1..=bays {
        assembly.add_component(ComponentSpec {
            id: ComponentId(i),
            name: format!("bay {i}"),
            mass_kg: bay_mass,
            dimensions: dims,
            target_position: Vector3::new(pitch * i as f64 - pitch / 2.0, 0.0, 0.0),
        });
    }
    let robot_spec = manifest.get("robot").expect("robot");
    let links = robot_spec
        .get("links_m")
        .and_then(|l| l.as_array())
        .expect("links")
        .iter()
        .map(|n| n.as_f64().unwrap())
        .collect::<Vec<_>>();
    let force_n = num(robot_spec, "gripper_force_n");
    let mut robot = RoboticArm::new(
        tpt_yard_core::RobotId(1),
        links
            .iter()
            .map(|_| Joint::revolute(-3.0, 3.0, 0.5))
            .collect(),
        links.clone(),
        EndEffector::Gripper { force_n },
    );
    robot.base = Pose::origin();
    assembly.add_robot(robot);
    // Assembly-parameterised cross-section (review 7B: no hard-codeds).
    assembly.chord_area_m2 = num(bay, "chord_area_m2");
    assembly.bay_height_m = num(bay, "bay_height_m");

    // 2. Plan the sequence: anchor bay first, build outwards.
    let steps = assembly.plan_sequence().expect("manifest plan is acyclic");
    println!(
        "Truss: {bays} bays x {pitch} m (from {manifest_path}), {} assembly steps",
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
        bays as f64 * pitch - pitch / 2.0,
        assembly.docking_impulse_n
    );
    assert_eq!(state.installed_components.len(), bays as usize);
    assert!(final_check.passed);
}
