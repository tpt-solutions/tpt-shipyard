# Security Policy

## Supported Versions

The project follows a 6-week release cadence (see [RELEASES.md](RELEASES.md)).
Security fixes are released for the latest stable release and, when warranted,
as patch releases of the previous minor line.

| Version | Supported |
|---|---|
| latest release | ✅ |
| previous minor line | ✅ (security patches only) |
| older | ❌ |

## Reporting a Vulnerability

**Please do not open a public issue for security problems.**

Use GitHub's **private vulnerability reporting** for this repository
(*Security → Report a vulnerability*), or contact us privately at
`security@tpt.solutions` (PGP key available on request).

Include, where possible:

- Affected crate(s) and version(s)
- A minimal reproduction or proof of concept
- The impact you believe it has (especially: wrong numerical results that could
  affect structural safety decisions, panics/unsoundness reachable from public
  APIs, and supply-chain issues in the dependency chain)

## What to Expect

- **Acknowledgement** within 3 business days.
- **Initial assessment** (severity, affected versions) within 14 days.
- **Fix or mitigation** coordinated with you; credit given unless you prefer
  otherwise. We publish advisories through GitHub Security Advisories and
  release patched versions on the regular cadence or sooner for critical issues.

## Scope

In scope: all crates in this workspace, the build/CI configuration, and the
published artifacts. Numerical-correctness reports (a solver returning unsafe
results for valid input) are treated as security-relevant given the domain.

Out of scope: the example/test data files, and issues requiring a malicious or
physically impossible input file to trigger a panic in a non-`unsafe` code path
(we still want to hear about those as regular bugs).

## Dependency Chain

`cargo-deny` enforces an MIT/Apache-2.0-compatible allow-list with copyleft
denied. Dependabot/Renovate-style updates and `cargo audit` run in CI; supply
chain concerns follow the same private disclosure route.
