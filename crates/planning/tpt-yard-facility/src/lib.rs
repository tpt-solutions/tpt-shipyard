//! Shipyard and orbital-facility layout planning with capacity checks.
//!
//! [`FacilityPlan`] models the yard's resources — cranes, workshops,
//! docks, orbital assembly bays — as placed facilities with capacities.
//! [`FacilityPlan::check_capacity`] validates that peak resource demand
//! fits (cross-referencing the scheduler's peak figures), and
//! [`FacilityPlan::can_place`] enforces layout clearances.
//!
//! # Example
//!
//! ```
//! use tpt_yard_facility::{Facility, FacilityKind, FacilityPlan};
//! use tpt_yard_core::Vector3;
//!
//! let mut plan = FacilityPlan::new();
//! plan.add(Facility {
//!     name: "Goliath crane".into(),
//!     kind: FacilityKind::Crane,
//!     capacity: 1_200.0,
//!     position: Vector3::new(0.0, 0.0, 0.0),
//!     footprint_m: (30.0, 30.0),
//!     height_m: 40.0,
//! });
//! // A demand of 900 t of crane work fits; 1,500 t does not.
//! assert!(plan.check_capacity(FacilityKind::Crane, 900.0));
//! assert!(!plan.check_capacity(FacilityKind::Crane, 1_500.0));
//! ```
#![allow(clippy::doc_markdown)]

use std::fmt;

use tpt_yard_core::Vector3;

/// Facility categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FacilityKind {
    /// Gantry/jib crane (capacity in tonnes).
    Crane,
    /// Panel/sub-assembly workshop (area in m²).
    Workshop,
    /// Building dock or slipway (berth slots).
    Drydock,
    /// Outfitting quay (berth slots).
    OutfittingQuay,
    /// Orbital assembly bay (slots).
    OrbitalBay,
    /// Additive manufacturing cell (cells).
    ManufacturingCell,
}

impl FacilityKind {
    /// Unit of the capacity figure.
    pub fn unit(self) -> &'static str {
        match self {
            FacilityKind::Crane => "t",
            FacilityKind::Workshop => "m2",
            FacilityKind::Drydock | FacilityKind::OutfittingQuay | FacilityKind::OrbitalBay => {
                "slots"
            }
            FacilityKind::ManufacturingCell => "cells",
        }
    }
}

/// A placed facility.
#[derive(Debug, Clone, PartialEq)]
pub struct Facility {
    /// Name.
    pub name: String,
    /// Kind.
    pub kind: FacilityKind,
    /// Capacity (units per [`FacilityKind::unit`]).
    pub capacity: f64,
    /// Centre position, m (z matters: stacked or elevated facilities do
    /// not collide with ground-level ones).
    pub position: Vector3,
    /// Plan-view footprint (width, depth), m.
    pub footprint_m: (f64, f64),
    /// Vertical extent, m (0 for ground-level pads).
    pub height_m: f64,
}

/// Errors from facility planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FacilityError {
    /// Two facilities overlap beyond their clearance.
    Overlap {
        /// First facility.
        a: String,
        /// Second facility.
        b: String,
    },
}

impl fmt::Display for FacilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FacilityError::Overlap { a, b } => write!(f, "facilities '{a}' and '{b}' overlap"),
        }
    }
}

impl std::error::Error for FacilityError {}

/// The facility plan.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FacilityPlan {
    /// Facilities in the plan.
    pub facilities: Vec<Facility>,
}

impl FacilityPlan {
    /// Creates an empty plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a facility.
    pub fn add(&mut self, facility: Facility) {
        self.facilities.push(facility);
    }

