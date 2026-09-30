# Introduction

`tpt-shipyard` is a fully open-source, MIT OR Apache-2.0 licensed computational
engine for **vehicle construction**: sea shipyards, orbital assembly, in-space
manufacturing, and the digital twin of the build process.

It is the construction phase of the vehicle lifecycle — it sits between design
(`tpt-transport`) and operation, managing the transformation of raw materials
into a complete vessel, whether that vessel is a container ship assembled from
hull blocks in a drydock, or a rotating habitat welded by robotic arms at
Earth-Moon L2.

## The core insight

**A spaceship built in space does not need to be a rocket.**

On Earth, every spacecraft is constrained by aerodynamic drag, Max-Q, fairing
diameter, launch vibration, and gravity sag during assembly. In orbit none of
these constraints exist. Almost no simulation tools model structures *during*
construction — where the structure is incomplete, partially welded, and subject
to completely different load cases. That gap is what this engine fills.

## How to read this book

Each chapter documents one domain crate: the types it exposes, the physics it
implements (with governing equations), and worked examples. Verification
methodology and golden data are described in the final chapter.

## Quick start

```rust
use tpt_yard_core::VesselProject;
use tpt_yard_digital_twin::DigitalTwin;

let project = VesselProject::from_json("container-ship.json").unwrap();
let mut twin = DigitalTwin::new(project);

// Advance through construction, tracking weight and CoG as you go.
// (Collect the activity ids first: advance_phase takes &mut twin.)
let activity_ids: Vec<_> = twin
    .vessel
    .build_phases
    .iter()
    .flat_map(|p| p.activities.iter().map(|a| a.id))
    .collect();
for id in &activity_ids {
    twin.advance_phase(id).unwrap();
    println!("Weight: {} kg", twin.weight_model().total_weight());
}
```
