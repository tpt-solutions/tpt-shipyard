//! Project commands: `validate`, `plan`, `report` and `new` (scaffolding).

use tpt_yard_core::{json::Value, MassProperties, Vector3};

use crate::args::{self, CliError, Parsed, CHECK_PREFIX};

/// A hull manifest as the planner consumes it (shared by `validate`,
/// `plan` and `export`).
pub(crate) struct Manifest {
    pub vessel: Option<String>,
    pub hull: tpt_yard::tpt_yard_hull::HullGeometry,
    pub crane_kn: f64,
    pub workshop: tpt_yard::tpt_yard_core::Dimensions,
}

/// Reads and validates a hull manifest: every dimension finite and positive,
/// `depth_bands` a whole number from 1 to 1000, and the workshop section
/// present.
///
/// # Errors
///
/// A message naming the file and the offending field; problems with the file
/// contents carry the check prefix (exit 1), I/O problems do not (exit 3).
pub(crate) fn load_manifest(path: &str) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let bad = |m: String| format!("{CHECK_PREFIX}{path}: {m}");
    let v = Value::parse(&text).map_err(|e| bad(e.to_string()))?;
    let num = |o: &Value, k: &str| -> Result<f64, String> {
        o.get(k)
            .and_then(Value::as_f64)
            .filter(|n| n.is_finite() && *n > 0.0)
            .ok_or_else(|| bad(format!("manifest: '{k}' missing or not a positive number")))
    };
    let hull_v = v
        .get("hull")
        .ok_or_else(|| bad("manifest: 'hull' section missing".into()))?;
    let bands = hull_v
        .get("depth_bands")
        .and_then(Value::as_u64)
        .filter(|b| (1..=1000).contains(b))
        .ok_or_else(|| {
            bad("manifest: 'depth_bands' must be a whole number from 1 to 1000".into())
        })?;
    let hull = tpt_yard::tpt_yard_hull::HullGeometry {
        loa_m: num(hull_v, "loa_m")?,
        boa_m: num(hull_v, "boa_m")?,
        depth_m: num(hull_v, "depth_m")?,
        areal_density_kg_m2: num(hull_v, "areal_density_kg_m2")?,
        depth_bands: u32::try_from(bands).map_err(|_| bad("depth_bands out of range".into()))?,
    };
    let yard = v
        .get("yard_capabilities")
        .ok_or_else(|| bad("manifest: 'yard_capabilities' section missing".into()))?;
    let crane_kn = num(yard, "crane_capacity_kn")?;
    let ws = yard
        .get("workshop")
        .ok_or_else(|| bad("manifest: 'workshop' section missing".into()))?;
    let workshop = tpt_yard::tpt_yard_core::Dimensions::new(
        num(ws, "length_m")?,
        num(ws, "breadth_m")?,
        num(ws, "depth_m")?,
    );
    Ok(Manifest {
        vessel: v.get("vessel").and_then(Value::as_str).map(str::to_string),
        hull,
        crane_kn,
        workshop,
    })
}

/// Reads a project file as a vessel project (file contents problems are
/// check failures, I/O problems are runtime failures).
fn load_project(path: &str) -> Result<tpt_yard_core::VesselProject, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = Value::parse(&text).map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
    tpt_yard_core::VesselProject::from_json_value(&v)
        .map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))
}

/// Prints a document to stdout, or writes it to `--out` (refusing to
/// overwrite without `--force`).
fn emit(doc: &str, out: Option<&str>, force: bool) -> Result<(), String> {
    match out {
        Some(path) => {
            args::write_output(path, format!("{doc}\n").as_bytes(), force)?;
            eprintln!("wrote {path}");
        }
        None => println!("{doc}"),
    }
    Ok(())
}

// -------------------------------------------------------------------- plan

/// The end-to-end plan (7H): block division → erection order → lift checks
/// → schedule → critical path, one report.
pub fn plan(p: &Parsed, json_mode: bool) -> Result<(), CliError> {
    plan_impl(p.pos(0).unwrap_or_default(), json_mode).map_err(CliError::from)
}

