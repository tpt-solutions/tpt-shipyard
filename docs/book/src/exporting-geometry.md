# Exporting Geometry

The `tpt-yard` facade writes the engine's render mesh
([`Geometry3D`](tpt_yard_core::Geometry3D) — vertices plus triangles,
metres) to the two interchange formats that matter for shipyards.

## glTF 2.0

[`geometry_to_gltf`](tpt_yard::export::geometry_to_gltf) emits a
self-contained `.gltf` document (embedded base64 buffer, TRIANGLES
mode, required POSITION min/max) that any three.js/Babylon/Blender
viewer loads — this is what the WASM dashboard and the published demo
consume.

## IFC (STEP)

[`elements_to_ifc`](tpt_yard::export::elements_to_ifc) writes an IFC4
STEP physical file (ISO 10303-21): the standard
Project/Site/Building/Storey spatial structure with one
IfcBuildingElementProxy per element, each mesh carried losslessly as an
IfcTriangulatedFaceSet. BIM tools (Revit, Blender, IFC.js) read it
directly.

```rust
use tpt_yard::export::{elements_to_ifc, IfcElement};
use tpt_yard_core::Geometry3D;

let block = Geometry3D::from_box(12.0, 3.0, 4.0);
let ifc = elements_to_ifc(
    &[IfcElement { name: "block 41", geometry: &block }],
    "quay 7",
);
assert!(ifc.starts_with("ISO-10303-21;"));
assert!(ifc.contains("IFCTRIANGULATEDFACESET"));
assert!(ifc.ends_with("END-ISO-10303-21;\n"));
```

The writer is dependency-free and deterministic: entity ids run
sequentially, GUIDs are synthetic 22-character IFC-alphabet strings
from a per-entity counter (not registry GUIDs), and the header
timestamp is fixed so re-exports diff cleanly. Scope notes: one
building storey per file, no property sets or materials — the
geometric interchange core.
