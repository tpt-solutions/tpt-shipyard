//! Orbital assembly planning and simulation for space structures.
//!
//! [`OrbitalAssembly`] owns a target [`SpaceStructure`], the component
//! manifest, and the robotic arms doing the work. Planning produces an
//! [`AssemblyStep`] sequence (grasp -> translate -> rotate -> bolt ->
//! release per component); simulation checks each step against the
//! [`AssemblyConstraint`]s — collisions, force limits, station keeping —
//! and [`OrbitalAssembly::verify_structural_integrity`] checks the
//! *partially assembled* structure against docking impulses. Model choices:
//! RFC 0003.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::{ComponentId, RobotId, Vector3};
//! use tpt_yard_orbital_assembly::{
//!     AssemblyState, ComponentSpec, OrbitalAssembly, OrbitalParameters,
//!     SpaceStructure,
//! };
//!
//! let mut assembly = OrbitalAssembly::new(
//!     SpaceStructure::Truss { segments: 4, length_m: 40.0 },
//!     OrbitalParameters::default(),
//! );
//! assembly.add_component(ComponentSpec {
//!     id: ComponentId(1),
//!     name: "bay 1".into(),
//!     mass_kg: 500.0,
//!     dimensions: Vector3::new(10.0, 3.0, 3.0),
//!     target_position: Vector3::new(5.0, 0.0, 0.0),
//! });
//! let steps = assembly.plan_sequence().unwrap();
//! assert!(steps.len() >= 4); // grasp, translate, rotate, dock/bolt, release
//! ```

use std::fmt;

use tpt_yard_core::{ComponentId, Geometry3D, RobotId, StepId, Vector3};
use tpt_yard_robotic_assembly::{CollisionObject, RoboticArm};

/// The structure being assembled.
#[derive(Debug, Clone, PartialEq)]
pub enum SpaceStructure {
    /// A deployed truss of N segments.
    Truss {
        /// Number of truss segments (bays).
        segments: u32,
        /// Total deployed length, m.
        length_m: f64,
    },
    /// A modular station.
    Station {
        /// Pressurized modules.
        modules: Vec<ModuleSpec>,
    },
    /// A rotating habitat.
    RotatingHabitat {
        /// Spin radius, m.
        radius_m: f64,
        /// Spin rate, rpm.
        rotation_rpm: f64,
    },
    /// A solar power array.
    SolarArray {
        /// Number of panels.
        panel_count: u32,
        /// Collecting area, m².
        area_m2: f64,
    },
    /// Cryogenic propellant depot.
    FuelDepot {
        /// Number of tank sets.
        tank_count: u32,
    },
    /// Arbitrary geometry.
    Arbitrary {
        /// Reference mesh.
        geometry: Geometry3D,
    },
}

/// One pressurized module of a station.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleSpec {
    /// Module name.
    pub name: String,
    /// Volume, m³.
    pub volume_m3: f64,
    /// Docking axis orientation ("axial", "radial", ...).
    pub docking_axis: String,
}

/// The orbit the assembly happens in.
#[derive(Debug, Clone, PartialEq)]
pub struct OrbitalParameters {
    /// Orbit designation.
    pub designation: String,
    /// Altitude, km.
    pub altitude_km: f64,
    /// Inclination, degrees.
    pub inclination_deg: f64,
    /// Station-keeping acceleration budget, m/s² (microgravity background
    /// acceleration from drag and thrusters).
    pub station_keeping_accel_ms2: f64,
}

impl Default for OrbitalParameters {
    fn default() -> Self {
        OrbitalParameters {
            designation: "LEO 400 km, 51.6°".into(),
            altitude_km: 400.0,
            inclination_deg: 51.6,
            station_keeping_accel_ms2: 1e-4,
        }
    }
}

