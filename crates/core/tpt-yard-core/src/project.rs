//! Vessel projects: what is being built, how, and through which phases.
//!
//! The centrepiece is [`VesselProject`]. Note that it does **not** embed a
//! [`digital twin`](https://docs.rs/tpt-yard-digital-twin): the twin owns the
//! project, not the other way around — see RFC 0001 for that decision.

use crate::ids::ProjectId;
use crate::json::Value;
use crate::phase::BuildPhase;
use crate::{CoreError, PhaseId};
use tpt_yard_assembly::{ActivityGraph, ActivityId, GraphError};

/// What kind of vessel is being constructed.
#[derive(Debug, Clone, PartialEq)]
pub enum VesselType {
    /// A sea-going vessel.
    Sea(SeaVesselType),
    /// A space vessel / structure.
    Space(SpaceVesselType),
    /// Both worlds: e.g. a space-rated pressure hull tested at sea.
    Hybrid,
}

/// Categories of sea-going vessels.
#[derive(Debug, Clone, PartialEq)]
pub enum SeaVesselType {
    /// Container ship with nominal TEU capacity.
    ContainerShip {
        /// Twenty-foot-equivalent unit capacity.
        teu_capacity: u32,
    },
    /// Tanker with deadweight.
    Tanker {
        /// Deadweight tonnage.
        deadweight_tonnes: f64,
    },
    /// LNG carrier with cargo capacity.
    LngCarrier {
        /// Cargo volume, m³.
        cargo_volume_m3: f64,
    },
    /// Cruise ship with passenger capacity.
    CruiseShip {
        /// Passenger capacity.
        passengers: u32,
    },
    /// Naval vessel with classification.
    NavalVessel {
        /// Classification string (e.g. "frigate").
        classification: String,
    },
    /// Submarine with hull type.
    Submarine {
        /// Single or double hull.
        hull_type: HullType,
    },
    /// Offshore support vessel with class.
    OffshoreVessel {
        /// Vessel class string.
        vessel_class: String,
    },
    /// Fishing vessel.
    FishingVessel,
}

/// Categories of space vessels and structures.
#[derive(Debug, Clone, PartialEq)]
pub enum SpaceVesselType {
    /// Space station with module count.
    SpaceStation {
        /// Number of pressurized modules.
        modules: u32,
    },
    /// Rotating artificial-gravity habitat.
    OrbitalHabitat {
        /// Rotation rate, rpm.
        rotation_rpm: f64,
        /// Rotation radius, m.
        radius_m: f64,
    },
    /// Solar power station with collecting area.
    SolarPowerStation {
        /// Array area, m².
        array_area_m2: f64,
    },
    /// Deep-space vessel with a propulsion type.
    DeepSpaceVessel {
        /// Propulsion system.
        propulsion: PropulsionType,
    },
    /// Lunar surface vehicle.
    LunarVehicle,
    /// Mars surface vehicle.
    MarsVehicle,
    /// Orbital factory.
    OrbitalFactory,
    /// Cryogenic propellant depot.
    FuelDepot,
}

/// Submarine hull configurations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HullType {
    /// Single pressure hull.
    SingleHull,
    /// Pressure hull inside an outer (light) hull.
    DoubleHull,
}

/// Spacecraft propulsion families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropulsionType {
    /// Chemical (bipropellant).
    ChemicalBiPropellant,
    /// Chemical (monopropellant).
    ChemicalMonoPropellant,
    /// Solid motors.
    Solid,
    /// Nuclear thermal.
    NuclearThermal,
    /// Nuclear electric.
    NuclearElectric,
    /// Solar electric (ion).
    SolarElectric,
    /// Solar sail.
    SolarSail,
}

/// How the vessel will be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionMethod {
    /// Sea: block construction in workshop + dock erection.
    SeaBlockConstruction,
    /// Sea: building on a sloping slipway, end launch.
    SeaSlipwayLaunch,
    /// Sea: construction inside a drydock, float-out.
    SeaDrydock,
    /// Space: assembly of prefabricated modules in orbit.
    OrbitalAssembly,
    /// Space: manufacturing structure in space (additive, wire feed, ...).
    InSpaceManufacturing,
    /// Surface construction on the Moon.
    LunarSurfaceConstruction,
}

