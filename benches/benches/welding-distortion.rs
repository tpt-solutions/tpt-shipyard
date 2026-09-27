//! Benchmark: welding simulation chain on the reference hull panel.
//!
//! Measures the full analytical pipeline — Rosenthal thermal cycles,
//! residual-stress field, distortion, and sequence ranking — on the same
//! panel as `test-data/golden/sea/welding-distortion-panel.json`.

use std::time::Instant;

use tpt_yard_core::{Material, Vector3};
use tpt_yard_joints::{GrooveType, JointGeometry, JointKind};
use tpt_yard_welding::{WeldDirection, WeldPass, WeldProcedure, WeldProcess, WeldingSimulation};

fn reference_panel() -> WeldingSimulation {
    let procedure = WeldProcedure {
        process: WeldProcess::Saw,
        heat_input_kj_mm: 12.0,
        travel_speed_mm_s: 8.0,
        preheat_temp_c: 20.0,
        interpass_temp_c: 150.0,
        filler_metal: "S2Si2 / SA AB1 47".into(),
        sequence: vec![],
    };
    let joint = JointGeometry::new(JointKind::Butt)
        .with_thickness_mm(12.0)
        .with_groove(GrooveType::V)
        .with_groove_angle_deg(60.0)
        .with_root_gap_mm(3.0)
        .with_root_face_mm(2.0)
        .with_length_mm(1000.0);
    WeldingSimulation::new(procedure, Material::ah36(), joint)
}

fn bench(name: &str, iterations: u32, mut f: impl FnMut()) {
    // Warm-up.
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

fn main() {
    println!("== tpt-shipyard: welding-distortion benchmark (reference AH36 panel) ==");
    let sim = reference_panel();

    bench("thermal_cycle (d = 8 mm)", 200, || {
        let _ = sim.thermal_cycle(8.0).unwrap();
    });

    bench("thermal_cycle (d = 20 mm)", 200, || {
        let _ = sim.thermal_cycle(20.0).unwrap();
    });

    bench("residual_stress", 500, || {
        let _ = sim.residual_stress().unwrap();
    });

    bench("distortion", 500, || {
        let _ = sim.distortion().unwrap();
    });

    // Multi-pass sequence ranking with 8 candidate sequences of 6 passes.
    let candidates: Vec<Vec<WeldPass>> = (0..8)
        .map(|seed| {
            (0..6)
                .map(|i| WeldPass {
                    id: i + 1,
                    heat_input_kj_mm: 3.0 + (i % 3) as f64,
                    direction: if (i + seed) % 2 == 0 {
                        WeldDirection::Forward
                    } else {
                        WeldDirection::Reverse
                    },
                    start_mm: Vector3::new((i * 150) as f64, 0.0, 0.0),
                    end_mm: Vector3::new((i * 150 + 120) as f64, 0.0, 0.0),
                })
                .collect()
        })
        .collect();
    bench("sequence_optimization (8x6 passes)", 50, || {
        let _ = sim.welding_sequence_optimization(&candidates).unwrap();
    });

    bench("distortion_of_sequence (6 passes)", 500, || {
        let mut s = sim.clone();
        s.weld_procedure.sequence = candidates[0].clone();
        let _ = s.distortion_of_sequence().unwrap();
    });
}
