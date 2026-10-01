# Contributing to tpt-shipyard

Thank you for your interest in `tpt-shipyard`! This project is 100% open source
(MIT OR Apache-2.0), but it **does not accept pull requests**. Contributions are
welcome as **issues only**.

## How to Contribute

Open an issue at <https://github.com/tpt-solutions/tpt-shipyard/issues> for:

- **Bug reports** — include the crate and version, a minimal reproduction, the
  expected result and the actual result. For numerical problems, include the inputs
  and any reference values (analytical solution, class-society rule, published data).
- **Feature requests** — describe the use case and, for physical models, the
  governing equations or standards the feature should follow.
- **Questions and design discussion** — including proposed API or model changes.
- **Documentation problems** — unclear, missing or wrong docs.

Pull requests opened without prior agreement will be closed. Maintainers
implement accepted issues themselves.

## Building Locally

You are welcome to build and run the project to reproduce an issue:

```bash
git clone https://github.com/tpt-solutions/tpt-shipyard
cd tpt-shipyard
cargo build --workspace
cargo test --workspace
```

Requires stable Rust (the MSRV is documented in [RELEASES.md](RELEASES.md)).

## Design Changes

Changes to public API shape, dependencies or physical models are decided by the
maintainers through the RFC process — see [GOVERNANCE.md](GOVERNANCE.md). Raise the
idea as an issue first.

## Licensing

The project is licensed as **MIT OR Apache-2.0**.

## Conduct

Be excellent to each other. Maintainers may remove comments that are hostile or
off-topic.