fn plan_impl(path: &str, json_mode: bool) -> Result<(), String> {
    let Manifest {
        vessel,
        hull,
        crane_kn,
        workshop,
    } = load_manifest(path)?;

    // 1. Block division (plus the explicit workshop cross-section check:
    // blocks are the full beam wide, so a narrow workshop is infeasible
    // even when the lengths divide cleanly).
    let construction = tpt_yard::tpt_yard_hull::HullConstruction::new(hull);
    if let Err(workshop_violation) = construction.check_workshop(workshop) {
        return Err(format!(
            "{CHECK_PREFIX}workshop cross-section: {workshop_violation}"
        ));
    }
    let blocks = construction.block_division(crane_kn, workshop);
    if blocks.is_empty() {
        return Err("block division produced no blocks (depth_bands == 0?)".into());
    }

    // 2. Erection order.
    let joins = construction.erection_sequence(&blocks);

    // 3. Lift check for the heaviest block (4-point pick at its corners).
    let heaviest = blocks
        .iter()
        .max_by(|a, b| a.weight_kg.total_cmp(&b.weight_kg))
        .expect("non-empty");
    let (hw, hd) = (
        heaviest.dimensions().length / 2.0,
        heaviest.dimensions().breadth / 2.0,
    );
    let c = heaviest.geometry.centre;
    let lift_points = [
        Vector3::new(c.x - hw, c.y - hd, 0.0),
        Vector3::new(c.x + hw, c.y - hd, 0.0),
        Vector3::new(c.x + hw, c.y + hd, 0.0),
        Vector3::new(c.x - hw, c.y + hd, 0.0),
    ];
    let solver = tpt_yard::tpt_yard_structural::ConstructionStructuralSolver::new(
        tpt_yard::tpt_yard_structural::PartialStructure::default(),
        355.0,
    );
    let lift = solver
        .lifting_analysis(
            &tpt_yard::tpt_yard_structural::LiftableBody {
                mass_properties: MassProperties {
                    mass_kg: heaviest.weight_kg,
                    cog: heaviest.cog,
                },
            },
            crane_kn, // sling capacity tracks the crane
            crane_kn, // single-crane pick
            &lift_points,
            4.0,
        )
        .map_err(|e| format!("lift check: {e}"))?;

    // 4. Schedule: one erection activity per block, in erection order.
    let acts: Vec<tpt_yard_core::AssemblyActivity> = joins
        .iter()
        .map(|j| {
            let block = blocks
                .iter()
                .find(|b| b.id == j.block)
                .expect("join references a block");
            let deps: Vec<tpt_yard::tpt_yard_assembly::ActivityId> = j
                .onto
                .map(|onto| {
                    joins
                        .iter()
                        .position(|x| x.block == onto)
                        .map(|pos| tpt_yard::tpt_yard_assembly::ActivityId(pos as u64 + 1))
                        .into_iter()
                        .collect()
                })
                .unwrap_or_default();
            tpt_yard_core::AssemblyActivity::new(
                tpt_yard::tpt_yard_assembly::ActivityId(j.sequence_index as u64 + 1),
                format!("erect {}", block.name),
                tpt_yard_core::ActivityType::JoinBlock,
                8.0,
            )
            .with_dependencies(&deps)
        })
        .collect();
    let scheduler = tpt_yard::tpt_yard_scheduling::ShipyardScheduler::new(acts);
    let early = scheduler
        .optimize_sequence(tpt_yard::tpt_yard_scheduling::ScheduleObjective::MinimizeDuration)
        .map_err(|e| format!("schedule: {e}"))?;
    let levelled = scheduler
        .resource_leveling()
        .map_err(|e| format!("levelling: {e}"))?;
    let cp = scheduler
        .critical_path()
        .map_err(|e| format!("critical path: {e}"))?;

    // 5. Report.
    if json_mode {
        println!(
            "{{\"blocks\":{},\"n_joins\":{},\"heaviest_block_kg\":{:.0},\"lift_safe\":{},\"makespan_h\":{:.1},\"levelled_makespan_h\":{:.1},\"critical_path_len\":{}}}",
            blocks.len(),
            joins.len(),
            heaviest.weight_kg,
            lift.safe,
            early.makespan_hours,
            levelled.makespan_hours,
            cp.len()
        );
        return Ok(());
    }
    let total_steel: f64 = blocks.iter().map(|b| b.weight_kg).sum();
    println!(
        "Construction plan for {}",
        vessel.as_deref().unwrap_or(path)
    );
    println!("======================================================");
    println!(
        "1. Block division: {} blocks, {:.0} t total steel",
        blocks.len(),
        total_steel / 1000.0
    );
    println!(
        "2. Erection order: {} joins; first block on the dock floor, tiers bottom-up",
        joins.len()
    );
    println!(
        "3. Lift check (heaviest block {:.0} t, 4-point pick): {}",
        heaviest.weight_kg / 1000.0,
        if lift.safe { "SAFE" } else { "UNSAFE" }
    );
    for n in &lift.notes {
        println!("   - {n}");
    }
    println!(
        "4. Schedule: makespan {:.1} h (levelled {:.1} h), critical path {} activities",
        early.makespan_hours,
        levelled.makespan_hours,
        cp.len()
    );
    println!("5. Drydock/launch: run the drydock and launch crates on the launch weight for the float-out plan");
    Ok(())
}