    /// Capacity of a kind.
    ///
    /// Cranes do **not** aggregate: two cranes of 1200 t and 50 t cannot
    /// jointly lift a 600 t unit (a single pick runs on one crane; tandem
    /// lifts need an explicit engineering case). For cranes this returns
    /// the largest single capacity; for workshops, docks and bays — where
    /// demand genuinely pools — it returns the sum.
    pub fn capacity_of(&self, kind: FacilityKind) -> f64 {
        let capacities: Vec<f64> = self
            .facilities
            .iter()
            .filter(|f| f.kind == kind)
            .map(|f| f.capacity)
            .collect();
        if kind == FacilityKind::Crane {
            capacities.iter().cloned().fold(0.0, f64::max)
        } else {
            capacities.iter().sum()
        }
    }

    /// Capacity check: does peak demand of a kind fit?
    pub fn check_capacity(&self, kind: FacilityKind, peak_demand: f64) -> bool {
        self.capacity_of(kind) >= peak_demand
    }

    /// True if `candidate` fits without overlapping an existing facility
    /// (axis-aligned footprint check with clearance).
    pub fn can_place(&self, candidate: &Facility, clearance_m: f64) -> bool {
        self.facilities
            .iter()
            .all(|existing| !overlaps(candidate, existing, clearance_m))
    }

