//! `html-report` and `pdf-report`: the calculation package as HTML and PDF.

use crate::args::{self, html_escape, CliError, Parsed, CHECK_PREFIX};

/// The HTML calculation-package report (review 7H roadmap item: "report
/// generator — HTML/PDF calculation package per phase").
pub fn html_report(p: &Parsed, json_mode: bool) -> Result<(), CliError> {
    let out = p.value("out").unwrap_or("report.html");
    html_report_impl(
        p.pos(0).unwrap_or_default(),
        out,
        p.value("structure"),
        p.value("gltf-viewer"),
        p.has("force"),
    )
    .map_err(CliError::from)?;
    if json_mode {
        println!("{{\"written\":\"{}\"}}", args::json_escape(out));
    } else {
        println!("HTML report written to {out}");
    }
    Ok(())
}

fn html_report_impl(
    path: &str,
    out_path: &str,
    structure_path: Option<&str>,
    gltf_viewer: Option<&str>,
    force: bool,
) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = tpt_yard_core::json::Value::parse(&text)
        .map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
    let project = tpt_yard_core::VesselProject::from_json_value(&v)
        .map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
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
            if max_phase > 1_000 {
                return Err(format!(
                    "{spath}: an element is erected at phase {max_phase}; phases above 1000 are not supported"
                ));
            }
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

    // 3-D viewer section: an optional glTF (the export command's
    // output) inlined as vertex/index arrays with a three.js viewer
    // (the same CDN-with-fallback pattern as the WASM demo).
    let viewer_section = match gltf_viewer {
        Some(gpath) => {
            let gtext =
                std::fs::read_to_string(gpath).map_err(|e| format!("reading {gpath}: {e}"))?;
            let g = tpt_yard::export::geometry_from_gltf(&gtext)
                .map_err(|e| format!("{gpath}: {e}"))?;
            let verts: Vec<String> = g
                .vertices
                .iter()
                .map(|v| format!("[{:.3},{:.3},{:.3}]", v.x, v.y, v.z))
                .collect();
            let ids: Vec<String> = g
                .faces
                .iter()
                .flat_map(|f| [f[0], f[1], f[2]])
                .map(|i| i.to_string())
                .collect();
            let verts_per_row = 6;
            let verts_txt: Vec<String> = verts.chunks(verts_per_row).map(|c| c.join(",")).collect();
            let ids_txt: Vec<String> = ids.chunks(24).map(|c| c.join(",")).collect();
            let template = r##"<h2>Hull geometry (3-D)</h2>
<div id="viewer3d" style="width:100%;max-width:900px;aspect-ratio:16/9;position:relative;background:#0a101b;border-radius:6px;">
<p id="viewer-note" style="color:#8fa3b8;padding:1rem;">three.js could not be loaded (offline?) — geometry as data: @NV@ vertices, @NT@ triangles.</p>
</div>
<script type="importmap">{"imports":{"three":"https://cdn.jsdelivr.net/npm/three@0.170.0/build/three.module.js","three/addons/":"https://cdn.jsdelivr.net/npm/three@0.170.0/examples/jsm/"}}</script>
<script type="module">
const VERTS = [@VERTS@];
const IDS = [@IDS@];
try {
  const THREE = await import("three");
  const { OrbitControls } = await import("three/addons/controls/OrbitControls.js");
  const box = document.getElementById("viewer3d");
  box.querySelector("#viewer-note")?.remove();
  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.setSize(box.clientWidth, box.clientHeight);
  box.appendChild(renderer.domElement);
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x0a101b);
  const camera = new THREE.PerspectiveCamera(50, box.clientWidth / box.clientHeight, 0.1, 4000);
  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  scene.add(new THREE.HemisphereLight(0x9fc2ff, 0x1a2438, 1.4));
  const sun = new THREE.DirectionalLight(0xffffff, 1.6);
  sun.position.set(80, 160, 120);
  scene.add(sun);
  // The hull is z-up; the group maps it to three.js y-up.
  const group = new THREE.Group();
  group.rotation.x = -Math.PI / 2;
  scene.add(group);
  const geom = new THREE.BufferGeometry();
  geom.setAttribute("position", new THREE.Float32BufferAttribute(VERTS.flat(), 3));
  geom.setIndex(IDS);
  geom.computeVertexNormals();
  group.add(new THREE.Mesh(geom, new THREE.MeshStandardMaterial({ color: 0x3f7fd4, metalness: 0.15, roughness: 0.65 })));
  let span = 1;
  for (const v of VERTS) {
    span = Math.max(span, Math.abs(v[0]), Math.abs(v[1]), Math.abs(v[2]));
  }
  camera.position.set(span * 1.8, span * 1.2, span * 2.2);
  controls.target.set(0, 0, 0);
  window.addEventListener("resize", () => {
    renderer.setSize(box.clientWidth, box.clientHeight);
    camera.aspect = box.clientWidth / box.clientHeight;
    camera.updateProjectionMatrix();
  });
  renderer.setAnimationLoop(() => {
    controls.update();
    renderer.render(scene, camera);
  });
} catch (err) {
  const note = document.getElementById("viewer-note");
  if (note) note.textContent = "three.js could not be loaded (offline?): " + err;
}
</script>"##;
            template
                .replace("@NV@", &g.vertices.len().to_string())
                .replace("@NT@", &g.faces.len().to_string())
                .replace("@VERTS@", &verts_txt.join(",\n"))
                .replace("@IDS@", &ids_txt.join(","))
        }
        None => String::new(),
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
            "<tr><td>{}</td><td>{kg:.0}</td></tr>
",
            html_escape(group)
        ));
    }
    let mut phase_rows = String::new();
    for phase in &twin.vessel.build_phases {
        phase_rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{:.0}</td><td>{}</td></tr>
",
            phase.id.0,
            html_escape(&phase.name),
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
{viewer_section}
<h2>Structural check (start state)</h2>
<p class="{cls}">{verdict}</p>
<p>Support reactions and full per-phase FEM: run the structural crate per phase
(roadmap — per-phase HTML sections).</p>
</body></html>
"#,
        name = html_escape(&twin.vessel.name),
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
    args::write_output(out_path, html.as_bytes(), force)?;
    Ok(())
}

// ---------------------------------------------------------------- pdf report

/// `pdf-report project.json [--out file.pdf]`: the same report package as
/// html-report (weights, by-group, phases, schedule, risk, structural
/// check) rendered as a dependency-free PDF document.
pub fn pdf_report(p: &Parsed, json_mode: bool) -> Result<(), CliError> {
    let out = p.value("out").unwrap_or("report.pdf");
    pdf_report_impl(p.pos(0).unwrap_or_default(), out, p.has("force")).map_err(CliError::from)?;
    if json_mode {
        println!("{{\"written\":\"{}\"}}", args::json_escape(out));
    } else {
        println!("PDF report written to {out}");
    }
    Ok(())
}

fn pdf_report_impl(path: &str, out_path: &str, force: bool) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let v = tpt_yard_core::json::Value::parse(&text)
        .map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
    let project = tpt_yard_core::VesselProject::from_json_value(&v)
        .map_err(|e| format!("{CHECK_PREFIX}{path}: {e}"))?;
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
    args::write_output(out_path, doc.as_bytes(), force)?;
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
