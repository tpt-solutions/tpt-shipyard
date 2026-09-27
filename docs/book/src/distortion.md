# Distortion Management

`tpt-yard-distortion` closes the loop after welding: compare the as-measured
block against target geometry and plan the correction.

## Deviation map

[`DistortionControl::deviation_map`](tpt_yard_distortion::DistortionControl::deviation_map)
pairs measured and target vertices (identical topology required) and reports
per-vertex signed deviation (dominant axis, mm), max, and RMS.

## Correction policy

[`correction_plan(tol_mm)`](tpt_yard_distortion::DistortionControl::correction_plan)
applies tolerance thresholds (unit-tested):

```text
|deviation| ≤ tol                  → no action (single Accept if all pass)
tol < |deviation| ≤ 5·tol          → HeatStraighten { location, heat_input }
|deviation| > 5·tol                → Reject { reason }
> 20 % of vertices out of tol      → Rework (systematic distortion)
```

The plan is appended to `corrections`, giving the block's correction
history — which feeds the digital twin's as-built geometry and the transport
handover.

## Joint geometry support

Joint shapes (butt V/X, fillets, T-joints, lap, corner) and their derived
quantities (groove area, effective throat, weld volume and mass) live in
`tpt-yard-joints`, shared by the welding and structural crates.
