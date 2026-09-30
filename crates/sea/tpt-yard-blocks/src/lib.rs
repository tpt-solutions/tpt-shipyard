//! Shared block-lifting and handling primitives: lift points and sling
//! loads.
//!
//! The canonical statics for distributing a hook load over lift points,
//! used by the hull-erection planners, the structural lifting checks, and
//! the block-lifting benchmark. Sling loads follow the lever rule for
//! two-point picks; for three or more points an inverse-distance weighting
//! distributes the CoG eccentricity (statically indeterminate — the
//! convention is documented in RFC 0002).
//!
//! # Example
//!
//! ```
//! use tpt_yard_blocks::{sling_angles_deg, distribute_load_shares};
//! use tpt_yard_core::{Material, Vector3};
//!
//! let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(12.0, 0.0, 0.0)];
//! let shares = distribute_load_shares(240.0, Vector3::new(6.0, 0.0, 0.0), &lifts);
//! assert!((shares[0] - 0.5).abs() < 1e-9); // centred CoG splits evenly
//!
//! let angles = sling_angles_deg(&lifts, Vector3::new(6.0, 0.0, 4.0));
//! assert!((angles[0] - angles[1]).abs() < 1e-9);
//! ```

use tpt_yard_core::{Material, Vector3};

/// A lift point on a block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiftPoint {
    /// Position in block coordinates, m.
    pub position: Vector3,
    /// Certified working load limit of the padeye, kN.
    pub allowable_load_kn: f64,
}

impl LiftPoint {
    /// Creates a lift point at `position` with the given WLL.
    pub fn new(position: Vector3, allowable_load_kn: f64) -> Self {
        Self {
            position,
            allowable_load_kn,
        }
    }
}

/// The load distribution over a set of lift points.
#[derive(Debug, Clone, PartialEq)]
pub struct SlingLoad {
    /// Index of the lift point this leg attaches to.
    pub lift_point: usize,
    /// Vertical load share carried by this leg, kN.
    pub load_kn: f64,
    /// Sling leg angle from the horizontal, degrees.
    pub angle_from_horizontal_deg: f64,
    /// Actual leg tension including the angle, kN (load / sin θ).
    pub leg_tension_kn: f64,
}

/// Distributes a total vertical load over lift points as *shares* summing
/// to 1.
///
/// - 2 points: exact lever rule — the CoG projection is scalared onto the
///   lift line, and the shares split at that point (planar Euclidean
///   geometry, not a Manhattan proxy).
/// - ≥ 3 points: inverse-distance weighting of the CoG offset (the
///   documented convention; true multi-point statics are indeterminate
///   without sling stiffnesses).
///
/// Returns an empty vec if `lift_points` is empty.
pub fn distribute_load_shares(
    _total_weight_kn: f64,
    cog: Vector3,
    lift_points: &[Vector3],
) -> Vec<f64> {
    let n = lift_points.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![1.0];
    }
    if n == 2 {
        let u = lift_points[1] - lift_points[0];
        let span_sq = u.dot(u);
        if span_sq <= 1e-9 {
            return vec![0.5, 0.5];
        }
        let t = ((cog - lift_points[0]).dot(u) / span_sq).clamp(0.0, 1.0);
        return vec![1.0 - t, t];
    }
    // Inverse-distance weighting (planar Euclidean distance).
    let weights: Vec<f64> = lift_points
        .iter()
        .map(|p| 1.0 / ((p.x - cog.x).hypot(p.y - cog.y)).max(1e-3))
        .collect();
    let total: f64 = weights.iter().sum();
    weights.iter().map(|w| w / total).collect()
}

/// Sling leg angles from the horizontal, in degrees, for a hook at `hook`
/// (each leg runs from its lift point to the hook).
pub fn sling_angles_deg(lift_points: &[Vector3], hook: Vector3) -> Vec<f64> {
    lift_points
        .iter()
        .map(|p| {
            let dx = hook.x - p.x;
            let dy = hook.y - p.y;
            let dz = hook.z - p.z;
            (dz).atan2(dx.hypot(dy)).to_degrees()
        })
        .collect()
}

