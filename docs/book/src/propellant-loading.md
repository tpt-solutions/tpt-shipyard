# Propellant Loading

`tpt-yard-propellant` plans cryogenic and storable propellant loading and
manages boil-off.

## Loading plans

[`LoadingPlanner::plan_loading`](tpt_yard_propellant::LoadingPlanner::plan_loading)
produces a [`PropellantLoadingPlan`](tpt_yard_propellant::PropellantLoadingPlan):

- **chilldown** — cryogenic tanks are chilled from ambient to the boiling
  point before useful fill (the first fill would otherwise flash to vapour);
  the time comes from the tank thermal mass against the latent heat absorbed
  by vented vapour;
- **fill rate and venting** — cryogenic fills vent; storables (MMH/NTO)
  need neither chilldown nor venting;
- **subcooling target** — 3 K below the boiling point for cryogenics.

## Boil-off management

[`boil_off_report`](tpt_yard_propellant::LoadingPlanner::boil_off_report)
converts the tank heat leak to boil-off through the latent heat
(kg/day and %/day), estimates time-to-vent from a cold soak, sizes the
cryocooler for zero-boil-off (ZBO at < 0.1 %/day), and flags which
[`BoilOffPolicy`](tpt_yard_propellant::BoilOffPolicy) the tank qualifies
for.

## Verification

- The cryogenic stability test balances the heat leak against the latent
  heat (2000 W on a 300 m³ LOX tank = 811 kg/day, vented policy).
- LH2 verified as the hardest cryo (lowest density and modest latent heat
  give the worst percentage boil-off).
- ZBO qualification tested both ways: a tank under the threshold qualifies,
  one over does not.
