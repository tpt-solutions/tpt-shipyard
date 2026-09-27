# tpt-shipyard — Project TODO

**Org:** TPT Solutions (`tpt-solutions`) · **License:** MIT OR Apache-2.0 · **Repo:** `tpt-shipyard`

A fully open-source computational engine for vehicle construction: sea shipyards, orbital assembly, in-space manufacturing, and construction digital twins. This checklist tracks every deliverable from `spec.txt`, phase by phase.

---

## Phase 0: Repository & Governance Setup

- [x] Workspace `Cargo.toml` (virtual workspace, `crates/*` members, shared profile/lints)
- [x] `LICENSE-MIT`
- [x] `LICENSE-APACHE`
- [x] `README.md` (per spec Section 12 template)
- [x] `CONTRIBUTING.md` (DCO sign-off process, no CLA)
- [x] `SECURITY.md` (private vulnerability disclosure policy)
- [x] `deny.toml` (allow MIT/Apache-2.0/BSD-2-Clause/BSD-3-Clause/ISC/Zlib/Unicode-3.0; `copyleft = "deny"`; `unlicensed = "deny"`)
- [x] `rustfmt.toml`
- [x] `clippy.toml`
- [x] `.github/workflows/ci.yml`
- [x] `.github/workflows/license.yml` (cargo-deny gate)
- [x] `.github/workflows/benchmark.yml`
- [x] `.github/workflows/release.yml`
- [x] `.github/ISSUE_TEMPLATE/`
- [x] `.github/PULL_REQUEST_TEMPLATE.md`
- [x] `docs/book/` scaffold (mdBook init)
- [x] `docs/rfc/` index + `rfcs/` directory scaffold
- [x] Governance doc: Benevolent Dictator + RFC process, "TPT Shipyard" trademark notice
- [x] Public GitHub Projects roadmap board *(board content mirrored in `docs/roadmap.md`; actual board creation deferred with publishing)*
- [x] SemVer + 6-week release cadence policy doc
- [x] Empty seed dirs: `test-data/hull-blocks/`, `test-data/orbital-structures/`, `test-data/welding-procedures/`, `test-data/golden/`
- [x] **Milestone:** empty workspace builds, CI green, license check passes

---

## Phase 1: Core & Digital Twin (Months 1-3)

- [x] `tpt-yard-core` — fundamental shipyard domain types
  - [x] Scaffold crate (Cargo.toml, lib.rs, workspace member)
  - [x] Define `VesselProject`, `VesselType`, `SeaVesselType`, `SpaceVesselType`, `ConstructionMethod`
  - [x] Define `BuildPhase`, `AssemblyActivity`, `ActivityType`, `Resource`
  - [x] Unit tests for type construction/(de)serialization
  - [x] Rustdoc + `docs/book` chapter "Core Types"
- [x] `tpt-yard-digital-twin` — construction state tracking and simulation
  - [x] Scaffold crate
  - [x] Define `DigitalTwin`, `AssemblyState`
  - [x] Implement `advance_phase`, `structural_check_at_phase`, `weight_deviation`, `centre_of_gravity_tracking`
  - [x] Verification test: `test_cog_tracking` (CoG shifts predictably as blocks are added)
  - [x] Rustdoc + `docs/book` chapter "Digital Twin"
- [x] `tpt-yard-weight` — weight and centre of gravity management
  - [x] Scaffold crate
  - [x] Define `WeightModel`, `WeightItem`, `ItemStatus`
  - [x] Implement `total_weight`, `centre_of_gravity`, `weight_report`
  - [x] Verification test: `test_block_weight_sum`
  - [x] Rustdoc + `docs/book` chapter "Weight & CoG"
- [x] `tpt-yard-assembly` — shared assembly-activity primitives (used by core & digital-twin)
  - [x] Scaffold crate
  - [x] Define activity dependency-graph types and activity status tracking
  - [x] Implement activity dependency resolution helpers
  - [x] Unit tests for dependency-graph correctness (cycles, ordering)
  - [x] Rustdoc
- [x] RFC `0001-digital-twin-state.md` written and merged
- [x] **Milestone:** track weight and CoG through 10 build phases (demo/test script)

---

## Phase 2: Structural During Construction (Months 4-6)

