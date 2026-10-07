//! Export adapters for the engine's geometry (review 7H: glTF export;
//! 2026-10-02: IFC export).
//!
//! glTF 2.0 is a JSON scene description with base64- or buffer-referenced
//! binary data; the simplest interoperable form is a *glTF with embedded
//! base64 buffer*, which every three.js/Babylon/Blender viewer loads.
//!
//! [`elements_to_ifc`] writes the same geometry as an IFC4 STEP
//! physical file (ISO 10303-21) — the BIM interchange format that
//! Blender/Revit/IFC.js load. The mesh ships losslessly as an
//! IfcTriangulatedFaceSet inside the standard Project/Site/Building/
//! Storey spatial structure. Scope, documented honestly: deterministic
//! synthetic GUIDs (a per-entity counter, not registry GUIDs), a fixed
//! header timestamp, one building storey, and no property sets — the
//! geometric interchange core, not an authoring application.

use tpt_yard_core::{Geometry3D, Vector3};

/// Escapes text for a JSON string literal body.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Why a mesh cannot be written as a valid glTF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfError {
    /// No vertices or no triangles (a glTF primitive needs both).
    EmptyGeometry,
    /// A vertex coordinate is NaN or infinite.
    NonFiniteCoordinate,
    /// A triangle indexes a vertex that does not exist.
    IndexOutOfRange,
}

impl std::fmt::Display for GltfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GltfError::EmptyGeometry => "the geometry has no vertices or no triangles",
            GltfError::NonFiniteCoordinate => "a vertex coordinate is NaN or infinite",
            GltfError::IndexOutOfRange => "a triangle indexes a vertex that does not exist",
        })
    }
}

impl std::error::Error for GltfError {}

/// Writes `geometry` as a glTF 2.0 document, refusing meshes that would
/// produce an invalid file (empty, non-finite coordinates, out-of-range
/// indices). The mesh is z-up (the engine's convention); glTF is y-up, so
/// the node carries a -90 degree rotation about X that stands the model upright
/// in any viewer. Accessor bounds are computed from the f32 values actually
/// stored, and the name is JSON-escaped.
///
/// # Errors
///
/// [`GltfError`] for an unwritable mesh.
pub fn try_geometry_to_gltf(geometry: &Geometry3D, name: &str) -> Result<String, GltfError> {
    if geometry.vertices.is_empty() || geometry.faces.is_empty() {
        return Err(GltfError::EmptyGeometry);
    }
    if geometry
        .vertices
        .iter()
        .any(|v| !(v.x.is_finite() && v.y.is_finite() && v.z.is_finite()))
    {
        return Err(GltfError::NonFiniteCoordinate);
    }
    let n = geometry.vertices.len();
    if geometry.faces.iter().flatten().any(|&i| i as usize >= n) {
        return Err(GltfError::IndexOutOfRange);
    }
    Ok(write_gltf(geometry, name))
}

/// Writes `geometry` as a glTF 2.0 JSON string for *valid* meshes — see
/// [`try_geometry_to_gltf`], which checks that and is what callers with
/// untrusted geometry should use. Non-finite coordinates are written as 0
/// so the document stays valid JSON.
pub fn geometry_to_gltf(geometry: &Geometry3D, name: &str) -> String {
    write_gltf(geometry, name)
}

