//! Example: rotating habitat construction — spin rate, Coriolis comfort,
//! thermal-fatigue life, and structural sizing for the Stanford torus and
//! an O'Neill cylinder reference.

use tpt_yard_core::Material;
use tpt_yard_habitat::{HabitatDesigner, HabitatType};
use tpt_yard_space_structural::SpaceStructuralDesigner;

fn main() {
    // The habitat reference parameters (review 7F: examples read their test
    // data and accept a path argument). Defaults to the repo's Stanford
    // torus reference (RFC 0005).
    let manifest_path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-data/orbital-structures/rotating-habitat-torus.json"
        )
        .to_string()
    });
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading {manifest_path}: {e}"));
    let manifest = tpt_yard_core::json::Value::parse(&text).expect("manifest parses");
    let num = |o: &tpt_yard_core::json::Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let radius_m = num(&manifest, "radius_m");
    let tube_diameter_m = num(&manifest, "tube_diameter_m");
    let target_g = num(&manifest, "target_gravity_g");
    let rotating_mass_kg = num(&manifest, "total_rotating_mass_kg");
    let safety_factor = num(&manifest, "safety_factor");
    let material = match manifest.get("material").and_then(|s| s.as_str()) {
        Some("AA5083") => Material::aa5083(),
        Some("AH36") => Material::ah36(),
        other => panic!("unsupported habitat material {other:?} (AA5083, AH36)"),
    };

    // ---- Stanford torus from the manifest -------------------------------
    let torus = HabitatDesigner::new(
        HabitatType::StanfordTorus {
            radius_m,
            tube_diameter_m,
        },
        material.clone(),
    );

    println!(
        "== Stanford torus (r = {radius_m:.0} m, tube {tube_diameter_m:.0} m, {}, from {manifest_path}) ==",
        material.name
    );
    for g in [target_g, 0.38] {
        let rpm = torus.required_rotation_rpm(radius_m, g);
        let report = torus.coriolis_effects(radius_m, rpm);
        println!(
            "  {g:.2} g -> {:>5.2} rpm (rim {:>6.1} m/s) — Coriolis {:>4.1}% g: {}",
            rpm,
            report.rim_velocity_ms,
            report.coriolis_acceleration_pct_g,
            if report.within_comfort {
                "COMFORTABLE"
            } else {
                "ABOVE 2 rpm COMFORT LIMIT"
            }
        );
    }

    // Structural sizing at the target gravity for the manifest's rotating mass.
    let design = torus.structural_design(rotating_mass_kg, safety_factor);
    println!(
        "  structure: shell {:.1} mm, hoop {:.1} MPa (util {:.2}%), mass {:.0} t",
        design.shell_thickness_mm,
        design.hoop_stress_mpa,
        design.utilization * 100.0,
        design.structure_mass_kg / 1000.0
    );

    // ---- Hoop stress verification against sigma = rho*omega^2*r^2 ------
    let structural = SpaceStructuralDesigner::new(material.clone());
    let rpm_1g = torus.required_rotation_rpm(radius_m, target_g);
    let stress = structural.rotating_habitat_stress(radius_m, rpm_1g, rotating_mass_kg);
    println!(
        "  ring self-stress at {rpm_1g:.2} rpm: {:.2} MPa (util {:.2}%)",
        stress.hoop_stress_mpa,
        stress.utilization * 100.0
    );

    // ---- Thermal cycling over 15 years in LEO --------------------------
    let fatigue = structural.thermal_cycling_fatigue(15 * 5_660); // ~5,660 orbits/yr
    println!(
        "  15-yr thermal cycling (210 °C swing): strain range {:.2e}, life fraction {:.2}",
        fatigue.strain_range, fatigue.life_fraction
    );

    // ---- O'Neill cylinder: a second reference design, not in the
    //      manifest (4 km radius, 32 km long, steel) -----------------------
    let oneill = HabitatDesigner::new(
        HabitatType::ONeillCylinder {
            radius_m: 4000.0,
            length_m: 32_000.0,
        },
        Material::ah36(),
    );
    let rpm = oneill.required_rotation_rpm(4000.0, 1.0);
    println!("\n== O'Neill cylinder (r = 4 km, L = 32 km, AH36) ==");
    let report = oneill.coriolis_effects(4000.0, rpm);
    println!(
        "  1.00 g -> {rpm:.3} rpm (rim {:>7.0} m/s) — Coriolis {:>4.2}% g: {}",
        report.rim_velocity_ms,
        report.coriolis_acceleration_pct_g,
        if report.within_comfort {
            "COMFORTABLE"
        } else {
            "ABOVE COMFORT LIMIT"
        }
    );
    let design = oneill.structural_design(500_000_000.0, 2.0);
    println!(
        "  structure: shell {:.0} mm, hoop {:.1} MPa (util {:.2}%), mass {:.0} kt",
        design.shell_thickness_mm,
        design.hoop_stress_mpa,
        design.utilization * 100.0,
        design.structure_mass_kg / 1_000_000.0
    );

    // Micrometeoroid shielding for the habitat shell.
    let shield = structural.micrometeoroid_shielding(10.0);
    println!(
        "  Whipple shield for 10 mm particles: bumper {:.1} mm + standoff {:.1} m + wall {:.1} mm ({:.0} kg/m2)",
        shield.bumper_thickness_mm,
        shield.standoff_m,
        shield.rear_wall_thickness_mm,
        shield.areal_density_kg_m2
    );

    assert!(torus.required_rotation(radius_m, target_g) > 0.0);
    let rpm_at_limit = torus.required_rotation_rpm(radius_m, 1.0);
    assert!(
        !torus
            .coriolis_effects(radius_m, rpm_at_limit)
            .within_comfort
    );
    assert!(oneill.coriolis_effects(4000.0, rpm).within_comfort);
}
