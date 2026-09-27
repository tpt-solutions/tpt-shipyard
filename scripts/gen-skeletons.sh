#!/usr/bin/env bash
# One-off generator for the Phase 0 crate skeletons.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# dir|crate|description
CRATES="
crates/core/tpt-yard-core|tpt-yard-core|Fundamental shipyard domain types for the TPT Shipyard construction engine.
crates/core/tpt-yard-assembly|tpt-yard-assembly|Shared assembly-activity primitives: dependency graphs and activity status tracking.
crates/core/tpt-yard-weight|tpt-yard-weight|Weight and centre-of-gravity management for vessels under construction.
crates/core/tpt-yard-digital-twin|tpt-yard-digital-twin|Construction state tracking and simulation: the shipyard digital twin.
crates/core/tpt-yard-wasm|tpt-yard-wasm|WebAssembly bindings for interactive shipyard construction dashboards.
crates/structural/tpt-yard-structural|tpt-yard-structural|Structural analysis of incomplete structures at every build phase.
crates/structural/tpt-yard-welding|tpt-yard-welding|Welding simulation and distortion control for ship construction.
crates/structural/tpt-yard-distortion|tpt-yard-distortion|Block distortion management: deviation maps and correction planning.
crates/structural/tpt-yard-joints|tpt-yard-joints|Shared joint-geometry primitives for structural and welding analysis.
crates/sea/tpt-yard-hull|tpt-yard-hull|Hull construction and block management for sea shipyards.
crates/sea/tpt-yard-blocks|tpt-yard-blocks|Shared block-lifting and handling primitives: lift points and sling loads.
crates/sea/tpt-yard-drydock|tpt-yard-drydock|Drydock flooding and ballast sequencing with stability at every water level.
crates/sea/tpt-yard-launch|tpt-yard-launch|Launch calculations: slipway, shiplift, drydock flooding and side launch.
crates/sea/tpt-yard-outfitting|tpt-yard-outfitting|Systems installation and routing for vessel outfitting.
crates/sea/tpt-yard-sea-trials|tpt-yard-sea-trials|Post-launch sea-trial test planning and acceptance criteria.
crates/space/tpt-yard-orbital-assembly|tpt-yard-orbital-assembly|Orbital assembly planning and simulation for space structures.
crates/space/tpt-yard-space-structural|tpt-yard-space-structural|Structural design without launch constraints: vacuum, rotation, thermal cycling.
crates/space/tpt-yard-space-manufacturing|tpt-yard-space-manufacturing|In-space manufacturing and additive construction planning.
crates/space/tpt-yard-robotic-assembly|tpt-yard-robotic-assembly|Robotic arm kinematics and collision-free path planning for space assembly.
crates/space/tpt-yard-habitat|tpt-yard-habitat|Rotating habitat design: artificial gravity, Coriolis comfort, structure.
crates/space/tpt-yard-propellant|tpt-yard-propellant|Propellant loading and boil-off management for space vessels.
crates/planning/tpt-yard-scheduling|tpt-yard-scheduling|Construction scheduling: critical path, optimisation and resource levelling.
crates/planning/tpt-yard-logistics|tpt-yard-logistics|Material and resource logistics: delivery, staging and transport scheduling.
crates/planning/tpt-yard-facility|tpt-yard-facility|Shipyard and orbital-facility layout planning with capacity constraints.
crates/planning/tpt-yard-quality|tpt-yard-quality|Quality control and inspection: NDT plans and defect tracking.
crates/integration/tpt-yard-transport-link|tpt-yard-transport-link|Bridge between vehicle design (tpt-transport) and shipyard construction.
crates/integration/tpt-yard-process-link|tpt-yard-process-link|Bridge between process engineering (tpt-process) and construction planning.
crates/integration/tpt-yard-earth-link|tpt-yard-earth-link|Bridge between weather and sea-state data (tpt-earth) and launch planning.
"

for row in $CRATES; do
  dir="${row%%|*}"
  rest="${row#*|}"
  crate="${rest%%|*}"
  desc="${rest#*|}"
  mkdir -p "$ROOT/$dir/src"
  cat > "$ROOT/$dir/Cargo.toml" <<EOF
[package]
name = "$crate"
description = "$desc"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true

[dependencies]
EOF
  cat > "$ROOT/$dir/src/lib.rs" <<EOF
//! $desc
EOF
done

# Example packages (each an own workspace member).
EXAMPLES="container-ship-block-assembly orbital-station-truss rotating-habitat-construction submarine-pressure-hull space-solar-array-assembly drydock-flooding-sequence"
for ex in $EXAMPLES; do
  mkdir -p "$ROOT/examples/$ex/src"
  cat > "$ROOT/examples/$ex/Cargo.toml" <<EOF
[package]
name = "$ex"
description = "TPT Shipyard example: $ex"
publish = false
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
EOF
  cat > "$ROOT/examples/$ex/src/main.rs" <<EOF
//! TPT Shipyard example: $ex (implemented in a later phase).
fn main() {
    println!("$ex: not implemented yet");
}
EOF
done

# Benchmarks package.
mkdir -p "$ROOT/benches/benches"
cat > "$ROOT/benches/Cargo.toml" <<'EOF'
[package]
name = "tpt-shipyard-benches"
description = "Benchmarks for the TPT Shipyard construction engine"
publish = false
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true

[[bench]]
name = "block-lifting"
harness = false

[[bench]]
name = "welding-distortion"
harness = false

[[bench]]
name = "orbital-assembly-sequence"
harness = false

[[bench]]
name = "launch-stability"
harness = false
EOF
for b in block-lifting welding-distortion orbital-assembly-sequence launch-stability; do
  cat > "$ROOT/benches/benches/$b.rs" <<EOF
//! Benchmark: $b (implemented in a later phase).
fn main() {
    println!("$b: not implemented yet");
}
EOF
done

echo "skeletons generated"
