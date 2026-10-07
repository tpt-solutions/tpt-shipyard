//! `tpt-yard new-hull` — writes a runnable starting point for the
//! `stability` command: an offsets CSV for a parametric hull plus the case
//! file that points at it.
//!
//! - `wigley` — the analytic reference hull (closed-form hydrostatics).
//! - `barge` — a box barge: rectangular sections, `V = L·B·T`.
//! - `workboat` — a hard-chine V-bottom boat with flared topsides and a
//!   transom, tapering to the bow.
//! - `tug` — a short, beamy harbour tug with a full round-bilge section.
//! - `sailboat` — a narrow, fine-ended monohull (ballast keel not modelled;
//!   KG is the whole-boat value).
//! - `ferry` — a long, shallow, full-bodied ro-pax hull with a high KG.

use std::path::Path;

#[derive(Clone, Copy)]
enum Kind {
    Wigley,
    Barge,
    Workboat,
    Tug,
    Sailboat,
    Ferry,
}

impl Kind {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "wigley" => Ok(Kind::Wigley),
            "barge" => Ok(Kind::Barge),
            "workboat" => Ok(Kind::Workboat),
            "tug" => Ok(Kind::Tug),
            "sailboat" => Ok(Kind::Sailboat),
            "ferry" => Ok(Kind::Ferry),
            other => Err(format!(
                "unknown hull '{other}' (try wigley, barge, workboat, tug, sailboat or ferry)"
            )),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Wigley => "Wigley hull",
            Kind::Barge => "Box barge",
            Kind::Workboat => "Hard-chine workboat",
            Kind::Tug => "Harbour tug",
            Kind::Sailboat => "Sailing monohull",
            Kind::Ferry => "Ro-pax ferry",
        }
    }

    /// Default `(loa, beam, draft, depth)`, m.
    fn defaults(self) -> (f64, f64, f64, f64) {
        match self {
            Kind::Wigley => (100.0, 10.0, 5.0, 7.5),
            Kind::Barge => (60.0, 14.0, 2.0, 4.5),
            Kind::Workboat => (24.0, 6.0, 1.2, 2.8),
            Kind::Tug => (30.0, 10.0, 4.0, 5.0),
            Kind::Sailboat => (12.0, 3.6, 0.6, 1.6),
            Kind::Ferry => (80.0, 18.0, 3.2, 8.0),
        }
    }

    /// Default KG as a fraction of the draft.
    fn kg_per_draft(self) -> f64 {
        match self {
            Kind::Wigley => 0.70,
            Kind::Barge => 0.80,
            Kind::Workboat => 1.25,
            Kind::Tug => 1.1,
            Kind::Sailboat => 0.6,
            Kind::Ferry => 2.0,
        }
    }

    /// Half-breadth at longitudinal position `x` (from midship) and height
    /// `z` above the baseline.
    fn half_breadth(self, x: f64, z: f64, loa: f64, beam: f64, draft: f64, depth: f64) -> f64 {
        let u = 2.0 * x / loa;
        match self {
            Kind::Wigley => {
                let shape = if z <= draft {
                    1.0 - ((draft - z) / draft).powi(2)
                } else {
                    1.0
                };
                0.5 * beam * (1.0 - u * u) * shape
            }
            Kind::Barge => 0.5 * beam,
            Kind::Tug => round_bilge(u, z, beam, draft, depth, 3.0, 3.0, 1.6, 0.10),
            Kind::Sailboat => round_bilge(u, z, beam, draft, depth, 1.8, 2.6, 1.2, 0.05),
            Kind::Ferry => round_bilge(u, z, beam, draft, depth, 3.5, 4.0, 2.5, 0.04),
            Kind::Workboat => {
                // Plan form: full width to 5 % of the length forward of
                // midship, then a fair taper to a point at the stem.
                let a = 0.05 * loa;
                let taper = if x <= a {
                    1.0
                } else {
                    let t = ((x - a) / (0.5 * loa - a)).clamp(0.0, 1.0);
                    1.0 - t.powf(1.7)
                };
                // Section: V bottom to the chine, flared topsides above.
                let chine = chine_height(draft);
                let half = 0.5 * beam * taper;
                if z <= chine {
                    half * z / chine
                } else {
                    half * (1.0 + 0.08 * (z - chine) / (depth - chine).max(1e-9))
                }
            }
        }
    }
}

