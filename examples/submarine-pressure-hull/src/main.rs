//! Example: submarine pressure-hull section planning — ring-section
//! division, circumferential weld joints, and a workshop weld-procedure
//! check of the full-penetration circumferential seam.

use tpt_yard_core::{Material, PhaseId, Vector3};
use tpt_yard_joints::{GrooveType, JointGeometry, JointKind};
use tpt_yard_structural::{
    ConstructionLoad, ConstructionStructuralSolver, PartialElement, PartialStructure, Support,
};
use tpt_yard_welding::{WeldProcedure, WeldProcess, WeldingSimulation};

const RING_COUNT: usize = 12;

fn main() {
    // 1. Ring-section division of the 55 m pressure hull (test-data/
    //    hull-blocks/submarine-pressure-hull.json).
    let length = 55.0;
    let ring_len = length / RING_COUNT as f64;
    println!("Pressure hull: {RING_COUNT} ring sections of {ring_len:.2} m, Ø8.0 m, 40 mm shell");

    // 2. The circumferential seam: single-V full-penetration butt, 40 mm.
    let seam = JointGeometry::new(JointKind::Butt)
        .with_thickness_mm(40.0)
        .with_groove(GrooveType::DoubleV)
        .with_groove_angle_deg(60.0)
        .with_root_gap_mm(2.0)
        .with_root_face_mm(2.0)
        .with_length_mm(8_000.0 / std::f64::consts::PI) // circumference = πD
        .with_length_mm(std::f64::consts::PI * 8000.0);
    println!(
        "Circumferential seam: double-V 60°, weld area {:.0} mm², {:.1} kg of filler per seam",
        seam.weld_area_mm2_total(),
        seam.weld_mass_kg(7850.0)
    );

    // 3. SAW procedure check on the seam: thermal cycle + distortion.
    let procedure = WeldProcedure {
        process: WeldProcess::Saw,
        heat_input_kj_mm: 18.0,
        travel_speed_mm_s: 6.0,
        preheat_temp_c: 80.0,
        interpass_temp_c: 180.0,
        filler_metal: "S3NiMo / SA AB1 47".into(),
        sequence: vec![],
    };
    let sim = WeldingSimulation::new(procedure, Material::ah36(), seam.clone());
    let cycle = sim.thermal_cycle(10.0).expect("valid procedure");
    let distortion = sim.distortion().expect("valid procedure");
    println!(
        "Weld HAZ @10 mm: peak {:.0} °C, t8/5 {:.1} s (target 12–25 s for HY-80 class)",
        cycle.peak_temp_c,
        cycle.t8_5_s.unwrap_or(0.0)
    );
    println!(
        "Shrinkage per seam: transverse {:.2} mm, angular {:.2}°",
        distortion.transverse_shrinkage_mm, distortion.angular_distortion_deg
    );

    // 4. Structural check of a partial ring assembly in the workshop:
    //    two rings joined, cradled at four saddles, hoisting the next ring.
    let mut structure = PartialStructure::default();
    // Simple cradle model: hull section as a beam between two saddle pairs.
    structure.nodes = vec![
        Vector3::new(0.0, 0.0, 0.0),            // 0 saddle A (port)
        Vector3::new(0.0, 2.0, 0.0),            // 1 saddle A (stbd)
        Vector3::new(ring_len * 4.0, 0.0, 0.0), // 2 saddle B (port)
        Vector3::new(ring_len * 4.0, 2.0, 0.0), // 3 saddle B (stbd)
        Vector3::new(0.0, 1.0, 4.0),            // 4 hull node A
        Vector3::new(ring_len * 4.0, 1.0, 4.0), // 5 hull node B
    ];
    structure.elements = vec![
        leg(&structure, 0, 4),
        leg(&structure, 1, 4),
        leg(&structure, 2, 5),
        leg(&structure, 3, 5),
        PartialElement {
            nodes: [4, 5],
            area_m2: 0.05,
            youngs_modulus_gpa: 210.0,
            density_kg_m3: 0.0,
            erected_at: PhaseId(1),
        },
        PartialElement {
            nodes: [0, 1],
            area_m2: 0.02,
            youngs_modulus_gpa: 210.0,
            density_kg_m3: 0.0,
            erected_at: PhaseId(1),
        },
        PartialElement {
            nodes: [2, 3],
            area_m2: 0.02,
            youngs_modulus_gpa: 210.0,
            density_kg_m3: 0.0,
            erected_at: PhaseId(1),
        },
    ];
    structure.supports = vec![
        Support::pinned(0),
        Support::pinned(1),
        Support::pinned(2),
        Support::pinned(3),
        // Longitudinal chocks: without them the whole cradle + hull can
        // slide as a rigid body along the workshop X axis.
        Support {
            node: 4,
            fix_x: true,
            fix_y: false,
            fix_z: false,
        },
    ];
    let solver = ConstructionStructuralSolver::new(structure, 355.0);
    let result = solver
        .analyze_at_phase(
            PhaseId(1),
            &[
                ConstructionLoad::Gravity,
                ConstructionLoad::CraneLoad {
                    capacity_kn: ring_weight_kn() * 0.5,
                    dynamic_factor: 1.15,
                    node: 5,
                },
            ],
        )
        .expect("cradle model solves");
    println!(
        "Workshop cradle: {} members, max stress {:.1} MPa (util {:.2}), max deflection {:.2} mm — {}",
        result.active_members,
        result.max_axial_stress_mpa,
        result.max_utilization,
        result.max_displacement_mm,
        if result.passed { "OK" } else { "OVERSTRESSED" }
    );

    println!(
        "\nSection plan: weld all {RING_COUNT} rings into 4 sections (3 seams each) in the \
workshop, pressure-test, then join the sections in the dock — 3 circumferential \
dock joints with 100% RT."
    );
}

fn leg(s: &PartialStructure, from: usize, to: usize) -> PartialElement {
    let len = s.nodes[from].distance(s.nodes[to]);
    PartialElement {
        nodes: [from, to],
        area_m2: (0.01 * len / 4.0).max(0.004),
        youngs_modulus_gpa: 210.0,
        density_kg_m3: 0.0,
        erected_at: PhaseId(1),
    }
}

fn ring_weight_kn() -> f64 {
    // ~700 t per ring of a 40 mm, Ø8 m, 4.6 m ring (stiffened).
    700_000.0 * 9.81 / 1000.0
}
