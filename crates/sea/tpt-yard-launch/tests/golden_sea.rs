//! Golden-data verification for slipway launch and drydock flooding.

use std::path::PathBuf;

use tpt_yard_core::{json::Value, MassProperties, Vector3};
use tpt_yard_drydock::{DockedVessel, Drydock};
use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};

fn golden(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../../test-data/golden/sea/{name}"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Value::parse(&text).expect("golden JSON parses")
}

#[test]
fn golden_slipway_launch_stability() {
    let v = golden("slipway-launch-stability.json");
    let p = v.get("parameters").expect("parameters");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let a = LaunchAnalysis {
        launch_method: LaunchMethod::Slipway {
            slope_deg: num(p, "slope_deg"),
            ways: p.get("ways").and_then(|n| n.as_u64()).unwrap() as u32,
        },
        vessel_weight: MassProperties {
            mass_kg: num(p, "launch_mass_kg"),
            cog: Vector3::new(num(p, "cog_x_from_way_end_m"), 0.0, 6.0),
        },
        way_length_m: num(p, "way_length_m"),
        way_width_m: num(p, "way_width_m"),
        friction_coefficient: num(p, "friction_coefficient"),
        poppet_to_cog_m: num(p, "cog_x_from_way_end_m"),
        end_bearing_m: num(p, "end_bearing_m"),
        immersion_length_m: num(p, "immersion_length_m"),
        block_coefficient: num(p, "block_coefficient"),
        breadth_m: num(p, "breadth_m"),
        site: SiteConditions { max_sea_state: 3 },
    };
    let r = a.slipway_launch();

    let exp = v.get("expected").expect("expected");
    let tol = v
        .get("tolerance")
        .and_then(|t| t.get("relative"))
        .and_then(|x| x.as_f64())
        .unwrap_or(0.002);
    let check = |name: &str, computed: f64, expected: f64| {
        assert!(
            ((computed - expected) / expected.abs().max(1e-9)) <= tol,
            "{name}: computed {computed} vs golden {expected}"
        );
    };
    check(
        "sliding_velocity_ms",
        r.sliding_velocity_ms,
        num(exp, "sliding_velocity_ms"),
    );
    check(
        "full_contact_pressure_mpa",
        r.way_pressure_mpa[0],
        num(exp, "full_contact_pressure_mpa"),
    );
    check(
        "pivot_pressure_mpa",
        r.way_pressure_mpa[3],
        num(exp, "pivot_pressure_mpa"),
    );
    check(
        "launch_draft_m",
        a.launch_draft_m(),
        num(exp, "launch_draft_m"),
    );
    check(
        "slamming_pressure_mpa",
        r.slamming_pressure_mpa,
        num(exp, "slamming_pressure_mpa"),
    );
    assert_eq!(r.entry_angle_deg, num(exp, "entry_angle_deg"));
    assert_eq!(
        r.tip_up_risk,
        exp.get("tip_up_risk").and_then(|b| b.as_bool()).unwrap()
    );
    assert_eq!(r.safe, exp.get("safe").and_then(|b| b.as_bool()).unwrap());
}

#[test]
fn golden_drydock_flooding_sequence() {
    let v = golden("drydock-flooding-sequence.json");
    let dock_v = v.get("dock").expect("dock");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let dock = Drydock {
        length_m: num(dock_v, "length_m"),
        width_m: num(dock_v, "width_m"),
        depth_m: num(dock_v, "depth_m"),
    };
    let vessel_v = v.get("vessel").expect("vessel");
    let vessel = DockedVessel {
        launch_weight_kg: num(vessel_v, "launch_weight_kg"),
        cog_above_keel_m: num(vessel_v, "cog_above_keel_m"),
        length_m: num(vessel_v, "length_m"),
        breadth_m: num(vessel_v, "breadth_m"),
        block_coefficient: num(vessel_v, "block_coefficient"),
        ballast_tanks: vec![],
    };
    let rate = num(&v, "flood_rate_m3_hr");
    let seq = dock.flooding_sequence(&vessel, rate).expect("floats out");

    let exp = v.get("expected").expect("expected");
    let tol = v
        .get("tolerance")
        .and_then(|t| t.get("relative"))
        .and_then(|x| x.as_f64())
        .unwrap_or(0.002);
    let check = |name: &str, computed: f64, expected: f64| {
        assert!(
            ((computed - expected) / expected.abs().max(1e-9)) <= tol,
            "{name}: computed {computed} vs golden {expected}"
        );
    };

    assert_eq!(
        seq.steps.len(),
        exp.get("n_steps").and_then(|n| n.as_u64()).unwrap() as usize
    );
    let last = seq.steps.last().unwrap();
    check(
        "final_water_level_m",
        last.water_level_m,
        num(exp, "final_water_level_m"),
    );
    check("final_draft_m", last.draft_m, num(exp, "final_draft_m"));
    check(
        "final_gm_m",
        last.gm_m.expect("afloat has GM"),
        num(exp, "final_gm_m"),
    );
    check(
        "final_displaced_kg",
        last.displaced_mass_kg,
        num(exp, "final_displaced_kg"),
    );
    check(
        "total_time_hours",
        seq.total_time_hours,
        num(exp, "total_time_hours"),
    );
    assert_eq!(
        seq.steps.iter().filter(|s| s.gm_m.is_none()).count(),
        exp.get("aground_steps").and_then(|n| n.as_u64()).unwrap() as usize
    );
    assert!(seq.stable_at_every_level);
}
