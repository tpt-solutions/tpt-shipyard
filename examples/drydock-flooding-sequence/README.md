# Drydock flooding sequence

> Floats a 4,000 t vessel out of a 200 m building dock: level-by-level draft, displacement, GM and aground/afloat state from the as-built weight model.

**Run it:**

```bash
cargo run -p drydock-flooding-sequence
```

**Expected output:** A 13-level flooding table ending afloat with GM ~7.8 m, stable at every level.

This example is part of the [tpt-shipyard](https://github.com/tpt-solutions/tpt-shipyard)
workspace (Phase 3). It is a workspace-only package (`publish = false`) — the
publishable crates live under `crates/`.

## License

MIT OR Apache-2.0, same as the workspace.
