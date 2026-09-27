//! WebAssembly bindings for interactive shipyard construction dashboards
//! (spec §7).
//!
//! The crate wraps the digital twin and the orbital assembly planner behind
//! `wasm-bindgen` façades so a browser can drive a build: advance phases,
//! read geometry for rendering, and pull weight/CoG and structural-check
//! reports as JSON.
//!
//! The façades hold no rendering state — the browser draws; the engine
//! computes. All methods are `#[wasm_bindgen]`-exported; the crate also
//! compiles and unit-tests on native targets (wasm-bindgen degrades to
//! plain Rust there).
//!
//! # Usage from JS (after `wasm-bindgen` bundling)
//!
//! ```js
//! const twin = WasmDigitalTwin.new(projectJson);
//! const ok = twin.advance_phase(0);   // erect the first block
//! const verts = twin.get_geometry();  // Float64Array triples
//! const report = JSON.parse(twin.get_weight_report());
//! ```

use std::collections::HashSet;

use wasm_bindgen::prelude::*;

use tpt_yard_assembly::ActivityId;
#[allow(unused_imports)]
use tpt_yard_core::{ActivityType, AssemblyActivity, PhaseId};
use tpt_yard_core::{Geometry3D, ItemId, RobotId, Vector3, VesselProject};
use tpt_yard_digital_twin::{DigitalTwin, SupportCondition};
use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

/// A construction digital twin driven from the browser.
#[wasm_bindgen]
pub struct WasmDigitalTwin {
    twin: DigitalTwin,
    /// Ordered activities across phases (drives the demo geometry layout).
    activity_ids: Vec<ActivityId>,
}

#[wasm_bindgen]
impl WasmDigitalTwin {
    /// Loads a vessel project from JSON (the `VesselProject` schema) and
    /// builds the twin: one weight item per activity (design weight taken
    /// from the phase weights, distributed evenly), keel-block support
    /// spanning the plan.
    ///
    /// # Errors
    ///
    /// Panics with a message on malformed JSON — the JS side is expected to
    /// validate first via `TryFrom`-style handling in its own loader.
    #[wasm_bindgen(constructor)]
    pub fn new(project_json: &str) -> WasmDigitalTwin {
        let project = VesselProject::from_json_str(project_json)
            .unwrap_or_else(|e| panic!("invalid project json: {e}"));

        // One activity + weight item per erection step, laid out along +X.
        let n: usize = project
            .build_phases
            .iter()
            .map(|p| p.activities.len())
            .sum();
        let design_total: f64 = project
            .build_phases
            .iter()
            .map(|p| p.weight_state.design_kg)
            .sum();
        let per_block = if n > 0 { design_total / n as f64 } else { 0.0 };

        let mut project = project;
        let mut weight = WeightModel::new(design_total, Vector3::ZERO);
        let mut activity_ids = Vec::with_capacity(n);
        let mut seq = 0usize;
        for phase in project.build_phases.iter_mut() {
            for act in phase.activities.iter_mut() {
                let aid = act.id;
                activity_ids.push(aid);
                weight.add_item(WeightItem {
                    id: ItemId(seq as u64 + 1),
                    name: act.name.clone(),
                    group: phase.name.clone(),
                    weight_kg: per_block,
                    cog: Vector3::new(seq as f64 * 10.0, 0.0, 5.0),
                    status: ItemStatus::Design,
                    margin_pct: 0.0,
                    installed_by: Some(aid),
                });
                seq += 1;
            }
        }

        let span = (n.max(1) as f64) * 10.0;
        let twin = DigitalTwin::with_weight_model(
            project,
            weight,
            SupportCondition::KeelBlocks {
                positions: vec![
                    Vector3::new(-span * 0.1, -4.0, 0.0),
                    Vector3::new(-span * 0.1, 4.0, 0.0),
                    Vector3::new(span * 1.1, -4.0, 0.0),
                    Vector3::new(span * 1.1, 4.0, 0.0),
                ],
            },
        );
        WasmDigitalTwin { twin, activity_ids }
    }

    /// Advances construction by completing the next activity in plan
    /// order. Returns true if the twin is still structurally sound.
    pub fn advance_next(&mut self) -> bool {
        let next = self
            .activity_ids
            .iter()
            .find(|a| !self.twin.assembly_state.completed_activities.contains(a));
        let Some(next) = next else { return true }; // nothing left: sound
        self.twin.advance_phase(next).is_ok()
    }

    /// Completes a specific activity. Returns true on success.
    pub fn advance_phase(&mut self, activity: u64) -> bool {
        self.twin.advance_phase(&ActivityId(activity)).is_ok()
    }

    /// Number of completed activities.
    pub fn completed_count(&self) -> usize {
        self.twin.assembly_state.completed_activities.len()
    }

    /// Current phase id.
    pub fn current_phase(&self) -> u64 {
        self.twin.vessel.current_phase.0
    }

