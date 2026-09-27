//! Golden-data verification for the welding distortion model (RFC 0004).
//!
//! Loads `test-data/golden/sea/welding-distortion-panel.json` and requires
//! the computed model output to match the locked golden values within the
//! tolerances recorded in the file.

use std::path::PathBuf;

use tpt_yard_core::{json::Value, Material};
use tpt_yard_joints::{GrooveType, JointGeometry, JointKind};
use tpt_yard_welding::{WeldProcedure, WeldProcess, WeldingSimulation};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../")
}

fn golden_case() -> (WeldingSimulation, Value) {
    let path = repo_root().join("test-data/golden/sea/welding-distortion-panel.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let v = Value::parse(&text).expect("golden file must parse");
    let proc_v = v.get("procedure").expect("procedure");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let procedure = WeldProcedure {
        process: match proc_v.get("process").and_then(|p| p.as_str()) {
            Some("SAW") => WeldProcess::Saw,
            Some("FCAW") => WeldProcess::Fcaw,
            Some("GMAW") => WeldProcess::Gmaw,
            Some("GTAW") => WeldProcess::Gtaw,
            Some("EBW") => WeldProcess::Ebw,
            Some("LaserWeld") => WeldProcess::LaserWeld,
            Some("Smaw") | Some("SMAW") => WeldProcess::Smaw,
            other => panic!("golden process {other:?}"),
        },
        heat_input_kj_mm: num(proc_v, "heat_input_kj_mm"),
        travel_speed_mm_s: num(proc_v, "travel_speed_mm_s"),
        preheat_temp_c: num(proc_v, "preheat_temp_c"),
        interpass_temp_c: num(proc_v, "interpass_temp_c"),
        filler_metal: proc_v
            .get("filler_metal")
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_string(),
        sequence: vec![],
    };
    let joint_v = v.get("joint").expect("joint");
    let joint = JointGeometry::new(JointKind::Butt)
        .with_thickness_mm(num(joint_v, "thickness_mm"))
        .with_groove(match joint_v.get("groove").and_then(|g| g.as_str()) {
            Some("V") => GrooveType::V,
            Some("DoubleV") => GrooveType::DoubleV,
            _ => GrooveType::Square,
        })
        .with_groove_angle_deg(num(joint_v, "groove_angle_deg"))
        .with_root_gap_mm(num(joint_v, "root_gap_mm"))
        .with_root_face_mm(num(joint_v, "root_face_mm"))
        .with_length_mm(1000.0);
    (
        WeldingSimulation::new(procedure, Material::ah36(), joint),
        v,
    )
}

#[test]
fn golden_welding_distortion_panel() {
    let (sim, golden) = golden_case();
    let tol_rel = golden
        .get("tolerance")
        .and_then(|t| t.get("relative"))
        .and_then(|r| r.as_f64())
        .unwrap_or(0.05);

    let expected = golden.get("expected").expect("expected block");
    let d = sim.distortion().unwrap();
    let rs = sim.residual_stress().unwrap();
    let cycle = sim.thermal_cycle(8.0).unwrap();

    let check = |name: &str, computed: f64, golden_v: f64, tol: f64| {
        let rel = ((computed - golden_v).abs()) / golden_v.abs().max(1e-9);
        assert!(
            rel <= tol,
            "{name}: computed {computed} vs golden {golden_v} (rel {rel:.4} > {tol})"
        );
    };

    let exp_d = expected.get("residual_stress").map(|_| ());
    let _ = exp_d;
    let transverse = expected
        .get("transverse_shrinkage_mm")
        .and_then(|v| v.as_f64())
        .unwrap();
    let longitudinal = expected
        .get("longitudinal_shrinkage_mm")
        .and_then(|v| v.as_f64())
        .unwrap();
    let angular = expected
        .get("angular_distortion_deg")
        .and_then(|v| v.as_f64())
        .unwrap();
    let bowing = expected.get("bowing_mm").and_then(|v| v.as_f64()).unwrap();
    check(
        "transverse_shrinkage_mm",
        d.transverse_shrinkage_mm,
        transverse,
        tol_rel,
    );
    check(
        "longitudinal_shrinkage_mm",
        d.longitudinal_shrinkage_mm,
        longitudinal,
        tol_rel,
    );
    check(
        "angular_distortion_deg",
        d.angular_distortion_deg,
        angular,
        tol_rel,
    );
    check("bowing_mm", d.bowing_mm, bowing, tol_rel);

    let rs_g = expected.get("residual_stress").expect("residual_stress");
    check(
        "tension_half_width_mm",
        rs.tension_half_width_mm,
        rs_g.get("tension_half_width_mm")
            .and_then(|v| v.as_f64())
            .unwrap(),
        tol_rel,
    );
    check(
        "compressive_stress_mpa",
        rs.compressive_stress_mpa,
        rs_g.get("compressive_stress_mpa")
            .and_then(|v| v.as_f64())
            .unwrap(),
        tol_rel,
    );

    let cy_g = expected
        .get("thermal_cycle_at_8mm")
        .expect("thermal_cycle_at_8mm");
    check(
        "peak_temp_c",
        cycle.peak_temp_c,
        cy_g.get("peak_temp_c").and_then(|v| v.as_f64()).unwrap(),
        tol_rel,
    );
    check(
        "t8_5_s",
        cycle.t8_5_s.expect("t8/5 defined"),
        cy_g.get("t8_5_s").and_then(|v| v.as_f64()).unwrap(),
        0.10,
    );
}

#[test]
fn sample_wps_records_load() {
    let dir = repo_root().join("test-data/welding-procedures");
    let mut loaded = 0;
    let entries = std::fs::read_dir(&dir).expect("wps directory");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable WPS");
        let v = Value::parse(&text).expect("valid WPS JSON");
        let proc =
            WeldProcedure::from_json(&v).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(proc.heat_input_kj_mm > 0.0);
        assert!(proc.travel_speed_mm_s > 0.0);
        loaded += 1;
    }
    assert!(
        loaded >= 3,
        "expected at least 3 WPS records, loaded {loaded}"
    );
}
