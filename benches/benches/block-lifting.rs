//! Benchmark: block lifting and hull block planning.
//!
//! Measures sling-load distribution and erection-sequence planning across
//! the reference container-ship division (12 blocks) and synthetic large
//! divisions. Criterion-managed (review 7G).

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, Criterion};

use tpt_yard_blocks::{cog_within_lifts, sling_loads};
use tpt_yard_core::{Dimensions, Vector3};
use tpt_yard_hull::{HullConstruction, HullGeometry};

fn bench_block_lifting(c: &mut Criterion) {
    // Single heavy pick: 12 lift points, 800 t block.
    let lifts: Vec<Vector3> = (0..12)
        .map(|i| {
            let side = i % 2;
            let row = i / 2;
            Vector3::new(row as f64 * 3.0, side as f64 * 8.0, 0.0)
        })
        .collect();
    let hook = Vector3::new(16.5, 4.0, 25.0);

    c.bench_function("lifting/sling_loads_12_legs", |b| {
        b.iter(|| {
            let loads = sling_loads(
                800.0 * 9.81,
                Vector3::new(7.5, 3.5, 0.0),
                std::hint::black_box(&lifts),
                hook,
            );
            assert_eq!(loads.len(), 12);
        })
    });
    c.bench_function("lifting/cog_within_lifts_12", |b| {
        b.iter(|| cog_within_lifts(Vector3::new(7.5, 4.0, 0.0), std::hint::black_box(&lifts)))
    });

    // Division + erection planning at three scales.
    for loa in [140.0, 300.0, 400.0] {
        let hull = HullConstruction::new(HullGeometry {
            loa_m: loa,
            boa_m: 22.0 + loa / 20.0,
            depth_m: 12.0 + loa / 50.0,
            areal_density_kg_m2: 180.0,
            depth_bands: 3,
        });
        let workshop = Dimensions::new(24.0, 40.0, 20.0);
        c.bench_function(&format!("lifting/division_erection_{loa}m"), |b| {
            b.iter(|| {
                let blocks = hull.block_division(40_000.0, workshop);
                hull.erection_sequence(&blocks).len()
            })
        });
    }
}

criterion_group!(benches, bench_block_lifting);
criterion_main!(benches);
