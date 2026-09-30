//! The README Quick Start, compiled and asserted so it cannot rot
//! (review 7F: the old quick start had a borrow error and a phantom file).

use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{
    ActivityType, AssemblyActivity, BuildPhase, ConstructionMethod, PhaseId, ProjectId,
    SeaVesselType, VesselProject, VesselType,
};
use tpt_yard_digital_twin::DigitalTwin;

#[test]
fn readme_quick_start_works() {
    let mut phase = BuildPhase::new(PhaseId(1), "Erection", 4.0);
    for i in 1..=3u64 {
        let deps: Vec<ActivityId> = if i == 1 {
            vec![]
        } else {
            vec![ActivityId(i - 1)]
        };
        phase.activities.push(
            AssemblyActivity::new(
                ActivityId(i),
                format!("Erect block {i}"),
                ActivityType::JoinBlock,
                6.0,
            )
            .with_dependencies(&deps),
        );
    }
    let project = VesselProject::new(
        ProjectId(1),
        "Demo barge",
        VesselType::Sea(SeaVesselType::ContainerShip { teu_capacity: 800 }),
        ConstructionMethod::SeaDrydock,
        vec![phase],
    )
    .unwrap();
    let mut twin = DigitalTwin::new(project);

    let ids: Vec<ActivityId> = twin
        .vessel
        .build_phases
        .iter()
        .flat_map(|p| p.activities.iter().map(|a| a.id))
        .collect();
    assert_eq!(ids.len(), 3);
    for id in &ids {
        twin.advance_phase(id)
            .expect("activity must be sound to run");
    }
    assert_eq!(twin.assembly_state.completed_activities.len(), 3);
    // The single-phase plan stays on phase 1; `DigitalTwin::new` carries no
    // weight items, so mass tracking starts from `with_weight_model`.
    assert_eq!(twin.weight_model().installed_weight(), 0.0);
}
