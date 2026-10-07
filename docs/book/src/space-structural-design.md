# Space Structural Design

`tpt-yard-space-structural` implements the load cases that govern in orbit,
where launch constraints no longer exist. Model:
[RFC 0005](../../rfcs/0005-rotating-habitat-structural.md).

## The freedom statement

Assembled in orbit, a structure escapes the fairing, aerodynamics, and 1-g
handling entirely —
[`no_launch_constraint`](tpt_yard_space_structural::SpaceStructuralDesigner::no_launch_constraint)
returns that formal statement (`ShapeConstraint::None`, size unbounded) for
planners.

## Rotation

Hoop stress of a thin spinning ring:

```text
sigma = rho * omega^2 * r^2
```

with `omega = 2 pi rpm / 60`. Verified to machine precision against the
spec's reference case (2 rpm, 100 m, aluminium) and locked as golden data.
Radial growth from elastic strain is reported alongside.

## Thermal cycling

Orbit day/night swings drive a strain range `dE = alpha·dT`; life follows
the Coffin–Manson form with Miner's-rule life fractions. Ordering is tested:
higher expansion (aluminium) or lower ductility burns life faster.

## Micrometeoroid shielding

Whipple-shield screening sizes: bumper `d/8` mm, standoff `10·d` mm, rear
wall `0.4·d` mm for protected particle diameter `d`, with package areal
density. Environment flux converts to expected impacts per year per area.
