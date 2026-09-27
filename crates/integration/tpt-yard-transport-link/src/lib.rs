//! Bridge between vehicle design (`tpt-transport`) and shipyard
//! construction.
//!
//! The `tpt-transport` substrate is not published yet, so this crate
//! vendors the minimal design-side types ([`VehicleDesign`],
//! [`AsBuiltProperties`]) behind the same shapes the substrate will use —
//! the swap is mechanical when it lands (the design sketch in the spec §6
//! is the contract).
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::{ConstructionMethod, VesselType};
//! use tpt_yard_transport_link::{plan_construction, DesignKind, VehicleDesign};
//!
//! let design = VehicleDesign {
//!     name: "Container ship 1400 TEU".into(),
//!     kind: DesignKind::Maritime {
//!         loa_m: 140.0,
//!         deadweight_t: 18_500.0,
//!     },
//! };
//! let project = plan_construction(&design);
//! assert!(matches!(project.vessel_type, VesselType::Sea(_)));
//! assert_eq!(project.construction_method, ConstructionMethod::SeaDrydock);
//! ```
#![allow(clippy::doc_markdown)]

use tpt_yard_core::{
    BuildPhase, ConstructionMethod, PhaseId, ProjectId, SeaVesselType, SpaceVesselType, Vector3,
    VesselProject, VesselType,
};
use tpt_yard_digital_twin::DigitalTwin;

/// Accepted quality records handed to operations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QualityRecords {
    /// Total records.
    pub total: usize,
    /// Accepted records.
    pub accepted: usize,
}

/// The design-side vehicle description (stand-in for
/// `tpt_transport::core::Vehicle`).
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleDesign {
    /// Design name.
    pub name: String,
    /// The vehicle family.
    pub kind: DesignKind,
}

/// Vehicle families on the design side.
#[derive(Debug, Clone, PartialEq)]
pub enum DesignKind {
    /// Maritime vessel.
    Maritime {
        /// Length overall, m.
        loa_m: f64,
        /// Deadweight, tonnes.
        deadweight_t: f64,
    },
    /// Spacecraft / space structure.
    Spacecraft {
        /// Launch (assembled) mass, kg.
        mass_kg: f64,
        /// True if assembled in orbit rather than launched complete.
        assembled_in_orbit: bool,
    },
}

/// As-built properties handed over to operations (stand-in for
/// `tpt_transport::core::AsBuiltProperties`, spec §6).
#[derive(Debug, Clone, PartialEq)]
pub struct AsBuiltProperties {
    /// As-built lightship weight, kg.
    pub weight_kg: f64,
    /// As-built centre of gravity, m.
    pub cog: Vector3,
    /// As-built structural state summary.
    pub structural_state: String,
    /// Accepted quality records.
    pub quality_records: QualityRecords,
}

/// Maps a design to a construction plan: vessel type and construction
/// method follow the design family; the build-phase skeleton is created
/// with one placeholder phase per construction stage for the yard to fill.
pub fn plan_construction(design: &VehicleDesign) -> VesselProject {
    match &design.kind {
        DesignKind::Maritime {
            loa_m,
            deadweight_t,
        } => {
            let vessel_type = VesselType::Sea(SeaVesselType::ContainerShip {
                teu_capacity: (deadweight_t / 10.0) as u32,
            });
            let method = ConstructionMethod::SeaDrydock;
            let phases = vec![
                BuildPhase::new(PhaseId(1), "Steel prefabrication", *loa_m / 5.0),
                BuildPhase::new(PhaseId(2), "Block assembly", *loa_m / 4.0),
                BuildPhase::new(PhaseId(3), "Dock erection", *loa_m / 4.0),
                BuildPhase::new(PhaseId(4), "Outfitting", *loa_m / 3.0),
                BuildPhase::new(PhaseId(5), "Launch & trials", *loa_m / 10.0),
            ];
            VesselProject::new(
                ProjectId(1),
                design.name.clone(),
                vessel_type,
                method,
                phases,
            )
            .expect("non-empty phase plan")
        }
        DesignKind::Spacecraft {
            mass_kg: _,
            assembled_in_orbit,
        } => {
            let vessel_type = VesselType::Space(SpaceVesselType::SpaceStation { modules: 4 });
            let method = if *assembled_in_orbit {
                ConstructionMethod::OrbitalAssembly
            } else {
                ConstructionMethod::InSpaceManufacturing
            };
            let phases = vec![
                BuildPhase::new(PhaseId(1), "Component fabrication", 60.0),
                BuildPhase::new(PhaseId(2), "Launch & rendezvous", 10.0),
                BuildPhase::new(PhaseId(3), "Orbital assembly", 90.0),
                BuildPhase::new(PhaseId(4), "Verification", 20.0),
            ];
            VesselProject::new(
                ProjectId(1),
                design.name.clone(),
                vessel_type,
                method,
                phases,
            )
            .expect("non-empty phase plan")
        }
    }
}