/// The complete build plan of a vessel under construction.
#[derive(Debug, Clone, PartialEq)]
pub struct VesselProject {
    /// Project identifier.
    pub id: ProjectId,
    /// Project name.
    pub name: String,
    /// What is being built.
    pub vessel_type: VesselType,
    /// How it is being built.
    pub construction_method: ConstructionMethod,
    /// Ordered build phases.
    pub build_phases: Vec<BuildPhase>,
    /// The phase currently in progress.
    pub current_phase: PhaseId,
}

impl VesselProject {
    /// Creates a project with `current_phase` set to the first phase.
    ///
    /// Returns `None` if `build_phases` is empty.
    pub fn new(
        id: ProjectId,
        name: impl Into<String>,
        vessel_type: VesselType,
        construction_method: ConstructionMethod,
        build_phases: Vec<BuildPhase>,
    ) -> Option<Self> {
        let current_phase = build_phases.first()?.id;
        Some(Self {
            id,
            name: name.into(),
            vessel_type,
            construction_method,
            build_phases,
            current_phase,
        })
    }

    /// Look up a phase by id.
    pub fn phase(&self, id: PhaseId) -> Option<&BuildPhase> {
        self.build_phases.iter().find(|p| p.id == id)
    }

    /// Checks the project for internal consistency.
    ///
    /// # Errors
    ///
    /// [`CoreError::Validation`] describing the first inconsistency found:
    /// duplicate phase/activity ids, a dependency that does not resolve,
    /// `current_phase` not naming a phase of this project, empty phase
    /// activity lists, non-finite or negative numbers, or installed weight
    /// above design.
    pub fn validate(&self) -> Result<(), CoreError> {
        let err = |msg: String| Err(CoreError::Validation(msg));

        if self.name.trim().is_empty() {
            return err("project name is empty".into());
        }
        if self.build_phases.is_empty() {
            return err("project has no build phases".into());
        }
        if !self.build_phases.iter().any(|p| p.id == self.current_phase) {
            return err(format!(
                "current_phase {:?} is not one of the project's phases",
                self.current_phase
            ));
        }

        let mut phase_ids = std::collections::BTreeSet::new();
        let mut activity_ids = std::collections::BTreeSet::new();
        let mut dependencies: Vec<(ActivityId, Vec<ActivityId>)> = Vec::new();
        for phase in &self.build_phases {
            if !phase_ids.insert(phase.id) {
                return err(format!("duplicate phase id {:?}", phase.id));
            }
            if !(phase.duration_days >= 0.0) || !phase.duration_days.is_finite() {
                return err(format!(
                    "phase {:?} has non-finite or negative duration {}",
                    phase.id, phase.duration_days
                ));
            }
            if !(phase.weight_state.design_kg >= 0.0)
                || !phase.weight_state.design_kg.is_finite()
                || !(phase.weight_state.installed_kg >= 0.0)
                || !phase.weight_state.installed_kg.is_finite()
            {
                return err(format!(
                    "phase {:?} has non-finite or negative weight state",
                    phase.id
                ));
            }
            if phase.weight_state.installed_kg > phase.weight_state.design_kg + 1e-9 {
                return err(format!("phase {:?} installed kg exceeds design kg", phase.id));
            }
            if phase.activities.is_empty() {
                return err(format!("phase {:?} has no activities", phase.id));
            }
            for a in &phase.activities {
                if !activity_ids.insert(a.id) {
                    return err(format!("duplicate activity id {}", a.id));
                }
                if !(a.duration_hours > 0.0) || !a.duration_hours.is_finite() {
                    return err(format!(
                        "activity {} has non-finite or non-positive duration",
                        a.id
                    ));
                }
                dependencies.push((a.id, a.dependencies.clone()));
            }
        }
        for (id, deps) in dependencies {
            for d in &deps {
                if !activity_ids.contains(d) {
                    return err(format!("activity {} depends on unknown activity {}", id, d));
                }
            }
        }
        Ok(())
    }

    /// Index of a phase in `build_phases`.
    pub fn phase_index(&self, id: PhaseId) -> Option<usize> {
        self.build_phases.iter().position(|p| p.id == id)
    }

