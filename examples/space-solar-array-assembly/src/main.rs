//! Milestone example (Phase 5): plan in-space additive manufacturing of a
//! solar array end-to-end — print times, energy, heat rejection, quality
//! plan, and the robotic assembly of the printed bays into the deployed
//! wing.

use tpt_yard_core::{json::Value, ComponentId, Material, RobotId, Vector3};
use tpt_yard_orbital_assembly::{
    AssemblyState, ComponentSpec, OrbitalAssembly, OrbitalParameters, SpaceStructure,
};
use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose, RoboticArm};
use tpt_yard_space_manufacturing::{
    AdditiveTechnique, Feedstock, InSpaceManufacturing, ManufacturingProcess,
};

fn main() {
    // The array manifest (review 7F: examples read their test data and
    // accept a path argument). Defaults to the repo's reference array;
    // pass another manifest path to plan a different array.
    let manifest_path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-data/orbital-structures/solar-array-manifest.json"
        )
        .to_string()
    });
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading {manifest_path}: {e}"));
    let manifest = Value::parse(&text).expect("manifest parses");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let panel_v = manifest.get("panel").expect("panel");
    let panels = manifest
        .get("panels")
        .and_then(|n| n.as_u64())
        .expect("panels");
    let panel_mass_kg = num(panel_v, "mass_kg");
    let panel_length_m = num(panel_v, "length_m");
    let panel_width_m = num(panel_v, "width_m");
    let substrate_m = num(panel_v, "substrate_thickness_m");
    let print_rate = num(panel_v, "print_rate_m2_hr");
    // Print volume of one panel substrate: length x width x thickness.
    let panel_volume_m3 = panel_length_m * panel_width_m * substrate_m;
    let robot_v = manifest.get("robot").expect("robot");
    let links: Vec<f64> = robot_v
        .get("links_m")
        .and_then(|l| l.as_array())
        .expect("links")
        .iter()
        .map(|n| n.as_f64().expect("link length"))
        .collect();
    let gripper_force_n = num(robot_v, "gripper_force_n");

    // 1. The printer: wire-arc additive for the panel substrates.
    let printer = InSpaceManufacturing::new(
        ManufacturingProcess::AdditiveManufacturing {
            technique: AdditiveTechnique::WireArcAdditive,
            material: "AA5083".into(),
        },
        Feedstock::earth_launched(Material::aa5083().density_kg_m3),
        Material::aa5083(),
    );

    println!(
        "== Solar array: {panels} panels, wire-arc additive substrates (from {manifest_path}) =="
    );
    let print_hours = printer
        .print_time_estimate(panel_volume_m3, print_rate)
        .expect("valid inputs");
    let energy = printer.energy_kwh(panel_volume_m3).unwrap();
    let thermal = printer.thermal_control_during_print(print_rate).unwrap();
    let quality = printer.quality_verification();
    println!(
        "  per panel: {:.1} kg feedstock, {:.1} h print, {:.0} kWh",
        printer.feedstock_mass_kg(panel_volume_m3),
        print_hours,
        energy
    );
    println!(
        "  thermal: {:.0} kW deposition -> {:.1} m2 radiator, eclipse pauses: {}",
        thermal.deposition_power_kw, thermal.radiator_area_m2, thermal.eclipse_pauses
    );
    println!(
        "  quality: {:}% layer imaging, in-situ {:}",
        quality.layer_imaging_pct,
        quality.in_situ_ndt.join(" + ")
    );
    let total_print_hours = print_hours * panels as f64;
    println!(
        "  batch: {panels} panels = {:.0} h ({:.0} days of printing)",
        total_print_hours,
        total_print_hours / 24.0
    );

    // 2. Robotic assembly of the printed panels into the deployed wing.
    let mut assembly = OrbitalAssembly::new(
        SpaceStructure::SolarArray {
            panel_count: panels as u32,
            area_m2: panel_length_m * panel_width_m * panels as f64,
        },
        OrbitalParameters::default(),
    );
    for i in 1..=panels {
        assembly.add_component(ComponentSpec {
            id: ComponentId(i),
            name: format!("panel {i}"),
            mass_kg: panel_mass_kg,
            dimensions: Vector3::new(panel_length_m, panel_width_m, 0.05),
            target_position: Vector3::new(
                panel_length_m / 2.0 + panel_length_m * (i - 1) as f64,
                0.0,
                0.0,
            ),
        });
    }
    let mut robot = RoboticArm::new(
        RobotId(1),
        links
            .iter()
            .map(|_| Joint::revolute(-3.0, 3.0, 0.5))
            .collect(),
        links.clone(),
        EndEffector::Gripper {
            force_n: gripper_force_n,
        },
    );
    robot.base = Pose::origin();
    assembly.add_robot(robot);

    let steps = assembly.plan_sequence();
    let mut state = AssemblyState::default();
    let mut all_ok = true;
    for step in &steps {
        let result = assembly.simulate_step(step, &state).expect("valid step");
        all_ok &= result.ok;
        if matches!(
            step.action,
            tpt_yard_orbital_assembly::AssemblyAction::Release
        ) {
            state.installed_components.push(step.component);
        }
        state.completed_steps.push(step.id);
    }
    let total_assembly_hours: f64 = steps.iter().map(|s| s.duration_hours).sum();
    let check = assembly
        .verify_structural_integrity(&tpt_yard_core::StepId(steps.len() as u64))
        .unwrap();
    println!(
        "\n  assembly: {} steps, {:.1} h, all constraints ok: {}",
        steps.len(),
        total_assembly_hours,
        if all_ok { "yes" } else { "NO" }
    );
    println!(
        "  deployed wing: {} m2, root stress {:.3} MPa (util {:.1}%)",
        panel_length_m * panel_width_m * panels as f64,
        check.root_stress_mpa,
        check.utilization * 100.0
    );

    // 3. Milestone summary.
    let total_days = (total_print_hours + total_assembly_hours) / 24.0;
    println!(
        "\nMilestone: in-space additive manufacturing of a solar array planned end-to-end: \
{panels} panels, {:.0} kg feedstock, {:.0} kWh, total {:.1} days",
        printer.feedstock_mass_kg(panel_volume_m3) * panels as f64,
        energy * panels as f64,
        total_days
    );
    assert!(all_ok);
    assert!(check.passed);
    assert_eq!(state.installed_components.len(), panels as usize);
}