    /// Validates the whole plan: no facility pair overlaps.
    ///
    /// # Errors
    ///
    /// [`FacilityError::Overlap`] naming the first offending pair.
    pub fn validate(&self, clearance_m: f64) -> Result<(), FacilityError> {
        for i in 0..self.facilities.len() {
            for j in i + 1..self.facilities.len() {
                let (a, b) = (&self.facilities[i], &self.facilities[j]);
                if overlaps(a, b, clearance_m) {
                    return Err(FacilityError::Overlap {
                        a: a.name.clone(),
                        b: b.name.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

fn overlaps(a: &Facility, b: &Facility, clearance: f64) -> bool {
    let (aw, ad) = a.footprint_m;
    let (bw, bd) = b.footprint_m;
    let dx = (a.position.x - b.position.x).abs();
    let dy = (a.position.y - b.position.y).abs();
    // Vertical separation counts too: a platform 10 m above a pad is not
    // in conflict with it.
    let dz = (a.position.z - b.position.z).abs();
    let vz = (a.height_m + b.height_m) / 2.0;
    dx < (aw + bw) / 2.0 + clearance
        && dy < (ad + bd) / 2.0 + clearance
        && dz < vz + clearance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> FacilityPlan {
        let mut plan = FacilityPlan::new();
        plan.add(Facility {
            name: "Goliath 1".into(),
            kind: FacilityKind::Crane,
            capacity: 1_200.0,
            position: Vector3::new(0.0, 0.0, 0.0),
            footprint_m: (30.0, 30.0),
            height_m: 40.0,
        });
        plan.add(Facility {
            name: "Workshop A".into(),
            kind: FacilityKind::Workshop,
            capacity: 5_000.0,
            position: Vector3::new(200.0, 0.0, 0.0),
            footprint_m: (120.0, 60.0),
            height_m: 12.0,
        });
        plan.add(Facility {
            name: "Dock 1".into(),
            kind: FacilityKind::Drydock,
            capacity: 2.0,
            position: Vector3::new(0.0, 300.0, 0.0),
            footprint_m: (200.0, 60.0),
            height_m: 0.0,
        });
        plan
    }

    #[test]
    fn capacity_checks() {
        let plan = plan();
        assert_eq!(plan.capacity_of(FacilityKind::Crane), 1_200.0);
        assert!(plan.check_capacity(FacilityKind::Crane, 900.0));
        assert!(!plan.check_capacity(FacilityKind::Crane, 1_500.0));
        // Two dock slots: one vessel in dock, a second fits, a third waits.
        assert!(plan.check_capacity(FacilityKind::Drydock, 2.0));
        assert!(!plan.check_capacity(FacilityKind::Drydock, 3.0));

        // Regression (review 7B): crane capacities do not aggregate — a
        // second small crane cannot help lift a single heavy unit, while
        // workshop area does pool.
        let mut plan = FacilityPlan::new();
        plan.add(Facility {
            name: "Goliath".into(),
            kind: FacilityKind::Crane,
            capacity: 800.0,
            position: Vector3::ZERO,
            footprint_m: (30.0, 30.0),
            height_m: 40.0,
        });
        plan.add(Facility {
            name: "Jib".into(),
            kind: FacilityKind::Crane,
            capacity: 200.0,
            position: Vector3::new(100.0, 0.0, 0.0),
            footprint_m: (10.0, 10.0),
            height_m: 12.0,
        });
        assert_eq!(plan.capacity_of(FacilityKind::Crane), 800.0);
        assert!(!plan.check_capacity(FacilityKind::Crane, 900.0),
            "800 t + 200 t cranes cannot make a 900 t single pick");
        plan.add(Facility {
            name: "W1".into(),
            kind: FacilityKind::Workshop,
            capacity: 3_000.0,
            position: Vector3::new(200.0, 0.0, 0.0),
            footprint_m: (100.0, 50.0),
            height_m: 12.0,
        });
        plan.add(Facility {
            name: "W2".into(),
            kind: FacilityKind::Workshop,
            capacity: 2_000.0,
            position: Vector3::new(320.0, 0.0, 0.0),
            footprint_m: (100.0, 50.0),
            height_m: 12.0,
        });
        assert_eq!(plan.capacity_of(FacilityKind::Workshop), 5_000.0,
            "workshop area pools");
    }

    /// Regression (review 7B): facilities at different elevations do not
    /// conflict in plan view.
    #[test]
    fn vertical_separation_avoids_conflict() {
        let mut plan = FacilityPlan::new();
        plan.add(Facility {
            name: "pad".into(),
            kind: FacilityKind::Workshop,
            capacity: 1.0,
            position: Vector3::ZERO,
            footprint_m: (50.0, 50.0),
            height_m: 0.0,
        });
        let elevated = Facility {
            name: "platform".into(),
            kind: FacilityKind::OrbitalBay,
            capacity: 1.0,
            position: Vector3::ZERO,
            footprint_m: (50.0, 50.0),
            height_m: 5.0,
        };
        // Centred 20 m above the pad: clear of it.
        let mut above = elevated.clone();
        above.position = Vector3::new(0.0, 0.0, 20.0 + 2.5);
        assert!(plan.can_place(&above, 1.0));
        // At the same elevation: clash.
        assert!(!plan.can_place(&elevated, 1.0));
    }

    #[test]
    fn placement_clearance() {
        let plan = plan();
        // Overlapping the crane footprint.
        let bad = Facility {
            name: "clutter".into(),
            kind: FacilityKind::Workshop,
            capacity: 10.0,
            position: Vector3::new(5.0, 5.0, 0.0),
            footprint_m: (20.0, 20.0),
            height_m: 12.0,
        };
        assert!(!plan.can_place(&bad, 2.0));
        // Far enough away.
        let good = Facility {
            position: Vector3::new(500.0, 500.0, 0.0),
            ..bad
        };
        assert!(plan.can_place(&good, 2.0));
    }

    #[test]
    fn validate_detects_overlaps() {
        let mut plan = plan();
        plan.add(Facility {
            name: "Workshop B".into(),
            kind: FacilityKind::Workshop,
            capacity: 1.0,
            position: Vector3::new(210.0, 0.0, 0.0),
            footprint_m: (120.0, 60.0),
            height_m: 12.0,
        });
        // Workshop B overlaps Workshop A (200 vs 210 centres, 120 wide).
        assert_eq!(
            plan.validate(2.0),
            Err(FacilityError::Overlap {
                a: "Workshop A".into(),
                b: "Workshop B".into(),
            })
        );
    }

    #[test]
    fn clean_plan_validates() {
        let plan = plan();
        assert_eq!(plan.validate(2.0), Ok(()));
    }
}
