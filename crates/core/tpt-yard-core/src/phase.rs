//! Build phases and assembly activities: how a vessel is actually built.
//!
//! A [`VesselProject`](crate::VesselProject) is a sequence of
//! [`BuildPhase`]s (keel laying, block erection, outfitting, launch, ...),
//! each holding the [`AssemblyActivity`]s that must complete in dependency
//! order. Activities consume [`Resource`]s (cranes, workshops, crews) and are
//! typed by [`ActivityType`].

use crate::ids::PhaseId;
use crate::json::Value;
use crate::CoreError;
use tpt_yard_assembly::ActivityId;

/// The kind of work an assembly activity represents.
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityType {
    /// Cut steel plates and profiles.
    CutSteel,
    /// Form plates (rolling, pressing).
    FormPlate,
    /// Weld sub-blocks/panels into a block.
    WeldBlock,
    /// Lift a block with a crane.
    LiftBlock,
    /// Join a block to the growing hull.
    JoinBlock,
    /// Install an outfit system (piping, electrical, HVAC, ...).
    Outfit {
        /// The system being installed.
        system: OutfitSystem,
    },
    /// Paint / coat.
    Paint,
    /// Perform a test.
    Test {
        /// The kind of test.
        test_type: TestType,
    },
    /// Launch the vessel.
    Launch,
    /// Dock the vessel (drydock entrance, mating).
    Dock,
    /// Robotic assembly action.
    RoboticAssembly {
        /// The robot performing the action.
        robot: crate::RobotId,
    },
    /// Additive manufacturing of a component.
    AdditiveManufacturing {
        /// Feedstock material name.
        material: String,
    },
    /// Load propellant.
    PropellantLoad,
}

impl ActivityType {
    /// Serializes to a JSON object (tagged by `kind`).
    pub fn to_json(&self) -> Value {
        let mut o = vec![(
            "kind".to_string(),
            Value::String(
                match self {
                    ActivityType::CutSteel => "cut_steel",
                    ActivityType::FormPlate => "form_plate",
                    ActivityType::WeldBlock => "weld_block",
                    ActivityType::LiftBlock => "lift_block",
                    ActivityType::JoinBlock => "join_block",
                    ActivityType::Outfit { .. } => "outfit",
                    ActivityType::Paint => "paint",
                    ActivityType::Test { .. } => "test",
                    ActivityType::Launch => "launch",
                    ActivityType::Dock => "dock",
                    ActivityType::RoboticAssembly { .. } => "robotic_assembly",
                    ActivityType::AdditiveManufacturing { .. } => "additive_manufacturing",
                    ActivityType::PropellantLoad => "propellant_load",
                }
                .to_string(),
            ),
        )];
        match self {
            ActivityType::Outfit { system } => o.push(("system".into(), system.to_json())),
            ActivityType::Test { test_type } => {
                o.push(("test_type".into(), Value::String(format!("{test_type:?}"))))
            }
            ActivityType::RoboticAssembly { robot } => {
                o.push(("robot".into(), Value::Number(robot.0 as f64)))
            }
            ActivityType::AdditiveManufacturing { material } => {
                o.push(("material".into(), Value::String(material.clone())))
            }
            _ => {}
        }
        Value::Object(o)
    }

