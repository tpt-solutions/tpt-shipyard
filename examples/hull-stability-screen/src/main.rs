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
    attained_subdivision_index, combined_cargo_index, cross_flooding_time, high_strength_factor,
    local_plate_scantling, multi_zone_p_factor, p_factor, plate_buckling_check_ec3,
    required_index_cargo, s_factor_cargo, s_intermediate_factor, stiffener_scantling, DamageCase,
    DamageLengthDensity, HullForm, LoadingCondition, LocalPlateScantlingInput, TankCompartment,
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

    // 4. Damage stability screen: two double-bottom TANKS with real
    // vertical-walled geometry (12 m x 18 m plan, 2.5 m height, mu 0.85),
    // flooded to the full stage via the tank-plan geometry bridge.
    let tank = |name: &str, x: f64| {
        (
            name.to_string(),
            x,
            TankCompartment {
                name: name.to_string(),
                bottom_z: 0.0,
                plan_area_m2: 12.0 * 18.0,
                height_m: 2.5,
                permeability: 0.85,
            },
        )
    };
    let tanks = [tank("DB 3", 0.15 * loa), tank("DB 4", -0.05 * loa)];
    let cases: Vec<DamageCase> = tanks
        .iter()
        .map(|(name, x, t)| {
            let mut c =
                tpt_yard_hydrostatics::tank_stage_compartment(t, 1.0, None).expect("tank geometry");
            c.name = name.clone();
            c.centroid.0 = *x;
            DamageCase {
                name: name.clone(),
                compartments: vec![c],
            }
        })
        .collect();
    let summary = hull
        .damage_screen(hs.displacement_t, 0.0, kg, 250.0, &cases)
        .expect("damage cases solve");
    let governing = &summary.cases[summary.governing_index];
    println!(
        "4. Damage screen (tank plan): {} of {} cases pass the 0.05 m floor; governing '{}': GM {:.2} m, list {:.2} deg",
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
    // Staged flooding of the governing case: per-stage heel/GZ with the
    // Reg. 7-2.2 intermediate factor per stage, plus the Torricelli
    // cross-flooding equalization time through a 0.2 m2 duct.
    let gov_compartments = &cases[summary.governing_index].compartments;
    let stages = hull
        .damage_stages(hs.displacement_t, 0.0, kg, 250.0, gov_compartments, 4)
        .expect("stages solve");
    let worst_int = stages
        .iter()
        .map(|st| s_intermediate_factor(st.heel_deg, st.gz_max_m, st.range_deg, false))
        .fold(1.0_f64, f64::min);
    let eq_time = cross_flooding_time(2.0, 0.0, 0.2, 0.6, 12.0 * 18.0, 40.0).expect("duct inputs");
    println!(
        "   staged flood (4 stages): worst intermediate s = {worst_int:.2}; cross-flooding equalization {eq_time:.0} s ({})",
        if eq_time <= 600.0 {
            "within 10 min"
        } else {
            "EXCEEDS 10 min"
        }
    );

    // 5. Probabilistic damage stability (cargo): one three-zone group
    // (the midbody) scored with the p·r combination through a 2 m wing
    // bulkhead, plus the single-zone contributions, at the deepest
    // condition only (the full method weighs three conditions).
    let ls = 0.96 * loa; // subdivision length screen
    let density = DamageLengthDensity::for_subdivision_length(ls).unwrap();
    let bulkhead_b = 2.0; // m off the shell: one longitudinal bulkhead
    let zones = [(0.25, 0.45), (0.45, 0.65), (0.65, 0.85)];
    // The three-zone GROUP factor: the p combination with the r
    // reduction through the wing bulkhead. The alternating form can
    // cancel to a hair negative for bulkheads shallow against the zone
    // spans; a probability floors at zero.
    let group = {
        let z: Vec<(f64, f64)> = zones.iter().map(|(x1, x2)| (x1 * ls, x2 * ls)).collect();
        multi_zone_p_factor(&density, ls, boa, &z, Some(bulkhead_b))
            .expect("zones")
            .max(0.0)
    };
    let singles: Vec<f64> = zones
        .iter()
        .map(|(x1, x2)| p_factor(&density, ls, x1 * ls, x2 * ls).expect("zone"))
        .collect();
    let cases_p: Vec<_> = zones
        .iter()
        .zip(&singles)
        .map(|((x1, x2), p)| {
            let s = s_factor_cargo(0.0, 0.35, 25.0); // healthy damaged GZ screen
            format!("{:.0}-{:.0}%L p={p:.3} s={s:.2}", x1 * 100.0, x2 * 100.0)
        })
        .collect();
    // The group (3-zone, bulkhead-reduced) plus the two outboard
    // single-zone damages, all with healthy survivability.
    let a_deepest = attained_subdivision_index(&[
        tpt_yard_hydrostatics::DamageCaseProbability {
            name: "3-zone midbody group".into(),
            p_factor: group,
            s_factor: s_factor_cargo(0.0, 0.35, 25.0),
        },
        tpt_yard_hydrostatics::DamageCaseProbability {
            name: "fore single zone".into(),
            p_factor: singles[0],
            s_factor: 0.5,
        },
    ]);
    let a = combined_cargo_index(a_deepest, 0.8 * a_deepest, 0.6 * a_deepest);
    if let Some(r_req) = required_index_cargo(loa) {
        println!(
            "5. Probabilistic damage (SOLAS): A = {a:.3} vs R = {r_req:.3} -> {}",
            if a >= r_req { "PASS" } else { "FAIL" }
        );
        println!(
            "   3-zone group (25-85%L, 2 m wing bulkhead): p·r = {group:.3}; singles p = {}",
            singles
                .iter()
                .map(|p| format!("{p:.3}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("   {}", cases_p.join(", "));
        println!(
            "   screen note: one group plus two singles — a design study enumerates all zone combinations and horizontal decks (v)"
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
