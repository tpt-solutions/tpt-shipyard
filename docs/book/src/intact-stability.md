# Intact Stability from Offsets

`tpt-yard stability` takes a hull and a loading condition and reports
hydrostatics, the GZ curve and the IMO 2008 IS Code general criteria.
The hull is either a **hull-offsets table** (full form) or **prismatic
coefficients** (a box-like screening model).

```text
tpt-yard stability test-data/stability/wigley.json --csv out --svg gz.svg --strict
```

## Starting from a template

```text
tpt-yard new-hull barge --out my-barge          # also: wigley, workboat, tug, sailboat, ferry
tpt-yard stability my-barge/barge.json
```

`new-hull` writes `NAME-offsets.csv` and `NAME.json`, with `--loa`, `--beam`,
`--draft`, `--depth` and `--kg` to resize the hull (`--force` overwrites).
The generated tables are the format to copy when exporting offsets from a
lines-plan tool. The box barge has exact hydrostatics (`V = L·B·T`,
`KB = T/2`, `BM = B²/(12T)`), which the test suite checks; the workboat is a
hard-chine V-bottom with a transom and flared topsides.

## The case file

```json
{
  "vessel": "Wigley hull (analytic reference)",
  "offsets_csv": "wigley-offsets.csv",
  "loading": { "draft_m": 5.0, "kg_m": 3.5, "free_surface_moment_tm": 0.0 }
}
```

- `offsets_csv` is resolved relative to the case file. Without it the case
  needs a `hull` object (`loa_m`, `boa_m`, optionally `cb` and `cwp`); a
  hull manifest with `depth_m` also works, with the draft and KG taken as
  65 % and 55 % of depth (the report says so).
- Flags override the file: `--offsets`, `--draft`, `--kg`, `--fsm`
  (free-surface moment, t·m), `--to-deg` (default 60, at least 40).

## The offsets CSV

One row per offset: `station_x_m, draft_m, half_breadth_m`.

```text
station_x_m,draft_m,half_breadth_m
-50,0,0
-50,0.25,0
...
```

`x` is from midship, positive forward; `draft_m` is the height above the
baseline; each station must start at `draft_m = 0`. Commas, semicolons or
tabs separate columns, `#` starts a comment, and one header row is
allowed. Errors name the offending line. Take the top row of every
station to deck height: the section is closed by a flat deck there.

## What is computed

- **Hydrostatics** (upright, even keel): displacement, KB, KM, LCB, LCF,
  TPC, MCT1cm, from the station sections by Simpson integration.
- **GZ curve**: cross curves at *constant displacement*. Each heeled
  section is clipped exactly against the inclined waterline, the plane is
  solved so the displaced volume equals the upright volume, and
  `GZ = KN − (KG + FSC)·sin φ` with `KN = −y_B cos φ + z_B sin φ`.
- **IMO 2008 criteria**: areas to 30° and 40°, angle of maximum GZ and
  corrected GM, with a pass/fail row each. `--strict` turns a failure into
  exit status 1.

## Outputs

`--csv PREFIX` writes `PREFIX-hydrostatics.csv` (a table from 60 % to 140 %
of the design draft) and `PREFIX-gz.csv`. `--svg FILE` writes the GZ curve
with the initial-GM tangent and the 25°/30°/40° marks. `--json` prints the
whole report as one JSON object.

## How far to trust it

The Wigley hull has closed-form hydrostatics (`∇ = 4/9·L·B·T`,
`KB = 5T/8`, `BM = 3B²/(35T)`), and its waterline is wall-sided, so small
angle GZ follows `sin φ (GM + ½·BM·tan² φ)`. The test suite checks the
offsets path against both. Limits: symmetric hull, zero trim while heeling,
no superstructure or appendage buoyancy, and a flat deck at the top offset
(reserve buoyancy above it is ignored). The prismatic model has no deck
edge, so its GZ keeps rising with heel; the report warns when the maximum
is not resolved. Neither path is a class submission.
