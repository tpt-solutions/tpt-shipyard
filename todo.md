# tpt-shipyard — Project TODO

**Org:** TPT Solutions (`tpt-solutions`) · **License:** MIT OR Apache-2.0 · **Repo:** `tpt-shipyard`

A fully open-source computational engine for vehicle construction: sea shipyards, orbital assembly, in-space manufacturing, and construction digital twins. This checklist tracks every deliverable from `spec.txt`, phase by phase.

---

## Phase 0: Repository & Governance Setup

- [ ] Workspace `Cargo.toml` (virtual workspace, `crates/*` members, shared profile/lints)
- [ ] `LICENSE-MIT`
- [ ] `LICENSE-APACHE`
- [ ] `README.md` (per spec Section 12 template)
- [ ] `CONTRIBUTING.md` (DCO sign-off process, no CLA)
- [ ] `SECURITY.md` (private vulnerability disclosure policy)
- [ ] `deny.toml` (allow MIT/Apache-2.0/BSD-2-Clause/BSD-3-Clause/ISC/Zlib/Unicode-3.0; `copyleft = "deny"`; `unlicensed = "deny"`)
- [ ] `rustfmt.toml`
- [ ] `clippy.toml`
- [ ] `.github/workflows/ci.yml`
- [ ] `.github/workflows/license.yml` (cargo-deny gate)
- [ ] `.github/workflows/benchmark.yml`
- [ ] `.github/workflows/release.yml`
- [ ] `.github/ISSUE_TEMPLATE/`
- [ ] `.github/PULL_REQUEST_TEMPLATE.md`
- [ ] `docs/book/` scaffold (mdBook init)
- [ ] `docs/rfc/` index + `rfcs/` directory scaffold
- [ ] Governance doc: Benevolent Dictator + RFC process, "TPT Shipyard" trademark notice
- [ ] Public GitHub Projects roadmap board
- [ ] SemVer + 6-week release cadence policy doc
- [ ] Empty seed dirs: `test-data/hull-blocks/`, `test-data/orbital-structures/`, `test-data/welding-procedures/`, `test-data/golden/`
- [ ] **Milestone:** empty workspace builds, CI green, license check passes

---

## Phase 1: Core & Digital Twin (Months 1-3)

- [ ] `tpt-yard-core` — fundamental shipyard domain types
  - [ ] Scaffold crate (Cargo.toml, lib.rs, workspace member)
  - [ ] Define `VesselProject`, `VesselType`, `SeaVesselType`, `SpaceVesselType`, `ConstructionMethod`
  - [ ] Define `BuildPhase`, `AssemblyActivity`, `ActivityType`, `Resource`
  - [ ] Unit tests for type construction/(de)serialization
  - [ ] Rustdoc + `docs/book` chapter "Core Types"
- [ ] `tpt-yard-digital-twin` — construction state tracking and simulation
  - [ ] Scaffold crate
  - [ ] Define `DigitalTwin`, `AssemblyState`
  - [ ] Implement `advance_phase`, `structural_check_at_phase`, `weight_deviation`, `centre_of_gravity_tracking`
  - [ ] Verification test: `test_cog_tracking` (CoG shifts predictably as blocks are added)
  - [ ] Rustdoc + `docs/book` chapter "Digital Twin"
- [ ] `tpt-yard-weight` — weight and centre of gravity management
  - [ ] Scaffold crate
  - [ ] Define `WeightModel`, `WeightItem`, `ItemStatus`
  - [ ] Implement `total_weight`, `centre_of_gravity`, `weight_report`
  - [ ] Verification test: `test_block_weight_sum`
  - [ ] Rustdoc + `docs/book` chapter "Weight & CoG"
- [ ] `tpt-yard-assembly` — shared assembly-activity primitives (used by core & digital-twin)
  - [ ] Scaffold crate
  - [ ] Define activity dependency-graph types and activity status tracking
  - [ ] Implement activity dependency resolution helpers
  - [ ] Unit tests for dependency-graph correctness (cycles, ordering)
  - [ ] Rustdoc
- [ ] RFC `0001-digital-twin-state.md` written and merged
- [ ] **Milestone:** track weight and CoG through 10 build phases (demo/test script)

---

## Phase 2: Structural During Construction (Months 4-6)

