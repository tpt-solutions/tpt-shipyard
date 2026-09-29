# Submarine pressure hull

> Ring-section planning for a 55 m pressure hull: circumferential double-V seam geometry, SAW procedure thermal cycle and shrinkage, and a workshop cradle FEM check with hoisting loads.

**Run it:**

```bash
cargo run -p submarine-pressure-hull
```

**Expected output:** Seam weld-metal mass, HAZ t8/5 timing, and a cradle utilization report (OK / OVERSTRESSED).

This example is part of the [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
workspace (Phase 3). It is a workspace-only package (`publish = false`) — the
publishable crates live under `crates/`.

## License

MIT OR Apache-2.0, same as the workspace.
