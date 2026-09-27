# Contributing to tpt-shipyard

Thank you for contributing to `tpt-shipyard`! This project is 100% open source and
accepts contributions **CLA-free** under the Developer Certificate of Origin (DCO).

## Developer Certificate of Origin (DCO)

All contributions must be signed off. By signing off you certify that you wrote or
have the right to submit the contribution under the project's licenses
(MIT OR Apache-2.0), per the [DCO 1.1](https://developercertificate.org/).

Sign every commit with a `Signed-off-by` trailer:

```text
Signed-off-by: Your Name <you@example.com>
```

The easiest way is `git commit --signoff` (or `-s`). CI rejects commits without a
matching sign-off.

## Getting Started

```bash
git clone https://github.com/tpt-solutions/tpt-shipyard
cd tpt-shipyard
cargo build --workspace
cargo test --workspace
```

Requirements:

- Stable Rust (the MSRV is documented in [RELEASES.md](RELEASES.md))
- `rustfmt` and `clippy` (via `rustup component add rustfmt clippy`)
- `cargo-deny` for the license gate (`cargo install cargo-deny --locked`)

## Before You Open a PR

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

### PR Checklist

- [ ] Commits are DCO-signed
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] New/changed behaviour has tests
- [ ] Public API items have rustdoc comments (`missing_docs` is enforced)
- [ ] New public API is reflected in `docs/book` where relevant
- [ ] Numerical code has a verification path (analytical test or golden data)

## Code Layout

- Crates live under `crates/{core,structural,sea,space,planning,integration}`.
- Dependency direction: `assembly`/`core` at the bottom; domain crates above;
  `integration/*` and `wasm` at the top. Never introduce a cycle.
- Golden verification data lives in `test-data/golden/`; sample inputs in
  `test-data/*`.
- End-to-end demonstrations live in `examples/` and benchmarks in `benches/`.

## Design Changes

Anything that changes public API shape, adds a dependency, or changes a physical
model goes through the RFC process first — see [GOVERNANCE.md](GOVERNANCE.md) and
`rfcs/0000-template.md`.

## Licensing

By contributing you agree your contributions are licensed as
**MIT OR Apache-2.0**. The `cargo-deny` gate keeps the dependency chain free of
copyleft licenses — do not add dependencies that fail `cargo deny check licenses`.

## Conduct

Be excellent to each other. Maintainance may remove comments that are hostile or
off-topic.
