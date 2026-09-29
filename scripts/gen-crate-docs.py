"""Generate per-crate README.md, CHANGELOG.md and Cargo.toml metadata
(keywords, categories, readme) for every publishable crate in the
tpt-shipyard workspace, plus README/CHANGELOG for the example and bench
packages.

Run from the workspace root:  python scripts/gen-crate-docs.py
"""
import os
import re
from string import Template

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TODAY = "2026-09-28"

VALID_CATEGORIES = {
    "science", "science::robotics", "simulation", "mathematics", "algorithms",
    "data-structures", "development-tools", "aerospace", "aerospace::simulation",
    "web-programming", "web-programming::wasm", "graphics",
    "rendering::data-formats", "parser-implementations", "encoding",
}

# ---------------------------------------------------------------------------
# Per-crate content. Paths are relative to the workspace root.
# ---------------------------------------------------------------------------
CRATES = {
    "crates/core/tpt-yard-core": {
        "tagline": "Fundamental shipyard domain types for the TPT Shipyard construction engine.",
        "overview": [
            "`tpt-yard-core` defines *what* is being built: vessel projects, build phases, "
            "assembly activities, and the yard resources they consume. It is the shared "
            "vocabulary of the tpt-shipyard construction engine — every other crate "
            "consumes these types, so the dependency graph stays acyclic and the data "
            "model stays coherent from steel cutting to orbital bolt-up.",
            "The crate also ships the primitives everything else builds on: a small 3D "
            "geometry kernel (`Vector3`, triangle meshes, mass properties), newtype "
            "identifiers, engineering material presets (AH36 to Inconel 718), and a "
            "dependency-free JSON implementation so project files round-trip without "
            "pulling in external crates.",
        ],
        "features": [
            "`VesselProject` — the complete build plan: vessel type, construction method, ordered build phases",
            "Sea and space vessel taxonomies (`SeaVesselType`, `SpaceVesselType`, hybrid) exactly as the master spec defines them",
            "`BuildPhase` / `AssemblyActivity` / `ActivityType` — the activity model with yard `Resource`s (cranes, workshops, robots, crews)",
            "Geometry kernel: `Vector3`, `Geometry3D` triangle meshes (box, cylinder, merge, transform, bounding box, centroid), `MassProperties`",
            "`Material` presets: AH36, S235, AA5083, AISI 316L, Inconel 718, Ti-6Al-4V with full thermal/mechanical property sets",
            "Newtype ids (`ProjectId`, `PhaseId`, `BlockId`, ...) via the exported `define_id!` macro",
            "Embedded JSON value/parser/writer — full grammar, surrogate pairs, zero dependencies",
        ],
        "usage": r"""use tpt_yard_core::*;

let mut phase = BuildPhase::new(PhaseId(1), "Erection", 10.0);
phase.activities.push(AssemblyActivity::new(
    ActivityId(1),
    "Erect block 212",
    ActivityType::JoinBlock,
    8.0,
));
let project = VesselProject::new(
    ProjectId(1),
    "Container ship 1400 TEU",
    VesselType::Sea(SeaVesselType::ContainerShip { teu_capacity: 1400 }),
    ConstructionMethod::SeaDrydock,
    vec![phase],
)
.unwrap();

// Round-trips through the dependency-free JSON module.
let json = project.to_json().to_string_pretty();
let same = VesselProject::from_json_str(&json).unwrap();
assert_eq!(same, project);""",
        "model": [
            "The project deliberately carries **no** digital-twin back-reference — the twin (in `tpt-yard-digital-twin`) owns the project. See RFC 0001 for the ownership decision.",
            "`VesselProject::activity_graph()` validates the cross-phase dependency network with the `tpt-yard-assembly` algorithms.",
            "Numbers follow shipyard convention: metres, kilograms, hours, days; vessel dimensions use LOA / breadth / depth nomenclature.",
        ],
        "verification": [
            "JSON round-trip tests over every enum variant (compact and pretty writers)",
            "Geometry tests: box/cylinder bounds, centroids, rotation length preservation",
            "Rosenthal-relevant material diffusivity and shear-modulus consistency checks",
        ],
        "keywords": ["shipyard", "shipbuilding", "vessel", "digital-twin", "simulation"],
        "categories": ["science", "simulation", "mathematics"],
    },
    "crates/core/tpt-yard-assembly": {
        "tagline": "Shared assembly-activity primitives: dependency graphs and activity status tracking.",
        "overview": [
            "Every construction plan is a directed acyclic graph of activities: cut "
            "steel before welding, weld before erection. `tpt-yard-assembly` is that "
            "graph — a small, deterministic, dependency-free DAG with the algorithms "
            "planners actually ask for: topological ordering, cycle detection, "
            "readiness checks, and a critical-path-method forward pass.",
            "It sits at the very bottom of the `tpt-yard-*` dependency stack, so the "
            "digital twin, the scheduler, and the logistics planner all share one "
            "well-tested implementation instead of three divergent ones.",
        ],
        "features": [
            "`ActivityGraph` — a DAG of activities (id, name, duration, dependencies, status)",
            "Kahn topological sort with deterministic (BTreeSet-ordered) output",
            "Cycle detection via DFS colouring, returning the offending closed path",
            "Readiness queries (`is_ready`) against live activity statuses",
            "CPM forward pass: earliest finish times and project makespan",
            "Status lifecycle: `Pending -> InProgress -> Completed`, plus `Blocked` and `Cancelled`",
        ],
        "usage": r"""use tpt_yard_assembly::{ActivityGraph, ActivityId};

let mut g = ActivityGraph::new();
g.add_activity(ActivityId(1), "Cut steel", 8.0, &[])?;
g.add_activity(ActivityId(2), "Form frames", 12.0, &[])?;
g.add_activity(ActivityId(3), "Weld panel", 16.0, &[ActivityId(1), ActivityId(2)])?;
g.add_activity(ActivityId(4), "Outfit panel", 10.0, &[ActivityId(3)])?;

assert_eq!(g.makespan_hours()?, 38.0); // max(8, 12) + 16 + 10""",
        "model": [
            "Dependencies must reference already-existing activities, so a well-formed graph is guaranteed while it is being built.",
            "All iteration is id-ordered: identical graphs produce identical orderings on every platform.",
            "The forward pass assumes every activity starts as early as its dependencies allow; backward passes and float live in `tpt-yard-scheduling`.",
        ],
        "verification": [
            "Topological order across a diamond dependency network",
            "Cycle detection including a manually closed loop",
            "CPM forward pass against hand-computed finish times",
        ],
        "keywords": ["dependency-graph", "dag", "topological-sort", "cpm", "scheduling"],
        "categories": ["algorithms", "data-structures", "science"],
    },
    "crates/core/tpt-yard-weight": {
        "tagline": "Weight and centre-of-gravity management for vessels under construction.",
        "overview": [
            "Shipyards live and die by two numbers: how much does it weigh now, and "
            "where is the centre of gravity? `tpt-yard-weight` is the single source of "
            "truth for both. Every measurable mass is a `WeightItem` with a lifecycle "
            "(`Design -> Ordered -> Received -> Installed`, with `Replaced` kept for "
            "history), a growth margin, and — crucially — the assembly activity that "
            "installs it.",
            "Because items are wired to activities, the model can answer the master-"
            "plan question (what will it weigh?) and the as-built question (what is in "
            "the ship *right now*?) from the same data, and the digital twin can flip "
            "items to installed automatically as erection proceeds.",
        ],
        "features": [
            "`WeightModel` — all items plus the contractual design weight and design CoG",
            "Best-estimate vs installed vs design weight, with deviation in kg and %",
            "Best-estimate and installed-only centres of gravity (weighted means)",
            "Growth margins per item, aggregated for the weight report",
            "Group-by reports (by system or by zone), outstanding item counts",
            "`installed_by: ActivityId` wiring — the only path from predicted to as-built mass",
        ],
        "usage": r"""use tpt_yard_core::{ItemId, Vector3};
use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

let mut model = WeightModel::new(190_000.0, Vector3::new(70.0, 0.0, 7.0));
model.add_item(WeightItem {
    id: ItemId(1),
    name: "Block 211".into(),
    group: "hull".into(),
    weight_kg: 15_000.0,
    cog: Vector3::new(10.0, 0.0, 6.0),
    status: ItemStatus::Installed,
    margin_pct: 2.0,
    installed_by: None,
});

assert_eq!(model.installed_weight(), 15_000.0);
assert_eq!(model.weight_deviation(), 0.0); // vs design on creation""",
        "model": [
            "`total_weight()` counts every non-replaced item: installed items at as-built mass, the rest at predicted mass (the lightship best estimate).",
            "`installed_weight()` counts only what is physically aboard — the launch officer's number.",
            "Replaced items are kept for audit but never counted.",
        ],
        "verification": [
            "`test_block_weight_sum`: ten blocks sum exactly to the vessel total",
            "CoG equals the closed-form weighted mean; replaced items never count",
            "Deviation, margin aggregation and report grouping unit-tested",
        ],
        "keywords": ["weight", "centre-of-gravity", "mass-properties", "shipbuilding"],
        "categories": ["science", "simulation"],
    },
    "crates/core/tpt-yard-digital-twin": {
        "tagline": "Construction state tracking and simulation: the shipyard digital twin.",
        "overview": [
            "The digital twin mirrors a build in progress: which activities are "
            "complete, what mass is physically installed, where the centre of gravity "
            "sits — and, the question no in-service analysis tool asks, whether the "
            "*partially built* structure can still stand on its supports at every "
            "stage.",
            "Everything flows through `advance_phase`: the twin validates dependencies, "
            "dry-runs the support check with the activity's mass added, and only then "
            "commits — installing wired weight items, recomputing mass properties, and "
            "advancing the phase pointer. An erection step that would tip the hull is "
            "refused outright.",
        ],
        "features": [
            "`DigitalTwin` — owns the `VesselProject`, the weight model, and the assembly state (RFC 0001)",
            "`advance_phase` — dependency-validated, support-checked activity completion; failures leave the twin untouched",
            "`SupportCondition` — keel blocks, slipway ways, floating, or orbital (microgravity)",
            "`structural_check_at_phase` — the same tipping analysis projected at any past or future phase end",
            "`centre_of_gravity_tracking` — the cumulative (weight, CoG) curve per phase for launch officers and orbital integrators",
            "Quality records and sensor readings attached to the build",
        ],
        "usage": r"""use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{ItemId, PhaseId, ProjectId, Vector3};
use tpt_yard_digital_twin::{DigitalTwin, SupportCondition};
use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};

let mut twin = DigitalTwin::with_weight_model(project, weight_model,
    SupportCondition::KeelBlocks {
        positions: vec![
            Vector3::new(-10.0, -4.0, 0.0), Vector3::new(-10.0, 4.0, 0.0),
            Vector3::new(130.0, -4.0, 0.0), Vector3::new(130.0, 4.0, 0.0),
        ],
    },
);

twin.advance_phase(&ActivityId(1))?; // Err(UnsoundStructure) if it would tip
let report = twin.centre_of_gravity_tracking()?;""",
        "model": [
            "Support statics treat the partial structure as a rigid body: longitudinal tipping against the extreme keel supports (negative reaction = refuse), transverse CoG against the block half-track.",
            "The dry run includes the completing activity's wired mass *before* committing, so an unsound advance never mutates state.",
            "Phase 2's `tpt-yard-structural` adds FEM-level checks on top; the twin's rigid-body gate stays valid in every domain.",
        ],
        "verification": [
            "Milestone test `ten_phase_tracking`: 10 phases, weight and CoG match the closed form at every step, reactions positive throughout",
            "Tipping refusal test: a CoG marched outboard of the cribbing is rejected and rolled back",
            "Dependency, double-completion and unknown-phase error paths covered",
        ],
        "keywords": ["digital-twin", "construction", "simulation", "monitoring", "shipbuilding"],
        "categories": ["science", "simulation"],
    },
    "crates/core/tpt-yard-wasm": {
        "tagline": "WebAssembly bindings for interactive shipyard construction dashboards.",
        "overview": [
            "`tpt-yard-wasm` compiles the construction engine to WebAssembly so a "
            "browser can drive a build: advance erection phases, pull geometry for "
            "WebGL buffers, and read weight/CoG and structural-check reports as JSON "
            "— all computed by the same Rust engine the server-side tools use.",
            "The façades hold no rendering state: the browser draws, the engine "
            "computes. The workspace ships a working reference dashboard in `www/` "
            "that erects a 12-block ship on a plain canvas with zero runtime "
            "dependencies.",
        ],
        "features": [
            "`WasmDigitalTwin` — load a `VesselProject` from JSON, advance phases, dependency-gated",
            "`get_geometry` / `get_geometry_indices` — flat triangle-soup vertex and index buffers, upload-ready",
            "`get_weight_report` / `structural_check` — JSON reports (installed vs design weight, CoG, keel reactions, margin)",
            "`WasmOrbitalAssembly` — step through a truss build (`simulate_next_step`), robot pose and installed components for rendering",
            "Compiles and unit-tests on native targets (wasm-bindgen degrades gracefully)",
        ],
        "usage": r"""import init, { WasmDigitalTwin } from "./pkg/tpt_yard_wasm.js";

await init();
const twin = new WasmDigitalTwin(projectJson);

twin.advance_next();                  // erect the next block
const verts = twin.get_geometry();    // Float32Array triples for WebGL
const report = JSON.parse(twin.get_weight_report());
console.log(report.installed_kg, report.cog);""",
        "model": [
            "One weight item per activity is synthesised from the phase design weights, laid out along +X; keel-block supports span the plan automatically.",
            "Geometry is a box per completed erection step in plan order — dashboards that carry real block meshes swap the buffer source, the report path is unchanged.",
        ],
        "verification": [
            "Native tests: JSON round-trip, dependency-gated advance, geometry buffer sizes",
            "Orbital façade: full 18-step sequence simulated step by step",
            "Browser smoke test documented in the book (WASM chapter); reference dashboard in `www/`",
        ],
        "keywords": ["wasm", "webassembly", "digital-twin", "dashboard", "shipyard"],
        "categories": ["web-programming::wasm", "science", "simulation", "graphics"],
    },
    "crates/structural/tpt-yard-structural": {
        "tagline": "Structural analysis of incomplete structures at every build phase.",
        "overview": [
            "In-service analysis tools assume the vessel is complete. During "
            "construction it is not: blocks are missing, welds are partial, and the "
            "loads are entirely different — crane picks, temporary supports, "
            "launching ways. `tpt-yard-structural` analyses the structure *as it "
            "exists at a build phase*.",
            "Members carry the phase at which they become load-bearing; the solver "
            "assembles the sub-model erected so far and solves the construction load "
            "cases. A phase whose staging is a mechanism is refused outright rather "
            "than silently solved — a missing diagonal is a finding, not a NaN.",
        ],
        "features": [
            "`PartialStructure` — every member tagged with its erection phase",
            "`analyze_at_phase` — staged truss analysis under construction loads: gravity, wind, crane picks (with dynamic factor), hydrostatic, slamming, docking, robotic reactions",
            "`fem::TrussModel` — dependency-free 3D truss solver: direct stiffness, penalty BCs, dense elimination; swap-ready for the `tpt-fem` substrate",
            "`lifting_analysis` — crane-pick statics: sling loads by lever rule, angles, tip check, sling and crane utilization",
            "`launch_analysis` — way-pressure screening against the 500 kPa class limit",
            "Refuses mechanisms with `FemError::SingularSystem` instead of returning nonsense",
        ],
        "usage": r"""use tpt_yard_core::{PhaseId, Vector3};
use tpt_yard_structural::{
    ConstructionLoad, ConstructionStructuralSolver, PartialElement,
    PartialStructure, Support,
};

let structure = PartialStructure {
    nodes: vec![
        Vector3::new(0.0, 0.0, 4.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(3.0, 0.0, 4.0),
    ],
    elements: vec![
        PartialElement { nodes: [1, 0], area_m2: 0.01, youngs_modulus_gpa: 210.0,
            density_kg_m3: 7850.0, erected_at: PhaseId(1) },
        // ... more members, each with its erection phase
    ],
    supports: vec![Support { node: 1, fix_x: true, fix_y: true, fix_z: true }],
};
let solver = ConstructionStructuralSolver::new(structure, 355.0);
let result = solver.analyze_at_phase(PhaseId(2), &[ConstructionLoad::Gravity])?;""",
        "model": [
            "Direct stiffness method on 2-node bar elements; penalty stiffness 1e13 gives ~1e-4 relative compliance (documented in the test tolerances).",
            "Lifting statics: exact lever rule for two-point picks, inverse-distance sharing beyond; a CoG outside the lift points is a failed pick.",
            "The substrate `tpt-fem` is not published yet; the staging concepts are solver-independent so the swap is mechanical.",
        ],
        "verification": [
            "Single axial bar vs `delta = FL/EA`; two-bar truss vs `N = F/(2 sin theta)`; vertical bar self weight vs `delta = WL/(2EA)`",
            "Staged erection test: the open frame is a mechanism at phase 1, the closed triangle solves at phase 2",
            "Crane overload and slipway over-pressure paths flagged as `passed: false`",
        ],
        "keywords": ["fem", "truss", "structural-analysis", "construction", "lifting"],
        "categories": ["science", "simulation", "mathematics"],
    },
    "crates/structural/tpt-yard-welding": {
        "tagline": "Welding simulation and distortion control for ship construction.",
        "overview": [
            "What does a weld do to the surrounding plate? `tpt-yard-welding` answers "
            "with the classical analytical chain: a Rosenthal moving point source for "
            "the thermal cycle, a yield-limited residual-stress field after cooling, "
            "calibrated contraction estimates for shrinkage and angular distortion, "
            "and a sequence ranker that prefers backstep-style, spread-out pass "
            "orders.",
            "It is engineering-grade, not thermo-mechanical FEM: fast enough to "
            "evaluate every seam on the ship inside a digital-twin loop (microseconds "
            "per weld), verified against published ranges and a locked golden panel, "
            "and documented equation by equation in RFC 0004.",
        ],
        "features": [
            "Rosenthal quasi-stationary 3D thermal cycle: temperature history, peak temperature, and the t8/5 cooling time at any distance from the seam",
            "Residual stress: parabolic tension zone bounded by the T_mech isotherm, balanced by uniform compression",
            "Distortion: transverse and longitudinal shrinkage, angular distortion (single-side vs balanced grooves), bowing",
            "Seven processes with arc efficiencies (SMAW through EBW and laser)",
            "Multi-pass superposition and candidate-sequence optimisation",
            "WPS record loading from JSON (`test-data/welding-procedures/`)",
        ],
        "usage": r"""use tpt_yard_core::Material;
use tpt_yard_joints::{GrooveType, JointGeometry, JointKind};
use tpt_yard_welding::{WeldProcess, WeldProcedure, WeldingSimulation};

let procedure = WeldProcedure {
    process: WeldProcess::Saw,
    heat_input_kj_mm: 12.0,
    travel_speed_mm_s: 8.0,
    preheat_temp_c: 50.0,
    interpass_temp_c: 150.0,
    filler_metal: "S2Si2 / SA AB1 47".into(),
    sequence: vec![],
};
let joint = JointGeometry::new(JointKind::Butt)
    .with_thickness_mm(12.0)
    .with_groove(GrooveType::V)
    .with_groove_angle_deg(60.0)
    .with_root_gap_mm(3.0)
    .with_root_face_mm(2.0);

let sim = WeldingSimulation::new(procedure, Material::ah36(), joint);
let cycle = sim.thermal_cycle(8.0)?;      // temperature history at 8 mm
let distortion = sim.distortion()?;       // shrinkage + angular""",
        "model": [
            "Rosenthal 3D: `T - T0 = Q/(2 pi k R) * exp(-v (R + xi) / (2 alpha))`; the sample window covers at least a metre of travel because the 3D tail decays like 1/R.",
            "t8/5 is (near) distance-independent in 3D and scales with heat input — both properties are asserted in tests.",
            "Distortion constants are calibrated against published AH36 panel data and locked in the golden file; see RFC 0004 for the formulas.",
        ],
        "verification": [
            "Golden: `test-data/golden/sea/welding-distortion-panel.json` at ±5 % (±10 % for t8/5)",
            "Peak decays monotonically with distance; heavier heat input lengthens t8/5",
            "Balanced X grooves halve angular distortion; sequence ranker prefers spread, alternating passes",
        ],
        "keywords": ["welding", "rosenthal", "heat-input", "distortion", "haz"],
        "categories": ["science", "simulation"],
    },
    "crates/structural/tpt-yard-distortion": {
        "tagline": "Block distortion management: deviation maps and correction planning.",
        "overview": [
            "After a block is welded it never matches the drawing exactly. "
            "`tpt-yard-distortion` closes the quality loop: compare the laser-measured "
            "geometry against target, quantify the deviation field, and plan the "
            "correction — accept inside tolerance, heat-straighten the moderate "
            "deviations, reject the gross ones, and rework blocks that are "
            "systematically out of tolerance.",
            "The correction policy is explicit and threshold-tested, so the plan a "
            "planner sees is the plan the code produces: no silent acceptance beyond "
            "tolerance, no heat straightening prescribed where rework is the "
            "economical answer.",
        ],
        "features": [
            "`DistortionControl` — target vs measured geometry with a correction history",
            "`deviation_map` — per-vertex signed deviation (dominant axis, mm), max and RMS",
            "`correction_plan(tol)` — Accept / HeatStraighten / Reject / Rework decisions from explicit thresholds",
            "Systematic-distortion detection: > 20 % of vertices out of tolerance triggers rework instead of point-wise heating",
            "Topology mismatch (target vs measurement) is an explicit error, never silently ignored",
        ],
        "usage": r"""use tpt_yard_core::{Geometry3D, Vector3};
use tpt_yard_distortion::{CorrectionAction, DistortionControl};

let target = Geometry3D::from_box(2000.0, 500.0, 100.0);
let mut measured = target.clone();
measured.vertices[0] = measured.vertices[0] + Vector3::new(0.0, 0.0, 4.0);

let mut dc = DistortionControl::new(target, measured);
let plan = dc.correction_plan(3.0); // +/- 3 mm tolerance
// The 4 mm bump is heat-straightened (or rejected), never accepted.""",
        "model": [
            "Thresholds: within tolerance accepted; up to 5x tolerance heat-straightened (heat input scaled to the excess); beyond 5x rejected.",
            "Point-wise straightening is suppressed when more than a fifth of the vertices are out — the block needs systematic rework.",
            "Every planned action is appended to `corrections`, giving the block's as-built correction history for the handover record.",
        ],
        "verification": [
            "Boundary tests just inside and just outside the tolerance",
            "Gross-deviation rejection and systematic-rework trigger tests",
            "Mismatched-topology rejection with a typed error",
        ],
        "keywords": ["distortion", "heat-straightening", "deviation", "tolerance", "quality"],
        "categories": ["science", "simulation"],
    },
    "crates/structural/tpt-yard-joints": {
        "tagline": "Shared joint-geometry primitives for structural and welding analysis.",
        "overview": [
            "A `JointGeometry` describes the *shape* of the connection — joint kind, "
            "groove preparation, thickness, root gap, fillet legs, penetration "
            "credit. From those it derives the quantities downstream physics needs: "
            "groove cross-section for weld-metal heat budgets, effective throat for "
            "strength, and weld volume/mass for consumable planning.",
            "Both the welding and structural crates build on these primitives, so a "
            "60-degree single-V means exactly the same thing everywhere in the "
            "engine.",
        ],
        "features": [
            "Joint families: butt, fillet, lap, T-joint, corner, edge",
            "Groove preparations: square, V, double-V, bevel, K, J, U",
            "Groove cross-section area (trapezoidal profiles + root gap + penetration credit)",
            "Effective fillet throat (`leg / sqrt 2` + penetration) for strength checks",
            "Weld volume and filler mass for the whole seam",
            "Double-side accounting (`weld_area_mm2_total`) for X/K grooves and two-toe joints",
        ],
        "usage": r"""use tpt_yard_joints::{GrooveType, JointGeometry, JointKind};

// Single-V butt weld, 60 degrees included, 12 mm plate.
let butt = JointGeometry::new(JointKind::Butt)
    .with_thickness_mm(12.0)
    .with_groove(GrooveType::V)
    .with_groove_angle_deg(60.0)
    .with_root_gap_mm(3.0)
    .with_root_face_mm(2.0);

println!("weld metal: {:.1} kg", butt.weld_mass_kg(7850.0));""",
        "model": [
            "V-family areas are trapezoidal: `(t - face)^2 * tan(half_angle)` plus the gap filling the root face height.",
            "Deep penetration (SAW-class) extends the fused zone below the root; the added area is the penetration depth continuing through the gap channel.",
            "T-joints are fillet-welded when a leg is given and groove-welded (full penetration) otherwise.",
        ],
        "verification": [
            "Closed-form areas for V-groove, square-groove and fillet cases",
            "Double-side totals for X grooves and two-toe T-joints",
            "Volume/mass consistency at steel density",
        ],
        "keywords": ["welding", "joint-geometry", "groove", "fillet", "throat"],
        "categories": ["science"],
    },
    "crates/sea/tpt-yard-hull": {
        "tagline": "Hull construction and block management for sea shipyards.",
        "overview": [
            "`tpt-yard-hull` turns a hull form into an erection plan. Block division "
            "splits the hull under the yard's real constraints — every block liftable "
            "by the crane (with a rigging allowance), every block fitting the "
            "workshop — and the erection sequence builds the ship the way yards "
            "actually do: keel tier first, from midship outwards, each block landing "
            "against its nearest erected neighbour.",
            "The sequence feeds straight into the digital twin as activities, so the "
            "tipping gate (RFC 0001) guards every erection step and the weight/CoG "
            "curve grows with the ship.",
        ],
        "features": [
            "`HullConstruction::block_division` — crane- and workshop-constrained division into tiers and longitudinal bands",
            "Pre-design weight model from areal steel density (deterministic, auditable)",
            "`erection_sequence` — bottom-tier-first, midship-outward joins with seam typing (dock joint / butt seam)",
            "`HullBlock` lifecycle status: Design through Cutting, Welding, Outfitting, Ready, Erected",
            "Total steel weight and per-block load reporting",
        ],
        "usage": r"""use tpt_yard_core::Dimensions;
use tpt_yard_hull::{HullConstruction, HullGeometry};

let hull = HullConstruction::new(HullGeometry {
    loa_m: 140.0,
    boa_m: 22.0,
    depth_m: 12.0,
    areal_density_kg_m2: 180.0,
    depth_bands: 2,
});

let blocks = hull.block_division(40_000.0, Dimensions::new(24.0, 30.0, 14.0));
let joins = hull.erection_sequence(&blocks);
assert!(joins[0].z_band == 0); // keel tier first""",
        "model": [
            "Pre-design weight: areal density over `2*(B + tier height)` per metre — the classic estimate, replaced by itemised weights as the twin fills in.",
            "Crane limit includes a 10 % rigging allowance; the workshop bounds block length and height independently.",
            "Midship-outward keeps the growing centroid near midship, maximising keel-block reactions and confining shrinkage away from end joints (RFC 0002).",
        ],
        "verification": [
            "Golden: `container-ship-block-division.json` — 12 blocks of 23.33 m / 142.8 t, closed form",
            "Constraint tests: every block under the crane limit and inside the workshop; tighter cranes yield more blocks",
            "Order tests: tiers never interleave, midship distance non-decreasing, first join a dock joint",
        ],
        "keywords": ["hull", "block-division", "erection", "shipbuilding", "shipyard"],
        "categories": ["science", "simulation"],
    },
    "crates/sea/tpt-yard-blocks": {
        "tagline": "Shared block-lifting and handling primitives: lift points and sling loads.",
        "overview": [
            "Every heavy lift in the yard asks the same statics question: given the "
            "hook position and the block's centre of gravity, what does each sling "
            "leg carry? `tpt-yard-blocks` is the canonical answer, shared by the "
            "hull-erection planner, the structural lifting checks, and the "
            "block-lifting benchmark.",
            "Two-point picks use the exact lever rule; three or more points use an "
            "inverse-distance convention (multi-point statics are indeterminate "
            "without sling stiffnesses) — both documented in RFC 0002.",
        ],
        "features": [
            "`distribute_load_shares` — vertical load shares over lift points, summing to 1",
            "`sling_loads` — per-leg table: share, angle from horizontal, leg tension (`share / sin theta`)",
            "`sling_angles_ok` — the 30-degree practice-minimum angle check",
            "`cog_within_lifts` — the pick-time tipping check",
            "`LiftPoint` with certified working load limit for utilization math",
        ],
        "usage": r"""use tpt_yard_blocks::{cog_within_lifts, sling_loads};
use tpt_yard_core::Vector3;

let lifts = [Vector3::new(0.0, 0.0, 0.0), Vector3::new(12.0, 0.0, 0.0)];
let hook = Vector3::new(6.0, 0.0, 8.0);
let loads = sling_loads(800.0 * 9.81, Vector3::new(6.0, 0.0, 0.0), &lifts, hook);

for leg in &loads {
    println!("lift {}: {:.0} kN at {:.0} deg", leg.lift_point,
             leg.leg_tension_kn, leg.angle_from_horizontal_deg);
}
assert!(cog_within_lifts(Vector3::new(6.0, 0.0, 0.0), &lifts));""",
        "model": [
            "Two points: shares = opposite lever arms of the CoG offset (exact).",
            "N >= 3 points: inverse-distance weighting of the CoG offset — deterministic and conservative for planning.",
            "Leg tension divides the vertical share by `sin(angle)`; shallow legs (large tension) are exactly what the angle check exists to catch.",
        ],
        "verification": [
            "Centred CoG splits evenly; 3-of-12 m offset gives the exact 75/25 lever split",
            "45-degree legs: tension exactly `share * sqrt 2`",
            "Shallow-sling and outside-CoG rejections tested",
        ],
        "keywords": ["lifting", "sling", "rigging", "statics", "crane"],
        "categories": ["science", "simulation"],
    },
    "crates/sea/tpt-yard-drydock": {
        "tagline": "Drydock flooding and ballast sequencing with stability at every water level.",
        "overview": [
            "Flooding a building dock is a controlled stability exercise: as the water "
            "rises the vessel takes load off the keel blocks, starts to float, and "
            "must be upright at *every* intermediate level before the caisson opens.",
            "`Drydock::flooding_sequence` walks the water level bottom-up and reports "
            "draft, displaced mass, and metacentric height at each step — flagging "
            "any level where the GM drops below the 0.15 m screening minimum, and "
            "any vessel that simply cannot float out of the dock.",
        ],
        "features": [
            "`Drydock::flooding_sequence` — level-by-level state: draft, displacement, GM, hours of flooding",
            "Aground/afloat transitions with keel-block load sharing reported per level",
            "Rectangular-block hydrostatics: `GM = KB + BM - KG`, `BM = B^2/(12 d)`",
            "Fit checks: breadth, length and float-out draft against the dock",
            "Total flooding time from the dock volume and pump rate",
        ],
        "usage": r"""use tpt_yard_drydock::{DockedVessel, Drydock};

let dock = Drydock { length_m: 200.0, width_m: 30.0, depth_m: 10.0 };
let vessel = DockedVessel {
    launch_weight_kg: 6_000_000.0,
    cog_above_keel_m: 5.5,
    length_m: 140.0,
    breadth_m: 22.0,
    block_coefficient: 0.85,
    ballast_tanks: vec![],
};

let seq = dock.flooding_sequence(&vessel, 5_000.0)?; // m3/h
assert!(seq.stable_at_every_level);""",
        "model": [
            "Displacement: rectangular block, `rho * Cb * L * B * d` with sea water at 1025 kg/m3.",
            "The float-off draft solves the displacement equation for the launch weight; levels below it are aground.",
            "Screening-grade hydrostatics (documented in RFC 0002) — fine forms overestimate BM slightly; full hydrostatics belongs to a hull-mesh substrate.",
        ],
        "verification": [
            "Golden: `drydock-flooding-sequence.json` — 13 levels, 9 aground steps, final GM 7.66 m",
            "Capsize test: KG above the metacentre yields negative GM",
            "Too-wide and too-deep vessels rejected with typed errors",
        ],
        "keywords": ["drydock", "flooding", "stability", "metacentric", "ballast"],
        "categories": ["science", "simulation"],
    },
    "crates/sea/tpt-yard-launch": {
        "tagline": "Launch calculations: slipway, shiplift, drydock flooding and side launch.",
        "overview": [
            "How does the ship get into the water? For a slipway end launch, "
            "`tpt-yard-launch` runs the classical statics chain: friction-limited "
            "sliding down the ways (energy balance), way pressures from full contact "
            "through shrinkage to the way-end cribbing, the tip-up race between "
            "float-off travel and the CoG crossing the way end, and water-entry "
            "slamming pressure.",
            "Dock float-out delegates to `tpt-yard-drydock`'s flooding sequence, and "
            "post-launch stability is evaluated from the as-built weight model — the "
            "vessel in its partial outfitting state, not its design state.",
        ],
        "features": [
            "`slipway_launch` — sliding velocity, way-pressure progression, tip-up risk, entry angle, slamming pressure",
            "End-poppet bearing check against the 0.5 MPa class screening limit with explicit `end_bearing_m` cribbing length",
            "Sticky-ways detection (grease friction >= tan slope): the launch will not run",
            "`launch_stability` — GM at the launch condition from the weight model",
            "`drydock_flooding` — float-out sequence via the docking crate",
        ],
        "usage": r"""use tpt_yard_core::{MassProperties, Vector3};
use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};

let analysis = LaunchAnalysis {
    launch_method: LaunchMethod::Slipway { slope_deg: 3.0, ways: 2 },
    vessel_weight: MassProperties { mass_kg: 4_000_000.0, cog: Vector3::new(70.0, 0.0, 6.0) },
    way_length_m: 120.0, way_width_m: 2.0,
    friction_coefficient: 0.02,
    poppet_to_cog_m: 70.0, end_bearing_m: 20.0,
    immersion_length_m: 90.0, block_coefficient: 0.8, breadth_m: 20.0,
    site: SiteConditions { max_sea_state: 3 },
};
let result = analysis.slipway_launch();
assert!(!result.tip_up_risk);
assert!(result.safe);""",
        "model": [
            "Sliding: `v^2 = 2 g s (sin theta - mu cos theta)` over the way travel.",
            "Tip-up: the CoG crossing the way end must come *after* float-off (`draft / sin theta` travel); otherwise the stern pivots on the poppet.",
            "Slamming: `p = 1/2 rho v^2 C_imp` with a von Karman-type screening coefficient (RFC 0002).",
        ],
        "verification": [
            "Golden: `slipway-launch-stability.json` — 8.73 m/s entry, 0.163 -> 0.490 MPa pressures, no tip-up",
            "Tip-up detection for a known-bad CoG position; sticky ways; undersized cribbing",
            "Capsize detection from the weight model with KG above the metacentre",
        ],
        "keywords": ["ship-launch", "slipway", "launching", "stability", "shipyard"],
        "categories": ["science", "simulation"],
    },
    "crates/sea/tpt-yard-outfitting": {
        "tagline": "Systems installation and routing for vessel outfitting.",
        "overview": [
            "Outfitting is where ships quietly go wrong: a sea-water line crossing a "
            "cable tray, a duct leaving the hull envelope. `tpt-yard-outfitting` "
            "plans the routes, detects the clashes (route vs route and route vs "
            "hull), and sequences the installation the way yards actually work — "
            "large systems first, smaller routing after.",
        ],
        "features": [
            "`OutfittingPlan` — outfit systems plus their 3D polyline routes",
            "`collision_detection` — bounding-box screening with clearance, against other routes and the hull envelope",
            "Typed clashes: `SystemVsSystem` and `SystemVsHull`, each with an approximate location",
            "`installation_sequence` — size-descending order (machinery before ducting before cable trays)",
            "Route bounding boxes grown by half the cross-section, so clearance is explicit",
        ],
        "usage": r"""use tpt_yard_core::{FluidType, Geometry3D, OutfitSystem, Vector3};
use tpt_yard_outfitting::{OutfittingPlan, Route};

let mut plan = OutfittingPlan::new();
plan.add_system(OutfitSystem::Piping { fluid: FluidType::SeaWater, diameter_mm: 200.0 });
plan.add_system(OutfitSystem::Electrical { voltage_v: 440.0, cable_type: "FEF".into() });

plan.routes.push(Route {
    system: 0,
    waypoints: vec![Vector3::new(0.0, 0.0, 2.0), Vector3::new(20.0, 0.0, 2.0)],
    cross_section_m: 0.3,
});
let clashes = plan.collision_detection(&hull_geometry);""",
        "model": [
            "Screening is axis-aligned bounding boxes — fast, deterministic, and conservative; exact pipe-vs-tray contact needs a collision substrate.",
            "The hull check requires each route to stay inside the envelope's bounding box; anything poking out is a hull clash.",
            "Installation order ties break by plan order, so the sequence is stable across runs.",
        ],
        "verification": [
            "Crossing routes clash exactly once; parallel routes with clearance do not",
            "Routes poking out of the hull are flagged `SystemVsHull`",
            "Large-first installation order verified",
        ],
        "keywords": ["outfitting", "routing", "clash-detection", "piping", "installation"],
        "categories": ["science", "simulation", "algorithms"],
    },
    "crates/sea/tpt-yard-sea-trials": {
        "tagline": "Post-launch sea-trial test planning and acceptance-criteria evaluation.",
        "overview": [
            "Between launch and delivery stands the trial program: speed runs on the "
            "measured mile, manoeuvring circles, crash stops, seakeeping, noise "
            "surveys, endurance, the inclining experiment, and the class society's "
            "acceptance protocol. `tpt-yard-sea-trials` models the program and judges "
            "the measurements.",
            "An incomplete program never passes: a trial without a measurement is an "
            "explicit failure state, which is exactly what a delivery gate needs.",
        ],
        "features": [
            "`TrialProgram` — the ordered trial list with categories from speed to class acceptance",
            "`Acceptance` windows on typed metrics (knots, turning-circle lengths, dB(A), GM, ...) with inclusive bounds",
            "`evaluate` — per-trial outcomes: passed, failed with reason, or *not performed*",
            "`TrialReport` — delivery-facing summary with passed/total counts",
        ],
        "usage": r"""use tpt_yard_sea_trials::{Acceptance, Metric, Trial, TrialProgram};

let mut program = TrialProgram::new();
program.push(Trial {
    id: 1,
    name: "Speed trial".into(),
    kind: Default::default(),
    acceptance: Acceptance { metric: Metric::SpeedKn, minimum: Some(15.0), maximum: None },
    measured: Some(15.6),
});
let report = program.evaluate();
assert!(report.all_passed);""",
        "model": [
            "Bounds are inclusive on both ends; a one-sided window leaves the other end `None`.",
            "Trial kinds map to the standard program sections; metrics carry their units in `Display` for report formatting.",
        ],
        "verification": [
            "Passing, failing and unperformed trials each tested",
            "Inclusive-bound edge values accepted exactly at the limit",
            "Inclining-experiment GM gate covered",
        ],
        "keywords": ["sea-trials", "acceptance", "testing", "marine", "delivery"],
        "categories": ["science", "simulation"],
    },
    "crates/space/tpt-yard-orbital-assembly": {
        "tagline": "Orbital assembly planning and simulation for space structures.",
        "overview": [
            "Building a truss at Earth-Moon L2 looks nothing like building a ship — "
            "no gravity, but docking impulses at the growing free end, handling "
            "force limits at the grippers, and collision geometry everywhere. "
            "`tpt-yard-orbital-assembly` plans the sequence (anchored bays first, "
            "build outwards), simulates every step against constraint tags, and "
            "verifies the partially assembled cantilever against docking impulses.",
            "Root stress under a tip impulse grows *linearly* with deployed length — "
            "the deployment risk that makes assembly order matter, and the reason the "
            "integrity verifier reports at every single step.",
        ],
        "features": [
            "`OrbitalAssembly::plan_sequence` — grasp, translate, rotate, dock, bolt, release per component, constraint-tagged",
            "`simulate_step` — handling force (`m a` with station keeping + manoeuvre allowance), swept bounding-sphere collision checks against installed components",
            "`verify_structural_integrity` — cantilever root stress at any step: `sigma = P L / (A h)`, growing with deployment",
            "`SpaceStructure` taxonomy: truss, station, rotating habitat, solar array, fuel depot, arbitrary mesh",
            "Constraint vocabulary: `NoCollision`, `ForceLimit`, `MaintainStationKeeping`, thermal and line-of-sight tags",
            "Collision objects exported for robot path planning",
        ],
        "usage": r"""use tpt_yard_core::{ComponentId, Vector3};
use tpt_yard_orbital_assembly::{
    ComponentSpec, OrbitalAssembly, OrbitalParameters, SpaceStructure,
};

let mut assembly = OrbitalAssembly::new(
    SpaceStructure::Truss { segments: 4, length_m: 20.0 },
    OrbitalParameters::default(),
);
assembly.add_component(ComponentSpec {
    id: ComponentId(1),
    name: "bay 1".into(),
    mass_kg: 500.0,
    dimensions: Vector3::new(5.0, 3.0, 3.0),
    target_position: Vector3::new(2.5, 0.0, 0.0),
});
let steps = assembly.plan_sequence();""",
        "model": [
            "Collision spheres use the largest half-axis — the 3D diagonal makes adjacent same-pitch bays overlap spuriously (RFC 0003).",
            "The cantilever check assumes a constant cross-section (0.01 m2 chord, 3 m bay height, ISS-like); stress therefore grows linearly with length, matching the golden growth law.",
            "Sequence planning is doctrinal and deterministic; time/energy optimisation belongs to `tpt-yard-scheduling`.",
        ],
        "verification": [
            "`test_orbital_assembly_collision`: a mis-planned bay sharing a target with an installed bay is caught",
            "Golden: `orbital-assembly-sequence.json` and `iss-truss-assembly.json` (linear growth law)",
            "Over-mass components violate force limits; violent docking impulses fail the integrity check",
        ],
        "keywords": ["orbital-assembly", "space", "truss", "robotics", "simulation"],
        "categories": ["aerospace", "aerospace::simulation", "science", "simulation"],
    },
    "crates/space/tpt-yard-space-structural": {
        "tagline": "Structural design without launch constraints: vacuum, rotation, thermal cycling.",
        "overview": [
            "A structure assembled in orbit never rides a rocket: no fairing diameter, "
            "no Max-Q, no ascent vibration, no 1-g handling loads. "
            "`tpt-yard-space-structural` states that freedom formally and implements "
            "the load cases that *do* govern: hoop stress in a spinning habitat ring, "
            "thermal-cycling fatigue across eclipse transitions, and Whipple-shield "
            "sizing against micrometeoroids.",
            "The centrepiece is the spec's verification identity — hoop stress "
            "`sigma = rho omega^2 r^2` — verified to machine precision and locked as "
            "golden data.",
        ],
        "features": [
            "`rotating_habitat_stress` — hoop stress, angular velocity, radial growth, and yield utilization for a spinning ring",
            "`thermal_cycling_fatigue` — strain range from the orbit hot/cold swing, Coffin-Manson life, Miner's-rule life fraction",
            "`micrometeoroid_shielding` — Whipple bumper / standoff / rear-wall sizing with areal density",
            "`no_launch_constraint` — the formal design-freedom statement (`ShapeConstraint::None`, unbounded size)",
            "`SpaceEnvironment` — orbit, thermal cycling, flux, radiation, atomic oxygen",
        ],
        "usage": r"""use tpt_yard_core::Material;
use tpt_yard_space_structural::SpaceStructuralDesigner;

let designer = SpaceStructuralDesigner::new(Material::aa5083());

// The spec's reference case: 2 rpm, 100 m ring.
let stress = designer.rotating_habitat_stress(100.0, 2.0, 1.0e6);
println!("hoop: {:.2} MPa (util {:.1}%)",
         stress.hoop_stress_mpa, stress.utilization * 100.0);

let fatigue = designer.thermal_cycling_fatigue(15 * 5_660); // 15 LEO years
println!("life fraction: {:.2}", fatigue.life_fraction);""",
        "model": [
            "Thin-ring hoop stress with the structural material density; radial growth is the elastic strain times the radius.",
            "Coffin-Manson life `N_f = 0.5 (dE / (3.5 sigma_u / E))^(-1/0.12)`; aluminium's higher expansion burns life faster than steel (tested ordering).",
            "Whipple ratios (bumper d/6, standoff d/10, wall d/12) are NASA ship-set screening practice, documented in RFC 0005.",
        ],
        "verification": [
            "Golden: `rotating-habitat-stress.json` at machine precision",
            "Quadratic scaling in rpm and radius asserted; steel at high rpm exceeds yield",
            "Shield sizing ratios and fatigue ordering by material tested",
        ],
        "keywords": ["space-structures", "habitat", "whipple-shield", "thermal-fatigue", "aerospace"],
        "categories": ["aerospace", "science", "simulation"],
    },
    "crates/space/tpt-yard-space-manufacturing": {
        "tagline": "In-space manufacturing and additive construction planning.",
        "overview": [
            "Printing structure in vacuum changes the economics: feedstock mass is "
            "launch mass (unless it comes from regolith), deposition power has nowhere "
            "to go but radiation, and destructive testing may not be able to come "
            "home. `tpt-yard-space-manufacturing` plans all of it — print time, "
            "energy, heat rejection, and the in-situ quality plan — and chains into "
            "the orbital assembly planner so manufacturing and construction are one "
            "pipeline.",
        ],
        "features": [
            "`print_time_estimate` — feedstock mass (with ISRU scrap) over the deposition rate",
            "Process energy intensities from powder-bed (12 kWh/kg) to wire-arc (4 kWh/kg)",
            "`thermal_control_during_print` — deposition power and radiator area from the Stefan-Boltzmann law (eps 0.85, 350 K), eclipse pauses flagged",
            "`quality_verification` — 100 % layer imaging, vacuum NDT subset, witness coupons, destructive testing only if samples come home",
            "`Feedstock` from Earth launch or ISRU sources (lunar/martian regolith, asteroid metal, recycled debris)",
        ],
        "usage": r"""use tpt_yard_core::Material;
use tpt_yard_space_manufacturing::{
    AdditiveTechnique, Feedstock, InSpaceManufacturing, ManufacturingProcess,
};

let printer = InSpaceManufacturing::new(
    ManufacturingProcess::AdditiveManufacturing {
        technique: AdditiveTechnique::WireArcAdditive,
        material: "AA5083".into(),
    },
    Feedstock::earth_launched(2660.0),
    Material::aa5083(),
);

let hours = printer.print_time_estimate(0.144, 6.0)?; // one panel substrate
let thermal = printer.thermal_control_during_print(6.0)?;""",
        "model": [
            "Print time: `t = rho V (1 + scrap) / rate`; ISRU feedstock carries a processing scrap fraction.",
            "Radiators reject by radiation only: `A = P / (eps sigma T^4)`; a 40 kW deposition needs ~33 m2 at 350 K.",
            "Energy intensity is process-typical and documented; swap in measured values as they arrive.",
        ],
        "verification": [
            "Print time equals mass over rate, including the ISRU scrap factor",
            "Radiator area matches the closed-form Stefan-Boltzmann result",
            "ISRU builds flagged non-destructive-testing; error paths covered",
        ],
        "keywords": ["additive-manufacturing", "isru", "in-space-manufacturing", "aerospace", "deposition"],
        "categories": ["aerospace", "science", "simulation"],
    },
    "crates/space/tpt-yard-robotic-assembly": {
        "tagline": "Robotic arm kinematics and collision-free path planning for space assembly.",
        "overview": [
            "Free-flying manipulators need three competences: solve for joint angles, "
            "find a collision-free path, and plan the grasp. "
            "`tpt-yard-robotic-assembly` implements all three for serial revolute "
            "arms in a planar work area — the documented simplification (RFC 0003) "
            "that already validates the constraint machinery end to end.",
            "Two-link arms get closed-form inverse kinematics; longer chains get "
            "Levenberg-Marquardt damped least squares with step clamping, full-turn "
            "range wrapping, and a deterministic seed sweep to escape local minima. "
            "Paths come from RRT with swept collision sampling.",
        ],
        "features": [
            "`forward_kinematics` / `inverse_kinematics` — closed form for two links, DLS refinement for longer chains, `None` for unreachable targets",
            "Range-aware solvers: revolute joints wrap modulo 2-pi, others clamp",
            "`plan_path` — RRT in joint space with circular obstacles, clearance inflation, and swept segment checks",
            "`MotionPlan` — Cartesian waypoints, joint trajectory, duration at the arm's velocity limits",
            "`grasp_planning` — side-face grasp poses and required grip force from the handling acceleration",
            "Typed `ArmError` for joint/link count mismatches",
        ],
        "usage": r"""use tpt_yard_core::{RobotId, Vector3};
use tpt_yard_robotic_assembly::{EndEffector, Joint, Pose, RoboticArm};

let arm = RoboticArm::new(
    RobotId(1),
    vec![
        Joint::revolute(0.0, 2.0 * std::f64::consts::PI, 1.0),
        Joint::revolute(0.0, 2.0 * std::f64::consts::PI, 1.0),
    ],
    vec![1.0, 1.0],
    EndEffector::Gripper { force_n: 500.0 },
);
let target = Pose { position: Vector3::new(1.0, 1.0, 0.0), yaw_rad: 0.0 };
let q = arm.inverse_kinematics(&target)?.expect("reachable");
let fk = arm.forward_kinematics(&q)?;""",
        "model": [
            "DLS damping is adaptive: high early for robust steps from bad seeds, relaxed near the solution; steps are clamped so Newton cannot throw the chain across the workspace.",
            "Local minima are a property of the problem, not a bug — the multi-seed sweep is the standard cure and is deterministic.",
            "The obstacle check samples points along every link, so swept motion cannot tunnel through a circle between waypoints.",
        ],
        "verification": [
            "IK-to-FK round trips on multiple targets; unreachable and mismatched cases typed",
            "Golden: `robotic-arm-path.json` — RRT finds a collision-free path reaching the goal within tolerance",
            "Grasp force vs gripper capability tested both ways",
        ],
        "keywords": ["robotics", "inverse-kinematics", "rrt", "path-planning", "motion-planning"],
        "categories": ["science::robotics", "algorithms", "aerospace"],
    },
    "crates/space/tpt-yard-habitat": {
        "tagline": "Rotating habitat design: artificial gravity, Coriolis comfort, structure.",
        "overview": [
            "How fast must a 100 m torus spin for 1 g, and will the occupants get "
            "sick? `tpt-yard-habitat` answers the three rotating-habitat questions: "
            "spin rate (`omega = sqrt(g/r)`), the 2 rpm Coriolis comfort limit, and "
            "how much structure the spinning hull needs.",
            "The design tension is explicit in the tests: 1 g at 100 m needs 2.99 rpm "
            "(above the comfort limit — mitigate or grow); 1 g at 4 km needs 0.47 rpm "
            "(comfortable). Radius is the knob, and the crate makes the trade "
            "computable.",
        ],
        "features": [
            "`required_rotation` / `required_rotation_rpm` — `omega = sqrt(g/r)` for any target gravity (Earth, Mars, lunar)",
            "`coriolis_effects` — cross-coupled acceleration for head movements and the inclusive 2 rpm comfort check",
            "`structural_design` — hoop area from ring tension `T = m omega^2 r / (2 pi)`, shell thickness from the circumference, with safety factor",
            "`HabitatType` taxonomy: O'Neill cylinder, Stanford torus, Bernal sphere, ring station, custom",
            "Self-consistent results: utilization <= 1 by construction, monotone in mass and safety factor (tested)",
        ],
        "usage": r"""use tpt_yard_core::Material;
use tpt_yard_habitat::{HabitatDesigner, HabitatType};

let designer = HabitatDesigner::new(
    HabitatType::StanfordTorus { radius_m: 100.0, tube_diameter_m: 20.0 },
    Material::aa5083(),
);

let rpm = designer.required_rotation_rpm(100.0, 1.0);   // ~2.99 rpm
let comfort = designer.coriolis_effects(100.0, rpm);
assert!(!comfort.within_comfort); // above the 2 rpm limit""",
        "model": [
            "Comfort criterion: rotation <= 2 rpm keeps cross-coupled accelerations from 1 m/s head movements (~4.3 % g at 2 rpm) below the ~10 % nausea threshold.",
            "Ring tension distributes the total rotating mass around the circumference; the shell thickness follows for the habitat archetype's radius.",
            "The 1 g design point is documented; re-run `structural_design` math for Mars-g by scaling omega.",
        ],
        "verification": [
            "`omega = sqrt(g/r)` verified exactly, including the Mars-gravity `sqrt(0.38)` scaling",
            "The 2 rpm comfort boundary tested inclusive and exclusive",
            "O'Neill reference: 4 km radius at 0.47 rpm is comfortable",
        ],
        "keywords": ["habitat", "artificial-gravity", "coriolis", "rotating-ring", "oneill"],
        "categories": ["aerospace", "science", "simulation"],
    },
    "crates/space/tpt-yard-propellant": {
        "tagline": "Propellant loading and boil-off management for space vessels.",
        "overview": [
            "Loading cryogen into a depot tank is a thermal problem wearing a plumbing "
            "hat: chill down first or the fill flashes, watch the vent capacity, and "
            "every hour on orbit leaks heat that becomes boil-off. "
            "`tpt-yard-propellant` plans the load and manages the leak — from LOX and "
            "LH2 to storables like MMH and NTO.",
            "Zero-boil-off is a qualification, not a slogan: a tank earns the ZBO "
            "label by holding its heat leak under 0.1 % of load per day, and the "
            "planner tells you whether yours qualifies.",
        ],
        "features": [
            "`PropellantSpec` presets: LOX, LH2, LCH4, MMH, NTO with density, latent heat, boiling point",
            "`LoadingPlanner::plan_loading` — fill rate, chilldown time (tank thermal mass vs latent heat), venting requirement, subcooling target",
            "`boil_off_report` — kg/day and %/day boil-off, time-to-vent, cryocooler power for ZBO",
            "`BoilOffPolicy` — Vented, Recooled, or ZeroBoilOff, assigned by qualification",
            "Storable propellants: no chilldown, no venting, ZBO by construction",
        ],
        "usage": r"""use tpt_yard_propellant::{LoadingPlanner, PropellantSpec, TankSpec};

let planner = LoadingPlanner::new();
let tank = TankSpec { volume_m3: 300.0, ullage_frac: 0.03, heat_leak_w: 2_000.0 };

let plan = planner.plan_loading(&PropellantSpec::lox(), &tank, 50.0)?;
assert!(plan.venting_required);            // LOX is cryogenic
assert!(plan.thermal_control.chilldown_required);

let boil_off = planner.boil_off_report(&PropellantSpec::lox(), &tank);
println!("{:.1} kg/day ({:.3} %/day)", boil_off.boil_off_kg_day, boil_off.boil_off_pct_day);""",
        "model": [
            "Boil-off converts the heat leak through the latent heat: `kg/day = W * 86400 / (L * 1000)`.",
            "Chilldown consumes ~80 % of the tank thermal-mass cooling demand as vapour, timed at the fill rate.",
            "The ZBO threshold is 0.1 %/day of load through latent heat; the cryocooler is sized from a specific-power ratio (100:1 default).",
        ],
        "verification": [
            "Cryogenic stability test balances 2000 W against LOX latent heat (811 kg/day, vented policy)",
            "LH2 verified as the worst percentage boil-off (low density, modest latent heat)",
            "ZBO qualification tested both sides of the threshold",
        ],
        "keywords": ["propellant", "cryogenic", "boil-off", "zero-boil-off", "depot"],
        "categories": ["aerospace", "science", "simulation"],
    },
    "crates/planning/tpt-yard-scheduling": {
        "tagline": "Construction scheduling: critical path, optimisation and resource levelling.",
        "overview": [
            "Which activities drive the ship's delivery date, and can the cranes "
            "actually keep up? `tpt-yard-scheduling` runs the critical-path method "
            "over the assembly network, schedules under explicit objectives, and "
            "levels resources with a real serial schedule-generation scheme — the "
            "kind that trades makespan for flat peaks and says so.",
        ],
        "features": [
            "`critical_path` — zero-float activities from a full CPM pass (earliest/latest starts, float per activity)",
            "`optimize_sequence` — objectives: minimum duration, cost, crane usage, drydock time, maximum parallelism",
            "`resource_leveling` — serial RCPSP placement: each activity at the earliest clash-free slot for its resources",
            "Honest levelling: may stretch the makespan to flatten peaks (golden case: 32 h -> 40 h for a crane peak of 2 -> 1)",
            "Peak concurrent demand per resource kind on a 1-hour grid",
        ],
        "usage": r"""use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{ActivityType, AssemblyActivity};
use tpt_yard_scheduling::{ScheduleObjective, ShipyardScheduler};

let scheduler = ShipyardScheduler::new(vec![
    AssemblyActivity::new(ActivityId(1), "Cut", ActivityType::CutSteel, 8.0),
    AssemblyActivity::new(ActivityId(2), "Weld", ActivityType::WeldBlock, 16.0)
        .with_dependencies(&[ActivityId(1)]),
    AssemblyActivity::new(ActivityId(3), "Erect", ActivityType::JoinBlock, 4.0)
        .with_dependencies(&[ActivityId(2)]),
]);

let path = scheduler.critical_path()?;
let levelled = scheduler.resource_leveling()?;""",
        "model": [
            "CPM: forward pass for earliest finishes, backward pass for latest starts; float is the difference, zero float is critical.",
            "Levelling places activities min-float-first at the earliest time their resources conflict with nothing already placed — clamped steps, bounded scan.",
            "Zero-capacity resource entries do not consume: a named-but-free resource never triggers a clash.",
        ],
        "verification": [
            "Golden: `critical-path-schedule.json` (34 h network, 4 h float branch)",
            "Golden: `resource-leveling.json` — crane peak 2 -> 1 at a makespan cost of 32 -> 40 h",
            "Cycle and empty-schedule error paths typed and tested",
        ],
        "keywords": ["scheduling", "critical-path", "resource-leveling", "project-planning", "cpm"],
        "categories": ["algorithms", "science", "simulation"],
    },
    "crates/planning/tpt-yard-logistics": {
        "tagline": "Material and resource logistics: delivery, staging and transport scheduling.",
        "overview": [
            "Yard logistics is a pull system: every steel delivery exists because an "
            "assembly activity needs it on a computable date. `tpt-yard-logistics` "
            "computes order-by and arrive-by dates from lead times and the CPM "
            "schedule, enforces a staging dwell limit, and refuses plans that would "
            " bury the laydown area.",
        ],
        "features": [
            "`MaterialFlow` — the manifest: items with quantity, lead time, staging footprint, and the consuming activity",
            "`schedule_deliveries` — order-by, arrive-by, need date and dwell per item",
            "Staging occupancy sweep: peak concurrent footprint checked against the available laydown area",
            "Negative order dates (order before project start) fall out naturally",
        ],
        "usage": r"""use tpt_yard_assembly::ActivityId;
use tpt_yard_core::AssemblyActivity;
use tpt_yard_logistics::{MaterialFlow, MaterialItem};

let mut flow = MaterialFlow::new();
flow.add_item(MaterialItem {
    id: 1,
    name: "Block 212 steel".into(),
    quantity_t: 142.0,
    needed_for: ActivityId(1),
    lead_time_days: 30.0,
    footprint_m2: 200.0,
});
let deliveries = flow.schedule_deliveries(&activities, 5.0, 1_000.0)?;""",
        "model": [
            "Need dates come from the CPM earliest start (8-hour days); orders go `lead_time + buffer` days ahead.",
            "Staging occupancy is a +footprint/-footprint event sweep between arrival and consumption — arrivals before need are the dwell.",
            "Overflow is a typed error naming the required and available areas, not a warning.",
        ],
        "verification": [
            "Order/arrive/need dates match the hand-computed schedule for a two-activity chain",
            "Staging overflow detected and cleared by enlarging the laydown area",
            "Unknown-activity references rejected",
        ],
        "keywords": ["logistics", "material-flow", "staging", "supply", "delivery"],
        "categories": ["science", "simulation", "algorithms"],
    },
    "crates/planning/tpt-yard-facility": {
        "tagline": "Shipyard and orbital-facility layout planning with capacity constraints.",
        "overview": [
            "Can the yard actually build what the schedule promises? "
            "`tpt-yard-facility` models the yard's physical plant — cranes, "
            "workshops, docks, orbital bays — as placed facilities with capacities, "
            "checks peak demand against them, and keeps the layout honest with "
            "clearance-aware placement validation.",
        ],
        "features": [
            "`FacilityPlan` — placed facilities with typed capacities (tonnes, m2, berth slots, cells)",
            "`check_capacity` — peak demand of a kind against the summed capacity",
            "`can_place` / `validate` — axis-aligned footprint placement with clearance; overlapping pairs named",
            "`FacilityKind` units documented per kind (`t`, `m2`, `slots`, `cells`)",
        ],
        "usage": r"""use tpt_yard_core::Vector3;
use tpt_yard_facility::{Facility, FacilityKind, FacilityPlan};

let mut plan = FacilityPlan::new();
plan.add(Facility {
    name: "Goliath crane".into(),
    kind: FacilityKind::Crane,
    capacity: 1_200.0,
    position: Vector3::new(0.0, 0.0, 0.0),
    footprint_m: (30.0, 30.0),
});

assert!(plan.check_capacity(FacilityKind::Crane, 900.0));
assert!(!plan.check_capacity(FacilityKind::Crane, 1_500.0));""",
        "model": [
            "Placement overlap is axis-aligned footprint rectangles grown by the clearance — conservative and deterministic.",
            "Capacity checks sum over all facilities of a kind, so adding a second crane simply raises the ceiling.",
        ],
        "verification": [
            "Capacity pass/fail on both sides of the limit",
            "Overlap detection names the offending facility pair",
            "Clearance-respecting placement accepted",
        ],
        "keywords": ["facility-layout", "capacity", "shipyard", "crane", "planning"],
        "categories": ["science", "simulation"],
    },
    "crates/planning/tpt-yard-quality": {
        "tagline": "Quality control and inspection: NDT plans and defect tracking.",
        "overview": [
            "Class societies do not take your word for it, and neither does this "
            "crate. `tpt-yard-quality` turns the build plan into an NDT inspection "
            "plan — every weld-bearing activity gets a method and coverage by "
            "criticality — and tracks findings through disposition into a "
            "repair-rate report.",
        ],
        "features": [
            "`generate_inspection_plan` — activity type drives the method: structural butts get UT at criticality I, piping gets PT at II, pressure tests get PRESS, everything else visual at III",
            "Coverage per point (100 % for pressure boundaries and full-penetration welds)",
            "`defect_tracking` — counts by type, dominant defect, repair rate, rejections to engineering",
            "`NdtMethod` with standard codes (UT, RT, MT, PT, ET, VT, VAC, PRESS)",
            "`AcceptanceCriteria` referencing the governing standard and maximum indication",
        ],
        "usage": r"""use tpt_yard_core::{ActivityId, ActivityType, BuildPhase, PhaseId};
use tpt_yard_quality::{NdtMethod, QualityManagement};

let mut phase = BuildPhase::new(PhaseId(1), "Panel line", 5.0);
phase.activities.push(tpt_yard_core::AssemblyActivity::new(
    ActivityId(1), "Weld butt seam", ActivityType::WeldBlock, 8.0,
));

let qm = QualityManagement::default();
let plan = qm.generate_inspection_plan(&[phase]);
assert_eq!(plan[0].method, NdtMethod::UltrasonicTesting);""",
        "model": [
            "Method selection mirrors class practice: full-penetration structural welds are volumetric (UT), fillets and piping surface (PT/MT), tests become their own inspection point.",
            "Defect aggregation sorts by count then name, so reports are stable across runs.",
            "Repair rate counts `Repair` and `RepairAndReinspect` over total findings.",
        ],
        "verification": [
            "Golden: `ndt-inspection-plan.json` — method/criticality/coverage per activity",
            "Golden: `weld-defect-tracking.json` — aggregation, dominant defect, repair rate",
            "Empty-tracking and boundary behaviours tested",
        ],
        "keywords": ["quality", "ndt", "inspection", "defect-tracking", "welding"],
        "categories": ["science", "simulation"],
    },
    "crates/integration/tpt-yard-transport-link": {
        "tagline": "Bridge between vehicle design (tpt-transport) and shipyard construction.",
        "overview": [
            "The lifecycle reads: design, build, operate. `tpt-yard-transport-link` "
            "is the first and last mile — mapping a vehicle design to a construction "
            "plan, and handing the finished twin's as-built truth back to the "
            "operational model.",
            "The `tpt-transport` substrate is not published yet, so the design-side "
            "types (`VehicleDesign`, `AsBuiltProperties`) are vendored behind the "
            "same shapes the substrate will use; the swap is mechanical when it "
            "lands.",
        ],
        "features": [
            "`plan_construction` — maritime designs become drydock projects with the five-stage skeleton; orbital spacecraft become assembly projects",
            "`handover_to_operations` — as-built weight, CoG, structural summary and accepted quality records from the finished twin",
            "The spec section 6 contract, implemented and round-trip tested",
        ],
        "usage": r"""use tpt_yard_core::{ConstructionMethod, VesselType};
use tpt_yard_transport_link::{plan_construction, DesignKind, VehicleDesign};

let design = VehicleDesign {
    name: "Container ship 1400 TEU".into(),
    kind: DesignKind::Maritime { loa_m: 140.0, deadweight_t: 18_500.0 },
};
let project = plan_construction(&design);
assert_eq!(project.construction_method, ConstructionMethod::SeaDrydock);""",
        "model": [
            "Construction method follows the design family: maritime -> `SeaDrydock`, orbit-assembled spacecraft -> `OrbitalAssembly`.",
            "The handover carries *installed* (as-built) numbers — the operational model gets reality, not the design brochure.",
        ],
        "verification": [
            "Round-trip test: design a ship, build all five phases through the twin, assert the handover carries exactly the installed mass and closed-form CoG",
            "Spacecraft designs map to `OrbitalAssembly`",
        ],
        "keywords": ["integration", "as-built", "handover", "vessel", "lifecycle"],
        "categories": ["science", "simulation", "development-tools"],
    },
    "crates/integration/tpt-yard-process-link": {
        "tagline": "Bridge between process engineering (tpt-process) and construction planning.",
        "overview": [
            "Propellant loading is thermodynamics with a schedule. "
            "`tpt-yard-process-link` vendors a minimal Peng-Robinson equation of "
            "state (the spec section 6 contract) and wires it into the propellant "
            "loading planner: the EOS computes ullage vapour densities at loading "
            "conditions, and the loading plan says what to do about them.",
            "When the `tpt-process` substrate publishes, the vendored EOS yields to "
            "it — the planning API is the contract.",
        ],
        "features": [
            "`PengRobinson` — single-component EOS with the classical constants, cubic root solve, vapour and liquid roots",
            "`vapour_density` at (T, P) from the compressibility factor",
            "EOS parameter table for LOX, LH2, LCH4, MMH, NTO",
            "`plan_propellant_loading` — the operational plan plus the EOS-derived venting decision",
        ],
        "usage": r"""use tpt_yard_process_link::PengRobinson;

// Methane at 111 K and 0.1 MPa: near-ideal vapour.
let eos = PengRobinson::new(190.6, 4.599e6, 0.011);
let z = eos.compressibility(111.0, 0.1e6);
assert!((z - 0.97).abs() < 0.05);

let rho = eos.vapour_density(111.0, 0.1e6, 16.04);""",
        "model": [
            "PR constants: `a = 0.45724 R^2 Tc^2 / Pc`, `b = 0.07780 R Tc / Pc`, kappa from the acentric factor; alpha per temperature.",
            "The compressed-liquid region (T < Tc, P > Pc) returns the liquid root; otherwise the vapour root.",
            "Venting by EOS: dense ullage vapour (> 1 kg/m3 at loading conditions) means vent capacity governs the fill.",
        ],
        "verification": [
            "Methane Z near unity at low pressure; sub-unity near critical pressure",
            "LOX loading plan requires venting and chilldown, agreeing with the EOS decision",
            "Unknown propellants rejected with a typed error",
        ],
        "keywords": ["peng-robinson", "eos", "propellant", "thermodynamics", "cryogenic"],
        "categories": ["science", "simulation", "aerospace"],
    },
    "crates/integration/tpt-yard-earth-link": {
        "tagline": "Bridge between weather and sea-state data (tpt-earth) and launch planning.",
        "overview": [
            "No launch officer floats a 4,000-tonne ship down greased ways into a "
            "rising sea. `tpt-yard-earth-link` gates launch methods on the Douglas "
            "sea-state forecast and returns the safe window — first safe hour, "
            "duration, and the governing limit per method.",
            "The `tpt-earth` forecast type is vendored behind the shape the "
            "substrate will use; the gating logic is the contract.",
        ],
        "features": [
            "`plan_launch_window` — the longest calm run in the forecast for the launch method",
            "Per-method screening limits: side launch 1, slipway/shiplift 2, dock flooding 3 (Douglas scale)",
            "Never-safe forecasts are a typed error, never an empty window",
            "`LaunchWindow` with first hour, exclusive end, limit, calm hours, and a plain-language note",
        ],
        "usage": r"""use tpt_yard_core::{MassProperties, Vector3};
use tpt_yard_earth_link::{plan_launch_window, SeaStateForecast};
use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};

let forecast = SeaStateForecast {
    hourly_sea_state: (0..24).map(|h| if (9..17).contains(&h) { 4.0 } else { 1.0 }).collect(),
};
let window = plan_launch_window(&analysis, &forecast)?;
println!("launch between hours {} and {}", window.earliest_hour, window.latest_hour);""",
        "model": [
            "Limits are yard-practice screening values per launch method; the site's own `max_sea_state` may tighten them further.",
            "The window is the first (and longest) run of hours at or below the limit — deterministic and forecast-stable.",
        ],
        "verification": [
            "A mid-day storm pushes the window to the longer calm run",
            "The same forecast clears dock flooding but blocks a side launch (method limits differ)",
            "Never-safe and empty forecasts are typed errors",
        ],
        "keywords": ["sea-state", "launch-window", "weather", "douglas-scale", "marine"],
        "categories": ["science", "simulation", "aerospace"],
    },
}

