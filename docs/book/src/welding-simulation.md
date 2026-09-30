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

## Procedure advisor

Before a seam is welded, `tpt_yard_welding::advisor` screens the WPS:

- **Weldability** — carbon equivalents CE(IIW), CET (EN 1011-2) and Pcm
  (Ito-Bessyo) from the ladle chemistry, and the Graville class that says
  whether cracking risk is chemistry-driven or hydrogen/heat-input-driven.
- **Preheat** — the SEW 088-style screening table (`advise_preheat`), and
  `preheat_for_target_t8_5`: the Rosenthal t8/5 closed form inverted for
  the preheat that hits a target cooling time (monotone in T0, solved by
  bisection).
- **Cooling window** — `advise` screens the proposed procedure against a
  supplier t8/5 window (hardenability floor, toughness ceiling) and
  proposes a preheat when the floor is missed.
- **Qualification** — `Pqr::qualifies` checks the WPS inside the PQR
  essential-variable envelope (process, heat input, preheat/interpass,
  filler, and the ISO 15614-1 Table 5 thickness rule 0.5t to 2t; ASME IX
  screening uses the same envelope with finer deposit rules left manual).

## Verification

- Golden panel: `test-data/golden/sea/welding-distortion-panel.json`,
  enforced to ±5 % by the crate's golden test — and independently
  re-derived in the test from the closed-form Rykalin t8/5 (29.08 s) and a
  dense analytic Rosenthal peak scan.
- Sample WPS records in `test-data/welding-procedures/` load and validate.
