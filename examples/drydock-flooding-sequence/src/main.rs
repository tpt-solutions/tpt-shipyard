//! Example: drydock flooding sequence — level-by-level stability of a
//! 4,000 t vessel floating out of a 200 m building dock.

use tpt_yard_core::{MassProperties, Vector3};
use tpt_yard_drydock::Drydock;
use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};
use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

fn main() {
    let dock = Drydock {
        length_m: 200.0,
        width_m: 30.0,
        depth_m: 10.0,
    };

    // Launch weight from the digital twin's weight model (as-built).
    let mut weight = WeightModel::new(4_000_000.0, Vector3::new(45.0, 0.0, 6.0));
    weight.add_item(WeightItem {
        id: tpt_yard_core::ItemId(1),
        name: "hull + machinery".into(),
        group: "lightship".into(),
        weight_kg: 3_700_000.0,
        cog: Vector3::new(45.0, 0.0, 5.6),
        status: ItemStatus::Installed,
        margin_pct: 0.0,
        installed_by: None,
    });
    weight.add_item(WeightItem {
        id: tpt_yard_core::ItemId(2),
        name: "outfit afloat items".into(),
        group: "outfit".into(),
        weight_kg: 300_000.0,
        cog: Vector3::new(48.0, 0.0, 8.5),
        status: ItemStatus::Installed,
        margin_pct: 0.0,
        installed_by: None,
    });

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
        immersion_length_m: 90.0,
        block_coefficient: 0.8,
        breadth_m: 20.0,
        site: SiteConditions { max_sea_state: 3 },
    };

    println!(
        "Float-out: {:.0} t, KG {:.2} m, Lx90 Bx20 Cb0.8 into a 200x30x10 m dock at 5,000 m³/h",
        weight.installed_weight() / 1000.0,
        cog.z
    );

    let seq = launch_analysis
        .drydock_flooding(&dock, 5_000.0)
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
