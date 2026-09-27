//! Milestone verification for Phase 1: track weight and CoG through 10 build
//! phases end-to-end, against closed-form values.

use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{
    ActivityType, BuildPhase, ConstructionMethod, ItemId, PhaseId, ProjectId, SeaVesselType,
    Vector3, VesselProject, VesselType,
};
use tpt_yard_digital_twin::{CoGReport, DigitalTwin, SupportCondition, TwinError};
use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

/// A 140 m barge built from 10 double-bottom blocks erected fore-to-aft on
/// keel blocks: one phase per block, one activity per phase.
fn barge_project() -> (VesselProject, WeightModel) {
    const BLOCKS: usize = 10;
    const BLOCK_KG: f64 = 180_000.0; // 180 t per double-bottom block
    const BLOCK_PITCH: f64 = 13.0; // m between block CoGs

    let mut phases = Vec::new();
    let mut weight = WeightModel::new(BLOCKS as f64 * BLOCK_KG, Vector3::new(58.5, 0.0, 5.0));
    for i in 0..BLOCKS {
        let activity_id = ActivityId(i as u64 + 1);
        let mut phase = BuildPhase::new(
            PhaseId(i as u64 + 1),
            format!("Erection phase {} (block {})", i + 1, 200 + i),
            4.0,
        );
        let deps: Vec<ActivityId> = if i == 0 {
            vec![]
        } else {
            vec![ActivityId(i as u64)]
        };
        phase.activities.push(
            tpt_yard_core::AssemblyActivity::new(
                activity_id,
                format!("Erect block {}", 200 + i),
                ActivityType::JoinBlock,
                6.0,
            )
            .with_dependencies(&deps),
        );
        phase.weight_state = tpt_yard_core::WeightState {
            design_kg: BLOCK_KG,
            installed_kg: 0.0,
        };
        weight.add_item(WeightItem {
            id: ItemId(i as u64 + 1),
            name: format!("Block {}", 200 + i),
            group: "hull".into(),
            weight_kg: BLOCK_KG,
            cog: Vector3::new(i as f64 * BLOCK_PITCH, 0.0, 5.0),
            status: ItemStatus::Design,
            margin_pct: 2.0,
            installed_by: Some(activity_id),
        });
        phases.push(phase);
    }

    let project = VesselProject::new(
        ProjectId(1),
        "10-phase verification barge",
        VesselType::Sea(SeaVesselType::FishingVessel),
        ConstructionMethod::SeaDrydock,
        phases,
    )
    .unwrap();
    (project, weight)
}

#[test]
fn milestone_weight_and_cog_through_10_phases() {
    let (project, weight) = barge_project();
    let mut twin = DigitalTwin::with_weight_model(
        project,
        weight,
        // Keel blocks fore and aft of the 117 m block row.
        SupportCondition::KeelBlocks {
            positions: vec![
                Vector3::new(-10.0, -4.0, 0.0),
                Vector3::new(-10.0, 4.0, 0.0),
                Vector3::new(130.0, -4.0, 0.0),
                Vector3::new(130.0, 4.0, 0.0),
            ],
        },
    );

    // Closed-form expectations after n phases.
    let expected_installed = |n: f64| n * 180_000.0;
    let expected_cog_x = |n: i32| (0..n).map(|i| i as f64 * 13.0).sum::<f64>() / n as f64;

    let mut prev_x = f64::MIN;
    for phase_idx in 0..10u64 {
        twin.advance_phase(&ActivityId(phase_idx + 1))
            .expect("each phase must advance on sound supports");

        // As-built weight grows by exactly one block per phase.
        let dev = twin.weight_deviation();
        assert_eq!(
            dev.as_built_installed_kg,
            expected_installed((phase_idx + 1) as f64),
            "installed weight after phase {}",
            phase_idx + 1
        );

        // CoG marches monotonically fore and matches the weighted mean.
        let CoGReport { samples } = twin.centre_of_gravity_tracking().unwrap();
        let last = samples.last().expect("a sample exists after each phase");
        assert!(last.cog.x > prev_x, "CoG must advance monotonically");
        prev_x = last.cog.x;
        assert!(
            (last.cog.x - expected_cog_x((phase_idx + 1) as i32)).abs() < 1e-9,
            "CoG after phase {} must match closed form",
            phase_idx + 1
        );
        assert_eq!(
            last.installed_kg,
            expected_installed((phase_idx + 1) as f64)
        );

        // The partial structure stands on the keel blocks at every stage.
        let check = twin.structural_check();
        assert!(check.passed, "structure must stand at every phase");
        assert!(check.reactions_kn.iter().all(|(_, r)| *r > 0.0));

        // Best-estimate total never moves (all mass is accounted from day 1).
        assert_eq!(dev.best_estimate_kg, 1_800_000.0);
        assert_eq!(dev.design_kg, 1_800_000.0);
    }

    // After 10 phases: as-built equals design, phase pointer at the end.
    let dev = twin.weight_deviation();
    assert_eq!(dev.as_built_installed_kg, 1_800_000.0);
    assert_eq!(dev.deviation_kg, 0.0);
    assert!(twin.vessel.is_in_final_phase());
    assert_eq!(
        twin.assembly_state.completed_activities.len(),
        10,
        "all activities completed"
    );

    // Final CoG: middle of the block row.
    let final_cog = twin.weight_model().installed_centre_of_gravity().unwrap();
    assert!((final_cog.x - 58.5).abs() < 1e-9);

    // Out-of-order completion is impossible: everything is done.
    assert!(matches!(
        twin.advance_phase(&ActivityId(1)),
        Err(TwinError::AlreadyCompleted(_))
    ));
}
