//! Offsets-driven hydrostatics and stability: the full-form counterpart
//! of the prismatic [`HullForm`](crate::HullForm) screening model.
//!
//! A hull is a set of stations (`x`, then `(draft, half-breadth)` pairs)
//! — the offsets table of a lines plan, read from CSV by
//! [`parse_offsets_csv`]. Between stations the half-breadth at a given
//! height is interpolated linearly. From that [`Bonjean`] gives:
//!
//! - [`Bonjean::hydrostatics`] — displacement, KB, KM, LCB, LCF, TPC and
//!   MCT1cm at an upright, even-keel draft;
//! - [`Bonjean::gz_curve`] — the GZ curve from cross curves of stability
//!   at constant displacement, `GZ = KN − (KG + FSC)·sinφ`, with each
//!   heeled section clipped exactly against the inclined waterline.
//!
//! Assumptions, stated plainly: the hull is symmetric about the
//! centreline, trim is zero while heeling, the section is closed by a flat
//! deck at the top offset of the table (reserve buoyancy above it is
//! ignored, so submerging the deck edge shows as a falling GZ), and
//! superstructure and appendages are not modelled.

use super::{
    Bonjean, GzCurve, GzPoint, Hydrostatics, LoadingCondition, SectionOffsets, RHO_SEA_T_M3,
};

/// Reads an offsets table from CSV text.
///
/// One row per offset: `station_x_m, draft_m, half_breadth_m` (comma,
/// semicolon or tab separated; `#` starts a comment; one optional header
/// row; rows in any order). `x` is measured from midship, positive
/// forward; `draft_m` is the height above the baseline. Each station must
/// start at the baseline (`draft_m = 0`).
///
/// # Errors
///
/// A message naming the offending line when a row is malformed, a value is
/// negative or not finite, a station repeats a draft or does not start at
/// the baseline, or fewer than two stations (each of at least two
/// offsets) are given.
pub fn parse_offsets_csv(text: &str) -> Result<Bonjean, String> {
    let mut rows: Vec<(f64, f64, f64, usize)> = Vec::new();
    let mut seen_content = false;
    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let first_content = !seen_content;
        seen_content = true;
        let cells: Vec<&str> = line.split([',', ';', '\t']).map(str::trim).collect();
        let nums: Vec<Result<f64, _>> = cells.iter().map(|c| c.parse::<f64>()).collect();
        if nums.iter().any(Result::is_err) {
            if first_content {
                continue; // header row
            }
            return Err(format!(
                "line {line_no}: expected three numbers, got '{line}'"
            ));
        }
        if cells.len() != 3 {
            return Err(format!(
                "line {line_no}: expected station_x_m, draft_m, half_breadth_m ({} columns found)",
                cells.len()
            ));
        }
        let v: Vec<f64> = nums.into_iter().map(Result::unwrap_or_default).collect();
        if v.iter().any(|n| !n.is_finite()) {
            return Err(format!("line {line_no}: values must be finite"));
        }
        if v[1] < 0.0 || v[2] < 0.0 {
            return Err(format!(
                "line {line_no}: draft and half-breadth must not be negative"
            ));
        }
        rows.push((v[0], v[1], v[2], line_no));
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));

    let mut stations: Vec<SectionOffsets> = Vec::new();
    for &(x, z, y, line_no) in &rows {
        match stations.last_mut() {
            Some(s) if s.x_from_midship_m == x => {
                if s.half_breadths.last().is_some_and(|p| p.0 == z) {
                    return Err(format!("line {line_no}: station x = {x} repeats draft {z}"));
                }
                s.half_breadths.push((z, y));
            }
            _ => stations.push(SectionOffsets {
                x_from_midship_m: x,
                half_breadths: vec![(z, y)],
            }),
        }
    }
    for s in &stations {
        if s.half_breadths.len() < 2 {
            return Err(format!(
                "station x = {}: needs at least two offsets",
                s.x_from_midship_m
            ));
        }
        if s.half_breadths[0].0 != 0.0 {
            return Err(format!(
                "station x = {}: the lowest draft must be 0 (the baseline), found {}",
                s.x_from_midship_m, s.half_breadths[0].0
            ));
        }
    }
    if stations.len() < 2 {
        return Err("an offsets table needs at least two stations".into());
    }
    Ok(Bonjean { stations })
}

