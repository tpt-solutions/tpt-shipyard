//! `tpt-yard import-hull` — turns a hull surface mesh (Wavefront OBJ or
//! STL, ASCII or binary) into the offsets CSV and stability case the
//! `stability` command reads.
//!
//! The mesh is sliced by transverse planes at evenly spaced stations; at
//! each waterline height the half-breadth is the widest crossing of the
//! section there. That is exact for hulls whose sections do not re-enter
//! (no tumblehome pockets or tunnels) and for either a whole-hull mesh or
//! a half mesh on one side of the centreline.
//!
//! The mesh is taken as a *whole* hull (the centreline is the middle of its
//! beam, so a hull modelled over y = 0..B is centred first); `--half` says it
//! is one half with the centreline on y = 0. `--baseline Z` and `--depth M`
//! cut away keels, skegs, rudders and superstructure that would otherwise
//! set the baseline and depth.
//!
//! Axes: the hull length runs along the mesh X axis (bow towards `+x`
//! unless `--bow -x`), up is `--up z` (default) or `--up y`. The baseline is
//! the lowest point of the mesh and midship is the middle of its length.

use std::path::Path;

/// A triangle soup.
struct Mesh {
    tris: Vec<[[f64; 3]; 3]>,
}

fn finite_coords<'a>(it: impl Iterator<Item = &'a str>, line: usize) -> Result<[f64; 3], String> {
    let c: Vec<f64> = it
        .map(|t| t.parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect::<Option<_>>()
        .ok_or(format!("line {line}: bad vertex"))?;
    match c.as_slice() {
        [x, y, z, ..] => Ok([*x, *y, *z]),
        _ => Err(format!("line {line}: a vertex needs x y z")),
    }
}

fn parse_obj(text: &str) -> Result<Mesh, String> {
    let mut verts: Vec<[f64; 3]> = Vec::new();
    let mut tris = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => verts.push(finite_coords(it.take(3), n + 1)?),
            Some("f") => {
                let idx: Vec<[f64; 3]> = it
                    .map(|t| {
                        let raw = t.split('/').next().unwrap_or("");
                        let i: i64 = raw
                            .parse()
                            .map_err(|_| format!("line {}: bad face index '{t}'", n + 1))?;
                        let len = verts.len() as i64;
                        let k = if i > 0 { i - 1 } else { len + i };
                        usize::try_from(k)
                            .ok()
                            .and_then(|k| verts.get(k).copied())
                            .ok_or(format!("line {}: face index {i} out of range", n + 1))
                    })
                    .collect::<Result<_, _>>()?;
                if idx.len() < 3 {
                    return Err(format!("line {}: a face needs 3+ vertices", n + 1));
                }
                for k in 1..idx.len() - 1 {
                    tris.push([idx[0], idx[k], idx[k + 1]]);
                }
            }
            _ => {}
        }
    }
    Ok(Mesh { tris })
}

fn parse_stl(bytes: &[u8]) -> Result<Mesh, String> {
    let ascii_head = bytes.len() >= 5 && bytes[..5].eq_ignore_ascii_case(b"solid");
    if ascii_head && bytes.windows(5).any(|w| w == b"facet") {
        let text = String::from_utf8_lossy(bytes);
        let mut pts: Vec<[f64; 3]> = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let mut it = line.split_whitespace();
            if it.next() == Some("vertex") {
                pts.push(finite_coords(it, n + 1)?);
            }
        }
        if !pts.len().is_multiple_of(3) {
            return Err("ASCII STL vertex count is not a multiple of 3".into());
        }
        return Ok(Mesh {
            tris: pts.chunks(3).map(|c| [c[0], c[1], c[2]]).collect(),
        });
    }
    if bytes.len() < 84 {
        return Err("not an OBJ or STL file (too short for a binary STL)".into());
    }
    let n = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    if bytes.len() != 84 + 50 * n {
        return Err(format!(
            "binary STL declares {n} triangles but the file size does not match"
        ));
    }
    let f = |o: usize| {
        f64::from(f32::from_le_bytes([
            bytes[o],
            bytes[o + 1],
            bytes[o + 2],
            bytes[o + 3],
        ]))
    };
    let mut tris = Vec::with_capacity(n);
    for t in 0..n {
        let b = 84 + 50 * t + 12; // skip the normal
        let v = |k: usize| [f(b + 12 * k), f(b + 12 * k + 4), f(b + 12 * k + 8)];
        let tri = [v(0), v(1), v(2)];
        if tri.iter().flatten().any(|c| !c.is_finite()) {
            return Err(format!("triangle {t} has a non-finite coordinate"));
        }
        tris.push(tri);
    }
    Ok(Mesh { tris })
}

