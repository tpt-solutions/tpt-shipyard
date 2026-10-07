//! `export`: the block division as glTF 2.0 and/or IFC4 STEP.

use tpt_yard_core::Vector3;

use crate::args::{self, CliError, Parsed};
use crate::project::load_manifest;

// ------------------------------------------------------------------ export

/// `export MANIFEST.json [--gltf out.gltf] [--ifc out.ifc]`
///
/// Divides the hull into blocks (the plan command's step 1) and writes
/// the erection geometry out: one merged-mesh glTF 2.0 document and/or
/// one IFC4 STEP file with a named product per block — the hand-off to
/// web viewers and BIM tools.
pub fn export(p: &Parsed, json_mode: bool) -> Result<(), CliError> {
    let path = p.pos(0).unwrap_or_default();
    let (gltf_out, ifc_out) = (p.value("gltf"), p.value("ifc"));
    if gltf_out.is_none() && ifc_out.is_none() {
        return Err(CliError::Usage(
            "export needs --gltf and/or --ifc with output paths".into(),
        ));
    }
    export_impl(path, gltf_out, ifc_out, p.has("force"), json_mode).map_err(CliError::from)
}

fn export_impl(
    path: &str,
    gltf_out: Option<&str>,
    ifc_out: Option<&str>,
    force: bool,
    json_mode: bool,
) -> Result<(), String> {
    // Manifest -> blocks (the same division the plan command reports).
    let m = load_manifest(path)?;
    let construction = tpt_yard::tpt_yard_hull::HullConstruction::new(m.hull);
    let blocks = construction.block_division(m.crane_kn, m.workshop);
    if blocks.is_empty() {
        return Err("block division produced no blocks".into());
    }

    // Per-block boxes at their geometric centres (the footprint the plan
    // command's lift points use); merged for glTF, individual for IFC.
    let mut merged = tpt_yard::tpt_yard_core::Geometry3D::new();
    let mut block_geometries: Vec<tpt_yard::tpt_yard_core::Geometry3D> =
        Vec::with_capacity(blocks.len());
    for block in &blocks {
        let d = block.dimensions();
        let mut box_geometry =
            tpt_yard::tpt_yard_core::Geometry3D::from_box(d.length, d.breadth, d.depth);
        box_geometry.translate(block.geometry.centre);
        merged.merge(&box_geometry, Vector3::ZERO);
        block_geometries.push(box_geometry);
    }
    let elements: Vec<tpt_yard::export::IfcElement> = blocks
        .iter()
        .zip(&block_geometries)
        .map(|(block, geometry)| tpt_yard::export::IfcElement {
            name: &block.name,
            geometry,
        })
        .collect();

    // (path, bytes) of every file written.
    let mut written: Vec<(String, usize)> = Vec::new();
    if let Some(out) = gltf_out {
        let doc = tpt_yard::export::try_geometry_to_gltf(&merged, path)
            .map_err(|e| format!("glTF export: {e}"))?;
        args::write_output(out, doc.as_bytes(), force)?;
        written.push((out.to_string(), doc.len()));
    }
    if let Some(out) = ifc_out {
        let doc = tpt_yard::export::elements_to_ifc(&elements, "hull erection");
        args::write_output(out, doc.as_bytes(), force)?;
        written.push((out.to_string(), doc.len()));
    }
    if json_mode {
        use tpt_yard_core::json::Value as J;
        let doc = J::Object(vec![
            ("blocks".into(), J::Number(blocks.len() as f64)),
            ("vertices".into(), J::Number(merged.vertices.len() as f64)),
            ("triangles".into(), J::Number(merged.faces.len() as f64)),
            (
                "written".into(),
                J::Array(written.iter().map(|(p, _)| J::String(p.clone())).collect()),
            ),
        ]);
        println!("{}", doc.to_string_compact());
        return Ok(());
    }
    println!(
        "Exported {} blocks ({} vertices, {} triangles):",
        blocks.len(),
        merged.vertices.len(),
        merged.faces.len()
    );
    for (p, n) in &written {
        println!("  - {p} ({n} bytes)");
    }
    Ok(())
}