fn write_gltf(geometry: &Geometry3D, name: &str) -> String {
    let name = json_escape(name);
    let finite = |x: f64| if x.is_finite() { x } else { 0.0 };
    // Binary chunk: positions (f32 xyz per vertex), then indices (u32).
    let mut bin: Vec<u8> = Vec::new();
    for v in &geometry.vertices {
        bin.extend_from_slice(&(finite(v.x) as f32).to_le_bytes());
        bin.extend_from_slice(&(finite(v.y) as f32).to_le_bytes());
        bin.extend_from_slice(&(finite(v.z) as f32).to_le_bytes());
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let indices_offset = bin.len();
    for f in &geometry.faces {
        bin.extend_from_slice(&f[0].to_le_bytes());
        bin.extend_from_slice(&f[1].to_le_bytes());
        bin.extend_from_slice(&f[2].to_le_bytes());
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let bin_len = bin.len();

    // Base64-encode.
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut b64 = String::with_capacity(bin_len.div_ceil(3) * 4);
    for chunk in bin.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        b64.push(TABLE[(b[0] >> 2) as usize] as char);
        b64.push(TABLE[(((b[0] & 0x03) << 4) | (b[1] >> 4)) as usize] as char);
        b64.push(if chunk.len() > 1 {
            TABLE[(((b[1] & 0x0F) << 2) | (b[2] >> 6)) as usize] as char
        } else {
            '='
        });
        b64.push(if chunk.len() > 2 {
            TABLE[(b[2] & 0x3F) as usize] as char
        } else {
            '='
        });
    }

    let positions_byte_length = indices_offset;
    let indices_byte_length = bin_len - indices_offset;

    // Min/max position accessors (glTF requires them for POSITION), taken
    // from the f32 values that are actually in the buffer.
    let mut min = [f64::MAX; 3];
    let mut max = [f64::MIN; 3];
    for v in &geometry.vertices {
        for (i, c) in [v.x, v.y, v.z].iter().enumerate() {
            let stored = f64::from(finite(*c) as f32);
            min[i] = min[i].min(stored);
            max[i] = max[i].max(stored);
        }
    }
    if geometry.vertices.is_empty() {
        min = [0.0; 3];
        max = [0.0; 3];
    }
    let max_index = geometry
        .faces
        .iter()
        .flat_map(|f| f.iter())
        .copied()
        .max()
        .unwrap_or(0);
    let min_index = geometry
        .faces
        .iter()
        .flat_map(|f| f.iter())
        .copied()
        .min()
        .unwrap_or(0);

    format!(
        r#"{{"asset":{{"version":"2.0","generator":"tpt-shipyard"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0,"name":"{name}","rotation":[-0.7071067811865476,0,0,0.7071067811865476]}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"mode":4}}]}}],"buffers":[{{"uri":"data:application/octet-stream;base64,{b64}","byteLength":{bin_len}}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":{positions_byte_length},"target":34962}},{{"buffer":0,"byteOffset":{indices_offset},"byteLength":{indices_byte_length},"target":34963}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":{v_count},"type":"VEC3","min":[{min_x},{min_y},{min_z}],"max":[{max_x},{max_y},{max_z}]}},{{"bufferView":1,"componentType":5125,"count":{i_count},"type":"SCALAR","min":[{min_index}],"max":[{max_index}]}}]}}"#,
        v_count = geometry.vertices.len(),
        i_count = geometry.faces.len() * 3,
        min_x = min[0],
        min_y = min[1],
        min_z = min[2],
        max_x = max[0],
        max_y = max[1],
        max_z = max[2],
    )
}

/// Decodes a base64 string back to bytes (the reader's counterpart to
/// the writer's encoder).
fn decode_base64(s: &str) -> Vec<u8> {
    fn val(c: u8) -> u32 {
        match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
    let s = s.trim_end_matches('=');
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for chunk in s.as_bytes().chunks(4) {
        let n = chunk.iter().fold(0u32, |acc, &c| (acc << 6) | val(c));
        let bytes = n.to_be_bytes();
        out.extend_from_slice(&bytes[1..chunk.len()]);
    }
    out
}

// ------------------------------------------------------------------- IFC

/// An element to export into the IFC spatial structure.
#[derive(Debug, Clone, Copy)]
pub struct IfcElement<'a> {
    /// Product name (written into the IfcBuildingElementProxy Name).
    pub name: &'a str,
    /// Triangle mesh, m (lostlessly stored as an IfcTriangulatedFaceSet).
    pub geometry: &'a Geometry3D,
}

/// Formats a float as an ISO 10303-21 real: the shortest round-trip
/// representation, forced to contain a decimal point (STEP reals need
/// one; Rust prints `0` for `0.0`).
fn step_real(v: f64) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') || s.contains('E') {
        if s.contains('.') {
            s
        } else {
            // "1e300" style: give the mantissa its decimal point.
            let (m, e) = s.split_once(['e', 'E']).expect("checked e above");
            format!("{m}.0e{e}")
        }
    } else {
        format!("{s}.0")
    }
}

/// STEP string literal (ISO 10303-21): single quotes doubled, backslash
/// doubled (it is the escape character), and every non-ASCII character
/// encoded as `\X2\hhhh\X0\` (BMP) or `\X4\hhhhhhhh\X0\` (beyond it).
fn step_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        match c {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            c if (c as u32) <= 0xFFFF => out.push_str(&format!("\\X2\\{:04X}\\X0\\", c as u32)),
            c => out.push_str(&format!("\\X4\\{:08X}\\X0\\", c as u32)),
        }
    }
    out.push('\'');
    out
}

