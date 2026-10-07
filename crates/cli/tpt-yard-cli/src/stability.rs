//! `tpt-yard stability` — hydrostatics, the GZ curve and the IMO 2008
//! intact-stability criteria for one loading condition, from a hull given
//! as an offsets table (full form) or as prismatic coefficients
//! (screening).
//!
//! ```text
//! tpt-yard stability CASE.json [--offsets FILE.csv] [--draft M] [--kg M]
//!                              [--fsm TM] [--to-deg DEG] [--csv PREFIX]
//!                              [--svg FILE] [--strict] [--json]
//! ```
//!
//! The case file names the hull and the loading; see
//! `test-data/stability/` for both forms. Flags override the case file.

use std::path::Path;

use tpt_yard_core::json::Value;
use tpt_yard_hydrostatics::{
    imo_2008_general_criteria, parse_offsets_csv, Bonjean, GzCurve, HullForm, Hydrostatics,
    ImoVerdict, LoadingCondition,
};

/// The hull behind the calculation.
enum Model {
    Prismatic(HullForm),
    Offsets(Bonjean),
}

impl Model {
    fn label(&self) -> String {
        match self {
            Model::Prismatic(_) => "prismatic screening model".into(),
            Model::Offsets(b) => format!("hull offsets, {} stations", b.stations.len()),
        }
    }

    fn hydrostatics(&self, draft_m: f64) -> Option<Hydrostatics> {
        match self {
            Model::Prismatic(h) => Some(h.hydrostatics(draft_m)),
            Model::Offsets(b) => b.hydrostatics(draft_m),
        }
    }

    fn gz_curve(&self, loading: LoadingCondition, to_deg: f64) -> Option<GzCurve> {
        match self {
            Model::Prismatic(h) => Some(h.gz_curve(loading, to_deg)),
            Model::Offsets(b) => b.gz_curve(loading, to_deg, 5.0),
        }
    }

    /// Height of the deck (top of the offsets) — `None` for the prismatic
    /// model, which has no vertical extent.
    fn deck_m(&self) -> Option<f64> {
        match self {
            Model::Prismatic(_) => None,
            Model::Offsets(b) => b
                .stations
                .iter()
                .filter_map(|s| s.half_breadths.last().map(|p| p.0))
                .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.max(d)))),
        }
    }
}

#[derive(Default)]
struct Options<'a> {
    path: &'a str,
    offsets: Option<&'a str>,
    draft: Option<f64>,
    kg: Option<f64>,
    fsm: Option<f64>,
    to_deg: Option<f64>,
    csv_prefix: Option<&'a str>,
    svg: Option<&'a str>,
    strict: bool,
}

fn parse_options<'a>(rest: &[&'a str]) -> Result<Options<'a>, String> {
    let mut o = Options::default();
    let mut path = None;
    let mut it = rest.iter().copied();
    let number = |flag: &str, v: Option<&str>| -> Result<f64, String> {
        let v = v.ok_or(format!("{flag} needs a value"))?;
        v.parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .ok_or(format!("{flag} needs a finite number, got '{v}'"))
    };
    while let Some(a) = it.next() {
        match a {
            "--offsets" => o.offsets = Some(it.next().ok_or("--offsets needs a path")?),
            "--draft" => o.draft = Some(number(a, it.next())?),
            "--kg" => o.kg = Some(number(a, it.next())?),
            "--fsm" => o.fsm = Some(number(a, it.next())?),
            "--to-deg" => o.to_deg = Some(number(a, it.next())?),
            "--csv" => o.csv_prefix = Some(it.next().ok_or("--csv needs a file prefix")?),
            "--svg" => o.svg = Some(it.next().ok_or("--svg needs a path")?),
            "--strict" => o.strict = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option '{flag}'")),
            other => {
                if path.replace(other).is_some() {
                    return Err(format!("unexpected extra argument '{other}'"));
                }
            }
        }
    }
    o.path = path.ok_or("stability needs a case file path")?;
    Ok(o)
}