/// Full sling-load table: shares × total weight, resolved into leg tensions.
pub fn sling_loads(
    total_weight_kn: f64,
    cog: Vector3,
    lift_points: &[Vector3],
    hook: Vector3,
) -> Vec<SlingLoad> {
    let shares = distribute_load_shares(total_weight_kn, cog, lift_points);
    let angles = sling_angles_deg(lift_points, hook);
    shares
        .iter()
        .zip(&angles)
        .enumerate()
        .map(|(i, (share, angle))| {
            let load_kn = total_weight_kn * share;
            let sin = angle.to_radians().sin().max(1e-6);
            SlingLoad {
                lift_point: i,
                load_kn,
                angle_from_horizontal_deg: *angle,
                leg_tension_kn: load_kn / sin,
            }
        })
        .collect()
}

/// Geometry of a lifting lug (padeye) on the parent structure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadeyeGeometry {
    /// Lug plate thickness, mm.
    pub plate_thickness_mm: f64,
    /// Pin-hole diameter, mm.
    pub hole_diameter_mm: f64,
    /// Distance from the hole centre to the lug root (weld line), mm —
    /// the arm that sees out-of-plane bending.
    pub root_arm_mm: f64,
    /// Lug width across the root, mm.
    pub root_width_mm: f64,
    /// Distance from the hole centre to the outer edge (the tear-out
    /// path is twice this minus the hole radius), mm.
    pub edge_distance_mm: f64,
    /// Pin (shackle) diameter bearing in the hole, mm.
    pub pin_diameter_mm: f64,
    /// Fillet-weld leg attaching the lug to the parent, mm (0 = no weld
    /// check requested).
    pub weld_leg_mm: f64,
}

/// Result of the padeye check: one utilization per member plus the verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct PadeyeCheck {
    /// Pin bearing utilization (bearing stress against the plate-yield
    /// allowable with the 0.9 bearing factor).
    pub bearing_utilization: f64,
    /// Net-section tension utilization across the hole.
    pub net_section_utilization: f64,
    /// Tear-out (double shear rupture to the edge) utilization.
    pub tear_out_utilization: f64,
    /// Out-of-plane bending plus tension interaction at the root.
    pub root_bending_utilization: f64,
    /// Fillet-weld throat shear utilization (None when no weld given).
    pub weld_utilization: Option<f64>,
    /// Design load after the dynamic amplification factor, kN.
    pub design_load_kn: f64,
    /// All checks pass at or below 1.0.
    pub safe: bool,
    /// Findings.
    pub notes: Vec<String>,
}

