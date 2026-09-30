//! Geometry and mass-property primitives shared by all `tpt-yard-*` crates.
//!
//! Deliberately minimal: [`Vector3`], [`Dimensions`], [`Geometry3D`] (a
//! triangle mesh), and [`MassProperties`]. Crates with heavier needs (FEM,
//! kinematics) build their own abstractions on top of these.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A 3D vector in metres (unless documented otherwise).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector3 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
}

impl Vector3 {
    /// Creates a vector.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    /// Unit vector along X.
    pub const EX: Self = Self::new(1.0, 0.0, 0.0);
    /// Unit vector along Y.
    pub const EY: Self = Self::new(0.0, 1.0, 0.0);
    /// Unit vector along Z.
    pub const EZ: Self = Self::new(0.0, 0.0, 1.0);

    /// Euclidean length.
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Dot product.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product (`self × other`).
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Unit vector in the same direction. The zero vector normalizes to zero.
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            Self::ZERO
        }
    }

    /// Euclidean distance between two points.
    pub fn distance(self, other: Self) -> f64 {
        (self - other).length()
    }

    /// Linear interpolation; `t = 0` gives `self`, `t = 1` gives `other`.
    pub fn lerp(self, other: Self, t: f64) -> Self {
        self + (other - self) * t
    }

    /// Component-wise product (Hadamard).
    pub fn scale_components(self, other: Self) -> Self {
        Self::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }

    /// `(x, y, z)` as a slice-compatible array.
    pub const fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl Add for Vector3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vector3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Neg for Vector3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<f64> for Vector3 {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Mul<Vector3> for f64 {
    type Output = Vector3;
    fn mul(self, v: Vector3) -> Vector3 {
        v * self
    }
}

impl Div<f64> for Vector3 {
    type Output = Self;
    fn div(self, s: f64) -> Self {
        Self::new(self.x / s, self.y / s, self.z / s)
    }
}

impl fmt::Display for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.4}, {:.4}, {:.4})", self.x, self.y, self.z)
    }
}

/// Length × breadth × depth in metres (ship nomenclature).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dimensions {
    /// Length overall, m.
    pub length: f64,
    /// Breadth (moulded), m.
    pub breadth: f64,
    /// Depth, m.
    pub depth: f64,
}

impl Dimensions {
    /// Creates a dimension triple.
    pub const fn new(length: f64, breadth: f64, depth: f64) -> Self {
        Self {
            length,
            breadth,
            depth,
        }
    }
}

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    /// Minimum corner.
    pub min: Vector3,
    /// Maximum corner.
    pub max: Vector3,
}

impl BoundingBox {
    /// The degenerate box around a single point.
    pub fn point(p: Vector3) -> Self {
        Self { min: p, max: p }
    }

    /// Smallest box containing both boxes.
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: Vector3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            max: Vector3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        }
    }

    /// Centre of the box.
    pub fn centre(&self) -> Vector3 {
        (self.min + self.max) * 0.5
    }

    /// Extents (full size) of the box.
    pub fn extents(&self) -> Vector3 {
        self.max - self.min
    }

    /// True if the point is inside or on the box.
    pub fn contains(&self, p: Vector3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }
}

/// A triangle-soup mesh used for block/outfit geometry and rendering.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Geometry3D {
    /// Vertices, m.
    pub vertices: Vec<Vector3>,
    /// Triangles as indices into `vertices`.
    pub faces: Vec<[u32; 3]>,
}

impl Geometry3D {
    /// Empty geometry.
    pub fn new() -> Self {
        Self::default()
    }

    /// A box of the given full extents, centred on the origin.
    pub fn from_box(dx: f64, dy: f64, dz: f64) -> Self {
        let (hx, hy, hz) = (dx / 2.0, dy / 2.0, dz / 2.0);
        let vertices = vec![
            Vector3::new(-hx, -hy, -hz),
            Vector3::new(hx, -hy, -hz),
            Vector3::new(hx, hy, -hz),
            Vector3::new(-hx, hy, -hz),
            Vector3::new(-hx, -hy, hz),
            Vector3::new(hx, -hy, hz),
            Vector3::new(hx, hy, hz),
            Vector3::new(-hx, hy, hz),
        ];
        // +x, -x, +y, -y, +z, -z faces (outward winding).
        let faces = vec![
            [1, 2, 6],
            [1, 6, 5], // +x
            [0, 4, 7],
            [0, 7, 3], // -x
            [2, 3, 7],
            [2, 7, 6], // +y
            [0, 1, 5],
            [0, 5, 4], // -y
            [4, 5, 6],
            [4, 6, 7], // +z
            [0, 3, 2],
            [0, 2, 1], // -z
        ];
        Self { vertices, faces }
    }

