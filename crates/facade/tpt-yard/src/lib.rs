//! `tpt-yard` — the one-dependency facade for the tpt-shipyard workspace.
//!
//! Each domain crate stays individually publishable and dependency-light;
//! this crate exists for everyone who just wants the engine: it
//! re-exposes every sub-crate as a module and curates a [`prelude`] with
//! the types a construction planner touches first.
//!
//! Feature flags keep the compile lean: `sea`, `space`, and `planning`
//! gate their layers (all on by default), and `wasm` opts into the
//! browser-facing bindings.
//!
//! # Example
//!
//! ```
//! use tpt_yard::prelude::*;
//!
//! let mut phase = BuildPhase::new(PhaseId(1), "Erection", 4.0);
//! phase.activities.push(
//!     AssemblyActivity::new(ActivityId(1), "Erect block 1", ActivityType::JoinBlock, 6.0),
//! );
//! let project = VesselProject::new(
//!     ProjectId(1),
//!     "Facade demo",
//!     VesselType::Sea(SeaVesselType::ContainerShip { teu_capacity: 800 }),
//!     ConstructionMethod::SeaDrydock,
//!     vec![phase],
//! )
//! .unwrap();
//! let mut twin = DigitalTwin::new(project);
//! twin.advance_phase(&ActivityId(1)).unwrap();
//! assert_eq!(twin.assembly_state.completed_activities.len(), 1);
//! ```

pub mod export;

pub use tpt_yard_assembly;
pub use tpt_yard_core;
pub use tpt_yard_digital_twin;
pub use tpt_yard_distortion;
pub use tpt_yard_earth_link;
pub use tpt_yard_joints;
pub use tpt_yard_process_link;
pub use tpt_yard_structural;
pub use tpt_yard_transport_link;
pub use tpt_yard_weight;
pub use tpt_yard_welding;

#[cfg(feature = "sea")]
pub use tpt_yard_blocks;
#[cfg(feature = "sea")]
pub use tpt_yard_drydock;
#[cfg(feature = "sea")]
pub use tpt_yard_hull;
#[cfg(feature = "sea")]
pub use tpt_yard_launch;
#[cfg(feature = "sea")]
pub use tpt_yard_outfitting;
#[cfg(feature = "sea")]
pub use tpt_yard_sea_trials;

#[cfg(feature = "space")]
pub use tpt_yard_habitat;
#[cfg(feature = "space")]
pub use tpt_yard_orbital_assembly;
#[cfg(feature = "space")]
pub use tpt_yard_propellant;
#[cfg(feature = "space")]
pub use tpt_yard_robotic_assembly;
#[cfg(feature = "space")]
pub use tpt_yard_space_manufacturing;
#[cfg(feature = "space")]
pub use tpt_yard_space_structural;

#[cfg(feature = "planning")]
pub use tpt_yard_facility;
#[cfg(feature = "planning")]
pub use tpt_yard_logistics;
#[cfg(feature = "planning")]
pub use tpt_yard_quality;
#[cfg(feature = "planning")]
pub use tpt_yard_scheduling;

#[cfg(feature = "wasm")]
pub use tpt_yard_wasm;

/// The curated façade: the types a construction plan is written with.
///
/// ```rust
/// use tpt_yard::prelude::*;
/// // Every import below comes from the prelude alone.
/// let phase = BuildPhase::new(PhaseId(1), "Only phase", 2.0);
/// let project = VesselProject::new(
///     ProjectId(2),
///     "prelude check",
///     VesselType::Space(SpaceVesselType::SpaceStation { modules: 1 }),
///     ConstructionMethod::OrbitalAssembly,
///     vec![phase],
/// )
/// .unwrap();
/// assert_eq!(project.build_phases.len(), 1);
/// ```
pub mod prelude {
    // Core plan model.
    pub use tpt_yard_core::{
        ActivityType, AssemblyActivity, BuildPhase, ComponentId, ConstructionMethod, Geometry3D,
        MassProperties, Material, PhaseId, ProjectId, SeaVesselType, SpaceVesselType, Vector3,
        VesselProject, VesselType,
    };
    // Activity network.
    pub use tpt_yard_assembly::{ActivityGraph, ActivityId, ActivityStatus};
    // Weight and the twin.
    pub use tpt_yard_digital_twin::{DigitalTwin, SupportCondition};
    pub use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};
    // Construction-phase structural analysis and welding.
    pub use tpt_yard_structural::{ConstructionLoad, ConstructionStructuralSolver};
    pub use tpt_yard_welding::{WeldProcedure, WeldProcess, WeldingSimulation};

    #[cfg(feature = "sea")]
    pub use tpt_yard_hull::{HullConstruction, HullGeometry};
    #[cfg(feature = "sea")]
    pub use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};

    #[cfg(feature = "space")]
    pub use tpt_yard_habitat::{HabitatDesigner, HabitatType};
    #[cfg(feature = "space")]
    pub use tpt_yard_orbital_assembly::{OrbitalAssembly, OrbitalParameters, SpaceStructure};
    #[cfg(feature = "space")]
    pub use tpt_yard_robotic_assembly::{EndEffector, Joint, RoboticArm};

    #[cfg(feature = "planning")]
    pub use tpt_yard_quality::{NdtMethod, QualityManagement};
    #[cfg(feature = "planning")]
    pub use tpt_yard_scheduling::{ScheduleObjective, ShipyardScheduler};
}

#[cfg(test)]
mod tests {
    #[test]
    fn feature_gates_are_consistent() {
        // The default build carries all three domain layers. (`if` +
        // `panic!` rather than `assert!`: the conditions are compile-time
        // constants and clippy rightly objects to asserting constants.)
        if !cfg!(feature = "sea") {
            panic!("default build must carry the sea layer");
        }
        if !cfg!(feature = "space") {
            panic!("default build must carry the space layer");
        }
        if !cfg!(feature = "planning") {
            panic!("default build must carry the planning layer");
        }
        if cfg!(feature = "wasm") {
            panic!("wasm is opt-in, not a default feature");
        }
    }
}
