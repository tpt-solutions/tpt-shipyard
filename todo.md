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
  - [x] Implement `WasmDigitalTwin` (`new`, `advance_phase`, `get_geometry`, `get_weight_report`, `structural_check`) *(claim corrected 2026-09-30: the method is `get_geometry`)*
  - [x] Implement `WasmOrbitalAssembly` (`simulate_next_step`, `get_robot_pose`)
  - [x] Browser smoke tests delivered as `#[wasm_bindgen_test]` suites (`crates/web/tpt-yard-wasm/tests/browser.rs`): load a project, dependency-gated advance, weight report JSON, geometry index bounds, structural check, and a full orbital step loop with a finite robot pose — run under headless Chrome by the CI `wasm` job (and verified locally against Chrome 153 with the pinned `wasm-bindgen-cli@0.2.128`).
  - [x] Rustdoc + usage guide for WASM dashboard integration
- [x] **Milestone:** interactive 3D shipyard dashboard running in browser (WASM demo) — `www/index.html` uploads the `WasmDigitalTwin` geometry buffers into a three.js `BufferGeometry` (orbit/zoom/pan, block-gradient shading, as-built CoG marker, camera follows the erection until the user takes over), with the weight/structural panels and all the 7E hygiene retained. Verified end-to-end in headless Chrome (wasm + CDN three.js + step/auto/reset flow, screenshots).

---

## Cross-Cutting (ongoing, all phases)

- [x] Keep `deny.toml` MIT-chain check green on every PR
- [x] Keep crate `Status` table in `README.md` up to date (Planned → In Progress → Done)
- [x] Each new crate registered in workspace `Cargo.toml` members
- [x] Each new public API documented in `docs/book/`

---

## Phase 7: Review Follow-ups (2026-09-29 platform review)

Source: code review of the whole workspace. Line numbers are approximate. Fix each item with a regression test that fails before the fix. Only regenerate golden values after an independent hand or reference calculation.

### 7A. Correctness bugs (highest priority)

- [x] A1 `tpt-yard-welding`: Rosenthal temperatures subtract 273.15 from a °C value (`peak_temperature_at`, `thermal_cycle`). Fixed; arc efficiency η now applied to net plate power; sampling window widens until the 800→500 °C leg fits; `thermal_cycle(0.0)` clamps off the singularity. Golden regenerated and independently verified (closed-form 3D Rykalin t8/5 = 29.08 s; analytic Rosenthal peak scan 9605 °C at ξ = −10.9 mm).
- [x] A2 `tpt-yard-launch/tests/golden_sea.rs:48,115`: two-sided relative tolerance now enforced (`.abs()`)
- [x] A3 `tpt-yard-digital-twin` `advance_phase`: every fallible step (support check, mass-properties computation) now runs before any mutation; fresh `DigitalTwin::new` twins (no wired items) advance with zero mass properties. Regression tests: fresh twin advances; failed advances leave the twin untouched.
- [x] A4 `tpt-yard-wasm` `get_geometry_indices`: returns the real `Geometry3D` face table via a shared `build_geometry` helper; test asserts every index is inside the vertex buffer.
- [x] A5 `tpt-yard-robotic-assembly`: two-link IK wraps angles total-order (no hang on narrow ranges); DLS IK now solves the proper n×n `(JᵀJ + λI)⁻¹Jᵀe` in joint space with a pivoting Gaussian solver (4+ joints converge); FK/Jacobian handle prismatic joints (extension along heading). Repro tests: narrow ranges, 4-link, behind-the-arm, prismatic FK.
- [x] A6 `tpt-yard-scheduling`: levelling uses a precedence-feasible serial schedule generation scheme (priority among *eligible* activities only); clash scan uses exact breakpoint times (no 10 000 h cap / silent clash); real mutual-dependency cycle test across all objectives; order-feasibility asserted for every objective.
- [x] A7 `tpt-yard-hull` `erection_sequence`: `midship_distance` measures from LOA/2 (midship-outward order, was origin-ordered); first block of an upper tier lands on the block below (`onto` set); test asserts every upper-tier block has support.
- [x] A8 `tpt-yard-structural` FEM: orphan nodes condensed before assembly (displacements mapped back to model indexing; a *load* on an unconnected node is rejected); pivot threshold relative to matrix scale; penalty scaled to model stiffness. (Dense O(n³) solver remains a documented limitation — sparse solver stays on the 7H roadmap.)
- [x] A9 `tpt-yard-structural`/`tpt-yard-blocks` lifting: hook rides over the CoG projection (statically correct pick); 2-point lever rule via scalar projection (Euclidean, was Manhattan); tip check uses a real convex hull (was AABB — would pass notch CoGs); crane overload, sling-angle guideline and leg utilization all fail the pick; `lifting_analysis` takes `crane_capacity_kn`; `allowable_load_kn` checkable via `legs_within_allowable`; `launch_analysis` applies cos(slope) to slipway way pressure and returns `NotScreenable` (not a fake pass) for drydock/side-launch/shiplift.
- [x] A10 Panics on user input: `Geometry3D::bounding_box` empty mesh no longer panics; JSON parser has a 128-level nesting limit; `WasmDigitalTwin::new` and `simulate_next_step` return `Result<_, JsError>` (throw, not abort); `thermal_cycle(0.0)` finite; NaN fails the welding and sea-trial guards.
- [x] A11 Graph construction: `add_activity` accepts out-of-order networks with an explicit `validate()`; duplicate ids rejected (`DuplicateActivity`); duplicate dependencies collapsed (no false cycles); `topological_order` validates first (a dangling dependency from `activity_mut` is an error, not a phantom-id panic); scheduling/logistics builders propagate errors instead of silently dropping duplicates.
- [x] A12 `tpt-yard-sea-trials`: empty program reports `all_passed = false`; NaN never accepted by any acceptance window.

### 7B. Physics and model fixes (medium)

*(2026-09-30: first pass applied. Golden values were regenerated only after
independent hand/reference calculations; each fix carries a regression test.
Remaining sub-items are marked inline.)*

