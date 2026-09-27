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
    let crane_kn = 40_000.0;
    let workshop = Dimensions::new(24.0, 30.0, 14.0);

    // 1. Divide the hull into erection blocks.
    let mut hull = HullConstruction::new(HullGeometry {
        loa_m: 140.0,
        boa_m: 22.0,
        depth_m: 12.0,
        areal_density_kg_m2: 180.0,
        depth_bands: 2,
    });
    hull.blocks = hull.block_division(crane_kn, workshop);
    let joins = hull.erection_sequence(&hull.blocks);

    println!(
        "Container ship 140 m: {} blocks ({:.0} t steel), crane {:.0} kN, workshop {:.0}x{:.0}x{:.0} m",
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
        tpt_yard_core::Vector3::new(70.0, 0.0, 6.0),
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
        weight.add_item(WeightItem {
            id: ItemId(seq as u64 + 1),
            name: block.name.clone(),
            group: "hull".into(),
            weight_kg: block.weight_kg,
            cog: block.cog,
            status: ItemStatus::Design,
            margin_pct: 2.0,
            installed_by: Some(activity_id),
        });
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
            positions: vec![
                tpt_yard_core::Vector3::new(-5.0, -4.0, 0.0),
                tpt_yard_core::Vector3::new(-5.0, 4.0, 0.0),
                tpt_yard_core::Vector3::new(145.0, -4.0, 0.0),
                tpt_yard_core::Vector3::new(145.0, 4.0, 0.0),
            ],
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
