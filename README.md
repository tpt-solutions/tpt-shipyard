# tpt-shipyard

[![CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-shipyard#license) [![docs.rs](https://img.shields.io/docsrs/tpt-yard-core)](https://docs.rs/tpt-yard-core) [![MSRV 1.98](https://img.shields.io/badge/MSRV-1.98-orange)](https://github.com/tpt-solutions/tpt-shipyard)

A fully open-source, MIT-licensed computational engine for vehicle construction: sea shipyards, orbital assembly, in-space manufacturing, and construction digital twins.

**Build Anything, Anywhere. Sea or Space. No Gravity Constraints. No Proprietary Lock-in.**

```text
┌─────────────────────────────────────────────────────────────┐
│              THE VEHICLE LIFECYCLE STACK                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │  tpt-    │  │  tpt-    │  │  tpt-    │  │  (end of │    │
│  │transport │→ │ shipyard │→ │transport │→ │   life)  │    │
│  │ (design) │  │ (build)  │  │(operate) │  │          │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
│                                                             │
│         SEA            SPACE           BOTH                 │
│    Drydock / Slipway   OrbitalAssembly  Digital Twin        │
│    Launch / Outfit     In-space Mfg     Weight & CoG        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Crates

Every crate ships its own `README.md` (overview, features, usage, verification)
and `CHANGELOG.md`; crates.io `keywords` and `categories` are declared in each
manifest. See for example
[`crates/structural/tpt-yard-welding`](crates/structural/tpt-yard-welding).

| Crate | Description | Status |
|---|---|---|

Status legend: **Done** = implemented and verified against analytical/golden
cases. **Simplified model** = working, but with documented modelling
simplifications (see each crate's README — several are roadmap items to
deepen). **Stand-in** = a thin, honest bridge awaiting the companion
`tpt-transport` / `tpt-process` / `tpt-earth` substrates.

| `tpt-yard-core` | Core shipyard types: vessel projects, build phases, activities | Done |
| `tpt-yard-assembly` | Shared assembly-activity primitives: dependency graphs | Done |
| `tpt-yard-weight` | Weight and CoG management | Done |
| `tpt-yard-digital-twin` | Construction state tracking and simulation | Done |
| `tpt-yard-structural` | Structural analysis during build | Simplified model (3-D truss + 2-D frame FEM, penalty BCs, sparse CG) |
| `tpt-yard-welding` | Welding simulation | Done (analytical Rosenthal, experimentally verified) |
| `tpt-yard-distortion` | Block distortion management | Simplified model (threshold-based correction planning) |
| `tpt-yard-joints` | Shared joint-geometry primitives | Done (closed-form groove/fillet geometry) |
| `tpt-yard-hull` | Hull block construction | Done (pre-design weight estimate) |
| `tpt-yard-blocks` | Block lifting and sling loads | Done |
| `tpt-yard-drydock` | Drydock flooding and ballast sequencing | Simplified model (rectangular-block hydrostatics) |
| `tpt-yard-launch` | Launch calculations | Simplified model (statics screening + dynamic slipway simulation; stern-lift/poppet load split on roadmap) |
| `tpt-yard-outfitting` | Systems installation and routing | Simplified model (AABB routing checks) |
| `tpt-yard-sea-trials` | Sea-trial test planning | Done |
| `tpt-yard-hydrostatics` | Hydrostatics, GZ curves, IMO 2008 stability | Simplified model (prismatic screening; Bonjean/damage stability on roadmap) |
| `tpt-yard-orbital-assembly` | Orbital assembly planning | Simplified model (greedy sequence, kinematic checks) |
| `tpt-yard-space-structural` | Structures without launch constraints | Simplified model (analytical screening) |
| `tpt-yard-space-manufacturing` | In-space manufacturing and additive construction | Simplified model (estimate-grade constants) |
| `tpt-yard-robotic-assembly` | Robotic arm planning | Simplified model (planar RRT, 2-4 link arms) |
| `tpt-yard-habitat` | Rotating habitat design | Simplified model (analytical shell stress) |
| `tpt-yard-propellant` | Propellant loading and boil-off management | Simplified model (lump-parameter thermal) |
| `tpt-yard-scheduling` | Construction scheduling | Done (exact CPM; heuristic levelling) |
| `tpt-yard-logistics` | Material and resource logistics | Simplified model (flow accounting) |
| `tpt-yard-facility` | Shipyard facility layout planning | Done (plan-view capacity/placement) |
| `tpt-yard-quality` | Quality control and inspection | Simplified model (plan generation only) |
| `tpt-yard-transport-link` | Vehicle design ⇄ construction bridge | Stand-in (awaits the `tpt-transport` substrate) |
| `tpt-yard-process-link` | Process engineering bridge | Stand-in (awaits the `tpt-process` substrate) |
| `tpt-yard-earth-link` | Weather/sea-state bridge | Stand-in (awaits the `tpt-earth` substrate) |
| `tpt-yard-cli` | CLI: `validate` / `plan` / `schedule` / `report` / `new` / `html-report` | Done |
| `tpt-yard` | Facade: one dependency, curated prelude, feature flags | Done |
| `tpt-yard-wasm` | WebAssembly bindings for dashboards | Simplified model (browser tests in CI; 3-D three.js dashboard shipped) |

## Quick Start

The easiest way in is the [`tpt-yard`](crates/facade/tpt-yard) facade — one
dependency with a curated prelude and `sea`/`space`/`planning`/`wasm`
feature flags. Or depend on `tpt-yard-digital-twin` directly and define the
erection plan:

```rust
use tpt_yard_assembly::ActivityId;
use tpt_yard_core::{ActivityType, AssemblyActivity, BuildPhase, ConstructionMethod,
    PhaseId, ProjectId, SeaVesselType, VesselProject, VesselType};
use tpt_yard_digital_twin::DigitalTwin;

// One erection activity per block; dependencies gate the order.
let mut phase = BuildPhase::new(PhaseId(1), "Erection", 4.0);
for i in 1..=3u64 {
    let deps: Vec<ActivityId> = if i == 1 { vec![] } else { vec![ActivityId(i - 1)] };
    phase.activities.push(
        AssemblyActivity::new(ActivityId(i), format!("Erect block {i}"), ActivityType::JoinBlock, 6.0)
            .with_dependencies(&deps),
    );
}

let project = VesselProject::new(
    ProjectId(1),
    "Demo barge",
    VesselType::Sea(SeaVesselType::ContainerShip { teu_capacity: 800 }),
    ConstructionMethod::SeaDrydock,
    vec![phase],
)
.unwrap();
let mut twin = DigitalTwin::new(project);

// Collect the ids first: `advance_phase` takes `&mut twin`, and the
// activity list borrows `twin.vessel`.
let ids: Vec<ActivityId> = twin
    .vessel
    .build_phases
    .iter()
    .flat_map(|p| p.activities.iter().map(|a| a.id))
    .collect();
for id in &ids {
    twin.advance_phase(id).expect("activity must be sound to run");
}
println!(
    "{}/{} activities complete, phase {:?}",
    twin.assembly_state.completed_activities.len(),
    ids.len(),
    twin.vessel.current_phase
);
```

`DigitalTwin::new` tracks the plan and structure; pass a populated
[`WeightModel`](https://docs.rs/tpt-yard-weight) to
`DigitalTwin::with_weight_model` (wiring each item's `installed_by` to an
activity) and the twin also tracks installed mass and CoG as erection
proceeds — see the `tpt-yard-digital-twin` docs and the worked examples.

Run a complete worked example with
`cargo run -p container-ship-block-assembly`; sample vessel projects ship in
`test-data/`. The `examples/` directory covers block assembly, a submarine
pressure hull, drydock flooding, orbital truss assembly, a rotating habitat,
and in-space solar-array manufacturing.

## Build Anything, Anywhere

`tpt-shipyard` supports:

- **Sea shipyards** — block construction, drydock flooding, slipway launch, outfitting, sea trials
- **Orbital assembly** — robotic, microgravity, constraint-checked assembly sequences
- **In-space manufacturing** — additive construction, ISRU feedstock, print planning
- **Rotating habitats** — O'Neill cylinders, Stanford torus, Bernal spheres

No gravity constraints. No aerodynamic limits. No proprietary lock-in.

### A spacecraft built in space is not a rocket

On Earth, every spacecraft is constrained by aerodynamic drag, Max-Q, fairing diameter
limits, launch vibration, and gravity sag during horizontal assembly. In orbit, none of
these constraints exist. `tpt-shipyard` is the engine that plans, simulates, and validates
construction — whether in a drydock in Busan or a robotic assembly bay at Earth-Moon L2.

## Design Principles

1. **100% open source** — MIT OR Apache-2.0, no open-core, no proprietary file formats.
2. **Permissive dependency chain** — the crates are `MIT OR Apache-2.0`; `cargo-deny` enforces the allow-list (MIT / Apache-2.0 / BSD / ISC / Zlib / Unicode) and denies copyleft on every PR.
3. **Pure Rust** — zero heavy external dependencies in the core crates; compiles to
   WebAssembly for interactive dashboards.
4. **Physics you can audit** — every solver documents its governing equations and is
   verified against analytical cases and golden data (`test-data/golden/`).

## Documentation

- The Book: `docs/book` (mdBook) — run `mdbook build docs/book` or read the sources in `docs/book/src/`.
- RFCs: `rfcs/` with the process index in `docs/rfc/index.md`.
- API docs: `cargo doc --workspace --open`.

## Governance

Benevolent Dictator + public RFC process. See [GOVERNANCE.md](GOVERNANCE.md).
Releases follow SemVer on a 6-week cadence — see [RELEASES.md](RELEASES.md).
Roadmap tracking happens on the public GitHub Projects board linked from the repository.

## Contributing

Contributions are welcome — CLA-free, DCO sign-off only. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

Please report vulnerabilities privately — see [SECURITY.md](SECURITY.md).

## License

Licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

The "TPT Shipyard" name and logo are trademarks of TPT Solutions — see [GOVERNANCE.md](GOVERNANCE.md).
