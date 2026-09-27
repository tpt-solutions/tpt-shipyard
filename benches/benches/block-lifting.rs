//! Benchmark: block lifting and hull block planning.
//!
//! Measures sling-load distribution and erection-sequence planning across
//! the reference container-ship division (12 blocks) and synthetic large
//! divisions.

use std::time::Instant;

use tpt_yard_blocks::{cog_within_lifts, sling_loads};
use tpt_yard_core::{Dimensions, Vector3};
use tpt_yard_hull::{HullConstruction, HullGeometry};

fn bench(name: &str, iterations: u32, mut f: impl FnMut()) {
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
    println!("== tpt-shipyard: block-lifting benchmark ==");

    // Single heavy pick: 12 lift points, 800 t block.
    let lifts: Vec<Vector3> = (0..12)
        .map(|i| {
            let side = i % 2;
            let row = i / 2;
            Vector3::new(row as f64 * 3.0, side as f64 * 8.0, 0.0)
        })
        .collect();
    let hook = Vector3::new(16.5, 4.0, 25.0);

    bench("sling_loads (12 legs, 800 t)", 2_000, || {
        let loads = sling_loads(800.0 * 9.81, Vector3::new(7.5, 3.5, 0.0), &lifts, hook);
        assert_eq!(loads.len(), 12);
    });

    bench("cog_within_lifts (12 legs)", 2_000, || {
        assert!(cog_within_lifts(Vector3::new(7.5, 4.0, 0.0), &lifts));
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
        let name = format!("division + erection ({:.0} m, 3 tiers)", loa);
        bench(&name, 200, || {
            let mut h = HullConstruction::new(hull.hull_form);
            h.blocks = h.block_division(40_000.0, workshop);
            let joins = h.erection_sequence(&h.blocks);
            assert!(!joins.is_empty());
        });
    }
}
