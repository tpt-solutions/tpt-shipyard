# tpt-yard-py

Python bindings for tpt-shipyard (PyO3, first slice): the construction
digital twin and the prismatic hull form.

```python
import tpt_yard_py

twin = tpt_yard_py.DigitalTwin.from_project_json(open("project.json").read())
twin.advance_phase(1)
print(twin.weight_report()["design_weight_kg"])

hull = tpt_yard_py.HullForm(140.0, 22.0, 0.72, 0.85)
print(hull.hydrostatics(6.0)["displacement_t"])
print(hull.gz_curve(6.0, 8.0, 40.0)[:3])
```

## Building

This crate is deliberately **detached from the workspace** (the empty
`[workspace]` in its Cargo.toml): pyo3 needs a Python interpreter at
build time, so plain workspace builds never touch it.

```sh
cargo build --release --manifest-path crates/bindings/tpt-yard-py/Cargo.toml
```

Import the produced cdylib renamed to the module name
(`tpt_yard_py.pyd` on Windows, `libtpt_yard_py.so` → `tpt_yard_py.so`
on Linux) from a directory on `sys.path`. The CI `python-bindings` job
does exactly that and drives both classes against the container-ship
template.

## Notes

- `pyo3` is `MIT OR Apache-2.0`, consistent with the workspace
  allow-list; it sits outside the workspace `cargo-deny` graph because
  the crate is detached.
- Scope: `DigitalTwin` (strict project-JSON loading, phase advance,
  weight report, JSON persistence) and `HullForm` (hydrostatics, GZ
  curve, IMO 2008 general check). Engine errors surface as
  `ValueError`.