// ------------------------------------------------------------------ report

pub fn report(p: &Parsed, json_mode: bool) -> Result<(), CliError> {
    report_impl(p.pos(0).unwrap_or_default(), json_mode).map_err(CliError::from)
}

fn report_impl(path: &str, json_mode: bool) -> Result<(), String> {
    let project = load_project(path)?;
    let twin = tpt_yard::tpt_yard_digital_twin::DigitalTwin::new(project);
    let report = twin.weight_model().weight_report();
    let check = twin.structural_check();
    if json_mode {
        println!(
            "{{\"design_kg\":{:.1},\"best_estimate_kg\":{:.1},\"margin_kg\":{:.1},\"structural_passed\":{}}}",
            report.design_weight_kg, report.total_weight_kg, report.margin_kg, check.passed
        );
        return Ok(());
    }
    println!("Project report: {}", twin.vessel.name);
    println!("  design weight   {:>10.0} kg", report.design_weight_kg);
    println!("  best estimate   {:>10.0} kg", report.total_weight_kg);
    println!("  growth margin   {:>10.0} kg", report.margin_kg);
    println!(
        "  structural check at start: {}",
        if check.passed { "passed" } else { "FAILED" }
    );
    Ok(())
}

// --------------------------------------------------------------------- new

pub fn new(p: &Parsed, json_mode: bool) -> Result<(), CliError> {
    let kind = p.pos(0).unwrap_or_default();
    new_impl(kind, p.pos(1), p.value("out"), p.has("force"), json_mode).map_err(|e| {
        if e.starts_with("unknown kind") || e.starts_with("unknown template") {
            CliError::Usage(e)
        } else {
            CliError::from(e)
        }
    })
}

fn new_impl(
    kind: &str,
    name_arg: Option<&str>,
    out: Option<&str>,
    force: bool,
    json_mode: bool,
) -> Result<(), String> {
    // Named project templates (review 7F).
    if matches!(
        kind,
        "container-ship" | "submarine" | "orbital-truss" | "habitat" | "solar-array"
    ) {
        return template(kind, name_arg, out, force, json_mode);
    }
    let (phases, vessel, method, name): (
        Vec<tpt_yard_core::BuildPhase>,
        tpt_yard_core::VesselType,
        _,
        String,
    ) = match kind {
        "sea" => {
            let mut phase =
                tpt_yard_core::BuildPhase::new(tpt_yard_core::PhaseId(1), "Erection", 8.0);
            for i in 1..=4u64 {
                let deps: Vec<tpt_yard::tpt_yard_assembly::ActivityId> = if i == 1 {
                    vec![]
                } else {
                    vec![tpt_yard::tpt_yard_assembly::ActivityId(i - 1)]
                };
                phase.activities.push(
                    tpt_yard_core::AssemblyActivity::new(
                        tpt_yard::tpt_yard_assembly::ActivityId(i),
                        format!("Erect block {i}"),
                        tpt_yard_core::ActivityType::JoinBlock,
                        8.0,
                    )
                    .with_dependencies(&deps),
                );
            }
            phase.weight_state = tpt_yard_core::WeightState {
                design_kg: 4_000_000.0,
                installed_kg: 0.0,
            };
            (
                vec![phase],
                tpt_yard_core::VesselType::Sea(tpt_yard_core::SeaVesselType::ContainerShip {
                    teu_capacity: 800,
                }),
                tpt_yard_core::ConstructionMethod::SeaDrydock,
                name_arg.unwrap_or("New sea vessel").to_string(),
            )
        }
        "space" => {
            let mut phase =
                tpt_yard_core::BuildPhase::new(tpt_yard_core::PhaseId(1), "Orbital assembly", 12.0);
            for i in 1..=3u64 {
                phase.activities.push(tpt_yard_core::AssemblyActivity::new(
                    tpt_yard::tpt_yard_assembly::ActivityId(i),
                    format!("Install module {i}"),
                    tpt_yard_core::ActivityType::JoinBlock,
                    10.0,
                ));
            }
            phase.weight_state = tpt_yard_core::WeightState {
                design_kg: 150_000.0,
                installed_kg: 0.0,
            };
            (
                vec![phase],
                tpt_yard_core::VesselType::Space(tpt_yard_core::SpaceVesselType::SpaceStation {
                    modules: 3,
                }),
                tpt_yard_core::ConstructionMethod::OrbitalAssembly,
                name_arg.unwrap_or("New space vessel").to_string(),
            )
        }
        other => return Err(format!("unknown kind '{other}' (expected sea|space)")),
    };
    let project = tpt_yard_core::VesselProject::new(
        tpt_yard_core::ProjectId(1),
        name,
        vessel,
        method,
        phases,
    )
    .map_err(|e| format!("scaffold failed: {e}"))?;
    let json = project.to_json().to_string_pretty();
    // The banner goes to stderr so `tpt-yard new sea > project.json` is
    // valid JSON; `--out FILE` writes the file directly.
    if !json_mode && out.is_none() {
        eprintln!(
            "(Scaffolded project: save it as project.json, or use --out, then run `tpt-yard validate project.json`)"
        );
    }
    emit(&json, out, force)
}

