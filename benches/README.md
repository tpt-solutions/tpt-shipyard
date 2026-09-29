# tpt-shipyard benchmarks

Timing benches over the engine's hot paths, written against `std::time::Instant`
with `harness = false` so they run on stable without criterion:

| Bench | What it measures |
|---|---|
| `block-lifting` | Sling-load distribution (12 legs) and hull division + erection planning at 140/300/400 m |
| `welding-distortion` | Rosenthal thermal cycles, residual stress, distortion, and sequence ranking on the reference AH36 panel |
| `orbital-assembly-sequence` | Sequence planning, full 15-bay build simulation, and robot path planning around installed structure |
| `launch-stability` | Slipway statics (safe and tip-up cases), 13-level dock flooding, weight-model stability |

**Run:**

```bash
cargo bench -p tpt-shipyard-benches
cargo bench -p tpt-shipyard-benches --bench welding-distortion
```

All hot paths are microsecond-scale — comfortably inside an interactive
digital-twin budget. CI runs the same benches via
`.github/workflows/benchmark.yml` and uploads the timing logs as artifacts.

This is a workspace-only package (`publish = false`); the publishable crates
live under `crates/`. Licensed MIT OR Apache-2.0 like the workspace.
