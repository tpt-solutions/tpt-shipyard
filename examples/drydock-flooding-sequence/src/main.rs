//! Example: drydock flooding sequence — level-by-level stability of a
//! 4,000 t vessel floating out of a 200 m building dock.

use tpt_yard_core::{MassProperties, Vector3};
use tpt_yard_drydock::Drydock;
use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};
use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

/// Reports a bad path or manifest as one `error:` line and a nonzero
/// exit, instead of the default panic message and backtrace hint.
fn install_error_hook() {
    std::panic::set_hook(Box::new(|info| {
        let payload = info.payload();
        let msg = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("unexpected failure");
        eprintln!("error: {msg} (check the manifest path and fields)");
    }));
}

fn main() {
    install_error_hook();
    // The float-out case (review 7F: examples read their test data and
    // accept a path argument). Defaults to the golden drydock case; pass
    // another case file to float out a different vessel.
    let manifest_path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-data/golden/sea/drydock-flooding-sequence.json"
        )
        .to_string()
    });
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading {manifest_path}: {e}"));
    let manifest = tpt_yard_core::json::Value::parse(&text).expect("manifest parses");
    let num = |o: &tpt_yard_core::json::Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let dock_v = manifest.get("dock").expect("dock");
    let vessel_v = manifest.get("vessel").expect("vessel");
    let rate = num(&manifest, "flood_rate_m3_hr");
    let dock = Drydock {
        length_m: num(dock_v, "length_m"),
        width_m: num(dock_v, "width_m"),
        depth_m: num(dock_v, "depth_m"),
    };

    // Launch weight from the digital twin's weight model (as-built): one
    // installed item at the manifest's launch weight and KG, at half the
    // vessel length.
    let launch_weight = num(vessel_v, "launch_weight_kg");
    let kg = num(vessel_v, "cog_above_keel_m");
    let l_v = num(vessel_v, "length_m");
    let mut weight = WeightModel::new(launch_weight, Vector3::new(l_v / 2.0, 0.0, kg));
    weight
        .add_item(WeightItem {
            id: tpt_yard_core::ItemId(1),
            name: "hull + machinery + outfit afloat".into(),
            group: "lightship".into(),
            weight_kg: launch_weight,
            cog: Vector3::new(l_v / 2.0, 0.0, kg),
            status: ItemStatus::Installed,
            margin_pct: 0.0,
            installed_by: None,
        })
        .expect("valid weight item");

    let cog = weight
        .installed_centre_of_gravity()
        .expect("installed mass exists");
    let launch_analysis = LaunchAnalysis {
        launch_method: LaunchMethod::DrydockFlooding,
        vessel_weight: MassProperties {
            mass_kg: weight.installed_weight(),
            cog,
        },
        way_length_m: 0.0,
        way_width_m: 0.0,
        friction_coefficient: 0.0,
        poppet_to_cog_m: 0.0,
        end_bearing_m: 0.0,
        immersion_length_m: l_v,
        block_coefficient: num(vessel_v, "block_coefficient"),
        breadth_m: num(vessel_v, "breadth_m"),
        site: SiteConditions { max_sea_state: 3 },
    };

    println!(
        "Float-out: {:.0} t, KG {:.2} m, Lx{:.0} Bx{:.0} Cb{:.2} into a {:.0}x{:.0}x{:.0} m dock at {rate:.0} m³/h (from {manifest_path})",
        weight.installed_weight() / 1000.0,
        cog.z,
        l_v,
        num(vessel_v, "breadth_m"),
        num(vessel_v, "block_coefficient"),
        dock.length_m,
        dock.width_m,
        dock.depth_m
    );

    let seq = launch_analysis
        .drydock_flooding(&dock, rate)
        .expect("the vessel must fit and float out");

    println!("\n level | draft | displaced |   GM   | state       | note");
    for step in &seq.steps {
        println!(
            "{:>6.2} | {:>5.2} | {:>7.0} t | {:>6} | {:<11} | {}",
            step.water_level_m,
            step.draft_m,
            step.displaced_mass_kg / 1000.0,
            step.gm_m
                .map(|g| format!("{g:.2} m"))
                .unwrap_or_else(|| "-".into()),
            if step.gm_m.is_none() {
                "aground"
            } else {
                "afloat"
            },
            step.notes.join("; ")
        );
    }

    println!(
        "\nTotal flooding time {:.1} h; stable at every level: {}",
        seq.total_time_hours, seq.stable_at_every_level
    );
    assert!(seq.stable_at_every_level);
    let stability = launch_analysis.launch_stability(&weight);
    println!(
        "Post float-out stability: GM {:.2} m at draft {:.2} m — {}",
        stability.gm_m,
        stability.draft_m,
        if stability.stable {
            "STABLE"
        } else {
            "UNSTABLE"
        }
    );
    assert!(stability.stable);
}