    /// All activities of all phases as a dependency graph.
    ///
    /// # Errors
    ///
    /// [`GraphError`] if the declared dependency network is malformed
    /// (duplicate activity id, unknown reference or cycle).
    pub fn activity_graph(&self) -> Result<ActivityGraph, GraphError> {
        let mut g = ActivityGraph::new();
        for phase in &self.build_phases {
            for a in &phase.activities {
                g.add_activity(a.id, a.name.clone(), a.duration_hours, &a.dependencies)?;
            }
        }
        g.validate()?;
        Ok(g)
    }

    /// True when `current_phase` is the last phase of the plan. Activity-level
    /// completion is tracked by the digital twin's [`AssemblyState`]
    /// (`tpt-yard-digital-twin`), not by the project itself.
    pub fn is_in_final_phase(&self) -> bool {
        self.current_phase == self.build_phases.last().map(|p| p.id).unwrap_or_default()
    }

    // ------------------------------------------------------------------- JSON

    /// Wire-format schema version of the serialized project (bumped on
    /// breaking JSON changes; see `schemas/vessel-project.schema.json`).
    pub const SCHEMA_VERSION: u64 = 1;

    /// Serializes to a JSON value.
    pub fn to_json(&self) -> Value {
        let vessel = |k: &str, extra: Vec<(String, Value)>| -> Value {
            let mut o = vec![("kind".to_string(), Value::String(k.to_string()))];
            o.extend(extra);
            Value::Object(o)
        };
        let vessel_type = match &self.vessel_type {
            VesselType::Sea(sea) => {
                let (k, extra) = match sea {
                    SeaVesselType::ContainerShip { teu_capacity } => (
                        "container_ship",
                        vec![(
                            "teu_capacity".to_string(),
                            Value::Number(*teu_capacity as f64),
                        )],
                    ),
                    SeaVesselType::Tanker { deadweight_tonnes } => (
                        "tanker",
                        vec![(
                            "deadweight_tonnes".to_string(),
                            Value::Number(*deadweight_tonnes),
                        )],
                    ),
                    SeaVesselType::LngCarrier { cargo_volume_m3 } => (
                        "lng_carrier",
                        vec![(
                            "cargo_volume_m3".to_string(),
                            Value::Number(*cargo_volume_m3),
                        )],
                    ),
                    SeaVesselType::CruiseShip { passengers } => (
                        "cruise_ship",
                        vec![("passengers".to_string(), Value::Number(*passengers as f64))],
                    ),
                    SeaVesselType::NavalVessel { classification } => (
                        "naval_vessel",
                        vec![(
                            "classification".to_string(),
                            Value::String(classification.clone()),
                        )],
                    ),
                    SeaVesselType::Submarine { hull_type } => (
                        "submarine",
                        vec![(
                            "hull_type".to_string(),
                            Value::String(format!("{hull_type:?}")),
                        )],
                    ),
                    SeaVesselType::OffshoreVessel { vessel_class } => (
                        "offshore_vessel",
                        vec![(
                            "vessel_class".to_string(),
                            Value::String(vessel_class.clone()),
                        )],
                    ),
                    SeaVesselType::FishingVessel => ("fishing_vessel", vec![]),
                };
                vessel("sea", vec![("value".to_string(), vessel(k, extra))])
            }
            VesselType::Space(space) => {
                let (k, extra): (&str, Vec<(String, Value)>) = match space {
                    SpaceVesselType::SpaceStation { modules } => (
                        "space_station",
                        vec![("modules".to_string(), Value::Number(*modules as f64))],
                    ),
                    SpaceVesselType::OrbitalHabitat {
                        rotation_rpm,
                        radius_m,
                    } => (
                        "orbital_habitat",
                        vec![
                            ("rotation_rpm".to_string(), Value::Number(*rotation_rpm)),
                            ("radius_m".to_string(), Value::Number(*radius_m)),
                        ],
                    ),
                    SpaceVesselType::SolarPowerStation { array_area_m2 } => (
                        "solar_power_station",
                        vec![("array_area_m2".to_string(), Value::Number(*array_area_m2))],
                    ),
                    SpaceVesselType::DeepSpaceVessel { propulsion } => (
                        "deep_space_vessel",
                        vec![(
                            "propulsion".to_string(),
                            Value::String(format!("{propulsion:?}")),
                        )],
                    ),
                    SpaceVesselType::LunarVehicle => ("lunar_vehicle", vec![]),
                    SpaceVesselType::MarsVehicle => ("mars_vehicle", vec![]),
                    SpaceVesselType::OrbitalFactory => ("orbital_factory", vec![]),
                    SpaceVesselType::FuelDepot => ("fuel_depot", vec![]),
                };
                vessel("space", vec![("value".to_string(), vessel(k, extra))])
            }
            VesselType::Hybrid => vessel("hybrid", vec![]),
        };
        let method = Value::String(
            match self.construction_method {
                ConstructionMethod::SeaBlockConstruction => "sea_block_construction",
                ConstructionMethod::SeaSlipwayLaunch => "sea_slipway_launch",
                ConstructionMethod::SeaDrydock => "sea_drydock",
                ConstructionMethod::OrbitalAssembly => "orbital_assembly",
                ConstructionMethod::InSpaceManufacturing => "in_space_manufacturing",
                ConstructionMethod::LunarSurfaceConstruction => "lunar_surface_construction",
            }
            .to_string(),
        );
        Value::Object(vec![
        (
            "schema_version".to_string(),
            Value::Number(Self::SCHEMA_VERSION as f64),
        ),

            ("id".into(), Value::Number(self.id.0 as f64)),
            ("name".into(), Value::String(self.name.clone())),
            ("vessel_type".into(), vessel_type),
            ("construction_method".into(), method),
            (
                "build_phases".into(),
                Value::Array(self.build_phases.iter().map(|p| p.to_json()).collect()),
            ),
            (
                "current_phase".into(),
                Value::Number(self.current_phase.0 as f64),
            ),
        ])
    }