/// A round-bilge hull: plan-form half-breadth falls as `1 - |u|^p` towards
/// the bow (`u > 0`, exponent `p_bow`) and the stern (`p_stern`), the
/// underwater section fills as `1 - ((T - z)/T)^q`, and the topsides flare
/// outward by `flare` of the half-beam between the waterline and the deck.
#[allow(clippy::too_many_arguments)]
fn round_bilge(
    u: f64,
    z: f64,
    beam: f64,
    draft: f64,
    depth: f64,
    p_bow: f64,
    p_stern: f64,
    q: f64,
    flare: f64,
) -> f64 {
    let p = if u >= 0.0 { p_bow } else { p_stern };
    let plan = (1.0 - u.abs().powf(p)).max(0.0);
    let section = if z <= draft {
        1.0 - ((draft - z) / draft).powf(q)
    } else {
        1.0 + flare * (z - draft) / (depth - draft).max(1e-9)
    };
    0.5 * beam * plan * section
}

fn chine_height(draft: f64) -> f64 {
    0.5 * draft
}

/// The baseline-to-deck levels the table is cut at: an even grid plus the
/// draft and the chine, so the waterline and the knuckle are exact.
fn levels(depth: f64, draft: f64, chine: Option<f64>) -> Vec<f64> {
    let mut zs: Vec<f64> = (0..=40).map(|k| depth * k as f64 / 40.0).collect();
    zs.push(draft);
    zs.extend(chine);
    zs.sort_by(f64::total_cmp);
    zs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    zs
}

/// Offsets as CSV text (`station_x_m,draft_m,half_breadth_m`).
fn offsets_csv(kind: Kind, loa: f64, beam: f64, draft: f64, depth: f64) -> String {
    let mut s = format!(
        "# {} — L {loa} m, B {beam} m, T {draft} m, depth {depth} m. Generated by `tpt-yard new-hull`.\n\
         station_x_m,draft_m,half_breadth_m\n",
        kind.label()
    );
    let chine = matches!(kind, Kind::Workboat).then(|| chine_height(draft));
    let zs = levels(depth, draft, chine);
    let n = 21;
    for i in 0..n {
        let x = -0.5 * loa + loa * i as f64 / (n - 1) as f64;
        for &z in &zs {
            let y = kind.half_breadth(x, z, loa, beam, draft, depth);
            s.push_str(&format!("{x:.4},{z:.4},{:.5}\n", y.max(0.0)));
        }
    }
    s
}