    /// Geometry of the current build state as a flat triangle soup:
    /// 9 floats per triangle (3 vertices × xyz), metres. Each completed
    /// erection step contributes one box in the plan layout — a dashboard
    /// uploads this straight into a WebGL buffer.
    pub fn get_geometry(&self) -> Vec<f32> {
        let mut geometry = Geometry3D::new();
        let completed: HashSet<ActivityId> = self
            .twin
            .assembly_state
            .completed_activities
            .iter()
            .copied()
            .collect();
        for (i, aid) in self.activity_ids.iter().enumerate() {
            if !completed.contains(aid) {
                continue;
            }
            let mut block = Geometry3D::from_box(9.0, 8.0, 3.0);
            block.translate(Vector3::new(i as f64 * 10.0, 0.0, 1.5));
            geometry.merge(&block, Vector3::ZERO);
        }
        geometry
            .vertices
            .iter()
            .flat_map(|v| [v.x as f32, v.y as f32, v.z as f32])
            .collect()
    }

    /// Vertex indices of the geometry (flat, 3 per triangle).
    pub fn get_geometry_indices(&self) -> Vec<u32> {
        let completed: HashSet<ActivityId> = self
            .twin
            .assembly_state
            .completed_activities
            .iter()
            .copied()
            .collect();
        let mut count = 0u32;
        for aid in &self.activity_ids {
            if completed.contains(aid) {
                count += 1;
            }
        }
        // 12 triangles per box, 3 indices each.
        let mut indices = Vec::with_capacity(count as usize * 36);
        for b in 0..count {
            for t in 0..12u32 {
                indices.push(b * 8 + t * 3);
                indices.push(b * 8 + t * 3 + 1);
                indices.push(b * 8 + t * 3 + 2);
            }
        }
        indices
    }

    /// Weight and CoG report as a JSON string.
    pub fn get_weight_report(&self) -> String {
        let dev = self.twin.weight_deviation();
        let report = self.twin.weight_model().weight_report();
        // Dashboards track the *as-built* CoG; fall back to the best
        // estimate before anything is installed.
        let cog = self
            .twin
            .weight_model()
            .installed_centre_of_gravity()
            .unwrap_or(report.cog);
        format!(
            r#"{{"installed_kg":{:.1},"best_estimate_kg":{:.1},"design_kg":{:.1},"deviation_kg":{:.1},"cog":[{:.3},{:.3},{:.3}],"completed":{}}}"#,
            dev.as_built_installed_kg,
            dev.best_estimate_kg,
            dev.design_kg,
            dev.deviation_kg,
            cog.x,
            cog.y,
            cog.z,
            self.completed_count()
        )
    }