    /// Deserializes from a JSON object produced by [`ActivityType::to_json`].
    ///
    /// # Errors
    ///
    /// [`CoreError`] on unknown kinds or malformed payloads.
    pub fn from_json(v: &Value) -> Result<Self, CoreError> {
        let kind = v
            .get("kind")
            .and_then(|k| k.as_str())
            .ok_or_else(|| CoreError::missing_field("ActivityType.kind"))?;
        match kind {
            "cut_steel" => Ok(ActivityType::CutSteel),
            "form_plate" => Ok(ActivityType::FormPlate),
            "weld_block" => Ok(ActivityType::WeldBlock),
            "lift_block" => Ok(ActivityType::LiftBlock),
            "join_block" => Ok(ActivityType::JoinBlock),
            "outfit" => Ok(ActivityType::Outfit {
                system: OutfitSystem::from_json(
                    v.get("system")
                        .ok_or_else(|| CoreError::missing_field("system"))?,
                )?,
            }),
            "paint" => Ok(ActivityType::Paint),
            "test" => {
                let t = v
                    .get("test_type")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| CoreError::missing_field("test_type"))?;
                TestType::from_name(t)
                    .map(|test_type| ActivityType::Test { test_type })
                    .ok_or_else(|| CoreError::type_error(format!("unknown test type '{t}'")))
            }
            "launch" => Ok(ActivityType::Launch),
            "dock" => Ok(ActivityType::Dock),
            "robotic_assembly" => Ok(ActivityType::RoboticAssembly {
                robot: crate::RobotId(
                    v.get("robot")
                        .and_then(|r| r.as_u64())
                        .ok_or_else(|| CoreError::missing_field("robot"))?,
                ),
            }),
            "additive_manufacturing" => Ok(ActivityType::AdditiveManufacturing {
                material: v
                    .get("material")
                    .and_then(|m| m.as_str())
                    .ok_or_else(|| CoreError::missing_field("material"))?
                    .to_string(),
            }),
            "propellant_load" => Ok(ActivityType::PropellantLoad),
            other => Err(CoreError::type_error(format!(
                "unknown activity type '{other}'"
            ))),
        }
    }
}

/// An outfit system installed during construction.
///
/// Defined here (rather than in `tpt-yard-outfitting`) because
/// [`ActivityType::Outfit`] references it; the outfitting crate extends it
/// with routing and clash analysis.
#[derive(Debug, Clone, PartialEq)]
pub enum OutfitSystem {
    /// Piping run.
    Piping {
        /// Fluid carried.
        fluid: FluidType,
        /// Nominal diameter, mm.
        diameter_mm: f64,
    },
    /// Electrical distribution.
    Electrical {
        /// Voltage, V.
        voltage_v: f64,
        /// Cable type designation.
        cable_type: String,
    },
    /// HVAC ducting.
    Hvac {
        /// Duct size, mm.
        duct_size_mm: f64,
    },
    /// Secondary/tertiary structure.
    Structural {
        /// Member type designation.
        member_type: String,
    },
    /// Machinery unit.
    Machinery {
        /// Equipment designation.
        equipment: String,
    },
    /// Navigation equipment.
    Navigation {
        /// Equipment designation.
        equipment: String,
    },
}

impl OutfitSystem {
    /// Serializes to a JSON object.
    pub fn to_json(&self) -> Value {
        let mut o = vec![(
            "kind".to_string(),
            Value::String(
                match self {
                    OutfitSystem::Piping { .. } => "piping",
                    OutfitSystem::Electrical { .. } => "electrical",
                    OutfitSystem::Hvac { .. } => "hvac",
                    OutfitSystem::Structural { .. } => "structural",
                    OutfitSystem::Machinery { .. } => "machinery",
                    OutfitSystem::Navigation { .. } => "navigation",
                }
                .to_string(),
            ),
        )];
        match self {
            OutfitSystem::Piping { fluid, diameter_mm } => {
                o.push(("fluid".into(), Value::String(format!("{fluid:?}"))));
                o.push(("diameter_mm".into(), Value::Number(*diameter_mm)));
            }
            OutfitSystem::Electrical {
                voltage_v,
                cable_type,
            } => {
                o.push(("voltage_v".into(), Value::Number(*voltage_v)));
                o.push(("cable_type".into(), Value::String(cable_type.clone())));
            }
            OutfitSystem::Hvac { duct_size_mm } => {
                o.push(("duct_size_mm".into(), Value::Number(*duct_size_mm)));
            }
            OutfitSystem::Structural { member_type }
            | OutfitSystem::Machinery {
                equipment: member_type,
            }
            | OutfitSystem::Navigation {
                equipment: member_type,
            } => {
                o.push(("designation".into(), Value::String(member_type.clone())));
            }
        }
        Value::Object(o)
    }