struct Options {
    up_y: bool,
    bow_neg: bool,
    scale: f64,
    /// `--scale` was given explicitly (confirms the unit).
    scale_given: bool,
    stations: usize,
    levels: usize,
    /// The mesh is one half of the hull, centreline on y = 0.
    half: bool,
    /// Height of the keel line in the (scaled) mesh vertical axis, m.
    baseline: Option<f64>,
    /// Deck height above the baseline, m.
    depth: Option<f64>,
}

/// The longest ship that is plausible in metres; beyond this the mesh is
/// almost certainly in millimetres.
const MAX_PLAUSIBLE_LOA_M: f64 = 600.0;

/// Slices the mesh and returns `(csv, loa, beam, depth)` in metres.
fn mesh_to_offsets(mesh: &Mesh, o: &Options) -> Result<(String, f64, f64, f64), String> {
    if mesh.tris.is_empty() {
        return Err("the mesh has no triangles".into());
    }
    // (long, trans, vert) in metres.
    let map = |p: [f64; 3]| {
        let (l, t, v) = if o.up_y {
            (p[0], p[2], p[1])
        } else {
            (p[0], p[1], p[2])
        };
        [
            if o.bow_neg { -l } else { l } * o.scale,
            t * o.scale,
            v * o.scale,
        ]
    };
    let tris: Vec<[[f64; 3]; 3]> = mesh
        .tris
        .iter()
        .map(|t| [map(t[0]), map(t[1]), map(t[2])])
        .collect();
    let (mut lmin, mut lmax, mut vmin, mut vmax) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    let (mut tmin, mut tmax) = (f64::MAX, f64::MIN);
    for p in tris.iter().flatten() {
        lmin = lmin.min(p[0]);
        lmax = lmax.max(p[0]);
        vmin = vmin.min(p[2]);
        vmax = vmax.max(p[2]);
        tmin = tmin.min(p[1]);
        tmax = tmax.max(p[1]);
    }
    let loa = lmax - lmin;
    if !o.scale_given && loa > MAX_PLAUSIBLE_LOA_M {
        return Err(format!(
            "the mesh is {loa:.0} m long: it looks like millimetres. Pass --scale 0.001 (or --scale 1 to confirm metres)"
        ));
    }
    // Keel line and deck: the lowest/highest mesh point unless given.
    let z0 = o.baseline.unwrap_or(vmin);
    let depth = o.depth.unwrap_or(vmax - z0);
    // Whole hull: centre on the middle of its beam; half hull: on y = 0.
    let (centre, beam) = if o.half {
        (0.0, 2.0 * tmin.abs().max(tmax.abs()))
    } else {
        ((tmin + tmax) / 2.0, tmax - tmin)
    };
    if loa <= 0.0 || depth.is_nan() || depth <= 0.0 || beam.is_nan() || beam <= 0.0 {
        return Err(
            "the mesh is flat along length, height or beam (or --baseline/--depth leave no hull) — check --up"
                .into(),
        );
    }
    let eps = 1e-6 * loa;
    let n_levels = o.levels;
    let dz = depth / (n_levels - 1) as f64;
    let mut csv = String::from("station_x_m,draft_m,half_breadth_m\n");
    for i in 0..o.stations {
        let f = i as f64 / (o.stations - 1) as f64;
        let x = lmin + eps + f * (loa - 2.0 * eps);
        // Widest crossing of the section at each waterline, found by
        // walking every section segment across the levels it spans.
        let mut half_breadth = vec![0.0_f64; n_levels];
        let mut hit = |zrel: f64, y: f64| {
            let k = (zrel / dz).round();
            if k >= 0.0 && (k as usize) < n_levels && (zrel - k * dz).abs() < 1e-9 * depth {
                let slot = &mut half_breadth[k as usize];
                *slot = slot.max((y - centre).abs());
            }
        };
        for t in &tris {
            let mut pts: Vec<[f64; 2]> = Vec::new();
            for e in 0..3 {
                let (a, b) = (t[e], t[(e + 1) % 3]);
                let (da, db) = (a[0] - x, b[0] - x);
                if da == 0.0 && db == 0.0 {
                    pts.push([a[1], a[2]]);
                    pts.push([b[1], b[2]]);
                } else if da * db <= 0.0 {
                    let u = da / (da - db);
                    pts.push([a[1] + u * (b[1] - a[1]), a[2] + u * (b[2] - a[2])]);
                }
            }
            for w in pts.windows(2) {
                let (a, b) = (w[0], w[1]);
                let (lo, hi) = (a[1].min(b[1]) - z0, a[1].max(b[1]) - z0);
                if hi < -1e-9 * depth || lo > depth * (1.0 + 1e-9) {
                    continue;
                }
                if hi - lo < 1e-12 {
                    hit(lo, a[0]);
                    hit(lo, b[0]);
                    continue;
                }
                let k0 = ((lo.max(0.0)) / dz).ceil() as usize;
                let k1 = ((hi.min(depth)) / dz).floor() as usize;
                for k in k0..=k1.min(n_levels - 1) {
                    let zrel = k as f64 * dz;
                    let u = ((zrel + z0 - a[1]) / (b[1] - a[1])).clamp(0.0, 1.0);
                    hit(zrel, a[0] + u * (b[0] - a[0]));
                }
            }
        }
        let xm = x - (lmin + lmax) / 2.0;
        for (k, y) in half_breadth.iter().enumerate() {
            csv.push_str(&format!("{xm:.4},{:.4},{y:.5}\n", k as f64 * dz));
        }
    }
    Ok((csv, loa, beam, depth))
}

