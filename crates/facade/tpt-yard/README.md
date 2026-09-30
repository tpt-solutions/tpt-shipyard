# tpt-yard

The one-dependency facade for [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard):
every domain crate re-exported as a module, plus a curated [`prelude`] with the
types a construction plan is written with.

## Features

- **default** = `sea` + `space` + `planning`
- `sea` — hull blocks, lifting, drydock, launch, outfitting, sea trials
- `space` — orbital assembly, space structures, robotic arms, habitats, propellant, manufacturing
- `planning` — scheduling, logistics, facility layout, quality management
- `wasm` — browser-facing WebAssembly bindings (native-compilable too)

## Usage

```rust
use tpt_yard::prelude::*;

let phase = BuildPhase::new(PhaseId(1), "Erection", 4.0);
let project = VesselProject::new(
    ProjectId(1),
    "Barge",
    VesselType::Sea(SeaVesselType::ContainerShip { teu_capacity: 800 }),
    ConstructionMethod::SeaDrydock,
    vec![phase],
)
.unwrap();
let twin = DigitalTwin::new(project);
```

See the [workspace README](https://github.com/tpt-solutions/tpt-shipyard) for the
crate map and status legend.
