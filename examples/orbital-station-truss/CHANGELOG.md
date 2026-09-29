# Changelog

All notable changes to this example are documented here
([Keep a Changelog](https://keepachangelog.com/en/1.1.0/), SemVer).

## [0.1.0] - 2026-09-28

### Added

- Simulates a robotic truss build: sequence planning (grasp, translate, rotate, dock, bolt, release per bay), step simulation with collision and force-limit checks, and partial-structure integrity at every bay release.
- Runnable via `cargo run -p orbital-station-truss`.