pub fn run(rest: &[&str], json_mode: bool) -> Result<(), String> {
    let mut it = rest.iter().copied();
    let mut file: Option<&str> = None;
    let mut o = Options {
        up_y: false,
        bow_neg: false,
        scale: 1.0,
        scale_given: false,
        stations: 21,
        levels: 41,
        half: false,
        baseline: None,
        depth: None,
    };
    let (mut name, mut out_dir) = (None::<String>, ".".to_string());
    let (mut draft, mut kg) = (None::<f64>, None::<f64>);
    let mut force = false;
    let num = |flag: &str, v: Option<&str>| -> Result<f64, String> {
        let v = v.ok_or(format!("{flag} needs a value"))?;
        v.parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && *n > 0.0)
            .ok_or(format!("{flag} needs a positive number, got '{v}'"))
    };
    while let Some(a) = it.next() {
        match a {
            "--up" => match it.next() {
                Some("z") => o.up_y = false,
                Some("y") => o.up_y = true,
                other => {
                    return Err(format!(
                        "--up must be y or z, got '{}'",
                        other.unwrap_or("")
                    ))
                }
            },
            "--bow" => match it.next() {
                Some("+x") => o.bow_neg = false,
                Some("-x") => o.bow_neg = true,
                other => {
                    return Err(format!(
                        "--bow must be +x or -x, got '{}'",
                        other.unwrap_or("")
                    ))
                }
            },
            "--scale" => {
                o.scale = num(a, it.next())?;
                o.scale_given = true;
            }
            "--half" => o.half = true,
            "--baseline" => {
                let v = it.next().ok_or("--baseline needs a value")?;
                o.baseline = Some(
                    v.parse::<f64>()
                        .ok()
                        .filter(|n| n.is_finite())
                        .ok_or(format!("--baseline needs a number, got '{v}'"))?,
                );
            }
            "--depth" => o.depth = Some(num(a, it.next())?),
            "--stations" => {
                o.stations = num(a, it.next())? as usize;
                if !(3..=201).contains(&o.stations) {
                    return Err("--stations must be between 3 and 201".into());
                }
            }
            "--levels" => {
                o.levels = num(a, it.next())? as usize;
                if !(5..=401).contains(&o.levels) {
                    return Err("--levels must be between 5 and 401".into());
                }
            }
            "--draft" => draft = Some(num(a, it.next())?),
            "--kg" => kg = Some(num(a, it.next())?),
            "--name" => name = Some(it.next().ok_or("--name needs a value")?.to_string()),
            "--out" => out_dir = it.next().ok_or("--out needs a directory")?.to_string(),
            "--force" => force = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option '{flag}'")),
            other => {
                if file.replace(other).is_some() {
                    return Err(format!("unexpected extra argument '{other}'"));
                }
            }
        }
    }
    let file = file.ok_or("import-hull needs a mesh file (.obj or .stl)")?;
    let bytes = std::fs::read(file).map_err(|e| format!("reading {file}: {e}"))?;
    let ext = Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let mesh = match ext.as_deref() {
        Some("obj") => parse_obj(&String::from_utf8_lossy(&bytes))?,
        Some("stl") => parse_stl(&bytes)?,
        _ => return Err("the mesh file must end in .obj or .stl".into()),
    };
    let (csv, loa, beam, depth) = mesh_to_offsets(&mesh, &o)?;
    let hull = tpt_yard_hydrostatics::parse_offsets_csv(&csv)?;
    let draft = draft.unwrap_or(0.5 * depth);
    if draft >= depth {
        return Err(format!(
            "draft {draft} m must be below the hull depth {depth:.3} m"
        ));
    }
    let kg = kg.unwrap_or(0.6 * depth);
    let hs = hull
        .hydrostatics(draft)
        .ok_or("the hull has no displacement at that draft")?;
    if let Some(n) = &name {
        if n.is_empty()
            || !n
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("--name may only use letters, digits, '-' and '_'".into());
        }
    }
    let name = name.unwrap_or_else(|| {
        Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("hull")
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    });
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
    let header = format!(
        "# Imported from {} — L {loa:.3} m, B {beam:.3} m, depth {depth:.3} m. Generated by `tpt-yard import-hull`.\n",
        Path::new(file)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("mesh")
    );
    std::fs::write(&csv_path, format!("{header}{csv}"))
        .map_err(|e| format!("writing {}: {e}", csv_path.display()))?;
    let vessel = format!("{name} ({loa:.2} x {beam:.2} x {depth:.2} m)");
    let case = format!(
        "{{\n  \"vessel\": \"{}\",\n  \"offsets_csv\": \"{}\",\n  \"loading\": {{ \"draft_m\": {draft}, \"kg_m\": {kg:.3}, \"free_surface_moment_tm\": 0.0 }}\n}}\n",
        crate::args::json_escape(&vessel),
        crate::args::json_escape(&csv_name),
    );
    std::fs::write(&case_path, case)
        .map_err(|e| format!("writing {}: {e}", case_path.display()))?;
    if json_mode {
        println!(
            "{{\"case\":{:?},\"offsets\":{:?},\"loa_m\":{loa},\"beam_m\":{beam},\"depth_m\":{depth},\"displacement_t\":{}}}",
            case_path.display().to_string(),
            csv_path.display().to_string(),
            hs.displacement_t
        );
    } else {
        println!("hull: L {loa:.3} m, B {beam:.3} m, depth {depth:.3} m");
        println!(
            "at draft {draft:.3} m: displacement {:.1} t (KG defaulted to {kg:.3} m — set --draft/--kg for your loading)",
            hs.displacement_t
        );
        println!("wrote {}", csv_path.display());
        println!("wrote {}", case_path.display());
        println!("next: tpt-yard stability {}", case_path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A closed box L x B x H centred on x/y with its base at z = 0, as OBJ.
    fn box_obj(l: f64, b: f64, h: f64) -> String {
        let (x, y) = (l / 2.0, b / 2.0);
        let mut s = String::new();
        for z in [0.0, h] {
            for (vx, vy) in [(-x, -y), (x, -y), (x, y), (-x, y)] {
                s.push_str(&format!("v {vx} {vy} {z}\n"));
            }
        }
        for f in [
            "1 4 3 2", "5 6 7 8", "1 2 6 5", "2 3 7 6", "3 4 8 7", "4 1 5 8",
        ] {
            s.push_str(&format!("f {f}\n"));
        }
        s
    }

    fn opts() -> Options {
        Options {
            up_y: false,
            bow_neg: false,
            scale: 1.0,
            scale_given: false,
            stations: 21,
            levels: 41,
            half: false,
            baseline: None,
            depth: None,
        }
    }

    #[test]
    fn a_box_mesh_reproduces_the_barge_closed_form() {
        let (l, b, h, t) = (60.0, 14.0, 4.5, 2.0);
        let mesh = parse_obj(&box_obj(l, b, h)).expect("parses");
        assert_eq!(mesh.tris.len(), 12);
        let (csv, loa, beam, depth) = mesh_to_offsets(&mesh, &opts()).expect("slices");
        assert!((loa - l).abs() < 1e-9 && (beam - b).abs() < 1e-9 && (depth - h).abs() < 1e-9);
        let hs = tpt_yard_hydrostatics::parse_offsets_csv(&csv)
            .expect("valid offsets")
            .hydrostatics(t)
            .expect("hydrostatics");
        let exact = 1.025 * l * b * t;
        assert!(
            (hs.displacement_t - exact).abs() < 1e-3 * exact,
            "{}",
            hs.displacement_t
        );
        assert!((hs.kb_m - t / 2.0).abs() < 1e-3);
    }

    #[test]
    fn y_up_and_flipped_bow_give_the_same_hull() {
        // The same box with Y and Z swapped, and X negated.
        let swapped: String = box_obj(60.0, 14.0, 4.5)
            .lines()
            .map(|l| {
                let p: Vec<&str> = l.split_whitespace().collect();
                if p[0] == "v" {
                    let x: f64 = p[1].parse().unwrap();
                    format!("v {} {} {}\n", -x, p[3], p[2])
                } else {
                    format!("{l}\n")
                }
            })
            .collect();
        let mesh = parse_obj(&swapped).expect("parses");
        let o = Options {
            up_y: true,
            bow_neg: true,
            ..opts()
        };
        let (csv, loa, beam, depth) = mesh_to_offsets(&mesh, &o).expect("slices");
        assert!(
            (loa - 60.0).abs() < 1e-9 && (beam - 14.0).abs() < 1e-9 && (depth - 4.5).abs() < 1e-9
        );
        let hs = tpt_yard_hydrostatics::parse_offsets_csv(&csv)
            .unwrap()
            .hydrostatics(2.0)
            .unwrap();
        let exact = 1.025 * 60.0 * 14.0 * 2.0;
        assert!((hs.displacement_t - exact).abs() < 1e-3 * exact);
    }

    #[test]
    fn stl_ascii_and_binary_match_the_obj() {
        let obj = parse_obj(&box_obj(10.0, 4.0, 3.0)).unwrap();
        let mut ascii = String::from("solid box\n");
        let mut bin = vec![0u8; 80];
        bin.extend((obj.tris.len() as u32).to_le_bytes());
        for t in &obj.tris {
            ascii.push_str("facet normal 0 0 0\nouter loop\n");
            bin.extend([0u8; 12]);
            for v in t {
                ascii.push_str(&format!("vertex {} {} {}\n", v[0], v[1], v[2]));
                for c in v {
                    bin.extend((*c as f32).to_le_bytes());
                }
            }
            ascii.push_str("endloop\nendfacet\n");
            bin.extend([0u8; 2]);
        }
        ascii.push_str("endsolid box\n");
        let key = |m: &Mesh| mesh_to_offsets(m, &opts()).unwrap().0;
        let want = key(&obj);
        assert_eq!(key(&parse_stl(ascii.as_bytes()).unwrap()), want);
        assert_eq!(key(&parse_stl(&bin).unwrap()), want);
        bin.pop();
        assert!(parse_stl(&bin).is_err());
    }

    #[test]
    fn negative_indices_and_bad_input() {
        let tri = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n";
        assert_eq!(parse_obj(tri).unwrap().tris.len(), 1);
        assert!(parse_obj("v 0 0 0\nf 1 2 3\n").is_err());
        assert!(parse_obj("v 0 0\n").is_err());
        assert!(parse_obj("v 0 0 nan\n").is_err());
        assert!(mesh_to_offsets(&parse_obj("v 0 0 0\n").unwrap(), &opts()).is_err());
    }

    #[test]
    fn the_command_writes_a_case_the_stability_command_runs() {
        let dir = std::env::temp_dir().join(format!("tpt-yard-import-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let obj = dir.join("scow.obj");
        std::fs::write(&obj, box_obj(60.0, 14.0, 4.5)).unwrap();
        let o = obj.to_string_lossy().into_owned();
        let out = dir.join("out").to_string_lossy().into_owned();
        run(&[&o, "--out", &out, "--draft", "2", "--kg", "2.5"], true).expect("imports");
        let case = dir
            .join("out")
            .join("scow.json")
            .to_string_lossy()
            .into_owned();
        crate::stability::run(&[&case], true).expect("stability runs");
        assert!(
            run(&[&o, "--out", &out], true).is_err(),
            "no silent overwrite"
        );
        assert!(run(&["hull.txt"], true).is_err());
        assert!(run(&[&o, "--up", "w"], true).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
    /// Shifts every vertex of an OBJ by `dy` along y.
    fn shift_y(obj: &str, dy: f64) -> String {
        obj.lines()
            .map(|l| {
                let p: Vec<&str> = l.split_whitespace().collect();
                if p.first() == Some(&"v") {
                    let y: f64 = p[2].parse().unwrap();
                    format!("v {} {} {}\n", p[1], y + dy, p[3])
                } else {
                    format!("{l}\n")
                }
            })
            .collect()
    }

    /// 8B10: a hull modelled over y = 0..B is centred, not given twice its
    /// beam; `--half` reads a half mesh with the centreline on y = 0.
    #[test]
    fn offset_and_half_meshes_give_the_right_beam() {
        let full =
            mesh_to_offsets(&parse_obj(&box_obj(60.0, 14.0, 4.5)).unwrap(), &opts()).unwrap();
        let offset = parse_obj(&shift_y(&box_obj(60.0, 14.0, 4.5), 7.0)).unwrap();
        let moved = mesh_to_offsets(&offset, &opts()).unwrap();
        assert!((moved.2 - 14.0).abs() < 1e-9, "beam {}", moved.2);
        assert_eq!(moved.0, full.0, "same offsets once centred");
        // Half hull: y = 0..7 with the centreline on y = 0.
        let half_mesh = parse_obj(&shift_y(&box_obj(60.0, 7.0, 4.5), 3.5)).unwrap();
        let half = Options {
            half: true,
            ..opts()
        };
        let (csv, _, beam, _) = mesh_to_offsets(&half_mesh, &half).unwrap();
        assert!((beam - 2.0 * 7.0).abs() < 1e-9, "beam {beam}");
        let hs = tpt_yard_hydrostatics::parse_offsets_csv(&csv)
            .unwrap()
            .hydrostatics(2.0)
            .unwrap();
        // |y| max of the y = 0..7 box is 7: a 14 m beam box.
        assert!((hs.displacement_t - 1.025 * 60.0 * 14.0 * 2.0).abs() < 1e-3 * 1722.0);
    }

    /// 8B10: a millimetre mesh is refused unless the unit is confirmed.
    #[test]
    fn millimetre_meshes_need_an_explicit_scale() {
        let mm = parse_obj(&box_obj(60_000.0, 14_000.0, 4_500.0)).unwrap();
        let err = mesh_to_offsets(&mm, &opts()).unwrap_err();
        assert!(
            err.contains("millimetres") && err.contains("--scale 0.001"),
            "{err}"
        );
        let ok = Options {
            scale: 0.001,
            scale_given: true,
            ..opts()
        };
        let (_, loa, beam, depth) = mesh_to_offsets(&mm, &ok).unwrap();
        assert!(
            (loa - 60.0).abs() < 1e-9 && (beam - 14.0).abs() < 1e-9 && (depth - 4.5).abs() < 1e-9
        );
    }

    /// 8B10: `--baseline` and `--depth` cut away a skeg and a superstructure
    /// that would set the baseline and depth.
    #[test]
    fn baseline_and_depth_trim_appendages() {
        // A box hull 0..4.5 plus a 1 m skeg below (z -1..0) is the same
        // mesh as the hull plus extra triangles: emulate by shifting the
        // box down by 1 and cutting at z = 0.
        let obj = box_obj(60.0, 14.0, 5.5)
            .lines()
            .map(|l| {
                let p: Vec<&str> = l.split_whitespace().collect();
                if p.first() == Some(&"v") {
                    let z: f64 = p[3].parse().unwrap();
                    format!("v {} {} {}\n", p[1], p[2], z - 1.0)
                } else {
                    format!("{l}\n")
                }
            })
            .collect::<String>();
        let mesh = parse_obj(&obj).unwrap();
        let trimmed = Options {
            baseline: Some(0.0),
            depth: Some(4.5),
            ..opts()
        };
        let (csv, _, _, depth) = mesh_to_offsets(&mesh, &trimmed).unwrap();
        assert!((depth - 4.5).abs() < 1e-12);
        let hs = tpt_yard_hydrostatics::parse_offsets_csv(&csv)
            .unwrap()
            .hydrostatics(2.0)
            .unwrap();
        assert!((hs.displacement_t - 1.025 * 60.0 * 14.0 * 2.0).abs() < 1e-3 * 1722.0);
        // Without the trim the lowest point (z = -1) is the baseline.
        let (_, _, _, d_all) = mesh_to_offsets(&mesh, &opts()).unwrap();
        assert!((d_all - 5.5).abs() < 1e-9);
    }

    #[test]
    fn names_that_would_break_the_case_json_are_refused() {
        let dir = std::env::temp_dir().join(format!("tpt-yard-name-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let obj = dir.join("h.obj");
        std::fs::write(&obj, box_obj(60.0, 14.0, 4.5)).unwrap();
        let o = obj.to_string_lossy().into_owned();
        let out = dir.join("out").to_string_lossy().into_owned();
        for bad in ["a\"b", "a\\b", ""] {
            let err = run(&[&o, "--name", bad, "--out", &out], true).unwrap_err();
            assert!(err.contains("--name"), "{bad}: {err}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
