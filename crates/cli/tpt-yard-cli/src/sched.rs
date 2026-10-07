//! `schedule` (CPM + capacity levelling) and `risk` (Monte Carlo).

use std::collections::BTreeMap;

use tpt_yard_core::json::Value;

use crate::args::{self, CliError, Parsed, CHECK_PREFIX};

// ---------------------------------------------------------------- schedule

/// `schedule project.json [--limit kind=value]...`
///
/// Critical path plus capacity-aware resource levelling. Each `--limit`
/// states the yard-wide concurrent availability of one resource kind, in
/// the same units as the activities' resource capacities (e.g.
/// `--limit crane=200` for 200 t of crane, `--limit crew=40` for 40
/// welders); kinds without a limit are unconstrained.
/// The resource kinds `--limit` accepts.
const KINDS: &str = "crane|workshop|drydock|welding|robot|crew|transport";

/// Parses `--limit kind=value` arguments: known kind, finite positive
/// value, each kind at most once.
fn parse_limits(specs: &[&str]) -> Result<BTreeMap<String, f64>, CliError> {
    let mut limits = BTreeMap::new();
    for spec in specs {
        let (kind, val) = spec.split_once('=').ok_or_else(|| {
            CliError::Usage(format!("--limit {spec}: expected kind=value ({KINDS})"))
        })?;
        let kind = kind.to_ascii_lowercase();
        if !KINDS.split('|').any(|k| k == kind) {
            return Err(CliError::Usage(format!(
                "--limit {kind}: unknown resource kind (use {KINDS})"
            )));
        }
        let v = args::positive(&format!("limit {kind}"), val)?;
        if limits.insert(kind.clone(), v).is_some() {
            return Err(CliError::Usage(format!(
                "--limit {kind} given more than once"
            )));
        }
    }
    Ok(limits)
}

pub fn schedule(p: &Parsed, json_mode: bool) -> Result<(), CliError> {
    let limits = parse_limits(&p.all("limit"))?;
    schedule_impl(p.pos(0).unwrap_or_default(), &limits, json_mode).map_err(CliError::from)
}

