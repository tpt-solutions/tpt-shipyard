# tpt-shipyard Roadmap

This file mirrors the public **GitHub Projects** roadmap board for
`tpt-shipyard` (linked from the repository sidebar). The board is the live
tracker; this document is the durable, in-repo snapshot of the delivery plan.

## Board columns

`Backlog` → `Planned` → `In Progress` → `In Review` → `Done`

## Milestones

| Phase | Window | Milestone | Status |
|---|---|---|---|
| 0 — Repository & Governance | pre-launch | Empty workspace builds, CI green, license check passes | ✅ Done |
| 1 — Core & Digital Twin | Months 1–3 | Track weight and CoG through 10 build phases | ✅ Done |
| 2 — Structural During Construction | Months 4–6 | Simulate welding distortion in a hull panel end-to-end | ✅ Done |
| 3 — Sea Shipyard | Months 7–12 | Plan block erection sequence for a container ship end-to-end | ✅ Done |
| 4 — Space Shipyard | Months 13–18 | Simulate orbital truss assembly with robotic arm end-to-end | ✅ Done |
| 5 — Manufacturing & Planning | Months 19–24 | Plan in-space additive manufacturing of a solar array end-to-end | ✅ Done |
| 6 — Integration & WASM | Months 25–30 | Interactive 3D shipyard dashboard running in browser | ✅ Done |

## Standing work items

- Keep the `deny.toml` MIT-chain check green on every PR.
- Keep the crate `Status` table in `README.md` in sync with reality.
- Register every new crate in the workspace members list.
- Document every new public API in `docs/book`.

Board sync policy: maintainers update the GitHub Projects board when a milestone
or epic changes state; this file is updated on the same PR when a milestone
completes.
