# Welding Simulation

`tpt-yard-welding` predicts what a weld does to the surrounding plate:
temperature history, residual stress, distortion — and ranks candidate
welding sequences. Model choices and calibration are documented in
[RFC 0004](../../rfcs/0004-welding-distortion-model.md).

## The simulation

A [`WeldingSimulation`](tpt_yard_welding::WeldingSimulation) combines:

- a [`WeldProcedure`](tpt_yard_welding::WeldProcedure) — process (SMAW
  through EBW with arc efficiencies), heat input, travel speed, preheat,
  interpass, filler, and the pass sequence;
- a [`Material`](tpt_yard_core::Material) — AH36, S235, AA5083, 316L,
  Inconel 718, Ti-6Al-4V presets;
- a [`JointGeometry`](tpt_yard_joints::JointGeometry) — joint kind, groove,
  thickness, root gap, fillet legs.

## Thermal cycle — Rosenthal

```text
T − T0 = Q/(2πkR) · exp(−v(R+ξ)/(2α)),  ξ = −v·t,  R = √(ξ²+d²)
```

[`thermal_cycle(d)`](tpt_yard_welding::WeldingSimulation::thermal_cycle)
returns the temperature history at distance `d`, the peak, and the t8/5
cooling time (the HAZ toughness knob). Verified properties: monotone peak
decay with distance; distance-independent t8/5 in 3D; t8/5 grows with heat
input.

## Residual stress

Yield-limited tension zone around the seam with parabolic roll-off, balanced
by uniform compression. The tension half-width is the T_mech = 0.8·T_melt
isotherm of the Rosenthal peak — physical, and ≈10 mm for the reference
panel.

## Distortion

Calibrated contraction estimates (see RFC 0004 for the formulas):

- transverse shrinkage — heat-driven: `δ_T = 0.5·α·Q'/(ρ c t)`;
- longitudinal shrinkage — weld section contracting against the panel;
- angular distortion — off-mid-plane weld metal rotates the joint (single-V
  dominates, balanced X grooves nearly cancel);
- bowing — longitudinal shrinkage eccentricity.

## Sequence optimisation

[`welding_sequence_optimization`](tpt_yard_welding::WeldingSimulation::welding_sequence_optimization)
ranks candidate sequences by a heat-concentration heuristic: spread-out,
alternating-direction (backstep-style) sequences beat concentrated ones.

## Verification

- Golden panel: `test-data/golden/sea/welding-distortion-panel.json`,
  enforced to ±5 % by the crate's golden test.
- Sample WPS records in `test-data/welding-procedures/` load and validate.
