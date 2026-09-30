//! Browser smoke tests (reviews 7D + 7G): the WASM bindings run under
//! headless Chrome via `wasm-bindgen-test` in the CI `wasm` job —
//!
//! ```sh
//! cargo install wasm-bindgen-cli@0.2.128
//! cargo test -p tpt-yard-wasm --target wasm32-unknown-unknown
//! ```
//!
//! (the workspace `.cargo/config.toml` wires `wasm-bindgen-test-runner` as
//! the wasm32 test runner). The file is compiled out on native targets, so
//! the regular `cargo test --workspace` is unaffected.
#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// The demo barge project the dashboard loads, as compact JSON — the same
/// shape `www/index.html` feeds to the `WasmDigitalTwin` constructor.
fn demo_project_json() -> String {
    use tpt_yard_core::{
        BuildPhase, ConstructionMethod, PhaseId, ProjectId, SeaVesselType, VesselProject,
        VesselType, WeightState,
    };

    let mut phase = BuildPhase::new(PhaseId(1), "Erect", 2.0);
    for (id, _x) in [(1u64, 0.0), (2, 10.0)] {
        let mut a = tpt_yard_core::AssemblyActivity::new(
            tpt_yard_assembly::ActivityId(id),
            format!("block {id}"),
            tpt_yard_core::ActivityType::JoinBlock,
            8.0,
        );
        a.dependencies = if id == 1 {
            vec![]
        } else {
            vec![tpt_yard_assembly::ActivityId(1)]
        };
        phase.activities.push(a);
    }
    phase.weight_state = WeightState {
        design_kg: 100.0,
        installed_kg: 0.0,
    };
    let project = VesselProject::new(
        ProjectId(1),
        "WASM demo barge",
        VesselType::Sea(SeaVesselType::FishingVessel),
        ConstructionMethod::SeaDrydock,
        vec![phase],
    )
    .unwrap();
    project.to_json().to_string_compact()
}

#[wasm_bindgen_test]
fn twin_loads_advances_and_reports_in_browser() {
    use tpt_yard_wasm::WasmDigitalTwin;

    let mut twin = WasmDigitalTwin::new(&demo_project_json()).expect("demo project loads");
    assert_eq!(twin.completed_count(), 0);

    // Dependency-gated advance: block 1 first, then block 2.
    assert!(twin.advance_next());
    assert_eq!(twin.completed_count(), 1);
    assert!(twin.advance_next());
    assert_eq!(twin.completed_count(), 2);
    // advance_next reports *soundness*, not "did advance": with the plan
    // exhausted it still returns true (nothing left, structure sound) and
    // the completed count stays put.
    assert!(twin.advance_next());
    assert_eq!(twin.completed_count(), 2);

    // The weight report is well-formed JSON; the constructor distributes
    // the 100 kg phase design weight evenly over the two blocks, so the
    // as-built installed weight after both advances is 100 kg.
    let report = twin.get_weight_report();
    let v = tpt_yard_core::json::Value::parse(&report).expect("weight report is JSON");
    let installed = v.get("installed_kg").and_then(|n| n.as_f64());
    assert_eq!(installed, Some(100.0), "report: {report}");

    // Geometry buffers are consistent: indices inside the vertex buffer.
    let verts = twin.get_geometry();
    let idx = twin.get_geometry_indices();
    if !idx.is_empty() {
        assert!(
            idx.iter().all(|&i| (i as usize) * 3 < verts.len()),
            "index buffer exceeds vertex buffer"
        );
    }

    // The structural check is JSON with a verdict field.
    let check = twin.structural_check();
    let v = tpt_yard_core::json::Value::parse(&check).expect("structural check is JSON");
    assert!(v.get("passed").is_some(), "check: {check}");
}

#[wasm_bindgen_test]
fn orbital_assembly_steps_in_browser() {
    use tpt_yard_wasm::WasmOrbitalAssembly;

    let mut assembly = WasmOrbitalAssembly::new(3).expect("constructor");
    let mut steps = 0;
    while assembly.simulate_next_step().expect("step simulates") {
        steps += 1;
        assert!(steps < 100, "sequence does not terminate");
    }
    assert_eq!(assembly.steps_done(), steps);
    assert!(steps > 0, "the default truss plans a sequence");

    // The robot pose is a usable transform (finite numbers).
    let pose = assembly.get_robot_pose();
    assert!(!pose.is_empty());
    assert!(pose.iter().all(|v| v.is_finite()));
}
