# tpt-yard-process-link

> Bridge between process engineering (tpt-process) and construction planning.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-process-link.svg)](https://crates.io/crates/tpt-yard-process-link)
[![Docs.rs](https://docs.rs/tpt-yard-process-link/badge.svg)](https://docs.rs/tpt-yard-process-link)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Propellant loading is thermodynamics with a schedule. `tpt-yard-process-link` vendors a minimal Peng-Robinson equation of state (the spec section 6 contract) and wires it into the propellant loading planner: the EOS computes ullage vapour densities at loading conditions, and the loading plan says what to do about them.

When the `tpt-process` substrate publishes, the vendored EOS yields to it — the planning API is the contract.

## Features

- `PengRobinson` — single-component EOS with the classical constants, cubic root solve, vapour and liquid roots
- `vapour_density` at (T, P) from the compressibility factor
- EOS parameter table for LOX, LH2, LCH4, MMH, NTO
- `plan_propellant_loading` — the operational plan plus the EOS-derived venting decision

## Installation

```toml
[dependencies]
tpt-yard-process-link = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-process-link = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_process_link::PengRobinson;

// Methane at 111 K and 0.1 MPa: near-ideal vapour.
let eos = PengRobinson::new(190.6, 4.599e6, 0.011);
let z = eos.compressibility(111.0, 0.1e6);
assert!((z - 0.97).abs() < 0.05);

let rho = eos.vapour_density(111.0, 0.1e6, 16.04);
```

## How it works

- PR constants: `a = 0.45724 R^2 Tc^2 / Pc`, `b = 0.07780 R Tc / Pc`, kappa from the acentric factor; alpha per temperature.
- The compressed-liquid region (T < Tc, P > Pc) returns the liquid root; otherwise the vapour root.
- Venting by EOS: dense ullage vapour (> 1 kg/m3 at loading conditions) means vent capacity governs the fill.

## Verification

- Methane Z near unity at low pressure; sub-unity near critical pressure
- LOX loading plan requires venting and chilldown, agreeing with the EOS decision
- Unknown propellants rejected with a typed error

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `peng-robinson` `eos` `propellant` `thermodynamics` `cryogenic` |
| Categories | `science` `simulation` `aerospace` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-process-link` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