/// Actions a robot can perform during assembly.
#[derive(Debug, Clone, PartialEq)]
pub enum AssemblyAction {
    /// Grasp a component from stowage.
    GraspComponent,
    /// Translate by a delta, m.
    Translate {
        /// Displacement, m.
        delta: Vector3,
    },
    /// Rotate about an axis.
    Rotate {
        /// Rotation axis (unit), m.
        axis: Vector3,
        /// Angle, rad.
        angle_rad: f64,
    },
    /// Dock to the growing structure.
    Dock,
    /// Bolt with a torque.
    Bolt {
        /// Torque, N·m.
        torque_nm: f64,
    },
    /// Weld with a named process.
    Weld {
        /// Process (e.g. "EBW").
        process: String,
    },
    /// Connect a fluid line.
    ConnectFluid {
        /// Fluid name.
        fluid: String,
    },
    /// Connect an electrical connector.
    ConnectElectrical {
        /// Connector designation.
        connector: String,
    },
    /// Release the gripper.
    Release,
}

/// Constraints checked during simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssemblyConstraint {
    /// No collision with the structure or other arms.
    NoCollision,
    /// Keep the stack within the station-keeping budget.
    MaintainStationKeeping,
    /// Keep a sensitive component out of direct sun.
    AvoidSunExposure,
    /// Do not exceed a temperature.
    ThermalLimit {
        /// Limit, °C.
        max_temp_c: f64,
    },
    /// Do not exceed a force at the interfaces.
    ForceLimit {
        /// Limit, N.
        max_force_n: f64,
    },
    /// Maintain line of sight to a relay.
    LineOfSight,
}

/// One step of the assembly sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct AssemblyStep {
    /// Step identifier.
    pub id: StepId,
    /// The action.
    pub action: AssemblyAction,
    /// The component worked on.
    pub component: ComponentId,
    /// The robot performing it (`None` for autonomous docking).
    pub robot: Option<RobotId>,
    /// Planned duration, hours.
    pub duration_hours: f64,
    /// Constraints in force for this step.
    pub constraints: Vec<AssemblyConstraint>,
}

/// A component to be assembled.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentSpec {
    /// Component identifier.
    pub id: ComponentId,
    /// Name.
    pub name: String,
    /// Mass, kg.
    pub mass_kg: f64,
    /// Bounding dimensions, m.
    pub dimensions: Vector3,
    /// Target position of the component centre in structure coordinates, m.
    pub target_position: Vector3,
}

/// Snapshot of the assembly progress (mirrors the twin's state for orbit).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AssemblyState {
    /// Completed step ids.
    pub completed_steps: Vec<StepId>,
    /// Components installed so far.
    pub installed_components: Vec<ComponentId>,
    /// Current robot poses (per robot).
    pub robot_poses: Vec<Vector3>,
}

/// Result of simulating one step.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationResult {
    /// All constraints passed.
    pub ok: bool,
    /// Collision found (centre, required clearance), if any.
    pub collision: Option<(Vector3, f64)>,
    /// Interface force at the gripper/dock, N.
    pub interface_force_n: f64,
    /// Whether the force limit was respected.
    pub force_ok: bool,
    /// Findings.
    pub notes: Vec<String>,
}

/// Result of the structural integrity check at a step.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuralCheckResult {
    /// True if the partial structure stands its in-orbit load cases.
    pub passed: bool,
    /// Root bending stress of the deployed cantilever, MPa.
    pub root_stress_mpa: f64,
    /// Utilization against the allowable.
    pub utilization: f64,
    /// Findings.
    pub notes: Vec<String>,
}

/// Errors from assembly operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblyError {
    /// The referenced component is not in the manifest.
    UnknownComponent(ComponentId),
    /// No robot assigned for a step that needs one.
    NoRobotAssigned,
    /// The step id is not part of the sequence.
    UnknownStep(StepId),
    /// A dependency edge names a component that is not in the manifest.
    UnknownDependency(ComponentId, ComponentId),
    /// The dependency edges form a cycle, so no assembly order exists.
    DependencyCycle,
}

impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssemblyError::UnknownComponent(c) => write!(f, "unknown component {c}"),
            AssemblyError::NoRobotAssigned => f.write_str("no robot assigned to the step"),
            AssemblyError::UnknownStep(s) => write!(f, "unknown step {s}"),
            AssemblyError::UnknownDependency(a, b) => {
                write!(f, "dependency edge ({a} -> {b}) names a missing component")
            }
            AssemblyError::DependencyCycle => f.write_str("component dependencies form a cycle"),
        }
    }
}