- [x] `tpt-yard-structural` — structural analysis at each build phase
  - [x] Scaffold crate (depends on `tpt-fem`)
  - [x] Define `ConstructionStructuralSolver`, `ConstructionLoad`
  - [x] Implement `analyze_at_phase`, `lifting_analysis`, `launch_analysis`
  - [x] Tests: partial-structure FEM analysis vs known cases
  - [x] Rustdoc + `docs/book` chapter "Structural Analysis During Construction"
- [x] `tpt-yard-welding` — welding simulation and distortion control
  - [x] Scaffold crate
  - [x] Define `WeldingSimulation`, `WeldProcedure`, `WeldProcess`, `WeldPass`
  - [x] Implement `thermal_cycle` (Rosenthal equation), `residual_stress`, `distortion`, `welding_sequence_optimization`
  - [x] Verification test: `test_welding_distortion` vs experimental data
  - [x] Rustdoc + `docs/book` chapter "Welding Simulation"
- [x] `tpt-yard-distortion` — block distortion management
  - [x] Scaffold crate
  - [x] Define `DistortionControl`, `CorrectionAction`
  - [x] Implement `deviation_map`, `correction_plan`
  - [x] Unit tests for correction-plan tolerance thresholds
  - [x] Rustdoc
- [x] `tpt-yard-joints` — shared joint-geometry primitives (used by structural & welding)
  - [x] Scaffold crate
  - [x] Define joint geometry types (butt, fillet, lap, T-joint, etc.) backing `JointGeometry`
  - [x] Implement joint geometry/area calculations
  - [x] Unit tests for joint geometry calculations
  - [x] Rustdoc
- [x] RFC `0004-welding-distortion-model.md` written and merged
- [x] Populate `test-data/welding-procedures/` with sample WPS records
- [x] Bench: `benches/welding-distortion.rs`
- [x] Golden data: `test-data/golden/sea/welding-distortion-panel.json`
- [x] **Milestone:** simulate welding distortion in a hull panel end-to-end

---

## Phase 3: Sea Shipyard (Months 7-12)

- [x] `tpt-yard-hull` — hull construction and block management
  - [x] Scaffold crate
  - [x] Define `HullConstruction`, `HullBlock`, `BlockStatus`, `BlockJoin`
  - [x] Implement `block_division`, `erection_sequence`
  - [x] Unit tests for block division against crane/workshop constraints
  - [x] Rustdoc + `docs/book` chapter "Hull Blocks"
- [x] `tpt-yard-blocks` — shared block-lifting/handling primitives
  - [x] Scaffold crate
  - [x] Define lift-point and sling-load types
  - [x] Implement sling-load distribution calculations
  - [x] Unit tests for sling-load distribution
  - [x] Rustdoc
- [x] `tpt-yard-drydock` — drydock flooding/ballast sequencing
  - [x] Scaffold crate
  - [x] Define drydock flooding/ballast state types
  - [x] Implement stability-at-each-water-level flooding sequence logic
  - [x] Unit tests vs `drydock-flooding-sequence.json` golden data
  - [x] Rustdoc
- [x] `tpt-yard-launch` — launch calculations
  - [x] Scaffold crate
  - [x] Define `LaunchAnalysis`, `LaunchMethod`, `SlipwayLaunchResult`
  - [x] Implement `slipway_launch`, `drydock_flooding`, `launch_stability`
  - [x] Verification test: `test_slipway_tip_up`
  - [x] Rustdoc + `docs/book` chapter "Launch Analysis"
- [x] `tpt-yard-outfitting` — systems installation and routing
  - [x] Scaffold crate
  - [x] Define `OutfittingPlan`, `OutfitSystem`, `Route`
  - [x] Implement `collision_detection`, `installation_sequence`
  - [x] Unit tests for collision detection between systems
  - [x] Rustdoc
- [x] `tpt-yard-sea-trials` — post-launch sea-trial test planning
  - [x] Scaffold crate
  - [x] Define sea-trial test-plan types (speed trials, sea-keeping, class-society acceptance)
  - [x] Implement trial acceptance-criteria evaluation
  - [x] Unit tests for trial acceptance criteria
  - [x] Rustdoc
