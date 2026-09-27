//! Benchmark: launch statics and flooding sequences.

use std::time::Instant;

use tpt_yard_core::{MassProperties, Vector3};
use tpt_yard_drydock::{DockedVessel, Drydock};
use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};

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

fn main() {
    println!("== tpt-shipyard: launch-stability benchmark ==");

    bench("slipway_launch (full statics chain)", 2_000, || {
        let r = slipway(70.0).slipway_launch();
        assert!(r.safe);
    });

    bench("slipway_launch (tip-up case)", 2_000, || {
        let r = slipway(40.0).slipway_launch();
        assert!(r.tip_up_risk);
    });

    let dock = Drydock {
        length_m: 200.0,
        width_m: 30.0,
        depth_m: 10.0,
    };
    let vessel = DockedVessel {
        launch_weight_kg: 6_000_000.0,
        cog_above_keel_m: 5.5,
        length_m: 140.0,
        breadth_m: 22.0,
        block_coefficient: 0.85,
        ballast_tanks: vec![],
    };
    bench("drydock flooding_sequence (13 levels)", 2_000, || {
        let seq = dock.flooding_sequence(&vessel, 5_000.0).unwrap();
        assert!(seq.stable_at_every_level);
    });

    bench("launch_stability via weight model", 2_000, || {
        let a = slipway(70.0);
        let s = a.launch_stability(&tpt_yard_weight::WeightModel::new(
            4_000_000.0,
            Vector3::new(70.0, 0.0, 6.0),
        ));
        assert!(s.stable);
    });
}