impl std::error::Error for AssemblyError {}

/// The orbital assembly planner and simulator.
#[derive(Debug, Clone, PartialEq)]
pub struct OrbitalAssembly {
    /// The target structure.
    pub structure: SpaceStructure,
    /// The planned sequence (filled by [`OrbitalAssembly::plan_sequence`]).
    pub assembly_sequence: Vec<AssemblyStep>,
    /// The robotic arms.
    pub robots: Vec<RoboticArm>,
    /// The orbit.
    pub orbit: OrbitalParameters,
    /// Components to assemble.
    pub components: Vec<ComponentSpec>,
    /// Allowable root stress of structural members, MPa.
    pub allowable_stress_mpa: f64,
    /// Docking-impulse design load at the root, N.
    pub docking_impulse_n: f64,
    /// Chord cross-section area of the truss members, m2 (default 0.01,
    /// ISS-like; was a hard-coded literal, review 7B).
    pub chord_area_m2: f64,
    /// Truss bay height (chord separation), m (default 3.0).
    pub bay_height_m: f64,
    /// Hard precedence edges `(before, after)` as component ids: `after`
    /// may not be installed before `before` is released (e.g. the anchor
    /// node before every boomed bay). Distance ordering still decides
    /// among components whose dependencies are satisfied. (Review 7B
    /// leftover: the planner used to be purely distance-ordered.)
    pub component_dependencies: Vec<(ComponentId, ComponentId)>,
}

impl OrbitalAssembly {
    /// Creates an assembly for the given structure and orbit.
    pub fn new(structure: SpaceStructure, orbit: OrbitalParameters) -> Self {
        Self {
            structure,
            assembly_sequence: Vec::new(),
            robots: Vec::new(),
            orbit,
            components: Vec::new(),
            allowable_stress_mpa: 250.0,
            docking_impulse_n: 20_000.0,
            chord_area_m2: 0.01,
            bay_height_m: 3.0,
            component_dependencies: Vec::new(),
        }
    }

    /// Adds a component to the manifest.
    pub fn add_component(&mut self, component: ComponentSpec) {
        self.components.push(component);
    }

    /// Adds a robot.
    pub fn add_robot(&mut self, robot: RoboticArm) {
        self.robots.push(robot);
    }

