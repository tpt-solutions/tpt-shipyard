//! `tpt-yard` — the command-line front end for tpt-shipyard.
//!
//! Subcommands (review 7F CLI item, with the 7H end-to-end `plan`):
//!
//! - `validate FILE.json` — load a vessel project (or hull manifest) and
//!   report every inconsistency; exit 1 on failure.
//! - `plan MANIFEST.json` — the end-to-end construction plan for a hull
//!   manifest: block division → erection order → lift checks → schedule →
//!   critical path, one report.
//! - `schedule FILE.json [--limit kind=value]...` — CPM critical path and
//!   resource levelling for a vessel project's activities; `--limit`
//!   states yard-wide capacities per resource kind (crane tonnes, crew
//!   persons, …) for capacity-aware levelling that shares resources up to
//!   the limit.
//! - `report FILE.json` — weight/CoG/structural summary for a project.
//! - `new <sea|space> [NAME]` — print a scaffolded project JSON built from
//!   the workspace templates.
//!
//! Every command takes `--json` to emit machine-readable output.

use std::collections::BTreeMap;
use std::process::ExitCode;

use tpt_yard_core::{json::Value, MassProperties, Vector3};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (json_mode, args) = split_flag(&args, "--json");
    match run(&args, json_mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!(
                "usage: tpt-yard <validate|plan|schedule|risk|report|new|html-report|pdf-report> [--json] ..."
            );
            ExitCode::FAILURE
        }
    }
}

fn split_flag<'a>(args: &'a [String], flag: &str) -> (bool, Vec<&'a str>) {
    let mut json = false;
    let mut rest = Vec::new();
    for a in args {
        if a == flag {
            json = true;
        } else {
            rest.push(a.as_str());
        }
    }
    (json, rest)
}

fn run(args: &[&str], json_mode: bool) -> Result<(), String> {
    let (cmd, rest) = args.split_first().ok_or("missing subcommand")?;
    match *cmd {
        "validate" => validate(rest.first().ok_or("validate needs a file path")?),
        "plan" => plan(
            rest.first().ok_or("plan needs a hull manifest path")?,
            json_mode,
        ),
        "schedule" => schedule(rest, json_mode),
        "risk" => risk(rest, json_mode),
        "report" => report(rest.first().ok_or("report needs a file path")?, json_mode),
        "new" => new(rest, json_mode),
        "html-report" => {
            // --out and --structure are accepted anywhere among the
            // arguments.
            let mut out = "report.html".to_string();
            let mut structure_path: Option<&str> = None;
            let mut args: Vec<&str> = Vec::new();
            let mut it = rest.iter();
            while let Some(a) = it.next() {
                match *a {
                    "--out" => {
                        out = it.next().copied().ok_or("--out needs a path")?.to_string();
                    }
                    "--structure" => {
                        structure_path =
                            Some(it.next().copied().ok_or("--structure needs a path")?);
                    }
                    _ => args.push(a),
                }
            }
            html_report(
                args.first().ok_or("html-report needs a project path")?,
                &out,
                structure_path,
            )
        }
        "pdf-report" => {
            let mut out = "report.pdf".to_string();
            let mut args: Vec<&str> = Vec::new();
            let mut it = rest.iter();
            while let Some(a) = it.next() {
                if *a == "--out" {
                    out = it.next().copied().ok_or("--out needs a path")?.to_string();
                } else {
                    args.push(a);
                }
            }
            pdf_report(args.first().ok_or("pdf-report needs a project path")?, &out)
        }
        other => Err(format!("unknown subcommand '{other}'")),
    }
}

