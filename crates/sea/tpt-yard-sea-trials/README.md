# tpt-yard-sea-trials

> Post-launch sea-trial test planning and acceptance-criteria evaluation.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-sea-trials.svg)](https://crates.io/crates/tpt-yard-sea-trials)
[![Docs.rs](https://docs.rs/tpt-yard-sea-trials/badge.svg)](https://docs.rs/tpt-yard-sea-trials)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Between launch and delivery stands the trial program: speed runs on the measured mile, manoeuvring circles, crash stops, seakeeping, noise surveys, endurance, the inclining experiment, and the class society's acceptance protocol. `tpt-yard-sea-trials` models the program and judges the measurements.

An incomplete program never passes: a trial without a measurement is an explicit failure state, which is exactly what a delivery gate needs.

## Features

- `TrialProgram` — the ordered trial list with categories from speed to class acceptance
- `Acceptance` windows on typed metrics (knots, turning-circle lengths, dB(A), GM, ...) with inclusive bounds
- `evaluate` — per-trial outcomes: passed, failed with reason, or *not performed*
- `TrialReport` — delivery-facing summary with passed/total counts

## Installation

```toml
[dependencies]
tpt-yard-sea-trials = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-sea-trials = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_sea_trials::{Acceptance, Metric, Trial, TrialProgram};

let mut program = TrialProgram::new();
program.push(Trial {
    id: 1,
    name: "Speed trial".into(),
    kind: Default::default(),
    acceptance: Acceptance { metric: Metric::SpeedKn, minimum: Some(15.0), maximum: None },
    measured: Some(15.6),
});
let report = program.evaluate();
assert!(report.all_passed);
```

## How it works

- Bounds are inclusive on both ends; a one-sided window leaves the other end `None`.
- Trial kinds map to the standard program sections; metrics carry their units in `Display` for report formatting.

## Verification

- Passing, failing and unperformed trials each tested
- Inclusive-bound edge values accepted exactly at the limit
- Inclining-experiment GM gate covered

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `sea-trials` `acceptance` `testing` `marine` `delivery` |
| Categories | `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-sea-trials` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
