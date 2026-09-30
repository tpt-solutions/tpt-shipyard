//! Systems installation and routing for vessel outfitting.
//!
//! [`OutfittingPlan`] holds the outfit systems and their 3D routes.
//! [`OutfittingPlan::collision_detection`] checks route-against-route and
//! route-against-hull clashes (segment-to-segment distance with a
//! clearance, after a bounding-box screen);
//! [`OutfittingPlan::installation_sequence`] orders the work — large
//! systems first, smaller routing after (the "big items in first" doctrine
//! of pre-outfitted block construction).
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::{ConstructionMethod, FluidType, Geometry3D, OutfitSystem, Vector3};
//! use tpt_yard_outfitting::{OutfittingPlan, Route};
//!
//! let piping = OutfitSystem::Piping { fluid: FluidType::SeaWater, diameter_mm: 200.0 };
//! let cable = OutfitSystem::Electrical { voltage_v: 440.0, cable_type: "FEF".into() };
//! let mut plan = OutfittingPlan::new();
//! plan.add_system(piping.clone());
//! plan.add_system(cable.clone());
//!
//! // Two routes crossing at the same point clash.
//! plan.routes.push(Route {
//!     system: 0,
//!     waypoints: vec![Vector3::new(0.0, 0.0, 2.0), Vector3::new(20.0, 0.0, 2.0)],
//!     cross_section_m: 0.3,
//! });
//! plan.routes.push(Route {
//!     system: 1,
//!     waypoints: vec![Vector3::new(10.0, -2.0, 2.0), Vector3::new(10.0, 2.0, 2.0)],
//!     cross_section_m: 0.2,
//! });
//!
//! let hull = Geometry3D::from_box(60.0, 12.0, 12.0);
//! let clashes = plan.collision_detection(&hull, 0.1);
//! assert_eq!(clashes.len(), 1); // the route crossing, not the hull wall
//! assert_eq!(clashes[0].between, (0, 1));
//! ```

use std::fmt;

use tpt_yard_core::{BoundingBox, Geometry3D, OutfitSystem, Vector3};

/// A routed run of an outfit system.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// Index into [`OutfittingPlan::systems`].
    pub system: usize,
    /// Polyline waypoints, m.
    pub waypoints: Vec<Vector3>,
    /// Overall cross-section size (pipe diameter incl. insulation, cable
    /// tray width, duct side), m.
    pub cross_section_m: f64,
}

impl Route {
    /// Axis-aligned bounding box of the route grown by half the cross
    /// section.
    pub fn bounding_box(&self) -> BoundingBox {
        let half = self.cross_section_m / 2.0;
        let mut bb = match self.waypoints.first() {
            Some(&w) => BoundingBox {
                min: w - Vector3::new(half, half, half),
                max: w + Vector3::new(half, half, half),
            },
            None => BoundingBox::point(Vector3::ZERO),
        };
        for w in &self.waypoints {
            bb = bb.union(&BoundingBox {
                min: *w - Vector3::new(half, half, half),
                max: *w + Vector3::new(half, half, half),
            });
        }
        bb
    }
}

/// A detected clash.
#[derive(Debug, Clone, PartialEq)]
pub struct Collision {
    /// The two clash participants: (system index, system index) or
    /// (system index, hull sentinel = usize::MAX).
    pub between: (usize, usize),
    /// Approximate clash location, m.
    pub at: Vector3,
    /// What kind of clash.
    pub kind: CollisionKind,
}

/// Kinds of clash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionKind {
    /// Two systems overlap each other.
    SystemVsSystem,
    /// A system leaves the hull envelope.
    SystemVsHull,
}

impl fmt::Display for CollisionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CollisionKind::SystemVsSystem => f.write_str("system vs system"),
            CollisionKind::SystemVsHull => f.write_str("system vs hull"),
        }
    }
}

/// The outfitting plan: systems + routes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutfittingPlan {
    /// Systems to install.
    pub systems: Vec<OutfitSystem>,
    /// Routes, one per (usually) system.
    pub routes: Vec<Route>,
    /// Hard precedence edges `(before, after)` as system indices: the
    /// installation of `after` must not start before `before` is complete
    /// (e.g. a machinery unit before its exhaust ducting). Size ordering
    /// still decides among systems whose dependencies are satisfied.
    pub precedes: Vec<(usize, usize)>,
}