fn schedule_impl(
    path: &str,
    limits: &BTreeMap<String, f64>,
    json_mode: bool,
) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = Value::parse(&text).map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
    let project = tpt_yard_core::VesselProject::from_json_value(&v)
        .map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
    let acts: Vec<tpt_yard_core::AssemblyActivity> = project
        .build_phases
        .iter()
        .flat_map(|p| p.activities.iter().cloned())
        .collect();
    if acts.is_empty() {
        return Err("project has no activities to schedule".into());
    }
    let kind_of = |name: &str| match name {
        "crane" => Some(tpt_yard_core::ResourceKind::Crane),
        "workshop" => Some(tpt_yard_core::ResourceKind::Workshop),
        "drydock" => Some(tpt_yard_core::ResourceKind::Drydock),
        "welding" => Some(tpt_yard_core::ResourceKind::WeldingStation),
        "robot" => Some(tpt_yard_core::ResourceKind::Robot),
        "crew" => Some(tpt_yard_core::ResourceKind::Crew),
        "transport" => Some(tpt_yard_core::ResourceKind::Transport),
        _ => None,
    };
    let mut limit_kinds: BTreeMap<tpt_yard_core::ResourceKind, f64> = BTreeMap::new();
    for (name, v) in limits {
        let kind = kind_of(name).ok_or_else(|| {
            format!(
                "--limit {name}: unknown resource kind (use crane|workshop|drydock|welding|robot|crew|transport)"
            )
        })?;
        limit_kinds.insert(kind, *v);
    }
    let scheduler = tpt_yard::tpt_yard_scheduling::ShipyardScheduler::new(acts);
    let cp = scheduler.critical_path().map_err(|e| format!("{e}"))?;
    let levelled = if limit_kinds.is_empty() {
        scheduler.resource_leveling()
    } else {
        scheduler.resource_leveling_with_limits(&limit_kinds)
    }
    .map_err(|e| format!("levelling: {e}"))?;
    let kind_name =
        |k: tpt_yard_core::ResourceKind| tpt_yard::tpt_yard_scheduling::resource_kind_name(k);
    if json_mode {
        let starts: Vec<String> = levelled
            .start_hours
            .iter()
            .map(|(a, s)| format!("[{},{}]", a.0, s))
            .collect();
        let lims: Vec<String> = limit_kinds
            .iter()
            .map(|(k, v)| format!("\"{}\":{}", kind_name(*k), v))
            .collect();
        let dock = levelled
            .dock_occupancy_h
            .map(|d| d.to_string())
            .unwrap_or_else(|| "null".into());
        println!(
            "{{\"critical_path\":[{}],\"makespan_h\":{:.1},\"dock_occupancy_h\":{},\"starts\":[{}],\"limits\":{{{}}}}}",
            cp.iter()
                .map(|a| a.0.to_string())
                .collect::<Vec<_>>()
                .join(","),
            levelled.makespan_hours,
            dock,
            starts.join(","),
            lims.join(",")
        );
        return Ok(());
    }
    println!(
        "Critical path: {} ({} activities), levelled makespan {:.1} h",
        cp.iter()
            .map(|a| format!("A{}", a.0))
            .collect::<Vec<_>>()
            .join(" -> "),
        cp.len(),
        levelled.makespan_hours
    );
    if let Some(dock) = levelled.dock_occupancy_h {
        println!("Dock occupancy: {:.1} h", dock);
    }
    if !limit_kinds.is_empty() {
        println!(
            "Capacity limits: {}",
            limit_kinds
                .iter()
                .map(|(k, v)| format!("{} {}", kind_name(*k), v))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    for note in &levelled.notes {
        println!("  - {note}");
    }
    for (a, s) in &levelled.start_hours {
        println!("  A{} starts at {:.1} h", a.0, s);
    }
    Ok(())
}

// ------------------------------------------------------------------- risk

/// `risk project.json [--samples N] [--uncertainty F] [--gate id=h[:slip]]...`
///
/// Monte Carlo schedule risk (review 7H): duration uncertainty per
/// activity plus optional delivery gates (an activity cannot start before
/// its material arrives; the gate itself slips triangularly).
/// Everything `risk` reads from the command line, validated.
struct RiskArgs {
    samples: u32,
    uncertainty: f64,
    seed: u64,
    gates: Vec<(u64, f64, f64)>,
    weather: Option<String>,
    weather_activity: Option<u64>,
    launch_method: String,
    max_sea_state: u32,
    vessel_mass_t: f64,
    way_length_m: f64,
    friction: f64,
    breadth_m: f64,
}

fn parse_risk(p: &Parsed) -> Result<RiskArgs, CliError> {
    let samples = p.count("samples", 1, 200_000)?.unwrap_or(2_000);
    let uncertainty = match p.number("uncertainty")? {
        Some(u) if (0.0..=1.0).contains(&u) => u,
        Some(u) => {
            return Err(CliError::Usage(format!(
            "--uncertainty is a fraction from 0 to 1 (above 1 gives negative durations), got {u}"
        )))
        }
        None => 0.25,
    };
    let seed = p.count("seed", 0, u64::MAX)?.unwrap_or(42);
    let mut gates = Vec::new();
    for spec in p.all("gate") {
        let (id_part, rest) = spec.split_once('=').ok_or_else(|| {
            CliError::Usage(format!("--gate {spec}: expected id=hours[:slippage]"))
        })?;
        let id = id_part.parse::<u64>().map_err(|_| {
            CliError::Usage(format!(
                "--gate {spec}: the activity id must be a whole number"
            ))
        })?;
        let (hours, slip) = match rest.split_once(':') {
            Some((h, sl)) => (h, Some(sl)),
            None => (rest, None),
        };
        let hours = args::number("gate hours", hours)?;
        let slip = slip
            .map(|s| args::number("gate slippage", s))
            .transpose()?
            .unwrap_or(0.0);
        if hours < 0.0 || !(0.0..=1.0).contains(&slip) {
            return Err(CliError::Usage(format!(
                "--gate {spec}: hours must be >= 0 and slippage a fraction from 0 to 1"
            )));
        }
        gates.push((id, hours, slip));
    }
    let launch_method = p.value("launch-method").unwrap_or("drydock");
    if !matches!(launch_method, "slipway" | "drydock" | "side" | "shiplift") {
        return Err(CliError::Usage(format!(
            "--launch-method must be slipway, drydock, side or shiplift, got '{launch_method}'"
        )));
    }
    Ok(RiskArgs {
        samples: u32::try_from(samples).unwrap_or(2_000),
        uncertainty,
        seed,
        gates,
        weather: p.value("weather").map(str::to_string),
        weather_activity: p.count("weather-activity", 0, u64::MAX)?,
        launch_method: launch_method.to_string(),
        max_sea_state: u32::try_from(p.count("max-sea-state", 0, 9)?.unwrap_or(3)).unwrap_or(3),
        vessel_mass_t: p.positive("vessel-mass-t")?.unwrap_or(4_000.0),
        way_length_m: p.positive("way-length-m")?.unwrap_or(120.0),
        friction: p.positive("friction")?.unwrap_or(0.02),
        breadth_m: p.positive("breadth-m")?.unwrap_or(20.0),
    })
}

pub fn risk(p: &Parsed, json_mode: bool) -> Result<(), CliError> {
    let a = parse_risk(p)?;
    risk_impl(p.pos(0).unwrap_or_default(), &a, json_mode)
}

fn risk_impl(path: &str, a: &RiskArgs, json_mode: bool) -> Result<(), CliError> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = Value::parse(&text).map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
    let project = tpt_yard_core::VesselProject::from_json_value(&v)
        .map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
    let acts: Vec<tpt_yard_core::AssemblyActivity> = project
        .build_phases
        .iter()
        .flat_map(|p| p.activities.iter().cloned())
        .collect();
    if acts.is_empty() {
        return Err("project has no activities to analyse".into());
    }
    let known = |id: u64| acts.iter().any(|x| x.id.0 == id);
    for (id, _, _) in &a.gates {
        if !known(*id) {
            return Err(CliError::Usage(format!(
                "--gate {id}: the project has no activity with that id"
            )));
        }
    }
    let scheduler = tpt_yard::tpt_yard_scheduling::ShipyardScheduler::new(acts.clone());
    let mut gate_objs: Vec<tpt_yard::tpt_yard_scheduling::DeliveryGate> = a
        .gates
        .iter()
        .map(
            |(id, h, slip)| tpt_yard::tpt_yard_scheduling::DeliveryGate {
                activity: tpt_yard::tpt_yard_assembly::ActivityId(*id),
                expected_available_h: *h,
                slippage_frac: *slip,
            },
        )
        .collect();

    // Weather window: a sea-state forecast gates the launch activity — it
    // cannot start before the first calm run of hours at or below the
    // sea-state limit. The gated activity is `--weather-activity`, else the
    // last activity of the plan (typically the launch).
    let mut weather: Option<(u64, usize, usize, f64)> = None;
    if let Some(fpath) = &a.weather {
        let activity = a
            .weather_activity
            .unwrap_or_else(|| acts.last().map_or(0, |x| x.id.0));
        if !known(activity) {
            return Err(CliError::Usage(format!(
                "--weather-activity {activity}: the project has no activity with that id"
            )));
        }
        let ftext = std::fs::read_to_string(fpath).map_err(|e| format!("reading {fpath}: {e}"))?;
        let fv = tpt_yard_core::json::Value::parse(&ftext)
            .map_err(|e| format!("{CHECK_PREFIX}{fpath}: {e}"))?;
        let forecast = tpt_yard::tpt_yard_earth_link::SeaStateForecast::from_json_value(&fv)
            .map_err(|e| format!("{CHECK_PREFIX}{fpath}: {e}"))?;
        let analysis = tpt_yard::tpt_yard_launch::LaunchAnalysis {
            launch_method: match a.launch_method.as_str() {
                "slipway" => tpt_yard::tpt_yard_launch::LaunchMethod::Slipway {
                    slope_deg: 3.0,
                    ways: 2,
                },
                "side" => tpt_yard::tpt_yard_launch::LaunchMethod::SideLaunch,
                "shiplift" => tpt_yard::tpt_yard_launch::LaunchMethod::Shiplift {
                    capacity_kn: 40_000.0,
                },
                _ => tpt_yard::tpt_yard_launch::LaunchMethod::DrydockFlooding,
            },
            vessel_weight: tpt_yard_core::MassProperties {
                mass_kg: a.vessel_mass_t * 1000.0,
                cog: tpt_yard_core::Vector3::new(0.0, 0.0, 6.0),
            },
            way_length_m: a.way_length_m,
            way_width_m: 2.0,
            friction_coefficient: a.friction,
            poppet_to_cog_m: 70.0,
            end_bearing_m: 20.0,
            immersion_length_m: 90.0,
            block_coefficient: 0.8,
            breadth_m: a.breadth_m,
            site: tpt_yard::tpt_yard_launch::SiteConditions {
                max_sea_state: a.max_sea_state,
            },
        };
        let window = tpt_yard::tpt_yard_earth_link::plan_launch_window(&analysis, &forecast)
            .map_err(|e| format!("weather: {e}"))?;
        weather = Some((
            activity,
            window.earliest_hour,
            window.calm_hours,
            window.sea_state_limit,
        ));
        gate_objs.insert(
            0,
            tpt_yard::tpt_yard_scheduling::DeliveryGate {
                activity: tpt_yard::tpt_yard_assembly::ActivityId(activity),
                expected_available_h: window.earliest_hour as f64,
                slippage_frac: 0.0,
            },
        );
    }
    let risk = scheduler
        .monte_carlo_risk_with_gates(&gate_objs, a.uncertainty, a.samples, a.seed)
        .map_err(|e| format!("{e}"))?;

    let top: Vec<(u64, f64)> = risk
        .criticality_frequency
        .iter()
        .filter(|(_a, f)| *f > 0.0)
        .take(5)
        .map(|(id, f)| (id.0, *f))
        .collect();
    if json_mode {
        use tpt_yard_core::json::Value as J;
        let num = J::Number;
        let mut fields = vec![
            ("samples", num(f64::from(risk.samples))),
            ("seed", num(a.seed as f64)),
            ("uncertainty", num(a.uncertainty)),
            ("p50_makespan_h", num(risk.p50_makespan_h)),
            ("p90_makespan_h", num(risk.p90_makespan_h)),
            ("mean_makespan_h", num(risk.mean_makespan_h)),
            (
                "gates",
                J::Array(
                    a.gates
                        .iter()
                        .map(|(id, h, s)| {
                            J::Object(vec![
                                ("activity".into(), num(*id as f64)),
                                ("hours".into(), num(*h)),
                                ("slippage".into(), num(*s)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "most_critical",
                J::Array(
                    top.iter()
                        .map(|(id, f)| {
                            J::Object(vec![
                                ("activity".into(), num(*id as f64)),
                                ("frequency".into(), num(*f)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ];
        if let Some((activity, from, hours, limit)) = weather {
            fields.push((
                "weather",
                J::Object(vec![
                    ("activity".into(), num(activity as f64)),
                    ("earliest_hour".into(), num(from as f64)),
                    ("calm_hours".into(), num(hours as f64)),
                    ("sea_state_limit".into(), num(limit)),
                    ("launch_method".into(), J::String(a.launch_method.clone())),
                ]),
            ));
        }
        let doc = J::Object(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        );
        println!("{}", doc.to_string_compact());
        return Ok(());
    }
    println!(
        "Schedule risk ({} samples, seed {}, duration uncertainty {:.0}%):",
        risk.samples,
        a.seed,
        a.uncertainty * 100.0
    );
    println!(
        "  makespan P50 {:.1} h | P90 {:.1} h | mean {:.1} h",
        risk.p50_makespan_h, risk.p90_makespan_h, risk.mean_makespan_h
    );
    if let Some((activity, from, hours, limit)) = weather {
        println!(
            "  weather gate on A{activity}: safe from hour {from} for {hours} h at sea state <= {limit} ({}, vessel {:.0} t)",
            a.launch_method, a.vessel_mass_t
        );
    }
    if !a.gates.is_empty() {
        println!(
            "  delivery gates: {}",
            a.gates
                .iter()
                .map(|(id, h, slip)| format!("A{id} @ {h:.0} h (+/-{:.0}%)", slip * 100.0))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !top.is_empty() {
        println!(
            "  most critical: {}",
            top.iter()
                .map(|(id, f)| format!("A{id} ({:.0}%)", f * 100.0))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}
