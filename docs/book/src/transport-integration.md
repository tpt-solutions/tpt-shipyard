# Transport Integration

`tpt-yard-transport-link` bridges vehicle design and construction — the
spec §6 contract, with the `tpt-transport` types vendored until that
substrate publishes.

## Design → construction

[`plan_construction`](tpt_yard_transport_link::plan_construction) maps a
[`VehicleDesign`](tpt_yard_transport_link::VehicleDesign) to a
[`VesselProject`](tpt_yard_core::VesselProject):

- Maritime designs become `SeaDrydock` projects with the five-stage
  skeleton (steel → blocks → dock erection → outfitting → launch & trials);
- Spacecraft designed for orbital assembly become `OrbitalAssembly`
  projects with the fabrication/rendezvous/assembly/verification skeleton.

## Build → operations

[`handover_to_operations`](tpt_yard_transport_link::handover_to_operations)
extracts the as-built truth from a finished twin: installed weight, as-built
CoG, the structural connection summary, and the accepted quality records —
the [`AsBuiltProperties`](tpt_yard_transport_link::AsBuiltProperties) the
operational model consumes instead of the design numbers.

## Verification

`round_trip_design_build_handover` runs the full loop: design a ship, fill
the twin with activities and weight items, advance all five phases, and
assert the handover carries exactly the installed mass (5,000 kg) and the
closed-form CoG (x = 60 m).