# Example packages and the benches package: README + CHANGELOG only
# (they are `publish = false` workspace packages, so keywords/categories
# do not apply).
EXAMPLES = {
    "examples/container-ship-block-assembly": {
        "title": "Container ship block assembly",
        "body": "Plans the erection of a 1,400 TEU container ship end-to-end: crane- and workshop-constrained block division, the midship-outward erection sequence, and a digital-twin run through every phase with live weight/CoG tracking and keel-block reactions.",
        "run": "cargo run -p container-ship-block-assembly",
        "expect": "A 12-block erection table (installed tonnage, CoG, keel reactions) and a completion summary at 1,714 t with zero deviation.",
        "phase": "Phase 3 milestone",
    },
    "examples/orbital-station-truss": {
        "title": "Orbital station truss assembly",
        "body": "Simulates a robotic truss build: sequence planning (grasp, translate, rotate, dock, bolt, release per bay), step simulation with collision and force-limit checks, and partial-structure integrity at every bay release.",
        "run": "cargo run -p orbital-station-truss",
        "expect": "A 36-step table for 6 bays with root stress per step, all constraints green, ending at the deployed-cantilever integrity report.",
        "phase": "Phase 4 milestone",
    },
    "examples/rotating-habitat-construction": {
        "title": "Rotating habitat construction",
        "body": "The full rotating-habitat design chain for a Stanford torus and an O'Neill cylinder: spin rate, Coriolis comfort, thermal-fatigue life, structural sizing, and Whipple-shield screening.",
        "run": "cargo run -p rotating-habitat-construction",
        "expect": "Spin-rate/comfort tables for 1 g and Mars g, shell sizing, and 15-year thermal-cycling life fractions.",
        "phase": "Phase 4",
    },
    "examples/submarine-pressure-hull": {
        "title": "Submarine pressure hull",
        "body": "Ring-section planning for a 55 m pressure hull: circumferential double-V seam geometry, SAW procedure thermal cycle and shrinkage, and a workshop cradle FEM check with hoisting loads.",
        "run": "cargo run -p submarine-pressure-hull",
        "expect": "Seam weld-metal mass, HAZ t8/5 timing, and a cradle utilization report (OK / OVERSTRESSED).",
        "phase": "Phase 3",
    },
    "examples/space-solar-array-assembly": {
        "title": "Space solar array assembly",
        "body": "Plans in-space additive manufacturing of a solar array end-to-end: wire-arc print times and energy per panel substrate, vacuum heat rejection, in-situ quality plan, then robotic assembly of the printed panels into the deployed wing.",
        "run": "cargo run -p space-solar-array-assembly",
        "expect": "Per-panel print/energy figures, a 48-step assembly with all constraints ok, and the deployed-wing integrity report.",
        "phase": "Phase 5 milestone",
    },
    "examples/drydock-flooding-sequence": {
        "title": "Drydock flooding sequence",
        "body": "Floats a 4,000 t vessel out of a 200 m building dock: level-by-level draft, displacement, GM and aground/afloat state from the as-built weight model.",
        "run": "cargo run -p drydock-flooding-sequence",
        "expect": "A 13-level flooding table ending afloat with GM ~7.8 m, stable at every level.",
        "phase": "Phase 3",
    },
    "benches": {
        "title": "tpt-shipyard benchmarks",
        "body": "Criterion-free timing benches over the engine's hot paths: block lifting statics, the welding distortion chain, orbital assembly sequences, and launch stability.",
        "run": "cargo bench -p tpt-shipyard-benches",
        "expect": "Per-iteration timings printed per bench; all hot paths are microsecond-scale (see `.github/workflows/benchmark.yml` for the CI harness).",
        "phase": "All phases",
    },
}

