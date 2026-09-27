# tpt-shipyard

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

| Crate | Description | Status |
|---|---|---|
| `tpt-yard-core` | Core shipyard types: vessel projects, build phases, activities | In Progress |
| `tpt-yard-assembly` | Shared assembly-activity primitives: dependency graphs | Done |
| `tpt-yard-weight` | Weight and CoG management | Done |
| `tpt-yard-digital-twin` | Construction state tracking and simulation | Done |
| `tpt-yard-structural` | Structural analysis during build | Done |
| `tpt-yard-welding` | Welding simulation | Done |
| `tpt-yard-distortion` | Block distortion management | Done |
| `tpt-yard-joints` | Shared joint-geometry primitives | Done |
| `tpt-yard-hull` | Hull block construction | Done |
| `tpt-yard-blocks` | Block lifting and sling loads | Done |
| `tpt-yard-drydock` | Drydock flooding and ballast sequencing | Done |
| `tpt-yard-launch` | Launch calculations | Done |
| `tpt-yard-outfitting` | Systems installation and routing | Done |
| `tpt-yard-sea-trials` | Sea-trial test planning | Done |
| `tpt-yard-orbital-assembly` | Orbital assembly planning | Done |
| `tpt-yard-space-structural` | Structures without launch constraints | Done |
| `tpt-yard-space-manufacturing` | In-space manufacturing and additive construction | Done |
| `tpt-yard-robotic-assembly` | Robotic arm planning | Done |
| `tpt-yard-habitat` | Rotating habitat design | Done |
| `tpt-yard-propellant` | Propellant loading and boil-off management | Done |
| `tpt-yard-scheduling` | Construction scheduling | Done |
| `tpt-yard-logistics` | Material and resource logistics | Done |
| `tpt-yard-facility` | Shipyard facility layout planning | Done |
| `tpt-yard-quality` | Quality control and inspection | Done |
| `tpt-yard-transport-link` | Vehicle design ⇄ construction bridge | Done |
| `tpt-yard-process-link` | Process engineering bridge | Done |
| `tpt-yard-earth-link` | Weather/sea-state bridge | Done |
| `tpt-yard-wasm` | WebAssembly bindings for dashboards | Done |

## Quick Start

```rust
use tpt_yard_core::VesselProject;
use tpt_yard_digital_twin::DigitalTwin;

fn main() {
    let project = VesselProject::from_json("container-ship.json").unwrap();
    let mut twin = DigitalTwin::new(project);

    // Advance through construction phases
    for activity in twin.vessel().build_phases[0].activities.clone() {
        twin.advance_phase(&activity.id).unwrap();
        println!("Weight: {} kg", twin.weight_model().total_weight());
    }
}
```

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
2. **MIT-only dependency chain** — `cargo-deny` enforces the allow-list on every PR.
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
