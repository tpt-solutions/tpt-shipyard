# Damage Stability

`tpt-yard-hydrostatics` answers the damage-stability question at two
levels of fidelity: a deterministic added-weight screen for one or many
flooding cases, and the probabilistic factors of the SOLAS harmonized
method (Ch. II-1 Part B-1) for regulated ships.

## Added-weight damage screen

[`damage_stability`](tpt_yard_hydrostatics::HullForm::damage_stability)
floods a set of [`DamageCompartment`](tpt_yard_hydrostatics::DamageCompartment)s
to the sea line, re-solves the trim equilibrium with the grown
displacement and shifted LCG/TCG, and reports the damaged GM, list and
trim against the 0.05 m one-compartment screening floor.
[`damage_screen`](tpt_yard_hydrostatics::HullForm::damage_screen) runs a
named case set and picks the governing (lowest-GM) case.

## The probabilistic method (SOLAS 2009)

The harmonized method scores each damage case by a probability of
occurrence times a probability of survival.
[`DamageLengthDensity`](tpt_yard_hydrostatics::DamageLengthDensity) is
the bi-linear damage-length distribution of Reg. 7-1 (knuckle at
`Jkn = 5/33`, cumulative `11/12` there, support `min(10/33, 60 m/Ls)`);
[`p_factor`](tpt_yard_hydrostatics::p_factor) returns the probability
that the damage opens exactly a longitudinal zone, and
[`s_factor_cargo`](tpt_yard_hydrostatics::s_factor_cargo) scores the
damaged GZ curve (`s = K·[(range/16°)·(GZmax/0.12 m)]^{1/4}` with the
15°–30° heel gate for cargo ships).
[`required_index_cargo`](tpt_yard_hydrostatics::required_index_cargo)
gives `R = 1 − 128/(Ls + 152)`; the case scores sum to the attained
index ([`attained_subdivision_index`](tpt_yard_hydrostatics::attained_subdivision_index))
which must reach `R`.
[`damaged_survivability_cargo`](tpt_yard_hydrostatics::HullForm::damaged_survivability_cargo)
runs the added-weight flood and extracts the Reg. 7-2 inputs (peak arm,
range, equilibrium heel) from the wall-sided damaged GZ scan.

```rust
use tpt_yard_hydrostatics::{
    attained_subdivision_index, p_factor, required_index_cargo,
    DamageCaseProbability, DamageLengthDensity,
};

let ls = 150.0; // subdivision length, m
let density = DamageLengthDensity::for_subdivision_length(ls).unwrap();

// Two midship single-zone damages (x from the aft terminal of Ls).
let p_hold2 = p_factor(&density, ls, 40.0, 70.0).unwrap();
let p_hold3 = p_factor(&density, ls, 70.0, 100.0).unwrap();

// Survivability from the damaged GZ curves (s = 1 when the damaged
// ship keeps a healthy range and arm).
let cases = vec![
    DamageCaseProbability { name: "hold 2".into(), p_factor: p_hold2, s_factor: 1.0 },
    DamageCaseProbability { name: "hold 3".into(), p_factor: p_hold3, s_factor: 0.6 },
];
let a = attained_subdivision_index(&cases);
let r = required_index_cargo(ls).unwrap();
assert!(a < r, "two vulnerable holds do not satisfy a 150 m cargo ship");
```

Longitudinal zones combine with the regulation's alternating forms via
[`multi_zone_p_factor`](tpt_yard_hydrostatics::multi_zone_p_factor), and
wing bulkheads reduce a group through the Reg. 7-1.2 penetration factor
[`r_factor`](tpt_yard_hydrostatics::r_factor)
(`r = 1 − (1−C)·[1 − G/p]`, `C = 12·Jb·(4−45·Jb)` — zero at no
penetration, exactly 1 at `B/2`).

The remaining gaps: horizontal-deck `v` factors, passenger intermediate
stages and the 80–100 m `R` interpolation stay class-society work.