README_TEMPLATE = Template("""# $name

> $tagline

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/$name.svg)](https://crates.io/crates/$name)
[![Docs.rs](https://docs.rs/$name/badge.svg)](https://docs.rs/$name)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

$overview

## Features

$features

## Installation

```toml
[dependencies]
$name = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
$name = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
$usage
```

## How it works

$model

## Verification

$verification

## Crate metadata

| Field | Value |
|---|---|
| Keywords | $keyword_row |
| Categories | $category_row |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`$name` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
""")

CHANGELOG_TEMPLATE = Template("""# Changelog for $name

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - $today

### Added

$added

### Verification

$verification

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
""")

EXAMPLE_README_TEMPLATE = Template("""# $title

> $body

**Run it:**

```bash
$run
```

**Expected output:** $expect

This example is part of the [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
workspace ($phase). It is a workspace-only package (`publish = false`) — the
publishable crates live under `crates/`.

## License

MIT OR Apache-2.0, same as the workspace.
""")

EXAMPLE_CHANGELOG_TEMPLATE = Template("""# Changelog

All notable changes to this example are documented here
([Keep a Changelog](https://keepachangelog.com/en/1.1.0/), SemVer).

## [0.1.0] - $today

### Added

- $body
- Runnable via `$run`.
""")

BENCH_README = """# tpt-shipyard benchmarks

Timing benches over the engine's hot paths, written against `std::time::Instant`
with `harness = false` so they run on stable without criterion:

| Bench | What it measures |
|---|---|
| `block-lifting` | Sling-load distribution (12 legs) and hull division + erection planning at 140/300/400 m |
| `welding-distortion` | Rosenthal thermal cycles, residual stress, distortion, and sequence ranking on the reference AH36 panel |
| `orbital-assembly-sequence` | Sequence planning, full 15-bay build simulation, and robot path planning around installed structure |
| `launch-stability` | Slipway statics (safe and tip-up cases), 13-level dock flooding, weight-model stability |

**Run:**

```bash
cargo bench -p tpt-shipyard-benches
cargo bench -p tpt-shipyard-benches --bench welding-distortion
```

All hot paths are microsecond-scale — comfortably inside an interactive
digital-twin budget. CI runs the same benches via
`.github/workflows/benchmark.yml` and uploads the timing logs as artifacts.

This is a workspace-only package (`publish = false`); the publishable crates
live under `crates/`. Licensed MIT OR Apache-2.0 like the workspace.
"""

