//! Milestone example (Phase 5): plan in-space additive manufacturing of a
//! solar array end-to-end — print times, energy, heat rejection, quality
//! plan, and the robotic assembly of the printed bays into the deployed
//! wing.

use tpt_yard_core::{ComponentId, Material, RobotId, Vector3};
use tpt_yard_orbital_assembly::{
    AssemblyState, ComponentSpec, OrbitalAssembly, OrbitalParameters, SpaceStructure,
};
use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose, RoboticArm};
use tpt_yard_space_manufacturing::{
    AdditiveTechnique, Feedstock, InSpaceManufacturing, ManufacturingProcess,
};

const PANELS: u64 = 8;
const PANEL_MASS_KG: f64 = 85.0;
/// Substrate area of one panel: 12 m x 3 m x 4 mm aluminium.
const PANEL_VOLUME_M3: f64 = 12.0 * 3.0 * 0.004;

fn main() {
    // 1. The printer: wire-arc additive for the panel substrates.
    let printer = InSpaceManufacturing::new(
        ManufacturingProcess::AdditiveManufacturing {
            technique: AdditiveTechnique::WireArcAdditive,
            material: "AA5083".into(),
        },
        Feedstock::earth_launched(Material::aa5083().density_kg_m3),
        Material::aa5083(),
    );

    println!("== Solar array: {PANELS} panels, wire-arc additive substrates ==");
    let print_hours = printer
        .print_time_estimate(PANEL_VOLUME_M3, 6.0)
        .expect("valid inputs");
    let energy = printer.energy_kwh(PANEL_VOLUME_M3).unwrap();
    let thermal = printer.thermal_control_during_print(6.0).unwrap();
    let quality = printer.quality_verification();
    println!(
        "  per panel: {:.1} kg feedstock, {:.1} h print, {:.0} kWh",
        printer.feedstock_mass_kg(PANEL_VOLUME_M3),
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
    let total_print_hours = print_hours * PANELS as f64;
    println!(
        "  batch: {PANELS} panels = {:.0} h ({:.0} days of printing)",
        total_print_hours,
        total_print_hours / 24.0
    );

    // 2. Robotic assembly of the printed panels into the deployed wing.
    let mut assembly = OrbitalAssembly::new(
        SpaceStructure::SolarArray {
            panel_count: PANELS as u32,
            area_m2: 12.0 * 3.0 * PANELS as f64,
        },
        OrbitalParameters::default(),
    );
    for i in 1..=PANELS {
        assembly.add_component(ComponentSpec {
            id: ComponentId(i),
            name: format!("panel {i}"),
            mass_kg: PANEL_MASS_KG,
            dimensions: Vector3::new(12.0, 3.0, 0.05),
            target_position: Vector3::new(6.0 + 12.0 * (i - 1) as f64, 0.0, 0.0),
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
        12.0 * 3.0 * PANELS as f64,
        check.root_stress_mpa,
        check.utilization * 100.0
    );

    // 3. Milestone summary.
    let total_days = (total_print_hours + total_assembly_hours) / 24.0;
    println!(
        "\nMilestone: in-space additive manufacturing of a solar array planned end-to-end — \
{PANELS} panels, {:.0} kg feedstock, {:.0} kWh, total {:.1} days",
        printer.feedstock_mass_kg(PANEL_VOLUME_M3) * PANELS as f64,
        energy * PANELS as f64,
        total_days
    );
    assert!(all_ok);
    assert!(check.passed);
    assert_eq!(state.installed_components.len(), PANELS as usize);
}