    /// Deserializes from a JSON value produced by [`VesselProject::to_json`].
    ///
    /// # Errors
    ///
    /// [`CoreError`] on malformed payloads.
    pub fn from_json_value(v: &Value) -> Result<Self, CoreError> {
        let field = |k: &str| v.get(k).ok_or_else(|| CoreError::missing_field(k));
        let err = |msg: &str| CoreError::type_error(msg.to_string());

        let id = ProjectId(field("id")?.as_u64().ok_or_else(|| err("id must be u64"))?);
        let name = field("name")?
            .as_str()
            .ok_or_else(|| err("name must be a string"))?
            .to_string();

        let vessel_type_v = field("vessel_type")?;
        let kind_tag = vessel_type_v
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("");
        let vessel_type = match kind_tag {
            "hybrid" => VesselType::Hybrid,
            "sea" => {
                let inner = vessel_type_v
                    .get("value")
                    .ok_or_else(|| err("sea vessel needs a value"))?;
                let kind = inner.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                let num = |k: &str| inner.get(k).and_then(|n| n.as_f64());
                VesselType::Sea(match kind {
                    "container_ship" => SeaVesselType::ContainerShip {
                        teu_capacity: num("teu_capacity").unwrap_or(0.0) as u32,
                    },
                    "tanker" => SeaVesselType::Tanker {
                        deadweight_tonnes: num("deadweight_tonnes").unwrap_or(0.0),
                    },
                    "lng_carrier" => SeaVesselType::LngCarrier {
                        cargo_volume_m3: num("cargo_volume_m3").unwrap_or(0.0),
                    },
                    "cruise_ship" => SeaVesselType::CruiseShip {
                        passengers: num("passengers").unwrap_or(0.0) as u32,
                    },
                    "naval_vessel" => SeaVesselType::NavalVessel {
                        classification: inner
                            .get("classification")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string(),
                    },
                    "submarine" => SeaVesselType::Submarine {
                        hull_type: match inner.get("hull_type").and_then(|s| s.as_str()) {
                            Some("DoubleHull") => HullType::DoubleHull,
                            _ => HullType::SingleHull,
                        },
                    },
                    "offshore_vessel" => SeaVesselType::OffshoreVessel {
                        vessel_class: inner
                            .get("vessel_class")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string(),
                    },
                    "fishing_vessel" => SeaVesselType::FishingVessel,
                    other => return Err(err(&format!("unknown sea vessel type '{other}'"))),
                })
            }
            "space" => {
                let inner = vessel_type_v
                    .get("value")
                    .ok_or_else(|| err("space vessel needs a value"))?;
                let kind = inner.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                let num = |k: &str| inner.get(k).and_then(|n| n.as_f64());
                VesselType::Space(match kind {
                    "space_station" => SpaceVesselType::SpaceStation {
                        modules: num("modules").unwrap_or(0.0) as u32,
                    },
                    "orbital_habitat" => SpaceVesselType::OrbitalHabitat {
                        rotation_rpm: num("rotation_rpm").unwrap_or(0.0),
                        radius_m: num("radius_m").unwrap_or(0.0),
                    },
                    "solar_power_station" => SpaceVesselType::SolarPowerStation {
                        array_area_m2: num("array_area_m2").unwrap_or(0.0),
                    },
                    "deep_space_vessel" => SpaceVesselType::DeepSpaceVessel {
                        propulsion: match inner.get("propulsion").and_then(|s| s.as_str()) {
                            Some("NuclearThermal") => PropulsionType::NuclearThermal,
                            Some("NuclearElectric") => PropulsionType::NuclearElectric,
                            Some("SolarElectric") => PropulsionType::SolarElectric,
                            Some("SolarSail") => PropulsionType::SolarSail,
                            Some("Solid") => PropulsionType::Solid,
                            Some("ChemicalMonoPropellant") => {
                                PropulsionType::ChemicalMonoPropellant
                            }
                            _ => PropulsionType::ChemicalBiPropellant,
                        },
                    },
                    "lunar_vehicle" => SpaceVesselType::LunarVehicle,
                    "mars_vehicle" => SpaceVesselType::MarsVehicle,
                    "orbital_factory" => SpaceVesselType::OrbitalFactory,
                    "fuel_depot" => SpaceVesselType::FuelDepot,
                    other => return Err(err(&format!("unknown space vessel type '{other}'"))),
                })
            }
            other => return Err(err(&format!("unknown vessel kind '{other}'"))),
        };

        let method_tag = field("construction_method")?.as_str().unwrap_or("");
        let construction_method = match method_tag {
            "sea_block_construction" => ConstructionMethod::SeaBlockConstruction,
            "sea_slipway_launch" => ConstructionMethod::SeaSlipwayLaunch,
            "sea_drydock" => ConstructionMethod::SeaDrydock,
            "orbital_assembly" => ConstructionMethod::OrbitalAssembly,
            "in_space_manufacturing" => ConstructionMethod::InSpaceManufacturing,
            "lunar_surface_construction" => ConstructionMethod::LunarSurfaceConstruction,
            other => return Err(err(&format!("unknown construction method '{other}'"))),
        };

        let build_phases = field("build_phases")?
            .as_array()
            .ok_or_else(|| err("build_phases must be an array"))?
            .iter()
            .map(BuildPhase::from_json)
            .collect::<Result<Vec<_>, _>>()?;

        let current_phase = PhaseId(
            field("current_phase")?
                .as_u64()
                .ok_or_else(|| err("current_phase must be u64"))?,
        );

        let project = Self {
            id,
            name,
            vessel_type,
            construction_method,
            build_phases,
            current_phase,
        };
        // Loaded projects must be internally consistent — a file with
        // dangling dependencies or non-finite numbers must not enter the
        // engine silently.
        project.validate()?;
        Ok(project)
    }