/// Why an installation sequence could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceError {
    /// A precedence edge names a system index that does not exist.
    DanglingDependency(usize, usize),
    /// The precedence edges form a cycle, so no order exists.
    DependencyCycle,
}

impl fmt::Display for SequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SequenceError::DanglingDependency(a, b) => {
                write!(f, "precedence edge ({a}, {b}) names a missing system")
            }
            SequenceError::DependencyCycle => f.write_str("precedence edges form a cycle"),
        }
    }
}

impl std::error::Error for SequenceError {}

impl OutfittingPlan {
    /// Creates an empty plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a system; returns its index.
    pub fn add_system(&mut self, system: OutfitSystem) -> usize {
        self.systems.push(system);
        self.systems.len() - 1
    }

    /// Cross-section size of a system (used for sequencing).
    fn system_size_m(system: &OutfitSystem) -> f64 {
        match system {
            OutfitSystem::Piping { diameter_mm, .. } => diameter_mm / 1000.0,
            OutfitSystem::Electrical { .. } => 0.2,
            OutfitSystem::Hvac { duct_size_mm } => duct_size_mm / 1000.0,
            OutfitSystem::Structural { .. } => 1.0,
            OutfitSystem::Machinery { .. } => 3.0,
            OutfitSystem::Navigation { .. } => 0.5,
        }
    }

    /// Detects clashes: route vs route and route outside the hull
    /// envelope.
    ///
    /// Route pairs are screened by bounding box, then checked
    /// **segment to segment**: the distance between the closest points of
    /// the polylines must exceed the sum of the half cross-sections plus
    /// `clearance_m`. Two L-shaped routes whose boxes overlap but whose
    /// runs never approach each other do **not** clash (the old one-box-
    /// per-route check produced exactly those false clashes).
    pub fn collision_detection(
        &self,
        hull_geometry: &Geometry3D,
        clearance_m: f64,
    ) -> Vec<Collision> {
        let hull_bb = hull_geometry.bounding_box();
        let mut collisions = Vec::new();

        for (i, a) in self.routes.iter().enumerate() {
            // Screening boxes grow by the clearance as well: the exact
            // segment test only runs on pairs that could possibly clash.
            let screen_a = inflate(a, clearance_m);
            // Route vs route: bbox screen, then exact segment distance.
            for b in self.routes.iter().skip(i + 1) {
                let screen_b = inflate(b, clearance_m);
                if !intersects(&screen_a, &screen_b) {
                    continue;
                }
                let limit = a.cross_section_m / 2.0 + b.cross_section_m / 2.0 + clearance_m;
                if let Some(at) = segments_within(a, b, limit) {
                    collisions.push(Collision {
                        between: (a.system, b.system),
                        at,
                        kind: CollisionKind::SystemVsSystem,
                    });
                }
            }
            // Route vs hull: every waypoint (inflated by the half
            // cross-section) must stay inside the envelope.
            let half = a.cross_section_m / 2.0;
            let outside = a.waypoints.iter().find(|w| {
                !(w.x - half >= hull_bb.min.x
                    && w.x + half <= hull_bb.max.x
                    && w.y - half >= hull_bb.min.y
                    && w.y + half <= hull_bb.max.y
                    && w.z - half >= hull_bb.min.z
                    && w.z + half <= hull_bb.max.z)
            });
            if let Some(w) = outside {
                collisions.push(Collision {
                    between: (a.system, usize::MAX),
                    at: *w,
                    kind: CollisionKind::SystemVsHull,
                });
            }
        }
        collisions
    }