    /// Structural check result as a JSON string.
    pub fn structural_check(&self) -> String {
        let check = self.twin.structural_check();
        let reactions: Vec<String> = check
            .reactions_kn
            .iter()
            .map(|(name, r)| format!(r#""{name}":{r:.1}"#))
            .collect();
        format!(
            r#"{{"passed":{},"margin":{:.4},"weight_kg":{:.1},"reactions":{{{}}}}}"#,
            check.passed,
            check.longitudinal_margin,
            check.weight_kg,
            reactions.join(",")
        )
    }
}

/// An orbital assembly sequence driven from the browser.
#[wasm_bindgen]
pub struct WasmOrbitalAssembly {
    inner: tpt_yard_orbital_assembly::OrbitalAssembly,
    state: tpt_yard_orbital_assembly::AssemblyState,
    next_step: usize,
}

#[wasm_bindgen]
impl WasmOrbitalAssembly {
    /// Builds a linear truss assembly of `bays` bays with one robot.
    #[wasm_bindgen(constructor)]
    pub fn new(bays: u32) -> WasmOrbitalAssembly {
        let mut inner = tpt_yard_orbital_assembly::OrbitalAssembly::new(
            tpt_yard_orbital_assembly::SpaceStructure::Truss {
                segments: bays,
                length_m: bays as f64 * 5.0,
            },
            tpt_yard_orbital_assembly::OrbitalParameters::default(),
        );
        for i in 1..=bays as u64 {
            inner.add_component(tpt_yard_orbital_assembly::ComponentSpec {
                id: tpt_yard_core::ComponentId(i),
                name: format!("bay {i}"),
                mass_kg: 500.0,
                dimensions: Vector3::new(5.0, 3.0, 3.0),
                target_position: Vector3::new(5.0 * i as f64 - 2.5, 0.0, 0.0),
            });
        }
        let robot = tpt_yard_robotic_assembly::RoboticArm::new(
            RobotId(1),
            vec![
                tpt_yard_robotic_assembly::Joint::revolute(-3.0, 3.0, 0.5),
                tpt_yard_robotic_assembly::Joint::revolute(-3.0, 3.0, 0.5),
            ],
            vec![4.0, 4.0],
            tpt_yard_robotic_assembly::EndEffector::Gripper { force_n: 400.0 },
        );
        inner.add_robot(robot);
        inner.plan_sequence();
        WasmOrbitalAssembly {
            inner,
            state: tpt_yard_orbital_assembly::AssemblyState::default(),
            next_step: 0,
        }
    }

    /// Simulates the next step of the sequence; returns false when the
    /// sequence is exhausted or the step violates constraints.
    pub fn simulate_next_step(&mut self) -> bool {
        let Some(step) = self.inner.assembly_sequence.get(self.next_step) else {
            return false;
        };
        let step = step.clone();
        let result = self
            .inner
            .simulate_step(&step, &self.state)
            .expect("step is valid");
        if !result.ok {
            return false;
        }
        if matches!(
            step.action,
            tpt_yard_orbital_assembly::AssemblyAction::Release
        ) {
            self.state.installed_components.push(step.component);
        }
        self.state.completed_steps.push(step.id);
        self.next_step += 1;
        true
    }

    /// Number of steps simulated so far.
    pub fn steps_done(&self) -> usize {
        self.next_step
    }

    /// Robot pose for visualisation: [x, y, z, yaw] of the first arm.
    pub fn get_robot_pose(&self) -> Vec<f32> {
        match self.inner.robots.first() {
            Some(robot) => {
                let p = robot.base;
                vec![
                    p.position.x as f32,
                    p.position.y as f32,
                    p.position.z as f32,
                    p.yaw_rad as f32,
                ]
            }
            None => vec![0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Installed components as a flat [id, x, y, z, ...] array.
    pub fn get_installed(&self) -> Vec<f32> {
        self.state
            .installed_components
            .iter()
            .filter_map(|id| self.inner.components.iter().find(|c| c.id == *id))
            .flat_map(|c| {
                vec![
                    c.id.0 as f32,
                    c.target_position.x as f32,
                    c.target_position.y as f32,
                    c.target_position.z as f32,
                ]
            })
            .collect()
    }
}

// Native-side logic tests (wasm-bindgen compiles on native too).
#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_core::ConstructionMethod;

    fn sample_project_json() -> String {
        let mut phase = tpt_yard_core::BuildPhase::new(PhaseId(1), "Erection", 4.0);
        for i in 1..=3u64 {
            let deps: Vec<ActivityId> = if i == 1 {
                vec![]
            } else {
                vec![ActivityId(i - 1)]
            };
            phase.activities.push(
                AssemblyActivity::new(
                    ActivityId(i),
                    format!("Erect block {i}"),
                    ActivityType::JoinBlock,
                    6.0,
                )
                .with_dependencies(&deps),
            );
        }
        phase.weight_state = tpt_yard_core::WeightState {
            design_kg: 3_000.0,
            installed_kg: 0.0,
        };
        let project = VesselProject::new(
            tpt_yard_core::ProjectId(1),
            "WASM demo barge",
            tpt_yard_core::VesselType::Sea(tpt_yard_core::SeaVesselType::FishingVessel),
            ConstructionMethod::SeaDrydock,
            vec![phase],
        )
        .unwrap();
        project.to_json().to_string_compact()
    }

    #[test]
    fn twin_advances_and_reports() {
        let json = sample_project_json();
        let mut twin = WasmDigitalTwin::new(&json);
        assert_eq!(twin.completed_count(), 0);
        assert!(twin.advance_next());
        assert_eq!(twin.completed_count(), 1);
        assert_eq!(twin.current_phase(), 1); // single-phase project stays put

        let geometry = twin.get_geometry();
        // One box = 8 vertices = 24 floats.
        assert_eq!(geometry.len(), 24);
        assert_eq!(twin.get_geometry_indices().len(), 36);

        let report = twin.get_weight_report();
        assert!(report.contains("\"installed_kg\":1000.0"), "{report}");
        let check = twin.structural_check();
        assert!(check.contains("\"passed\":true"), "{check}");

        // Finish the build.
        assert!(twin.advance_next());
        assert!(twin.advance_next());
        assert!(twin.advance_next()); // exhausted: still sound
        assert_eq!(twin.completed_count(), 3);
    }

    #[test]
    fn specific_phase_advance_gated() {
        let json = sample_project_json();
        let mut twin = WasmDigitalTwin::new(&json);
        // Activity 2 depends on 1: gated.
        assert!(!twin.advance_phase(2));
        assert!(twin.advance_phase(1));
        assert!(twin.advance_phase(2));
    }

    #[test]
    fn orbital_assembly_wasm_facade() {
        let mut asm = WasmOrbitalAssembly::new(3);
        assert_eq!(asm.get_installed().len(), 0);
        // 18 steps: 3 bays x 6 actions.
        for _ in 0..18 {
            assert!(asm.simulate_next_step());
        }
        assert!(!asm.simulate_next_step()); // exhausted
        assert_eq!(asm.steps_done(), 18);
        assert_eq!(asm.get_installed().len(), 12); // 4 floats x 3 bays
        assert_eq!(asm.get_robot_pose().len(), 4);
    }
}