- [ ] `tpt-yard-structural` — structural analysis at each build phase
  - [ ] Scaffold crate (depends on `tpt-fem`)
  - [ ] Define `ConstructionStructuralSolver`, `ConstructionLoad`
  - [ ] Implement `analyze_at_phase`, `lifting_analysis`, `launch_analysis`
  - [ ] Tests: partial-structure FEM analysis vs known cases
  - [ ] Rustdoc + `docs/book` chapter "Structural Analysis During Construction"
- [ ] `tpt-yard-welding` — welding simulation and distortion control
  - [ ] Scaffold crate
  - [ ] Define `WeldingSimulation`, `WeldProcedure`, `WeldProcess`, `WeldPass`
  - [ ] Implement `thermal_cycle` (Rosenthal equation), `residual_stress`, `distortion`, `welding_sequence_optimization`
  - [ ] Verification test: `test_welding_distortion` vs experimental data
  - [ ] Rustdoc + `docs/book` chapter "Welding Simulation"
- [ ] `tpt-yard-distortion` — block distortion management
  - [ ] Scaffold crate
  - [ ] Define `DistortionControl`, `CorrectionAction`
  - [ ] Implement `deviation_map`, `correction_plan`
  - [ ] Unit tests for correction-plan tolerance thresholds
  - [ ] Rustdoc
- [ ] `tpt-yard-joints` — shared joint-geometry primitives (used by structural & welding)
  - [ ] Scaffold crate
  - [ ] Define joint geometry types (butt, fillet, lap, T-joint, etc.) backing `JointGeometry`
  - [ ] Implement joint geometry/area calculations
  - [ ] Unit tests for joint geometry calculations
  - [ ] Rustdoc
- [ ] RFC `0004-welding-distortion-model.md` written and merged
- [ ] Populate `test-data/welding-procedures/` with sample WPS records
- [ ] Bench: `benches/welding-distortion.rs`
- [ ] Golden data: `test-data/golden/sea/welding-distortion-panel.json`
- [ ] **Milestone:** simulate welding distortion in a hull panel end-to-end

---

## Phase 3: Sea Shipyard (Months 7-12)

- [ ] `tpt-yard-hull` — hull construction and block management
  - [ ] Scaffold crate
  - [ ] Define `HullConstruction`, `HullBlock`, `BlockStatus`, `BlockJoin`
  - [ ] Implement `block_division`, `erection_sequence`
  - [ ] Unit tests for block division against crane/workshop constraints
  - [ ] Rustdoc + `docs/book` chapter "Hull Blocks"
- [ ] `tpt-yard-blocks` — shared block-lifting/handling primitives
  - [ ] Scaffold crate
  - [ ] Define lift-point and sling-load types
  - [ ] Implement sling-load distribution calculations
  - [ ] Unit tests for sling-load distribution
  - [ ] Rustdoc
- [ ] `tpt-yard-drydock` — drydock flooding/ballast sequencing
  - [ ] Scaffold crate
  - [ ] Define drydock flooding/ballast state types
  - [ ] Implement stability-at-each-water-level flooding sequence logic
  - [ ] Unit tests vs `drydock-flooding-sequence.json` golden data
  - [ ] Rustdoc
- [ ] `tpt-yard-launch` — launch calculations
  - [ ] Scaffold crate
  - [ ] Define `LaunchAnalysis`, `LaunchMethod`, `SlipwayLaunchResult`
  - [ ] Implement `slipway_launch`, `drydock_flooding`, `launch_stability`
  - [ ] Verification test: `test_slipway_tip_up`
  - [ ] Rustdoc + `docs/book` chapter "Launch Analysis"
- [ ] `tpt-yard-outfitting` — systems installation and routing
  - [ ] Scaffold crate
  - [ ] Define `OutfittingPlan`, `OutfitSystem`, `Route`
  - [ ] Implement `collision_detection`, `installation_sequence`
  - [ ] Unit tests for collision detection between systems
  - [ ] Rustdoc
- [ ] `tpt-yard-sea-trials` — post-launch sea-trial test planning
  - [ ] Scaffold crate
  - [ ] Define sea-trial test-plan types (speed trials, sea-keeping, class-society acceptance)
  - [ ] Implement trial acceptance-criteria evaluation
  - [ ] Unit tests for trial acceptance criteria
  - [ ] Rustdoc