    /// Installation order (review 7B leftover): a precedence-feasible
    /// serial schedule — at every step the *eligible* systems (all
    /// `precedes` dependencies already installed) compete by size, largest
    /// first, ties by plan order. Without edges this is exactly the old
    /// large-first ordering.
    ///
    /// # Errors
    ///
    /// [`SequenceError::DanglingDependency`] when an edge names a missing
    /// system, [`SequenceError::DependencyCycle`] when the edges form a
    /// cycle (detected by stalling with systems left over).
    pub fn installation_sequence(&self) -> Result<Vec<usize>, SequenceError> {
        let n = self.systems.len();
        for &(a, b) in &self.precedes {
            if a >= n || b >= n {
                return Err(SequenceError::DanglingDependency(a, b));
            }
        }
        // Remaining dependency count per system.
        let mut pending: Vec<usize> = vec![0; n];
        for &(_, b) in &self.precedes {
            pending[b] += 1;
        }
        let mut order = Vec::with_capacity(n);
        let mut installed = vec![false; n];
        while order.len() < n {
            // Eligible: everything installed; pick the largest (ties by
            // index for stability).
            let size = |i: usize| Self::system_size_m(&self.systems[i]);
            let pick = (0..n)
                .filter(|i| !installed[*i] && pending[*i] == 0)
                .min_by(|&a, &b| {
                    // Descending size, ascending index: `min_by` wants
                    // Less on the preferred side, i.e. a larger size.
                    size(b).total_cmp(&size(a)).then(a.cmp(&b))
                });
            let Some(i) = pick else {
                // Nothing eligible with systems left: a cycle.
                return Err(SequenceError::DependencyCycle);
            };
            installed[i] = true;
            for &(from, to) in &self.precedes {
                if from == i {
                    pending[to] -= 1;
                }
            }
            order.push(i);
        }
        Ok(order)
    }
}

/// The route bounding box grown by an extra margin (for the clash screen).
fn inflate(r: &Route, extra: f64) -> BoundingBox {
    let half = r.cross_section_m / 2.0 + extra;
    let mut bb = match r.waypoints.first() {
        Some(&w) => BoundingBox {
            min: w - Vector3::new(half, half, half),
            max: w + Vector3::new(half, half, half),
        },
        None => BoundingBox::point(Vector3::ZERO),
    };
    for w in &r.waypoints {
        bb = bb.union(&BoundingBox {
            min: *w - Vector3::new(half, half, half),
            max: *w + Vector3::new(half, half, half),
        });
    }
    bb
}

fn intersects(a: &BoundingBox, b: &BoundingBox) -> bool {
    a.min.x <= b.max.x
        && a.max.x >= b.min.x
        && a.min.y <= b.max.y
        && a.max.y >= b.min.y
        && a.min.z <= b.max.z
        && a.max.z >= b.min.z
}

/// Closest point on segment `p0-p1` to `q`.
fn closest_point_on_segment(p0: Vector3, p1: Vector3, q: Vector3) -> Vector3 {
    let d = p1 - p0;
    let len2 = d.dot(d);
    if len2 <= 1e-12 {
        return p0;
    }
    let t = ((q - p0).dot(d) / len2).clamp(0.0, 1.0);
    p0 + d * t
}

/// If any segment pair of the two polylines comes within `limit`, returns
/// an approximate clash location (the midpoint of the closest pair).
fn segments_within(a: &Route, b: &Route, limit: f64) -> Option<Vector3> {
    let limit2 = limit * limit;
    for w in a.waypoints.windows(2) {
        for v in b.waypoints.windows(2) {
            let (pa, pb) = closest_points_between_segments(w[0], w[1], v[0], v[1]);
            if (pa - pb).dot(pa - pb) <= limit2 {
                return Some((pa + pb) * 0.5);
            }
        }
    }
    // Degenerate single-point routes.
    if a.waypoints.len() == 1 || b.waypoints.len() == 1 {
        for wa in &a.waypoints {
            for wb in &b.waypoints {
                let d = *wa - *wb;
                if d.dot(d) <= limit2 {
                    return Some((*wa + *wb) * 0.5);
                }
            }
        }
    }
    None
}