/// Volume integrals of the submerged part of the hull.
struct Moments {
    vol: f64,
    mx: f64,
    my: f64,
    mz: f64,
}

/// The stations resampled along the length: one closed section polygon
/// per sample, plus the Simpson weights to integrate over them.
struct Prepared<'a> {
    stations: Vec<&'a SectionOffsets>,
    xs: Vec<f64>,
    weights: Vec<f64>,
    /// Station pair and blend factor at each sample.
    pairs: Vec<(usize, usize, f64)>,
    /// `(y, z)` section outlines, counter-clockwise, closed by the deck.
    polys: Vec<Vec<(f64, f64)>>,
    dx: f64,
    deck_m: f64,
}

impl<'a> Prepared<'a> {
    fn new(hull: &'a Bonjean) -> Option<Self> {
        let mut stations: Vec<&SectionOffsets> = hull.stations.iter().collect();
        stations.sort_by(|a, b| a.x_from_midship_m.total_cmp(&b.x_from_midship_m));
        if stations.len() < 2 || stations.iter().any(|s| s.half_breadths.len() < 2) {
            return None;
        }
        let x_min = stations[0].x_from_midship_m;
        let x_max = stations[stations.len() - 1].x_from_midship_m;
        if x_max <= x_min {
            return None;
        }
        // Even interval count, as Simpson's rule needs.
        let n = (stations.len() * 8).max(200);
        let dx = (x_max - x_min) / n as f64;
        let deck_m = stations
            .iter()
            .filter_map(|s| s.half_breadths.last().map(|p| p.0))
            .fold(0.0_f64, f64::max);

        let mut xs = Vec::with_capacity(n + 1);
        let mut weights = Vec::with_capacity(n + 1);
        let mut pairs = Vec::with_capacity(n + 1);
        let mut polys = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let x = x_min + i as f64 * dx;
            let k = stations
                .windows(2)
                .position(|w| x <= w[1].x_from_midship_m)
                .unwrap_or(stations.len() - 2);
            let (a, b) = (stations[k], stations[k + 1]);
            let f = ((x - a.x_from_midship_m) / (b.x_from_midship_m - a.x_from_midship_m))
                .clamp(0.0, 1.0);
            xs.push(x);
            weights.push(if i == 0 || i == n {
                1.0
            } else if i % 2 == 1 {
                4.0
            } else {
                2.0
            });
            pairs.push((k, k + 1, f));
            polys.push(section_outline(a, b, f));
        }
        Some(Self {
            stations,
            xs,
            weights,
            pairs,
            polys,
            dx,
            deck_m,
        })
    }

    /// Half-breadth of the interpolated section at sample `i`, height `z`.
    fn half_breadth(&self, i: usize, z: f64) -> f64 {
        let (a, b, f) = self.pairs[i];
        self.stations[a].half_breadth_at(z) * (1.0 - f) + self.stations[b].half_breadth_at(z) * f
    }

    /// Volume and first moments of the hull below the plane
    /// `z·cosφ + y·sinφ = c` (the heeled waterline; `y` is positive on the
    /// side that goes down).
    fn below_plane(&self, sin: f64, cos: f64, c: f64) -> Moments {
        let mut m = Moments {
            vol: 0.0,
            mx: 0.0,
            my: 0.0,
            mz: 0.0,
        };
        for (i, poly) in self.polys.iter().enumerate() {
            let (area, yc, zc) = clipped_area_centroid(poly, sin, cos, c);
            let wa = self.weights[i] * area;
            m.vol += wa;
            m.mx += wa * self.xs[i];
            m.my += wa * yc;
            m.mz += wa * zc;
        }
        let f = self.dx / 3.0;
        m.vol *= f;
        m.mx *= f;
        m.my *= f;
        m.mz *= f;
        m
    }

    /// Range of the plane constant `c` over the hull: below the lower end
    /// nothing is submerged, above the upper end everything is.
    fn plane_range(&self, sin: f64, cos: f64) -> (f64, f64) {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for &(y, z) in self.polys.iter().flatten() {
            let s = z * cos + y * sin;
            lo = lo.min(s);
            hi = hi.max(s);
        }
        (lo, hi)
    }

    fn hydrostatics(&self, draft_m: f64) -> Option<Hydrostatics> {
        if draft_m.is_nan() || draft_m <= 0.0 || draft_m > self.deck_m {
            return None;
        }
        let m = self.below_plane(0.0, 1.0, draft_m);
        if m.vol <= 1e-9 {
            return None;
        }
        let (mut aw, mut mx_w, mut ix_w, mut il_w) = (0.0, 0.0, 0.0, 0.0);
        for i in 0..self.xs.len() {
            let (w, x) = (self.weights[i], self.xs[i]);
            let y = self.half_breadth(i, draft_m);
            aw += w * 2.0 * y;
            mx_w += w * 2.0 * y * x;
            ix_w += w * (2.0 / 3.0) * y.powi(3);
            il_w += w * 2.0 * y * x * x;
        }
        let f = self.dx / 3.0;
        let (aw, mx_w, ix_w, il_w) = (aw * f, mx_w * f, ix_w * f, il_w * f);
        let lcf = if aw > 1e-9 { mx_w / aw } else { 0.0 };
        // Longitudinal inertia about the centre of flotation.
        let il = (il_w - aw * lcf * lcf).max(0.0);
        let kb = m.mz / m.vol;
        let bm = ix_w / m.vol;
        let bml = il / m.vol;
        let displacement_t = m.vol * RHO_SEA_T_M3;
        let length = self.xs[self.xs.len() - 1] - self.xs[0];
        Some(Hydrostatics {
            draft_m,
            displacement_t,
            kb_m: kb,
            km_m: kb + bm,
            lcb_m: m.mx / m.vol,
            lcf_m: lcf,
            tpc_t_cm: aw * RHO_SEA_T_M3 / 100.0,
            // GML ≈ BML (KG is not known to a hydrostatic table).
            mct1cm_tm_cm: displacement_t * bml / (100.0 * length),
            waterplane_area_m2: aw,
        })
    }
}

