//! Benchmark: welding simulation chain on the reference hull panel.
//!
//! Measures the full analytical pipeline — Rosenthal thermal cycles,
//! residual-stress field, distortion, and sequence ranking — on the same
//! panel as `test-data/golden/sea/welding-distortion-panel.json`.
//! Criterion-managed (review 7G: baselines + regression gates).

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, Criterion};

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

fn bench_welding(c: &mut Criterion) {
    let sim = reference_panel();

    c.bench_function("welding/thermal_cycle_8mm", |b| {
        b.iter(|| std::hint::black_box(&sim).thermal_cycle(8.0).unwrap())
    });
    c.bench_function("welding/thermal_cycle_20mm", |b| {
        b.iter(|| std::hint::black_box(&sim).thermal_cycle(20.0).unwrap())
    });
    c.bench_function("welding/residual_stress", |b| {
        b.iter(|| std::hint::black_box(&sim).residual_stress().unwrap())
    });
    c.bench_function("welding/distortion", |b| {
        b.iter(|| std::hint::black_box(&sim).distortion().unwrap())
    });
    c.bench_function("welding/sequence_optimization", |b| {
        let mk = |i: u32, x: f64, dir| WeldPass {
            id: i,
            heat_input_kj_mm: 6.0,
            direction: dir,
            start_mm: Vector3::ZERO,
            end_mm: Vector3::new(x + 500.0, 0.0, 0.0),
        };
        let concentrated = vec![
            mk(1, 0.0, WeldDirection::Forward),
            mk(2, 0.0, WeldDirection::Forward),
            mk(3, 0.0, WeldDirection::Forward),
        ];
        let balanced = vec![
            mk(1, 0.0, WeldDirection::Forward),
            mk(2, 600.0, WeldDirection::Reverse),
            mk(3, 1200.0, WeldDirection::Forward),
        ];
        b.iter(|| {
            std::hint::black_box(&sim)
                .welding_sequence_optimization(&[concentrated.clone(), balanced.clone()])
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_welding);
criterion_main!(benches);
