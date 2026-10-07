# tpt-shipyard Governance

## Summary

`tpt-shipyard` is governed by a **Benevolent Dictator** model with a fully public
**RFC process**. The project is developed in the open: roadmap, decisions, and
review history are visible to everyone.

```text
┌─────────────────────────────────────────────────────────────┐
│                   TPT SHIPYARD GOVERNANCE                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  License:        MIT OR Apache-2.0 (dual)                   │
│  Contributions:  MIT OR Apache-2.0 (no CLA)                 │
│  Trademark:      "TPT Shipyard" name reserved by TPT        │
│  Governance:     Benevolent Dictator + RFC process          │
│  Roadmap:        Public GitHub Projects board               │
│  Releases:       SemVer, 6-week cadence                     │
│  Security:       SECURITY.md, private disclosure            │
│  Standards:      ISO 19847 (ship data), CCSDS (space)       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Roles

### Benevolent Dictator (BD)

The BD (currently TPT Solutions) has final authority on technical direction,
RFC acceptance, and releases. The BD is expected to exercise this authority
rarely: the default is consensus among active contributors, and every contested
decision is settled by a written rationale in public.

The BD may delegate ownership of crates or subsystems to maintainers, who then
review and merge within the delegated scope.

### Maintainers & Contributors

Anyone may comment on issues and RFCs. Anyone may submit PRs; contributions are
accepted under the project's MIT OR Apache-2.0 licence. Maintainers are added by the BD based on sustained, high-quality
contributions and are listed here when appointed.

## RFC Process

Significant changes — new crates, public API shape changes, new dependencies,
changes to physical models or numerical methods — require an RFC in `rfcs/`.

### Lifecycle

```text
Draft  →  Review  →  Accepted | Rejected | Withdrawn  →  Implemented
```

1. **Draft.** Copy `rfcs/0000-template.md` to `rfcs/NNNN-short-name.md` using
   the next free number. Open a PR. Add the entry to `docs/rfc/index.md`.
2. **Review.** Discussion happens on the PR for at least 14 days (longer for
   safety-critical models). Anyone may comment; the BD and maintainers give a
   explicit final comment.
3. **Decision.** The BD marks the RFC **Accepted** or **Rejected** with a
   written rationale. Accepted RFCs merge to `main`; the PR adding the RFC may
   land together with an initial implementation.
4. **Implementation.** The implementing PR references the RFC number. If the
   implementation diverges materially, the RFC is updated first.

### What Requires an RFC

- A new crate or the removal/deprecation of a crate
- Any change to a published struct/enum/function signature (post-1.0)
- A new external dependency
- A change to verification methodology, tolerances, or golden data
- Changes to governance, licensing, or the release process

Everything else is a normal PR.

## Trademark Notice

The names **"TPT Shipyard"**, **"TPT Solutions"**, and the project logos are
trademarks of TPT Solutions. The MIT/Apache-2.0 licenses grant broad rights to
the **code and documentation** in this repository, but **not** to the trademarks:
you may not distribute modified versions under a name confusingly similar to
"TPT Shipyard" or imply endorsement by TPT Solutions. See the trademark section
of each license for the general reservation of naming rights.

## Decision Records

All accepted and rejected RFCs remain in `rfcs/` permanently — including
rejections, so the reasoning is auditable. This repository is the single source
of truth; there is no private decision channel for technical direction.

## Roadmap

Work is tracked on the public GitHub Projects board linked from the repository
sidebar. The board mirrors the phased delivery plan (`todo.md` / spec §10):
Core & Digital Twin → Structural During Construction → Sea Shipyard →
Space Shipyard → Manufacturing & Planning → Integration & WASM.

## Code of Conduct

Be excellent to each other. Maintainers may remove hostile or off-topic content.
Reports can go to any maintainer or the BD privately.
