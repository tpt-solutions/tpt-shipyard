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
//! use tpt_yard_core::Vector3;
//!
//! let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(12.0, 0.0, 0.0)];
//! let shares = distribute_load_shares(240.0, Vector3::new(6.0, 0.0, 0.0), &lifts);
//! assert!((shares[0] - 0.5).abs() < 1e-9); // centred CoG splits evenly
//!
//! let angles = sling_angles_deg(&lifts, Vector3::new(6.0, 0.0, 4.0));
//! assert!((angles[0] - angles[1]).abs() < 1e-9);
//! ```

use tpt_yard_core::Vector3;

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
/// - 2 points: exact lever rule on the CoG offset along the lift line.
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
        let d0 = manhattan(cog, lift_points[0]);
        let d1 = manhattan(cog, lift_points[1]);
        let span = d0 + d1;
        if span <= 1e-9 {
            return vec![0.5, 0.5];
        }
        return vec![d1 / span, d0 / span];
    }
    // Inverse-distance weighting.
    let weights: Vec<f64> = lift_points
        .iter()
        .map(|p| 1.0 / manhattan(cog, *p).max(1e-3))
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

/// True if every leg is at or above the practice minimum sling angle
/// (default 30° from horizontal).
pub fn sling_angles_ok(loads: &[SlingLoad], min_angle_deg: f64) -> bool {
    loads
        .iter()
        .all(|l| l.angle_from_horizontal_deg >= min_angle_deg)
}

/// True if the CoG projects inside the axis-aligned hull of the lift points
/// (the tipping check for a pick).
pub fn cog_within_lifts(cog: Vector3, lift_points: &[Vector3]) -> bool {
    if lift_points.is_empty() {
        return false;
    }
    let (min_x, max_x) = lift_points
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
    let (min_y, max_y) = lift_points
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
    cog.x >= min_x && cog.x <= max_x && cog.y >= min_y && cog.y <= max_y
}

fn manhattan(a: Vector3, b: Vector3) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
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
}
