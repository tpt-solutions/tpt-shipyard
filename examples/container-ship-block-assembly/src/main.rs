//! Milestone example (Phase 3): plan the block erection sequence for a
//! container ship end-to-end — division, weights, erection order, digital
//! twin advance with weight/CoG tracking.

use tpt_yard_core::{
    ActivityId, ConstructionMethod, Dimensions, ItemId, PhaseId, ProjectId, SeaVesselType,
    VesselProject, VesselType,
};
use tpt_yard_digital_twin::{DigitalTwin, SupportCondition};
use tpt_yard_hull::{HullConstruction, HullGeometry};
use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

fn main() {
    // The block-division manifest (review 7F: examples read their test data
    // and accept a path argument). Defaults to the repo's 140 m container
    // ship; pass another manifest path to plan a different hull.
    let manifest_path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-data/hull-blocks/container-ship-140m.json"
        )
        .to_string()
    });
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading {manifest_path}: {e}"));
    let manifest = tpt_yard_core::json::Value::parse(&text).expect("manifest parses");
    let num = |o: &tpt_yard_core::json::Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let hull_v = manifest.get("hull").expect("hull");
    let yard = manifest
        .get("yard_capabilities")
        .expect("yard_capabilities");
    let workshop_v = yard.get("workshop").expect("workshop");
    let vessel_name = manifest
        .get("vessel")
        .and_then(|s| s.as_str())
        .unwrap_or("Container ship")
        .to_string();

    let crane_kn = num(yard, "crane_capacity_kn");
    let workshop = Dimensions::new(
        num(workshop_v, "length_m"),
        num(workshop_v, "breadth_m"),
        num(workshop_v, "depth_m"),
    );
    let loa = num(hull_v, "loa_m");

    // 1. Divide the hull into erection blocks.
    let mut hull = HullConstruction::new(HullGeometry {
        loa_m: loa,
        boa_m: num(hull_v, "boa_m"),
        depth_m: num(hull_v, "depth_m"),
        areal_density_kg_m2: num(hull_v, "areal_density_kg_m2"),
        depth_bands: hull_v
            .get("depth_bands")
            .and_then(|n| n.as_u64())
            .expect("depth_bands") as u32,
    });
    hull.blocks = hull.block_division(crane_kn, workshop);
    let joins = hull.erection_sequence(&hull.blocks);

    println!(
        "{vessel_name} (from {manifest_path}): {} blocks ({:.0} t steel), crane {:.0} kN, workshop {:.0}x{:.0}x{:.0} m",
        hull.blocks.len(),
        hull.total_steel_kg() / 1000.0,
        crane_kn,
        workshop.length,
        workshop.breadth,
        workshop.depth
    );

    // 2. Build the project: one phase per erection step (keel tier, then
    //    upper tier), one activity + weight item per block.
    let n = hull.blocks.len();
    let mut phases = Vec::new();
    let mut weight = WeightModel::new(
        hull.total_steel_kg(),
        tpt_yard_core::Vector3::new(loa / 2.0, 0.0, 6.0),
    );
    for (seq, join) in joins.iter().enumerate() {
        let block = hull.blocks.iter().find(|b| b.id == join.block).unwrap();
        let activity_id = ActivityId(seq as u64 + 1);
        let deps: Vec<ActivityId> = if seq == 0 {
            vec![]
        } else {
            vec![ActivityId(seq as u64)]
        };

        let mut phase = tpt_yard_core::BuildPhase::new(
            PhaseId(seq as u64 + 1),
            format!("Erect {} (tier {})", block.name, block.z_band + 1),
            2.0,
        );
        phase.activities.push(
            tpt_yard_core::AssemblyActivity::new(
                activity_id,
                format!("Erect {}", block.name),
                tpt_yard_core::ActivityType::JoinBlock,
                6.0,
            )
            .with_dependencies(&deps),
        );
        weight
            .add_item(WeightItem {
                id: ItemId(seq as u64 + 1),
                name: block.name.clone(),
                group: "hull".into(),
                weight_kg: block.weight_kg,
                cog: block.cog,
                status: ItemStatus::Design,
                margin_pct: 2.0,
                installed_by: Some(activity_id),
            })
            .expect("valid weight item");
        phases.push(phase);
    }

    let project = VesselProject::new(
        ProjectId(1),
        "Container ship 1400 TEU",
        VesselType::Sea(SeaVesselType::ContainerShip { teu_capacity: 1400 }),
        ConstructionMethod::SeaDrydock,
        phases,
    )
    .expect("non-empty phase plan");

    // 3. Advance the twin through the whole erection, on keel blocks.
    let mut twin = DigitalTwin::with_weight_model(
        project,
        weight,
        SupportCondition::KeelBlocks {
            // Keel blocks 3.6% of LOA beyond each end, at ~B/5.5 off the
            // centreline (the 5 m / 4 m offsets of the 140 m reference,
            // scaled with the hull).
            positions: {
                let overhang = 0.0357 * loa;
                let half_track = num(hull_v, "boa_m") / 5.5;
                vec![
                    tpt_yard_core::Vector3::new(-overhang, -half_track, 0.0),
                    tpt_yard_core::Vector3::new(-overhang, half_track, 0.0),
                    tpt_yard_core::Vector3::new(loa + overhang, -half_track, 0.0),
                    tpt_yard_core::Vector3::new(loa + overhang, half_track, 0.0),
                ]
            },
        },
    );

    println!("\n step | block | tier | installed [t] | CoG x [m] | keel reactions [kN]");
    for (seq, join) in joins.iter().enumerate() {
        twin.advance_phase(&ActivityId(seq as u64 + 1))
            .expect("erection must stay sound at every step");
        let _ = join;
        if seq % 4 == 0 || seq == n - 1 {
            let block = hull
                .blocks
                .iter()
                .find(|b| b.id == joins[seq].block)
                .unwrap();
            let check = twin.structural_check();
            let (aft, fore) = (check.reactions_kn[0].1, check.reactions_kn[1].1);
            println!(
                "{:>5} | {:>5} | {:>4} | {:>13.0} | {:>9.2} | {:>6.0} / {:<6.0}",
                seq + 1,
                block.name,
                block.z_band + 1,
                twin.weight_model().installed_weight() / 1000.0,
                twin.assembly_state.current_mass_properties.cog.x,
                aft,
                fore
            );
        }
    }

    // 4. Final state.
    let dev = twin.weight_deviation();
    println!(
        "\nComplete: installed {:.0} t (best estimate {:.0} t, deviation {:+.0} kg), final CoG {}",
        dev.as_built_installed_kg / 1000.0,
        dev.best_estimate_kg / 1000.0,
        dev.deviation_kg,
        twin.assembly_state.current_mass_properties.cog
    );
    assert!(twin.vessel.is_in_final_phase());
    assert_eq!(twin.assembly_state.completed_activities.len(), n);
    assert!(dev.deviation_kg.abs() < 1e-6);
}