/// True when every edge is shared by exactly two triangles (a closed
/// surface); only then may an IfcTriangulatedFaceSet claim `Closed`.
fn is_closed_mesh(faces: &[[u32; 3]]) -> bool {
    use std::collections::HashMap;
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
    for f in faces {
        for k in 0..3 {
            let (a, b) = (f[k], f[(k + 1) % 3]);
            *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    !edges.is_empty() && edges.values().all(|&n| n == 2)
}

/// The 22-character compressed IfcGloballyUniqueId for a counter value
/// (128-bit, base-64 over the IFC alphabet `0-9A-Za-z_$`). Deterministic
/// per entity order — synthetic ids, not registry GUIDs.
fn ifc_guid(counter: u128) -> String {
    const CHARS: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let mut out = [b'0'; 22];
    let mut v = counter;
    for slot in out.iter_mut().rev() {
        *slot = CHARS[(v & 63) as usize];
        v >>= 6;
    }
    String::from_utf8(out.to_vec()).expect("alphabet is ASCII")
}

/// Sequential STEP entity writer: ids run 1..n in dependency order.
struct StepWriter {
    lines: Vec<String>,
}

impl StepWriter {
    fn entity(&mut self, attrs: String) -> usize {
        self.lines
            .push(format!("#{}={};", self.lines.len() + 1, attrs));
        self.lines.len()
    }
}

/// Writes `elements` as a minimal IFC4 STEP document: the
/// Project/Site/Building/Storey spatial structure plus one
/// IfcBuildingElementProxy per element, its mesh carried losslessly as
/// an IfcTriangulatedFaceSet (IfcCartesianPointList3D coordinates,
/// 1-based triangle indices). Returns the `.ifc` text — write it to a
/// `<project>.ifc` file.
pub fn elements_to_ifc(elements: &[IfcElement<'_>], project_name: &str) -> String {
    let mut w = StepWriter { lines: Vec::new() };
    let mut guid = 0_u128;

    // Spatial skeleton: context/units, project, site, building, storey.
    let origin_pt = w.entity("IFCCARTESIANPOINT((0.,0.,0.))".into());
    let world_axis = w.entity(format!("IFCAXIS2PLACEMENT3D(#{origin_pt},$,$)"));
    let context = w.entity(format!(
        "IFCGEOMETRICREPRESENTATIONCONTEXT($,$,'Model',3,1.0E-6,#{world_axis},$)"
    ));
    let unit_len = w.entity("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)".into());
    let unit_area = w.entity("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)".into());
    let unit_vol = w.entity("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)".into());
    let unit_angle = w.entity("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)".into());
    let units = w.entity(format!(
        "IFCUNITASSIGNMENT((#{unit_len},#{unit_area},#{unit_vol},#{unit_angle}))"
    ));
    guid += 1;
    let project = w.entity(format!(
        "IFCPROJECT({},$,{},{},{},{},{},(#{context}),#{units},$)",
        ifc_guid(guid),
        step_string(project_name),
        step_string(project_name),
        step_string(""),
        step_string(project_name),
        step_string("construction"),
    ));
    let site_placement = w.entity(format!("IFCLOCALPLACEMENT($,#{world_axis})"));
    guid += 1;
    let site = w.entity(format!(
        "IFCSITE({},$,{},{},{},{},#{},$,$,.ELEMENT.,$,$,0.,$,$)",
        ifc_guid(guid),
        step_string("site"),
        step_string("site"),
        step_string("site"),
        step_string("launch site"),
        site_placement,
    ));
    let building_placement = w.entity(format!(
        "IFCLOCALPLACEMENT(#{site_placement},#{world_axis})"
    ));
    guid += 1;
    let building = w.entity(format!(
        "IFCBUILDING({},$,{},{},{},{},#{},$,$,.ELEMENT.,$,$,$)",
        ifc_guid(guid),
        step_string("building"),
        step_string("building"),
        step_string("building"),
        step_string("assembly building"),
        building_placement,
    ));
    let storey_placement = w.entity(format!(
        "IFCLOCALPLACEMENT(#{building_placement},#{world_axis})"
    ));
    guid += 1;
    let storey = w.entity(format!(
        "IFCBUILDINGSTOREY({},$,{},{},{},{},#{},$,$,.ELEMENT.,0.)",
        ifc_guid(guid),
        step_string("construction stage"),
        step_string("construction stage"),
        step_string("construction stage"),
        step_string("current construction stage"),
        storey_placement,
    ));
    guid += 1;
    w.entity(format!(
        "IFCRELAGGREGATES({},$,$,$,#{project},(#{site}))",
        ifc_guid(guid)
    ));
    guid += 1;
    w.entity(format!(
        "IFCRELAGGREGATES({},$,$,$,#{site},(#{building}))",
        ifc_guid(guid)
    ));
    guid += 1;
    w.entity(format!(
        "IFCRELAGGREGATES({},$,$,$,#{building},(#{storey}))",
        ifc_guid(guid)
    ));

    // Products: one proxy per element, mesh as a tessellated face set.
    let mut product_ids: Vec<usize> = Vec::with_capacity(elements.len());
    for element in elements {
        let coords: Vec<String> = element
            .geometry
            .vertices
            .iter()
            .map(|v: &Vector3| {
                format!("({},{},{})", step_real(v.x), step_real(v.y), step_real(v.z))
            })
            .collect();
        let point_list = w.entity(format!("IFCCARTESIANPOINTLIST3D(({}))", coords.join(",")));
        let faces: Vec<String> = element
            .geometry
            .faces
            .iter()
            .map(|f| {
                format!(
                    "({},{},{})",
                    u64::from(f[0]) + 1,
                    u64::from(f[1]) + 1,
                    u64::from(f[2]) + 1
                )
            })
            .collect();
        let face_set = w.entity(format!(
            "IFCTRIANGULATEDFACESET(#{point_list},$,{},({}),$)",
            if is_closed_mesh(&element.geometry.faces) {
                ".T."
            } else {
                ".F."
            },
            faces.join(",")
        ));
        let shape_rep = w.entity(format!(
            "IFCSHAPEREPRESENTATION(#{context},'Body','Tessellation',(#{face_set}))"
        ));
        let product_shape = w.entity(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{shape_rep}))"));
        let placement = w.entity(format!(
            "IFCLOCALPLACEMENT(#{storey_placement},#{world_axis})"
        ));
        guid += 1;
        product_ids.push(w.entity(format!(
            "IFCBUILDINGELEMENTPROXY({},$,{},{},{},#{},#{},$,$)",
            ifc_guid(guid),
            step_string(element.name),
            step_string(element.name),
            step_string("tpt-shipyard block"),
            placement,
            product_shape,
        )));
    }

    // Containment: every product into the storey (the relation requires
    // at least one related element, so it is skipped with none).
    if !product_ids.is_empty() {
        let contained: Vec<String> = product_ids.iter().map(|p| format!("#{p}")).collect();
        guid += 1;
        w.entity(format!(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE({},$,$,$,({}),#{storey})",
            ifc_guid(guid),
            contained.join(",")
        ));
    }

    let mut out = String::with_capacity(w.lines.iter().map(|l| l.len() + 1).sum::<usize>() + 320);
    out.push_str("ISO-10303-21;\nHEADER;\n");
    out.push_str("FILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\n");
    out.push_str(&format!(
        "FILE_NAME({},{},('tpt-shipyard'),('TPT Solutions'),'tpt-shipyard','','');\n",
        step_string(&format!("{project_name}.ifc")),
        step_string("2026-01-01T00:00:00"),
    ));
    out.push_str("FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n");
    for line in &w.lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    out
}

/// Writes a single named `geometry` as an IFC4 document with one
/// product — the one-element form of [`elements_to_ifc`].
pub fn geometry_to_ifc(geometry: &Geometry3D, name: &str) -> String {
    elements_to_ifc(&[IfcElement { name, geometry }], name)
}

/// Parses a glTF document written by [`geometry_to_gltf`] back into a
/// `Geometry3D`: the embedded base64 buffer decodes into the POSITION
/// (f32) and index (u32) views named by the two accessors. Exact for
/// f32-representable coordinates — the round trip is tested.
///
/// # Errors
///
/// A message when the JSON is malformed or not the writer's layout
/// (embedded buffer, POSITION accessor 0, indices accessor 1).
pub fn geometry_from_gltf(text: &str) -> Result<Geometry3D, String> {
    let v = tpt_yard_core::json::Value::parse(text).map_err(|e| format!("gltf JSON: {e}"))?;
    let uri = v
        .get("buffers")
        .and_then(|b| b.as_array())
        .and_then(|b| b.first())
        .and_then(|b| b.get("uri"))
        .and_then(|u| u.as_str())
        .ok_or("gltf: embedded buffer missing")?;
    let b64 = uri
        .strip_prefix("data:application/octet-stream;base64,")
        .ok_or("gltf: buffer is not an embedded base64 data URI")?;
    let bin = decode_base64(b64);
    let accessors = v.get("accessors").ok_or("gltf: accessors missing")?;
    let accessors = accessors.as_array().ok_or("gltf: accessors not a list")?;
    let positions = accessors
        .first()
        .and_then(|a| a.get("count"))
        .and_then(|n| n.as_u64())
        .ok_or("gltf: POSITION count missing")? as usize;
    let index_count = accessors
        .get(1)
        .and_then(|a| a.get("count"))
        .and_then(|n| n.as_u64())
        .ok_or("gltf: indices count missing")? as usize;
    let views = v
        .get("bufferViews")
        .and_then(|b| b.as_array())
        .ok_or("gltf: bufferViews missing")?;
    let pos_offset = views
        .first()
        .and_then(|b| b.get("byteOffset"))
        .and_then(|n| n.as_u64())
        .ok_or("gltf: POSITION bufferView offset missing")? as usize;
    let idx_offset = views
        .get(1)
        .and_then(|b| b.get("byteOffset"))
        .and_then(|n| n.as_u64())
        .ok_or("gltf: index bufferView offset missing")? as usize;

    // The declared counts are untrusted: check that the views fit the
    // decoded buffer before allocating or slicing.
    let face_count = index_count / 3;
    let fits = |offset: usize, n: usize| {
        n.checked_mul(12)
            .and_then(|len| offset.checked_add(len))
            .is_some_and(|end| end <= bin.len())
    };
    if !fits(pos_offset, positions) {
        return Err("gltf: POSITION view exceeds the buffer".into());
    }
    if !fits(idx_offset, face_count) {
        return Err("gltf: index view exceeds the buffer".into());
    }
    let word = |o: usize| -> [u8; 4] { [bin[o], bin[o + 1], bin[o + 2], bin[o + 3]] };

    let mut vertices = Vec::with_capacity(positions);
    for i in 0..positions {
        let o = pos_offset + i * 12;
        let x = f32::from_le_bytes(word(o)) as f64;
        let y = f32::from_le_bytes(word(o + 4)) as f64;
        let z = f32::from_le_bytes(word(o + 8)) as f64;
        vertices.push(Vector3::new(x, y, z));
    }
    let mut faces = Vec::with_capacity(face_count);
    for i in 0..face_count {
        let o = idx_offset + i * 12;
        let tri = [
            u32::from_le_bytes(word(o)),
            u32::from_le_bytes(word(o + 4)),
            u32::from_le_bytes(word(o + 8)),
        ];
        if tri.iter().any(|&ix| ix as usize >= positions) {
            return Err(format!("gltf: face {i} indexes a vertex out of range"));
        }
        faces.push(tri);
    }
    Ok(Geometry3D { vertices, faces })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collects `(id, line_body)` pairs from the DATA section and the
    /// set of referenced ids. A `#N` at the start of a line (before the
    /// `=`) defines; every other occurrence references.
    fn parse_step(text: &str) -> (Vec<(usize, &str)>, Vec<usize>) {
        let mut defs = Vec::new();
        let mut refs = Vec::new();
        for line in text.lines() {
            let Some(rest) = line.strip_prefix('#') else {
                continue;
            };
            let Some(eq) = rest.find('=') else { continue };
            let id: usize = rest[..eq].parse().expect("definition id");
            defs.push((id, &rest[eq + 1..]));
            let body = &rest[eq + 1..];
            let bytes = body.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'#' {
                    let start = i + 1;
                    let mut j = start;
                    while j < bytes.len() && bytes[j].is_ascii_digit() {
                        j += 1;
                    }
                    if j > start {
                        refs.push(body[start..j].parse().expect("ref id"));
                    }
                    i = j;
                } else {
                    i += 1;
                }
            }
        }
        (defs, refs)
    }

    /// The glTF reader is the writer's exact inverse for
    /// f32-representable coordinates: export a box, parse it back,
    /// compare every vertex and face.
    #[test]
    fn gltf_reader_round_trips_the_writer() {
        let g = Geometry3D::from_box(2.5, 3.5, 7.25);
        let text = geometry_to_gltf(&g, "round trip");
        let back = geometry_from_gltf(&text).expect("the writer's layout parses");
        assert_eq!(back.vertices.len(), g.vertices.len());
        for (a, b) in back.vertices.iter().zip(&g.vertices) {
            assert_eq!(a.x, b.x as f32 as f64, "x round-trips through f32");
            assert_eq!(a.y, b.y as f32 as f64, "y round-trips through f32");
            assert_eq!(a.z, b.z as f32 as f64, "z round-trips through f32");
        }
        assert_eq!(back.faces, g.faces);
        // Malformed input is a message, not a panic.
        assert!(geometry_from_gltf("{}").is_err());
        assert!(geometry_from_gltf("not json").is_err());
    }

    /// Declared counts larger than the embedded buffer (truncated file,
    /// hostile count) are errors, not slice panics or huge allocations.
    #[test]
    fn gltf_reader_rejects_counts_beyond_the_buffer() {
        let g = Geometry3D::from_box(1.0, 1.0, 1.0);
        let text = geometry_to_gltf(&g, "bad counts");
        let n = g.vertices.len();
        let inflated = text.replacen(&format!("\"count\":{n}"), "\"count\":4000000000", 1);
        assert_ne!(inflated, text, "the POSITION count was rewritten");
        assert!(geometry_from_gltf(&inflated).is_err());
    }

    /// The IFC document is a well-formed STEP physical file: markers in
    /// order, entity ids sequential from 1, and every referenced id in
    /// range.
    #[test]
    fn ifc_structure_is_wellformed() {
        let g = Geometry3D::from_box(2.0, 3.0, 4.0);
        let text = geometry_to_ifc(&g, "test block");
        for marker in [
            "ISO-10303-21;",
            "HEADER;",
            "FILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');",
            "FILE_SCHEMA(('IFC4'));",
            "DATA;",
            "ENDSEC;",
            "END-ISO-10303-21;",
        ] {
            assert!(text.contains(marker), "missing {marker}");
        }
        let (defs, refs) = parse_step(&text);
        assert!(!defs.is_empty());
        for (want, (got, _)) in defs.iter().enumerate() {
            assert_eq!(*got, want + 1, "entity ids must run 1..n in order");
        }
        let max = defs.last().map(|(id, _)| *id).expect("non-empty");
        for r in &refs {
            assert!(*r >= 1 && *r <= max, "reference #{r} out of range");
        }
        // STEP reals all carry a decimal point (spot check the origin).
        assert!(text.contains("IFCCARTESIANPOINT((0.,0.,0.))"));
    }

    /// The mesh round-trips exactly: the emitted Cartesian point list
    /// and 1-based triangle indices parse back to the very same f64
    /// vertices and faces (Rust's shortest round-trip formatting).
    #[test]
    fn ifc_geometry_round_trips_exactly() {
        let g = Geometry3D::from_box(2.5, 3.5, 7.25);
        let text = geometry_to_ifc(&g, "round trip");
        let point_line = text
            .lines()
            .find(|l| l.contains("IFCCARTESIANPOINTLIST3D"))
            .expect("point list");
        let inner = point_line
            .split_once("IFCCARTESIANPOINTLIST3D(")
            .expect("prefix")
            .1
            .trim_end_matches(';')
            .trim_start_matches('(')
            .trim_end_matches(')');
        let mut vertices = Vec::new();
        for tuple in inner.split("),(") {
            let t = tuple.trim_matches(['(', ')']);
            let mut c = t.split(',');
            vertices.push(Vector3::new(
                c.next().expect("x").parse().expect("x f64"),
                c.next().expect("y").parse().expect("y f64"),
                c.next().expect("z").parse().expect("z f64"),
            ));
        }
        assert_eq!(vertices.len(), g.vertices.len());
        for (a, b) in vertices.iter().zip(&g.vertices) {
            assert_eq!(a.x, b.x, "exact x");
            assert_eq!(a.y, b.y, "exact y");
            assert_eq!(a.z, b.z, "exact z");
        }
        let face_line = text
            .lines()
            .find(|l| l.contains("IFCTRIANGULATEDFACESET"))
            .expect("face set");
        let inner = face_line
            .split_once(".T.,(")
            .expect("coord index start")
            .1
            .rsplit_once("),$")
            .expect("coord index end")
            .0
            .trim_start_matches('(')
            .trim_end_matches(')');
        let mut faces = Vec::new();
        for tuple in inner.split("),(") {
            let mut c = tuple.trim_matches(['(', ')']).split(',');
            faces.push([
                c.next().expect("i").parse::<u32>().expect("i") - 1,
                c.next().expect("j").parse::<u32>().expect("j") - 1,
                c.next().expect("k").parse::<u32>().expect("k") - 1,
            ]);
        }
        assert_eq!(faces.len(), g.faces.len());
        assert!(faces.iter().zip(&g.faces).all(|(a, b)| a == b));
        // A box is closed: 8 points, 12 triangles.
        assert_eq!(vertices.len(), 8);
        assert_eq!(faces.len(), 12);
    }

    /// Multiple products appear in one containment relation with
    /// STEP-escaped names, every GUID is a 22-char IFC-alphabet string,
    /// and the document is deterministic.
    #[test]
    fn ifc_products_containment_and_determinism() {
        let a = Geometry3D::from_box(2.0, 2.0, 2.0);
        let b = Geometry3D::from_box(1.0, 1.0, 4.0);
        let elements = [
            IfcElement {
                name: "block 'A'",
                geometry: &a,
            },
            IfcElement {
                name: "block B",
                geometry: &b,
            },
        ];
        let text = elements_to_ifc(&elements, "quay 7");
        assert!(text.contains("'block ''A'''"), "STEP-escaped name");
        assert!(text.contains("'block B'"));
        assert_eq!(text.matches("IFCBUILDINGELEMENTPROXY").count(), 2);
        let containment = text
            .lines()
            .find(|l| l.contains("IFCRELCONTAINEDINSPATIALSTRUCTURE"))
            .expect("containment");
        let (defs, _) = parse_step(&text);
        let proxies: Vec<usize> = defs
            .iter()
            .filter(|(_, body)| body.contains("IFCBUILDINGELEMENTPROXY"))
            .map(|(id, _)| *id)
            .collect();
        for p in &proxies {
            assert!(containment.contains(&format!("#{p}")), "#{p} contained");
        }
        for (id, body) in &defs {
            if let Some(first) = body.strip_prefix('\'') {
                let end = first.find('\'').expect("closing quote");
                let guid = &first[..end];
                assert_eq!(guid.len(), 22, "guid length on entity {id}");
                assert!(
                    guid.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '$' || c == '_'),
                    "guid {guid} on entity {id}"
                );
            }
        }
        assert_eq!(text, elements_to_ifc(&elements, "quay 7"));
    }

    /// No products means no containment relation (it requires at least
    /// one element); an empty mesh exports empty lists without breaking
    /// the structure.
    #[test]
    fn ifc_edge_cases() {
        let text = elements_to_ifc(&[], "empty project");
        assert!(!text.contains("IFCRELCONTAINEDINSPATIALSTRUCTURE"));
        assert!(text.contains("IFCPROJECT"));
        let (defs, refs) = parse_step(&text);
        for r in &refs {
            assert!(defs.iter().any(|(id, _)| id == r), "#{r} defined");
        }

        let empty = Geometry3D::new();
        let text = geometry_to_ifc(&empty, "void");
        assert!(text.contains("IFCCARTESIANPOINTLIST3D(())"));
    }

    /// The exported document contains the required glTF keys with the right
    /// counts, and the embedded base64 buffer decodes to exactly the
    /// position + index bytes.
    #[test]
    fn gltf_export_structure() {
        let g = Geometry3D::from_box(2.0, 2.0, 2.0);
        let text = geometry_to_gltf(&g, "test box");

        // Required keys present.
        for key in [
            r#""asset":{"version":"2.0""#,
            r#""meshes":"#,
            r#""mode":4"#,
            r#""componentType":5126"#,
            r#""componentType":5125"#,
            r#""type":"VEC3""#,
            r#""type":"SCALAR""#,
            r#""name":"test box""#,
        ] {
            assert!(text.contains(key), "missing {key}");
        }
        // Counts: 8 vertices, 12 triangles = 36 indices.
        assert!(text.contains(r#""count":8"#));
        assert!(text.contains(r#""count":36"#));

        // Buffer decodes: 8 verts x 12 bytes + 36 indices x 4 bytes.
        let expected_len = 8 * 12 + 36 * 4;
        let uri_start = text.find("base64,").expect("embedded buffer") + "base64,".len();
        let uri_end = text[uri_start..].find('"').expect("uri end") + uri_start;
        let decoded = decode_base64(&text[uri_start..uri_end]);
        assert_eq!(decoded.len(), expected_len);
        // First vertex is (-1, -1, -1) for a centred 2 m box.
        let x = f32::from_le_bytes(decoded[0..4].try_into().unwrap());
        assert!((x - (-1.0)).abs() < 1e-6, "{x}");
    }
}

#[cfg(test)]
mod phase8_export_tests {
    use super::*;

    fn tetra() -> Geometry3D {
        Geometry3D {
            vertices: vec![
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector3::new(0.0, 0.0, 1.0),
            ],
            faces: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]],
        }
    }

    /// 8B20: a name with quotes, backslashes and control characters (a
    /// Windows path) is escaped, so the document is valid JSON and the
    /// name survives.
    #[test]
    fn gltf_name_is_escaped() {
        let name = "C:\\yard \"A\"\\n1.json";
        let doc = geometry_to_gltf(&tetra(), name);
        let v = tpt_yard_core::json::Value::parse(&doc).expect("valid JSON");
        let got = v
            .get("nodes")
            .and_then(|n| n.as_array())
            .and_then(|n| n[0].get("name"))
            .and_then(|n| n.as_str())
            .expect("name");
        assert_eq!(got, name);
    }

    /// 8B20: bounds come from the f32 values in the buffer, the index
    /// accessor reports its real minimum, and the node stands the z-up mesh
    /// upright for y-up viewers.
    #[test]
    fn gltf_bounds_match_the_stored_f32_values() {
        let g = Geometry3D {
            vertices: vec![
                Vector3::new(0.1, 0.2, 0.3),
                Vector3::new(1.7, 0.2, 0.3),
                Vector3::new(0.1, 2.9, 0.3),
                Vector3::new(0.1, 0.2, 4.1),
            ],
            faces: vec![[1, 2, 3], [1, 2, 3]],
        };
        let doc = try_geometry_to_gltf(&g, "t").expect("valid");
        let v = tpt_yard_core::json::Value::parse(&doc).unwrap();
        let acc = v.get("accessors").and_then(|a| a.as_array()).unwrap();
        let min = acc[0].get("min").and_then(|m| m.as_array()).unwrap();
        assert_eq!(min[0].as_f64().unwrap(), f64::from(0.1_f64 as f32));
        let max = acc[0].get("max").and_then(|m| m.as_array()).unwrap();
        assert_eq!(max[2].as_f64().unwrap(), f64::from(4.1_f64 as f32));
        // Indices 1..=3: the accessor must not claim a minimum of 0.
        let imin = acc[1].get("min").and_then(|m| m.as_array()).unwrap();
        assert_eq!(imin[0].as_f64(), Some(1.0));
        let node = &v.get("nodes").and_then(|n| n.as_array()).unwrap()[0];
        let rot = node
            .get("rotation")
            .and_then(|r| r.as_array())
            .expect("rotation");
        assert!((rot[0].as_f64().unwrap() + 0.5_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn unwritable_meshes_are_refused() {
        let empty = Geometry3D {
            vertices: vec![],
            faces: vec![],
        };
        assert_eq!(
            try_geometry_to_gltf(&empty, "e"),
            Err(GltfError::EmptyGeometry)
        );
        let mut nan = tetra();
        nan.vertices[1].x = f64::NAN;
        assert_eq!(
            try_geometry_to_gltf(&nan, "n"),
            Err(GltfError::NonFiniteCoordinate)
        );
        let mut oob = tetra();
        oob.faces[0] = [0, 1, 9];
        assert_eq!(
            try_geometry_to_gltf(&oob, "o"),
            Err(GltfError::IndexOutOfRange)
        );
        // The infallible writer still emits parseable JSON for a NaN.
        assert!(tpt_yard_core::json::Value::parse(&geometry_to_gltf(&nan, "n")).is_ok());
    }

    /// 8B20: STEP strings escape backslashes and non-ASCII text; the
    /// IFC face set is closed only for a closed mesh; indices cannot
    /// overflow.
    #[test]
    fn ifc_strings_closedness_and_index_overflow() {
        assert_eq!(step_string("a'b"), "'a''b'");
        assert_eq!(step_string("C:\\x"), "'C:\\\\x'");
        assert_eq!(step_string("Zürich"), "'Z\\X2\\00FC\\X0\\rich'");
        assert_eq!(step_string("🚢"), "'\\X4\\0001F6A2\\X0\\'");
        let closed = geometry_to_ifc(&tetra(), "t");
        assert!(closed.contains("IFCTRIANGULATEDFACESET(#") && closed.contains(",$,.T.,"));
        let mut open = tetra();
        open.faces.pop();
        assert!(geometry_to_ifc(&open, "t").contains(",$,.F.,"));
        let mut huge = tetra();
        huge.faces[0] = [0, 1, u32::MAX];
        let doc = geometry_to_ifc(&huge, "t");
        assert!(doc.contains("4294967296"), "u32::MAX + 1 must not wrap");
    }
}