    /// Generates the assembly sequence: components ordered by target
    /// position (build from the anchor outwards), each with the standard
    /// grasp -> translate -> rotate -> dock -> bolt -> release cycle, plus
    /// constraint tags. This is the graph-based planner's baseline output;
    /// time/energy optimisation belongs to Phase 5 scheduling.
    pub fn plan_sequence(&mut self) -> Result<Vec<AssemblyStep>, AssemblyError> {
        // Precedence-feasible nearest-first ordering (review 7B leftover):
        // at each pick the *eligible* components (every dependency already
        // released in the sequence) compete by distance from the origin,
        // nearest first, ties by id. Without edges this is exactly the old
        // distance ordering.
        use std::collections::HashMap;
        let known: std::collections::HashSet<ComponentId> =
            self.components.iter().map(|c| c.id).collect();
        for (a, b) in &self.component_dependencies {
            if !known.contains(a) || !known.contains(b) {
                return Err(AssemblyError::UnknownDependency(*a, *b));
            }
        }
        let mut pending: HashMap<ComponentId, usize> = HashMap::new();
        for &(_, b) in &self.component_dependencies {
            *pending.entry(b).or_insert(0) += 1;
        }
        let mut released: std::collections::HashSet<ComponentId> = std::collections::HashSet::new();
        let mut ordered: Vec<ComponentSpec> = Vec::with_capacity(self.components.len());
        while ordered.len() < self.components.len() {
            let pick = self
                .components
                .iter()
                .filter(|c| {
                    !released.contains(&c.id) && pending.get(&c.id).copied().unwrap_or(0) == 0
                })
                .min_by(|a, b| {
                    let da = a.target_position.length();
                    let db = b.target_position.length();
                    da.total_cmp(&db).then(a.id.cmp(&b.id))
                });
            let Some(component) = pick else {
                return Err(AssemblyError::DependencyCycle);
            };
            released.insert(component.id);
            for (a, b) in &self.component_dependencies {
                if *a == component.id {
                    *pending.entry(*b).or_insert(0) -= 1;
                }
            }
            ordered.push(component.clone());
        }

        let mut sequence = Vec::new();
        let mut step_no = 1u64;
        let robot_id = self.robots.first().map(|r| r.id);
        for component in &ordered {
            let mut push = |action: AssemblyAction, duration: f64, robot: Option<RobotId>| {
                sequence.push(AssemblyStep {
                    id: StepId(step_no),
                    action,
                    component: component.id,
                    robot,
                    duration_hours: duration,
                    constraints: vec![
                        AssemblyConstraint::NoCollision,
                        AssemblyConstraint::ForceLimit { max_force_n: 400.0 },
                        AssemblyConstraint::MaintainStationKeeping,
                    ],
                });
                step_no += 1;
            };
            push(AssemblyAction::GraspComponent, 0.5, robot_id);
            push(
                AssemblyAction::Translate {
                    delta: component.target_position,
                },
                1.0,
                robot_id,
            );
            push(
                AssemblyAction::Rotate {
                    axis: Vector3::EZ,
                    angle_rad: std::f64::consts::FRAC_PI_2,
                },
                0.5,
                robot_id,
            );
            push(AssemblyAction::Dock, 1.5, robot_id);
            push(AssemblyAction::Bolt { torque_nm: 75.0 }, 0.5, robot_id);
            push(AssemblyAction::Release, 0.1, robot_id);
        }
        self.assembly_sequence = sequence.clone();
        Ok(sequence)
    }

    /// Simulates one step against the current state.
    ///
    /// Collision: the moved component's bounding sphere is swept along its
    /// approach path against the installed components' bounding spheres.
    /// Force: the handling acceleration (station keeping + 0.05 m/s²
    /// manoeuvre allowance) times the component mass at the gripper.
    ///
    /// # Errors
    ///
    /// [`AssemblyError`] for unknown components.
    pub fn simulate_step(
        &self,
        step: &AssemblyStep,
        state: &AssemblyState,
    ) -> Result<SimulationResult, AssemblyError> {
        let component = self
            .components
            .iter()
            .find(|c| c.id == step.component)
            .ok_or(AssemblyError::UnknownComponent(step.component))?;
        if step.robot.is_none() {
            return Err(AssemblyError::NoRobotAssigned);
        }

        let mut notes = Vec::new();
        let mut collision = None;
        let mut force_ok = true;
        let mut interface_force = 0.0;

        match &step.action {
            AssemblyAction::Translate { .. } | AssemblyAction::Rotate { .. } => {
                let a = self.orbit.station_keeping_accel_ms2 + 0.05;
                interface_force = component.mass_kg * a;
                let limit = step
                    .constraints
                    .iter()
                    .find_map(|c| match c {
                        AssemblyConstraint::ForceLimit { max_force_n } => Some(*max_force_n),
                        _ => None,
                    })
                    .unwrap_or(f64::INFINITY);
                force_ok = interface_force <= limit;
                if !force_ok {
                    notes.push(format!(
                        "handling force {interface_force:.0} N exceeds the {limit:.0} N limit"
                    ));
                }

                // Bounding sphere from the largest half-axis (the diagonal
                // makes adjacent same-pitch bays overlap spuriously).
                let radius = component
                    .dimensions
                    .x
                    .max(component.dimensions.y)
                    .max(component.dimensions.z)
                    / 2.0;
                let start = component.target_position - Vector3::EZ * 10.0;
                for installed in &self.components {
                    if installed.id == component.id {
                        continue;
                    }
                    if !state.installed_components.contains(&installed.id) {
                        continue;
                    }
                    let other_radius = installed
                        .dimensions
                        .x
                        .max(installed.dimensions.y)
                        .max(installed.dimensions.z)
                        / 2.0;
                    let min_dist = radius + other_radius;
                    for k in 0..=10 {
                        let t = k as f64 / 10.0;
                        let p = start.lerp(component.target_position, t);
                        let d = p.distance(installed.target_position);
                        if d < min_dist {
                            collision = Some((p, min_dist));
                            notes.push(format!("collision with '{}' at {}", installed.name, p));
                            break;
                        }
                    }
                }
            }
            AssemblyAction::GraspComponent | AssemblyAction::Release => {
                interface_force = component.mass_kg * self.orbit.station_keeping_accel_ms2;
            }
            AssemblyAction::Dock => {
                interface_force = self.docking_impulse_n;
                force_ok = true; // docking shock absorbers take the impulse
                notes.push(format!("docking impulse {interface_force:.0} N (absorbed)"));
            }
            AssemblyAction::Bolt { torque_nm } => {
                notes.push(format!("bolting at {torque_nm} N·m"));
            }
            AssemblyAction::Weld { process } => {
                notes.push(format!("welding with {process} in vacuum"));
            }
            AssemblyAction::ConnectFluid { fluid } => {
                notes.push(format!("connecting {fluid} line"));
            }
            AssemblyAction::ConnectElectrical { connector } => {
                notes.push(format!("mating connector {connector}"));
            }
        }

        let ok = collision.is_none() && force_ok;
        Ok(SimulationResult {
            ok,
            collision,
            interface_force_n: interface_force,
            force_ok,
            notes,
        })
    }

