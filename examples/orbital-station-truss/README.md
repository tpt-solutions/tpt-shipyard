# Orbital station truss assembly

> Simulates a robotic truss build: sequence planning (grasp, translate, rotate, dock, bolt, release per bay), step simulation with collision and force-limit checks, and partial-structure integrity at every bay release.

**Run it:**

```bash
cargo run -p orbital-station-truss
```

**Expected output:** A 36-step table for 6 bays with root stress per step, all constraints green, ending at the deployed-cantilever integrity report.

This example is part of the [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
workspace (Phase 4 milestone). It is a workspace-only package (`publish = false`) — the
publishable crates live under `crates/`.

## License

MIT OR Apache-2.0, same as the workspace.