BENCH_CHANGELOG = """# Changelog

All notable changes to the benchmarks package are documented here
([Keep a Changelog](https://keepachangelog.com/en/1.1.0/), SemVer).

## [0.1.0] - 2026-09-28

### Added

- `block-lifting`: sling-load distribution and division/erection planning at three hull scales.
- `welding-distortion`: the full welding chain plus multi-pass sequence ranking.
- `orbital-assembly-sequence`: planning, full-build simulation, and RRT path planning.
- `launch-stability`: slipway statics (safe + tip-up), flooding sequence, weight-model stability.
"""


def bullets(items, indent=""):
    return "\n".join(f"{indent}- {i}" for i in items)


def md_keywords(kws):
    return " ".join(f"`{k}`" for k in kws)


def main():
    errors = []
    for rel, spec in CRATES.items():
        name = os.path.basename(rel)
        crate_dir = rel
        abs_dir = os.path.join(ROOT, rel)
        # sanity: keywords/categories validity
        if len(spec["keywords"]) > 5:
            errors.append(f"{name}: more than 5 keywords")
        for kw in spec["keywords"]:
            if not re.fullmatch(r"[A-Za-z0-9_\-]+", kw):
                errors.append(f"{name}: bad keyword '{kw}'")
        for c in spec["categories"]:
            if c not in VALID_CATEGORIES:
                errors.append(f"{name}: category '{c}' not in known slugs")

        readme = README_TEMPLATE.substitute(
            name=name,
            tagline=spec["tagline"],
            overview="\n\n".join(spec["overview"]),
            features=bullets(spec["features"]),
            usage=spec["usage"],
            model=bullets(spec["model"]),
            verification=bullets(spec["verification"]),
            rel_crate_dir=rel,
            keyword_row=md_keywords(spec["keywords"]),
            category_row=md_keywords(spec["categories"]),
        )
        added = bullets(spec["features"])
        changelog = CHANGELOG_TEMPLATE.substitute(
            name=name, today=TODAY, added=added, verification=bullets(spec["verification"])
        )
        with open(os.path.join(abs_dir, "README.md"), "w", encoding="utf-8", newline="\n") as f:
            f.write(readme)
        with open(os.path.join(abs_dir, "CHANGELOG.md"), "w", encoding="utf-8", newline="\n") as f:
            f.write(changelog)

        # Patch Cargo.toml: insert keywords/categories/readme after description.
        manifest_path = os.path.join(abs_dir, "Cargo.toml")
        manifest = open(manifest_path, encoding="utf-8").read()
        if "keywords" not in manifest:
            kw_line = "keywords = [" + ", ".join(f'"{k}"' for k in spec["keywords"]) + "]"
            cat_line = "categories = [" + ", ".join(f'"{c}"' for c in spec["categories"]) + "]"
            manifest = re.sub(
                r'(^description = ".*"\n)',
                rf'\1{kw_line}\n{cat_line}\nreadme = "README.md"\n',
                manifest,
                count=1,
                flags=re.M,
            )
            open(manifest_path, "w", encoding="utf-8", newline="\n").write(manifest)

    # Examples + benches
    for rel, spec in EXAMPLES.items():
        abs_dir = os.path.join(ROOT, rel)
        pkg_name = os.path.basename(rel)
        with open(os.path.join(abs_dir, "README.md"), "w", encoding="utf-8", newline="\n") as f:
            f.write(EXAMPLE_README_TEMPLATE.substitute(
                title=spec["title"], body=spec["body"], run=spec["run"],
                expect=spec["expect"], phase=spec["phase"],
            ))
        with open(os.path.join(abs_dir, "CHANGELOG.md"), "w", encoding="utf-8", newline="\n") as f:
            f.write(EXAMPLE_CHANGELOG_TEMPLATE.substitute(
                today=TODAY, body=spec["body"], run=spec["run"],
            ))

    # benches package
    with open(os.path.join(ROOT, "benches", "README.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write(BENCH_README)
    with open(os.path.join(ROOT, "benches", "CHANGELOG.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write(BENCH_CHANGELOG)

    if errors:
        print("ERRORS:")
        for e in errors:
            print(" -", e)
        raise SystemExit(1)
    print(f"generated docs for {len(CRATES)} crates + {len(EXAMPLES)} examples + benches")


if __name__ == "__main__":
    main()
