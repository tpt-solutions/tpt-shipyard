//! Export adapters for the engine's geometry (review 7H: glTF export).
//!
//! glTF 2.0 is a JSON scene description with base64- or buffer-referenced
//! binary data; the simplest interoperable form is a *glTF with embedded
//! base64 buffer*, which every three.js/Babylon/Blender viewer loads.

use tpt_yard_core::Geometry3D;

/// Writes `geometry` as a minimal glTF 2.0 JSON string (embedded base64
/// buffer, TRIANGLES mode). Returns the `.gltf` document text — write it
/// to a `<name>.gltf` file.
pub fn geometry_to_gltf(geometry: &Geometry3D, name: &str) -> String {
    // Binary chunk: positions (f32 xyz per vertex), then indices (u32).
    let mut bin: Vec<u8> = Vec::new();
    for v in &geometry.vertices {
        bin.extend_from_slice(&(v.x as f32).to_le_bytes());
        bin.extend_from_slice(&(v.y as f32).to_le_bytes());
        bin.extend_from_slice(&(v.z as f32).to_le_bytes());
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let indices_offset = bin.len();
    for f in &geometry.faces {
        bin.extend_from_slice(&f[0].to_le_bytes());
        bin.extend_from_slice(&f[1].to_le_bytes());
        bin.extend_from_slice(&f[2].to_le_bytes());
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let bin_len = bin.len();

    // Base64-encode.
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut b64 = String::with_capacity((bin_len + 2) / 3 * 4);
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

    // Min/max position accessors (glTF requires them for POSITION).
    let mut min = [f64::MAX; 3];
    let mut max = [f64::MIN; 3];
    for v in &geometry.vertices {
        for (i, c) in [v.x, v.y, v.z].iter().enumerate() {
            min[i] = min[i].min(*c);
            max[i] = max[i].max(*c);
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

    format!(
        r#"{{"asset":{{"version":"2.0","generator":"tpt-shipyard"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0,"name":"{name}"}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}},"indices":1,"mode":4}}]}}],"buffers":[{{"uri":"data:application/octet-stream;base64,{b64}","byteLength":{bin_len}}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":{positions_byte_length},"target":34962}},{{"buffer":0,"byteOffset":{indices_offset},"byteLength":{indices_byte_length},"target":34963}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":{v_count},"type":"VEC3","min":[{min_x},{min_y},{min_z}],"max":[{max_x},{max_y},{max_z}]}},{{"bufferView":1,"componentType":5125,"count":{i_count},"type":"SCALAR","min":[0],"max":[{max_index}]}}]}}"#,
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

/// Decodes a base64 string back to bytes (test/verification helper — the
/// standard decoder in any glTF consumer does this internally).
#[cfg(test)]
pub(crate) fn decode_base64(s: &str) -> Vec<u8> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