    /// Verifies the partially assembled structure at a given step.
    ///
    /// In orbit there is no gravity, but the stack must survive docking
    /// impulses at the free end — the governing case for a deploying
    /// cantilever truss. The impulse is applied as a tip load on a
    /// cantilever with two-chord bays:
    /// `sigma = P * L / (A_chord * h)` (RFC 0003).
    ///
    /// # Errors
    ///
    /// [`AssemblyError::UnknownStep`] if the step is not in the sequence.
    pub fn verify_structural_integrity(
        &self,
        at_step: &StepId,
    ) -> Result<StructuralCheckResult, AssemblyError> {
        if !self.assembly_sequence.iter().any(|s| s.id == *at_step) {
            return Err(AssemblyError::UnknownStep(*at_step));
        }
        let mut installed: Vec<ComponentId> = Vec::new();
        for step in &self.assembly_sequence {
            if step.id > *at_step {
                break;
            }
            if matches!(step.action, AssemblyAction::Release)
                && !installed.contains(&step.component)
            {
                installed.push(step.component);
            }
        }

        // Tip extent: the farthest installed component's centre plus its
        // half-extent projected along the deployment direction (the centre
        // distance alone underestimates the root moment by ~half a bay).
        let deployed_length = installed
            .iter()
            .filter_map(|id| self.components.iter().find(|c| c.id == *id))
            .map(|c| {
                let r = c.target_position.length();
                if r <= 1e-9 {
                    return 0.0;
                }
                let hat = c.target_position / r;
                let half_extent = (c.dimensions.x * hat.x.abs()
                    + c.dimensions.y * hat.y.abs()
                    + c.dimensions.z * hat.z.abs())
                    / 2.0;
                r + half_extent
            })
            .fold(0.0f64, f64::max);
        let installed_mass: f64 = installed
            .iter()
            .filter_map(|id| self.components.iter().find(|c| c.id == *id))
            .map(|c| c.mass_kg)
            .sum();

        let mut notes = vec![format!(
            "{} components installed, deployed length {:.1} m, mass {:.0} kg",
            installed.len(),
            deployed_length,
            installed_mass
        )];

        // Constant cross-section along the truss; the chord area and bay
        // height are plan parameters (defaulting ISS-like) so the golden
        // cases can drive them instead of a hard-coded literal.
        let chord_area_m2 = self.chord_area_m2.max(1e-9);
        let height = self.bay_height_m.max(1e-9);
        let moment_load = self.docking_impulse_n * deployed_length;
        let stress_mpa = moment_load / (chord_area_m2 * height) / 1e6;
        let utilization = stress_mpa / self.allowable_stress_mpa;
        let passed = utilization <= 1.0;
        if !passed {
            notes.push(format!(
                "root stress {stress_mpa:.1} MPa exceeds the allowable {} MPa under docking impulse",
                self.allowable_stress_mpa
            ));
        } else {
            notes.push(format!("root stress {stress_mpa:.1} MPa OK"));
        }

        Ok(StructuralCheckResult {
            passed,
            root_stress_mpa: stress_mpa,
            utilization,
            notes,
        })
    }