/// The closed outline of the section at blend factor `f` between two
/// stations, as `(y, z)` points counter-clockwise: up the starboard side,
/// across the deck, down the port side, across the keel.
fn section_outline(a: &SectionOffsets, b: &SectionOffsets, f: f64) -> Vec<(f64, f64)> {
    let mut zs: Vec<f64> = a
        .half_breadths
        .iter()
        .chain(&b.half_breadths)
        .map(|p| p.0)
        .collect();
    zs.sort_by(f64::total_cmp);
    zs.dedup();
    let profile: Vec<(f64, f64)> = zs
        .into_iter()
        .map(|z| {
            (
                a.half_breadth_at(z) * (1.0 - f) + b.half_breadth_at(z) * f,
                z,
            )
        })
        .collect();
    let mut poly: Vec<(f64, f64)> = profile.clone();
    poly.extend(profile.iter().rev().map(|&(y, z)| (-y, z)));
    poly
}

/// Area and centroid `(y, z)` of the part of a polygon on the submerged
/// side of `z·cosφ + y·sinφ = c` (Sutherland–Hodgman clip, then the
/// shoelace formulas).
fn clipped_area_centroid(poly: &[(f64, f64)], sin: f64, cos: f64, c: f64) -> (f64, f64, f64) {
    let side = |p: (f64, f64)| p.1 * cos + p.0 * sin - c;
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let (cur, nxt) = (poly[i], poly[(i + 1) % poly.len()]);
        let (sc, sn) = (side(cur), side(nxt));
        if sc <= 0.0 {
            out.push(cur);
        }
        if (sc < 0.0 && sn > 0.0) || (sc > 0.0 && sn < 0.0) {
            let t = sc / (sc - sn);
            out.push((cur.0 + t * (nxt.0 - cur.0), cur.1 + t * (nxt.1 - cur.1)));
        }
    }
    if out.len() < 3 {
        return (0.0, 0.0, 0.0);
    }
    let (mut a2, mut cy, mut cz) = (0.0, 0.0, 0.0);
    for i in 0..out.len() {
        let (p, q) = (out[i], out[(i + 1) % out.len()]);
        let cross = p.0 * q.1 - q.0 * p.1;
        a2 += cross;
        cy += (p.0 + q.0) * cross;
        cz += (p.1 + q.1) * cross;
    }
    if a2.abs() < 1e-12 {
        return (0.0, 0.0, 0.0);
    }
    (a2.abs() / 2.0, cy / (3.0 * a2), cz / (3.0 * a2))
}

