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
penetration, exactly 1 at `B/2`). Horizontal decks enter through
[`v_factor`](tpt_yard_hydrostatics::v_factor)
(`0.8 + 0.2·[(H−d)−7.8]/4.7`, the MSC.421(98) form).

Survival factors cover both ship types:
[`s_final_factor`](tpt_yard_hydrostatics::s_final_factor)
(`K·[(Range/TRange)·(GZmax/TGZmax)]^{1/4}` with the ro-ro deck caps and
the heel gates — cargo **25°–30°** in the amended text),
[`s_intermediate_factor`](tpt_yard_hydrostatics::s_intermediate_factor)
(the 0.05 m / 7° caps) and
[`s_mom_factor`](tpt_yard_hydrostatics::s_mom_factor)
(passenger heeling moments); [`s_factor_cargo`](tpt_yard_hydrostatics::s_factor_cargo)
stays the convenient cargo wrapper. The Reg. 6 requirement extends to
80 m with the printed interpolation
([`required_index_cargo`](tpt_yard_hydrostatics::required_index_cargo),
continuous with `R0` at 100 m).

The intermediate stages that feed
[`s_intermediate_factor`](tpt_yard_hydrostatics::s_intermediate_factor)
come from
[`damage_stages`](tpt_yard_hydrostatics::HullForm::damage_stages) —
partial-volume floods at i/n, the last stage reproducing the final
equilibrium exactly — and the survival-craft moment from
[`survival_craft_moment`](tpt_yard_hydrostatics::survival_craft_moment)
over the swung-out craft list.

Tank-plan geometry: a
[`TankCompartment`](tpt_yard_hydrostatics::TankCompartment) (bottom,
plan area, height, permeability) fills physically from the bottom up —
[`flood_volume_m3`](tpt_yard_hydrostatics::flood_volume_m3),
[`flood_cg_z`](tpt_yard_hydrostatics::flood_cg_z) and
[`tank_stage_compartment`](tpt_yard_hydrostatics::tank_stage_compartment)
bridge into the staged solver — and
[`cross_flooding_time`](tpt_yard_hydrostatics::cross_flooding_time)
integrates Torricelli's law for the Reg. 7-2.2 equalization time
through a duct (compare the 10-minute limit).

What remains class-society work: tank plans with non-vertical walls,
per-stage heel solvers with cross-flooding time effects inside the
stage physics, and survival-craft arrangement assumptions.
