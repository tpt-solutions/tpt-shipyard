# tpt-yard-quality

> Quality control and inspection: NDT plans and defect tracking.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-quality.svg)](https://crates.io/crates/tpt-yard-quality)
[![Docs.rs](https://docs.rs/tpt-yard-quality/badge.svg)](https://docs.rs/tpt-yard-quality)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Class societies do not take your word for it, and neither does this crate. `tpt-yard-quality` turns the build plan into an NDT inspection plan — every weld-bearing activity gets a method and coverage by criticality — and tracks findings through disposition into a repair-rate report.

## Features

- `generate_inspection_plan` — activity type drives the method: structural butts get UT at criticality I, piping gets PT at II, pressure tests get PRESS, everything else visual at III
- Coverage per point (100 % for pressure boundaries and full-penetration welds)
- `defect_tracking` — counts by type, dominant defect, repair rate, rejections to engineering
- `NdtMethod` with standard codes (UT, RT, MT, PT, ET, VT, VAC, PRESS)
- `AcceptanceCriteria` referencing the governing standard and maximum indication

## Installation

```toml
[dependencies]
tpt-yard-quality = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-quality = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{ActivityId, ActivityType, BuildPhase, PhaseId};
use tpt_yard_quality::{NdtMethod, QualityManagement};

let mut phase = BuildPhase::new(PhaseId(1), "Panel line", 5.0);
phase.activities.push(tpt_yard_core::AssemblyActivity::new(
    ActivityId(1), "Weld butt seam", ActivityType::WeldBlock, 8.0,
));

let qm = QualityManagement::default();
let plan = qm.generate_inspection_plan(&[phase]);
assert_eq!(plan[0].method, NdtMethod::UltrasonicTesting);
```

## How it works

- Method selection mirrors class practice: full-penetration structural welds are volumetric (UT), fillets and piping surface (PT/MT), tests become their own inspection point.
- Defect aggregation sorts by count then name, so reports are stable across runs.
- Repair rate counts `Repair` and `RepairAndReinspect` over total findings.

## Verification

- Golden: `ndt-inspection-plan.json` — method/criticality/coverage per activity
- Golden: `weld-defect-tracking.json` — aggregation, dominant defect, repair rate
- Empty-tracking and boundary behaviours tested

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `quality` `ndt` `inspection` `defect-tracking` `welding` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-quality` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