impl Bonjean {
    /// Upright, even-keel hydrostatics at a draft, integrated over the
    /// offsets. `None` when the table has fewer than two stations, the
    /// draft is not positive, or it exceeds the table's deck height.
    ///
    /// MCT1cm takes `GML ≈ BML`, the usual table approximation.
    pub fn hydrostatics(&self, draft_m: f64) -> Option<Hydrostatics> {
        Prepared::new(self)?.hydrostatics(draft_m)
    }

    /// The GZ curve from `0°` to `to_deg` every `step_deg`, from cross
    /// curves at constant displacement with the free-surface correction
    /// applied (`GZ = KN − (KG + FSC)·sinφ`). Negative GZ is kept — the
    /// IMO area criteria should see a curve that crosses zero.
    ///
    /// `None` when [`Bonjean::hydrostatics`] is `None` for the loading's
    /// draft, or the angles are not a positive step up to a limit within
    /// 0°–180°.
    pub fn gz_curve(
        &self,
        loading: LoadingCondition,
        to_deg: f64,
        step_deg: f64,
    ) -> Option<GzCurve> {
        if !step_deg.is_finite()
            || !to_deg.is_finite()
            || step_deg <= 0.0
            || !(0.0..=180.0).contains(&to_deg)
        {
            return None;
        }
        let p = Prepared::new(self)?;
        let hs = p.hydrostatics(loading.draft_m)?;
        let target = hs.displacement_t / RHO_SEA_T_M3;
        let fsc = loading.free_surface_moment_tm / hs.displacement_t.max(1e-9);
        let lever = loading.kg_m() + fsc;

        let steps = (to_deg / step_deg + 1e-9).floor() as usize;
        let mut points = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let heel_deg = i as f64 * step_deg;
            if i == 0 {
                points.push(GzPoint {
                    heel_deg,
                    gz_m: 0.0,
                });
                continue;
            }
            let (sin, cos) = heel_deg.to_radians().sin_cos();
            // Bisect the plane constant until the displaced volume matches
            // the upright volume.
            let (mut lo, mut hi) = p.plane_range(sin, cos);
            for _ in 0..60 {
                let mid = 0.5 * (lo + hi);
                if p.below_plane(sin, cos, mid).vol < target {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let m = p.below_plane(sin, cos, 0.5 * (lo + hi));
            if m.vol <= 1e-9 {
                return None;
            }
            // The submerged side is −y, so y_B < 0 when heeled and the
            // buoyancy lever from the keel is KN = −y_B·cosφ + z_B·sinφ.
            let kn = -(m.my / m.vol) * cos + (m.mz / m.vol) * sin;
            points.push(GzPoint {
                heel_deg,
                gz_m: kn - lever * sin,
            });
        }
        Some(GzCurve {
            points,
            gm_corrected_m: hs.km_m - loading.kg_m() - fsc,
            free_surface_correction_m: fsc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wigley hull offsets, `y = B/2 (1 − (2x/L)²)(1 − ((T − h)/T)²)` for
    /// height `h` above the keel up to the draft `T`, wall-sided above it
    /// up to a deck at `1.5 T`.
    fn wigley(l: f64, b: f64, t: f64, n_stations: usize) -> Bonjean {
        let stations = (0..n_stations)
            .map(|i| {
                let x = -l / 2.0 + l * i as f64 / (n_stations - 1) as f64;
                let u = 2.0 * x / l;
                // 40 levels per draft: the piecewise-linear section
                // under-fills the parabola by ~ (Δz)²/12 per metre.
                let half_breadths = (0..=60)
                    .map(|k| {
                        let h = k as f64 * t / 40.0;
                        let shape = if h <= t {
                            1.0 - ((t - h) / t).powi(2)
                        } else {
                            1.0
                        };
                        (h, 0.5 * b * (1.0 - u * u) * shape)
                    })
                    .collect();
                SectionOffsets {
                    x_from_midship_m: x,
                    half_breadths,
                }
            })
            .collect();
        Bonjean { stations }
    }

    fn close(actual: f64, expected: f64, rel: f64, what: &str) {
        assert!(
            (actual - expected).abs() <= rel * expected.abs().max(1e-9),
            "{what}: {actual} vs analytic {expected}"
        );
    }

    /// The Wigley hull has closed-form hydrostatics:
    /// `∇ = 4/9·L·B·T`, `KB = 5T/8`, `BM = 3B²/(35T)`, `Aw = 2/3·L·B`,
    /// LCB = LCF = 0.
    #[test]
    fn wigley_hydrostatics_match_the_analytic_values() {
        let (l, b, t) = (100.0, 10.0, 5.0);
        let hs = wigley(l, b, t, 41).hydrostatics(t).expect("hydrostatics");
        close(
            hs.displacement_t,
            RHO_SEA_T_M3 * 4.0 / 9.0 * l * b * t,
            2e-3,
            "displacement",
        );
        close(hs.kb_m, 5.0 * t / 8.0, 2e-3, "KB");
        close(hs.km_m - hs.kb_m, 3.0 * b * b / (35.0 * t), 5e-3, "BM");
        close(
            hs.waterplane_area_m2,
            2.0 / 3.0 * l * b,
            2e-3,
            "waterplane area",
        );
        assert!(hs.lcb_m.abs() < 1e-6 && hs.lcf_m.abs() < 1e-6);
        close(
            hs.tpc_t_cm,
            hs.waterplane_area_m2 * RHO_SEA_T_M3 / 100.0,
            1e-12,
            "TPC",
        );
    }

    /// More than 25 stations used to give Simpson an odd interval count;
    /// the answer must not move when stations are added.
    #[test]
    fn many_stations_do_not_break_the_integration() {
        let (l, b, t) = (100.0, 10.0, 5.0);
        let v = |n| {
            wigley(l, b, t, n)
                .hydrostatics(t)
                .expect("hs")
                .displacement_t
        };
        close(v(41), v(81), 1e-3, "41 vs 81 stations");
        close(
            v(81),
            RHO_SEA_T_M3 * 4.0 / 9.0 * l * b * t,
            1e-3,
            "81 stations",
        );
    }

    /// Near the upright the Wigley waterline is wall-sided, so the GZ
    /// curve follows `GZ = sinφ (GM + ½ BM tan²φ)` at small angles.
    #[test]
    fn wigley_gz_follows_the_wall_sided_formula_at_small_heel() {
        let (l, b, t) = (100.0, 10.0, 5.0);
        let hull = wigley(l, b, t, 41);
        let loading = LoadingCondition::new(t, 3.5);
        let gz = hull.gz_curve(loading, 20.0, 5.0).expect("curve");
        let hs = hull.hydrostatics(t).expect("hs");
        let (gm, bm) = (hs.km_m - 3.5, hs.km_m - hs.kb_m);
        close(gz.gm_corrected_m, gm, 1e-12, "GM");
        for p in gz
            .points
            .iter()
            .filter(|p| p.heel_deg > 0.0 && p.heel_deg <= 10.0)
        {
            let r = p.heel_deg.to_radians();
            let wall_sided = r.sin() * (gm + 0.5 * bm * r.tan().powi(2));
            close(p.gz_m, wall_sided, 0.03, &format!("GZ at {}°", p.heel_deg));
        }
        assert_eq!(gz.points[0].gz_m, 0.0);
    }

    /// A box hull has an exact heeled solution while the lost wedge stays
    /// clear of the keel and the deck is not immersed: the wedge pair moves
    /// the buoyancy centre to `z = T/2 + B² tan²φ/(24T)`,
    /// `y = −B² tanφ/(12T)`, so `GZ = −y cosφ + z sinφ − KG sinφ`. This
    /// checks the heeled clipping far beyond the wall-sided small angles.
    #[test]
    fn a_box_hull_matches_the_exact_wedge_solution_at_large_heel() {
        let (l, b, t, kg) = (40.0, 22.0, 6.0, 3.0);
        let stations = [-l / 2.0, l / 2.0]
            .iter()
            .map(|&x| SectionOffsets {
                x_from_midship_m: x,
                half_breadths: vec![(0.0, b / 2.0), (t, b / 2.0), (14.0, b / 2.0)],
            })
            .collect();
        let hull = Bonjean { stations };
        let gz = hull
            .gz_curve(LoadingCondition::new(t, kg), 25.0, 5.0)
            .expect("curve");
        // B tan(phi)/2 < T up to ~28.6 degrees; deck at 14 m is far above.
        for p in gz.points.iter().filter(|p| p.heel_deg > 0.0) {
            let (sin, cos) = p.heel_deg.to_radians().sin_cos();
            let tan = sin / cos;
            let z_b = t / 2.0 + b * b * tan * tan / (24.0 * t);
            let y_b = -b * b * tan / (12.0 * t);
            let exact = -y_b * cos + z_b * sin - kg * sin;
            close(p.gz_m, exact, 2e-3, &format!("box GZ at {}°", p.heel_deg));
        }
    }

    /// The offsets path feeds the same IMO check as the prismatic model,
    /// and a high KG fails it.
    #[test]
    fn offsets_gz_feeds_the_imo_check() {
        let (l, b, t) = (100.0, 10.0, 5.0);
        let hull = wigley(l, b, t, 41);
        let good = hull
            .gz_curve(LoadingCondition::new(t, 3.0), 60.0, 5.0)
            .expect("curve");
        let verdict = crate::imo_2008_general_criteria(&good);
        assert!(verdict.gm_corrected_m > 0.15);
        let bad = hull
            .gz_curve(LoadingCondition::new(t, 6.4), 60.0, 5.0)
            .expect("curve");
        let short = hull
            .gz_curve(LoadingCondition::new(t, 3.0), 30.0, 5.0)
            .expect("curve");
        assert!(
            !crate::imo_2008_general_criteria(&short).passed,
            "a curve that stops at 30° is not evaluated as a pass"
        );
        assert!(!crate::imo_2008_general_criteria(&bad).passed);
    }

    #[test]
    fn free_surface_lowers_gm_and_gz() {
        let hull = wigley(100.0, 10.0, 5.0, 41);
        let dry = hull
            .gz_curve(LoadingCondition::new(5.0, 3.5), 20.0, 10.0)
            .expect("curve");
        let wet = hull
            .gz_curve(
                LoadingCondition::new(5.0, 3.5).with_free_surface_tm(2000.0),
                20.0,
                10.0,
            )
            .expect("curve");
        assert!(wet.gm_corrected_m < dry.gm_corrected_m);
        assert!(wet.points[1].gz_m < dry.points[1].gz_m);
        assert!(wet.free_surface_correction_m > 0.0);
    }

    #[test]
    fn unusable_inputs_give_none() {
        let hull = wigley(100.0, 10.0, 5.0, 11);
        assert!(hull.hydrostatics(0.0).is_none());
        assert!(hull.hydrostatics(50.0).is_none(), "above the deck");
        assert!(hull
            .gz_curve(LoadingCondition::new(5.0, 3.0), 40.0, 0.0)
            .is_none());
        assert!(Bonjean::default().hydrostatics(1.0).is_none());
    }

    #[test]
    fn csv_round_trips_a_table_with_header_and_comments() {
        let text = "\
# a tiny two-station hull
station_x_m, draft_m, half_breadth_m
-5, 0, 0
-5, 2, 1
5; 2; 1
5; 0; 0
";
        let hull = parse_offsets_csv(text).expect("parses");
        assert_eq!(hull.stations.len(), 2);
        assert_eq!(hull.stations[1].half_breadths, vec![(0.0, 0.0), (2.0, 1.0)]);
        // A 10 m x 2 m x 2 m box-ish prism: half-breadth rises 0 -> 1 over
        // 2 m, so each section is a triangle of area 2.
        let hs = hull.hydrostatics(2.0).expect("hydrostatics");
        close(hs.displacement_t, RHO_SEA_T_M3 * 20.0, 1e-9, "prism volume");
    }

    #[test]
    fn csv_errors_name_the_line() {
        let cases = [
            ("0,0,0\n0,1", "line 2"),
            ("0,0,0\n0,1,x", "line 2"),
            ("0,0,0\n0,-1,1", "line 2"),
            ("0,0,0\n0,1,1\n0,1,2\n1,0,0\n1,1,1", "repeats draft"),
            ("0,0.5,1\n0,1,1\n1,0,0\n1,1,1", "baseline"),
            ("0,0,0\n0,1,1", "two stations"),
            ("0,0,0\n1,0,0\n1,1,1", "two offsets"),
        ];
        for (text, needle) in cases {
            let err = parse_offsets_csv(text).expect_err(text);
            assert!(err.contains(needle), "'{text}' -> {err}");
        }
    }
}
