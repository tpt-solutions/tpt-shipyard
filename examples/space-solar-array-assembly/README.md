# Space solar array assembly

> Plans in-space additive manufacturing of a solar array end-to-end: wire-arc print times and energy per panel substrate, vacuum heat rejection, in-situ quality plan, then robotic assembly of the printed panels into the deployed wing.

**Run it:**

```bash
cargo run -p space-solar-array-assembly
```

**Expected output:** Per-panel print/energy figures, a 48-step assembly with all constraints ok, and the deployed-wing integrity report.

This example is part of the [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
workspace (Phase 5 milestone). It is a workspace-only package (`publish = false`) — the
publishable crates live under `crates/`.

## License

MIT OR Apache-2.0, same as the workspace.
