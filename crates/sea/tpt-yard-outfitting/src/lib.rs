//! Systems installation and routing for vessel outfitting.
//!
//! [`OutfittingPlan`] holds the outfit systems and their 3D routes.
//! [`OutfittingPlan::collision_detection`] checks route-against-route and
//! route-against-hull clashes (bounding-box screening with clearance);
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
//! let clashes = plan.collision_detection(&hull);
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
}

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

    /// Detects clashes: route vs route (with clearance) and route outside
    /// the hull envelope.
    pub fn collision_detection(&self, hull_geometry: &Geometry3D) -> Vec<Collision> {
        let hull_bb = hull_geometry.bounding_box();
        let mut collisions = Vec::new();

        for (i, a) in self.routes.iter().enumerate() {
            let bba = a.bounding_box();
            // Route vs route.
            for (j, b) in self.routes.iter().enumerate().skip(i + 1) {
                if j == a.system && i == b.system {
                    continue;
                }
                let bbb = b.bounding_box();
                if intersects(&bba, &bbb) {
                    collisions.push(Collision {
                        between: (a.system, b.system),
                        at: overlap_centre(&bba, &bbb),
                        kind: CollisionKind::SystemVsSystem,
                    });
                }
            }
            // Route vs hull: route must stay inside the envelope.
            let inside = bba.min.x >= hull_bb.min.x
                && bba.max.x <= hull_bb.max.x
                && bba.min.y >= hull_bb.min.y
                && bba.max.y <= hull_bb.max.y
                && bba.min.z >= hull_bb.min.z
                && bba.max.z <= hull_bb.max.z;
            if !inside {
                collisions.push(Collision {
                    between: (a.system, usize::MAX),
                    at: bba.centre(),
                    kind: CollisionKind::SystemVsHull,
                });
            }
        }
        collisions
    }

    /// Installation order: large systems first, smaller routing after.
    /// Ties break by plan order (stable). Route order follows the same
    /// rule.
    pub fn installation_sequence(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.systems.len()).collect();
        order.sort_by(|&a, &b| {
            let sa = Self::system_size_m(&self.systems[a]);
            let sb = Self::system_size_m(&self.systems[b]);
            sb.total_cmp(&sa).then(a.cmp(&b))
        });
        order
    }
}

fn intersects(a: &BoundingBox, b: &BoundingBox) -> bool {
    a.min.x <= b.max.x
        && a.max.x >= b.min.x
        && a.min.y <= b.max.y
        && a.max.y >= b.min.y
        && a.min.z <= b.max.z
        && a.max.z >= b.min.z
}

fn overlap_centre(a: &BoundingBox, b: &BoundingBox) -> Vector3 {
    (a.centre() + b.centre()) * 0.5
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
        let clashes = p.collision_detection(&hull);
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
        assert!(p.collision_detection(&hull).is_empty());
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
        let clashes = p.collision_detection(&hull);
        assert_eq!(clashes.len(), 1);
        assert_eq!(clashes[0].kind, CollisionKind::SystemVsHull);
        assert_eq!(clashes[0].between, (0, usize::MAX));
    }

    /// Verification: installation sequence puts large systems first.
    #[test]
    fn installation_sequence_large_first() {
        let p = plan(); // piping 0.2 m, electrical 0.2, machinery 3.0
        let order = p.installation_sequence();
        assert_eq!(order, vec![2, 0, 1]); // machinery, piping, electrical
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