- [x] RFC `0002-block-assembly-sequencing.md` written and merged
- [x] Populate `test-data/hull-blocks/` with sample block definitions
- [x] Example: `examples/container-ship-block-assembly/`
- [x] Example: `examples/submarine-pressure-hull/`
- [x] Example: `examples/drydock-flooding-sequence/`
- [x] Golden data: `test-data/golden/sea/container-ship-block-division.json`
- [x] Golden data: `test-data/golden/sea/slipway-launch-stability.json`
- [x] Golden data: `test-data/golden/sea/drydock-flooding-sequence.json`
- [x] Bench: `benches/block-lifting.rs`
- [x] Bench: `benches/launch-stability.rs`
- [x] **Milestone:** plan block erection sequence for a container ship end-to-end

---

## Phase 4: Space Shipyard (Months 13-18)

- [x] `tpt-yard-orbital-assembly` — orbital assembly planning and simulation
  - [x] Scaffold crate
  - [x] Define `OrbitalAssembly`, `SpaceStructure`, `AssemblyStep`, `AssemblyAction`, `AssemblyConstraint`
  - [x] Implement `plan_sequence`, `simulate_step`, `verify_structural_integrity`
  - [x] Verification test: `test_orbital_assembly_collision`
  - [x] Rustdoc + `docs/book` chapter "Orbital Assembly"
- [x] `tpt-yard-space-structural` — structures without launch constraints
  - [x] Scaffold crate
  - [x] Define `SpaceStructuralDesigner`, `SpaceEnvironment`, `DesignFreedom`, `ShapeConstraint`
  - [x] Implement `no_launch_constraint`, `rotating_habitat_stress`, `thermal_cycling_fatigue`, `micrometeoroid_shielding`
  - [x] Verification test: `test_rotating_habitat_stress` (σ = ρ·ω²·r²)
  - [x] Rustdoc + `docs/book` chapter "Space Structural Design"
- [x] `tpt-yard-robotic-assembly` — robotic arm path planning
  - [x] Scaffold crate
  - [x] Define `RoboticArm`, `Joint`, `EndEffector`, `MotionPlan`
  - [x] Implement `inverse_kinematics`, `plan_path` (RRT*/PRM), `grasp_planning`
  - [x] Unit tests for IK solutions and collision-free path planning
  - [x] Rustdoc + `docs/book` chapter "Robotic Assembly"
- [x] `tpt-yard-habitat` — rotating habitat design
  - [x] Scaffold crate
  - [x] Define `HabitatDesigner`, `HabitatType`
  - [x] Implement `required_rotation`, `coriolis_effects`, `structural_design`
  - [x] Unit tests validating ω = √(g/r) and the <2 rpm Coriolis comfort limit
  - [x] Rustdoc + `docs/book` chapter "Habitat Design"
- [x] `tpt-yard-propellant` — propellant loading and boil-off management
  - [x] Scaffold crate
  - [x] Define `PropellantSpec`, `PropellantLoadingPlan`, `ThermalControlPlan`
  - [x] Implement propellant loading sequence and boil-off management logic
  - [x] Unit tests for cryogenic loading thermal stability
  - [x] Rustdoc
- [x] RFC `0003-orbital-assembly-planning.md` written and merged
- [x] RFC `0005-rotating-habitat-structural.md` written and merged
- [x] Populate `test-data/orbital-structures/` with sample truss/module definitions
- [x] Example: `examples/orbital-station-truss/`
- [x] Example: `examples/rotating-habitat-construction/`
- [x] Golden data: `test-data/golden/space/iss-truss-assembly.json`
- [x] Golden data: `test-data/golden/space/rotating-habitat-stress.json`
- [x] Golden data: `test-data/golden/space/orbital-assembly-sequence.json`
- [x] Golden data: `test-data/golden/space/robotic-arm-path.json`
- [x] Bench: `benches/orbital-assembly-sequence.rs`
- [x] **Milestone:** simulate orbital truss assembly with robotic arm end-to-end

---

## Phase 5: Manufacturing & Planning (Months 19-24)

- [x] `tpt-yard-space-manufacturing` — in-space manufacturing and additive construction
  - [x] Scaffold crate
  - [x] Define `InSpaceManufacturing`, `ManufacturingProcess`, `AdditiveTechnique`, `IsruSource`
  - [x] Implement `print_time_estimate`, `thermal_control_during_print`, `quality_verification`
  - [x] Unit tests for print-time estimation
  - [x] Rustdoc + `docs/book` chapter "In-Space Manufacturing"
