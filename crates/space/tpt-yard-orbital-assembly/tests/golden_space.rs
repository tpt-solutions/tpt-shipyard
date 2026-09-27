//! Golden verification for the orbital assembly sequence and the
//! ISS-class truss integrity screening (RFC 0003).

use std::path::PathBuf;

use tpt_yard_core::{json::Value, ComponentId, StepId, Vector3};
use tpt_yard_orbital_assembly::{
    AssemblyError, AssemblyState, ComponentSpec, OrbitalAssembly, OrbitalParameters, SpaceStructure,
};

fn golden(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../../test-data/golden/space/{name}"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Value::parse(&text).expect("parses")
}

fn truss_from_golden(v: &Value) -> OrbitalAssembly {
    let structure = v.get("structure").expect("structure");
    let mut a = OrbitalAssembly::new(
        SpaceStructure::Truss {
            segments: structure.get("segments").and_then(|n| n.as_u64()).unwrap() as u32,
            length_m: structure.get("length_m").and_then(|n| n.as_f64()).unwrap(),
        },
        OrbitalParameters::default(),
    );
    for bay in v.get("bays").and_then(|b| b.as_array()).expect("bays") {
        let dims = bay.get("dimensions_m").and_then(|d| d.as_array()).unwrap();
        let target = bay.get("target_m").and_then(|d| d.as_array()).unwrap();
        a.add_component(ComponentSpec {
            id: ComponentId(bay.get("id").and_then(|n| n.as_u64()).unwrap()),
            name: bay.get("name").and_then(|s| s.as_str()).unwrap().into(),
            mass_kg: bay.get("mass_kg").and_then(|n| n.as_f64()).unwrap(),
            dimensions: Vector3::new(
                dims[0].as_f64().unwrap(),
                dims[1].as_f64().unwrap(),
                dims[2].as_f64().unwrap(),
            ),
            target_position: Vector3::new(
                target[0].as_f64().unwrap(),
                target[1].as_f64().unwrap(),
                target[2].as_f64().unwrap(),
            ),
        });
    }
    a
}

#[test]
fn golden_orbital_assembly_sequence() {
    let v = golden("orbital-assembly-sequence.json");
    let mut a = truss_from_golden(&v);
    let steps = a.plan_sequence();

    let exp = v.get("expected").expect("expected");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);

    assert_eq!(
        steps.len(),
        exp.get("n_steps").and_then(|n| n.as_u64()).unwrap() as usize
    );
    let total: f64 = steps.iter().map(|s| s.duration_hours).sum();
    assert!(
        (total - num(exp, "total_duration_hours")).abs() < 1e-9,
        "total {total}"
    );

    // Handling force on the first translate.
    let state = AssemblyState::default();
    let translate = steps
        .iter()
        .find(|s| {
            matches!(
                s.action,
                tpt_yard_orbital_assembly::AssemblyAction::Translate { .. }
            )
        })
        .unwrap();
    let sim = a.simulate_step(translate, &state).unwrap();
    assert!((sim.interface_force_n - num(exp, "handling_force_n")).abs() < 1e-9);

    // Structural integrity at the last step.
    let last = StepId(steps.len() as u64);
    let check = a.verify_structural_integrity(&last).unwrap();
    let tol = v
        .get("tolerance")
        .and_then(|t| t.get("relative"))
        .and_then(|x| x.as_f64())
        .unwrap_or(1e-9);
    assert!(
        (check.root_stress_mpa - num(exp, "final_root_stress_mpa")).abs()
            <= num(exp, "final_root_stress_mpa") * tol
    );
    assert!(
        (check.utilization - num(exp, "final_utilization")).abs()
            <= num(exp, "final_utilization").abs() * tol + 1e-15
    );
    assert_eq!(
        check.passed,
        exp.get("structural_passed")
            .and_then(|b| b.as_bool())
            .unwrap()
    );
}

#[test]
fn golden_iss_truss_integrity_screening() {
    let v = golden("iss-truss-assembly.json");
    let p = v.get("parameters").expect("parameters");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);

    // Build a 15-bay, 5 m pitch truss through the planner.
    let n_bays = p.get("n_bays").and_then(|n| n.as_u64()).unwrap();
    let pitch = num(p, "bay_pitch_m");
    let mut a = OrbitalAssembly::new(
        SpaceStructure::Truss {
            segments: n_bays as u32,
            length_m: n_bays as f64 * pitch,
        },
        OrbitalParameters::default(),
    );
    for i in 1..=n_bays {
        a.add_component(ComponentSpec {
            id: ComponentId(i),
            name: format!("bay {i}"),
            mass_kg: 500.0,
            dimensions: Vector3::new(pitch, 3.0, 3.0),
            target_position: Vector3::new(pitch * i as f64 - pitch / 2.0, 0.0, 0.0),
        });
    }
    let steps = a.plan_sequence();
    a.docking_impulse_n = num(p, "docking_impulse_n");
    a.allowable_stress_mpa = num(p, "allowable_stress_mpa");

    let check = a
        .verify_structural_integrity(&StepId(steps.len() as u64))
        .unwrap();
    let exp = v.get("expected").expect("expected");
    let tol = v
        .get("tolerance")
        .and_then(|t| t.get("relative"))
        .and_then(|x| x.as_f64())
        .unwrap_or(1e-9);
    let expected_stress = num(exp, "root_stress_at_72_5_m_mpa");
    assert!((check.root_stress_mpa - expected_stress).abs() <= expected_stress * tol);
    let expected_util = num(exp, "utilization_at_72_5_m");
    assert!((check.utilization - expected_util).abs() <= expected_util * tol + 1e-17);
    assert_eq!(
        check.passed,
        exp.get("structural_passed")
            .and_then(|b| b.as_bool())
            .unwrap()
    );
}

#[test]
fn unknown_steps_stay_rejected_in_golden_flow() {
    let v = golden("orbital-assembly-sequence.json");
    let mut a = truss_from_golden(&v);
    a.plan_sequence();
    assert_eq!(
        a.verify_structural_integrity(&StepId(9999)),
        Err(AssemblyError::UnknownStep(StepId(9999)))
    );
}
