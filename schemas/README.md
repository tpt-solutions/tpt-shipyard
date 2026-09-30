# tpt-shipyard JSON Schemas

Wire-format schemas for the engine's JSON documents (review 7C):

- `vessel-project.schema.json` — the `VesselProject` document the loader
  accepts (`VesselProject::from_json_str` / `from_json_value`). Projects
  serialize a `schema_version: 1` field (`VesselProject::SCHEMA_VERSION`);
  breaking wire changes bump it.
- `hull-block-manifest.schema.json`
- `orbital-structure.schema.json`
- `weld-procedure.schema.json` — WPS records in `test-data/welding-procedures/`
  (`WeldProcedure::from_json`).

Validate a project against the schema with any draft-2020-12 validator, e.g.

```sh
npx ajv-cli validate -s schemas/vessel-project.schema.json -r schemas -- project.json
```

Note the loader enforces *more* than the schema: internal consistency
(unique ids, resolving dependencies, phase membership, finite numbers) is
checked by `VesselProject::validate()` on load.
