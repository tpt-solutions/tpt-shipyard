# In-Space Manufacturing

`tpt-yard-space-manufacturing` plans additive construction in vacuum
(milestone example: `examples/space-solar-array-assembly/`).

## Print time and energy

[`print_time_estimate`](tpt_yard_space_manufacturing::InSpaceManufacturing::print_time_estimate):
mass = `rho · V · (1 + scrap)` over the deposition rate. ISRU feedstock
carries a processing scrap fraction. Total energy is mass times the
process intensity (WireArc 4 kWh/kg through PowderBed 12 kWh/kg).

## Thermal control in vacuum

No convection: deposition power must be rejected by radiation alone.
[`thermal_control_during_print`](tpt_yard_space_manufacturing::InSpaceManufacturing::thermal_control_during_print)
sizes the radiator `A = P / (eps · sigma · T^4)` (eps 0.85, 350 K) and
flags eclipse pauses.

## Quality in situ

[`quality_verification`](tpt_yard_space_manufacturing::InSpaceManufacturing::quality_verification):
100 % layer imaging, thermography + eddy current (the vacuum NDT subset),
witness coupons per kilogram, and destructive testing only when samples can
come home (not for ISRU builds).

## The pipeline

The example chains the printer into the orbital-assembly planner: printed
panel substrates become components; the robotic arm bolts the wing together;
the deployed cantilever integrity check closes the loop — manufacturing and
assembly in one plan.
