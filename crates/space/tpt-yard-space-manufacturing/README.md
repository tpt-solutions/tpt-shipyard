# tpt-yard-space-manufacturing

> In-space manufacturing and additive construction planning.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/tpt-yard-space-manufacturing.svg)](https://crates.io/crates/tpt-yard-space-manufacturing)
[![Docs.rs](https://docs.rs/tpt-yard-space-manufacturing/badge.svg)](https://docs.rs/tpt-yard-space-manufacturing)
[![Workspace CI](https://github.com/tpt-solutions/tpt-shipyard/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-shipyard/actions)

Printing structure in vacuum changes the economics: feedstock mass is launch mass (unless it comes from regolith), deposition power has nowhere to go but radiation, and destructive testing may not be able to come home. `tpt-yard-space-manufacturing` plans all of it — print time, energy, heat rejection, and the in-situ quality plan — and chains into the orbital assembly planner so manufacturing and construction are one pipeline.

## Features

- `print_time_estimate` — feedstock mass (with ISRU scrap) over the deposition rate
- Process energy intensities from powder-bed (12 kWh/kg) to wire-arc (4 kWh/kg)
- `thermal_control_during_print` — deposition power and radiator area from the Stefan-Boltzmann law (eps 0.85, 350 K), eclipse pauses flagged
- `quality_verification` — 100 % layer imaging, vacuum NDT subset, witness coupons, destructive testing only if samples come home
- `Feedstock` from Earth launch or ISRU sources (lunar/martian regolith, asteroid metal, recycled debris)

## Installation

```toml
[dependencies]
tpt-yard-space-manufacturing = "0.1"
```

Until the workspace's first crates.io release, depend on it via git:

```toml
tpt-yard-space-manufacturing = { git = "https://github.com/tpt-solutions/tpt-shipyard" }
```

## Usage

```rust
use tpt_yard_core::Material;
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
let thermal = printer.thermal_control_during_print(6.0)?;
```

## How it works

- Print time: `t = rho V (1 + scrap) / rate`; ISRU feedstock carries a processing scrap fraction.
- Radiators reject by radiation only: `A = P / (eps sigma T^4)`; a 40 kW deposition needs ~33 m2 at 350 K.
- Energy intensity is process-typical and documented; swap in measured values as they arrive.

## Verification

- Print time equals mass over rate, including the ISRU scrap factor
- Radiator area matches the closed-form Stefan-Boltzmann result
- ISRU builds flagged non-destructive-testing; error paths covered

## Crate metadata

| Field | Value |
|---|---|
| Keywords | `additive-manufacturing` `isru` `in-space-manufacturing` `aerospace` `deposition` |
| Categories | `aerospace` `science` `simulation` |
| Version | 0.1.0 (workspace release train) |
| License | MIT OR Apache-2.0 |
| Workspace | [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard) |

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## Part of the tpt-shipyard engine

`tpt-yard-space-manufacturing` is one of the 28 crates of [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
— a fully open-source computational engine for vehicle construction: sea
shipyards, orbital assembly, in-space manufacturing, and construction digital
twins. See the workspace [README](https://github.com/tpt-solutions/tpt-shipyard#readme)
for the full crate map and the [book](https://github.com/tpt-solutions/tpt-shipyard/tree/main/docs/book)
for the physics behind every model.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or [Apache-2.0](../../LICENSE-APACHE)
at your option.
