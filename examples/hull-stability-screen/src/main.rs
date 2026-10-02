//! End-to-end stability and scantling screen: from a hull manifest to
//! hydrostatics, the IMO 2008 intact check, the hull-girder section
//! modulus, the SOLAS probabilistic damage index and the local plate +
//! stiffener scantlings. The hull is the prismatic screening model, and
//! the loading assumptions (block coefficient, draft, KG, still-water
//! moment) are stated below — this is a design-early screen, not a
//! class submission.
//!
//! Assumes the manifest layout of `test-data/hull-blocks/` (pass a
//! manifest path to screen a different hull).

use tpt_yard_core::json::Value;
use tpt_yard_hydrostatics::{
    attained_subdivision_index, combined_cargo_index, high_strength_factor, local_plate_scantling,
    p_factor, plate_buckling_check_ec3, required_index_cargo, s_factor_cargo, stiffener_scantling,
    DamageCase, DamageCompartment, DamageLengthDensity, HullForm, LoadingCondition,
    LocalPlateScantlingInput,
};

fn main() {
    let manifest_path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-data/hull-blocks/container-ship-140m.json"
        )
        .to_string()
    });
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading {manifest_path}: {e}"));
    let manifest = Value::parse(&text).expect("manifest parses");
    let hull_v = manifest.get("hull").expect("manifest: hull");
    let num = |o: &Value, k: &str| o.get(k).and_then(|n| n.as_f64()).expect(k);
    let loa = num(hull_v, "loa_m");
    let boa = num(hull_v, "boa_m");
    let depth = num(hull_v, "depth_m");
    let vessel = manifest
        .get("vessel")
        .and_then(|s| s.as_str())
        .unwrap_or("Container ship")
        .to_string();

    // Screen assumptions (documented, not manifest data): container-ship
    // block coefficient 0.70, waterplane coefficient 0.85, design draft
    // 65 % of depth, KG 55 % of depth.
    let cb = 0.70;
    let cwp = 0.85;
    let draft = 0.65 * depth;
    let kg = 0.55 * depth;
    let hull = HullForm {
        loa_m: loa,
        boa_m: boa,
        cb,
        cwp,
    };
    let loading = LoadingCondition::new(draft, kg);

    println!("Stability and scantling screen: {vessel} ({loa:.0} x {boa:.0} x {depth:.0} m)");
    println!("screen assumptions: cb {cb}, cwp {cwp}, draft {draft:.2} m, KG {kg:.2} m");
    println!("======================================================");

    // 1. Hydrostatics.
    let hs = hull.hydrostatics(draft);
    println!(
        "1. Hydrostatics: displacement {:.0} t, KB {:.2} m, KM {:.2} m, TPC {:.2} t/cm",
        hs.displacement_t, hs.kb_m, hs.km_m, hs.tpc_t_cm
    );

    // 2. Intact stability: IMO 2008 general criteria (the curve is cut
    // at 60 deg — the wall-sided term grows as tan^2 heel and the
    // prismatic model has no deck-edge limit, so far beyond the
    // criteria range the curve is not physically meaningful).
    let gz = hull.gz_curve(loading, 60.0);
    let verdict = hull.imo_2008_general(&gz, draft);
    println!(
        "2. IMO 2008 intact: {} (GM {:.2} m, max GZ {:.2} m at {:.0} deg, A30 {:.4} m*rad)",
        if verdict.passed { "PASS" } else { "FAIL" },
        verdict.gm_corrected_m,
        verdict.max_gz_m,
        verdict.max_gz_heel_deg,
        verdict.area_to_30_deg
    );
    for failure in &verdict.failures {
        println!("   - {failure}");
    }

    // 3. Hull girder: rule wave moments against a screen still-water
    // moment (half the wave sagging moment — the documented assumption
    // for a mid-loaded condition).
    let grade = "AH36";
    let k = high_strength_factor(grade).expect("grade maps");
    let (sag, _hog) = hull.wave_bending_moment().expect("length in rule range");
    let swbm = 0.5 * sag;
    let girder = hull
        .scantling_requirement(swbm, k)
        .expect("scantling screen");
    println!(
        "3. Hull girder ({grade}): wave sagging {sag:.0} kN*m, screen SWBM {swbm:.0} kN*m, \
allowable {:.0} MPa -> required modulus {:.3} m^3",
        girder.allowable_mpa, girder.required_modulus_m3
    );

    // 4. Damage stability screen: two double-bottom cases sized off the
    // hull (12 m x 18 m x 2.5 m = 540 m3 floodable volume each).
    let db = |name: &str, x: f64| DamageCompartment {
        name: name.to_string(),
        volume_m3: 540.0,
        centroid: (x, 0.0, 1.25),
        free_surface_moment_tm: 1.025 * 12.0 * 18.0_f64.powi(3) / 12.0,
    };
    let cases = [
        DamageCase {
            name: "DB 3 alone".into(),
            compartments: vec![db("DB 3", 0.15 * loa)],
        },
        DamageCase {
            name: "DB 4 alone".into(),
            compartments: vec![db("DB 4", -0.05 * loa)],
        },
    ];
    let summary = hull
        .damage_screen(hs.displacement_t, 0.0, kg, 250.0, &cases)
        .expect("damage cases solve");
    let governing = &summary.cases[summary.governing_index];
    println!(
        "4. Damage screen: {} of {} cases pass the 0.05 m floor; governing '{}': \
GM {:.2} m, list {:.2} deg",
        summary
            .cases
            .iter()
            .filter(|c| c.passes_one_compartment)
            .count(),
        summary.cases.len(),
        governing.name,
        governing.gm_m,
        governing.list_angle_deg
    );

    // 5. Probabilistic damage stability (cargo): three single-zone
    // damages along the cargo length with healthy survivability, at the
    // deepest condition only (the full method weighs three conditions).
    let ls = 0.96 * loa; // subdivision length screen
    let density = DamageLengthDensity::for_subdivision_length(ls).unwrap();
    let zones = [(0.25, 0.45), (0.45, 0.65), (0.65, 0.85)];
    let cases_p: Vec<_> = zones
        .iter()
        .map(|(x1, x2)| {
            let p = p_factor(&density, ls, x1 * ls, x2 * ls).expect("zone in range");
            let s = s_factor_cargo(0.0, 0.35, 25.0); // healthy damaged GZ screen
            format!("{:.0}-{:.0}%L p={p:.3} s={s:.2}", x1 * 100.0, x2 * 100.0)
        })
        .collect();
    let a_deepest = attained_subdivision_index(
        &(0..zones.len())
            .map(|i| {
                let (x1, x2) = zones[i];
                tpt_yard_hydrostatics::DamageCaseProbability {
                    name: format!("zone {i}"),
                    p_factor: p_factor(&density, ls, x1 * ls, x2 * ls).expect("zone"),
                    s_factor: s_factor_cargo(0.0, 0.35, 25.0),
                }
            })
            .collect::<Vec<_>>(),
    );
    let a = combined_cargo_index(a_deepest, 0.8 * a_deepest, 0.6 * a_deepest);
    if let Some(r) = required_index_cargo(loa) {
        println!(
            "5. Probabilistic damage (SOLAS): A = {a:.3} vs R = {r:.3} -> {}",
            if a >= r { "PASS" } else { "FAIL" }
        );
        println!("   {}", cases_p.join(", "));
        println!(
            "   screen note: three midship zones only — a design study enumerates all zone combinations, longitudinal bulkheads (r) and horizontal decks (v)"
        );
    } else {
        println!("5. Probabilistic damage (SOLAS): length below 100 m, R undefined");
    }

    // 6. Local scantlings: bottom plating (0.8 m frame spacing, 4:1
    // panels) under the hydrostatic head, AH36 with the CSR boundary
    // factor; the panel's in-plane demand is a screen 80 N/mm2.
    let pressure = 1.025 * 9.81 * draft; // kN/m2
    let plating = local_plate_scantling(LocalPlateScantlingInput {
        spacing_m: 0.8,
        long_span_m: Some(3.2),
        pressure_kn_m2: pressure,
        allowable_bending_mpa: 180.0,
        material_factor_k: k,
        boundary_factor: 1.2,
        corrosion_addition_mm: 1.5,
        applied_compression_mpa: Some(80.0),
        youngs_modulus_gpa: 206.0,
    })
    .expect("scantling inputs");
    println!(
        "6. Bottom plating (p {pressure:.1} kN/m2): net {:.2} mm ({:?} governs), \
with corrosion {:.2} mm; Euler {:.0} MPa at the governing thickness",
        plating.governing_net_mm,
        plating.mode,
        plating.with_corrosion_mm,
        plating.euler_stress_mpa.expect("buckling leg ran")
    );

    // 7. Frames: 0.8 m spacing over a 3.0 m span; bottom panel check
    // against the EN 1993-1-5 reduction.
    let frame =
        stiffener_scantling(0.8, 3.0, pressure, 180.0, k, Some(105.0)).expect("frame inputs");
    let buckling = plate_buckling_check_ec3(
        plating.governing_net_mm,
        800.0,
        3200.0,
        80.0,
        355.0,
        206_000.0,
        0.3,
    )
    .expect("buckling inputs");
    println!(
        "7. Frames (0.8 m x 3.0 m): end moment {:.1} kN*m -> Z {:.0} cm3, shear area {:.1} cm2; \
panel buckling lambda {:.2} -> capacity {:.0} MPa ({})",
        frame.end_moment_knm,
        frame.required_modulus_cm3,
        frame.required_shear_area_cm2.expect("shear leg ran"),
        buckling.lambda_bar,
        buckling.capacity_mpa,
        if buckling.passes { "PASS" } else { "FAIL" }
    );
}