/// Scaffold one of the named reference templates. Each template mirrors the
/// matching reference case in `test-data/` so the numbers are realistic.
fn template(
    kind: &str,
    name: Option<&str>,
    out: Option<&str>,
    force: bool,
    json_mode: bool,
) -> Result<(), String> {
    use tpt_yard::tpt_yard_assembly::ActivityId;
    let mut next_activity = 1u64;
    let mut mk = |idx: u64, phase_name: &str, days: f64, design_kg: f64, acts: &[(&str, f64)]| {
        let mut phase =
            tpt_yard_core::BuildPhase::new(tpt_yard_core::PhaseId(idx), phase_name, days);
        let base = next_activity;
        for (i, (n, h)) in acts.iter().enumerate() {
            let id = ActivityId(base + i as u64);
            let deps: Vec<ActivityId> = if i == 0 {
                vec![]
            } else {
                vec![ActivityId(base + i as u64 - 1)]
            };
            phase.activities.push(
                tpt_yard_core::AssemblyActivity::new(
                    id,
                    *n,
                    tpt_yard_core::ActivityType::JoinBlock,
                    *h,
                )
                .with_dependencies(&deps),
            );
        }
        phase.weight_state = tpt_yard_core::WeightState {
            design_kg,
            installed_kg: 0.0,
        };
        next_activity += acts.len() as u64;
        phase
    };
    let (name, vessel, method, phases) = match kind {
        "container-ship" => (
            name.unwrap_or("Container ship 1400 TEU").to_string(),
            tpt_yard_core::VesselType::Sea(tpt_yard_core::SeaVesselType::ContainerShip {
                teu_capacity: 1400,
            }),
            tpt_yard_core::ConstructionMethod::SeaDrydock,
            vec![
                mk(
                    1,
                    "Steel prefabrication",
                    28.0,
                    4_000_000.0,
                    &[("Cut steel", 8.0), ("Form panels", 8.0)],
                ),
                mk(
                    2,
                    "Block assembly",
                    21.0,
                    2_000_000.0,
                    &[("Assemble blocks", 16.0)],
                ),
                mk(
                    3,
                    "Dock erection",
                    30.0,
                    1_500_000.0,
                    &[("Erect tier 0", 8.0), ("Erect tier 1", 8.0)],
                ),
                mk(
                    4,
                    "Outfitting & launch",
                    25.0,
                    900_000.0,
                    &[("Outfit", 8.0), ("Launch", 4.0)],
                ),
            ],
        ),
        "submarine" => (
            name.unwrap_or("Submarine pressure hull").to_string(),
            tpt_yard_core::VesselType::Sea(tpt_yard_core::SeaVesselType::Submarine {
                hull_type: tpt_yard_core::HullType::DoubleHull,
            }),
            tpt_yard_core::ConstructionMethod::SeaBlockConstruction,
            vec![mk(
                1,
                "Pressure hull sections",
                40.0,
                2_300_000.0,
                &[
                    ("Roll and weld sections", 12.0),
                    ("Fit ring stiffeners", 10.0),
                    ("Hydro test", 6.0),
                ],
            )],
        ),
        "orbital-truss" => (
            name.unwrap_or("ISS-class truss").to_string(),
            tpt_yard_core::VesselType::Space(tpt_yard_core::SpaceVesselType::SpaceStation {
                modules: 5,
            }),
            tpt_yard_core::ConstructionMethod::OrbitalAssembly,
            vec![mk(
                1,
                "On-orbit truss assembly",
                45.0,
                7_500.0,
                &[
                    ("Deploy bay 1", 6.0),
                    ("Deploy bay 2", 6.0),
                    ("Deploy bay 3", 6.0),
                ],
            )],
        ),
        "habitat" => (
            name.unwrap_or("Stanford torus habitat").to_string(),
            tpt_yard_core::VesselType::Space(tpt_yard_core::SpaceVesselType::SpaceStation {
                modules: 8,
            }),
            tpt_yard_core::ConstructionMethod::InSpaceManufacturing,
            vec![mk(
                1,
                "Habitat construction",
                120.0,
                10_000_000.0,
                &[
                    ("Print ring sections", 24.0),
                    ("Close the ring", 16.0),
                    ("Spin-up checkout", 8.0),
                ],
            )],
        ),
        "solar-array" => (
            name.unwrap_or("Space solar array").to_string(),
            tpt_yard_core::VesselType::Space(tpt_yard_core::SpaceVesselType::SpaceStation {
                modules: 2,
            }),
            tpt_yard_core::ConstructionMethod::InSpaceManufacturing,
            vec![mk(
                1,
                "Array manufacture",
                30.0,
                12_000.0,
                &[
                    ("Print substrate", 12.0),
                    ("Lay cells", 10.0),
                    ("Deploy & verify", 6.0),
                ],
            )],
        ),
        other => return Err(format!("unknown template '{other}'")),
    };
    let project = tpt_yard_core::VesselProject::new(
        tpt_yard_core::ProjectId(1),
        name,
        vessel,
        method,
        phases,
    )
    .map_err(|e| format!("template failed: {e}"))?;
    if !json_mode && out.is_none() {
        eprintln!("(save as project.json, or use --out, then `tpt-yard validate project.json`)");
    }
    emit(&project.to_json().to_string_pretty(), out, force)
}

