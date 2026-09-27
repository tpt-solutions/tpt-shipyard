//! Golden-data verification for block division (RFC 0002 reference case).

use std::path::PathBuf;

use tpt_yard_core::{json::Value, Dimensions};
use tpt_yard_hull::{HullConstruction, HullGeometry, SeamType};

#[test]
fn golden_container_ship_block_division() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test-data/golden/sea/container-ship-block-division.json");
    let text = std::fs::read_to_string(&path).expect("golden file readable");
    let v = Value::parse(&text).expect("golden JSON parses");

    let hull_v = v.get("hull").expect("hull");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let hull_form = HullGeometry {
        loa_m: num(hull_v, "loa_m"),
        boa_m: num(hull_v, "boa_m"),
        depth_m: num(hull_v, "depth_m"),
        areal_density_kg_m2: num(hull_v, "areal_density_kg_m2"),
        depth_bands: num(hull_v, "depth_bands") as u32,
    };
    let cons = v.get("constraints").expect("constraints");
    let crane = num(cons, "crane_capacity_kn");
    let ws = cons.get("workshop").expect("workshop");
    let workshop = Dimensions::new(
        num(ws, "length_m"),
        num(ws, "breadth_m"),
        num(ws, "depth_m"),
    );

    let mut hull = HullConstruction::new(hull_form);
    hull.blocks = hull.block_division(crane, workshop);
    let joins = hull.erection_sequence(&hull.blocks);

    let exp = v.get("expected").expect("expected");
    let tol = v
        .get("tolerance")
        .and_then(|t| t.get("relative"))
        .and_then(|r| r.as_f64())
        .unwrap_or(0.001);
    let check = |name: &str, computed: f64, expected: f64| {
        assert!(
            ((computed - expected) / expected.max(1e-9)).abs() <= tol,
            "{name}: computed {computed} vs golden {expected}"
        );
    };

    assert_eq!(
        hull.blocks.len(),
        exp.get("n_blocks").and_then(|n| n.as_u64()).unwrap() as usize
    );
    check(
        "block_length_m",
        hull.blocks[0].dimensions().length,
        num(exp, "block_length_m"),
    );
    check(
        "block_weight_kg",
        hull.blocks[0].weight_kg,
        num(exp, "block_weight_kg"),
    );
    check(
        "max_block_load_kn",
        hull.blocks
            .iter()
            .map(|b| b.weight_kn())
            .fold(0.0, f64::max),
        num(exp, "max_block_load_kn"),
    );
    check(
        "total_steel_kg",
        hull.total_steel_kg(),
        num(exp, "total_steel_kg"),
    );
    check(
        "min_block_cog_z_m",
        hull.blocks.iter().map(|b| b.cog.z).fold(f64::MAX, f64::min),
        num(exp, "min_block_cog_z_m"),
    );
    assert_eq!(
        joins.iter().filter(|j| j.z_band == 0).count(),
        exp.get("keel_tier_blocks")
            .and_then(|n| n.as_u64())
            .unwrap() as usize
    );
    let first_seam = match exp.get("first_join_seam").and_then(|s| s.as_str()) {
        Some("DockJoint") => SeamType::DockJoint,
        Some("ButtSeam") => SeamType::ButtSeam,
        _ => panic!("unknown golden seam"),
    };
    assert_eq!(joins[0].seam, first_seam);

    // Constraint re-check (the golden case must respect crane + workshop).
    for b in &hull.blocks {
        assert!(b.weight_kn() <= crane * 0.9 + 1e-6);
        assert!(b.dimensions().length <= workshop.length + 1e-9);
    }
}
