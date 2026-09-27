//! Golden verification for the rotating habitat stress (spec reference
//! case, RFC 0005).

use std::path::PathBuf;

use tpt_yard_core::{json::Value, Material};
use tpt_yard_space_structural::SpaceStructuralDesigner;

#[test]
fn golden_rotating_habitat_stress() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test-data/golden/space/rotating-habitat-stress.json");
    let text = std::fs::read_to_string(&path).expect("golden readable");
    let v = Value::parse(&text).expect("parses");

    let params = v.get("parameters").expect("parameters");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);

    let material = Material {
        name: "golden AA5083".into(),
        density_kg_m3: num(params, "density_kg_m3"),
        ..Material::aa5083()
    };
    let designer = SpaceStructuralDesigner::new(material);
    let result = designer.rotating_habitat_stress(
        num(params, "radius_m"),
        num(params, "rotation_rpm"),
        num(params, "habitat_mass_kg"),
    );

    let exp = v.get("expected").expect("expected");
    let tol = v
        .get("tolerance")
        .and_then(|t| t.get("relative"))
        .and_then(|x| x.as_f64())
        .unwrap_or(1e-9);
    let check = |name: &str, computed: f64, expected: f64| {
        assert!(
            ((computed - expected) / expected.abs().max(1e-12)).abs() <= tol,
            "{name}: computed {computed} vs golden {expected}"
        );
    };
    check(
        "hoop_stress_mpa",
        result.hoop_stress_mpa,
        num(exp, "hoop_stress_mpa"),
    );
    check(
        "angular_velocity_rad_s",
        result.angular_velocity_rad_s,
        num(exp, "angular_velocity_rad_s"),
    );
    check("utilization", result.utilization, num(exp, "utilization"));
    check(
        "radial_growth_m",
        result.radial_growth_m,
        num(exp, "radial_growth_m"),
    );
}