    /// Deserializes from a JSON object produced by [`OutfitSystem::to_json`].
    ///
    /// # Errors
    ///
    /// [`CoreError`] on unknown kinds or malformed payloads.
    pub fn from_json(v: &Value) -> Result<Self, CoreError> {
        let kind = v
            .get("kind")
            .and_then(|k| k.as_str())
            .ok_or_else(|| CoreError::missing_field("OutfitSystem.kind"))?;
        let num = |k: &str| -> Result<f64, CoreError> {
            v.get(k)
                .and_then(|n| n.as_f64())
                .ok_or_else(|| CoreError::missing_field(k))
        };
        let string = |k: &str| -> Result<String, CoreError> {
            v.get(k)
                .and_then(|s| s.as_str())
                .map(str::to_string)
                .ok_or_else(|| CoreError::missing_field(k))
        };
        match kind {
            "piping" => {
                let fname = string("fluid")?;
                let fluid = FluidType::from_name(&fname)
                    .ok_or_else(|| CoreError::type_error(format!("unknown fluid '{fname}'")))?;
                Ok(OutfitSystem::Piping {
                    fluid,
                    diameter_mm: num("diameter_mm")?,
                })
            }
            "electrical" => Ok(OutfitSystem::Electrical {
                voltage_v: num("voltage_v")?,
                cable_type: string("cable_type")?,
            }),
            "hvac" => Ok(OutfitSystem::Hvac {
                duct_size_mm: num("duct_size_mm")?,
            }),
            "structural" => Ok(OutfitSystem::Structural {
                member_type: string("designation")?,
            }),
            "machinery" => Ok(OutfitSystem::Machinery {
                equipment: string("designation")?,
            }),
            "navigation" => Ok(OutfitSystem::Navigation {
                equipment: string("designation")?,
            }),
            other => Err(CoreError::type_error(format!(
                "unknown outfit system '{other}'"
            ))),
        }
    }
}

/// Fluid categories for piping systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidType {
    /// Fresh water systems.
    FreshWater,
    /// Sea water systems.
    SeaWater,
    /// Fuel oil.
    FuelOil,
    /// Lubricating oil.
    LubeOil,
    /// Hydraulic oil.
    HydraulicOil,
    /// Compressed air.
    CompressedAir,
    /// Steam.
    Steam,
    /// Cryogenic liquids (LNG, LH2, LOX).
    Cryogenic,
    /// Exhaust gas.
    Exhaust,
    /// Chemicals / dosing.
    Chemical,
}

impl FluidType {
    /// Parses a fluid from its `Debug`-style name.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "FreshWater" => Self::FreshWater,
            "SeaWater" => Self::SeaWater,
            "FuelOil" => Self::FuelOil,
            "LubeOil" => Self::LubeOil,
            "HydraulicOil" => Self::HydraulicOil,
            "CompressedAir" => Self::CompressedAir,
            "Steam" => Self::Steam,
            "Cryogenic" => Self::Cryogenic,
            "Exhaust" => Self::Exhaust,
            "Chemical" => Self::Chemical,
            _ => return None,
        })
    }
}

/// Test categories performed during construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestType {
    /// Hydrostatic / hose test of tanks and compartments.
    Hydrostatic,
    /// Structural strength (load) test.
    StrengthLoad,
    /// Tightness test.
    Tightness,
    /// Non-destructive examination.
    Ndt,
    /// Inclining / stability test.
    StabilityInclining,
    /// Makers' / harbour / sea trial.
    SeaTrial,
    /// Pressure test (piping, pressure vessels).
    Pressure,
}

