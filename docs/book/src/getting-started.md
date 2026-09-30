# Getting Started in 10 Minutes

This chapter takes you from `cargo install` to a full construction plan for
a 140 m container ship. Every output shown is what the current workspace
actually prints (regenerate with the shown commands if in doubt).

## 1. Build the workspace (2 min)

```sh
git clone https://github.com/tpt-solutions/tpt-shipyard
cd tpt-shipyard
cargo build --release
```

Or with the justfile: `just check` formats, lints and tests everything.

## 2. Run a worked example (1 min)

```sh
cargo run -p container-ship-block-assembly
```

Expected output (truncated):

```text
== container-ship block assembly ==
12 blocks, 6854.4 t total steel
erection: 12 joins, tier 0 first, midship outwards
...
```

The other examples: `submarine-pressure-hull`, `drydock-flooding-sequence`,
`orbital-station-truss` (reads its manifest from
`test-data/orbital-structures/`), `rotating-habitat-construction`,
`space-solar-array-assembly`.

## 3. Use the CLI (2 min)

```sh
cargo run -p tpt-yard-cli -- plan test-data/hull-blocks/container-ship-140m.json
```

Expected output:

```text
Construction plan for Container ship 1400 TEU (reference block-division case)
======================================================
1. Block division: 12 blocks, 6854 t total steel
2. Erection order: 12 joins; first block on the dock floor, tiers bottom-up
3. Lift check (heaviest block 571 t, 4-point pick): ...
4. Schedule: makespan 40.0 h ..., critical path 5 activities
5. Drydock/launch: ...
```

(The lift check reports *exactly* what the statics say — with a 4 m hook
height on a 22 m wide block the slings are shallow, and the tool says so.
Raise `hook height` in your own manifest to clear the 30° guideline.)

Scaffold a new project from a reference template:

```sh
cargo run -p tpt-yard-cli -- new container-ship > project.json
cargo run -p tpt-yard-cli -- validate project.json
# OK: project 'Container ship 1400 TEU' (4 phases, 7 activities) passes validation
```

Templates: `container-ship`, `submarine`, `orbital-truss`, `habitat`,
`solar-array` (the JSON files also sit in `templates/` for
`cargo-generate`-style workflows).

## 4. Drive the twin from Rust (3 min)

Add the facade crate:

```sh
cargo add tpt-yard
```

```rust
use tpt_yard::prelude::*;

let mut phase = BuildPhase::new(PhaseId(1), "Erection", 4.0);
phase.activities.push(
    AssemblyActivity::new(ActivityId(1), "Erect block 1", ActivityType::JoinBlock, 6.0),
);
let project = VesselProject::new(
    ProjectId(1),
    "My barge",
    VesselType::Sea(SeaVesselType::ContainerShip { teu_capacity: 800 }),
    ConstructionMethod::SeaDrydock,
    vec![phase],
)
.unwrap();
let mut twin = DigitalTwin::new(project);
twin.advance_phase(&ActivityId(1)).unwrap();
println!("{:?}", twin.assembly_state.current_mass_properties);
```

`DigitalTwin::to_json()` snapshots the whole session (plan, weight model,
progress) and `DigitalTwin::from_json_value` restores it.

## 5. Where to go next

- **Crate map & status legend**: the workspace README.
- **Verification**: every solver is checked against analytical cases and
  golden data — see [Verification & Golden Data](verification.md).
- **Domain chapters**: hull blocks, welding, orbital assembly, scheduling,
  and the rest in the summary.
- **RFCs**: `rfcs/` records why each model looks the way it does.