/// Hands the completed twin over to operations: as-built weight and CoG,
/// structural summary, and accepted quality records (spec §6 contract).
pub fn handover_to_operations(twin: &DigitalTwin) -> AsBuiltProperties {
    AsBuiltProperties {
        weight_kg: twin.weight_model().installed_weight(),
        cog: twin
            .weight_model()
            .installed_centre_of_gravity()
            .unwrap_or(Vector3::ZERO),
        structural_state: format!(
            "{:.0}% of {} structural activities connected",
            twin.structural_model.connected_fraction * 100.0,
            twin.structural_model.member_count
        ),
        quality_records: QualityRecords {
            total: twin.quality_records.len(),
            accepted: twin.quality_records.iter().filter(|r| r.accepted).count(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_assembly::ActivityId;
    use tpt_yard_core::{AssemblyActivity, ItemId};
    use tpt_yard_digital_twin::SupportCondition;
    use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

    /// Round-trip: design → build → as-built handover.
    #[test]
    fn round_trip_design_build_handover() {
        // 1. Design a small ship.
        let design = VehicleDesign {
            name: "Handover test ship".into(),
            kind: DesignKind::Maritime {
                loa_m: 100.0,
                deadweight_t: 12_000.0,
            },
        };
        let project = plan_construction(&design);
        assert_eq!(project.build_phases.len(), 5);

        // 2. Fill the twin with one activity + item per phase and advance.
        let mut project = project;
        let mut weight = WeightModel::new(5_000.0, Vector3::new(50.0, 0.0, 6.0));
        for (idx, phase) in project.build_phases.iter_mut().enumerate() {
            let activity_id = ActivityId(idx as u64 + 1);
            phase.activities.push(AssemblyActivity::new(
                activity_id,
                phase.name.clone(),
                tpt_yard_core::ActivityType::JoinBlock,
                8.0,
            ));
            weight.add_item(WeightItem {
                id: ItemId(idx as u64 + 1),
                name: phase.name.clone(),
                group: "hull".into(),
                weight_kg: 1_000.0,
                cog: Vector3::new(20.0 * (idx + 1) as f64, 0.0, 6.0),
                status: ItemStatus::Design,
                margin_pct: 0.0,
                installed_by: Some(activity_id),
            });
        }
        let project = project;
        let mut twin = DigitalTwin::with_weight_model(
            project,
            weight,
            SupportCondition::KeelBlocks {
                positions: vec![Vector3::new(-10.0, 0.0, 0.0), Vector3::new(150.0, 0.0, 0.0)],
            },
        );
        for idx in 0..5u64 {
            twin.advance_phase(&ActivityId(idx + 1))
                .expect("each phase advances");
        }

        // 3. Hand over: as-built weight equals the installed total.
        let as_built = handover_to_operations(&twin);
        assert_eq!(as_built.weight_kg, 5_000.0);
        // CoG: weighted mean of items at x = 20, 40, ..., 100 -> 60.
        assert!((as_built.cog.x - 60.0).abs() < 1e-9);
        assert!(as_built.structural_state.contains("100%"));
        assert_eq!(as_built.quality_records.total, 0);
    }

    #[test]
    fn spacecraft_maps_to_orbital_assembly() {
        let design = VehicleDesign {
            name: "Orbital station".into(),
            kind: DesignKind::Spacecraft {
                mass_kg: 400_000.0,
                assembled_in_orbit: true,
            },
        };
        let project = plan_construction(&design);
        assert!(matches!(project.vessel_type, VesselType::Space(_)));
        assert_eq!(
            project.construction_method,
            ConstructionMethod::OrbitalAssembly
        );
    }
}
