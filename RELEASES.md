# Release Policy

## Versioning: SemVer

Every crate in this workspace versions together as a single release train
(unified workspace version). We follow [Semantic Versioning](https://semver.org/):

- **MAJOR** — breaking public API or behaviour changes (removed/renamed items,
  semantic changes to results or error conditions).
- **MINOR** — new public API, new crates, meaningful model improvements,
  fully backward compatible.
- **PATCH** — bug fixes, documentation, performance, dependency updates.

While the workspace is `0.x` (pre-1.0), the **minor** digit acts as the breaking
slot: `0.3 → 0.4` may contain breaking changes, which are called out in the
changelog. Do not rely on pre-1.0 API stability.

Breaking changes to public API require an RFC (see [GOVERNANCE.md](GOVERNANCE.md)).

## Cadence: every 6 weeks

A release branch is cut every **6 weeks** from `main`:

| Step | Timing |
|---|---|
| Feature freeze / release branch | day 0 |
| Stabilisation (fixes only) | days 0–6 |
| Release published (crates.io + GitHub Release) | day 7 |
| Next cycle opens | immediately |

If there are no changes worth releasing, the release is skipped and the next
cycle opens on schedule.

## MSRV

The Minimum Supported Rust Version for a release is the **stable Rust current at
the time of the previous release** (i.e. roughly stable minus ~6 weeks). CI tests
the MSRV and current stable. MSRV bumps are MINOR-level changes.

## Release Mechanics

1. Update `CHANGELOG.md` (Keep a Changelog format) from the merged PRs.
2. Bump the workspace version, tag `vX.Y.Z`, and push the tag.
3. CI (`release.yml`) runs the full test suite, then
   `cargo publish --workspace` in dependency order and attaches a GitHub Release.
4. Crates are published under the `tpt-solutions` publisher; only the BD and
   release maintainers have publish rights.

## Deprecation Policy

Deprecated items survive at least **two minor releases** (or one major) with
`#[deprecated]` notices and a migration note in the changelog before removal.
