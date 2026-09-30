# Material & Resource Logistics

`tpt-yard-logistics` plans the material flow into (or out of) the
construction site: deliveries, staging areas, and transport scheduling. The
crate models a `MaterialFlow` of `MaterialItem`s — each with a quantity, a
delivery window, and a staging requirement — and provides resource-flow
scheduling helpers that answer the yard questions: what has to be on the
quay before erection of a block starts, and what is late.

The digital twin and the scheduler consume the flow to gate activities on
material availability; see the crate docs for the schema and worked calls.

> Status: simplified model — flow accounting with delivery windows; no
> traffic simulation or port layout solver yet.
