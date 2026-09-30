//! Benchmark: launch statics, dynamic trajectory, and flooding sequences.
//! Criterion-managed (review 7G).

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, Criterion, Throughput};

use tpt_yard_core::{MassProperties, Vector3};
use tpt_yard_drydock::{DockedVessel, Drydock};
use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};

fn slipway(cog_x: f64) -> LaunchAnalysis {
    LaunchAnalysis {
        launch_method: LaunchMethod::Slipway {
            slope_deg: 3.0,
            ways: 2,
        },
        vessel_weight: MassProperties {
            mass_kg: 4_000_000.0,
            cog: Vector3::new(cog_x, 0.0, 6.0),
        },
        way_length_m: 120.0,
        way_width_m: 2.0,
        friction_coefficient: 0.02,
        poppet_to_cog_m: cog_x,
        end_bearing_m: 20.0,
        immersion_length_m: 90.0,
        block_coefficient: 0.8,
        breadth_m: 20.0,
        site: SiteConditions { max_sea_state: 3 },
    }
}

fn bench_launch(c: &mut Criterion) {
    c.bench_function("launch/slipway_statics", |b| {
        b.iter(|| std::hint::black_box(&slipway(70.0)).slipway_launch())
    });
    c.bench_function("launch/dynamic_trajectory", |b| {
        b.iter(|| std::hint::black_box(&slipway(70.0)).dynamic_launch())
    });

    let dock = Drydock {
        length_m: 200.0,
        width_m: 30.0,
        depth_m: 10.0,
    };
    let vessel = DockedVessel {
        launch_weight_kg: 4_000_000.0,
        cog_above_keel_m: 6.0,
        length_m: 90.0,
        breadth_m: 20.0,
        block_coefficient: 0.8,
        ballast_tanks: vec![],
    };
    let mut group = c.benchmark_group("launch/flooding_sequence");
    // Throughput = flood rate so the numbers read as m3/h processed.
    group.throughput(Throughput::Bytes(5_000));
    group.bench_function("5000_m3_hr", |b| {
        b.iter(|| dock.flooding_sequence(std::hint::black_box(&vessel), 5_000.0).unwrap())
    });
    group.finish();
}

criterion_group!(benches, bench_launch);
criterion_main!(benches);