pub fn run(rest: &[&str], json_mode: bool) -> Result<(), String> {
    let mut it = rest.iter().copied();
    let mut kind: Option<Kind> = None;
    let (mut name, mut out_dir) = (None::<String>, ".".to_string());
    let (mut loa, mut beam, mut draft, mut depth, mut kg) = (None, None, None, None, None);
    let mut force = false;
    let value = |flag: &str, v: Option<&str>| -> Result<f64, String> {
        let v = v.ok_or(format!("{flag} needs a value"))?;
        v.parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && *n > 0.0)
            .ok_or(format!("{flag} needs a positive number, got '{v}'"))
    };
    while let Some(a) = it.next() {
        match a {
            "--loa" => loa = Some(value(a, it.next())?),
            "--beam" => beam = Some(value(a, it.next())?),
            "--draft" => draft = Some(value(a, it.next())?),
            "--depth" => depth = Some(value(a, it.next())?),
            "--kg" => kg = Some(value(a, it.next())?),
            "--name" => name = Some(it.next().ok_or("--name needs a value")?.to_string()),
            "--out" => out_dir = it.next().ok_or("--out needs a directory")?.to_string(),
            "--force" => force = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option '{flag}'")),
            other => {
                if kind.replace(Kind::parse(other)?).is_some() {
                    return Err(format!("unexpected extra argument '{other}'"));
                }
            }
        }
    }
    let kind =
        kind.ok_or("new-hull needs a hull: wigley, barge, workboat, tug, sailboat or ferry")?;
    let (d_loa, d_beam, d_draft, d_depth) = kind.defaults();
    let (loa, beam, draft) = (
        loa.unwrap_or(d_loa),
        beam.unwrap_or(d_beam),
        draft.unwrap_or(d_draft),
    );
    // A custom draft keeps the default freeboard ratio unless a depth is given.
    let depth = depth.unwrap_or(d_depth * draft / d_draft);
    if draft >= depth {
        return Err(format!(
            "draft {draft} m must be below the depth {depth} m (the deck)"
        ));
    }
    let kg = kg.unwrap_or(kind.kg_per_draft() * draft);
    let name = name.unwrap_or_else(|| {
        match kind {
            Kind::Wigley => "wigley",
            Kind::Barge => "barge",
            Kind::Workboat => "workboat",
            Kind::Tug => "tug",
            Kind::Sailboat => "sailboat",
            Kind::Ferry => "ferry",
        }
        .to_string()
    });
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("--name may only use letters, digits, '-' and '_'".into());
    }

    let dir = Path::new(&out_dir);
    let csv_name = format!("{name}-offsets.csv");
    let (csv_path, case_path) = (dir.join(&csv_name), dir.join(format!("{name}.json")));
    if !force {
        for p in [&csv_path, &case_path] {
            if p.exists() {
                return Err(format!("{} exists; use --force to overwrite", p.display()));
            }
        }
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    std::fs::write(&csv_path, offsets_csv(kind, loa, beam, draft, depth))
        .map_err(|e| format!("writing {}: {e}", csv_path.display()))?;
    let case = format!(
        "{{\n  \"vessel\": \"{} ({loa} x {beam} x {depth} m)\",\n  \"offsets_csv\": \"{csv_name}\",\n  \"loading\": {{ \"draft_m\": {draft}, \"kg_m\": {kg:.3}, \"free_surface_moment_tm\": 0.0 }}\n}}\n",
        kind.label()
    );
    std::fs::write(&case_path, case)
        .map_err(|e| format!("writing {}: {e}", case_path.display()))?;

    if json_mode {
        println!(
            "{{\"case\":{:?},\"offsets\":{:?}}}",
            case_path.display().to_string(),
            csv_path.display().to_string()
        );
    } else {
        println!("wrote {}", csv_path.display());
        println!("wrote {}", case_path.display());
        println!("next: tpt-yard stability {}", case_path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_hydrostatics::parse_offsets_csv;

    fn temp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tpt-yard-hulls-{tag}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir
    }

    #[test]
    fn the_box_barge_has_closed_form_hydrostatics() {
        let (l, b, t, d) = (60.0, 14.0, 2.0, 4.5);
        let hull = parse_offsets_csv(&offsets_csv(Kind::Barge, l, b, t, d)).expect("parses");
        let hs = hull.hydrostatics(t).expect("hydrostatics");
        let exact = |a: f64, e: f64| assert!((a - e).abs() < 1e-6 * e, "{a} vs {e}");
        exact(hs.displacement_t, 1.025 * l * b * t);
        exact(hs.kb_m, t / 2.0);
        exact(hs.km_m - hs.kb_m, b * b / (12.0 * t));
    }

    #[test]
    fn every_generated_hull_runs_through_the_stability_command() {
        for kind in ["wigley", "barge", "workboat", "tug", "sailboat", "ferry"] {
            let dir = temp(kind);
            let out = dir.to_string_lossy().into_owned();
            run(&[kind, "--out", &out], true).expect("generates");
            let case = dir
                .join(format!("{kind}.json"))
                .to_string_lossy()
                .into_owned();
            crate::stability::run(&[&case], true).unwrap_or_else(|e| panic!("{kind}: {e}"));
            // A second run refuses to overwrite unless forced.
            assert!(run(&[kind, "--out", &out], true).is_err());
            run(&[kind, "--out", &out, "--force"], true).expect("force overwrites");
            std::fs::remove_dir_all(&dir).ok();
        }
    }

    #[test]
    fn bad_arguments_are_errors() {
        assert!(run(&[], true).is_err());
        assert!(run(&["yacht"], true).is_err());
        assert!(run(&["barge", "--draft", "9", "--depth", "4"], true).is_err());
        assert!(run(&["barge", "--loa", "-5"], true).is_err());
        assert!(run(&["barge", "--name", "../evil"], true).is_err());
        assert!(run(&["barge", "wigley"], true).is_err());
    }
}