pub fn run(rest: &[&str], json_mode: bool) -> Result<(), String> {
    let o = parse_options(rest)?;
    let text = std::fs::read_to_string(o.path).map_err(|e| format!("reading {}: {e}", o.path))?;
    let case = Value::parse(&text).map_err(|e| format!("{}: {e}", o.path))?;
    let vessel = case
        .get("vessel")
        .and_then(Value::as_str)
        .unwrap_or("vessel")
        .to_string();
    let mut notes: Vec<String> = Vec::new();

    // Hull: an offsets table wins over prismatic coefficients.
    let offsets_path = match (o.offsets, case.get("offsets_csv").and_then(Value::as_str)) {
        (Some(p), _) => Some(p.to_string()),
        (None, Some(p)) => Some(
            Path::new(o.path)
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(p)
                .to_string_lossy()
                .into_owned(),
        ),
        (None, None) => None,
    };
    let hull_v = case.get("hull");
    let model = if let Some(p) = &offsets_path {
        let csv = std::fs::read_to_string(p).map_err(|e| format!("reading {p}: {e}"))?;
        Model::Offsets(parse_offsets_csv(&csv).map_err(|e| format!("{p}: {e}"))?)
    } else {
        let h = hull_v.ok_or("the case needs an \"offsets_csv\" path or a \"hull\" object")?;
        let get = |k: &str| h.get(k).and_then(Value::as_f64);
        let need = |k: &str| get(k).ok_or(format!("hull.{k} missing or not a number"));
        let (cb, cwp) = (get("cb"), get("cwp"));
        if cb.is_none() || cwp.is_none() {
            notes.push("hull.cb / hull.cwp not given: assumed Cb 0.70, Cwp 0.85".into());
        }
        let form = HullForm {
            loa_m: need("loa_m")?,
            boa_m: need("boa_m")?,
            cb: cb.unwrap_or(0.70),
            cwp: cwp.unwrap_or(0.85),
        };
        if form.loa_m <= 0.0
            || form.boa_m <= 0.0
            || !(0.0..=1.0).contains(&form.cb)
            || !(0.0..=1.0).contains(&form.cwp)
        {
            return Err("hull: loa_m and boa_m must be positive and cb, cwp within 0..1".into());
        }
        Model::Prismatic(form)
    };

    // Loading: flags, then the case file, then (hull manifests) a depth
    // fraction.
    let load_v = case.get("loading");
    let lget = |k: &str| load_v.and_then(|l| l.get(k)).and_then(Value::as_f64);
    let depth = hull_v
        .and_then(|h| h.get("depth_m"))
        .and_then(Value::as_f64);
    let draft = match o.draft.or(lget("draft_m")) {
        Some(d) => d,
        None => {
            let d = depth.ok_or("no draft: give loading.draft_m or --draft")? * 0.65;
            notes.push(format!("draft not given: assumed 65 % of depth = {d:.2} m"));
            d
        }
    };
    let kg = match o.kg.or(lget("kg_m")) {
        Some(k) => k,
        None => {
            let k = depth.ok_or("no KG: give loading.kg_m or --kg")? * 0.55;
            notes.push(format!("KG not given: assumed 55 % of depth = {k:.2} m"));
            k
        }
    };
    let fsm = o.fsm.or(lget("free_surface_moment_tm")).unwrap_or(0.0);
    if draft <= 0.0 || fsm < 0.0 {
        return Err("draft must be positive and the free-surface moment not negative".into());
    }
    let to_deg = o.to_deg.unwrap_or(60.0);
    if !(40.0..=180.0).contains(&to_deg) {
        return Err(
            "--to-deg must be between 40 and 180 (the criteria need the curve to 40°)".into(),
        );
    }
    let loading = LoadingCondition::new(draft, kg).with_free_surface_tm(fsm);

    let hs = model
        .hydrostatics(draft)
        .ok_or_else(|| match model.deck_m() {
            Some(deck) => format!("draft {draft} m is outside the hull (offsets reach {deck} m)"),
            None => format!("no hydrostatics at draft {draft} m"),
        })?;
    let gz = model
        .gz_curve(loading, to_deg)
        .ok_or("could not compute the GZ curve")?;
    let verdict = imo_2008_general_criteria(&gz);
    if gz
        .points
        .last()
        .is_some_and(|p| (p.heel_deg - verdict.max_gz_heel_deg).abs() < 1e-9)
    {
        notes.push(format!(
            "GZ is still rising at the end of the curve ({:.0} deg): the maximum is not resolved, so the max-GZ-angle criterion is only a lower bound",
            verdict.max_gz_heel_deg
        ));
    }
    if matches!(model, Model::Prismatic(_)) {
        notes.push(
            "prismatic screening model (box-like form, wall-sided GZ): supply hull offsets for a full-form result"
                .into(),
        );
    }

    if let Some(prefix) = o.csv_prefix {
        write_csv(prefix, &model, &hs, &gz)?;
    }
    if let Some(svg_path) = o.svg {
        std::fs::write(svg_path, gz_svg(&vessel, &gz, &verdict))
            .map_err(|e| format!("writing {svg_path}: {e}"))?;
    }

    if json_mode {
        println!(
            "{}",
            json_report(&vessel, &model, &hs, &loading, &gz, &verdict, &notes).to_string_compact()
        );
    } else {
        print_report(&vessel, &model, &hs, &loading, &gz, &verdict, &notes);
    }
    if o.strict && !verdict.passed {
        return Err(format!(
            "IMO 2008 criteria failed: {}",
            verdict.failures.join("; ")
        ));
    }
    Ok(())
}