- [ ] RFC `0002-block-assembly-sequencing.md` written and merged
- [ ] Populate `test-data/hull-blocks/` with sample block definitions
- [ ] Example: `examples/container-ship-block-assembly/`
- [ ] Example: `examples/submarine-pressure-hull/`
- [ ] Example: `examples/drydock-flooding-sequence/`
- [ ] Golden data: `test-data/golden/sea/container-ship-block-division.json`
- [ ] Golden data: `test-data/golden/sea/slipway-launch-stability.json`
- [ ] Golden data: `test-data/golden/sea/drydock-flooding-sequence.json`
- [ ] Bench: `benches/block-lifting.rs`
- [ ] Bench: `benches/launch-stability.rs`
- [ ] **Milestone:** plan block erection sequence for a container ship end-to-end

---

## Phase 4: Space Shipyard (Months 13-18)

- [ ] `tpt-yard-orbital-assembly` — orbital assembly planning and simulation
  - [ ] Scaffold crate
  - [ ] Define `OrbitalAssembly`, `SpaceStructure`, `AssemblyStep`, `AssemblyAction`, `AssemblyConstraint`
  - [ ] Implement `plan_sequence`, `simulate_step`, `verify_structural_integrity`
  - [ ] Verification test: `test_orbital_assembly_collision`
  - [ ] Rustdoc + `docs/book` chapter "Orbital Assembly"
- [ ] `tpt-yard-space-structural` — structures without launch constraints
  - [ ] Scaffold crate
  - [ ] Define `SpaceStructuralDesigner`, `SpaceEnvironment`, `DesignFreedom`, `ShapeConstraint`
  - [ ] Implement `no_launch_constraint`, `rotating_habitat_stress`, `thermal_cycling_fatigue`, `micrometeoroid_shielding`
  - [ ] Verification test: `test_rotating_habitat_stress` (σ = ρ·ω²·r²)
  - [ ] Rustdoc + `docs/book` chapter "Space Structural Design"
- [ ] `tpt-yard-robotic-assembly` — robotic arm path planning
  - [ ] Scaffold crate
  - [ ] Define `RoboticArm`, `Joint`, `EndEffector`, `MotionPlan`
  - [ ] Implement `inverse_kinematics`, `plan_path` (RRT*/PRM), `grasp_planning`
  - [ ] Unit tests for IK solutions and collision-free path planning
  - [ ] Rustdoc + `docs/book` chapter "Robotic Assembly"
- [ ] `tpt-yard-habitat` — rotating habitat design
  - [ ] Scaffold crate
  - [ ] Define `HabitatDesigner`, `HabitatType`
  - [ ] Implement `required_rotation`, `coriolis_effects`, `structural_design`
  - [ ] Unit tests validating ω = √(g/r) and the <2 rpm Coriolis comfort limit
  - [ ] Rustdoc + `docs/book` chapter "Habitat Design"
- [ ] `tpt-yard-propellant` — propellant loading and boil-off management
  - [ ] Scaffold crate
  - [ ] Define `PropellantSpec`, `PropellantLoadingPlan`, `ThermalControlPlan`
  - [ ] Implement propellant loading sequence and boil-off management logic
  - [ ] Unit tests for cryogenic loading thermal stability
  - [ ] Rustdoc
- [ ] RFC `0003-orbital-assembly-planning.md` written and merged
- [ ] RFC `0005-rotating-habitat-structural.md` written and merged
- [ ] Populate `test-data/orbital-structures/` with sample truss/module definitions
- [ ] Example: `examples/orbital-station-truss/`
- [ ] Example: `examples/rotating-habitat-construction/`
- [ ] Golden data: `test-data/golden/space/iss-truss-assembly.json`
- [ ] Golden data: `test-data/golden/space/rotating-habitat-stress.json`
- [ ] Golden data: `test-data/golden/space/orbital-assembly-sequence.json`
- [ ] Golden data: `test-data/golden/space/robotic-arm-path.json`
- [ ] Bench: `benches/orbital-assembly-sequence.rs`
- [ ] **Milestone:** simulate orbital truss assembly with robotic arm end-to-end

---

## Phase 5: Manufacturing & Planning (Months 19-24)

- [ ] `tpt-yard-space-manufacturing` — in-space manufacturing and additive construction
  - [ ] Scaffold crate
  - [ ] Define `InSpaceManufacturing`, `ManufacturingProcess`, `AdditiveTechnique`, `IsruSource`
  - [ ] Implement `print_time_estimate`, `thermal_control_during_print`, `quality_verification`
  - [ ] Unit tests for print-time estimation
  - [ ] Rustdoc + `docs/book` chapter "In-Space Manufacturing"
