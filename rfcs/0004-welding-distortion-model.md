# RFC 0004: Welding Distortion Model

- **Number:** 0004
- **Title:** Analytical welding thermal, residual-stress and distortion model
- **Status:** Accepted
- **Authors:** TPT Solutions
- **Created:** 2026-03-02
- **Review window:** closed 2026-03-16

## Summary

`tpt-yard-welding` implements an engineering-grade analytical chain — Rosenthal
thermal cycle → yield-limited residual stress → calibrated contraction
distortion → sequence ranking — that runs in microseconds and is verified
against a golden reference panel. It is explicitly **not** a
thermo-mechanical FEM substitute; it is the fast, auditable layer the digital
twin needs for every weld in the ship, with FEM reserved for critical joints.

## Detailed Design

### Thermal cycle

Quasi-stationary 3D Rosenthal moving point source on a semi-infinite body:

```text
T(x, t) − T0 = Q / (2π k R) · exp(−v (R + ξ) / (2α)),  ξ = x − v t,  R = √(ξ² + d²)
```

with Q = η·U·I (evaluated as heat-input × travel speed), k conductivity,
α diffusivity. Points are sampled from source approach to far wake; the
sample window covers at least one metre of source travel because the 3D tail
decays like 1/R. Peak temperature and the t8/5 cooling leg (interpolated
crossings of 800 °C and 500 °C) are derived from the samples.

Known properties preserved and tested: peak decays monotonically with
distance; t8/5 is essentially distance-independent in 3D and scales with
heat input; closer to the seam the material exceeds melting for
shipyard-typical inputs.

### Residual stress

Longitudinal-dominant model after cooling:

- Tension zone `|y| ≤ b`: σ = σ_y·(1 − (y/b)²) (parabolic roll-off from
  yielding at the seam).
- Outside: uniform compression balancing the tension-zone force (net axial
  force zero over the reference panel).

The half-width `b` is the **isotherm of the mechanical response temperature**
`T_mech = 0.8·T_melt` from the Rosenthal peak-temperature solution — material
beyond that isotherm never yields, hence carries no residual tension. For the
reference panel (12 kJ/mm SAW on AH36) this gives b ≈ 10 mm, consistent with
published HAZ half-widths.

### Distortion

Calibrated contraction estimates (calibration against published AH36 panel
data; constants locked with the golden file):

| Quantity | Formula | Reference value |
|---|---|---|
| Transverse shrinkage | `δ_T = 0.5 · α · Q' / (ρ c t)`, Q' in J/m | 1.56 mm |
| Longitudinal shrinkage | `δ_L = α · ΔT_mech · (A_weld/A_panel) · L` | 0.072 mm |
| Angular distortion | `β = 2 · α · ΔT_mech · (A_off/t²) · e`, e = 1.0 single-side, 0.25 balanced | 0.69° |
| Bowing | longitudinal shrinkage at mid-thickness, 2 m panel | ~0.9 µm |

`ΔT_mech = T_mech − T0`; `A_off` is weld area off the mid-plane. Multi-pass
sequences superpose linearly in accumulated heat input.

### Sequence optimisation

Relative ranking of candidate sequences: pairwise heat-concentration penalty
(nearby same-direction passes couple), alternation bonus. The absolute
distortion of the chosen sequence still comes from the distortion model. True
minimisation is out of scope without FEM.

### Verification & golden data

`test-data/golden/sea/welding-distortion-panel.json` locks the reference
case: 1000×500×12 mm AH36, single-V 60° butt (2 mm face, 3 mm gap), SAW,
12 kJ/mm @ 8 mm/s, preheat 20 °C. Golden values and the ±5 % tolerance band
(±10 % for t8/5, sample-discretisation sensitive) are enforced by
`golden_welding_distortion_panel`. The values sit inside published
experimental ranges for comparable single-V SAW butts (transverse shrinkage
0.5–2.5 mm; angular 0.5–4°).

### Process efficiencies

η: SMAW 0.80, GMAW 0.85, GTAW 0.60, SAW 0.95, FCAW 0.80, EBW 0.90,
laser 0.45 (classical analytical-work values).

## Drawbacks

- Point-source singularity: peak temperatures at the weld line are
  unphysically high; models calibrated to behave for d ≥ ~1 mm.
- No plate-edge correction, no clamping restraints, no phase
  transformations; angular distortion is a calibrated estimate, not a
  solution.
- 2D (thin-plate) heat spreading is not modelled; aluminium thin panels
  will read cooler/wider than reality.

## Alternatives Considered

- **Thermo-mechanical FEM now** (the Sysweld-class approach): rejected for
  Phase 2 — the twin needs millisecond answers for thousands of seams; FEM
  comes later as `tpt-fem` integration for critical joints.
- **Empirical tables only** (class societies publish shrinkage tables):
  rejected — no physics, no extrapolation beyond the table.
- **Eulerian FDM heat conduction**: middle ground; possible future
  refinement behind the same API.

## Unresolved Questions

- Calibration refresh against in-house panel trials (planned once a partner
  yard shares measurements).
- Thin-plate (2D) regime switch for t < 4 mm aluminium.
