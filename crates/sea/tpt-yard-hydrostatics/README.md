# tpt-yard-hydrostatics

Hydrostatics and intact stability for prismatic ship forms: the hydrostatic
table (displacement, KB, KM, TPC, MCT1cm), the wall-sided GZ curve with a
free-surface correction, and the IMO 2008 IS Code general intact-stability
criteria (areas to 30°/40°, max-GZ angle, minimum GM).

```rust
use tpt_yard_hydrostatics::{HullForm, LoadingCondition};

let hull = HullForm { loa_m: 140.0, boa_m: 22.0, cb: 0.72, cwp: 0.85 };
let gz = hull.gz_curve(LoadingCondition::new(6.0, 8.0), 40.0);
let verdict = hull.imo_2008_general(&gz, 6.0);
println!("{verdict}");
```

Status: screening-grade prismatic model (Bonjean curves, trim/list and
damage stability are roadmap work).