    /// A cylinder along the Z axis, centred on the origin. Fewer than 3
    /// segments cannot form a ring; the count is clamped up to 3 (an
    /// assertion would panic on caller-supplied values).
    pub fn from_cylinder(radius: f64, length: f64, segments: u32) -> Self {
        let segments = segments.max(3);
        let half = length / 2.0;
        let mut vertices = Vec::with_capacity(segments as usize * 2 + 2);
        for z in [-half, half] {
            for i in 0..segments {
                let a = 2.0 * std::f64::consts::PI * i as f64 / segments as f64;
                vertices.push(Vector3::new(radius * a.cos(), radius * a.sin(), z));
            }
        }
        let bottom = vertices.len() as u32;
        vertices.push(Vector3::new(0.0, 0.0, -half)); // bottom centre
        vertices.push(Vector3::new(0.0, 0.0, half)); // top centre
        let mut faces = Vec::with_capacity(segments as usize * 4);
        for i in 0..segments {
            let j = (i + 1) % segments;
            // Side quads (two triangles each).
            faces.push([i, j, segments + j]);
            faces.push([i, segments + j, segments + i]);
            // Caps.
            faces.push([bottom, j, i]);
            faces.push([bottom + 1, segments + i, segments + j]);
        }
        Self { vertices, faces }
    }

    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Number of triangular faces.
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Translates every vertex by `delta`.
    pub fn translate(&mut self, delta: Vector3) {
        for v in &mut self.vertices {
            *v = *v + delta;
        }
    }

    /// Returns a translated copy.
    pub fn translated(&self, delta: Vector3) -> Self {
        let mut g = self.clone();
        g.translate(delta);
        g
    }

    /// Scales about the origin by `(sx, sy, sz)`.
    pub fn scale(&mut self, sx: f64, sy: f64, sz: f64) {
        for v in &mut self.vertices {
            *v = Vector3::new(v.x * sx, v.y * sy, v.z * sz);
        }
    }

    /// Rotates every vertex about the Z axis by `angle_rad` (right-hand rule).
    pub fn rotate_z(&mut self, angle_rad: f64) {
        let (s, c) = angle_rad.sin_cos();
        for v in &mut self.vertices {
            *v = Vector3::new(c * v.x - s * v.y, s * v.x + c * v.y, v.z);
        }
    }

    /// Axis-aligned bounding box of all vertices (degenerate for empty
    /// geometry).
    pub fn bounding_box(&self) -> BoundingBox {
        // `&vertices[1..]` on an empty mesh would panic (range start out of
        // range); an empty mesh has an empty box.
        let Some(&first) = self.vertices.first() else {
            return BoundingBox::point(Vector3::ZERO);
        };
        let mut bb = BoundingBox::point(first);
        for v in &self.vertices[1..] {
            bb = bb.union(&BoundingBox::point(*v));
        }
        bb
    }

    /// Area-weighted centroid of the mesh surface.
    pub fn centroid(&self) -> Vector3 {
        let (mut area_sum, mut weighted) = (0.0, Vector3::ZERO);
        for f in &self.faces {
            let (a, b, c) = match (
                self.vertices.get(f[0] as usize),
                self.vertices.get(f[1] as usize),
                self.vertices.get(f[2] as usize),
            ) {
                (Some(a), Some(b), Some(c)) => (*a, *b, *c),
                _ => continue,
            };
            let area = (b - a).cross(c - a).length() * 0.5;
            weighted = weighted + (a + b + c) * (area / 3.0);
            area_sum += area;
        }
        if area_sum > 0.0 {
            weighted / area_sum
        } else {
            Vector3::ZERO
        }
    }