/// Closest points between two 3-D segments (standard clamped solving of
/// the quadratic system; degenerate segments fall back to point checks).
fn closest_points_between_segments(
    p0: Vector3,
    p1: Vector3,
    q0: Vector3,
    q1: Vector3,
) -> (Vector3, Vector3) {
    let d1 = p1 - p0;
    let d2 = q1 - q0;
    let r = p0 - q0;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);
    if a <= 1e-12 && e <= 1e-12 {
        return (p0, q0);
    }
    if a <= 1e-12 {
        return (p0, closest_point_on_segment(q0, q1, p0));
    }
    if e <= 1e-12 {
        return (closest_point_on_segment(p0, p1, q0), q0);
    }
    let c = d1.dot(r);
    let b = d1.dot(d2);
    let denom = a * e - b * b;
    let mut s = if denom > 1e-12 {
        ((b * f - c * e) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    // Clamp t, then re-solve s on the clamped boundary (Ericson, RTCD 5.1.9).
    let mut t = (b * s + f) / e;
    if t < 0.0 {
        t = 0.0;
        s = (-c / a).clamp(0.0, 1.0);
    } else if t > 1.0 {
        t = 1.0;
        s = ((b - c) / a).clamp(0.0, 1.0);
    }
    (p0 + d1 * s, q0 + d2 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_core::FluidType;

    fn plan() -> OutfittingPlan {
        let mut plan = OutfittingPlan::new();
        plan.add_system(OutfitSystem::Piping {
            fluid: FluidType::SeaWater,
            diameter_mm: 200.0,
        });
        plan.add_system(OutfitSystem::Electrical {
            voltage_v: 440.0,
            cable_type: "FEF".into(),
        });
        plan.add_system(OutfitSystem::Machinery {
            equipment: "main engine".into(),
        });
        plan
    }

    #[test]
    fn crossing_routes_clash() {
        let mut p = plan();
        p.routes.push(Route {
            system: 0,
            waypoints: vec![Vector3::new(0.0, 0.0, 2.0), Vector3::new(20.0, 0.0, 2.0)],
            cross_section_m: 0.3,
        });
        p.routes.push(Route {
            system: 1,
            waypoints: vec![Vector3::new(10.0, -2.0, 2.0), Vector3::new(10.0, 2.0, 2.0)],
            cross_section_m: 0.2,
        });
        let hull = Geometry3D::from_box(60.0, 12.0, 12.0);
        let clashes = p.collision_detection(&hull, 0.1);
        assert_eq!(clashes.len(), 1);
        assert_eq!(clashes[0].between, (0, 1));
        assert_eq!(clashes[0].kind, CollisionKind::SystemVsSystem);
    }

    #[test]
    fn parallel_routes_with_clearance_do_not_clash() {
        let mut p = plan();
        p.routes.push(Route {
            system: 0,
            waypoints: vec![Vector3::new(0.0, 0.0, 2.0), Vector3::new(20.0, 0.0, 2.0)],
            cross_section_m: 0.3,
        });
        p.routes.push(Route {
            system: 1,
            waypoints: vec![Vector3::new(0.0, 0.0, 3.5), Vector3::new(20.0, 0.0, 3.5)],
            cross_section_m: 0.2,
        });
        let hull = Geometry3D::from_box(60.0, 12.0, 12.0);
        assert!(p.collision_detection(&hull, 0.1).is_empty());
    }

    #[test]
    fn route_outside_hull_clashes_with_hull() {
        let mut p = plan();
        p.routes.push(Route {
            system: 0,
            waypoints: vec![
                Vector3::new(0.0, 0.0, 2.0),
                Vector3::new(45.0, 0.0, 2.0), // 45 m > 40 m hull
            ],
            cross_section_m: 0.3,
        });
        let hull = Geometry3D::from_box(40.0, 12.0, 12.0);
        let clashes = p.collision_detection(&hull, 0.1);
        assert_eq!(clashes.len(), 1);
        assert_eq!(clashes[0].kind, CollisionKind::SystemVsHull);
        assert_eq!(clashes[0].between, (0, usize::MAX));
    }

    /// Verification: installation sequence puts large systems first.
    #[test]
    fn installation_sequence_large_first() {
        let p = plan(); // piping 0.2 m, electrical 0.2, machinery 3.0
        let order = p.installation_sequence().expect("no edges");
        assert_eq!(order, vec![2, 0, 1]); // machinery, piping, electrical
    }

    /// Review 7B leftover: hard precedence edges beat the size rule, and
    /// cycles / dangling edges are errors rather than silent orders.
    #[test]
    fn installation_sequence_respects_dependencies() {
        let mut p = plan(); // 0 piping (0.2), 1 electrical (0.2), 2 machinery (3.0)
                            // The exhaust ducting (small) may only run after the machinery is
                            // set; the machinery itself waits for the piping (hangers first).
        p.precedes = vec![(1, 2), (0, 1)];
        let order = p.installation_sequence().expect("acyclic");
        // Eligible first step: only 0 (piping) — 1 and 2 are gated — even
        // though 2 is by far the largest.
        assert_eq!(order[0], 0);
        assert_eq!(order, vec![0, 1, 2]);

        // A cycle is detected.
        p.precedes = vec![(0, 1), (1, 2), (2, 0)];
        assert_eq!(
            p.installation_sequence(),
            Err(SequenceError::DependencyCycle)
        );

        // A dangling edge is detected.
        p.precedes = vec![(0, 7)];
        assert_eq!(
            p.installation_sequence(),
            Err(SequenceError::DanglingDependency(0, 7))
        );
    }

    /// Regression (review 7B): two L-shaped routes whose bounding boxes
    /// overlap must NOT clash when their runs never approach each other —
    /// the old one-box-per-route check produced exactly this false clash.
    #[test]
    fn bbox_overlap_alone_is_not_a_clash() {
        let mut p = plan();
        // Route A: along +x at z = 2, then turns away in -y.
        p.routes.push(Route {
            system: 0,
            waypoints: vec![
                Vector3::new(0.0, 0.0, 2.0),
                Vector3::new(20.0, 0.0, 2.0),
                Vector3::new(20.0, -5.0, 2.0),
            ],
            cross_section_m: 0.3,
        });
        // Route B: runs through the interior of A's L-shaped bounding box
        // (between the legs), 2.5 m away from every point of A.
        p.routes.push(Route {
            system: 1,
            waypoints: vec![Vector3::new(5.0, -2.5, 2.0), Vector3::new(15.0, -2.5, 2.0)],
            cross_section_m: 0.2,
        });
        let hull = Geometry3D::from_box(60.0, 12.0, 12.0);
        // Bounding boxes overlap (B is inside A's L footprint).
        assert!(intersects(
            &p.routes[0].bounding_box(),
            &p.routes[1].bounding_box()
        ));
        assert!(
            p.collision_detection(&hull, 0.1).is_empty(),
            "runs 2.5 m apart must not clash"
        );
        // Route B cutting across A at the same point does clash.
        p.routes[1].waypoints = vec![Vector3::new(10.0, -2.0, 2.0), Vector3::new(10.0, 2.0, 2.0)];
        let clashes = p.collision_detection(&hull, 0.1);
        assert_eq!(clashes.len(), 1);
        assert_eq!(clashes[0].between, (0, 1));
    }

    /// The clearance parameter separates routes that touch the geometric
    /// sum of their sections but sit inside the requested margin.
    #[test]
    fn clearance_widens_the_clash_test() {
        let mut p = plan();
        p.routes.push(Route {
            system: 0,
            waypoints: vec![Vector3::new(0.0, 0.0, 2.0), Vector3::new(20.0, 0.0, 2.0)],
            cross_section_m: 0.3,
        });
        // Surface-to-surface gap: (1.0 - 0.15 - 0.1) = 0.75 m.
        p.routes.push(Route {
            system: 1,
            waypoints: vec![Vector3::new(0.0, 1.0, 2.0), Vector3::new(20.0, 1.0, 2.0)],
            cross_section_m: 0.2,
        });
        let hull = Geometry3D::from_box(60.0, 24.0, 12.0);
        assert!(
            p.collision_detection(&hull, 0.1).is_empty(),
            "0.75 m gap clears a 0.1 m allowance: {:?}",
            p.collision_detection(&hull, 0.1)
        );
        assert_eq!(
            p.collision_detection(&hull, 1.0).len(),
            1,
            "a 1.0 m allowance engulfs the 0.75 m gap"
        );
    }

    #[test]
    fn route_bbox_grows_by_cross_section() {
        let r = Route {
            system: 0,
            waypoints: vec![Vector3::new(0.0, 0.0, 2.0)],
            cross_section_m: 1.0,
        };
        let bb = r.bounding_box();
        assert!((bb.extents().x - 1.0).abs() < 1e-12);
        assert!((bb.centre().z - 2.0).abs() < 1e-12);
    }
}
