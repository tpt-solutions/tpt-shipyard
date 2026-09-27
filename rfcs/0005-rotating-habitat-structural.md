# RFC 0005: Rotating Habitat Structural Model

- **Number:** 0005
- **Title:** Rotating habitat design: spin, comfort, structure, and the
  space environment
- **Status:** Accepted
- **Authors:** TPT Solutions
- **Created:** 2026-09-14
- **Review window:** closed 2026-09-28

## Summary

`tpt-yard-space-structural` and `tpt-yard-habitat` implement the
rotating-habitat design chain: spin rate for a target gravity
(`omega = sqrt(g/r)`), the 2 rpm Coriolis comfort limit, hoop stress of the
spinning hull (`sigma = rho * omega^2 * r^2`), thermal-cycling fatigue,
Whipple-shield sizing, and the structural sizing loop from target gravity
to shell thickness.

## Detailed Design

### Spin and comfort

- Required rotation for `g_target` at radius `r`:
  `omega = sqrt(g_target * 9.81 / r)` (rad/s), reported also in rpm.
- Comfort criterion: rotation **≤ 2 rpm**. Cross-coupled acceleration for a
  1 m/s head movement is `a = 2 * omega * v`; at 2 rpm that is ~4.3 % of g,
  comfortably below the ~10 % nausea threshold. The check is inclusive at
  2.0 rpm.
- Consequence encoded in tests: 1 g at 100 m needs 2.99 rpm (uncomfortable
  without mitigation); 1 g at 4 km (O'Neill cylinder) needs 0.47 rpm
  (comfortable). Designers trade radius against rpm explicitly.

### Ring structure

Thin-ring hoop stress: `sigma = rho * omega^2 * r^2` with the structural
material density; radial growth is the elastic strain times the radius.
Verified to machine precision against the closed form (golden:
`rotating-habitat-stress.json`).

Structural sizing (`HabitatDesigner::structural_design`): the ring's hoop
tension `T = m * omega^2 * r / (2 pi)` over the allowable (yield / safety
factor) gives the required hoop area; thickness follows from the
circumference. The result is self-consistent (utilization ≤ 1 by
construction) and monotone in mass and safety factor (tested).

### Space environment loads

- **Thermal fatigue**: strain range `dE = alpha * (T_hot − T_cold)`;
  Coffin–Manson life `N_f = 0.5 * (dE / (3.5 * sigma_u / E))^(-1/0.12)`;
  Miner's-rule life fraction for the demanded cycles. Ordering tested:
  higher expansion or lower ductility shortens life.
- **Micrometeoroid shielding** (Whipple): bumper `d/6` mm, standoff `d/10`
  m, rear wall `d/12` mm for protected particle diameter `d`, with areal
  density from the plate masses. Adapted from NASA ship-set sizing
  practices for screening use.
- **Design freedom**: `no_launch_constraint()` returns the formal statement
  (no fairing, no aero, unbounded size) used by planners to justify
  arbitrary geometry.

## Verification

- Closed-form verification: `test_rotating_habitat_stress` (spec §9
  reference: 2 rpm, 100 m, aluminium — machine-precision match), golden
  `rotating-habitat-stress.json`.
- `omega = sqrt(g/r)` verified exactly; rpm conversions verified.
- 2 rpm comfort boundary tested inclusive/exclusive; monotonicity of the
  structural sizing tested.
- Milestone example: `examples/rotating-habitat-construction/` prints the
  full design chain for a Stanford torus and an O'Neill cylinder.

## Drawbacks

- Thin-ring stress ignores tube bending, end closures, and boundary
  structures (a torus is not a pure ring; the model is a screening tool).
- Coffin–Manson with a single exponent ignores mean-stress and
  frequency effects.
- Shield sizing is empirical; ballistic-limit equations are out of scope.

## Alternatives Considered

- Shell FEM for the hull: deferred to `tpt-fem` integration; the closed
  form is exact for the thin-ring idealisation and adequate for sizing.
- Full attitude/spin dynamics (precession, wobble): Phase 5+ with
  `tpt-science` orbital mechanics.

## Unresolved Questions

- None blocking Phase 4. Two-load-path (collision + pressure) hull
  optimisation is a natural follow-up.
