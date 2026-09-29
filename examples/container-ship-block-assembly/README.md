# Container ship block assembly

> Plans the erection of a 1,400 TEU container ship end-to-end: crane- and workshop-constrained block division, the midship-outward erection sequence, and a digital-twin run through every phase with live weight/CoG tracking and keel-block reactions.

**Run it:**

```bash
cargo run -p container-ship-block-assembly
```

**Expected output:** A 12-block erection table (installed tonnage, CoG, keel reactions) and a completion summary at 1,714 t with zero deviation.

This example is part of the [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
workspace (Phase 3 milestone). It is a workspace-only package (`publish = false`) — the
publishable crates live under `crates/`.

## License

MIT OR Apache-2.0, same as the workspace.
