# tpt-yard-earth-link

> Bridge between weather and sea-state data (tpt-earth) and launch planning.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-earth-link.svg)](https://crates.io/crates/tpt-yard-earth-link)
[![Docs.rs](https://docs.rs/tpt-yard-earth-link/badge.svg)](https://docs.rs/tpt-yard-earth-link)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

No launch officer floats a 4,000-tonne ship down greased ways into a rising sea. `tpt-yard-earth-link` gates launch methods on the Douglas sea-state forecast and returns the safe window — first safe hour, duration, and the governing limit per method.

The `tpt-earth` forecast type is vendored behind the shape the substrate will use; the gating logic is the contract.

## Features

- `plan_launch_window` — the longest calm run in the forecast for the launch method
- Per-method screening limits: side launch 1, slipway/shiplift 2, dock flooding 3 (Douglas scale)
- Never-safe forecasts are a typed error, never an empty window
- `LaunchWindow` with first hour, exclusive end, limit, calm hours, and a plain-language note

## Installation

```toml
[dependencies]
tpt-yard-earth-link = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-earth-link = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::{MassProperties, Vector3};
use tpt_yard_earth_link::{plan_launch_window, SeaStateForecast};
use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions};

let forecast = SeaStateForecast {
    hourly_sea_state: (0..24).map(|h| if (9..17).contains(&h) { 4.0 } else { 1.0 }).collect(),
};
let window = plan_launch_window(&analysis, &forecast)?;
println!("launch between hours {} and {}", window.earliest_hour, window.latest_hour);
```

## How it works

- Limits are yard-practice screening values per launch method; the site's own `max_sea_state` may tighten them further.
- The window is the first (and longest) run of hours at or below the limit — deterministic and forecast-stable.

## Verification

- A mid-day storm pushes the window to the longer calm run
- The same forecast clears dock flooding but blocks a side launch (method limits differ)
- Never-safe and empty forecasts are typed errors

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `sea-state` `launch-window` `weather` `douglas-scale` `marine` |
| Categories | `science` `simulation` `aerospace` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-earth-link` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
