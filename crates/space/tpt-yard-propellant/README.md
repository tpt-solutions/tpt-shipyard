# tpt-yard-propellant

> Propellant loading and boil-off management for space vessels.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-propellant.svg)](https://crates.io/crates/tpt-yard-propellant)
[![Docs.rs](https://docs.rs/tpt-yard-propellant/badge.svg)](https://docs.rs/tpt-yard-propellant)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Loading cryogen into a depot tank is a thermal problem wearing a plumbing hat: chill down first or the fill flashes, watch the vent capacity, and every hour on orbit leaks heat that becomes boil-off. `tpt-yard-propellant` plans the load and manages the leak — from LOX and LH2 to storables like MMH and NTO.

Zero-boil-off is a qualification, not a slogan: a tank earns the ZBO label by holding its heat leak under 0.1 % of load per day, and the planner tells you whether yours qualifies.

## Features

- `PropellantSpec` presets: LOX, LH2, LCH4, MMH, NTO with density, latent heat, boiling point
- `LoadingPlanner::plan_loading` — fill rate, chilldown time (tank thermal mass vs latent heat), venting requirement, subcooling target
- `boil_off_report` — kg/day and %/day boil-off, time-to-vent, cryocooler power for ZBO
- `BoilOffPolicy` — Vented, Recooled, or ZeroBoilOff, assigned by qualification
- Storable propellants: no chilldown, no venting, ZBO by construction

## Installation

```toml
[dependencies]
tpt-yard-propellant = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-propellant = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_propellant::{LoadingPlanner, PropellantSpec, TankSpec};

let planner = LoadingPlanner::new();
let tank = TankSpec { volume_m3: 300.0, ullage_frac: 0.03, heat_leak_w: 2_000.0 };

let plan = planner.plan_loading(&PropellantSpec::lox(), &tank, 50.0)?;
assert!(plan.venting_required);            // LOX is cryogenic
assert!(plan.thermal_control.chilldown_required);

let boil_off = planner.boil_off_report(&PropellantSpec::lox(), &tank);
println!("{:.1} kg/day ({:.3} %/day)", boil_off.boil_off_kg_day, boil_off.boil_off_pct_day);
```

## How it works

- Boil-off converts the heat leak through the latent heat: `kg/day = W * 86400 / (L * 1000)`.
- Chilldown consumes ~80 % of the tank thermal-mass cooling demand as vapour, timed at the fill rate.
- The ZBO threshold is 0.1 %/day of load through latent heat; the cryocooler is sized from a specific-power ratio (100:1 default).

## Verification

- Cryogenic stability test balances 2000 W against LOX latent heat (811 kg/day, vented policy)
- LH2 verified as the worst percentage boil-off (low density, modest latent heat)
- ZBO qualification tested both sides of the threshold

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `propellant` `cryogenic` `boil-off` `zero-boil-off` `depot` |
| Categories | `aerospace` `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-propellant` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