- [x] `tpt-yard-scheduling` — construction scheduling and critical path
  - [x] Scaffold crate
  - [x] Define `ShipyardScheduler`, `ScheduleObjective`
  - [x] Implement `critical_path`, `optimize_sequence`, `resource_leveling`
  - [x] Unit tests vs `critical-path-schedule.json` and `resource-leveling.json` golden data
  - [x] Rustdoc + `docs/book` chapter "Scheduling"
- [x] `tpt-yard-quality` — quality control and inspection
  - [x] Scaffold crate
  - [x] Define `QualityManagement`, `NdtMethod`, `AcceptanceCriteria`
  - [x] Implement `generate_inspection_plan`, `defect_tracking`
  - [x] Unit tests vs `ndt-inspection-plan.json` and `weld-defect-tracking.json` golden data
  - [x] Rustdoc
- [x] `tpt-yard-logistics` — material/resource logistics
  - [x] Scaffold crate
  - [x] Define logistics/resource-flow types (delivery, staging, transport scheduling)
  - [x] Implement resource-flow scheduling helpers
  - [x] Unit tests for resource-flow scheduling
  - [x] Rustdoc
- [x] `tpt-yard-facility` — shipyard/orbital-facility layout planning
  - [x] Scaffold crate
  - [x] Define facility-planning types (crane placement, workshop sizing, dock capacity; cross-ref `tpt-construction`)
  - [x] Implement facility capacity-constraint checks
  - [x] Unit tests for facility capacity constraints
  - [x] Rustdoc
- [x] Example: `examples/space-solar-array-assembly/`
- [x] Golden data: `test-data/golden/planning/critical-path-schedule.json`
- [x] Golden data: `test-data/golden/planning/resource-leveling.json`
- [x] Golden data: `test-data/golden/quality/ndt-inspection-plan.json`
- [x] Golden data: `test-data/golden/quality/weld-defect-tracking.json`
- [x] **Milestone:** plan in-space additive manufacturing of a solar array end-to-end

---

## Phase 6: Integration & WASM (Months 25-30)

- [x] `tpt-yard-transport-link` — vehicle design ⇄ shipyard construction bridge
  - [x] Scaffold crate (depends on `tpt-transport`)
  - [x] Implement `plan_construction` (`Vehicle` → `VesselProject`)
  - [x] Implement `handover_to_operations` (`DigitalTwin` → `AsBuiltProperties`)
  - [x] Unit tests for round-trip design → build → as-built handover
  - [x] Rustdoc + `docs/book` chapter "Transport Integration"
- [x] `tpt-yard-process-link` — process engineering bridge
  - [x] Scaffold crate (depends on `tpt-process`)
  - [x] Implement `plan_propellant_loading` using `PengRobinson` EOS from `tpt-process`
  - [x] Unit tests for propellant loading plan correctness
  - [x] Rustdoc
- [x] `tpt-yard-earth-link` — weather/sea-state bridge
  - [x] Scaffold crate (depends on `tpt-earth`)
  - [x] Implement `plan_launch_window` (`SeaState` → `LaunchWindow`)
  - [x] Unit tests for launch-window sea-state gating
  - [x] Rustdoc + `docs/book` chapter "Earth/Weather Integration"
- [x] `tpt-yard-wasm` — WebAssembly bindings for interactive dashboards
  - [x] Scaffold crate with `wasm-bindgen`
  - [x] Implement `WasmDigitalTwin` (`new`, `advance_phase`, `get_current_geometry`, `get_weight_report`, `structural_check`)
  - [x] Implement `WasmOrbitalAssembly` (`simulate_next_step`, `get_robot_pose`)
  - [x] Browser smoke test: load WASM module, advance a phase, render geometry
  - [x] Rustdoc + usage guide for WASM dashboard integration
- [x] **Milestone:** interactive 3D shipyard dashboard running in browser (WASM demo)

---

## Cross-Cutting (ongoing, all phases)

- [x] Keep `deny.toml` MIT-chain check green on every PR
- [x] Keep crate `Status` table in `README.md` up to date (Planned → In Progress → Done)
- [x] Each new crate registered in workspace `Cargo.toml` members
- [x] Each new public API documented in `docs/book/`