pub fn validate(p: &Parsed, json_mode: bool) -> Result<(), CliError> {
    validate_impl(p.pos(0).unwrap_or_default(), json_mode).map_err(CliError::from)
}

fn validate_impl(path: &str, json_mode: bool) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = Value::parse(&text).map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
    if v.get("build_phases").is_some() {
        let project = tpt_yard_core::VesselProject::from_json_value(&v)
            .map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
        let acts: usize = project
            .build_phases
            .iter()
            .map(|p| p.activities.len())
            .sum();
        if json_mode {
            println!(
                "{{\"ok\":true,\"kind\":\"project\",\"name\":\"{}\",\"phases\":{},\"activities\":{acts}}}",
                args::json_escape(&project.name),
                project.build_phases.len()
            );
        } else {
            println!(
                "OK: project '{}' ({} phases, {acts} activities) passes validation",
                project.name,
                project.build_phases.len()
            );
        }
    } else {
        // Hull manifest: the shape the planner consumes, including the
        // workshop section and the cross-section fit.
        let m = load_manifest(path)?;
        tpt_yard::tpt_yard_hull::HullConstruction::new(m.hull)
            .check_workshop(m.workshop)
            .map_err(|e| format!("{CHECK_PREFIX}{path}: workshop cross-section: {e}"))?;
        if json_mode {
            println!("{{\"ok\":true,\"kind\":\"hull-manifest\"}}");
        } else {
            println!("OK: hull manifest '{path}' is well-formed and the workshop fits the blocks");
        }
    }
    Ok(())
}
