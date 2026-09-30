# WASM Dashboard Integration

`tpt-yard-wasm` compiles the construction engine to WebAssembly for
interactive dashboards (spec §7).

## The façades

- [`WasmDigitalTwin`](tpt_yard_wasm::WasmDigitalTwin) — loads a
  `VesselProject` JSON, erects blocks (`advance_next`, `advance_phase`),
  serves geometry (`get_geometry` returns a flat triangle soup ready for a
  WebGL buffer; `get_geometry_indices` the index buffer), and reports
  weight/CoG and structural checks as JSON strings.
- [`WasmOrbitalAssembly`](tpt_yard_wasm::WasmOrbitalAssembly) — simulates a
  truss build step by step (`simulate_next_step`) and exposes the robot pose
  and installed components for rendering.

## Building

```bash
cargo build -p tpt-yard-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir www/pkg \
    target/wasm32-unknown-unknown/release/tpt_yard_wasm.wasm
python -m http.server 8123 --directory www
# open http://localhost:8123
```

## The reference dashboard

`www/index.html` is the working construction dashboard: erect 12 hull blocks
one at a time or run to completion, watch the WASM mesh grow in 3-D (three.js:
orbit, zoom and pan; the camera follows the erection until you take over),
the as-built CoG marker march midship, keel reactions update, and the
structural check stay green — all computed in WebAssembly. The geometry and
index buffers from `get_geometry`/`get_geometry_indices` upload straight into
a three.js `BufferGeometry`; three.js loads from a CDN import map, and if it
cannot load the page says so instead of failing silently.

## Verification

The façades unit-test on native (wasm-bindgen degrades gracefully): JSON
round-trip, dependency-gated advance, geometry buffer sizes, and the
orbital façade's full 18-step sequence.