    /// Appends another mesh (offset by `delta`) into this one.
    pub fn merge(&mut self, other: &Geometry3D, delta: Vector3) {
        // Face indices are u32; a combined vertex count beyond that cannot
        // be represented (and cannot fit memory either). Saturate instead
        // of wrapping indices silently.
        let Ok(offset) = u32::try_from(self.vertices.len()) else {
            debug_assert!(false, "vertex count exceeds u32::MAX in merge");
            return;
        };
        for v in &other.vertices {
            self.vertices.push(*v + delta);
        }
        for f in &other.faces {
            self.faces
                .push([f[0] + offset, f[1] + offset, f[2] + offset]);
        }
    }
}

/// Mass properties of a body: total mass and centre of gravity.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MassProperties {
    /// Total mass, kg.
    pub mass_kg: f64,
    /// Centre of gravity, m.
    pub cog: Vector3,
}

impl MassProperties {
    /// Combines two bodies by weighted averaging of their CoGs.
    pub fn combine(a: MassProperties, b: MassProperties) -> MassProperties {
        let total = a.mass_kg + b.mass_kg;
        if total <= 0.0 {
            return MassProperties::default();
        }
        MassProperties {
            mass_kg: total,
            cog: (a.cog * a.mass_kg + b.cog * b.mass_kg) / total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn vector_arithmetic() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vector3::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vector3::new(3.0, 3.0, 3.0));
        assert_eq!(a * 2.0, Vector3::new(2.0, 4.0, 6.0));
        assert_eq!(-a, Vector3::new(-1.0, -2.0, -3.0));
        assert!(close(2.0 * Vector3::EX.dot(Vector3::EY), 0.0));
        assert!(close(a.dot(b), 32.0));
        assert_eq!(a.cross(b), Vector3::new(-3.0, 6.0, -3.0));
        assert!(close(a.length(), (14.0f64).sqrt()));
        assert!(close(a.normalize().length(), 1.0));
        assert_eq!(Vector3::ZERO.normalize(), Vector3::ZERO);
        assert!(close(a.distance(b), (27.0f64).sqrt()));
        assert_eq!(a.lerp(b, 0.5), Vector3::new(2.5, 3.5, 4.5));
    }

    #[test]
    fn box_geometry_bounds_and_centroid() {
        let g = Geometry3D::from_box(2.0, 4.0, 6.0);
        assert_eq!(g.vertex_count(), 8);
        assert_eq!(g.face_count(), 12);
        let bb = g.bounding_box();
        assert!(close(bb.extents().x, 2.0));
        assert!(close(bb.extents().y, 4.0));
        assert!(close(bb.extents().z, 6.0));
        let c = g.centroid();
        assert!(close(c.x, 0.0) && close(c.y, 0.0) && close(c.z, 0.0));
        assert!(bb.contains(Vector3::ZERO));
        assert!(!bb.contains(Vector3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn cylinder_geometry() {
        let g = Geometry3D::from_cylinder(1.0, 10.0, 16);
        assert_eq!(g.vertex_count(), 34);
        let bb = g.bounding_box();
        assert!((bb.extents().x - 2.0).abs() < 0.3);
        assert!(close(bb.extents().z, 10.0));
    }

    #[test]
    fn translate_merge_and_mass_combine() {
        let mut g = Geometry3D::from_box(1.0, 1.0, 1.0);
        g.translate(Vector3::new(10.0, 0.0, 0.0));
        assert!(close(g.bounding_box().centre().x, 10.0));

        let mut h = Geometry3D::from_box(1.0, 1.0, 1.0);
        h.merge(&g, Vector3::new(5.0, 0.0, 0.0));
        assert_eq!(h.vertex_count(), 16);

        let a = MassProperties {
            mass_kg: 100.0,
            cog: Vector3::new(0.0, 0.0, 0.0),
        };
        let b = MassProperties {
            mass_kg: 300.0,
            cog: Vector3::new(4.0, 0.0, 0.0),
        };
        let m = MassProperties::combine(a, b);
        assert!(close(m.mass_kg, 400.0));
        assert!(close(m.cog.x, 3.0));
    }

    #[test]
    fn rotate_z_preserves_length() {
        let mut g = Geometry3D::from_box(2.0, 2.0, 2.0);
        g.rotate_z(std::f64::consts::FRAC_PI_2);
        let bb = g.bounding_box();
        assert!(close(bb.extents().x, 2.0));
        assert!(close(bb.extents().y, 2.0));
    }
}