impl TestType {
    /// Parses a test type from its `Debug`-style name.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "Hydrostatic" => Self::Hydrostatic,
            "StrengthLoad" => Self::StrengthLoad,
            "Tightness" => Self::Tightness,
            "Ndt" => Self::Ndt,
            "StabilityInclining" => Self::StabilityInclining,
            "SeaTrial" => Self::SeaTrial,
            "Pressure" => Self::Pressure,
            _ => return None,
        })
    }
}

/// A shop or yard resource an activity consumes.
#[derive(Debug, Clone, PartialEq)]
pub struct Resource {
    /// Resource name (e.g. "Goliath crane No. 3").
    pub name: String,
    /// The kind of resource.
    pub kind: ResourceKind,
    /// Capacity in kind-specific units: tonnes for cranes, m² for workshops
    /// and staging, persons for crews, t/h for transports, unit-less (1.0) for
    /// exclusive-use facilities.
    pub capacity: f64,
}

/// Categories of yard resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceKind {
    /// Crane (capacity in tonnes).
    Crane,
    /// Workshop / assembly area (capacity in m²).
    Workshop,
    /// Drydock / building dock (exclusive use).
    Drydock,
    /// Welding station.
    WeldingStation,
    /// Construction or assembly robot.
    Robot,
    /// Crew (persons).
    Crew,
    /// Transport (capacity in tonnes).
    Transport,
}

/// A unit of work in the build plan.
#[derive(Debug, Clone, PartialEq)]
pub struct AssemblyActivity {
    /// Activity identifier, unique within the project.
    pub id: ActivityId,
    /// Human-readable name.
    pub name: String,
    /// What kind of work this is.
    pub activity_type: ActivityType,
    /// Activities that must complete first.
    pub dependencies: Vec<ActivityId>,
    /// Planned duration in hours.
    pub duration_hours: f64,
    /// Resources consumed while executing.
    pub resources: Vec<Resource>,
}

