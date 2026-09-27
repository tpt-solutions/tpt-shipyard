//! Fundamental shipyard domain types for the TPT Shipyard construction
//! engine.
//!
//! `tpt-yard-core` defines *what* is being built ([`VesselProject`],
//! [`VesselType`], [`BuildPhase`], [`AssemblyActivity`]) and the primitives
//! everything else shares: 3D geometry ([`Vector3`], [`Geometry3D`]), mass
//! properties, identifiers, and a dependency-free JSON implementation
//! ([`json`]).
//!
//! # Relationship to the digital twin
//!
//! The twin lives in `tpt-yard-digital-twin` and **owns** a `VesselProject`;
//! the project deliberately carries no back-reference (see RFC 0001). This
//! breaks what would otherwise be a circular dependency between the two
//! crates.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::json::Value;
//! use tpt_yard_core::*;
//!
//! let mut phase = BuildPhase::new(PhaseId(1), "Erection", 10.0);
//! phase.activities.push(AssemblyActivity::new(
//!     ActivityId(1),
//!     "Erect block 212",
//!     ActivityType::JoinBlock,
//!     8.0,
//! ));
//! let project = VesselProject::new(
//!     ProjectId(1),
//!     "Container ship 14k TEU",
//!     VesselType::Sea(SeaVesselType::ContainerShip { teu_capacity: 14_000 }),
//!     ConstructionMethod::SeaDrydock,
//!     vec![phase],
//! )
//! .unwrap();
//!
//! // Round-trips through the dependency-free JSON module.
//! let json = project.to_json().to_string_compact();
//! assert_eq!(VesselProject::from_json_str(&json).unwrap(), project);
//! ```

pub mod geometry;
pub mod ids;
pub mod json;
pub mod material;
pub mod phase;
pub mod project;

pub use geometry::{BoundingBox, Dimensions, Geometry3D, MassProperties, Vector3};
pub use ids::{BlockId, ComponentId, ItemId, PhaseId, ProjectId, RobotId, StepId, WeldId};
pub use material::Material;
pub use phase::{
    ActivityType, AssemblyActivity, BuildPhase, FluidType, OutfitSystem, Resource, ResourceKind,
    StructuralState, TestType, WeightState,
};
pub use project::{
    ConstructionMethod, HullType, PropulsionType, SeaVesselType, SpaceVesselType, VesselProject,
    VesselType,
};
pub use tpt_yard_assembly::{ActivityId, ActivityStatus};

use std::fmt;

/// Errors produced when (de)serializing core types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Malformed JSON text.
    Json(String),
    /// A required field is absent.
    MissingField(String),
    /// A field is present with the wrong shape or an unknown variant.
    TypeError(String),
    /// File I/O failure while loading a project.
    Io(String),
}

impl CoreError {
    /// Convenience constructor for [`CoreError::MissingField`].
    pub fn missing_field(name: impl Into<String>) -> Self {
        CoreError::MissingField(name.into())
    }

    /// Convenience constructor for [`CoreError::TypeError`].
    pub fn type_error(msg: impl Into<String>) -> Self {
        CoreError::TypeError(msg.into())
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Json(m) => write!(f, "json: {m}"),
            CoreError::MissingField(m) => write!(f, "missing field: {m}"),
            CoreError::TypeError(m) => write!(f, "type error: {m}"),
            CoreError::Io(m) => write!(f, "io: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<crate::json::JsonError> for CoreError {
    fn from(e: crate::json::JsonError) -> Self {
        CoreError::Json(e.0)
    }
}