    /// Deserializes from a JSON string.
    ///
    /// # Errors
    ///
    /// [`CoreError`] on malformed payloads.
    pub fn from_json_str(s: &str) -> Result<Self, CoreError> {
        let v = crate::json::Value::parse(s)?;
        Self::from_json_value(&v)
    }

    /// Loads a project from a JSON file.
    ///
    /// # Errors
    ///
    /// [`CoreError`] on I/O or parse failures.
    pub fn from_json(path: impl AsRef<std::path::Path>) -> Result<Self, CoreError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| CoreError::Io(format!("reading project JSON: {e}")))?;
        Self::from_json_str(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::{ActivityType, AssemblyActivity, WeightState};
    use tpt_yard_assembly::ActivityId;

    /// Helper: a minimal valid project for validation tests.
    fn valid_project() -> VesselProject {
        let mut phase = BuildPhase::new(PhaseId(1), "P1", 3.0);
        phase
            .activities
            .push(crate::AssemblyActivity::new(
                ActivityId(1),
                "a",
                crate::ActivityType::CutSteel,
                4.0,
            ));
        phase.weight_state = super::super::phase::WeightState {
            design_kg: 100.0,
            installed_kg: 0.0,
        };
        VesselProject::new(
            ProjectId(1),
            "test",
            VesselType::Sea(SeaVesselType::FishingVessel),
            ConstructionMethod::SeaDrydock,
            vec![phase],
        )
        .unwrap()
    }

    #[test]
    fn validate_accepts_a_consistent_project() {
        valid_project().validate().unwrap();
    }

    #[test]
    fn validate_rejects_inconsistencies() {
        // Dangling dependency.
        let mut p = valid_project();
        p.build_phases[0].activities[0].dependencies = vec![ActivityId(99)];
        assert!(matches!(
            p.validate(),
            Err(CoreError::Validation(_))
        ));

        // current_phase not a phase of the project.
        let mut p = valid_project();
        p.current_phase = PhaseId(9);
        assert!(matches!(p.validate(), Err(CoreError::Validation(_))));

        // Empty phase.
        let mut p = valid_project();
        p.build_phases[0].activities.clear();
        assert!(matches!(p.validate(), Err(CoreError::Validation(_))));

        // Non-finite duration.
        let mut p = valid_project();
        p.build_phases[0].duration_days = f64::NAN;
        assert!(matches!(p.validate(), Err(CoreError::Validation(_))));

        // Installed weight above design.
        let mut p = valid_project();
        p.build_phases[0].weight_state.installed_kg = 150.0;
        assert!(matches!(p.validate(), Err(CoreError::Validation(_))));

        // Duplicate activity id across phases.
        let mut p = valid_project();
        let mut phase2 = BuildPhase::new(PhaseId(2), "P2", 3.0);
        phase2.activities.push(crate::AssemblyActivity::new(
            ActivityId(1),
            "dup",
            crate::ActivityType::CutSteel,
            4.0,
        ));
        p.build_phases.push(phase2);
        assert!(matches!(p.validate(), Err(CoreError::Validation(_))));
    }

    #[test]
    fn json_loading_rejects_inconsistent_projects() {
        // A project JSON with a dangling dependency must not load.
        let mut p = valid_project();
        p.build_phases[0].activities[0].dependencies = vec![ActivityId(99)];
        let json = p.to_json().to_string_compact();
        assert!(VesselProject::from_json_str(&json).is_err());
    }

    fn sample() -> VesselProject {
        let mut p1 = BuildPhase::new(PhaseId(1), "Steel prefabrication", 30.0);
        p1.weight_state = WeightState {
            design_kg: 5_000.0,
            installed_kg: 0.0,
        };
        let mut a1 =
            AssemblyActivity::new(ActivityId(1), "Cut plates", ActivityType::CutSteel, 40.0);
        a1.dependencies = vec![];
        p1.activities.push(a1);
        let mut p2 = BuildPhase::new(PhaseId(2), "Block assembly", 45.0);
        p2.activities.push(AssemblyActivity::new(
            ActivityId(2),
            "Weld panels",
            ActivityType::WeldBlock,
            60.0,
        ));
        let mut a3 = AssemblyActivity::new(ActivityId(3), "Erect", ActivityType::JoinBlock, 8.0);
        a3.dependencies = vec![ActivityId(2)];
        p2.activities.push(a3);
        VesselProject::new(
            ProjectId(1),
            "Sample vessel",
            VesselType::Sea(SeaVesselType::ContainerShip {
                teu_capacity: 14_000,
            }),
            ConstructionMethod::SeaDrydock,
            vec![p1, p2],
        )
        .unwrap()
    }

    #[test]
    fn json_roundtrip_preserves_everything() {
        let project = sample();
        let json = project.to_json().to_string_pretty();
        let back = VesselProject::from_json_str(&json).unwrap();
        assert_eq!(back, project);
        // ...and through the compact writer too
        let back2 = VesselProject::from_json_str(&project.to_json().to_string_compact()).unwrap();
        assert_eq!(back2, project);
    }

    #[test]
    fn activity_graph_is_built_across_phases() {
        let project = sample();
        let g = project.activity_graph().unwrap();
        assert_eq!(g.len(), 3);
        // Paths: cut(40) and weld(60)+erect(8).
        assert_eq!(g.makespan_hours().unwrap(), 68.0);
    }

    #[test]
    fn is_in_final_phase_tracks_position() {
        let mut project = sample();
        assert!(!project.is_in_final_phase());
        project.current_phase = PhaseId(2);
        assert!(project.is_in_final_phase());
    }
}
