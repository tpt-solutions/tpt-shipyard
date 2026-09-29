# Changelog for tpt-yard-earth-link

All notable changes to this crate are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning
follows [Semantic Versioning](https://semver.org/). The workspace versions as
a single release train on a 6-week cadence — see the workspace
[RELEASES.md](../../RELEASES.md).

## [0.1.0] - 2026-09-28

### Added

- `plan_launch_window` — the longest calm run in the forecast for the launch method
- Per-method screening limits: side launch 1, slipway/shiplift 2, dock flooding 3 (Douglas scale)
- Never-safe forecasts are a typed error, never an empty window
- `LaunchWindow` with first hour, exclusive end, limit, calm hours, and a plain-language note

### Verification

- A mid-day storm pushes the window to the longer calm run
- The same forecast clears dock flooding but blocks a side launch (method limits differ)
- Never-safe and empty forecasts are typed errors

[0.1.0]: https://github.com/tpt-solutions/tpt-shipyard/releases/tag/v0.1.0
