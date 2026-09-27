//! Golden verification for scheduling: critical path and resource
//! levelling.

use std::path::PathBuf;

use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{json::Value, ActivityType, AssemblyActivity, Resource, ResourceKind};
use tpt_yard_scheduling::{ScheduleObjective, ShipyardScheduler};

fn golden(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../../test-data/golden/planning/{name}"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Value::parse(&text).expect("parses")
}

fn network_from(v: &Value, with_cranes: bool) -> ShipyardScheduler {
    let mut acts = Vec::new();
    for a in v
        .get("network")
        .and_then(|n| n.get("activities"))
        .and_then(|a| a.as_array())
        .expect("activities")
    {
        let mut activity = AssemblyActivity::new(
            ActivityId(a.get("id").and_then(|n| n.as_u64()).unwrap()),
            a.get("name").and_then(|s| s.as_str()).unwrap(),
            ActivityType::CutSteel,
            a.get("duration_hours").and_then(|n| n.as_f64()).unwrap(),
        )
        .with_dependencies(
            &a.get("dependencies")
                .and_then(|d| d.as_array())
                .unwrap()
                .iter()
                .map(|d| ActivityId(d.as_u64().unwrap()))
                .collect::<Vec<_>>(),
        );
        if with_cranes {
            let crane = a.get("crane").and_then(|c| c.as_f64()).unwrap_or(0.0);
            activity.resources.push(Resource {
                name: "crane".into(),
                kind: ResourceKind::Crane,
                capacity: crane,
            });
        }
        acts.push(activity);
    }
    ShipyardScheduler::new(acts)
}

#[test]
fn golden_critical_path_schedule() {
    let v = golden("critical-path-schedule.json");
    let s = network_from(&v, false);
    let result = s
        .optimize_sequence(ScheduleObjective::MinimizeDuration)
        .unwrap();
    let exp = v.get("expected").expect("expected");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);

    assert!((result.makespan_hours - num(exp, "makespan_hours")).abs() < 1e-9);
    let cp: Vec<u64> = s.critical_path().unwrap().iter().map(|id| id.0).collect();
    let expected_cp: Vec<u64> = exp
        .get("critical_path")
        .and_then(|c| c.as_array())
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap())
        .collect();
    assert_eq!(cp, expected_cp);

    let cpm = s.cpm().unwrap();
    let c_id = ActivityId(3);
    assert!((cpm[&c_id].float_h - num(exp, "float_c_hours")).abs() < 1e-9);
    let e_id = ActivityId(5);
    assert!((cpm[&e_id].float_h - num(exp, "float_e_hours")).abs() < 1e-9);
}

#[test]
fn golden_resource_leveling() {
    let v = golden("resource-leveling.json");
    let s = network_from(&v, true);
    let early = s
        .optimize_sequence(ScheduleObjective::MinimizeDuration)
        .unwrap();
    let levelled = s.resource_leveling().unwrap();

    let exp = v.get("expected").expect("expected");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let peak = |r: &tpt_yard_scheduling::ScheduleResult| {
        r.peak_resource_use
            .iter()
            .find(|(k, _)| *k == ResourceKind::Crane)
            .map(|(_, v)| *v)
            .unwrap_or(0.0)
    };

    assert!((early.makespan_hours - num(exp, "earliest_makespan_hours")).abs() < 1e-9);
    assert!((peak(&early) - num(exp, "earliest_crane_peak")).abs() < 1e-9);
    assert!((peak(&levelled) - num(exp, "levelled_crane_peak")).abs() < 1e-9);
    assert!((levelled.makespan_hours - num(exp, "levelled_makespan_hours")).abs() < 1e-9);
}