impl AssemblyActivity {
    /// Creates a new activity with no dependencies and no resources.
    pub fn new(
        id: ActivityId,
        name: impl Into<String>,
        activity_type: ActivityType,
        duration_hours: f64,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            activity_type,
            dependencies: Vec::new(),
            duration_hours,
            resources: Vec::new(),
        }
    }

    /// Builder-style dependency setter.
    pub fn with_dependencies(mut self, deps: &[ActivityId]) -> Self {
        self.dependencies = deps.to_vec();
        self
    }

    /// Builder-style resource setter.
    pub fn with_resources(mut self, resources: Vec<Resource>) -> Self {
        self.resources = resources;
        self
    }

    /// Serializes to a JSON object.
    pub fn to_json(&self) -> Value {
        Value::Object(vec![
            ("id".into(), Value::Number(self.id.0 as f64)),
            ("name".into(), Value::String(self.name.clone())),
            ("activity_type".into(), self.activity_type.to_json()),
            (
                "dependencies".into(),
                Value::Array(
                    self.dependencies
                        .iter()
                        .map(|d| Value::Number(d.0 as f64))
                        .collect(),
                ),
            ),
            ("duration_hours".into(), Value::Number(self.duration_hours)),
            (
                "resources".into(),
                Value::Array(
                    self.resources
                        .iter()
                        .map(|r| {
                            Value::Object(vec![
                                ("name".into(), Value::String(r.name.clone())),
                                ("kind".into(), Value::String(format!("{:?}", r.kind))),
                                ("capacity".into(), Value::Number(r.capacity)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }

    /// Deserializes from a JSON object produced by [`AssemblyActivity::to_json`].
    ///
    /// # Errors
    ///
    /// [`CoreError`] on malformed payloads.
    pub fn from_json(v: &Value) -> Result<Self, CoreError> {
        let field = |k: &str| v.get(k).ok_or_else(|| CoreError::missing_field(k));
        let err = |msg: &str| CoreError::type_error(msg.to_string());
        let id = ActivityId(
            field("id")?
                .as_u64()
                .ok_or_else(|| err("activity.id must be an integer"))?,
        );
        let name = field("name")?
            .as_str()
            .ok_or_else(|| err("activity.name must be a string"))?;
        let activity_type = ActivityType::from_json(field("activity_type")?)?;
        let deps_v = field("dependencies")?;
        let deps_arr = deps_v
            .as_array()
            .ok_or_else(|| err("activity.dependencies must be an array"))?;
        let mut dependencies = Vec::with_capacity(deps_arr.len());
        for (i, d) in deps_arr.iter().enumerate() {
            let n = d
                .as_u64()
                .ok_or_else(|| err(&format!("activity.dependencies[{i}] must be an integer")))?;
            dependencies.push(ActivityId(n));
        }
        let duration_hours = field("duration_hours")?
            .as_f64()
            .ok_or_else(|| err("activity.duration_hours must be a number"))?;
        let res_v = field("resources")?;
        let res_arr = res_v
            .as_array()
            .ok_or_else(|| err("activity.resources must be an array"))?;
        let mut resources = Vec::with_capacity(res_arr.len());
        for (i, r) in res_arr.iter().enumerate() {
            let bad =
                |msg: String| CoreError::type_error(format!("activity.resources[{i}]: {msg}"));
            let name = r
                .get("name")
                .ok_or_else(|| bad("missing 'name'".into()))?
                .as_str()
                .ok_or_else(|| bad("'name' must be a string".into()))?;
            let kind_str = r
                .get("kind")
                .ok_or_else(|| bad("missing 'kind'".into()))?
                .as_str()
                .ok_or_else(|| bad("'kind' must be a string".into()))?;
            let kind = match kind_str {
                "Crane" => ResourceKind::Crane,
                "Workshop" => ResourceKind::Workshop,
                "Drydock" => ResourceKind::Drydock,
                "WeldingStation" => ResourceKind::WeldingStation,
                "Robot" => ResourceKind::Robot,
                "Crew" => ResourceKind::Crew,
                "Transport" => ResourceKind::Transport,
                other => return Err(bad(format!("unknown resource kind '{other}'"))),
            };
            let capacity = r
                .get("capacity")
                .ok_or_else(|| bad("missing 'capacity'".into()))?
                .as_f64()
                .ok_or_else(|| bad("'capacity' must be a number".into()))?;
            if !capacity.is_finite() {
                return Err(bad("'capacity' must be finite".into()));
            }
            resources.push(Resource {
                name: name.to_string(),
                kind,
                capacity,
            });
        }
        Ok(Self {
            id,
            name: name.to_string(),
            activity_type,
            dependencies,
            duration_hours,
            resources,
        })
    }
}

/// How the incomplete structure stands at a given phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StructuralState {
    /// Nothing built yet.
    NotStarted,
    /// Partially built; `completion` in `[0, 1]` by installed weight.
    Partial {
        /// Completion fraction `[0, 1]`.
        completion: f64,
    },
    /// All primary structure closed (hull tight / pressure tight).
    Closed,
    /// Closed *and* strength-verified for the phase load cases.
    StrengthVerified,
}

/// Weight snapshot of a phase: design expectation vs installed reality.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WeightState {
    /// Design weight of everything in this phase, kg.
    pub design_kg: f64,
    /// Weight actually installed in this phase, kg.
    pub installed_kg: f64,
}

/// One stage of the build: a named bundle of activities.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildPhase {
    /// Phase identifier, unique within the project.
    pub id: PhaseId,
    /// Human-readable name (e.g. "Erection phase 3 — cargo hold area").
    pub name: String,
    /// Activities of this phase, in planned execution order.
    pub activities: Vec<AssemblyActivity>,
    /// Structural state reached when the phase completes.
    pub structural_state: StructuralState,
    /// Weight bookkeeping for the phase.
    pub weight_state: WeightState,
    /// Planned duration in days.
    pub duration_days: f64,
}

impl BuildPhase {
    /// Creates an empty phase.
    pub fn new(id: PhaseId, name: impl Into<String>, duration_days: f64) -> Self {
        Self {
            id,
            name: name.into(),
            activities: Vec::new(),
            structural_state: StructuralState::NotStarted,
            weight_state: WeightState::default(),
            duration_days,
        }
    }

    /// Serializes to a JSON object.
    pub fn to_json(&self) -> Value {
        let structural = match self.structural_state {
            StructuralState::NotStarted => {
                Value::Object(vec![("kind".into(), Value::String("not_started".into()))])
            }
            StructuralState::Partial { completion } => Value::Object(vec![
                ("kind".into(), Value::String("partial".into())),
                ("completion".into(), Value::Number(completion)),
            ]),
            StructuralState::Closed => {
                Value::Object(vec![("kind".into(), Value::String("closed".into()))])
            }
            StructuralState::StrengthVerified => Value::Object(vec![(
                "kind".into(),
                Value::String("strength_verified".into()),
            )]),
        };
        Value::Object(vec![
            ("id".into(), Value::Number(self.id.0 as f64)),
            ("name".into(), Value::String(self.name.clone())),
            (
                "activities".into(),
                Value::Array(self.activities.iter().map(|a| a.to_json()).collect()),
            ),
            ("structural_state".into(), structural),
            (
                "weight_state".into(),
                Value::Object(vec![
                    (
                        "design_kg".into(),
                        Value::Number(self.weight_state.design_kg),
                    ),
                    (
                        "installed_kg".into(),
                        Value::Number(self.weight_state.installed_kg),
                    ),
                ]),
            ),
            ("duration_days".into(), Value::Number(self.duration_days)),
        ])
    }

    /// Deserializes from a JSON object produced by [`BuildPhase::to_json`].
    ///
    /// # Errors
    ///
    /// [`CoreError`] on malformed payloads.
    pub fn from_json(v: &Value) -> Result<Self, CoreError> {
        let field = |k: &str| v.get(k).ok_or_else(|| CoreError::missing_field(k));
        let err = |msg: &str| CoreError::type_error(msg.to_string());
        let id = PhaseId(
            field("id")?
                .as_u64()
                .ok_or_else(|| err("phase.id must be an integer"))?,
        );
        let name = field("name")?
            .as_str()
            .ok_or_else(|| err("phase.name must be a string"))?;
        let activities = field("activities")?
            .as_array()
            .ok_or_else(|| CoreError::missing_field("activities"))?
            .iter()
            .map(AssemblyActivity::from_json)
            .collect::<Result<Vec<_>, _>>()?;
        let structural_state = {
            let s = field("structural_state")?;
            match s.get("kind").and_then(|k| k.as_str()) {
                Some("not_started") => StructuralState::NotStarted,
                Some("partial") => StructuralState::Partial {
                    completion: s
                        .get("completion")
                        .and_then(|c| c.as_f64())
                        .ok_or_else(|| CoreError::missing_field("completion"))?,
                },
                Some("closed") => StructuralState::Closed,
                Some("strength_verified") => StructuralState::StrengthVerified,
                _ => return Err(CoreError::type_error("unknown structural_state")),
            }
        };
        let ws = field("weight_state")?;
        let weight_num = |k: &str| -> Result<f64, CoreError> {
            ws.get(k)
                .ok_or_else(|| CoreError::missing_field(format!("weight_state.{k}")))?
                .as_f64()
                .ok_or_else(|| err(&format!("weight_state.{k} must be a number")))
        };
        let weight_state = WeightState {
            design_kg: weight_num("design_kg")?,
            installed_kg: weight_num("installed_kg")?,
        };
        let duration_days = field("duration_days")?
            .as_f64()
            .ok_or_else(|| err("phase.duration_days must be a number"))?;
        Ok(Self {
            id,
            name: name.to_string(),
            activities,
            structural_state,
            weight_state,
            duration_days,
        })
    }
}