/// Checks a lifting lug against the design load (review 7H roadmap item).
///
/// `load_kn` is the static sling load; `daf` is the dynamic amplification
/// factor (1.0 quiescent in-yard, 1.1-1.25 typical picks, offshore more).
/// `out_of_plane_deg` is the sling's angle out of the lug plane — it drives
/// the root bending check. Allowables (documented, screening-grade):
/// bearing `0.9·σy`, net-section and root interaction `σy/1.5`, tear-out
/// shear `0.6·σu/1.5`, weld throat shear `0.6·fu_weld/1.5` with
/// `fu_weld = σu` of the parent.
pub fn padeye_check(
    geo: &PadeyeGeometry,
    material: &Material,
    load_kn: f64,
    daf: f64,
    out_of_plane_deg: f64,
) -> PadeyeCheck {
    let mut notes = Vec::new();
    let daf = if daf >= 1.0 { daf } else { 1.0 };
    let design_kn = load_kn.abs() * daf;
    let t = geo.plate_thickness_mm.max(1e-6);
    let d = geo.hole_diameter_mm.max(1e-6);
    let sig_y = material.yield_mpa;
    let sig_u = material.ultimate_mpa;

    // 1. Pin bearing: sigma = P / (dp · t).
    let bearing = design_kn * 1000.0 / (geo.pin_diameter_mm.max(1e-6) * t);
    let bearing_allow = 0.9 * sig_y;
    let u_bearing = bearing / bearing_allow;

    // 2. Net section across the hole: sigma = P / ((w - d) · t).
    let net_width = (geo.root_width_mm - d).max(1e-6);
    let net_section = design_kn * 1000.0 / (net_width * t);
    let u_net = net_section / (sig_y / 1.5);

    // 3. Tear-out: double shear path from the hole to the edge, path
    //    length per side = edge_distance - d/2.
    let path = (geo.edge_distance_mm - d / 2.0).max(1e-6);
    let tear_capacity_n = 2.0 * path * t * (0.6 * sig_u / 1.5);
    let u_tear = design_kn * 1000.0 / tear_capacity_n;

    // 4. Root bending + tension interaction: out-of-plane component
    //    P·sin(beta) on the root arm, section modulus of the root width.
    let beta = out_of_plane_deg.to_radians();
    let p_out = design_kn * 1000.0 * beta.sin();
    let moment_n_mm = p_out * geo.root_arm_mm;
    let section_modulus = geo.root_width_mm * t * t / 6.0;
    let bending = moment_n_mm / section_modulus;
    let axial = design_kn * 1000.0 / (geo.root_width_mm * t);
    let u_root = (axial / (sig_y / 1.5)) + (bending / (sig_y / 1.5));

    // 5. Weld throat: two longitudinal fillets of length = root width.
    let u_weld = if geo.weld_leg_mm > 0.0 {
        let throat = geo.weld_leg_mm / std::f64::consts::SQRT_2;
        let capacity = 2.0 * geo.root_width_mm * throat * (0.6 * sig_u / 1.5);
        Some(design_kn * 1000.0 / capacity)
    } else {
        None
    };

    let mut safe = true;
    for (name, u) in [
        ("pin bearing", u_bearing),
        ("net section", u_net),
        ("tear-out", u_tear),
        ("root bending", u_root),
    ] {
        if u > 1.0 {
            safe = false;
            notes.push(format!("{name} utilization {u:.2} exceeds 1.0"));
        }
    }
    if let Some(u) = u_weld {
        if u > 1.0 {
            safe = false;
            notes.push(format!("weld throat utilization {u:.2} exceeds 1.0"));
        }
    }

    PadeyeCheck {
        bearing_utilization: u_bearing,
        net_section_utilization: u_net,
        tear_out_utilization: u_tear,
        root_bending_utilization: u_root,
        weld_utilization: u_weld,
        design_load_kn: design_kn,
        safe,
        notes,
    }
}

/// A spreader beam: compressive strut between two lift points, sling legs
/// down to the load.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpreaderBeam {
    /// Structural length between the head fittings, m.
    pub span_m: f64,
    /// Effective buckling radius of gyration of the beam section, m.
    pub radius_of_gyration_m: f64,
    /// Cross-section area, m².
    pub area_m2: f64,
    /// Young's modulus, GPa.
    pub youngs_modulus_gpa: f64,
    /// Yield strength, MPa.
    pub yield_mpa: f64,
}

/// Check of a spreader beam under a total lift load with slings at a given
/// head angle (from horizontal). Verifies beam compression (against yield
/// AND Euler buckling with K = 1 pin-pin), and the head-fitting vertical
/// components.
#[derive(Debug, Clone, PartialEq)]
pub struct SpreaderCheck {
    /// Axial compression in the beam, kN.
    pub compression_kn: f64,
    /// Yield utilization of the beam section.
    pub yield_utilization: f64,
    /// Euler buckling utilization (K = 1, pin-pin).
    pub buckling_utilization: f64,
    /// Vertical load at each head fitting, kN.
    pub head_vertical_kn: f64,
    /// True when both utilizations are at or below 1.
    pub safe: bool,
    /// Findings.
    pub notes: Vec<String>,
}

