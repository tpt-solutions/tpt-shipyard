# Scantling Checks

`tpt-yard-hydrostatics` covers the shipyard-side scantling questions at
two levels: the hull-girder section modulus and the local plating
(bending + buckling). Both are closed-form screening legs — the full
class calculations stay with the societies.

## Hull girder

[`scantling_requirement`](tpt_yard_hydrostatics::HullForm::scantling_requirement)
combines the supplied still-water bending moment with the IACS CSR
wave-induced moments ([`wave_bending_moment`](tpt_yard_hydrostatics::HullForm::wave_bending_moment))
against the `175/k` normal-stress allowable and returns the required
section modulus. [`high_strength_factor`](tpt_yard_hydrostatics::high_strength_factor)
maps steel grades to `k` (MS 1.0, AH32 0.91, AH36 0.78, AH40 0.72).

## Local plating

The lateral-pressure bending leg is the clamped-slab relation
`sigma = 0.5·p·s²/t²` in rule units — the constant the rules print as
22.4 is exactly `1000·sqrt(0.5e-3)`:

[`slab_bending_thickness_mm`](tpt_yard_hydrostatics::slab_bending_thickness_mm)
returns the net thickness, with a demand-side material factor (same
direction as `175/k`) and the rule boundary factor (CSR 1.2 for
load-carrying plates).

The buckling leg is the classical simply-supported plate Euler stress

[`plate_euler_stress_mpa`](tpt_yard_hydrostatics::plate_euler_stress_mpa)
with the half-wave aspect factor `(m·b/a + a/(m·b))²` — 4 for square
and long panels — and
[`plate_buckling_thickness_mm`](tpt_yard_hydrostatics::plate_buckling_thickness_mm)
is its exact inversion.
[`local_plate_scantling`](tpt_yard_hydrostatics::local_plate_scantling)
runs both legs, picks the governing thickness and adds the corrosion
margin:

```rust
use tpt_yard_hydrostatics::{
    high_strength_factor, local_plate_scantling, LocalPlateScantlingInput,
};

let k = high_strength_factor("AH36").unwrap();
let scantling = local_plate_scantling(LocalPlateScantlingInput {
    spacing_m: 0.8,            // stiffener spacing
    long_span_m: Some(3.2),    // 4:1 panel between floors
    pressure_kn_m2: 100.0,     // design sea pressure
    allowable_bending_mpa: 180.0,
    material_factor_k: k,
    boundary_factor: 1.2,      // load-carrying plate
    corrosion_addition_mm: 1.5,
    applied_compression_mpa: Some(80.0), // hull-girder in-plane demand
    youngs_modulus_gpa: 206.0,
})
.unwrap();
assert!(scantling.governing_net_mm > 0.0);
assert_eq!(scantling.mode, tpt_yard_hydrostatics::ScantlingMode::Bending);
assert!(scantling.with_corrosion_mm > scantling.governing_net_mm);
```

The buckling leg uses the bare Euler criterion on its own; for a
verified capacity curve,
[`plate_buckling_reduction_ec3`](tpt_yard_hydrostatics::plate_buckling_reduction_ec3)
transcribes the EN 1993-1-5 clause 4.4 Winter-type reduction for
internal elements (`ρ = (λ̄ − 0.055·(3+ψ))/λ̄²`, capped at 1) and
[`plate_buckling_check_ec3`](tpt_yard_hydrostatics::plate_buckling_check_ec3)
combines it with the Euler stress into the `ρ·f_y` capacity and
utilisation:

```rust
use tpt_yard_hydrostatics::plate_buckling_check_ec3;

// The 800x3200x10 panel from above, AH36 (fy 355 MPa), 150 MPa demand.
let check = plate_buckling_check_ec3(10.0, 800.0, 3200.0, 150.0, 355.0, 206_000.0, 0.3)
    .unwrap();
assert!(check.lambda_bar > 1.0);      // slender: euler stress below fy
assert!((check.reduction - 0.5).abs() < 0.01);
assert!(check.passes);
```

The CSR plates' own η curves are class-society content and are not
shipped; the Eurocode curve above is the stand-in. Minimum thicknesses
are handled by a user-supplied rule table (next section).

## Rule minimum thicknesses

Minimum scantlings are rule content that differs between societies and
changes with each edition, so the engine ships none. A yard loads its own
table and the engine only applies it:

```json
{ "society": "Your society", "edition": "2026",
  "min_thickness": [
    { "member": "bottom shell", "basis": "net", "base_mm": 5.0,
      "coeff_mm": 0.04, "length_exponent": 1.0, "max_mm": 16.0 },
    { "member": "tank boundary", "basis": "gross", "base_mm": 7.5 } ] }
```

Each rule is `t_min = base + coeff · L^exponent`, clamped to optional
`min_mm`/`max_mm`, in the ship length `L`; *net* minima get the scantling's
corrosion addition on top, *gross* ones are used as built. The loader is
strict (see `schemas/scantling-rule-table.schema.json`; the sample
`test-data/rules/example-rule-table.json` is placeholder numbers, not a
society's rules).

[`RuleTable::check_plate`](tpt_yard_hydrostatics::RuleTable::check_plate)
compares a calculated plate scantling with the member's minimum and reports
the larger, with which one governed:

```rust
use tpt_yard_hydrostatics::{
    local_plate_scantling, LocalPlateScantlingInput, RuleGoverned, RuleTable,
};

let table = RuleTable::from_json_str(
    r#"{"society":"Example","edition":"x","min_thickness":[
        {"member":"bottom shell","basis":"net","base_mm":5.0,
         "coeff_mm":0.04,"length_exponent":1.0}]}"#,
).unwrap();
let plate = local_plate_scantling(LocalPlateScantlingInput {
    spacing_m: 0.7, long_span_m: None, pressure_kn_m2: 10.0,
    allowable_bending_mpa: 160.0, material_factor_k: 1.0,
    boundary_factor: 1.0, corrosion_addition_mm: 1.5,
    applied_compression_mpa: None, youngs_modulus_gpa: 206.0,
}).unwrap();
let checked = table.check_plate("bottom shell", 140.0, &plate).unwrap();
// A lightly loaded plate is set by the rule minimum: 10.6 net + 1.5.
assert_eq!(checked.governed_by, RuleGoverned::RuleMinimum);
assert!((checked.required_gross_mm - 12.1).abs() < 1e-9);
```

## Stiffeners

[`stiffener_scantling`](tpt_yard_hydrostatics::stiffener_scantling)
sizes the longitudinal/transverse under lateral pressure with the
fixed-fixed load model: end moment `p·s·l²/12` (governing), midspan
`p·s·l²/24`, reaction `p·s·l/2`. It returns the required section
modulus (`1000·M/σ` cm³, demand-side `k`) and — when a shear
allowable is supplied — the required shear area (`10·V/τ` cm²).

```rust
use tpt_yard_hydrostatics::stiffener_scantling;

// Frame: 0.8 m spacing, 3 m span, 100 kN/m2, 150 MPa allowable.
let frame = stiffener_scantling(0.8, 3.0, 100.0, 150.0, 1.0, Some(90.0))
    .unwrap();
// The end moment carries the classic closed form p s l^2 / 12:
assert!((frame.end_moment_knm - 100.0 * 0.8 * 9.0 / 12.0).abs() < 1e-12);
assert!(frame.required_modulus_cm3 > 0.0);
assert!(frame.required_shear_area_cm2.unwrap() > 0.0);
```
