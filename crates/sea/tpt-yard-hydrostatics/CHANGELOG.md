# Changelog

## [Unreleased]

- `damage_screen` + `DamageCase`/`DamageSummary` (review 7H): multi-case
  damage screening — runs the added-weight method over a named case set
  and reports per-case GM/list/trim, the governing (lowest-GM) case and
  the all-pass verdict against the 0.05 m floor.

- `Bonjean` / `SectionOffsets` (review 7H): displacement and LCB from real
  hull offsets at any draft and trim, plus `cross_curve_ordinate` (KN) by
  strip integration under the inclined waterline (rotation about the
  centreline point). Verified against box and V-section closed forms
  including the exact constant-volume keel-clipped triangle at 45 deg.

- `damage_stability` (review 7H): added-weight one-compartment screen —
  flood compartments to the sea line, re-solve the trim equilibrium,
  report damaged GM and small-angle list against the 0.05 m floor.

### Added

- `trim_equilibrium` (review 7H): prismatic even-keel-plus-trim solution
  for a displacement and LCG — nested bisection over the trimmed-box
  integral (emerging ends included), fore/aft drafts, trim and optional GM;
  verified against the exact closed form of the same model.
- `wave_bending_moment` (review 7H): IACS CSR wave-induced vertical
  bending moments (sagging +0.11 / hogging -0.13 x Cw x L^2 x B x
  (Cb+0.7)), None outside the 90-300 m rule range.

## 0.1.0

- Initial hydrostatics: prismatic table, GZ curve, free-surface correction, IMO 2008 general criteria.