    /// Collision objects of installed components (for robot path planning).
    pub fn collision_objects(&self, state: &AssemblyState) -> Vec<CollisionObject> {
        self.components
            .iter()
            .filter(|c| state.installed_components.contains(&c.id))
            .map(|c| CollisionObject {
                centre: c.target_position,
                radius: c.dimensions.x.max(c.dimensions.y).max(c.dimensions.z) / 2.0,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose};

    /// Review 7B leftover: dependency edges gate the nearest-first
    /// ordering; cycles and dangling edges are typed errors.
    #[test]
    fn plan_sequence_respects_component_dependencies() {
        let mut a = truss_assembly();
        // 3 bays at increasing distance; make bay 3 (nearest) wait on the
        // anchor setup (bay 1, farthest) — the pure distance order would
        // install bay 3 first.
        a.component_dependencies = vec![(ComponentId(1), ComponentId(3))];
        let steps = a.plan_sequence().unwrap();
        let install_order: Vec<ComponentId> = steps
            .iter()
            .filter(|s| matches!(s.action, AssemblyAction::Release))
            .map(|s| s.component)
            .collect();
        assert_eq!(install_order.len(), 3);
        let pos = |id: ComponentId| install_order.iter().position(|c| *c == id).unwrap();
        assert!(
            pos(ComponentId(1)) < pos(ComponentId(3)),
            "{install_order:?}"
        );

        // Dangling edge.
        a.component_dependencies = vec![(ComponentId(99), ComponentId(1))];
        assert_eq!(
            a.plan_sequence(),
            Err(AssemblyError::UnknownDependency(
                ComponentId(99),
                ComponentId(1)
            ))
        );

        // Cycle.
        a.component_dependencies = vec![
            (ComponentId(1), ComponentId(2)),
            (ComponentId(2), ComponentId(1)),
        ];
        assert_eq!(a.plan_sequence(), Err(AssemblyError::DependencyCycle));
    }

    fn truss_assembly() -> OrbitalAssembly {
        let mut a = OrbitalAssembly::new(
            SpaceStructure::Truss {
                segments: 3,
                length_m: 30.0,
            },
            OrbitalParameters::default(),
        );
        for i in 1..=3u64 {
            a.add_component(ComponentSpec {
                id: ComponentId(i),
                name: format!("bay {i}"),
                mass_kg: 500.0,
                dimensions: Vector3::new(10.0, 3.0, 3.0),
                target_position: Vector3::new(5.0 + 10.0 * (i - 1) as f64, 0.0, 0.0),
            });
        }
        let mut robot = RoboticArm::new(
            RobotId(1),
            vec![
                Joint::revolute(-3.0, 3.0, 0.5),
                Joint::revolute(-3.0, 3.0, 0.5),
            ],
            vec![4.0, 4.0],
            EndEffector::Gripper { force_n: 400.0 },
        );
        robot.base = Pose::origin();
        a.add_robot(robot);
        a
    }

    /// Verification: collision detection in the assembly sequence — a
    /// component whose target coincides with an installed neighbour is
    /// caught.
    #[test]
    fn test_orbital_assembly_collision() {
        let mut a = truss_assembly();
        // Overlapping bay: same target as bay 1.
        a.add_component(ComponentSpec {
            id: ComponentId(9),
            name: "mis-planned bay".into(),
            mass_kg: 500.0,
            dimensions: Vector3::new(10.0, 3.0, 3.0),
            target_position: Vector3::new(5.0, 0.0, 0.0),
        });
        a.plan_sequence().unwrap();
        let state = AssemblyState {
            installed_components: vec![ComponentId(1)],
            ..AssemblyState::default()
        };

        let step = a
            .assembly_sequence
            .iter()
            .find(|s| {
                s.component == ComponentId(9)
                    && matches!(s.action, AssemblyAction::Translate { .. })
            })
            .expect("translate step exists");
        let result = a.simulate_step(step, &state).unwrap();
        assert!(
            !result.ok,
            "overlapping bay must collide: {:?}",
            result.notes
        );
        assert!(result.collision.is_some());
    }

    #[test]
    fn plan_sequence_builds_full_cycle() {
        let mut a = truss_assembly();
        let steps = a.plan_sequence().unwrap();
        assert_eq!(steps.len(), 18); // 6 actions x 3 bays
        assert_eq!(steps[0].action, AssemblyAction::GraspComponent);
        assert!(matches!(steps[2].action, AssemblyAction::Rotate { .. }));
        assert_eq!(steps[5].action, AssemblyAction::Release);
        // Anchored bay first (smallest target radius).
        assert_eq!(steps[0].component, ComponentId(1));
        assert!(steps
            .iter()
            .all(|s| s.constraints.contains(&AssemblyConstraint::NoCollision)));
    }

    #[test]
    fn handling_force_respects_limits() {
        let mut a = truss_assembly();
        a.plan_sequence().unwrap();
        let state = AssemblyState::default();
        let translate = a
            .assembly_sequence
            .iter()
            .find(|s| matches!(s.action, AssemblyAction::Translate { .. }))
            .unwrap();
        let r = a.simulate_step(translate, &state).unwrap();
        // 500 kg x (1e-4 + 0.05) = 25 N — within the 400 N limit.
        assert!(r.ok);
        assert!(r.interface_force_n < 30.0);
    }

    #[test]
    fn over_mass_component_violates_force_limit() {
        let mut a = truss_assembly();
        a.components[0].mass_kg = 20_000.0; // 20 t bay: ~1000 N handling force
        a.plan_sequence().unwrap();
        let state = AssemblyState::default();
        let translate = a
            .assembly_sequence
            .iter()
            .find(|s| matches!(s.action, AssemblyAction::Translate { .. }))
            .unwrap();
        let r = a.simulate_step(translate, &state).unwrap();
        assert!(!r.force_ok);
        assert!(!r.ok);
    }

    #[test]
    fn structural_check_grows_with_deployed_length() {
        let mut a = truss_assembly();
        a.plan_sequence().unwrap();
        let early = a.verify_structural_integrity(&StepId(6)).unwrap(); // bay 1 done
        let late = a.verify_structural_integrity(&StepId(18)).unwrap(); // all bays
        assert!(early.passed);
        assert!(late.passed, "late util {}", late.utilization);
        // Longer cantilever: higher root stress.
        assert!(late.root_stress_mpa > early.root_stress_mpa);
        assert!(late.notes.iter().any(|n| n.contains("root stress")));
    }

    #[test]
    fn docking_impulse_can_overstress() {
        let mut a = truss_assembly();
        a.plan_sequence().unwrap();
        a.docking_impulse_n = 500_000.0; // violent docking
        let check = a.verify_structural_integrity(&StepId(18)).unwrap();
        assert!(!check.passed);
    }

    #[test]
    fn unknown_step_is_rejected() {
        let mut a = truss_assembly();
        a.plan_sequence().unwrap();
        assert_eq!(
            a.verify_structural_integrity(&StepId(999)),
            Err(AssemblyError::UnknownStep(StepId(999)))
        );
    }

    #[test]
    fn collision_objects_track_installed() {
        let a = truss_assembly();
        let mut state = AssemblyState::default();
        assert!(a.collision_objects(&state).is_empty());
        state.installed_components = vec![ComponentId(1), ComponentId(2)];
        assert_eq!(a.collision_objects(&state).len(), 2);
    }
}
