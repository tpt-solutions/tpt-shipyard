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
    /// # Errors
    ///
    /// [`CoreError::Validation`] when `build_phases` is empty.
    pub fn new(
        id: ProjectId,
        name: impl Into<String>,
        vessel_type: VesselType,
        construction_method: ConstructionMethod,
        build_phases: Vec<BuildPhase>,
    ) -> Result<Self, CoreError> {
        let current_phase = build_phases
            .first()
            .map(|p| p.id)
            .ok_or_else(|| CoreError::Validation("project has no build phases".into()))?;
        Ok(Self {
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
            if !phase.duration_days.is_finite() || phase.duration_days < 0.0 {
                return err(format!(
                    "phase {:?} has non-finite or negative duration {}",
                    phase.id, phase.duration_days
                ));
            }
            if !phase.weight_state.design_kg.is_finite()
                || phase.weight_state.design_kg < 0.0
                || !phase.weight_state.installed_kg.is_finite()
                || phase.weight_state.installed_kg < 0.0
            {
                return err(format!(
                    "phase {:?} has non-finite or negative weight state",
                    phase.id
                ));
            }
            if phase.weight_state.installed_kg > phase.weight_state.design_kg + 1e-9 {
                return err(format!(
                    "phase {:?} installed kg exceeds design kg",
                    phase.id
                ));
            }
            if phase.activities.is_empty() {
                return err(format!("phase {:?} has no activities", phase.id));
            }
            for a in &phase.activities {
                if !activity_ids.insert(a.id) {
                    return err(format!("duplicate activity id {}", a.id));
                }
                if !a.duration_hours.is_finite() || a.duration_hours <= 0.0 {
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

        if let Some(sv) = v.get("schema_version") {
            let n = sv
                .as_u64()
                .ok_or_else(|| err("schema_version must be an integer"))?;
            if n != Self::SCHEMA_VERSION {
                return Err(err(&format!(
                    "unsupported schema_version {n} (loader accepts {})",
                    Self::SCHEMA_VERSION
                )));
            }
        }

        let id = ProjectId(field("id")?.as_u64().ok_or_else(|| err("id must be u64"))?);
        let name = field("name")?
            .as_str()
            .ok_or_else(|| err("name must be a string"))?
            .to_string();

        let vessel_type_v = field("vessel_type")?;
        let kind_tag = vessel_type_v
            .get("kind")
            .ok_or_else(|| CoreError::missing_field("vessel_type.kind"))?
            .as_str()
            .ok_or_else(|| err("vessel_type.kind must be a string"))?;
        let vessel_type = match kind_tag {
            "hybrid" => VesselType::Hybrid,
            "sea" => {
                let inner = vessel_type_v
                    .get("value")
                    .ok_or_else(|| err("sea vessel needs a value"))?;
                let kind = inner
                    .get("kind")
                    .ok_or_else(|| CoreError::missing_field("vessel_type.value.kind"))?
                    .as_str()
                    .ok_or_else(|| err("vessel_type.value.kind must be a string"))?;
                let num = |k: &str| -> Result<f64, CoreError> {
                    inner
                        .get(k)
                        .ok_or_else(|| CoreError::missing_field(format!("vessel_type.value.{k}")))?
                        .as_f64()
                        .ok_or_else(|| err(&format!("vessel_type.value.{k} must be a number")))
                };
                let u32_num = |k: &str| -> Result<u32, CoreError> {
                    let n = num(k)?;
                    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 || n > u32::MAX as f64 {
                        return Err(err(&format!(
                            "vessel_type.value.{k} must be a non-negative integer"
                        )));
                    }
                    Ok(n as u32)
                };
                let string = |k: &str| -> Result<String, CoreError> {
                    inner
                        .get(k)
                        .ok_or_else(|| CoreError::missing_field(format!("vessel_type.value.{k}")))?
                        .as_str()
                        .map(str::to_string)
                        .ok_or_else(|| err(&format!("vessel_type.value.{k} must be a string")))
                };
                VesselType::Sea(match kind {
                    "container_ship" => SeaVesselType::ContainerShip {
                        teu_capacity: u32_num("teu_capacity")?,
                    },
                    "tanker" => SeaVesselType::Tanker {
                        deadweight_tonnes: num("deadweight_tonnes")?,
                    },
                    "lng_carrier" => SeaVesselType::LngCarrier {
                        cargo_volume_m3: num("cargo_volume_m3")?,
                    },
                    "cruise_ship" => SeaVesselType::CruiseShip {
                        passengers: u32_num("passengers")?,
                    },
                    "naval_vessel" => SeaVesselType::NavalVessel {
                        classification: string("classification")?,
                    },
                    "submarine" => SeaVesselType::Submarine {
                        hull_type: match inner.get("hull_type") {
                            None => {
                                return Err(CoreError::missing_field(
                                    "vessel_type.value.hull_type",
                                ))
                            }
                            Some(s) => match s.as_str() {
                                Some("SingleHull") => HullType::SingleHull,
                                Some("DoubleHull") => HullType::DoubleHull,
                                _ => {
                                    return Err(err(
                                        "vessel_type.value.hull_type must be 'SingleHull' or 'DoubleHull'",
                                    ))
                                }
                            },
                        },
                    },
                    "offshore_vessel" => SeaVesselType::OffshoreVessel {
                        vessel_class: string("vessel_class")?,
                    },
                    "fishing_vessel" => SeaVesselType::FishingVessel,
                    other => return Err(err(&format!("unknown sea vessel type '{other}'"))),
                })
            }
            "space" => {
                let inner = vessel_type_v
                    .get("value")
                    .ok_or_else(|| err("space vessel needs a value"))?;
                let kind = inner
                    .get("kind")
                    .ok_or_else(|| CoreError::missing_field("vessel_type.value.kind"))?
                    .as_str()
                    .ok_or_else(|| err("vessel_type.value.kind must be a string"))?;
                let num = |k: &str| -> Result<f64, CoreError> {
                    inner
                        .get(k)
                        .ok_or_else(|| CoreError::missing_field(format!("vessel_type.value.{k}")))?
                        .as_f64()
                        .ok_or_else(|| err(&format!("vessel_type.value.{k} must be a number")))
                };
                let u32_num = |k: &str| -> Result<u32, CoreError> {
                    let n = num(k)?;
                    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 || n > u32::MAX as f64 {
                        return Err(err(&format!(
                            "vessel_type.value.{k} must be a non-negative integer"
                        )));
                    }
                    Ok(n as u32)
                };
                VesselType::Space(match kind {
                    "space_station" => SpaceVesselType::SpaceStation {
                        modules: u32_num("modules")?,
                    },
                    "orbital_habitat" => SpaceVesselType::OrbitalHabitat {
                        rotation_rpm: num("rotation_rpm")?,
                        radius_m: num("radius_m")?,
                    },
                    "solar_power_station" => SpaceVesselType::SolarPowerStation {
                        array_area_m2: num("array_area_m2")?,
                    },
                    "deep_space_vessel" => SpaceVesselType::DeepSpaceVessel {
                        propulsion: match inner.get("propulsion") {
                            None => {
                                return Err(CoreError::missing_field(
                                    "vessel_type.value.propulsion",
                                ))
                            }
                            Some(s) => match s.as_str() {
                                Some("ChemicalBiPropellant") => {
                                    PropulsionType::ChemicalBiPropellant
                                }
                                Some("ChemicalMonoPropellant") => {
                                    PropulsionType::ChemicalMonoPropellant
                                }
                                Some("Solid") => PropulsionType::Solid,
                                Some("NuclearThermal") => PropulsionType::NuclearThermal,
                                Some("NuclearElectric") => PropulsionType::NuclearElectric,
                                Some("SolarElectric") => PropulsionType::SolarElectric,
                                Some("SolarSail") => PropulsionType::SolarSail,
                                _ => return Err(err(
                                    "vessel_type.value.propulsion is not a known PropulsionType",
                                )),
                            },
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

        let method_tag = field("construction_method")?
            .as_str()
            .ok_or_else(|| err("construction_method must be a string"))?;
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
        phase.activities.push(crate::AssemblyActivity::new(
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
        assert!(matches!(p.validate(), Err(CoreError::Validation(_))));

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

    // -------------------------------------------------- strict JSON loading

    /// A minimal *valid* project JSON tree that individual tests mutate to
    /// prove the loader rejects malformed input instead of defaulting.
    fn strict_base() -> Value {
        let activity = |extra: Vec<(String, Value)>| {
            let mut o = vec![
                ("id".to_string(), Value::Number(1.0)),
                ("name".to_string(), Value::String("a".into())),
                (
                    "activity_type".to_string(),
                    Value::Object(vec![(
                        "kind".to_string(),
                        Value::String("cut_steel".into()),
                    )]),
                ),
                ("dependencies".to_string(), Value::Array(vec![])),
                ("duration_hours".to_string(), Value::Number(4.0)),
                ("resources".to_string(), Value::Array(vec![])),
            ];
            o.extend(extra);
            Value::Object(o)
        };
        let phase = Value::Object(vec![
            ("id".to_string(), Value::Number(1.0)),
            ("name".to_string(), Value::String("P1".into())),
            (
                "activities".to_string(),
                Value::Array(vec![activity(vec![])]),
            ),
            (
                "structural_state".to_string(),
                Value::Object(vec![(
                    "kind".to_string(),
                    Value::String("not_started".into()),
                )]),
            ),
            (
                "weight_state".to_string(),
                Value::Object(vec![
                    ("design_kg".to_string(), Value::Number(100.0)),
                    ("installed_kg".to_string(), Value::Number(0.0)),
                ]),
            ),
            ("duration_days".to_string(), Value::Number(3.0)),
        ]);
        Value::Object(vec![
            ("schema_version".to_string(), Value::Number(1.0)),
            ("id".to_string(), Value::Number(1.0)),
            ("name".to_string(), Value::String("strict".into())),
            (
                "vessel_type".to_string(),
                Value::Object(vec![
                    ("kind".to_string(), Value::String("sea".into())),
                    (
                        "value".to_string(),
                        Value::Object(vec![
                            ("kind".to_string(), Value::String("container_ship".into())),
                            ("teu_capacity".to_string(), Value::Number(100.0)),
                        ]),
                    ),
                ]),
            ),
            (
                "construction_method".to_string(),
                Value::String("sea_drydock".into()),
            ),
            ("build_phases".to_string(), Value::Array(vec![phase])),
            ("current_phase".to_string(), Value::Number(1.0)),
        ])
    }

    /// Mutable borrow of `key` inside a JSON object (test helper).
    fn slot<'a>(v: &'a mut Value, key: &str) -> &'a mut Value {
        match v {
            Value::Object(pairs) => {
                &mut pairs
                    .iter_mut()
                    .find(|(k, _)| k == key)
                    .unwrap_or_else(|| panic!("no key {key}"))
                    .1
            }
            _ => panic!("not an object"),
        }
    }

    fn remove_key(v: &mut Value, key: &str) {
        if let Value::Object(pairs) = v {
            pairs.retain(|(k, _)| k != key);
        }
    }

    /// Mutable borrow of element `i` inside a JSON array (test helper).
    fn elem(v: &mut Value, i: usize) -> &mut Value {
        match v {
            Value::Array(items) => &mut items[i],
            _ => panic!("not an array"),
        }
    }

    fn strict_load(v: &Value) -> Result<VesselProject, CoreError> {
        VesselProject::from_json_value(v)
    }

    #[test]
    fn strict_json_rejects_missing_and_wrong_typed_fields() {
        // Baseline must load.
        assert!(strict_load(&strict_base()).is_ok());

        // Missing vessel payload field: no silent 0.
        let mut j = strict_base();
        remove_key(slot(slot(&mut j, "vessel_type"), "value"), "teu_capacity");
        assert!(matches!(strict_load(&j), Err(CoreError::MissingField(_))));

        // Wrong type (string where number belongs): no silent 0 cast.
        let mut j = strict_base();
        *slot(slot(slot(&mut j, "vessel_type"), "value"), "teu_capacity") =
            Value::String("many".into());
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));

        // Fractional/negative counts are not u32s.
        let mut j = strict_base();
        *slot(slot(slot(&mut j, "vessel_type"), "value"), "teu_capacity") = Value::Number(99.5);
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));

        // Missing vessel kind tag: not an empty-string unknown.
        let mut j = strict_base();
        remove_key(slot(&mut j, "vessel_type"), "kind");
        assert!(matches!(strict_load(&j), Err(CoreError::MissingField(_))));

        // construction_method of the wrong JSON type: a type error, not
        // "unknown construction method ''".
        let mut j = strict_base();
        *slot(&mut j, "construction_method") = Value::Number(3.0);
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));

        // weight_state with a missing component: not silently zero.
        let mut j = strict_base();
        remove_key(
            slot(elem(slot(&mut j, "build_phases"), 0), "weight_state"),
            "design_kg",
        );
        assert!(matches!(strict_load(&j), Err(CoreError::MissingField(_))));

        // weight_state component of the wrong type: not silently zero.
        let mut j = strict_base();
        *slot(
            slot(elem(slot(&mut j, "build_phases"), 0), "weight_state"),
            "installed_kg",
        ) = Value::String("heavy".into());
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));
    }

    #[test]
    fn strict_json_rejects_unknown_enums_and_bad_schema_version() {
        // Unknown submarine hull type must not become SingleHull.
        let mut j = strict_base();
        {
            let value = slot(slot(&mut j, "vessel_type"), "value");
            *value = Value::Object(vec![
                ("kind".to_string(), Value::String("submarine".into())),
                ("hull_type".to_string(), Value::String("TrebleHull".into())),
            ]);
        }
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));

        // Missing hull_type must not become SingleHull either.
        let mut j = strict_base();
        {
            let value = slot(slot(&mut j, "vessel_type"), "value");
            *value = Value::Object(vec![(
                "kind".to_string(),
                Value::String("submarine".into()),
            )]);
        }
        assert!(matches!(strict_load(&j), Err(CoreError::MissingField(_))));

        // Unknown propulsion must not silently become ChemicalBiPropellant.
        let mut j = strict_base();
        {
            let value = slot(slot(&mut j, "vessel_type"), "value");
            *value = Value::Object(vec![
                (
                    "kind".to_string(),
                    Value::String("deep_space_vessel".into()),
                ),
                ("propulsion".to_string(), Value::String("Warp".into())),
            ]);
        }
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));

        // A future schema version must be refused, not loaded as-is.
        let mut j = strict_base();
        *slot(&mut j, "schema_version") = Value::Number(2.0);
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));
    }

    #[test]
    fn strict_json_rejects_malformed_dependency_and_resource_entries() {
        fn set_activity_field(j: &mut Value, key: &str, new: Value) {
            let acts = elem(slot(j, "build_phases"), 0);
            let acts = slot(acts, "activities");
            if let Value::Object(pairs) = elem(acts, 0) {
                for (k, v) in pairs.iter_mut() {
                    if k == key {
                        *v = new.clone();
                    }
                }
            }
        }

        // A non-integer dependency must be an error, not dropped.
        let mut j = strict_base();
        set_activity_field(
            &mut j,
            "dependencies",
            Value::Array(vec![Value::String("1".into())]),
        );
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));

        // A resource with an unknown kind must be an error, not dropped.
        let mut j = strict_base();
        set_activity_field(
            &mut j,
            "resources",
            Value::Array(vec![Value::Object(vec![
                ("name".to_string(), Value::String("Goliath".into())),
                ("kind".to_string(), Value::String("Skyhook".into())),
                ("capacity".to_string(), Value::Number(800.0)),
            ])]),
        );
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));

        // A resource with a missing capacity must be an error, not defaulted.
        let mut j = strict_base();
        set_activity_field(
            &mut j,
            "resources",
            Value::Array(vec![Value::Object(vec![
                ("name".to_string(), Value::String("Goliath".into())),
                ("kind".to_string(), Value::String("Crane".into())),
            ])]),
        );
        assert!(matches!(strict_load(&j), Err(CoreError::TypeError(_))));
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