- [ ] `tpt-yard-scheduling` — construction scheduling and critical path
  - [ ] Scaffold crate
  - [ ] Define `ShipyardScheduler`, `ScheduleObjective`
  - [ ] Implement `critical_path`, `optimize_sequence`, `resource_leveling`
  - [ ] Unit tests vs `critical-path-schedule.json` and `resource-leveling.json` golden data
  - [ ] Rustdoc + `docs/book` chapter "Scheduling"
- [ ] `tpt-yard-quality` — quality control and inspection
  - [ ] Scaffold crate
  - [ ] Define `QualityManagement`, `NdtMethod`, `AcceptanceCriteria`
  - [ ] Implement `generate_inspection_plan`, `defect_tracking`
  - [ ] Unit tests vs `ndt-inspection-plan.json` and `weld-defect-tracking.json` golden data
  - [ ] Rustdoc
- [ ] `tpt-yard-logistics` — material/resource logistics
  - [ ] Scaffold crate
  - [ ] Define logistics/resource-flow types (delivery, staging, transport scheduling)
  - [ ] Implement resource-flow scheduling helpers
  - [ ] Unit tests for resource-flow scheduling
  - [ ] Rustdoc
- [ ] `tpt-yard-facility` — shipyard/orbital-facility layout planning
  - [ ] Scaffold crate
  - [ ] Define facility-planning types (crane placement, workshop sizing, dock capacity; cross-ref `tpt-construction`)
  - [ ] Implement facility capacity-constraint checks
  - [ ] Unit tests for facility capacity constraints
  - [ ] Rustdoc
- [ ] Example: `examples/space-solar-array-assembly/`
- [ ] Golden data: `test-data/golden/planning/critical-path-schedule.json`
- [ ] Golden data: `test-data/golden/planning/resource-leveling.json`
- [ ] Golden data: `test-data/golden/quality/ndt-inspection-plan.json`
- [ ] Golden data: `test-data/golden/quality/weld-defect-tracking.json`
- [ ] **Milestone:** plan in-space additive manufacturing of a solar array end-to-end

---

## Phase 6: Integration & WASM (Months 25-30)

- [ ] `tpt-yard-transport-link` — vehicle design ⇄ shipyard construction bridge
  - [ ] Scaffold crate (depends on `tpt-transport`)
  - [ ] Implement `plan_construction` (`Vehicle` → `VesselProject`)
  - [ ] Implement `handover_to_operations` (`DigitalTwin` → `AsBuiltProperties`)
  - [ ] Unit tests for round-trip design → build → as-built handover
  - [ ] Rustdoc + `docs/book` chapter "Transport Integration"
- [ ] `tpt-yard-process-link` — process engineering bridge
  - [ ] Scaffold crate (depends on `tpt-process`)
  - [ ] Implement `plan_propellant_loading` using `PengRobinson` EOS from `tpt-process`
  - [ ] Unit tests for propellant loading plan correctness
  - [ ] Rustdoc
- [ ] `tpt-yard-earth-link` — weather/sea-state bridge
  - [ ] Scaffold crate (depends on `tpt-earth`)
  - [ ] Implement `plan_launch_window` (`SeaState` → `LaunchWindow`)
  - [ ] Unit tests for launch-window sea-state gating
  - [ ] Rustdoc + `docs/book` chapter "Earth/Weather Integration"
- [ ] `tpt-yard-wasm` — WebAssembly bindings for interactive dashboards
  - [ ] Scaffold crate with `wasm-bindgen`
  - [ ] Implement `WasmDigitalTwin` (`new`, `advance_phase`, `get_current_geometry`, `get_weight_report`, `structural_check`)
  - [ ] Implement `WasmOrbitalAssembly` (`simulate_next_step`, `get_robot_pose`)
  - [ ] Browser smoke test: load WASM module, advance a phase, render geometry
  - [ ] Rustdoc + usage guide for WASM dashboard integration
- [ ] **Milestone:** interactive 3D shipyard dashboard running in browser (WASM demo)

---

## Cross-Cutting (ongoing, all phases)

- [ ] Keep `deny.toml` MIT-chain check green on every PR
- [ ] Keep crate `Status` table in `README.md` up to date (Planned → In Progress → Done)
- [ ] Each new crate registered in workspace `Cargo.toml` members
- [ ] Each new public API documented in `docs/book/`