/// Checks a spreader beam carrying `load_kn` (total, after DAF) slung from
/// its two head fittings with the sling legs at `sling_angle_deg` from
/// horizontal (0° = horizontal legs, maximum compression; 90° = straight
/// vertical hang, no compression).
///
/// Statics: each head carries `load/(2·sin α)` leg tension; its vertical
/// component is `load/2`; the horizontal component `load/(2·tan α)`
/// compresses the beam. Euler capacity uses K = 1 (pinned ends) with the
/// practice 200 slenderness limit.
pub fn spreader_beam_check(
    beam: &SpreaderBeam,
    load_kn: f64,
    sling_angle_deg: f64,
) -> SpreaderCheck {
    let mut notes = Vec::new();
    let alpha = sling_angle_deg.to_radians();
    let head_vertical = load_kn / 2.0;
    let compression = load_kn / (2.0 * alpha.tan().max(1e-6));

    // Yield utilization.
    let stress_mpa = compression * 1000.0 / (beam.area_m2 * 1.0e6);
    let u_yield = stress_mpa / beam.yield_mpa;

    // Euler buckling: P_cr = pi^2 E I / L^2 = pi^2 E A r^2 / L^2, in kN.
    // SI units throughout: E in Pa (GPa x 1e9), A in m^2, r/L in m.
    let p_cr_kn = std::f64::consts::PI.powi(2)
        * beam.youngs_modulus_gpa
        * 1.0e9
        * beam.area_m2
        * beam.radius_of_gyration_m.powi(2)
        / beam.span_m.powi(2)
        / 1000.0;
    let u_buckle = compression / p_cr_kn.max(1e-9);

    let slenderness = beam.span_m / beam.radius_of_gyration_m.max(1e-6);
    let mut safe = u_yield <= 1.0 && u_buckle <= 1.0;
    if u_yield > 1.0 {
        notes.push(format!("beam yield utilization {u_yield:.2} exceeds 1.0"));
    }
    if u_buckle > 1.0 {
        notes.push(format!("Euler buckling utilization {u_buckle:.2} exceeds 1.0"));
    }
    if slenderness > 200.0 {
        safe = false;
        notes.push(format!("slenderness {slenderness:.0} exceeds the 200 practice limit"));
    }
    if sling_angle_deg <= 0.0 {
        safe = false;
        notes.push("sling legs must not be horizontal: unbounded compression".into());
    }

    SpreaderCheck {
        compression_kn: compression,
        yield_utilization: u_yield,
        buckling_utilization: u_buckle,
        head_vertical_kn: head_vertical,
        safe,
        notes,
    }
}

/// The worst-case lug load share under a CoG uncertainty envelope (review
/// 7H roadmap item): if the true CoG can sit anywhere within `uncertainty_m`
/// of the nominal position along the lift line, the lever-rule share of the
/// more heavily loaded of two lift points becomes
/// `share_max = (d_far + uncertainty) / span`, and its load share
/// `P = W · share_max / sin(angle)`.
///
/// Returns the worst-case leg tension in kN for a two-point pick.
pub fn cog_uncertainty_envelope(
    total_weight_kn: f64,
    lift_span_m: f64,
    uncertainty_m: f64,
    sling_angle_deg: f64,
) -> f64 {
    let span = lift_span_m.max(1e-6);
    let u = uncertainty_m.clamp(0.0, span);
    // Nominal centred CoG: each point takes half. Shifted CoG by u toward
    // one point: share = (span/2 + u) / span.
    let share = (span / 2.0 + u) / span;
    let vertical = total_weight_kn * share;
    vertical / sling_angle_deg.to_radians().sin().max(1e-6)
}

/// True if every leg is at or above the practice minimum sling angle
/// (default 30° from horizontal).
pub fn sling_angles_ok(loads: &[SlingLoad], min_angle_deg: f64) -> bool {
    loads
        .iter()
        .all(|l| l.angle_from_horizontal_deg >= min_angle_deg)
}

/// True if every leg tension is within the certified working load limit of
/// its padeye. `loads[i]` must correspond to `points[i]`.
pub fn legs_within_allowable(loads: &[SlingLoad], points: &[LiftPoint]) -> bool {
    loads.len() == points.len()
        && loads
            .iter()
            .zip(points)
            .all(|(l, p)| l.leg_tension_kn <= p.allowable_load_kn)
}

