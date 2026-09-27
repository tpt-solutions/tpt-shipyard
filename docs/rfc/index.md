# RFC Process & Index

Design changes in `tpt-shipyard` follow the RFC process described in
[GOVERNANCE.md](../../GOVERNANCE.md). RFC files live in the repository root
under [`rfcs/`](../../rfcs/); this page is the index of record.

## Lifecycle

```text
Draft  →  Review (PR, ≥14 days)  →  Accepted | Rejected | Withdrawn  →  Implemented
```

Anything that adds a crate, changes public API shape, adds a dependency, or
changes a physical model requires an RFC. Copy `rfcs/0000-template.md` to the
next free number and open a PR.

## Index

| Number | Title | Status |
|---|---|---|
| [0000](../../rfcs/0000-template.md) | RFC template | — |
| [0001](../../rfcs/0001-digital-twin-state.md) | Digital twin state model | Accepted |
| [0002](../../rfcs/0002-block-assembly-sequencing.md) | Block assembly sequencing | Accepted |
| [0003](../../rfcs/0003-orbital-assembly-planning.md) | Orbital assembly planning | Accepted |
| [0004](../../rfcs/0004-welding-distortion-model.md) | Welding distortion model | Accepted |
| [0005](../../rfcs/0005-rotating-habitat-structural.md) | Rotating habitat structural model | Accepted |

(Statuses reflect the merge state of each RFC; rejected/withdrawn RFCs remain
listed for the audit trail.)
