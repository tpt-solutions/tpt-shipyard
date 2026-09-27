# Habitat Design

`tpt-yard-habitat` answers the three rotating-habitat questions (RFC 0005).

## Spin rate

[`required_rotation`](tpt_yard_habitat::HabitatDesigner::required_rotation):
`omega = sqrt(g_target · 9.81 / r)` — verified exactly, including the
Mars-gravity scaling (`sqrt(0.38)` of the 1 g rate).

## Coriolis comfort

The 2 rpm guideline:
[`coriolis_effects`](tpt_yard_habitat::HabitatDesigner::coriolis_effects)
reports the cross-coupled acceleration for a 1 m/s head movement
(`a = 2·omega·v`, ~4.3 % of g at 2 rpm — below the ~10 % nausea threshold)
and flags rotations above the limit. The design tension is explicit in the
tests: 1 g at 100 m needs 2.99 rpm (above the limit); 1 g at 4 km needs
0.47 rpm (comfortable).

## Structure

[`structural_design`](tpt_yard_habitat::HabitatDesigner::structural_design)
sizes the spinning hull: hoop tension `T = m·omega²·r/(2π)` over the
allowable gives the hoop area; thickness follows from the circumference.
Self-consistency (utilization ≤ 1) and monotonicity in mass and safety
factor are unit-tested; the `sigma = rho·omega²·r²` self-stress check reuses
`tpt-yard-space-structural`.
