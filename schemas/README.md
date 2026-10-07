# tpt-shipyard JSON Schemas

Wire-format schemas for the engine's JSON documents (review 7C):

- `vessel-project.schema.json` — the `VesselProject` document the loader
  accepts (`VesselProject::from_json_str` / `from_json_value`). Projects
  serialize a `schema_version: 1` field (`VesselProject::SCHEMA_VERSION`);
  breaking wire changes bump it.
- `hull-block-manifest.schema.json` — block-division manifests
  (`test-data/hull-blocks/container-ship-140m.json`), input to
  `HullConstruction::block_division` and `tpt-yard plan`.
- `orbital-structure.schema.json` — assembly manifests
  (`test-data/orbital-structures/iss-truss-manifest.json`), input to
  `OrbitalAssembly` planning.
- `weld-procedure.schema.json` — WPS records in `test-data/welding-procedures/`
  (`WeldProcedure::from_json`).
- `stability-case.schema.json` — the case file read by `tpt-yard stability`
  (and written by `new-hull` / `import-hull`): a hull (offsets CSV or prismatic
  coefficients) plus a loading condition.
- `hull-offsets.table-schema.json` — the offsets CSV itself, as a
  [Frictionless Table Schema](https://specs.frictionlessdata.io/table-schema/)
  (`station_x_m, draft_m, half_breadth_m`; the file-level rules the schema
  cannot express — each station starts at the baseline, no repeated drafts —
  are in its description and enforced by `parse_offsets_csv`).

Two files in `test-data/` are reference parameter sets for the examples, not
engine wire formats, and are documented by their own contents instead of a
schema: `hull-blocks/submarine-pressure-hull.json` (pressure-hull section
layout for the `submarine-pressure-hull` example) and
`orbital-structures/rotating-habitat-torus.json` (RFC 0005 habitat
parameters for the `rotating-habitat-construction` example).

Validate a project against the schema with any draft-2020-12 validator, e.g.

```sh
npx ajv-cli validate -s schemas/vessel-project.schema.json -r schemas -- project.json
```

Note the loader enforces *more* than the schema: internal consistency
(unique ids, resolving dependencies, phase membership, finite numbers) is
checked by `VesselProject::validate()` on load.

The loaders are also stricter about shape than a schema validator must be:

- No silent defaults. A missing required field is a `MissingField` error —
  never a zero, empty string or default variant (vessel payloads, weight
  states, resource entries, item statuses, twin progress sets).
- Unknown enum values are errors, never mapped onto a default variant
  (`hull_type`, `propulsion`, resource kinds, item statuses).
- Entries inside arrays must be well-formed: a malformed dependency or
  resource entry fails the load instead of being dropped.
- A field of the wrong JSON type is a `TypeError`, not a `MissingField`.
- `schema_version`, when present, must be `1`; other versions are refused.
