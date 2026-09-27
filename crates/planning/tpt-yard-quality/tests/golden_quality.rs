//! Golden verification for quality: NDT plan generation and defect
//! tracking.

use std::path::PathBuf;

use tpt_yard_core::{
    json::Value, ActivityId, ActivityType, AssemblyActivity, BuildPhase, FluidType, OutfitSystem,
    PhaseId, TestType,
};
use tpt_yard_quality::{DefectRecord, DefectType, Disposition, NdtMethod, QualityManagement};

fn golden(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../../test-data/golden/quality/{name}"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Value::parse(&text).expect("parses")
}

fn phases_from(v: &Value) -> Vec<BuildPhase> {
    let mut phases = Vec::new();
    for p in v
        .get("build_phases")
        .and_then(|b| b.as_array())
        .expect("phases")
    {
        let mut phase = BuildPhase::new(
            PhaseId(p.get("id").and_then(|n| n.as_u64()).unwrap()),
            p.get("name").and_then(|s| s.as_str()).unwrap(),
            1.0,
        );
        for a in p.get("activities").and_then(|a| a.as_array()).unwrap() {
            let name = a.get("name").and_then(|s| s.as_str()).unwrap();
            let kind = a.get("type").and_then(|s| s.as_str()).unwrap();
            let activity_type = match kind {
                "WeldBlock" => ActivityType::WeldBlock,
                "Paint" => ActivityType::Paint,
                "OutfitPiping" => ActivityType::Outfit {
                    system: OutfitSystem::Piping {
                        fluid: FluidType::SeaWater,
                        diameter_mm: 200.0,
                    },
                },
                "TestHydrostatic" => ActivityType::Test {
                    test_type: TestType::Hydrostatic,
                },
                other => panic!("unknown activity kind {other}"),
            };
            phase.activities.push(AssemblyActivity::new(
                ActivityId(a.get("id").and_then(|n| n.as_u64()).unwrap()),
                name,
                activity_type,
                a.get("duration_hours").and_then(|n| n.as_f64()).unwrap(),
            ));
        }
        phases.push(phase);
    }
    phases
}

fn method_code(m: NdtMethod) -> &'static str {
    m.code()
}

#[test]
fn golden_ndt_inspection_plan() {
    let v = golden("ndt-inspection-plan.json");
    let qm = QualityManagement::default();
    let plan = qm.generate_inspection_plan(&phases_from(&v));

    let exp = v.get("expected").expect("expected");
    assert_eq!(
        plan.len(),
        exp.get("n_points").and_then(|n| n.as_u64()).unwrap() as usize
    );
    for (point, expected) in plan
        .iter()
        .zip(exp.get("points").and_then(|p| p.as_array()).unwrap())
    {
        let expected_method = expected.get("method").and_then(|m| m.as_str()).unwrap();
        assert_eq!(
            method_code(point.method),
            expected_method,
            "point {}",
            point.id
        );
        assert_eq!(
            point.criticality,
            expected
                .get("criticality")
                .and_then(|c| c.as_u64())
                .unwrap() as u8
        );
        assert_eq!(
            point.coverage_pct,
            expected
                .get("coverage_pct")
                .and_then(|c| c.as_f64())
                .unwrap()
        );
        assert_eq!(
            point.activity.0,
            expected.get("activity").and_then(|a| a.as_u64()).unwrap()
        );
    }
}

#[test]
fn golden_weld_defect_tracking() {
    let v = golden("weld-defect-tracking.json");
    let qm = QualityManagement::default();
    let records: Vec<DefectRecord> = v
        .get("records")
        .and_then(|r| r.as_array())
        .unwrap()
        .iter()
        .map(|r| DefectRecord {
            id: r.get("id").and_then(|n| n.as_u64()).unwrap(),
            inspection_point: r.get("inspection_point").and_then(|n| n.as_u64()).unwrap(),
            defect_type: match r.get("defect_type").and_then(|s| s.as_str()).unwrap() {
                "Porosity" => DefectType::Porosity,
                "Cracking" => DefectType::Cracking,
                "Undercut" => DefectType::Undercut,
                other => panic!("unknown defect {other}"),
            },
            size_mm: r.get("size_mm").and_then(|n| n.as_f64()).unwrap(),
            disposition: match r.get("disposition").and_then(|s| s.as_str()).unwrap() {
                "Repair" => Disposition::Repair,
                "AcceptAsIs" => Disposition::AcceptAsIs,
                "Reject" => Disposition::Reject,
                "RepairAndReinspect" => Disposition::RepairAndReinspect,
                other => panic!("unknown disposition {other}"),
            },
        })
        .collect();
    let report = qm.defect_tracking(&records);

    let exp = v.get("expected").expect("expected");
    assert_eq!(
        report.total,
        exp.get("total").and_then(|n| n.as_u64()).unwrap() as usize
    );
    let dominant = exp.get("dominant_defect").and_then(|d| d.as_str()).unwrap();
    assert_eq!(
        report.dominant_defect.map(|d| format!("{d:?}")),
        Some(dominant.to_string())
    );
    assert_eq!(
        report.by_type.len(),
        exp.get("by_type").and_then(|b| b.as_array()).unwrap().len()
    );
    for (got, expected) in report
        .by_type
        .iter()
        .zip(exp.get("by_type").and_then(|b| b.as_array()).unwrap())
    {
        let pair = expected.as_array().unwrap();
        let expected_type = pair[0].as_str().unwrap();
        let expected_count = pair[1].as_u64().unwrap() as usize;
        assert_eq!(format!("{:?}", got.0), expected_type);
        assert_eq!(got.1, expected_count);
    }
    assert!(
        (report.repair_rate - exp.get("repair_rate").and_then(|r| r.as_f64()).unwrap()).abs()
            < 1e-9
    );
    assert_eq!(
        report.rejected,
        exp.get("rejected").and_then(|n| n.as_u64()).unwrap() as usize
    );
}
