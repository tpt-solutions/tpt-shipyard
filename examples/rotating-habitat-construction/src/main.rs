//! Example: rotating habitat construction — spin rate, Coriolis comfort,
//! thermal-fatigue life, and structural sizing for the Stanford torus and
//! an O'Neill cylinder reference.

use tpt_yard_core::Material;
use tpt_yard_habitat::{HabitatDesigner, HabitatType};
use tpt_yard_space_structural::SpaceStructuralDesigner;

fn main() {
    // ---- Stanford torus: 100 m radius, 20 m tube -----------------------
    let torus = HabitatDesigner::new(
        HabitatType::StanfordTorus {
            radius_m: 100.0,
            tube_diameter_m: 20.0,
        },
        Material::aa5083(),
    );

    println!("== Stanford torus (r = 100 m, tube 20 m, AA5083) ==");
    for g in [1.0, 0.38] {
        let rpm = torus.required_rotation_rpm(100.0, g);
        let report = torus.coriolis_effects(100.0, rpm);
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

    // Structural sizing at 1 g for a 10,000 t rotating mass.
    let design = torus.structural_design(10_000_000.0, 2.0);
    println!(
        "  structure: shell {:.1} mm, hoop {:.1} MPa (util {:.2}%), mass {:.0} t",
        design.shell_thickness_mm,
        design.hoop_stress_mpa,
        design.utilization * 100.0,
        design.structure_mass_kg / 1000.0
    );

    // ---- Hoop stress verification against sigma = rho*omega^2*r^2 ------
    let structural = SpaceStructuralDesigner::new(Material::aa5083());
    let stress = structural.rotating_habitat_stress(100.0, 2.99, 10_000_000.0);
    println!(
        "  ring self-stress at 2.99 rpm: {:.2} MPa (util {:.2}%)",
        stress.hoop_stress_mpa,
        stress.utilization * 100.0
    );

    // ---- Thermal cycling over 15 years in LEO --------------------------
    let fatigue = structural.thermal_cycling_fatigue(15 * 5_660); // ~5,660 orbits/yr
    println!(
        "  15-yr thermal cycling (210 °C swing): strain range {:.2e}, life fraction {:.2}",
        fatigue.strain_range, fatigue.life_fraction
    );

    // ---- O'Neill cylinder: 4 km radius ---------------------------------
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

    assert!(torus.required_rotation(100.0, 1.0) > 0.0);
    assert!(!torus.coriolis_effects(100.0, 2.99).within_comfort);
    assert!(oneill.coriolis_effects(4000.0, rpm).within_comfort);
}