- [x] Welding: arc efficiency now applied to the net plate power (was dead code); thermal sampling window widens until the 800-500 C leg fits; `thermal_cycle(0.0)` clamped off the point-source singularity. Golden regenerated and independently verified. Panel width, bow arm, mechanical-response fraction and panel section are builder parameters (were hard-coded); `distortion_of_sequence` scales transverse with heat but shrinkage/rotation with deposited weld area (pass count); `thin_plate_warning` flags thermally thin plates via the melting isotherm. Goldens hand-verified.
- [x] GM formula (drydock, launch) uses the fineness-corrected `Cwp*B^2/(12*Cb*T)` with `Cwp = (1+2Cb)/3`; drydock goldens hand-verified. The exact float-off instant is always sampled; aground steps state that stability is judged at float-off instead of claiming it vacuously; `virtual_gm_touchdown_m` implements the classical docking virtual GM (`dGM = P*KM/W` with the keel-block reaction share), verified to stiffen the vessel on the blocks, scale with the reaction share, and collapse to the afloat GM exactly at float-off.
- [x] Slipway launch: cos(theta) on way loads; buoyancy relief at the pivot (parabolic immersion growth); slamming uses the vertical entry-velocity component; pivot pressure divides by total way width only (the times-ways double-count is gone); SideLaunch/Shiplift no longer fake slipway statics (explicit non-result); `drydock_flooding` validates the computed draft against the dock depth. Golden hand-verified.
- [x] Hull block division: internal-structure factor (x4, documented pre-design estimate) so a 140 m hull no longer estimates an implausible 1714 t of steel; `depth_bands == 0` returns no blocks; the upward length clamp that could hide crane overload is gone; crane-binding exercised in tests. Workshop cross-section check added: `HullConstruction::check_workshop` rejects a workshop narrower than the beam or shallower than the band height (blocks are full-beam wide), and `tpt-yard plan` fails with that message instead of dividing silently.
- [x] Joints: J/U use rounded-root geometry instead of the sharp V wedge; DoubleV/K weld half the depth per side (~1/4 the wedge area); fillet throat + penetration retained (correct).
- [x] Distortion: `deviation_map_form` removes the best-fit rigid-body motion (translation + small rotation, reported in the field) before differencing; `correction_plan` works on the form deviation, thresholds are explicit configured parameters (`reject_factor`, `rework_fraction`), and an optional `StraighteningModel` sizes straightening heat from the plate's contraction physics instead of a flat placeholder
- [x] Outfitting: collision checks run segment-to-segment (closest-points distance with a `clearance_m` parameter) after a clearance-inflated bbox screen — L-shaped routes whose boxes overlap but whose runs never approach no longer falsely clash. *(2026-10-01: install ordering now dependency-aware — `OutfittingPlan.precedes` carries hard (before, after) edges and `installation_sequence` runs a precedence-feasible serial scheme: among *eligible* systems the largest goes first, plan order breaks ties; dangling edges and cycles are typed errors. Without edges it is exactly the old large-first order.)*
- [x] Propellant: cooler input power is Carnot-scaled and temperature-dependent (was an inverted flat 100:1); NaN fill rates rejected; `Recooled` produced for moderate leaks (<=20x ZBO, vent beyond); heat leak validated non-negative/finite.
- [x] Habitat: the shell rounds up to a configurable fabrication minimum gauge so `utilization` is a real margin (was tautologically 1); the torus hoop load is carried on the *tube* circumference, not the major ring; `HabitatType::Custom` without an explicit radius (`with_custom_radius_m`) reports an empty design instead of silently assuming 100 m
- [x] Orbital assembly: `deployed_length` is the tip extent (centre + projected half-bay; goldens hand-verified at 30 m / 75 m); `chord_area_m2`/`bay_height_m` are plan fields driven from the golden JSON (were hard-coded); `simulate_step` returns `NoRobotAssigned` for robot-less steps. *(2026-10-01: component dependencies shipped — `OrbitalAssembly.component_dependencies` carries hard (before, after) id edges and `plan_sequence` runs a precedence-feasible nearest-first scheme with `UnknownDependency`/`DependencyCycle` errors; without edges it is exactly the old distance order. `ComponentSpec` itself stays dependency-free; the manifest schema can grow an edge list when a case needs it.)*
- [x] Space-structural: shield proportions calibrated to published ISS dual-wall sets (bumper d/8, standoff 10d, rear wall 0.4d — replaces the invented /6-/10-/12 split; full Christiansen ballistic-limit equations remain roadmap); `cycles = 0` and zero-swing cases verified finite and safe with regression tests
- [x] Space-manufacturing: energy constants documented as literature screening values with an overridable public field; `eclipse_pauses` follows the power source (solar pauses, nuclear prints through); radiator area uses the net flux `eps*sigma*(T_rad^4 - T_sink^4)` with configurable sink temperature and an environment-heat duty term
- [x] Scheduling: `critical_path` walks an actual zero-float chain (start, then earliest-finishing critical dependents) instead of an id-sorted set; levelling uses exact clash breakpoints (the 10 000 h cap and silent clash are gone). *(2026-10-02: **capacity-aware levelling shipped** — `resource_leveling_with_limits`/`optimize_sequence_with_limits` take yard-wide capacities per resource kind (same units as the activity demands) and let activities share a kind while the summed concurrent demand stays at or under the limit; the feasibility test covers the whole placement window (a demand rise inside the activity's own duration defers the placement), kinds without a limit are unconstrained, non-positive/non-finite limits and over-capacity demands are typed errors, and `ScheduleResult` now carries per-activity `start_hours`. Verified hand-checked: parallel sharing at 4 welders vs deferral at 3, the whole-window regression (a start-only check produces a 5-welder overrun), exact-limit validation, and the legacy path unchanged. CLI: `schedule project.json --limit crane=200 --limit crew=40`. **Dock-occupancy behavior shipped** (2026-10-02, closing the objectives note): `ScheduleResult.dock_occupancy_h` carries the first-start-to-last-finish span over the Drydock-drawing activities, `MinimizeDrydockTime` reports it plus the outside-dock tail in its notes (the other objectives carry the observable without the note), and the CLI `schedule` command emits it in text and JSON; pairing with a Drydock limit of 1.0 serialises dock work (occupancy = sum, hand-checked). The objectives remain two placement *strategies* — earliest-start and float-priority levelling, plus the capacity-aware variant — now differentiated by what they report.)*
- [x] Robotic assembly: docs say RRT (correct — no RRT* claims remain); `collision_free` is verified over the final chain rather than hard-coded; the IK goal wraps to the start's 2-pi branch so paths stop spinning joints the long way; grasp planning picks the long-axis faces, sizes force through a friction coefficient (`m*a*SF/(2*mu)`), and filters grasp points by reachability *(the 3x3 DLS and prismatic-joint bugs were fixed under 7A/A5)*
- [x] Facility: crane capacities no longer aggregate (`capacity_of` returns the largest single crane; workshops/slots pool); `overlaps` respects z with vertical extents.
- [x] Logistics: the CPM schedule is expanded from working days to calendar days (5-day week) before lead-time arithmetic; `schedule_deliveries` takes `max_dwell_days` and rejects over-dwell deliveries (`DwellExceeded`); deliveries carry their tonnage (`quantity_t`)
- [x] Quality: the inspection plan honours its documented coverage policy (structural butts sampled at 30%, pressure boundaries at 100%), timings reflect the activity kind (post-weld / post-erection / post-install / on completion), and methods degrade to the yard's available-method list; golden regenerated
- [x] Integration crates: process-link dead `* 0.0` arithmetic removed (the 1-atm saturation value is documented as the normal-boiling-point definition); transport-link projects derive their id from the design (was hard-coded 1) and the module count/TEU proxies are documented and derived from the mass; earth-link launch windows use `min(method limit, site.max_sea_state)` — the site limit is enforced with a regression test. All three remain stand-ins for the `tpt-*` substrates (by design)
- [x] Digital twin: `advance_phase` rejects activities from phases ahead of the current one (`ActivityNotInCurrentPhase`); the phase pointer only moves forward; the transverse support check uses true containment within the (possibly asymmetric) support track.
- [x] Weight model: NaN/negative weights rejected (`InvalidWeight`), duplicate `ItemId` rejected (`DuplicateItem`), margins reachable in totals (`total_with_margin_kg`, reported), `design_cog` surfaced in the weight report.
- [x] `Geometry3D`: `from_cylinder` clamps segments to >=3 (no assert on input); `merge` guards u32 index saturation; zero density/heat materials give zero diffusivity instead of a division by zero.

### 7C. Validation, schema and data integrity

- [x] `VesselProject::validate()`: duplicate phase/activity ids, dependency existence, `current_phase` membership, non-empty phases, finite non-negative numbers/durations, and installed-vs-design weight sanity; JSON loading (`from_json_value`) validates before returning, so inconsistent files cannot enter the engine silently. *(remaining: any deeper silent-defaulting that a future review turns up — the schema/publication/persistence/`Result` legs are the bullets below)*
- [x] Stop silent defaulting in JSON loading: missing vessel/weight-state/item fields are `MissingField`, wrong types are `TypeError` (never a zero or default variant), unknown enums (`hull_type`, `propulsion`, resource kinds, item statuses) are errors, malformed dependency/resource entries and twin progress-set entries fail the load instead of being dropped, NaN CoG components are rejected, and `schema_version` != 1 is refused (schemas/README.md documents the contract). Regression tests in `tpt-yard-core` (strict_json_*), `tpt-yard-weight` and `tpt-yard-digital-twin`.
- [x] `schema_version: 1` serialized in every project (`VesselProject::SCHEMA_VERSION`); draft-2020-12 schemas published in `schemas/` for the VesselProject and WPS wire formats with a README explaining validation plus the loader's stricter consistency checks. Hull-block and orbital-structure manifest schemas are published and cover the engine-consumed manifests (`container-ship-140m.json`, `iss-truss-manifest.json`); `submarine-pressure-hull.json` and `rotating-habitat-torus.json` are example reference parameter sets documented by their own contents (README lists them) — and the torus file's invalid `10_000_000.0` numeric literal (not JSON) was fixed.
- [x] JSON parser: strict RFC 8259 number grammar (`+1`, `.5`, `1.`, `01` rejected); `1e999` rejected as out of range; 128-level nesting limit; NaN/inf serialization already maps to `null` (`as_u64` range check pre-existing)
- [x] Persistence (JSON): `WeightModel::to_json`/`from_json_value` (exact round-trip, duplicate/invalid rejections), `DigitalTwin::to_json`/`from_json_value` (vessel + weight model + progress sets + mass properties; derived render geometry documented as rebuilt; malformed state rejected with `TwinError::Malformed`) — both doctested and unit-tested
- [x] Constructors return errors: `VesselProject::new` returns `Result<_, CoreError>` (was `Option`); `WeightModel::add_item` already returned `Result` after the 7B pass.

### 7D. Tests and verification quality

- [x] Independent references encoded alongside all four golden harnesses: welding asserts the closed-form Rykalin t8/5 (29.08 s) and a dense analytic Rosenthal peak scan (9605 C) against both the library and the golden file; slipway asserts hand hydrostatic draft (2.710 m), the buoyancy-free sliding-velocity bound (10.9 m/s) and the cos-corrected way pressure (163.3 kPa); drydock asserts Archimedes displacement at float-off, hand draft/GM (KB+BM−KG = 8.681 m with Cwp = (1+2Cb)/3), float-off level (draft + 1 m blocks) and a physical fill-time bracket (which honestly shows the fill-time model sits at the empty-basin upper bound — displacement ignored); the ISS truss asserts the tip-cantilever couple σ = F·L/(A·h) = 50 MPa at the 75 m tip extent. (σ = ρω²r² was already the `test_rotating_habitat_stress` verification.)
- [x] Deleted `robotic-assembly/tests/dls_probe.rs` (superseded by the assertion-carrying IK tests added under 7A/A5)
- [x] Duplicate grasp tests merged into one comprehensive test; the launch sliding-velocity test now asserts an independent hand-computed constant; the hull erection test verifies the midship-outward order with inline arithmetic; habitat utilization assertions are meaningful after the tautology fix (7B)
- [x] `rrt_paths_around_obstacle` asserts the path is collision-free, has bounded Cartesian length (< 3x the straight-line span) and stays inside every joint range
- [x] All covered: IK narrow ranges/behind-the-arm/4+ joints and prismatic FK shipped under 7A/A5 with `proptest_ik`; levelling precedence and exact-breakpoint clashes under 7A/A6; true cycle detection via `proptest_graph`; FEM two-bar equilibrium and closed-form bar displacement via `proptest_fem`; exhaustive JSON round-trip for every `VesselType`/`ConstructionMethod`/`ActivityType` (incl. all `OutfitSystem` and `TestType` payloads)/`StructuralState`/`ResourceKind` variant in `tpt-yard-core/tests/json_roundtrip_all_variants.rs`.
- [x] Property-based tests shipped: `proptest_graph` (random out-of-order DAGs topologically sort with every dependency first; makespan bounded; cycle detection agrees with topo-sort), `proptest_fem` (two-bar equilibrium sum-Fz = load for any geometry; closed-form bar displacement for any area/load), `proptest_ik` (FK is the exact vector sum for any angles; IK solutions reproduce their targets inside the workspace annulus and stay in-range) - 256/128 cases each. *(2026-10-02: `proptest_scantlings` extends the culture to the new modules — p monotone in zone width and bounded, terminal >= mid probability, the slab round-trip, the exact Euler inversion, the EC3 curve monotone in (0,1], and the stiffener closed forms — 128 cases each)*
- [x] `wasm-bindgen-test` browser tests added (see the Phase 6 box); compiled out on native targets so the desktop test suite is unaffected.

### 7E. Honesty and hygiene

- [x] README status table: split "Done" into Done / Simplified model / Stand-in (refreshed: launch row mentions the shipped dynamic slipway simulation, wording unified to "Simplified model", the `tpt-yard-cli` row added)
- [x] Correct todo.md claims: Phase 6 `get_current_geometry` is actually `get_geometry`; browser smoke test and 3D dashboard are not delivered (`www/index.html` draws 2D rectangles and ignores the WASM mesh) — corrections applied inline at the Phase 6 items
- [x] Applied `[lints] workspace = true` in all 35 member manifests; workspace builds warning-clean
- [x] Fixed false claims in `scripts/gen-crate-docs.py` (weight deviation example now states the real number; core round-trip claim scoped to the project model; wasm claim no longer asserts a browser smoke test) and regenerated all crate READMEs *(the twin "failures leave the twin untouched" claim is now true after 7A/A3, so it stays)*
- [x] `tpt-yard-wasm` in `crates/core` depends on `crates/space`; fixed by moving the crate to `crates/web/tpt-yard-wasm` (package name and API unchanged; workspace members, facade path and docs updated)
- [x] README wording aligned: crates are `MIT OR Apache-2.0`; cargo-deny enforces the permissive allow-list and denies copyleft
- [x] Removed the `.kilo/worktrees/` scratch checkout and added `.kilo/` to `.gitignore`
- [x] Book logistics/facility chapters added with plain prose (no unresolvable intra-doc links); README snippets compile and are asserted by a real test. *(the full-book doctest pass is 7F's `scripts/test-snippets.py`, which compiles every non-`ignore` book + README snippet in CI)*
- [x] Added logistics and facility chapters to `docs/book/src/SUMMARY.md` (with honest status notes)
- [x] `www/index.html`: "Run" disables itself while running (no double timers); the interval stops on completion AND on a failed step; Reset clears the timer before replacing the twin; the old twin is `free()`d on reset and unload; a missing `pkg/` build shows an actionable message with the build command instead of a bare module error

### 7F. Adoption and onboarding

- [x] README Quick Start rewritten (activity ids collected before `&mut twin` use; no phantom file; mass-tracking path documented); kept compiling and asserted by `tpt-yard-digital-twin/tests/readme_quickstart.rs` so it cannot rot
- [x] Compile-test every README and book snippet in CI: `scripts/test-snippets.py` extracts every non-`ignore` ```rust block from `docs/book/src/*.md` and the root README, wraps fragments (fallible fragments get a `Result`-returning main) and compiles each against the built workspace rlibs; runs in the CI `examples` job. The harness caught and fixed a real borrow bug in the introduction chapter; context-dependent continuation fragments are tagged `rust,ignore` honestly.
- [x] Facade crate `tpt-yard` (in `crates/facade/`): re-exports every domain crate as a module, a curated `prelude` (doctested), and `sea`/`space`/`planning`/`wasm` feature flags with the default build covering all three domain layers
- [x] CLI `tpt-yard-cli`: `new <sea|space>` scaffold, `validate | plan | schedule | report FILE.json`, text/JSON/HTML output *(2026-10-02: the shipped command set is wider still — `risk`, `html-report`, `pdf-report`, `--limit` capacity levelling on `schedule`)*
- [x] All six examples read their test data and accept a manifest path argument (defaulting to the shipped reference data, so bare `cargo run` still works): `orbital-station-truss` (iss-truss-manifest.json), `container-ship-block-assembly` (hull-blocks/container-ship-140m.json), `submarine-pressure-hull` (hull-blocks/submarine-pressure-hull.json), `drydock-flooding-sequence` (golden/sea/drydock-flooding-sequence.json), `rotating-habitat-construction` (orbital-structures/rotating-habitat-torus.json, now valid JSON), `space-solar-array-assembly` (orbital-structures/solar-array-manifest.json, added).
- [x] Project templates: `tpt-yard new container-ship|submarine|orbital-truss|habitat|solar-array` scaffolds a validating project; the same five JSONs ship in `templates/` for copy-into-repo workflows (every template passes `tpt-yard validate` — which caught two template bugs during development)
- [x] Deploy mdBook to GitHub Pages; add `[package.metadata.docs.rs]`: the `Pages` workflow builds the book (mdbook 0.4.40) plus the WASM dashboard (pinned `wasm-bindgen-cli@0.2.128`) and deploys `/` (demo) and `/book/` on every push to master; `[package.metadata.docs.rs] all-features = true` added to all 31 member manifests.
- [x] README badges added (CI, license, docs.rs, MSRV)
- [x] `justfile` with `check | test | wasm | book | docs` recipes; `.devcontainer` and a multi-stage CLI `Dockerfile`
- [x] Deployed WASM demo on Pages that draws the WASM mesh in 3D (three.js): the Pages workflow ships `www/` with a freshly built `www/pkg`; offline/CDN failure degrades to an explicit message and the buttons stay usable.

### 7G. CI and release automation

- [x] `Cargo.lock` tracked (removed from `.gitignore`); CI clippy/test/docs and the release publish all run `--locked`
- [x] The license workflow runs the FULL `cargo deny check` (licenses, bans, advisories, sources) weekly on a cron in addition to every PR; Dependabot config added for cargo + GitHub Actions ecosystems (weekly, grouped minor/patch)
- [x] DCO job added to CI: every commit in the PR/push range must carry `Signed-off-by:`, matching what CONTRIBUTING and the PR template promise
- [x] Release workflow: a `verify-tag` job fails unless the tag matches the workspace version; publish runs a `--dry-run` first and is `--locked` (no `--allow-dirty`); the WASM package is built with pinned `wasm-bindgen-cli` and attached to the GitHub release; notes are generated by the release action. release-plz added for changelog/version automation (`release-plz.toml` workspace config, `release-plz.yml` with the standard release/PR jobs; `publish = false` there — crates.io publishing stays with the tag-verified `release.yml`).
- [x] Real wasm CI job: installs `wasm-bindgen-cli@0.2.128` (matching the workspace `wasm-bindgen = "=0.2.128"` pin), installs the Chrome-for-Testing chromedriver matching the runner's Chrome, runs the browser tests via `cargo test --target wasm32-unknown-unknown` (`.cargo/config.toml` wires `wasm-bindgen-test-runner`; the same flow as `wasm-pack test --headless` but with the CLI pinned instead of wasm-pack's bundled version), and builds/uploads `www/pkg` as a CI artifact.
- [x] MSRV job runs `cargo test --workspace --locked`
- [x] Coverage CI job: cargo-llvm-cov over the workspace, lcov artifact uploaded
- [x] Benchmarks: all four benches converted to criterion (grouped benchmarks like `welding/thermal_cycle_8mm`); the benchmark workflow saves a criterion baseline on master pushes and gates PRs with critcmp at a 20 % threshold; a smoke `--quick` run happens on every PR
- [x] `cargo-semver-checks` job added to CI; docs build with `-D warnings` via the workspace `RUSTFLAGS: -D warnings` env (warnings fail the doc step)
- [x] `examples` CI job runs all six example binaries

### 7H. New features (after 7A-7D)

- [x] Hydrostatics and stability — **first slice shipped** as `tpt-yard-hydrostatics`: prismatic hydrostatic table (displacement, KB, KM, TPC, MCT1cm, LCB/LCF=0 for the prismatic model), wall-sided GZ curve, free-surface correction, and the IMO 2008 IS Code general criteria (A30/A40 areas, max-GZ angle, min GM) — closed-form-verified with tests. *(2026-10-01: trim shipped — `trim_equilibrium` solves the prismatic even-keel-plus-trim state for a displacement and LCG by nested bisection over the trimmed-box integral, verified against the exact closed form `tan theta = 12 W LCG/(rho Cb B L^3)` and loosely against the MCT1cm screening estimate. Damage stability added (2026-10-01): `damage_stability` runs the added-weight method — flood compartments to the sea line, re-solve the trim equilibrium with the grown displacement and shifted LCG/TCG, report damaged GM (intact free-surface moments supplied per compartment) and the small-angle list, screened against the 0.05 m one-compartment floor. Verified to the exact closed-form identities, including the honest physics that a LOW flood can raise GM (KG dilution beats the free surface). **Hull offsets, Bonjean curves and cross-curves shipped** (2026-10-01): `Bonjean` accepts per-station half-breadth offset tables (`SectionOffsets`), integrates displacement and LCB at any draft and trim (Simpson over the station span; verified against the box closed form and the V-section triangle area `A = T^2`), and `cross_curve_ordinate` strip-integrates the submerged widths under the inclined waterline (rotation about the centreline point, the classical convention) to give KN at any draft and heel — verified against the box closed form `KN = (T/2 + B^2 tan^2/(24T))cos - B^2 tan sin/(12T)` in the small-heel regime, the exact constant-volume keel-clipped triangle at 45 deg, and the honest GZ sign flip of a KG = T/2 box. Multi-case damage screen added (2026-10-01): `damage_screen` runs the added-weight method over a named case set and reports per-case GM/list/trim, the governing (lowest-GM) case and the all-pass verdict — the deterministic precursor to the probabilistic method (shipped below as `probabilistic.rs`). Per-station linear interpolation of the half-breadths is wired into the heeled strips (the nearest-station pick is gone). **Probabilistic damage stability shipped** (2026-10-02): `probabilistic.rs` implements the SOLAS Ch. II-1 Part B-1 harmonized method for cargo ships — the bi-linear damage-length density of Reg. 7-1 (`Jmax = 10/33`, knuckle `5/33`, cumulative `11/12` there, support `min(10/33, 60/Ls)`, rescaled by `L*/Ls` above 260 m; the four coefficients fixed by the published density properties), the zone factor `p_factor` (Reg. 7-1.1/1.2/1.3: mid-ship containment, terminal `(p+J)/2`, whole length 1), the cargo survival factor `s_factor_cargo` (Reg. 7-2: `K·[(range/16°)(GZmax/0.12 m)]^0.25` with the 15–30° heel gate), `required_index_cargo` (`R = 1 − 128/(Ls + 152)`, above 100 m), `attained_subdivision_index` + the 0.4/0.4/0.2 three-condition weighting, and `damaged_survivability_cargo` (added-weight flood → wall-sided damaged GZ scan → the Reg. 7-2 inputs). Verified: the coefficients match exact hand fractions (b11 = −3267/50, b12 = 11, b21 = −363/50, b22 = 11/5 at Ls ≤ 198 m), the density integrals, `p` against Simpson integration of the contained-damage integral in both branches, p2's derivative against the cumulative, `p(Jm) = Jm − E[J]`, terminal monotonicity, and s-factor hand values. **r-factor + multi-zone combinations shipped** (2026-10-02): `r_factor` (Reg. 7-1.2, transcribed verbatim — the statute's equation image was fetched and read directly: `r = 1 − (1−C)·[1 − G/p(x1,x2)]`, C = 12·Jb·(4−45·Jb), Jb = b/(15B), G per the three terminal cases) and `multi_zone_p_factor` (the p·(r_k − r_(k−1)) alternating combinations for one-, two- and three-plus-zone groups, pure-p when no bulkhead). The multi-zone test EXPOSED a transcription error in the earlier p2: the printed formula mixes the RAW zone length J (linear-coefficient terms) with Jn (the density-integration limits) — using Jn everywhere saturates long zones and drove the three-zone combination negative; fixed and the distinction documented. **Passenger s-factors, v factor and the 80–100 m R interpolation shipped** (2026-10-02): the statute's remaining equation images were fetched and read directly — `s_final = K·[(Range/TRange)(GZmax/TGZmax)]^{1/4}` with the ro-ro caps (0.20/20) and the heel gates (pax 7/15, cargo **25/30** in the MSC.421(98)-amended text — the pre-amendment literature's 15 was corrected, tests updated), `s_intermediate = [GZmax/0.05 × Range/7]^{1/4}` with the 15/30 gates, `s_mom = (GZmax − 0.04)·Δ/Mheel` capped, `v(H,d) = 0.8 + 0.2·[(H−d)−7.8]/4.7`, and the 80–100 m R interpolation (continuous with R0 at 100 m, hand-checked at 80 m). **Staged flooding + survival-craft moment shipped** (2026-10-02): `damage_stages` floods every compartment at fractions i/n (volume and free-surface moment scale, centroids fixed — the documented added-weight approximation) and reports each stage's state plus the wall-sided GZ scan; the final stage reproduces `damage_stability`/`damaged_survivability_cargo` EXACTLY (identity-tested), drafts grow monotonically, and an off-centre flood heels progressively. `survival_craft_moment` sums the swung-out craft list (mass * g * outboard arm). **Tank-plan flooding + cross-flooding dynamics shipped** (2026-10-02): `flooding.rs` — `TankCompartment` (bottom/plan-area/height/permeability) fills physically from the bottom up (`flood_volume_m3`, `flood_cg_z`, square-plan free-surface estimate, `tank_stage_compartment` bridging into the staged solver) and `cross_flooding_time` integrates Torricelli's orifice law for the Reg. 7-2.2 equalization (closed form `t = 2(sqrt(H0) − sqrt(H1))/(Cd·A·sqrt(2g)·(1/Sw + 1/Sf))` verified against brute-force ODE integration and hand values). **Cross-flooding dynamics inside the stage physics + tapered walls shipped** (2026-10-02): `damage_stages_equalization` (HullForm) drives the stage fractions by the Torricelli profile `f(u) = 2u − u²` (equal TIME slices — half the transfer in the first 29.3% of the equalization) instead of equal volume slices, returning the Reg. 7-2.2 equalization time alongside; the final stage reproduces `damage_stages` exactly. `TankCompartment` gains `top_plan_area_m2` (trapezoidal walls): the fill height solves the trapezoid integral with the quadratic formula, CG via the first-moment closed form — a flaring tank concentrates volume above the mid-fill mark. **Large-angle heel + cross-flooding inside the stage equilibrium shipped** (2026-10-08): `heel_equilibrium_deg` solves the wall-sided balance `GM sin(phi) + (BM/2) tan^2(phi) sin(phi) = TCG cos(phi)` by bisection (exactly `atan(TCG/GM)` with BM = 0, stiffer than small-angle for big lists, +/-90 when there is no equilibrium) and now sets `DamageResult::list_angle_deg` and every stage; `damage_stages_cross_flooding` tags each compartment breach or equalizing — breach tanks flood fully at stage 1, equalizing tanks follow the Torricelli profile — so the signed per-stage heel shows the asymmetric peak and its recovery (monotone to upright for mirror tanks, final stage identical to `damage_stability`). Rule constants were taken from the SOLAS Ch. II-1 regulation text (Reg. 6/7-1/7-2) and cross-checked against the Revised Explanatory Notes (MSC.429(98)/Rev.1) — including the R denominator 152 (not the older 148).)*
- [x] Dynamic slipway launch simulation — **shipped** in `tpt-yard-launch` as `dynamic_launch()`: time-stepped along-ways equation of motion (gravity, grease friction with static breakaway, buoyancy relief with the same immersion model as the static float-off screen, immersion-growing quadratic drag), sampled trajectory (t, travel, velocity, buoyancy, way reaction), and end conditions Afloat / Stuck / TipUp with a pivot moment. Verified against the closed-form energy balance (buoyancy-free run), stuck-ways, reference float-off with the classical checking phase, and tip-up detection. *(2026-10-01: the remaining sub-items were already shipped — `LaunchSample` carries `stern_lift_moment_knm`, `poppet_load_kn` and `end_of_ways_moment_knm` as per-step time series, verified by `dynamic_series_tracks_stern_lift_poppet_and_moment` (poppet stays zero in a healthy run, concentrates in a tip-up; stern lift grows with buoyancy; the moment curve is bounded by W x way length). The todo note was stale.)*
- [x] Drydock: virtual GM at touchdown (shipped in the 7B pass as `virtual_gm_touchdown_m`), keel-block reactions (`keel_reaction_distribution`: total from Archimedes, centroid from the moment balance about midship, linear pressure law over the block row with the single-end lift-off flagged past L/6 and clamped/renormalised), and ballast sequencing (`ballast_plan`: keel-line ballast in equal increments until the float-off GM reaches the target, steps emitted pre-flood at level 0, `InsufficientBallast` instead of a fake pass; `DockedVessel` gains `lcg_from_midship_m` and typed `BallastTank`s). Verified against hand hydrostatics (KM drop with ballast draft, discrete-centroid (1-1/n^2) factor).
- [x] Lifting lug/padeye checks — **shipped** in `tpt-yard-blocks`: `padeye_check` covers pin bearing (0.9 sigma_y), net-section across the hole (sigma_y/1.5), tear-out (double shear to the edge, 0.6 sigma_u/1.5), root out-of-plane bending + tension interaction, and fillet-weld throat shear, all against the DAF-amplified design load. *(2026-10-01: complete — the CoG uncertainty envelope and spreader-beam screen were already shipped with tests (`cog_envelope_widens_worst_case_leg`, `spreader_beam_statics_and_buckling`); the todo note was stale. Pin bending newly shipped: `pin_bending_check` models the shackle pin as a simply supported beam spanning the clevis with the lug bearing as a distributed load — midspan moment `P(L-t)/4`, double shear at the supports — hand-verified in `pin_check_matches_hand_values`.)*
- [x] Member buckling and slenderness - **shipped** in `tpt-yard-structural` as `check_member_buckling`: slenderness ratio `K*L/r` with the four end-fixity factors, Euler critical stress, the bilinear governing curve (Euler slender / yield stocky) and the 200 slenderness practice limit - the Euler closed form is verified against `pi^2 E I / L^2` including the fixed-fixed x4 factor; a demand-units bug in the first cut was caught by its own tests. **Sparse solver shipped** (2026-10-01): the penalized stiffness system now solves by Jacobi-preconditioned conjugate gradient over a CSR matrix (the dense Gaussian is gone), converging on the free-row residual relative to the loads — the penalty rows carry the support reactions and can never vanish, which the raw-|r| criterion ignored. Singularity is detected by a scale-invariant curvature breakdown test and non-convergence. Verified on the whole existing suite, both proptest batteries, and a new 100-segment hanging chain (~300 DOF) whose lumped-weight element forces and tip stretch match the closed form. **Beam elements shipped** (2026-10-01): `FrameModel` solves plane frames (2-node Euler-Bernoulli members, 3 DOF/node, full transformation, consistent uniform-load vectors, penalty BCs over the shared sparse CG solver) in `tpt-yard-structural::frame`. The Hermite-cubic elements are exact at the nodes for end loads: cantilever PL^3/3EI + PL^2/2EI + root moment PL, simply-supported midspan PL^3/48EI, uniform-load end rotation -wL^3/24EI with vanishing recovered support moments, axial stretch PL/EA, and a slender inclined triangle approaching the truss solution. One real sign error in the moment recovery (the -6EI Delta/L^2 slope-deflection term) was caught by the cantilever test. **Plate elements shipped** (2026-10-01): `plates.rs` — Bogner-Fox-Schmit 16-DOF rectangular plate bending elements (C1 conforming, Hermite tensor products, consistent pressure vectors) on axis-aligned meshes, solved by a dense Cholesky (the mixed w/slope/twist DOF scales put unpreconditioned CG beyond any iteration budget; truss/frame keep CG). Verified against the Timoshenko closed forms: simply supported 0.00406 q a^4/D and clamped 0.00126 q a^4/D on an 8x8 mesh, plus an SPD probe on the single-element matrix. Mindlin (shear-deformable) plates: **shipped 2026-10-02 on the second attempt** as `tpt-yard-structural::mindlin` — the MITC4 assumed-strain element (Bathe-Dvorkin) with the kinematics u = z·theta_y / v = −z·theta_x (thin limit theta_y = −w,x), covariant midsides shears on the four edges and bilinear interpolation between them, 2x2 Gauss bending + shear, dense Cholesky with penalty BCs (same path as BFS). The prescribed instrumented convergence study is the headline test: Mindlin vs BFS on identical 4/8-meshes agree within 5 % and both hit the Timoshenko 0.00406 / 0.00126 targets with monotonically shrinking error; strain-free probes (rigid, linear-slope) and the twisting thin-limit mode (energy exactly d_b(1−nu)·A, w-rows of K·u zero) pin the operators; a thick-plate test confirms shear adds real deflection at span/thickness 5. **The first attempt's failure was found and was exactly one of the recorded suspects**: the gamma_yz edge operator at midsides B had its deflection coefficients flipped (gamma = (w2−w3)/dy instead of (w3−w2)/dy) — a sign error invisible to rigid/linear probes (no y-variation) and invisible to the naive global comparison (the solution still LOOKS like a plate, it just locks ~500x); the twist-mode probe exposed it (2000x energy inflation), the fix took one line. **Curved members shipped** (2026-10-02): `FrameModel::add_arc` (faceted circular arcs on the frame element) + `solve_dense` (a dense-Cholesky path for closed rings/arcs, where self-equilibrated loads leave rigid modes the CG free-row residual test is blind to) — verified on the faceted ring's membrane state (a half ring under radial pressure with symmetry restraints: member axial converges to the hoop closed form pR, bending vanishing, 18→36 chords) and an inclined-cantilever closed form. **Cylindrical shell screening shipped** (2026-10-02): `shells.rs` — `cylinder_shell_screen` (thin-wall hoop `p·d/(2t)` and longitudinal `p·d/(4t)` stresses plus the Windenburg–Trilling elastic external-pressure buckling `2.42·E·(t/d)^2.5/(1−ν²)^0.75` with SF gate and salt-water crush depth; hand-verified at t/d = 0.005 → p_cr = 0.964 MPa, crush ~96 m; the pressure-hull slice of the curved-shell item). **The ring verification caught a REAL transformation bug in the frame element**: both rotation passes in the member assembly were transposed (kg = T·kl·Tᵀ instead of Tᵀ·kl·T) — invisible on horizontal members (identity rotation) and on axial-only inclined checks; an inclined-cantilever probe (tip deflection = PL³/3EI exactly) exposed it, one-line fix, all prior tests unaffected. The ring test also documents the linear-model boundary: above the classical ring buckling load 3EI/R³ the faceted response amplifies (no stress stiffness in a linear model). **Curved-shell FEM shipped** (2026-10-08, flat-facet formulation): `shell_fem.rs` — planar rectangular facets with a Q4 plane-stress membrane + the MITC4 Mindlin bending kernel + a small drilling stiffness, six DOF per node, any 3-D orientation, exact DOF elimination and a dense Cholesky solve. Verified: a stretched plate elongates exactly, a clamped plate gives the same deflection (to 1e-8) flat and tilted into an arbitrary plane (so the 24x24 rotation is right) and matches Timoshenko 0.00126 qa^4/D, and the Scordelis-Lo barrel vault converges to the MacNeal-Harder 0.3024 m (0.2848 at 4x4, 0.2951 at 8x8, -2.4 %). Scope: planar rectangles only — cylinders, prisms, tanks and flat panels mesh exactly; doubly curved surfaces would need triangles or warped quads. Class rules are tracked under hull-girder strength below.*
- [x] Hull-girder still-water bending/shear - **shipped** in `tpt-yard-hydrostatics`: `hull_girder_strength` computes the load (prismatic buoyancy minus lumped/spread weights), shear and moment curves with free-end trim correction, and screens the peak SWBM against a CSR-style `C*L^2*B*Cb` allowable. Verified: midship weights sag, end weights hog, the shear zero-crossing sits at the peak moment, both curve ends are free, and plausible loadings land at realistic utilizations (0.05-0.5). *(2026-10-01: wave-induced bending moment shipped — `wave_bending_moment` gives the IACS CSR sagging +0.11 Cw L^2 B (Cb+0.7) / hogging -0.13 ... closed form with Cw = 10.75 - ((300-L)/100)^1.5, None outside the 90-300 m rule range; combine with the still-water screen in `hull_girder_strength`. **CSR scantling screening shipped** (2026-10-01): `scantling_requirement` combines the supplied still-water moment with the rule wave moments against the `175/k` allowable (`high_strength_factor` maps MS/AH32/AH36/AH40 grades), returning the required hull-girder section modulus; hand-checked end to end, with the AH36-vs-mild 1/0.78 modulus ratio asserted. **Local scantlings + plate buckling shipped** (2026-10-02): `scantlings.rs` — the clamped-slab lateral-pressure leg `t = 22.36·alpha_p·s·sqrt(k·p/sigma)` (the constant derived from `sigma = 0.5·p·s^2/t^2` in (m, kN/m2, N/mm2) units — the rules print the rounded 22.4; the demand-side `k` matches the `175/k` hull-girder convention), the classical simply-supported Euler plate stress with the half-wave aspect factor `(m·b/a + a/(m·b))^2` (k = 4 at square panels and half-wave resonances, hand-checked at 116.4 MPa for an 800/3200/10 mm steel panel), its exact-inversion buckling thickness, and the combined `local_plate_scantling` with corrosion addition and governing-mode report. Verified: round-trip identities (the slab stress reproduces sigma/k; sigma_E(t_req) = sigma_applied), scaling laws, aspect factors, mode flipping, and typed errors. **Stiffener checks shipped** (2026-10-02): `stiffener_scantling` — the fixed-fixed lateral-load model (end moment p s l^2/12, midspan p s l^2/24, reaction p s l/2), required section modulus `1000·M/sigma` in cm3 and optional shear area `10·V/tau` in cm2, demand-side k. Verified against the closed forms with span/k scaling identities and a stress-recovery round-trip. **EN 1993-1-5 buckling reduction shipped** (2026-10-02): `plate_buckling_reduction_ec3` + `plate_buckling_check_ec3` — the clause 4.4 Winter-type curve for internal elements transcribed verbatim from the standard text (`rho = (lambda - 0.055(3+psi))/lambda^2` capped at 1, knee limit 0.5 + 0.085/(1+psi)), combined with the Euler stress into the rho·fy capacity and utilisation; hand-verified (rho(1) = 0.78, rho(2) = 0.445 for uniform compression, the 0.673 cap knee, psi-shifted knees) with an end-to-end 800x3200x10 AH36 panel check. **Rule minimum thicknesses via a user-supplied table shipped** (2026-10-08): `RuleTable` loads a yard's own society minima from JSON (`t_min = base + coeff·L^exp`, optional clamps, net or gross basis, strict loader, schema + placeholder sample) and `check_plate` reports the governing of the calculated scantling and the rule minimum; the engine ships no class values. **Won't do:** transcribing the CSR buckling eta curves and minimum thickness tables — they are society-owned, edition-specific rule content (the CSR PDFs are formula images with no plain-text source), would go stale in an open repo and could pass a design the current rules reject; the Eurocode curve is the verified stand-in and the rule table covers the minima.)*
- [x] Weld procedure advisor in `tpt-yard-welding::advisor`: carbon equivalents CE(IIW)/CET (EN 1011-2)/Pcm (Ito-Bessyo), Graville class, the SEW 088-style preheat screen (`advise_preheat`), the Rosenthal t8/5 closed form inverted for a target-cooling-time preheat (`preheat_for_target_t8_5`), end-to-end screening against a supplier t8/5 window (`advise`, with a recommended preheat when the floor is missed), and WPS-inside-PQR essential-variable checks (`Pqr::qualifies`: process, heat input, preheat/interpass, filler, and the ISO 15614-1 Table 5 0.5·t-2·t thickness rule; ASME IX labelled with the same screening envelope, deposit rules documented as finer). Verified against closed forms (CE/CET/Pcm hand values, inversion round-trip) and an end-to-end dirty-procedure test where the recommendation lands t8/5 inside the window.
- [x] `tpt-yard plan` end-to-end (in the CLI): block division → erection order → heaviest-block lift check (with notes) → erection schedule → critical path, one text or JSON report. *(the drydock/launch legs remain pointers to the dedicated crates)*
- [x] Monte Carlo schedule and weight risk — **schedule risk shipped** in `tpt-yard-scheduling`: `monte_carlo_risk` samples triangular duration distributions, runs the CPM per sample, and reports P50/P90/mean makespans plus per-activity criticality frequencies (seed-deterministic, verified against the closed-form makespan at zero uncertainty). **Weight risk shipped** (2026-10-01) as `WeightModel::monte_carlo_risk`: triangular item-weight sampling, P50/P90/mean totals plus P50/P90 CoG shifts, seed-deterministic, mean-at-nominal verified with a Replaced-item exclusion check. **Delivery-slippage coupling shipped** (2026-10-01): `cpm_with_releases` (earliest-start gates in the CPM forward pass; a gated activity that the gate dominates becomes critical) and `monte_carlo_risk_with_gates` / `DeliveryGate` — each gate's availability is sampled triangularly around the expectation per run, seed-deterministically. Verified: deterministic gate delays + criticality propagation through dependencies, zero-slippage reduction to the deterministic gated makespan, P90 >= P50 >= mode ordering, seed determinism, unknown-gate errors. The CLI exposes it as `risk project.json [--samples N] [--uncertainty F] [--gate id=h[:slip]]`. **Weather-window coupling shipped** (2026-10-01): `SeaStateForecast::from_json_value` (strict loader — Douglas range 0-9 enforced, non-numeric entries typed errors) plus the CLI `risk --weather forecast.json [--launch-method slipway|drydock|side|shiplift] [--max-sea-state N]` — the launch-window forecast gates the launch activity at the first calm run of hours at or below the sea-state limit (the stricter of the method practice limit and the site operational limit), and the risk run reports the weather gate alongside delivery gates; verified end-to-end in the earth-link loader tests. **tpt-earth bridge shipped** (2026-10-08): `tpt-earth` (the sibling repo, ocean-waves crate) is not on crates.io, so a dependency would break `cargo publish`; instead `douglas_from_hs` (WMO sea-state table, upper-inclusive) and `SeaStateForecast::from_significant_wave_heights` take the one number its `WaveSpectrum::significant_wave_height()` produces, and the forecast file (and so `risk --weather`) accepts `hourly_hs_m` as well as `hourly_sea_state`. A hard dependency waits for a published release.
- [x] Design-for-construction optimizer (2026-10-01): `HullConstruction::optimize_division` ranks (depth bands x longitudinal divisions) plans by an erection-effort proxy — seam length / weld speed + lifts x hours per lift — under the derated crane and workshop-length feasibility, best first. Tests hand-check the seam formula, the light-crane-forces-more-divisions trade-off, and that nothing liftable-infeasible survives.
- [x] Live digital-twin ingest: `DigitalTwin::ingest_telemetry` accepts a sensor/scan-deviation JSON batch (`test-data/telemetry/sample-batch.json` documents the schema), stores readings in `sensor_data`, and flags scan deviations beyond a 5 mm tolerance as *rejected quality records* (corrections are flagged for heat-straightening, never auto-executed); malformed batches are rejected whole, not partially applied
- [x] Report generator: `tpt-yard html-report <project.json> [--out file.html] [--structure partial.json]` produces a styled HTML calculation package - weights (design/best-estimate/margin), by-group table, per-phase table, structural-check verdict, **schedule section** (critical path + levelled makespan) and **schedule risk** (Monte Carlo P50/P90/mean + most-critical activities), and **per-phase FEM** (2026-10-01): with `--structure`, a strict JSON loader (`PartialStructure::from_json_with_loads` — nodes/elements/supports plus gravity/wind/crane loads, review-7C standards) feeds `analyze_at_phase` for every erection phase; the demo trestle shows deflection and stress dropping as the second member is erected (1.9 -> 1.0 mm, 13.5 -> 6.8 %). **PDF output shipped** (2026-10-01): `pdf-report project.json [--out file.pdf]` renders the full package (weights, by-group, phases, schedule, risk, structural check) as a dependency-free PDF 1.4 document — uncompressed text streams, Helvetica/Courier, A4 pagination with margin-aware page breaks; verified structurally (header, catalog, xref offsets point at "xref", EOF) and by unit tests on pagination, escaping and document structure. *(report item complete)*
- [x] glTF export - **shipped** in the `tpt-yard` facade: `export::geometry_to_gltf` emits a glTF 2.0 document (embedded base64 buffer, TRIANGLES, required POSITION min/max accessors) that any three.js/Babylon/Blender viewer loads; the test decodes the buffer back and checks byte-level layout. **IFC export shipped** (2026-10-02): `elements_to_ifc`/`geometry_to_ifc` write an IFC4 STEP physical file (ISO 10303-21) — Project/Site/Building/Storey structure, one IfcBuildingElementProxy per element with the mesh lossless as an IfcTriangulatedFaceSet; verified: STEP well-formedness (markers, sequential ids, every reference defined), an EXACT geometry round-trip (the emitted shortest-round-trip reals parse back to bit-identical vertices/faces), STEP string escaping, deterministic output and synthetic 22-char GUIDs; scope documented (one storey, no property sets, fixed header timestamp). Book chapter added. **CLI wiring + 3-D report viewer shipped** (2026-10-02): `export MANIFEST.json --gltf/--ifc` writes the block division end-to-end, `geometry_from_gltf` is the writer's round-trip-tested inverse, and `html-report --gltf-viewer hull.gltf` embeds an interactive three.js view of the blocks in the calculation package. **PyO3 Python bindings shipped** (2026-10-02, first slice): `crates/bindings/tpt-yard-py` — a workspace-EXCLUDED detached crate (pyo3 needs a Python interpreter at build time, so plain workspace builds never touch it; root Cargo.toml `exclude` documents it) exposing `DigitalTwin` (strict project-JSON loading, advance_phase, weight_report, JSON persistence), `HullForm` (hydrostatics, GZ curve, IMO 2008 check) and `Scheduler` (critical path, objectives with dock occupancy, capacity-aware levelling, Monte Carlo risk), engine errors as ValueError. Verified end-to-end on Python 3.13: import of the renamed cdylib, the container-ship template load, phase advance, the hydrostatics closed form (0.72·140·22·6·1.025), and ValueError on malformed JSON; a dedicated CI `python-bindings` job runs the same flow on 3.12. *(the 7H export/bindings items are complete; curved shell elements shipped under member buckling above; the class-rule tables remain under hull-girder strength)*

### 7I. Adoption pass (2026-10-08)

- [x] glTF reader bounds-checks declared counts against the buffer (no slice panic, no hostile allocation) and range-checks face indices; digital-twin keel-block sort uses `total_cmp`; examples report bad manifests as one `error:` line
- [x] `Bonjean` Simpson interval count is even for any station count (was odd, hence wrong, above 25 stations)
- [x] `Bonjean::cross_curve_ordinate` returned the earth-vertical height of B, not KN; now `KN = -y_B cos phi + z_B sin phi` (the 45° box test had encoded the wrong formula and called a GM 6.7 m box unstable — corrected to GZ = 4·sqrt(2) m)
- [x] Offsets path: `parse_offsets_csv`, `Bonjean::hydrostatics` (KB, KM, LCB, LCF, TPC, MCT1cm) and `Bonjean::gz_curve` (exact constant-displacement cross curves by polygon clipping), checked against the Wigley hull's closed-form values; `imo_2008_general_criteria` takes any `GzCurve` and refuses curves that stop short of 40°
- [x] `tpt-yard stability CASE.json` with `--csv`, `--svg`, `--json`, `--strict`; samples in `test-data/stability/`; book chapter `intact-stability.md`
- [x] `tpt-yard new-hull <wigley|barge|workboat>` writes a runnable offsets CSV + case; `--help`/`-h`, `--version`, `help <cmd>`, per-command usage and a did-you-mean hint for mistyped commands; box-hull exact wedge solution checks heeled GZ to 25°
- [x] `new-hull` templates `tug`, `sailboat`, `ferry` (round-bilge generator; each runs through `tpt-yard stability` in the test suite)
- [x] `tpt-yard import-hull MESH.obj|.stl` (`meshhull.rs`): OBJ (polygons fan-triangulated, negative indices) and ASCII/binary STL sliced by transverse planes into the offsets CSV + stability case (`--up`, `--bow`, `--scale`, `--stations`, `--levels`); a box mesh reproduces the barge closed form, and axis-swapped, flipped-bow and STL variants match the OBJ result. Limit: non-re-entrant sections.
- [x] Schemas for the offsets path: `schemas/stability-case.schema.json` (case file: offsets CSV or prismatic hull + loading) and `schemas/hull-offsets.table-schema.json` (Frictionless Table Schema for the CSV); a test checks the shipped cases' keys and the CSV header against them so they cannot drift
- [x] Python wheel for the offsets path: `OffsetsHull` (`from_csv`/`from_csv_file`, `hydrostatics`, `gz_curve`, `imo_2008_check`; bad input is `ValueError`) in `tpt-yard-py`, plus `pyproject.toml` for maturin; built, installed into a clean venv and checked against the Wigley closed forms (`tests/offsets_smoke.py`, also a CI step that uploads the wheel). PyPI publishing not set up.
- [x] Benchmark hulls KCS and DTMB 5415 via `scripts/fetch-benchmark-hulls.py`: downloads the SIMMAN 2014 IGES hulls on demand (not redistributed — the site states no licence; output in git-ignored `test-data/benchmarks/`), checks pinned SHA-256s, tessellates the NURBS surfaces (entity 128) to OBJ and runs `import-hull`; displacement at the published design draft lands at +1.02 % (KCS) and +0.12 % (5415) of `Cb·Lpp·B·T`, and the script fails beyond 2 %. The 5415 is a 1:24.83 model with the dome below the keel line (baseline offset 3.0 m, handled). Not in CI (network). Series 60 still needs a source.
- [x] Prebuilt release binaries: `release.yml` `binaries` job builds `tpt-yard` natively for x86_64/aarch64 Linux, x86_64/aarch64 macOS and x86_64 Windows (tag-verified, after the pre-release tests), smoke-tests each (`--version`, `validate`, `new-hull` + `stability --strict`), packages with `scripts/package-cli.py` (binary, both licences, README, templates, schemas; `.tar.gz`/`.zip` + `.sha256`) and attaches the archives plus a combined `SHA256SUMS` to the GitHub release; a `cli-binary` CI job runs the same build/smoke/package on Linux, macOS and Windows for every PR. Verified locally on Windows only (zip and tar branches, unpacked binary run end to end); the matrix itself has not run on GitHub yet. Binaries are unsigned.
- [-] Series 60 benchmark hull — **won't do**: the Todd 1963 offsets are only reachable behind bot-blocked hosts, and the benchmark role is already covered by the analytic Wigley hull plus the KCS and DTMB 5415 conversions. Any offsets CSV or mesh the user trusts goes through `import-hull` as is.

---

## Phase 8: Review Follow-ups (2026-10-08 platform review)

Source: three read-only audits (hydrostatics + CLI, core/planning/structural/space, adoption). Items tagged **[verified]** were re-checked against the code; the rest are audit claims to confirm first. Anything that depends on regulation text (SOLAS, IACS, EN 1993-1-5) must be checked against the standard, never fixed from memory. Every fix needs a regression test that fails before the fix; regenerate goldens only after an independent hand calculation.

*Already edited in the working tree, tests not yet run:* `yaw_diff` via `rem_euclid` plus a regression test; `eprintln!` debug removed from `structural/fem.rs`; Whipple standoff `d/100` m (tests, book, crate README and `gen-crate-docs.py` updated). Tick them after `cargo test` passes.

### 8A. Wrong answers that look like passes

- [x] 8A1 **[verified]** `tpt-yard-joints` `weld_area_mm2_total`: DoubleV/K groove area already sums both sides, then is doubled again (welding gets 2x shrinkage and angular distortion). Also: square butt area is 0 (gap applied only over a 0 root face), V omits gap x depth, single-bevel uses `d^2 tan(theta/2)`; no angle validation (180 deg = infinite area). The test `square_groove_is_gap_only` enshrines the 0. **Done:** DoubleV/K no longer doubled; the root gap fills gap x t; bevel/K profiles use the right-triangle d^2 tan(a)/2; angle and dimension validation (try_weld_area_mm2, NaN from weld_area_mm2 when invalid); welding golden re-derived by hand (area 93.735 mm2, longitudinal 0.0527, angular 1.0073, bow 0.000633).
- [ ] 8A2 **[verified]** `hydrostatics/probabilistic.rs` `gz_scan`: the wall-sided arm only grows and the scan runs to 90 deg (tan blows up), so damaged GZmax/range and the attained index A are overstated. Needs a downflooding / deck-immersion cap (a depth or openings input on `HullForm`).
- [x] 8A3 **[verified]** `scantlings.rs` `aspect_factor`: uses `m = ceil(a/b)`; take the minimum k over floor and ceil (a/b = 1.05 gives 5.9, should be 4.0; thickness underestimated by up to ~20 %). **Done:** aspect factor takes the minimum k over floor and ceil (regression at a/b = 1.05).
- [x] 8A4 **[verified]** `plate_buckling_reduction_ec3`: knee limit coded `0.5 + 0.085/(1+psi)`; EN 1993-1-5 4.4 is `0.5 + sqrt(0.085 - 0.055 psi)` (0.673 at psi = 1, which the doc comment already says). Division by zero at psi = -1. Update the tests that encode 0.5425 / 0.585. **Done:** knee limit is 0.5 + sqrt(0.085 - 0.055 psi) (0.673 at psi = 1), finite at psi = -1; tests updated.
- [x] 8A5 **[verified]** `high_strength_factor`: AH32/AH36/AH40 = 0.91/0.78/0.72 look shifted one grade (IACS k is 0.78/0.72/0.68 — verify); fix the test near `lib.rs:1276`. **Done:** k is 1.0 / 0.78 / 0.72 / 0.68 for mild / 32 / 36 / 40 by minimum yield (IACS UR S4); tests and book updated.
- [ ] 8A6 `hull_girder_strength` allowable `17.5 L^2 B Cb` is ~5e6 t.m for the 140 m test ship against a ~6e5 kN.m wave moment: a unit/scale error that lets the check never fail. Re-derive from the intended rule.
- [ ] 8A7 `probabilistic.rs`: `s_final_factor` caps the product, not GZmax/Range separately; `v_factor` below H-d = 7.8 m; `s_mom_factor` GZmax cap; `survival_craft_moment` returns kN.m but doc and consumer say t.m (verify all against SOLAS 7-2 / 7-1.3); `r_factor` uses clamped `jn` where `p_factor` uses raw J for zones longer than Jm.
- [x] 8A8 IMO 2008 check omits the 30-40 deg area >= 0.030 m.rad and GZ >= 0.20 m at >= 30 deg criteria and the flooding-angle limit on the 40 deg area, so `stability --strict` can exit 0 on a failing hull. Add them to the library and the printed table. **Done:** all six IS Code criteria incl. 30-40 deg area and GZ at 30 deg, plus a downflooding angle (imo_2008_general_criteria_with_flooding, --flooding-deg, loading.flooding_angle_deg); the area integral is now exact for the piecewise-linear curve.
- [ ] 8A9 Unit traps: `scantling_requirement` takes kN.m while `hull_girder_strength` returns t.m; give the quantities explicit names or types.
- [x] 8A10 Prismatic stability screen: the max-GZ-angle criterion always passes (GZ is monotonic); `cb = 0` accepted; no draft-vs-depth check; `--to-deg 90` gives GZ ~1e32. Validate inputs and cap the scan. **Done:** prismatic screen rejects cb/cwp <= 0, a draft at or above hull.depth_m, and --to-deg above 80; a monotonic-GZ note is printed. 8B4 also covered.
- [ ] 8A11 Damage stability: flooding beyond the reserve buoyancy still "passes" (no deck immersion); `trim_equilibrium` clamps silently and `lcb_m` reports the requested not the achieved LCG; `tank_stage_compartment` drops x/y position (centroid `(0, 0, z)`) and applies free-surface moment at 0 % and 100 % fill; `damage_stages_*` collapse every error to `UnsolvedDamage`.
- [ ] 8A12 `wave_bending_moment`: verify the hogging expression (`0.19 Cw L^2 B Cb` vs the scaled sagging form), the use of LOA as rule length, and the `None` above 300 m.
- [x] 8A13 **[verified]** `space-structural` Whipple standoff was 10x too large (`d/10` m for d in mm). Edited (see note above); finish with tests run. **Done:** standoff is d/100 m; tests, book, README and gen-crate-docs updated.
- [x] 8A14 `blocks` `padeye_check` / `pin_bending_check`: verdicts use `if u > 1.0 { safe = false }`, so NaN inputs report `safe = true`; negative `out_of_plane_deg` / `root_arm_mm` reduce utilisation; `cog_uncertainty_envelope` load share can exceed 1.0. Use `!(u <= 1.0)` and validate inputs. **Done:** NaN, infinite and non-physical inputs report an unsafe, NaN-utilisation check (negative root arm no longer helps; the sign of the out-of-plane angle is ignored); the envelope share is capped at 1.0.
- [x] 8A15 **[verified]** `robotic-assembly` `yaw_diff` infinite loop on huge/inf yaw. Edited (see note above). **Done:** yaw_diff wraps with rem_euclid (regression test for huge/inf yaw).
- [ ] 8A16 `digital-twin` persistence: `from_json_value` resets `SupportCondition` to `Floating` and drops `quality_records` / `sensor_data`; `to_json` iterates a `HashSet` (non-deterministic order); restored `mass_properties` and completed ids are not validated against the project (completed ids not in the project push `connected_fraction` above 1).
- [ ] 8A17 `structural`: `ConstructionLoad::Gravity` is a no-op (self-weight is always applied when density > 0); the 2-point tip check uses the AABB (a diagonal pick with the CoG off the line passes); 3+ point sling shares are inverse-distance, not statics; no DAF; the logic is duplicated in `tpt-yard-blocks`; `analyze_at_phase` passes on a negative allowable; wind load has no drag/height factor; no compression-member buckling in the phase check; the structural `launch_analysis` returns a hard-coded `tipping_margin: 1.0`.
- [ ] 8A18 `habitat`: structure mass uses tube circumference squared (~10x low for the torus); `ONeillCylinder.length_m` unused; cabin-pressure hoop load ignored.
- [ ] 8A19 `drydock` / `launch`: BM coefficient `Cwp` should be ~`Cwp^2` (Murray-type; verify) in drydock and launch; the flooding sequence floats the vessel at draft but keel-block reactions assume 1 m of blocks; `BallastTank.x_from_midship_m` unused and the reaction centroid ignores the ballast moment; `water_volume_m3` ignores displaced volume; static `slipway_launch` way-end velocity 8.7 m/s vs dynamic < 3 m/s (slamming pressure overstated ~9x) and the static and dynamic buoyancy models disagree; `launch_stability` with an empty weight model returns `stable = true`.
- [ ] 8A20 `hull` optimizer: longitudinal seam length `nx*(nz-1)*LOA` is wrong and its test restates the formula; transverse butt uses beam not girth; `block_division` ignores a NaN / negative crane capacity (can emit ~140 000 blocks per tier); `n_x as u32` saturates.
- [ ] 8A21 `welding`: `residual_stress` is unbounded as 2b approaches the panel width (about -4.7e5 MPa, no warning); `distortion_of_sequence` multiplies full-joint shrinkage by pass count; `distortion()` validates `total_heat_input` but uses `heat_input_kj_mm`; zero plate thickness / panel width gives inf with `Ok`; `thermal_cycle` window doubling leaves 400 samples (t8/5 resolution collapses for heavy heat input); `WeldProcedure::from_json` swallows wrong-typed fields; `advise_preheat` maps NaN thickness to the 200 C band.
- [ ] 8A22 `distortion`: the doc example mixes mm and m (a "4 mm" bump is read as 4000 mm and Rejected); heat sizing asks ~58 kJ/mm for 3 mm excess (practice is 1-3 kJ/mm); out-of-plane bumps use transverse shrinkage; a reject is pushed per vertex.
- [ ] 8A23 `quality`: `AcceptanceCriteria` (`max_indication_mm`, `coverage_pct`) is never read; `defect_tracking` never compares sizes (a crack can be `AcceptAsIs`); the method fallback silently turns hydro/pressure tests into UT/visual with the coverage figure unchanged.
- [ ] 8A24 `logistics`: `dwell == buffer_days` always (arrive = need - buffer), so the dwell and staging-overlap checks cannot detect early arrivals; duplicate activity ids silently skipped; no buffer / lead-time / footprint validation; `order_by_day` ignores weekends.
- [ ] 8A25 `scheduling`: peak resource usage sampled at integer hours only; `makespan.ceil() as i64` hangs / OOMs on huge or NaN makespans; `MinimizeCost` equals `MinimizeCraneUsage`; `MinimizeDrydockTime` is plain CPM; O(n^3) on large networks; NaN / negative durations and releases accepted; `DeliveryGate` input errors reported as `UnknownGateActivity`.
- [ ] 8A26 `propellant`: no vent-capacity model despite the doc; no boil-off accrued during fill; `zero_boil_off_achieved` is true for a passive 0.1 %/day leak; NaN volume / ullage passes validation; `boil_off_report` divides by zero for a zero load.
- [ ] 8A27 `orbital-assembly`: duplicate component ids reported as `DependencyCycle`; step parameters hard-coded (90 deg rotate, 75 N.m, 400 N) and always the first robot; no-robot plans make every `simulate_step` fail; the collision sweep is a fixed -Z 10 m approach with spheres only (no robot reach or arm-arm check despite the doc).
- [ ] 8A28 Smaller items: `outfitting` hull envelope is the mesh AABB, route-system indices unvalidated, only the first clash per pair reported; `blocks` spreader check is Euler-only (non-conservative for lambda 60-120); `weight` silently drops an invalid `installed_by` and does not reject non-finite CoG / margin in `add_item`; core `json.rs` accepts unescaped control characters, duplicate keys are first-wins, `-0.0` serialises as `0`, `as_u64` saturates at 2^64.
- [ ] 8A29 `VesselProject::validate`: no dependency-cycle check, no vessel-parameter / resource-capacity / `Partial.completion` range checks, ids written as `f64` (corrupt above 2^53), `phase.weight_state` and `structural_state` never updated by the twin (dead state), resource kind serialised via `{:?}`.

### 8B. Crashes, hangs and robustness

- [ ] 8B1 Remove debug output from library code: `structural/fem.rs` (edited, see note), and make zero-free-load PCG return an error for a mechanism instead of `Ok`. (The prints at `frame.rs:592` and `shell_fem.rs:604` are inside tests; tidy anyway.)
- [ ] 8B2 **[verified]** CLI `export --json` splits the result on the first space (`main.rs:1421`): a path with a space gives truncated, invalid JSON; Windows backslashes are unescaped. Build the JSON properly.
- [ ] 8B3 **[verified]** CLI `html-report --gltf-viewer`: the template is filled with `.replace` but written with doubled braces (`{{ OrbitControls }}`, `{{ antialias: true }}` at `main.rs:286,289`), a JS syntax error that also kills the fallback. Test the emitted HTML; escape vessel / phase / group names in the report HTML.
- [x] 8B4 CLI `stability --svg` can hang (`stability.rs:497`, axis loop `while m <= y_max { m += 1.0 }` with y_max ~1e32); add a finite check and a point cap. **Done:** non-finite or huge GZ is an error and the SVG gridline count is bounded.
- [ ] 8B5 CLI `new sea > project.json` prints a banner to stdout before the JSON (`template` correctly uses stderr); add `--out`.
- [ ] 8B6 CLI `risk --weather`: vessel mass (4 000 000 kg), way length, friction and breadth are hard-coded, an unknown `--launch-method` silently becomes drydock, `--max-sea-state` is unchecked (0-9), the gate attaches to the last `--gate` activity, `weather_gate_activity` is dead, JSON mode sends the weather result only to stderr.
- [ ] 8B7 CLI argument parsing: the positional path must come first (`schedule --limit crane=5 p.json` fails); `.filter(|a| *a != path)` drops later args equal to the path; `validate` / `plan` / `report` ignore extra args and unknown flags; `--help` anywhere (even as a flag value) short-circuits; `new-hull -draft` reports "unknown hull". Make flags position-independent and reject unknown ones.
- [ ] 8B8 CLI numeric options: `--samples 0` runs as 1, `--uncertainty` is silently clamped to [0, 10], negative samples reported as "needs a number", `--gate 5=-3` reported as `UnknownGateActivity`, `--limit` accepts NaN / inf / negatives locally and duplicate kinds overwrite.
- [ ] 8B9 CLI exit codes and I/O: every error exits 1 (usage, I/O and `--strict` failure alike) and command usage is printed after every runtime error (including file-not-found); `println!` panics on a closed pipe; `help.rs` says "every command takes --json" but `validate`, `html-report`, `pdf-report` ignore it; `export` / `html-report` / `pdf-report` / `--svg` / `--csv` overwrite silently and do not create parent directories; `--offsets` is relative to the CWD while `offsets_csv` is relative to the case file.
- [ ] 8B10 CLI `import-hull`: beam is `2*max|y|` (a hull spanning y = 0..B gets double the beam); superstructure / skegs set depth and baseline; no unit sanity check (a mm mesh becomes a 60 km hull); an explicit `--name` with a quote or backslash breaks the hand-built JSON; very slow at 201 stations x 401 levels.
- [ ] 8B11 CLI `plan` / `validate` / reports: lift points are built around `geometry.centre` while mass properties use `cog` (and `export` places boxes at `cog`); `depth_bands as u32` wraps; `validate` on a manifest never checks the workshop section yet prints "well-formed"; the `html-report` FEM loop iterates `1..=max_phase` with an unbounded `erected_at` and hard-codes a 355 MPa allowable; the PDF report does not wrap long lines and replaces non-ASCII with spaces.
- [ ] 8B12 Monte Carlo: uncertainty > 1 gives negative triangular samples (weight totals <= 0, negative durations), a NaN `uncertainty_frac` passes the clamp, `n_samples` / `bays` are unbounded (OOM through py / wasm). Reject or clamp with typed errors.
- [ ] 8B13 NaN validators: add one `finite_positive` helper per crate and sweep the `<= 0.0` guards (drydock, welding, propellant, weight, outfitting, hull, launch).
- [ ] 8B14 Truss assembly allocates a dense `dof^2` matrix before CSR conversion (~7 GB at 10k nodes); assemble triplets directly.
- [ ] 8B15 Offsets CSV reader: a UTF-8 BOM or a typo in the first numeric row is silently skipped as a header; decimal-comma / `;` files give a confusing column-count error. `half_breadth_at` and `area_to` disagree above the table top.
- [ ] 8B16 `hull_girder_strength` weight splitting: `x_m` outside [0, L] gives negative / non-conserving weights; node loads are read as interval loads (ds/2 shift).
- [ ] 8B17 WASM: the twin is demo data (equal weight per activity, CoG at `seq*10`, the structural check can never fail); `advance_next` always retries the first incomplete activity and stalls on a non-topological order (`false` conflates unsound and not-ready); the JS example calls `advance_phase(0)` (ids start at 1, `u64` needs a BigInt); the doc says `Float64Array` but the code returns `Vec<f32>`; weight-report JSON built with `format!` is invalid on NaN / inf. Either wire real data or label it as a demo.
- [ ] 8B18 Python: `from_project_json` builds a twin with an empty weight model and `Floating` support (`weight_report()` is all zeros); `to_json` output cannot be re-loaded by `from_project_json`; returned dicts have random key order; `HullForm::new` / `Scheduler::new` accept NaN / negative; `samples` is unbounded. Expose weight items and support conditions. The facade should also re-export `tpt_yard_hydrostatics`.
- [ ] 8B19 Dead configuration: `SiteConditions.max_sea_state`, `Slipway.ways`, `BallastTank.x_from_midship_m`, `QualityManagement.acceptance_criteria` are never read: wire them up or delete them.
- [ ] 8B20 Export: glTF `name` interpolated unescaped (a quote breaks the JSON); min / max in f64 but the buffer is f32; NaN / inf or empty geometry produce invalid glTF; indices accessor `min:[0]` hard-coded; no z-up to y-up conversion; IFC marks every face set closed (`.T.`); `f+1` can overflow at `u32::MAX`; non-ASCII names not STEP-encoded.
- [ ] 8B21 Repo hygiene: `.kilo/` scratch tree still present; `ci.yml` examples line has stray whitespace; the README CLI row lists 8 of 12 commands (`stability`, `new-hull`, `import-hull`, `export` missing); `todo.md:29` says the board is deferred while `docs/roadmap.md` and the README describe it as live; `getting-started.md` says "from `cargo install`" but uses `git clone`; check that the `cargo add tpt-yard` instructions work (is anything published on crates.io yet?).

### 8C. Adoption and onboarding

- [ ] 8C1 **[verified]** `CONTRIBUTING.md` (DCO sign-off, no CLA) and `CODE_OF_CONDUCT.md` do not exist although Phase 0 and 7G tick them. Create them; add `good first issue` / `help wanted` labels and seed a few issues.
- [ ] 8C2 README "Try it without Rust" box at the top: prebuilt release binaries, a `docker run` one-liner (the Dockerfile exists but is undocumented), the Pages demo and book links. Publish a GHCR image from CI; set up PyPI publishing for the wheel; document `cargo install tpt-yard-cli` once published.
- [ ] 8C3 "Is this for me?" page: scope (screening-grade, no CFD), when to use it versus class software, comparison with Maxsurf / NAPA / FreeShip / Orca3D, and a units and conventions glossary (SI, t vs kg, kN.m, z-up, degrees). Add an FAQ and a "limitations and validity" chapter.
- [ ] 8C4 CLI discoverability: `tpt-yard templates` (list and describe; `help new` should list the five templates), `completions <bash|zsh|fish|powershell>`, `schema <name>` (emit `schemas/`), `doctor`, `explain <criterion>` (cites the IMO / SOLAS clause), `--format text|json|csv|md`, `-o/--output`, stdin via `-`, `--seed`, `--quiet` / `--verbose`, `--no-color`, `--dry-run`, a defaults/config file (density, sea-state limits, yield allowable).
- [ ] 8C5 CLI surface for the hydrostatics already in the library: trim, damage / flooding stages, probabilistic index, scantlings + rule tables, hull-girder; `stability --trim --heel-step --density --criteria --format csv`. `risk --json` should include criticality, weather and limits; `plan --json` should use a real serializer.
- [ ] 8C6 CLI quality: split the 1,800-line `main.rs` (consider `clap`), add `assert_cmd` integration tests per command (none exist today), keep the typo-suggestion behaviour (`suggest("")` currently matches everything).
- [ ] 8C7 Templates and examples: README for `templates/` and `examples/hull-stability-screen`; new project templates (bulk carrier / tanker, tug or ferry project, offshore jacket, CubeSat / station module, yacht or naval combatant); run each example in CI against its golden output (today only exit 0 is checked); a worked tutorial for the tug and ferry hulls.
- [ ] 8C8 Book: CLI reference chapter, cookbook / how-to ("check IMO stability of my hull in 5 steps", "import a Maxsurf / Rhino export", "run a schedule risk study"), architecture / crate-map diagram, JSON-schema versioning and deprecation policy.
- [ ] 8C9 Interchange: CSV / XLSX weight-list import; schedule export to CSV / MS Project XML / Gantt SVG; CSV export of schedule and weights; DXF / IGES / STEP-section hull import; FreeShip `.fbm` import; Python `.pyi` stubs, `py.typed` and a Jupyter notebook gallery; an npm package and a live-demo link for the WASM build.

### 8D. Innovation candidates (choose with the maintainer)

- [ ] 8D1 Hosted browser playground: WASM build of the CLI with drag-and-drop offsets CSV / OBJ and a live GZ curve + IMO checklist, no install.
- [ ] 8D2 Report-as-code / evidence pack: one command emits a hash-stamped calculation package (inputs, tool version, git SHA, golden-case results) that a reviewer can reproduce.
- [ ] 8D3 Parametric sweeps: `tpt-yard sweep` over KG, draft and block split with Pareto output.
- [ ] 8D4 Uncertainty propagation across domains: carry weight / CoG uncertainty into stability margins and launch-window schedule risk (the Monte Carlo pieces already exist).
- [ ] 8D5 GitHub Action `tpt-yard-check` that fails a PR when a hull or project change breaks `--strict`, with SARIF / JUnit output.
- [ ] 8D6 Live twin connectors (MQTT / OPC-UA into `ingest_telemetry`) and an as-built scan-deviation heatmap in the viewer.
- [ ] 8D7 MCP server / LLM-friendly surface over the CLI for what-if studies.
- [ ] 8D8 Domain gaps: calendars / lags / SS-FF-SF links / earned value / baseline re-forecast in scheduling; SWBS hierarchy, inertias and inclining-test reconciliation in weight; crane radius-capacity charts and dock slot scheduling in facility; side launch, shiplift and fore-poppet design; orbital mechanics (eclipse, rendezvous, debris), power / thermal budgets and radiation / EVA in space; NCR repair-reinspect workflow and welder traceability in quality; hydrogen-cracking / HAZ hardness in welding; stiffened-panel and fatigue checks in structural.

### 8E. Suggested order

1. 8A1-8A5, 8A8, 8A13-8A15 with failing-first tests (hand values in the PR text).
2. 8A6, 8A7, 8A16-8A19 after a source check for each.
3. 8B hygiene and CLI fixes together with 8C1.
4. 8C2-8C4 (front door), then pick from 8D.

Verification for every item: `cargo fmt --all && cargo clippy --workspace --all-targets --locked && cargo test --workspace --locked`, `python scripts/test-snippets.py`, run all examples, and for CLI fixes run from a path containing spaces on Windows.