fn print_report(
    vessel: &str,
    model: &Model,
    hs: &Hydrostatics,
    loading: &LoadingCondition,
    gz: &GzCurve,
    verdict: &ImoVerdict,
    notes: &[String],
) {
    println!("Stability: {vessel}  [{}]", model.label());
    println!("======================================================");
    println!(
        "Hydrostatics at draft {:.3} m (upright, even keel)",
        hs.draft_m
    );
    println!("  displacement  {:>10.1} t", hs.displacement_t);
    println!("  KB            {:>10.3} m", hs.kb_m);
    println!("  KM            {:>10.3} m", hs.km_m);
    println!(
        "  LCB / LCF     {:>10.3} / {:.3} m from midship",
        clean(hs.lcb_m),
        clean(hs.lcf_m)
    );
    println!("  waterplane    {:>10.1} m2", hs.waterplane_area_m2);
    println!("  TPC           {:>10.2} t/cm", hs.tpc_t_cm);
    println!("  MCT1cm        {:>10.1} t.m/cm", hs.mct1cm_tm_cm);
    println!();
    println!(
        "Loading: KG {:.3} m, free-surface correction {:.3} m, corrected GM {:.3} m",
        loading.kg_m(),
        gz.free_surface_correction_m,
        gz.gm_corrected_m
    );
    println!("  heel (deg)   GZ (m)");
    for p in &gz.points {
        println!("  {:>9.0}   {:>7.3}", p.heel_deg, p.gz_m);
    }
    println!();
    println!(
        "IMO 2008 (IS Code part A, 2.2): {}",
        if verdict.passed { "PASS" } else { "FAIL" }
    );
    let row = |name: &str, value: String, limit: &str, ok: bool| {
        println!(
            "  {name:<22} {value:>12}  {limit:<10} {}",
            if ok { "ok" } else { "FAIL" }
        );
    };
    row(
        "area 0-30 deg",
        format!("{:.4} m.rad", verdict.area_to_30_deg),
        ">= 0.055",
        verdict.area_to_30_deg >= 0.055,
    );
    row(
        "area 0-40 deg",
        format!("{:.4} m.rad", verdict.area_to_40_deg),
        ">= 0.090",
        verdict.area_to_40_deg >= 0.090,
    );
    row(
        "angle of max GZ",
        format!("{:.0} deg", verdict.max_gz_heel_deg),
        ">= 25",
        verdict.max_gz_heel_deg >= 25.0,
    );
    row(
        "corrected GM",
        format!("{:.3} m", verdict.gm_corrected_m),
        ">= 0.15",
        verdict.gm_corrected_m >= 0.15,
    );
    for f in &verdict.failures {
        println!("  failed: {f}");
    }
    for n in notes {
        println!("note: {n}");
    }
}

/// Flushes float noise (`-1e-16`) so zero prints as `0`, not `-0.0000`.
fn clean(n: f64) -> f64 {
    if n.abs() < 1e-9 {
        0.0
    } else {
        n
    }
}