/// True if the CoG projects inside the **convex hull** of the lift points
/// (the tipping check for a pick). A degenerate (collinear) point set
/// checks the segment's bounding box, which is exact for a line.
pub fn cog_within_lifts(cog: Vector3, lift_points: &[Vector3]) -> bool {
    if lift_points.is_empty() {
        return false;
    }
    let mut pts: Vec<(f64, f64)> = lift_points.iter().map(|p| (p.x, p.y)).collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    pts.dedup();
    if pts.len() < 3 {
        let (min_x, max_x) = pts
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
        let (min_y, max_y) = pts
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
        return cog.x >= min_x && cog.x <= max_x && cog.y >= min_y && cog.y <= max_y;
    }
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut lower: Vec<(f64, f64)> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<(f64, f64)> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    let hull: Vec<(f64, f64)> = lower.into_iter().chain(upper).collect();
    if hull.len() < 3 {
        return false;
    }
    // Ray-crossing point-in-polygon (the guard guarantees yi != yj).
    let (px, py) = (cog.x, cog.y);
    let mut inside = false;
    let mut j = hull.len() - 1;
    for i in 0..hull.len() {
        let (xi, yi) = hull[i];
        let (xj, yj) = hull[j];
        if ((yi > py) != (yj > py)) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn centred_cog_splits_evenly() {
        let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(12.0, 0.0, 0.0)];
        let shares = distribute_load_shares(240.0, Vector3::new(6.0, 0.0, 0.0), &lifts);
        assert!(close(shares[0], 0.5, 1e-9));
        assert!(close(shares[1], 0.5, 1e-9));
    }

    #[test]
    fn lever_rule_shifts_load_to_near_point() {
        let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(12.0, 0.0, 0.0)];
        // CoG at 3 m: 3 m from point 0, 9 m from point 1 → 75 % on point 0.
        let shares = distribute_load_shares(240.0, Vector3::new(3.0, 0.0, 0.0), &lifts);
        assert!(close(shares[0], 0.75, 1e-9));
        assert!(close(shares[1], 0.25, 1e-9));
    }

    #[test]
    fn four_point_inverse_distance() {
        let lifts = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
            Vector3::new(0.0, 8.0, 0.0),
            Vector3::new(10.0, 8.0, 0.0),
        ];
        let shares = distribute_load_shares(100.0, Vector3::new(5.0, 4.0, 0.0), &lifts);
        let sum: f64 = shares.iter().sum();
        assert!(close(sum, 1.0, 1e-9));
        for s in &shares {
            assert!(close(*s, 0.25, 1e-9)); // symmetric → equal quarters
        }
        // Offset CoG: nearest point gets the largest share.
        let off = distribute_load_shares(100.0, Vector3::new(9.0, 7.0, 0.0), &lifts);
        assert!(off[3] == off.iter().cloned().fold(f64::MIN, f64::max));
    }

    #[test]
    fn sling_tension_includes_angle() {
        let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(6.0, 0.0, 0.0)];
        let hook = Vector3::new(3.0, 0.0, 3.0); // 45° legs (3 m run, 3 m rise)
        let loads = sling_loads(100.0, Vector3::new(3.0, 0.0, 0.0), &lifts, hook);
        for l in &loads {
            assert!(close(l.angle_from_horizontal_deg, 45.0, 1e-9), "{:?}", l);
            // 50 kN vertical share at 45° → √2 · 50 tension.
            assert!(close(l.leg_tension_kn, 50.0 * 2.0f64.sqrt(), 1e-9));
        }
        assert!(sling_angles_ok(&loads, 30.0));
    }

    #[test]
    fn shallow_sling_fails_the_angle_check() {
        let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(20.0, 0.0, 0.0)];
        let hook = Vector3::new(10.0, 0.0, 2.0); // ~11° legs: too shallow
        let loads = sling_loads(100.0, Vector3::new(10.0, 0.0, 0.0), &lifts, hook);
        assert!(!sling_angles_ok(&loads, 30.0));
    }

    #[test]
    fn tipping_check() {
        let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0)];
        assert!(cog_within_lifts(Vector3::new(5.0, 0.0, 3.0), &lifts));
        assert!(!cog_within_lifts(Vector3::new(15.0, 0.0, 3.0), &lifts));
        assert!(!cog_within_lifts(Vector3::new(5.0, 0.0, 0.0), &[]));
    }

    /// Regression (review 7A/A9): the tip check must use the convex hull —
    /// a CoG in the notch of an L-shaped lift set is inside the AABB but
    /// outside the hull, and it tips.
    #[test]
    fn tipping_uses_convex_hull_not_aabb() {
        let l_lifts = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
            Vector3::new(0.0, 10.0, 0.0),
        ];
        // (9, 9): inside [0,10]² but outside the triangle hull.
        assert!(!cog_within_lifts(Vector3::new(9.0, 9.0, 0.0), &l_lifts));
        // (2, 2): inside the hull.
        assert!(cog_within_lifts(Vector3::new(2.0, 2.0, 0.0), &l_lifts));
    }

    /// Regression (review 7A/A9): the lever rule uses planar Euclidean
    /// geometry — for a diagonal lift line the Manhattan proxy gave the
    /// wrong split.
    #[test]
    fn lever_rule_on_a_diagonal_lift_line() {
        // Lifts on the y = x diagonal; CoG projected at the quarter point.
        let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(8.0, 8.0, 0.0)];
        // CoG at (6, 6): projects to t = 0.75 along the line → 25 % on p0.
        let shares = distribute_load_shares(100.0, Vector3::new(6.0, 6.0, 0.0), &lifts);
        assert!((shares[1] - 0.75).abs() < 1e-9, "{shares:?}");
        assert!((shares[0] - 0.25).abs() < 1e-9);
        // The old Manhattan rule would have given 6/(6+12) = 1/3 / 2/3.
        // A CoG off the line projects correctly:
        let off = distribute_load_shares(100.0, Vector3::new(6.0, 8.0, 0.0), &lifts);
        // Projection of (6,8) onto y=x is (7,7): t = 7/8.
        assert!((off[1] - 0.875).abs() < 1e-9, "{off:?}");
    }

    /// Regression (review 7A/A9): `allowable_load_kn` has a check.
    /// Verification (review 7H): a properly proportioned lug passes every
    /// check; each undersizing trips its own check.
    #[test]
    fn padeye_check_passes_and_fails_the_right_member() {
        let steel = Material::ah36();
        let geo = PadeyeGeometry {
            plate_thickness_mm: 40.0,
            hole_diameter_mm: 60.0,
            root_arm_mm: 120.0,
            root_width_mm: 180.0,
            edge_distance_mm: 110.0,
            pin_diameter_mm: 57.0,
            weld_leg_mm: 12.0,
        };
        // 50 t sling leg, quiescent.
        let ok = padeye_check(&geo, &steel, 50.0 * 9.81, 1.0, 0.0);
        assert!(ok.safe, "{:?}", ok.notes);
        assert_eq!(ok.design_load_kn, 50.0 * 9.81);
        // Bearing: 490.5e3 / (57 x 40) = 215 MPa vs 0.9 x 355 = 319.5.
        assert!((ok.bearing_utilization - 0.673).abs() < 0.01, "{}", ok.bearing_utilization);

        // Thin plate: bearing and net section blow up, tear-out too.
        let mut thin = geo;
        thin.plate_thickness_mm = 8.0;
        let bad = padeye_check(&thin, &steel, 50.0 * 9.81, 1.0, 0.0);
        assert!(!bad.safe);
        assert!(bad.bearing_utilization > 1.0);
        assert!(bad.notes.iter().any(|n| n.contains("bearing")));

        // Short edge distance trips tear-out alone (path = 50 - 30 = 20 mm
        // per side; capacity 2 x 20 x 40 x 196 N ~ 314 kN < 490.5 kN).
        let mut short = geo;
        short.edge_distance_mm = 50.0;
        let tear = padeye_check(&short, &steel, 50.0 * 9.81, 1.0, 0.0);
        assert!(tear.tear_out_utilization > 1.0, "{}", tear.tear_out_utilization);
        assert!(tear.notes.iter().any(|n| n.contains("tear-out")));

        // Out-of-plane sling angle drives the root bending check.
        let mut tilted = geo;
        tilted.plate_thickness_mm = 12.0;
        let flat = padeye_check(&tilted, &steel, 50.0 * 9.81, 1.0, 0.0);
        let skewed = padeye_check(&tilted, &steel, 50.0 * 9.81, 1.0, 30.0);
        assert!(skewed.root_bending_utilization > flat.root_bending_utilization * 5.0);
        assert!(!skewed.safe, "12 mm lug at 30 deg out-of-plane must fail");
    }

    /// DAF scales the design load linearly.
    #[test]
    fn daf_scales_the_design_load() {
        let steel = Material::ah36();
        let geo = PadeyeGeometry {
            plate_thickness_mm: 40.0,
            hole_diameter_mm: 60.0,
            root_arm_mm: 120.0,
            root_width_mm: 180.0,
            edge_distance_mm: 110.0,
            pin_diameter_mm: 57.0,
            weld_leg_mm: 12.0,
        };
        let calm = padeye_check(&geo, &steel, 100.0, 1.0, 0.0);
        let dynamic = padeye_check(&geo, &steel, 100.0, 1.25, 0.0);
        assert!((dynamic.design_load_kn - 125.0).abs() < 1e-9);
        assert!((dynamic.bearing_utilization - calm.bearing_utilization * 1.25).abs() < 1e-9);
        // A DAF below 1 is treated as 1 (no un-amplification).
        let low = padeye_check(&geo, &steel, 100.0, 0.5, 0.0);
        assert_eq!(low.design_load_kn, 100.0);
    }

    /// Verification (review 7H): spreader-beam statics against hand
    /// values — compression = W/(2 tan a), head vertical = W/2.
    #[test]
    fn spreader_beam_statics_and_buckling() {
        let beam = SpreaderBeam {
            span_m: 12.0,
            radius_of_gyration_m: 0.08,
            area_m2: 0.01,
            youngs_modulus_gpa: 210.0,
            yield_mpa: 355.0,
        };
        // 1000 kN at 60 deg: compression = 1000/(2 tan60) = 288.7 kN.
        let check = spreader_beam_check(&beam, 1000.0, 60.0);
        assert!((check.compression_kn - 288.675).abs() < 0.01, "{}", check.compression_kn);
        assert!((check.head_vertical_kn - 500.0).abs() < 1e-9);
        // Yield: 288.7 kN / 0.01 m2 = 28.9 MPa vs 355 -> 0.081.
        assert!((check.yield_utilization - 0.0813).abs() < 0.001);
        assert!(check.safe, "{:?}", check.notes);
        // Euler capacity: Pcr = pi^2 x 210e9 x 0.01 x 0.08^2 / 144 = 9.2 MN.
        let p_cr = std::f64::consts::PI.powi(2) * 210.0e9 * 0.01 * 0.08f64.powi(2) / 144.0;
        assert!((check.buckling_utilization - 288.675 / (p_cr / 1000.0)).abs() < 0.01);

        // Shallow slings compress hard: at 10 deg the compression is
        // 1000/(2 tan10) = 2835 kN, over 3x the 60 deg case.
        let shallow = spreader_beam_check(&beam, 1000.0, 10.0);
        assert!(shallow.compression_kn > check.compression_kn * 3.0);
        // Horizontal legs are rejected outright.
        assert!(!spreader_beam_check(&beam, 1000.0, 0.0).safe);
    }

    /// Verification (review 7H): the CoG envelope — a +-u m uncertainty
    /// shifts the share from 50/50 to (span/2 + u)/span, and the tension
    /// follows through the sling angle.
    #[test]
    fn cog_envelope_widens_worst_case_leg() {
        let weight = 1000.0; // kN
        // No uncertainty, 45 deg: each leg takes 707.1 kN tension.
        let nominal = cog_uncertainty_envelope(weight, 10.0, 0.0, 45.0);
        assert!((nominal - weight / 2.0 / std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
        // +-0.5 m on a 10 m span: share = 5.5/10 = 55%.
        let with_u = cog_uncertainty_envelope(weight, 10.0, 0.5, 45.0);
        assert!((with_u - 550.0 / std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
        assert!(with_u > nominal);
        // Uncertainty cannot exceed the span (clamped: share <= 1).
        let clamped = cog_uncertainty_envelope(weight, 10.0, 9.0, 45.0);
        assert!(clamped >= with_u);
        // Shallower sling angle amplifies tension for the same share.
        assert!(cog_uncertainty_envelope(weight, 10.0, 0.5, 30.0) > with_u);
    }

    #[test]
    fn padeye_allowable_is_enforced() {
        let points = [
            LiftPoint::new(Vector3::new(0.0, 0.0, 0.0), 100.0),
            LiftPoint::new(Vector3::new(6.0, 0.0, 0.0), 100.0),
        ];
        let hook = Vector3::new(3.0, 0.0, 4.0); // ~53° legs
        let loads = sling_loads(120.0, Vector3::new(3.0, 0.0, 0.0), &points
            .iter()
            .map(|p| p.position)
            .collect::<Vec<_>>(), hook);
        // 60 kN share / sin 53° ≈ 75 kN < 100 kN WLL: fine.
        assert!(legs_within_allowable(&loads, &points));
        // Overload one padeye: 190 kN total → ~118 kN tension > 100 WLL.
        let loads = sling_loads(190.0, Vector3::new(3.0, 0.0, 0.0), &points
            .iter()
            .map(|p| p.position)
            .collect::<Vec<_>>(), hook);
        assert!(!legs_within_allowable(&loads, &points));
    }
}
