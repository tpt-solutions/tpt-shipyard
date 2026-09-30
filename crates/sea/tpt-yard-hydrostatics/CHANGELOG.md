# Changelog

## [Unreleased]

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