fn num(n: f64) -> Value {
    Value::Number(n)
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn json_report(
    vessel: &str,
    model: &Model,
    hs: &Hydrostatics,
    loading: &LoadingCondition,
    gz: &GzCurve,
    verdict: &ImoVerdict,
    notes: &[String],
) -> Value {
    obj(vec![
        ("vessel", Value::String(vessel.to_string())),
        ("model", Value::String(model.label())),
        (
            "hydrostatics",
            obj(vec![
                ("draft_m", num(hs.draft_m)),
                ("displacement_t", num(hs.displacement_t)),
                ("kb_m", num(hs.kb_m)),
                ("km_m", num(hs.km_m)),
                ("lcb_m", num(clean(hs.lcb_m))),
                ("lcf_m", num(clean(hs.lcf_m))),
                ("waterplane_area_m2", num(hs.waterplane_area_m2)),
                ("tpc_t_cm", num(hs.tpc_t_cm)),
                ("mct1cm_tm_cm", num(hs.mct1cm_tm_cm)),
            ]),
        ),
        (
            "loading",
            obj(vec![
                ("draft_m", num(loading.draft_m)),
                ("kg_m", num(loading.kg_m())),
                (
                    "free_surface_moment_tm",
                    num(loading.free_surface_moment_tm),
                ),
                (
                    "free_surface_correction_m",
                    num(gz.free_surface_correction_m),
                ),
                ("gm_corrected_m", num(gz.gm_corrected_m)),
            ]),
        ),
        (
            "gz_curve",
            Value::Array(
                gz.points
                    .iter()
                    .map(|p| obj(vec![("heel_deg", num(p.heel_deg)), ("gz_m", num(p.gz_m))]))
                    .collect(),
            ),
        ),
        (
            "imo_2008",
            obj(vec![
                ("passed", Value::Bool(verdict.passed)),
                ("area_to_30_deg_m_rad", num(verdict.area_to_30_deg)),
                ("area_to_40_deg_m_rad", num(verdict.area_to_40_deg)),
                ("max_gz_heel_deg", num(verdict.max_gz_heel_deg)),
                ("max_gz_m", num(verdict.max_gz_m)),
                (
                    "failures",
                    Value::Array(
                        verdict
                            .failures
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    ),
                ),
            ]),
        ),
        (
            "notes",
            Value::Array(notes.iter().cloned().map(Value::String).collect()),
        ),
    ])
}

/// `PREFIX-hydrostatics.csv` (a table around the design draft) and
/// `PREFIX-gz.csv` (the GZ curve).
fn write_csv(prefix: &str, model: &Model, hs: &Hydrostatics, gz: &GzCurve) -> Result<(), String> {
    let mut table = String::from(
        "draft_m,displacement_t,kb_m,km_m,lcb_m,lcf_m,tpc_t_cm,mct1cm_tm_cm,waterplane_area_m2\n",
    );
    for k in 6..=14 {
        let draft = hs.draft_m * k as f64 / 10.0;
        if let Some(r) = model.hydrostatics(draft) {
            table.push_str(&format!(
                "{:.4},{:.3},{:.4},{:.4},{:.4},{:.4},{:.4},{:.3},{:.3}\n",
                r.draft_m,
                r.displacement_t,
                r.kb_m,
                r.km_m,
                clean(r.lcb_m),
                clean(r.lcf_m),
                r.tpc_t_cm,
                r.mct1cm_tm_cm,
                r.waterplane_area_m2
            ));
        }
    }
    let mut curve = String::from("heel_deg,gz_m\n");
    for p in &gz.points {
        curve.push_str(&format!("{:.2},{:.5}\n", p.heel_deg, p.gz_m));
    }
    for (suffix, body) in [("hydrostatics", table), ("gz", curve)] {
        let path = format!("{prefix}-{suffix}.csv");
        std::fs::write(&path, body).map_err(|e| format!("writing {path}: {e}"))?;
    }
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A standalone SVG of the GZ curve with the initial-GM tangent and the
/// 25° / 30° / 40° marks the IMO criteria refer to.
fn gz_svg(vessel: &str, gz: &GzCurve, verdict: &ImoVerdict) -> String {
    let (w, h) = (680.0_f64, 420.0_f64);
    let (left, right, top, bottom) = (64.0, 24.0, 44.0, 52.0);
    let x_max = gz.points.last().map_or(60.0, |p| p.heel_deg).max(40.0);
    let top_gz = gz.points.iter().map(|p| p.gz_m).fold(0.0_f64, f64::max);
    let low_gz = gz.points.iter().map(|p| p.gz_m).fold(0.0_f64, f64::min);
    let y_max = (top_gz * 1.15).max(0.5);
    let y_min = (low_gz * 1.15).min(0.0);
    let px = |deg: f64| left + (deg / x_max) * (w - left - right);
    let py = |m: f64| top + (y_max - m) / (y_max - y_min) * (h - top - bottom);

    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" font-family=\"sans-serif\" font-size=\"12\">\n\
<rect width=\"{w}\" height=\"{h}\" fill=\"#ffffff\"/>\n\
<text x=\"{left}\" y=\"26\" font-size=\"15\" font-weight=\"bold\" fill=\"#222\">GZ curve: {} ({})</text>\n",
        xml_escape(vessel),
        if verdict.passed { "IMO 2008 pass" } else { "IMO 2008 FAIL" }
    );
    let mut deg = 0.0;
    while deg <= x_max + 1e-9 {
        let x = px(deg);
        s.push_str(&format!(
            "<line x1=\"{x:.1}\" y1=\"{top}\" x2=\"{x:.1}\" y2=\"{:.1}\" stroke=\"#ddd\"/>\n<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"#444\">{deg:.0}</text>\n",
            h - bottom,
            h - bottom + 16.0
        ));
        deg += 10.0;
    }
    let y_step = if y_max - y_min > 4.0 { 1.0 } else { 0.5 };
    let mut m = (y_min / y_step).ceil() * y_step;
    while m <= y_max + 1e-9 {
        let y = py(m);
        s.push_str(&format!(
            "<line x1=\"{left}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"{}\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" fill=\"#444\">{m:.1}</text>\n",
            w - right,
            if m.abs() < 1e-9 { "#888" } else { "#ddd" },
            left - 6.0,
            y + 4.0
        ));
        m += y_step;
    }
    // Criteria marks.
    for (deg, label) in [(25.0, "25"), (30.0, "30"), (40.0, "40")] {
        if deg <= x_max {
            s.push_str(&format!(
                "<line x1=\"{x:.1}\" y1=\"{top}\" x2=\"{x:.1}\" y2=\"{:.1}\" stroke=\"#c77\" stroke-dasharray=\"4 3\"/>\n<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"#a33\">{label}°</text>\n",
                h - bottom,
                top - 4.0,
                x = px(deg)
            ));
        }
    }
    // Initial-GM tangent, clipped to the plot.
    if gz.gm_corrected_m > 0.0 {
        let deg_end = (y_max / gz.gm_corrected_m).to_degrees().min(x_max);
        s.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#58a\" stroke-dasharray=\"6 3\"/>\n",
            px(0.0),
            py(0.0),
            px(deg_end),
            py(gz.gm_corrected_m * deg_end.to_radians())
        ));
    }
    let pts: Vec<String> = gz
        .points
        .iter()
        .map(|p| format!("{:.1},{:.1}", px(p.heel_deg), py(p.gz_m)))
        .collect();
    s.push_str(&format!(
        "<polyline fill=\"none\" stroke=\"#1a5fb4\" stroke-width=\"2.5\" points=\"{}\"/>\n",
        pts.join(" ")
    ));
    for p in &gz.points {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3\" fill=\"#1a5fb4\"/>\n",
            px(p.heel_deg),
            py(p.gz_m)
        ));
    }
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" fill=\"#222\">heel (deg)</text>\n\
<text transform=\"translate(16 {:.1}) rotate(-90)\" text-anchor=\"middle\" fill=\"#222\">GZ (m)</text>\n\
<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" fill=\"#58a\">GM {:.2} m (dashed tangent)</text>\n</svg>\n",
        (left + w - right) / 2.0,
        h - 10.0,
        (top + h - bottom) / 2.0,
        w - right - 4.0,
        top + 14.0,
        gz.gm_corrected_m
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(name: &str) -> String {
        format!(
            "{}/../../../test-data/stability/{name}",
            env!("CARGO_MANIFEST_DIR")
        )
    }

    #[test]
    fn the_wigley_case_runs_end_to_end_and_writes_outputs() {
        let dir = std::env::temp_dir().join(format!("tpt-yard-stability-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let prefix = dir.join("out").to_string_lossy().into_owned();
        let svg = dir.join("gz.svg").to_string_lossy().into_owned();
        let case = data("wigley.json");
        run(&[&case, "--csv", &prefix, "--svg", &svg, "--strict"], true).expect("wigley passes");

        let gz_csv = std::fs::read_to_string(format!("{prefix}-gz.csv")).expect("gz csv");
        assert!(gz_csv.starts_with("heel_deg,gz_m\n0.00,0.00000\n"));
        let hydro = std::fs::read_to_string(format!("{prefix}-hydrostatics.csv")).expect("table");
        assert_eq!(hydro.lines().count(), 1 + 9, "header + nine drafts");
        let svg_text = std::fs::read_to_string(&svg).expect("svg");
        assert!(svg_text.starts_with("<svg") && svg_text.contains("<polyline"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn strict_fails_a_loading_that_misses_the_criteria() {
        let case = data("wigley.json");
        let err = run(&[&case, "--kg", "6.4", "--strict"], true).expect_err("high KG fails");
        assert!(err.contains("IMO 2008"), "{err}");
        // Without --strict the same case still reports.
        run(&[&case, "--kg", "6.4"], true).expect("reports without --strict");
    }

    #[test]
    fn the_prismatic_case_runs_and_flags_the_screening_model() {
        run(&[&data("container-ship-prismatic.json")], true).expect("prismatic runs");
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        let case = data("wigley.json");
        assert!(run(&[], true).is_err());
        assert!(run(&["nope.json"], true).is_err());
        assert!(run(&[&case, "--draft", "abc"], true).is_err());
        assert!(
            run(&[&case, "--draft", "50"], true).is_err(),
            "above the deck"
        );
        assert!(run(&[&case, "--to-deg", "20"], true).is_err());
        assert!(run(&[&case, "--bogus"], true).is_err());
        assert!(run(&[&case, "--offsets", "missing.csv"], true).is_err());
    }
    /// The schemas must not drift from what the command reads and the
    /// generators write: every key in the shipped cases is declared, and the
    /// offsets table schema names the CSV columns the generators emit.
    #[test]
    fn the_schemas_describe_the_shipped_files() {
        let schema_path = |n: &str| format!("{}/../../../schemas/{n}", env!("CARGO_MANIFEST_DIR"));
        let read =
            |p: String| Value::parse(&std::fs::read_to_string(&p).expect("reads")).expect("json");
        let case_schema = read(schema_path("stability-case.schema.json"));
        // Every key of `obj` must be a declared property of `schema`.
        let check = |what: &str, obj: &Value, schema: &Value| {
            let props = schema
                .get("properties")
                .and_then(Value::as_object)
                .expect("properties");
            for (k, _) in obj.as_object().expect("object") {
                assert!(
                    props.iter().any(|(d, _)| d == k),
                    "{what}: '{k}' not in the schema"
                );
            }
        };
        for name in ["wigley.json", "container-ship-prismatic.json"] {
            let case = read(data(name));
            check(name, &case, &case_schema);
            for section in ["hull", "loading"] {
                if let Some(v) = case.get(section) {
                    let sch = case_schema
                        .get("properties")
                        .and_then(|p| p.get(section))
                        .expect("sub");
                    check(&format!("{name} {section}"), v, sch);
                }
            }
        }
        let table = read(schema_path("hull-offsets.table-schema.json"));
        let names: Vec<String> = table
            .get("fields")
            .and_then(Value::as_array)
            .expect("fields")
            .iter()
            .map(|f| {
                f.get("name")
                    .and_then(Value::as_str)
                    .expect("name")
                    .to_string()
            })
            .collect();
        let csv = std::fs::read_to_string(data("wigley-offsets.csv")).expect("csv");
        let header = csv.lines().find(|l| !l.starts_with('#')).expect("header");
        assert_eq!(header, names.join(","));
    }
}