/// The HTML calculation-package report (review 7H roadmap item: "report
/// generator — HTML/PDF calculation package per phase").
fn html_report(path: &str, out_path: &str, structure_path: Option<&str>) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = tpt_yard_core::json::Value::parse(&text).map_err(|e| format!("{path}: {e}"))?;
    let project =
        tpt_yard_core::VesselProject::from_json_value(&v).map_err(|e| format!("{path}: {e}"))?;
    let twin = tpt_yard::tpt_yard_digital_twin::DigitalTwin::new(project);
    let report = twin.weight_model().weight_report();
    let check = twin.structural_check();

    // Schedule and risk sections (review 7H report package): critical
    // path + levelled makespan, then a Monte Carlo risk run with default
    // screening settings (2000 samples, +/-25 % duration uncertainty).
    let acts: Vec<tpt_yard_core::AssemblyActivity> = twin
        .vessel
        .build_phases
        .iter()
        .flat_map(|p| p.activities.iter().cloned())
        .collect();
    // Per-phase FEM (review 7H report item): given an optional staged
    // structure, analyse every erection phase with its loads.
    let fem_section = match structure_path {
        Some(spath) => {
            let text =
                std::fs::read_to_string(spath).map_err(|e| format!("reading {spath}: {e}"))?;
            let sv =
                tpt_yard_core::json::Value::parse(&text).map_err(|e| format!("{spath}: {e}"))?;
            let loaded = tpt_yard::tpt_yard_structural::PartialStructure::from_json_with_loads(&sv)
                .map_err(|e| format!("{spath}: {e}"))?;
            let max_phase = loaded
                .structure
                .elements
                .iter()
                .map(|e| e.erected_at.0)
                .max()
                .unwrap_or(0);
            let solver = tpt_yard::tpt_yard_structural::ConstructionStructuralSolver::new(
                loaded.structure.clone(),
                355.0,
            );
            let mut fem_rows = String::new();
            for phase in 1..=max_phase {
                let result = solver.analyze_at_phase(tpt_yard_core::PhaseId(phase), &loaded.loads);
                match result {
                    Ok(r) => fem_rows.push_str(&format!(
                        "<tr><td>{phase}</td><td>{}</td><td>{:.1}</td><td>{:.2}</td><td>{:.1}</td><td class=\"{}\">{}</td></tr>
",
                        r.active_members,
                        r.max_displacement_mm,
                        r.max_axial_stress_mpa,
                        r.max_utilization * 100.0,
                        if r.passed { "pass" } else { "fail" },
                        if r.passed { "OK" } else { "OVERSTRESSED" },
                    )),
                    Err(e) => fem_rows.push_str(&format!(
                        "<tr><td>{phase}</td><td colspan=\"5\">{e}</td></tr>
"
                    )),
                }
            }
            format!(
                "<table>
<tr><th>Phase</th><th>Members</th><th>Max deflection (mm)</th><th>Max stress (MPa)</th><th>Utilization</th><th>Verdict</th></tr>
{fem_rows}</table>
<p>Loads per the structure file; allowable 355 MPa.</p>"
            )
        }
        None => {
            "<p>No staged structure supplied (pass --structure partial.json for per-phase FEM).</p>"
                .to_string()
        }
    };

    let (sched_section, risk_section) = if acts.is_empty() {
        (
            "<p>No activities to schedule.</p>".to_string(),
            String::new(),
        )
    } else {
        let scheduler = tpt_yard::tpt_yard_scheduling::ShipyardScheduler::new(acts.clone());
        let sched_section = match scheduler.critical_path() {
            Ok(cp) => {
                let levelled = scheduler
                    .resource_leveling()
                    .map(|l| l.makespan_hours)
                    .unwrap_or(0.0);
                format!(
                    "<p>Critical path: {} ({cp} activities, levelled makespan {lm:.0} h)</p>",
                    cp.iter()
                        .map(|a| format!("A{}", a.0))
                        .collect::<Vec<_>>()
                        .join(" &rarr; "),
                    cp = cp.len(),
                    lm = levelled
                )
            }
            Err(e) => format!("<p>Schedule: {e}</p>"),
        };
        let risk_section = match scheduler.monte_carlo_risk(0.25, 2_000, 42) {
            Ok(risk) => format!(
                "<table>
<tr><th>Samples</th><th>P50 makespan (h)</th><th>P90 (h)</th><th>Mean (h)</th></tr>
<tr><td>{}</td><td>{:.1}</td><td>{:.1}</td><td>{:.1}</td></tr>
</table>
<p>Most critical activities: {}</p>",
                risk.samples,
                risk.p50_makespan_h,
                risk.p90_makespan_h,
                risk.mean_makespan_h,
                risk.criticality_frequency
                    .iter()
                    .filter(|(_a, fr)| *fr > 0.0)
                    .take(5)
                    .map(|(a, fr)| format!("A{} ({:.0}%)", a.0, fr * 100.0))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Err(e) => format!("<p>Risk: {e}</p>"),
        };
        (sched_section, risk_section)
    };

    let mut rows = String::new();
    for (group, kg) in &report.by_group {
        rows.push_str(&format!(
            "<tr><td>{group}</td><td>{kg:.0}</td></tr>
"
        ));
    }
    let mut phase_rows = String::new();
    for phase in &twin.vessel.build_phases {
        phase_rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{:.0}</td><td>{}</td></tr>
",
            phase.id.0,
            phase.name,
            phase.weight_state.design_kg,
            phase.activities.len(),
        ));
    }
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<title>tpt-shipyard report — {name}</title>
<style>
 body {{ font-family: system-ui, sans-serif; margin: 2rem; color: #16222f; }}
 table {{ border-collapse: collapse; margin: 1rem 0; }}
 td, th {{ border: 1px solid #b8c4d0; padding: 4px 12px; }}
 th {{ background: #eef3f8; text-align: left; }}
 .pass {{ color: #157a3e; font-weight: 600; }}
 .fail {{ color: #b3261e; font-weight: 600; }}
 h1 {{ border-bottom: 2px solid #2c6bed; padding-bottom: .3rem; }}
</style></head><body>
<h1>Construction report — {name}</h1>
<p>{phases} build phases, {acts} activities · generated by tpt-yard-cli</p>
<h2>Weights</h2>
<table>
<tr><th>Design (kg)</th><th>Best estimate (kg)</th><th>Growth margin (kg)</th></tr>
<tr><td>{design:.0}</td><td>{est:.0}</td><td>{margin:.0}</td></tr>
</table>
<h2>By group</h2>
<table><tr><th>Group</th><th>kg</th></tr>
{rows}</table>
<h2>Phases</h2>
<table><tr><th>#</th><th>Name</th><th>Design kg</th><th>Activities</th></tr>
{phase_rows}</table>
<h2>Schedule</h2>
{sched_section}
<h2>Schedule risk (Monte Carlo, 2000 samples, &plusmn;25 % durations)</h2>
{risk_section}
<h2>Per-phase FEM</h2>
{fem_section}
<h2>Structural check (start state)</h2>
<p class="{cls}">{verdict}</p>
<p>Support reactions and full per-phase FEM: run the structural crate per phase
(roadmap — per-phase HTML sections).</p>
</body></html>
"#,
        name = twin.vessel.name,
        phases = twin.vessel.build_phases.len(),
        acts = twin
            .vessel
            .build_phases
            .iter()
            .map(|p| p.activities.len())
            .sum::<usize>(),
        design = report.design_weight_kg,
        est = report.total_weight_kg,
        margin = report.margin_kg,
        rows = rows,
        phase_rows = phase_rows,
        cls = if check.passed { "pass" } else { "fail" },
        verdict = if check.passed { "PASSED" } else { "FAILED" },
        sched_section = sched_section,
        risk_section = risk_section,
        fem_section = fem_section,
    );
    std::fs::write(out_path, html).map_err(|e| format!("writing {out_path}: {e}"))?;
    println!("HTML report written to {out_path}");
    Ok(())
}

// ---------------------------------------------------------------- pdf report

/// `pdf-report project.json [--out file.pdf]`: the same report package as
/// html-report (weights, by-group, phases, schedule, risk, structural
/// check) rendered as a dependency-free PDF document.
fn pdf_report(path: &str, out_path: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = tpt_yard_core::json::Value::parse(&text).map_err(|e| format!("{path}: {e}"))?;
    let project =
        tpt_yard_core::VesselProject::from_json_value(&v).map_err(|e| format!("{path}: {e}"))?;
    let twin = tpt_yard::tpt_yard_digital_twin::DigitalTwin::new(project);
    let report = twin.weight_model().weight_report();
    let check = twin.structural_check();

    let mut lines: Vec<pdf::Line> = Vec::new();
    fn heading(lines: &mut Vec<pdf::Line>, t: &str) {
        lines.push(("B", 16.0, t.to_string()));
    }
    fn sub(lines: &mut Vec<pdf::Line>, t: &str) {
        lines.push(("B", 12.0, t.to_string()));
    }
    fn body(lines: &mut Vec<pdf::Line>, t: &str) {
        lines.push(("H", 10.0, t.to_string()));
    }
    fn mono(lines: &mut Vec<pdf::Line>, t: &str) {
        lines.push(("C", 9.0, t.to_string()));
    }
    fn gap(lines: &mut Vec<pdf::Line>) {
        lines.push(("H", 8.0, String::new()));
    }

    heading(&mut lines, "Construction report");
    body(
        &mut lines,
        &format!(
            "{}: {} build phases, {} activities. Generated by tpt-yard-cli.",
            twin.vessel.name,
            twin.vessel.build_phases.len(),
            twin.vessel
                .build_phases
                .iter()
                .map(|p| p.activities.len())
                .sum::<usize>()
        ),
    );
    gap(&mut lines);
    sub(&mut lines, "Weights");
    mono(
        &mut lines,
        &format!("  design        {:>10.0} kg", report.design_weight_kg),
    );
    mono(
        &mut lines,
        &format!("  best estimate {:>10.0} kg", report.total_weight_kg),
    );
    mono(
        &mut lines,
        &format!("  growth margin {:>10.0} kg", report.margin_kg),
    );
    gap(&mut lines);
    sub(&mut lines, "By group");
    for (group, kg) in &report.by_group {
        mono(&mut lines, &format!("  {group:<24} {kg:>10.0} kg"));
    }
    gap(&mut lines);
    sub(&mut lines, "Phases");
    for phase in &twin.vessel.build_phases {
        mono(
            &mut lines,
            &format!(
                "  {:>2}  {:<30} {:>10.0} kg  {} activities",
                phase.id.0,
                phase.name,
                phase.weight_state.design_kg,
                phase.activities.len()
            ),
        );
    }
    gap(&mut lines);
    sub(&mut lines, "Schedule");
    let acts: Vec<tpt_yard_core::AssemblyActivity> = twin
        .vessel
        .build_phases
        .iter()
        .flat_map(|p| p.activities.iter().cloned())
        .collect();
    if acts.is_empty() {
        body(&mut lines, "  no activities to schedule");
    } else {
        let scheduler = tpt_yard::tpt_yard_scheduling::ShipyardScheduler::new(acts.clone());
        match scheduler.critical_path() {
            Ok(cp) => {
                let levelled = scheduler
                    .resource_leveling()
                    .map(|l| l.makespan_hours)
                    .unwrap_or(0.0);
                body(
                    &mut lines,
                    &format!(
                        "  critical path: {} ({} activities, levelled makespan {:.0} h)",
                        cp.iter()
                            .map(|a| format!("A{}", a.0))
                            .collect::<Vec<_>>()
                            .join(" -> "),
                        cp.len(),
                        levelled
                    ),
                );
            }
            Err(e) => body(&mut lines, &format!("  schedule: {e}")),
        }
        match scheduler.monte_carlo_risk(0.25, 2_000, 42) {
            Ok(risk) => {
                gap(&mut lines);
                sub(
                    &mut lines,
                    "Schedule risk (Monte Carlo, 2000 samples, +/-25% durations)",
                );
                mono(
                    &mut lines,
                    &format!(
                        "  P50 {:.1} h   P90 {:.1} h   mean {:.1} h",
                        risk.p50_makespan_h, risk.p90_makespan_h, risk.mean_makespan_h
                    ),
                );
                let top: Vec<String> = risk
                    .criticality_frequency
                    .iter()
                    .filter(|(_a, fr)| *fr > 0.0)
                    .take(5)
                    .map(|(a, fr)| format!("A{} ({:.0}%)", a.0, fr * 100.0))
                    .collect();
                if !top.is_empty() {
                    body(&mut lines, &format!("  most critical: {}", top.join(", ")));
                }
            }
            Err(e) => body(&mut lines, &format!("  risk: {e}")),
        }
    }
    gap(&mut lines);
    sub(&mut lines, "Structural check (start state)");
    body(
        &mut lines,
        &format!("  {}", if check.passed { "PASSED" } else { "FAILED" }),
    );

    let pages = pdf::paginate(&lines);
    let doc = pdf::document(&pages);
    std::fs::write(out_path, doc).map_err(|e| format!("writing {out_path}: {e}"))?;
    println!("PDF report written to {out_path} ({} pages)", pages.len());
    Ok(())
}

// ---------------------------------------------------------------------- pdf

/// Minimal PDF 1.4 writer for the report package (review 7H roadmap:
/// "PDF output") — no dependencies: uncompressed content streams with
/// Helvetica headings and Courier body text on A4 pages.
pub mod pdf {
    /// One laid-out line: (font, size, text). Fonts: "H" = Helvetica,
    /// "B" = Helvetica-Bold, "C" = Courier.
    pub type Line = (&'static str, f64, String);

    /// A4 portrait: 595 x 842 pt.
    const PAGE_W: f64 = 595.0;
    const PAGE_H: f64 = 842.0;
    const MARGIN: f64 = 54.0;
    const LINE_H: f64 = 14.0;

    /// Lays out lines into pages, breaking when the cursor falls below
    /// the bottom margin. Empty lines advance the cursor only.
    pub fn paginate(lines: &[Line]) -> Vec<Vec<Line>> {
        let mut pages: Vec<Vec<Line>> = vec![Vec::new()];
        let mut y = PAGE_H - MARGIN;
        for line in lines {
            let (_, size, text) = line;
            if text.is_empty() {
                y -= size.max(LINE_H);
                continue;
            }
            if y < MARGIN + LINE_H * 2.0 {
                pages.push(Vec::new());
                y = PAGE_H - MARGIN;
            }
            y -= size.max(LINE_H);
            pages
                .last_mut()
                .expect("pages never empty")
                .push(line.clone());
        }
        pages
    }

    /// Escapes PDF string specials and maps common non-ASCII to ASCII
    /// (the content streams use the WinAnsi-safe plain-ASCII subset).
    pub fn escape(text: &str) -> String {
        let ascii: String = text
            .chars()
            .map(|c| match c {
                '—' | '–' | '→' | '⇒' => '-',
                '≤' => '<',
                '≥' => '>',
                '±' => '+',
                '°' => 'd',
                '³' => '3',
                '²' => '2',
                '×' => 'x',
                '·' => '.',
                '’' | '‘' => '\'',
                '“' | '”' => '"',
                c if c.is_ascii() => c,
                _ => ' ',
            })
            .collect();
        let mut out = String::with_capacity(ascii.len());
        for c in ascii.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '(' => out.push_str("\\("),
                ')' => out.push_str("\\)"),
                c => out.push(c),
            }
        }
        out
    }

    /// Emits the complete PDF document for the laid-out pages.
    pub fn document(pages: &[Vec<Line>]) -> String {
        // 1 catalog, 2 pages tree (placeholder), 3-5 fonts; page and
        // stream objects follow.
        let mut objects: Vec<String> = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".into(),
            "<< >>".into(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".into(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>".into(),
        ];

        let mut kids = Vec::new();
        for page in pages {
            let page_no = objects.len() + 1;
            let stream_no = page_no + 1;
            kids.push(format!("{page_no} 0 R"));
            let mut content = String::new();
            let mut y = PAGE_H - MARGIN;
            for (font, size, text) in page {
                y -= size.max(LINE_H);
                let f = match *font {
                    "B" => "F2",
                    "C" => "F3",
                    _ => "F1",
                };
                content.push_str(&format!(
                    "BT /{f} {size} Tf {MARGIN:.1} {y:.1} Td ({}) Tj ET\n",
                    escape(text)
                ));
            }
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> \
                 /Contents {stream_no} 0 R >>"
            ));
            objects.push(format!(
                "<< /Length {} >>\nstream\n{content}endstream",
                content.len()
            ));
        }
        objects[1] = format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages.len()
        );

        // Serialize with byte offsets for the xref table.
        let mut pdf = String::from("%PDF-1.4\n%\u{e2}\u{e3}\u{cf}\u{d3}\n");
        let mut offsets = Vec::with_capacity(objects.len() + 1);
        for (i, obj) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.push_str(&format!("{} 0 obj\n{obj}\nendobj\n", i + 1));
        }
        let xref_at = pdf.len();
        pdf.push_str(&format!("xref\n0 {}\n", objects.len() + 1));
        pdf.push_str("0000000000 65535 f \n");
        for off in &offsets {
            pdf.push_str(&format!("{off:010} 00000 n \n"));
        }
        pdf.push_str(&format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF",
            objects.len() + 1
        ));
        pdf
    }
}

// -------------------------------------------------------------------- plan

/// The end-to-end plan (7H): block division → erection order → lift checks
/// → schedule → critical path, one report.
fn plan(path: &str, json_mode: bool) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = Value::parse(&text).map_err(|e| format!("{path}: {e}"))?;
    let num = |o: &Value, k: &str| {
        o.get(k)
            .and_then(|n| n.as_f64())
            .ok_or(format!("manifest: '{k}' missing or not a number"))
    };

    let hull_v = v.get("hull").ok_or("manifest: 'hull' missing")?;
    let hull = tpt_yard::tpt_yard_hull::HullGeometry {
        loa_m: num(hull_v, "loa_m")?,
        boa_m: num(hull_v, "boa_m")?,
        depth_m: num(hull_v, "depth_m")?,
        areal_density_kg_m2: num(hull_v, "areal_density_kg_m2")?,
        depth_bands: hull_v
            .get("depth_bands")
            .and_then(|n| n.as_u64())
            .ok_or("manifest: 'depth_bands' missing")? as u32,
    };
    let yard = v
        .get("yard_capabilities")
        .ok_or("manifest: 'yard_capabilities' missing")?;
    let crane_kn = num(yard, "crane_capacity_kn")?;
    let ws = yard.get("workshop").ok_or("manifest: 'workshop' missing")?;
    let workshop = tpt_yard::tpt_yard_core::Dimensions::new(
        num(ws, "length_m")?,
        num(ws, "breadth_m")?,
        num(ws, "depth_m")?,
    );

    // 1. Block division (plus the explicit workshop cross-section check:
    // blocks are the full beam wide, so a narrow workshop is infeasible
    // even when the lengths divide cleanly).
    let construction = tpt_yard::tpt_yard_hull::HullConstruction::new(hull);
    if let Err(workshop_violation) = construction.check_workshop(workshop) {
        return Err(format!("workshop cross-section: {workshop_violation}"));
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
        v.get("vessel").and_then(|x| x.as_str()).unwrap_or(path)
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

// ---------------------------------------------------------------- schedule

/// `schedule project.json [--limit kind=value]...`
///
/// Critical path plus capacity-aware resource levelling. Each `--limit`
/// states the yard-wide concurrent availability of one resource kind, in
/// the same units as the activities' resource capacities (e.g.
/// `--limit crane=200` for 200 t of crane, `--limit crew=40` for 40
/// welders); kinds without a limit are unconstrained.
fn schedule(rest: &[&str], json_mode: bool) -> Result<(), String> {
    let path: &str = rest
        .first()
        .copied()
        .filter(|a| !a.starts_with("--"))
        .ok_or("schedule needs a project file path")?;
    let rest: Vec<&str> = rest
        .iter()
        .skip(1)
        .copied()
        .filter(|a| *a != path)
        .collect();
    let mut limits: BTreeMap<String, f64> = BTreeMap::new();
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            "--limit" => {
                i += 1;
                let spec = rest.get(i).ok_or("--limit needs kind=value")?;
                let (kind, val) = spec
                    .split_once('=')
                    .ok_or_else(|| format!("--limit {spec}: expected kind=value"))?;
                let v: f64 = val.parse().map_err(|e| format!("--limit {kind}: {e}"))?;
                limits.insert(kind.to_ascii_lowercase(), v);
            }
            other => return Err(format!("unknown schedule flag '{other}'")),
        }
        i += 1;
    }

    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = Value::parse(&text).map_err(|e| format!("{path}: {e}"))?;
    let project =
        tpt_yard_core::VesselProject::from_json_value(&v).map_err(|e| format!("{path}: {e}"))?;
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
    for (name, v) in &limits {
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
fn risk(rest: &[&str], json_mode: bool) -> Result<(), String> {
    let path: &str = rest
        .first()
        .copied()
        .filter(|a| !a.starts_with("--"))
        .ok_or("risk needs a project file path")?;
    let rest: Vec<&str> = rest
        .iter()
        .skip(1)
        .copied()
        .filter(|a| *a != path)
        .collect();
    let mut samples = 2_000_u32;
    let mut uncertainty = 0.25_f64;
    let mut gates: Vec<(u64, f64, f64)> = Vec::new();
    let mut weather: Option<&str> = None;
    let mut launch_method = "drydock";
    let mut max_sea_state = 3_u32;
    let weather_gate_activity: Option<u64> = None;
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            "--samples" => {
                i += 1;
                samples = rest
                    .get(i)
                    .and_then(|v| v.parse().ok())
                    .ok_or("--samples needs a number")?;
            }
            "--uncertainty" => {
                i += 1;
                uncertainty = rest
                    .get(i)
                    .and_then(|v| v.parse().ok())
                    .ok_or("--uncertainty needs a fraction")?;
            }
            "--weather" => {
                i += 1;
                weather = Some(
                    rest.get(i)
                        .copied()
                        .ok_or("--weather needs a forecast path")?,
                );
            }
            "--launch-method" => {
                i += 1;
                launch_method = rest
                    .get(i)
                    .copied()
                    .ok_or("--launch-method needs slipway|drydock|side|shiplift")?;
            }
            "--max-sea-state" => {
                i += 1;
                max_sea_state = rest
                    .get(i)
                    .and_then(|v| v.parse().ok())
                    .ok_or("--max-sea-state needs a Douglas number (0-9)")?;
            }
            "--gate" => {
                i += 1;
                let spec = rest.get(i).ok_or("--gate needs id=hours[:slippage]")?;
                let (id_part, rest_part) = spec
                    .split_once('=')
                    .ok_or_else(|| format!("--gate {spec}: expected id=hours[:slippage]"))?;
                let id: u64 = id_part.parse().map_err(|e| format!("--gate id: {e}"))?;
                let (hours_str, slip_str) = match rest_part.split_once(':') {
                    Some((h, sl)) => (h, Some(sl)),
                    None => (rest_part, None),
                };
                let hours: f64 = hours_str
                    .parse()
                    .map_err(|e| format!("--gate hours: {e}"))?;
                let slip: f64 = match slip_str {
                    Some(sl) => sl.parse().map_err(|e| format!("--gate slip: {e}"))?,
                    None => 0.0,
                };
                gates.push((id, hours, slip));
            }
            other => return Err(format!("unknown risk flag '{other}'")),
        }
        i += 1;
    }

    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = Value::parse(&text).map_err(|e| format!("{path}: {e}"))?;
    let project =
        tpt_yard_core::VesselProject::from_json_value(&v).map_err(|e| format!("{path}: {e}"))?;
    let acts: Vec<tpt_yard_core::AssemblyActivity> = project
        .build_phases
        .iter()
        .flat_map(|p| p.activities.iter().cloned())
        .collect();
    if acts.is_empty() {
        return Err("project has no activities to analyse".into());
    }
    let scheduler = tpt_yard::tpt_yard_scheduling::ShipyardScheduler::new(acts.clone());
    let mut gate_objs: Vec<tpt_yard::tpt_yard_scheduling::DeliveryGate> = gates
        .iter()
        .map(
            |(id, h, slip)| tpt_yard::tpt_yard_scheduling::DeliveryGate {
                activity: tpt_yard::tpt_yard_assembly::ActivityId(*id),
                expected_available_h: *h,
                slippage_frac: *slip,
            },
        )
        .collect();
    // Weather window (review 7H Monte Carlo leftover): a sea-state
    // forecast gates the launch activity — it cannot start before the
    // first calm run of hours at or below the sea-state limit.
    let mut weather_note = String::new();
    if let Some(fpath) = weather {
        // The gated activity: an explicit --gate id if present, else the
        // last activity of the plan (typically the launch).
        let activity = weather_gate_activity
            .or_else(|| gates.last().map(|(id, _, _)| *id))
            .unwrap_or_else(|| acts.last().map(|a| a.id.0).unwrap_or(0));
        let ftext = std::fs::read_to_string(fpath).map_err(|e| format!("reading {fpath}: {e}"))?;
        let fv = tpt_yard_core::json::Value::parse(&ftext).map_err(|e| format!("{fpath}: {e}"))?;
        let forecast = tpt_yard::tpt_yard_earth_link::SeaStateForecast::from_json_value(&fv)
            .map_err(|e| format!("{fpath}: {e}"))?;
        let analysis = tpt_yard::tpt_yard_launch::LaunchAnalysis {
            launch_method: match launch_method {
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
                mass_kg: 4_000_000.0,
                cog: tpt_yard_core::Vector3::new(0.0, 0.0, 6.0),
            },
            way_length_m: 120.0,
            way_width_m: 2.0,
            friction_coefficient: 0.02,
            poppet_to_cog_m: 70.0,
            end_bearing_m: 20.0,
            immersion_length_m: 90.0,
            block_coefficient: 0.8,
            breadth_m: 20.0,
            site: tpt_yard::tpt_yard_launch::SiteConditions { max_sea_state },
        };
        let window = tpt_yard::tpt_yard_earth_link::plan_launch_window(&analysis, &forecast)
            .map_err(|e| format!("weather: {e}"))?;
        weather_note = format!(
            " (weather gate: safe from hour {} for {} h at sea state <= {})",
            window.earliest_hour, window.calm_hours, window.sea_state_limit
        );
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
        .monte_carlo_risk_with_gates(&gate_objs, uncertainty, samples, 42)
        .map_err(|e| format!("{e}"))?;

    if json_mode {
        println!(
            "{{\"samples\":{},\"p50_makespan_h\":{:.1},\"p90_makespan_h\":{:.1},\"mean_makespan_h\":{:.1},\"gates\":{}}}",
            risk.samples,
            risk.p50_makespan_h,
            risk.p90_makespan_h,
            risk.mean_makespan_h,
            gates.len()
        );
        if !weather_note.is_empty() {
            eprintln!("weather{weather_note}");
        }
        return Ok(());
    }
    println!(
        "Schedule risk ({} samples, duration uncertainty {:.0}%):",
        risk.samples,
        uncertainty * 100.0
    );
    println!(
        "  makespan P50 {:.1} h | P90 {:.1} h | mean {:.1} h",
        risk.p50_makespan_h, risk.p90_makespan_h, risk.mean_makespan_h
    );
    if !weather_note.is_empty() {
        println!("  weather{weather_note}");
    }
    if !gates.is_empty() {
        println!(
            "  delivery gates: {}",
            gates
                .iter()
                .map(|(id, h, slip)| format!("A{id} @ {h:.0} h (+/-{:.0}%)", slip * 100.0))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let top: Vec<(u64, f64)> = risk
        .criticality_frequency
        .iter()
        .filter(|(_a, f)| *f > 0.0)
        .take(5)
        .map(|(a, f)| (a.0, *f))
        .collect();
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

// ------------------------------------------------------------------ report

fn report(path: &str, json_mode: bool) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = Value::parse(&text).map_err(|e| format!("{path}: {e}"))?;
    let project =
        tpt_yard_core::VesselProject::from_json_value(&v).map_err(|e| format!("{path}: {e}"))?;
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

fn new(rest: &[&str], json_mode: bool) -> Result<(), String> {
    let kind = rest.first().copied().ok_or(
        "new needs a kind: sea|space, or a template: container-ship|submarine|orbital-truss|habitat|solar-array",
    )?;
    // Named project templates (review 7F).
    if matches!(
        kind,
        "container-ship" | "submarine" | "orbital-truss" | "habitat" | "solar-array"
    ) {
        return template(kind, rest.get(1).copied(), json_mode);
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
                rest.get(1).copied().unwrap_or("New sea vessel").to_string(),
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
                rest.get(1)
                    .copied()
                    .unwrap_or("New space vessel")
                    .to_string(),
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
    if json_mode {
        println!("{json}");
    } else {
        println!(
            "Scaffolded project (save as project.json and run `tpt-yard validate project.json`):"
        );
        println!("{json}");
    }
    Ok(())
}

/// Scaffold one of the named reference templates. Each template mirrors the
/// matching reference case in `test-data/` so the numbers are realistic.
fn template(kind: &str, name: Option<&str>, json_mode: bool) -> Result<(), String> {
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
    println!("{}", project.to_json().to_string_pretty());
    if !json_mode {
        eprintln!("(save as project.json, then `tpt-yard validate project.json`)");
    }
    Ok(())
}

fn validate(path: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = Value::parse(&text).map_err(|e| format!("{path}: {e}"))?;
    if v.get("build_phases").is_some() {
        let project = tpt_yard_core::VesselProject::from_json_value(&v)
            .map_err(|e| format!("{path}: {e}"))?;
        let acts: usize = project
            .build_phases
            .iter()
            .map(|p| p.activities.len())
            .sum();
        println!(
            "OK: project '{}' ({} phases, {acts} activities) passes validation",
            project.name,
            project.build_phases.len()
        );
    } else {
        // Hull manifest: validate the shape the planner consumes.
        let num = |o: &Value, k: &str| {
            o.get(k)
                .and_then(|n| n.as_f64())
                .ok_or(format!("manifest: '{k}' missing or not a number"))
        };
        let hull = v.get("hull").ok_or("manifest: 'hull' section missing")?;
        for k in [
            "loa_m",
            "boa_m",
            "depth_m",
            "areal_density_kg_m2",
            "depth_bands",
        ] {
            num(hull, k)?;
        }
        let yard = v
            .get("yard_capabilities")
            .ok_or("manifest: 'yard_capabilities' section missing")?;
        num(yard, "crane_capacity_kn")?;
        println!("OK: hull manifest '{path}' is well-formed");
    }
    Ok(())
}

#[cfg(test)]
mod pdf_tests {
    use super::pdf;

    /// Paginate breaks at the bottom margin: enough tall lines produce
    /// multiple pages in order.
    #[test]
    fn paginate_breaks_pages() {
        let lines: Vec<pdf::Line> = (0..60).map(|i| ("H", 12.0, format!("line {i}"))).collect();
        let pages = pdf::paginate(&lines);
        assert!(pages.len() >= 2, "60 x 12pt lines must span pages");
        let total: usize = pages.iter().map(|p| p.len()).sum();
        assert_eq!(total, 60, "no line lost or duplicated");
        // Order preserved across the break.
        assert_eq!(pages[0][0].2, "line 0");
        assert_eq!(
            pages[1][0].2,
            pages[0]
                .last()
                .unwrap()
                .2
                .rsplit_once(' ')
                .map(|(_, n)| format!("line {}", n.parse::<usize>().unwrap() + 1))
                .unwrap_or_default()
        );
    }

    /// Escape maps non-ASCII and PDF specials.
    #[test]
    fn escape_handles_specials() {
        assert_eq!(pdf::escape(r"a(b)c\d"), r"a\(b\)c\\d");
        assert_eq!(pdf::escape("±≤≥"), "+<>");
        assert_eq!(pdf::escape("—x"), "-x");
    }

    /// The emitted document is structurally valid: header, catalog,
    /// page objects, xref offsets that actually point at "xref", EOF.
    #[test]
    fn document_structure_is_valid() {
        let pages = vec![
            vec![
                ("B", 16.0, "Construction report".to_string()),
                ("C", 9.0, "  design        1,000 kg".to_string()),
            ],
            vec![("H", 10.0, "Page two".to_string())],
        ];
        let doc = pdf::document(&pages);
        assert!(doc.starts_with(
            "%PDF-1.4
"
        ));
        assert!(doc.contains("/Type /Catalog"));
        assert_eq!(doc.matches("/Type /Page ").count(), 2);
        assert!(doc.contains("startxref"));
        assert!(doc.ends_with("%%EOF"));
        // The xref offset points at the xref keyword.
        let start = doc.find("startxref").unwrap() + "startxref".len();
        let off: usize = doc[start..]
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(&doc[off..off + 4], "xref");
        // Content streams carry the escaped text.
        assert!(doc.contains("(Construction report) Tj"));
    }
}
// ---------------------------------------------------------------- validate
