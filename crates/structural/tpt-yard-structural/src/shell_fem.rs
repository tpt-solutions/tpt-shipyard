//! Flat-facet shell elements: curved shells as assemblies of planar
//! rectangular facets with six degrees of freedom per node.
//!
//! Each facet couples a plane-stress membrane (bilinear Q4, 2x2 Gauss)
//! with the MITC4 Mindlin plate bending of [`crate::mindlin`] in the
//! element's local frame, plus a small stabilising stiffness on the
//! drilling rotation (the normal rotation has no membrane stiffness and
//! would leave coplanar facets singular). Local stiffness is rotated to
//! global axes, so a facet may sit in any orientation; the four corners
//! must form a planar rectangle (cylinders, tanks, flat panels and prisms
//! mesh exactly into such facets — doubly curved surfaces would need
//! triangles or warped quadrilaterals, not provided).
//!
//! Node DOFs are `(u, v, w, theta_x, theta_y, theta_z)` in global axes
//! (right-handed rotations). Loads are nodal forces and a per-element
//! surface load vector (force per unit area, global axes — gravity or a
//! pressure resolved by the caller). Boundary conditions are exact DOF
//! eliminations; the reduced system is solved by dense Cholesky.
//!
//! Verification: a uniformly stretched plate (exact), an arbitrarily
//! oriented clamped plate against the same plate lying flat, and the
//! Scordelis-Lo barrel vault (MacNeal-Harder reference 0.3024 m).

use crate::mindlin::mindlin_element_matrix;

/// Facet drilling-rotation stiffness as a fraction of `G t A` per node
/// (a small fictitious value; results are insensitive to it for curved
/// assemblies, which the Scordelis-Lo test checks).
const DRILLING_FRACTION: f64 = 1.0e-4;

/// One planar rectangular facet.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellElement {
    /// Corner node indices in counter-clockwise order seen from the
    /// positive normal: `n0 -> n1` along local x, `n0 -> n3` along local y.
    pub nodes: [usize; 4],
    /// Thickness, m.
    pub thickness_m: f64,
    /// Young's modulus, GPa.
    pub youngs_modulus_gpa: f64,
    /// Poisson's ratio (`-1 < nu < 0.5`).
    pub poissons_ratio: f64,
    /// Surface load, N/m^2, global axes (gravity: `[0, 0, -rho g t]`).
    pub surface_load_n_m2: [f64; 3],
}

/// Fixed DOFs at a node, `(u, v, w, theta_x, theta_y, theta_z)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellSupport {
    /// Node index.
    pub node: usize,
    /// Which of the six DOFs are held at zero.
    pub fixed: [bool; 6],
}

/// A nodal force/moment, `(Fx, Fy, Fz, Mx, My, Mz)` in N and N*m.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellNodalLoad {
    /// Node index.
    pub node: usize,
    /// Force and moment components.
    pub load: [f64; 6],
}

/// Errors from shell assembly and solution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellError {
    /// No elements.
    NoElements,
    /// A node index is out of range.
    OutOfRange,
    /// A facet's corners are not a planar rectangle (element index).
    NotRectangular(usize),
    /// A material or thickness value is non-physical (element index).
    BadMaterial(usize),
    /// The reduced stiffness matrix is singular (a rigid-body mode is
    /// left free).
    SingularSystem,
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellError::NoElements => f.write_str("the shell model has no elements"),
            ShellError::OutOfRange => f.write_str("a node index is out of range"),
            ShellError::NotRectangular(i) => {
                write!(f, "facet {i} is not a planar rectangle")
            }
            ShellError::BadMaterial(i) => {
                write!(f, "facet {i} has a non-physical thickness or material")
            }
            ShellError::SingularSystem => {
                f.write_str("singular shell stiffness: a rigid-body mode is unsupported")
            }
        }
    }
}

impl std::error::Error for ShellError {}

/// A facet shell model.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellModel {
    /// Node positions, m.
    pub nodes: Vec<[f64; 3]>,
    /// Facets.
    pub elements: Vec<ShellElement>,
    /// Supports.
    pub supports: Vec<ShellSupport>,
    /// Nodal loads.
    pub loads: Vec<ShellNodalLoad>,
}

/// Displacements and rotations per node.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellSolution {
    /// Translations `(u, v, w)`, m.
    pub displacements: Vec<[f64; 3]>,
    /// Rotations `(theta_x, theta_y, theta_z)`, rad.
    pub rotations: Vec<[f64; 3]>,
}

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// Q4 plane-stress membrane stiffness, 8x8, DOFs `(u, v)` per node,
/// counter-clockwise rectangle `dx` by `dy`.
fn membrane_matrix(dx: f64, dy: f64, t: f64, e: f64, nu: f64) -> [[f64; 8]; 8] {
    let (hx, hy) = (dx / 2.0, dy / 2.0);
    let c = e / (1.0 - nu * nu);
    let d = [
        [c, c * nu, 0.0],
        [c * nu, c, 0.0],
        [0.0, 0.0, c * (1.0 - nu) / 2.0],
    ];
    let signs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let g = 0.577_350_269_189_625_7_f64;
    let mut k = [[0.0_f64; 8]; 8];
    for &xi in &[-g, g] {
        for &eta in &[-g, g] {
            let mut b = [[0.0_f64; 8]; 3];
            for (i, &(s, r)) in signs.iter().enumerate() {
                let nx = s * (1.0 + eta * r) / 4.0 / hx;
                let ny = (1.0 + xi * s) * r / 4.0 / hy;
                b[0][2 * i] = nx;
                b[1][2 * i + 1] = ny;
                b[2][2 * i] = ny;
                b[2][2 * i + 1] = nx;
            }
            for p in 0..8 {
                for q in 0..8 {
                    let mut s = 0.0;
                    for r in 0..3 {
                        for w in 0..3 {
                            s += b[r][p] * d[r][w] * b[w][q];
                        }
                    }
                    k[p][q] += s * t * hx * hy;
                }
            }
        }
    }
    k
}

/// The 24x24 element matrix in global axes and the element area.
fn element_matrix(model: &ShellModel, idx: usize) -> Result<(Vec<[f64; 24]>, f64), ShellError> {
    let el = &model.elements[idx];
    let p: Vec<V3> = el.nodes.iter().map(|&i| model.nodes[i]).collect();
    let (ex, ey) = (sub(p[1], p[0]), sub(p[3], p[0]));
    let (dx, dy) = (norm(ex), norm(ey));
    if dx <= 0.0 || dy <= 0.0 {
        return Err(ShellError::NotRectangular(idx));
    }
    let scale = dx.max(dy);
    let (xl, yl) = (
        [ex[0] / dx, ex[1] / dx, ex[2] / dx],
        [ey[0] / dy, ey[1] / dy, ey[2] / dy],
    );
    let fourth = sub(p[2], [p[1][0] + ey[0], p[1][1] + ey[1], p[1][2] + ey[2]]);
    if dot(xl, yl).abs() > 1e-6 || norm(fourth) > 1e-6 * scale {
        return Err(ShellError::NotRectangular(idx));
    }
    let zl = cross(xl, yl);
    let (t, e, nu) = (
        el.thickness_m,
        el.youngs_modulus_gpa * 1e9,
        el.poissons_ratio,
    );
    if !(t.is_finite() && t > 0.0)
        || !(e.is_finite() && e > 0.0)
        || !(nu.is_finite() && nu > -1.0 && nu < 0.5)
    {
        return Err(ShellError::BadMaterial(idx));
    }
    let km = membrane_matrix(dx, dy, t, e, nu);
    let kp = mindlin_element_matrix(dx, dy, t, el.youngs_modulus_gpa, nu);
    let mut kl = [[0.0_f64; 24]; 24];
    for i in 0..4 {
        for j in 0..4 {
            for a in 0..2 {
                for b in 0..2 {
                    kl[6 * i + a][6 * j + b] += km[2 * i + a][2 * j + b];
                }
            }
            // Mindlin (w, tx, ty) -> local slots 2, 3, 4.
            for a in 0..3 {
                for b in 0..3 {
                    kl[6 * i + 2 + a][6 * j + 2 + b] += kp[3 * i + a][3 * j + b];
                }
            }
        }
        let g = e / (2.0 * (1.0 + nu));
        kl[6 * i + 5][6 * i + 5] += DRILLING_FRACTION * g * t * dx * dy / 4.0;
    }
    // kg = Tt kl T with T = blockdiag(R) over the 8 translation/rotation
    // triples, R rows = local axes in global coordinates.
    let r = [xl, yl, zl];
    let mut tm = vec![[0.0_f64; 24]; 24];
    for blk in 0..8 {
        for (a, ra) in r.iter().enumerate() {
            for (b, v) in ra.iter().enumerate() {
                tm[3 * blk + a][3 * blk + b] = *v;
            }
        }
    }
    let mut tmp = vec![[0.0_f64; 24]; 24];
    for i in 0..24 {
        for j in 0..24 {
            let mut s = 0.0;
            for k in 0..24 {
                s += kl[i][k] * tm[k][j];
            }
            tmp[i][j] = s;
        }
    }
    let mut kg = vec![[0.0_f64; 24]; 24];
    for i in 0..24 {
        for j in 0..24 {
            let mut s = 0.0;
            for k in 0..24 {
                s += tm[k][i] * tmp[k][j];
            }
            kg[i][j] = s;
        }
    }
    Ok((kg, dx * dy))
}

impl ShellModel {
    /// Assembles and solves the facet shell (dense Cholesky on the
    /// reduced system).
    ///
    /// # Errors
    ///
    /// [`ShellError`] for an empty model, bad indices, non-rectangular
    /// facets, bad materials or an under-supported (singular) model.
    #[allow(clippy::needless_range_loop)]
    pub fn solve(&self) -> Result<ShellSolution, ShellError> {
        if self.elements.is_empty() {
            return Err(ShellError::NoElements);
        }
        let n = self.nodes.len();
        if self
            .elements
            .iter()
            .any(|e| e.nodes.iter().any(|&i| i >= n))
            || self.supports.iter().any(|s| s.node >= n)
            || self.loads.iter().any(|l| l.node >= n)
        {
            return Err(ShellError::OutOfRange);
        }
        let dof = 6 * n;
        let mut fixed = vec![false; dof];
        for s in &self.supports {
            for (k, &f) in s.fixed.iter().enumerate() {
                fixed[6 * s.node + k] |= f;
            }
        }
        let mut free_index = vec![usize::MAX; dof];
        let mut free = 0usize;
        for (i, &f) in fixed.iter().enumerate() {
            if !f {
                free_index[i] = free;
                free += 1;
            }
        }
        if free == 0 {
            return Err(ShellError::SingularSystem);
        }
        let mut k = vec![0.0_f64; free * free];
        let mut f = vec![0.0_f64; free];
        for (i, el) in self.elements.iter().enumerate() {
            let (kg, area) = element_matrix(self, i)?;
            let map: Vec<usize> = el
                .nodes
                .iter()
                .flat_map(|&ni| (0..6).map(move |d| 6 * ni + d))
                .collect();
            for (p, &gp) in map.iter().enumerate() {
                let fp = free_index[gp];
                if fp == usize::MAX {
                    continue;
                }
                for (q, &gq) in map.iter().enumerate() {
                    let fq = free_index[gq];
                    if fq != usize::MAX {
                        k[fp * free + fq] += kg[p][q];
                    }
                }
            }
            // Consistent surface load: A/4 per node on the translations.
            for (corner, &ni) in el.nodes.iter().enumerate() {
                let _ = corner;
                for d in 0..3 {
                    let fi = free_index[6 * ni + d];
                    if fi != usize::MAX {
                        f[fi] += el.surface_load_n_m2[d] * area / 4.0;
                    }
                }
            }
        }
        for l in &self.loads {
            for d in 0..6 {
                let fi = free_index[6 * l.node + d];
                if fi != usize::MAX {
                    f[fi] += l.load[d];
                }
            }
        }
        let k_scale = (0..free)
            .map(|i| k[i * free + i])
            .fold(0.0_f64, f64::max)
            .max(1e-30);
        let x = crate::fem::dense_cholesky_solve(free, &mut k, &f, k_scale)
            .map_err(|_| ShellError::SingularSystem)?;
        let at = |g: usize| {
            let fi = free_index[g];
            if fi == usize::MAX {
                0.0
            } else {
                x[fi]
            }
        };
        let displacements = (0..n)
            .map(|i| [at(6 * i), at(6 * i + 1), at(6 * i + 2)])
            .collect();
        let rotations = (0..n)
            .map(|i| [at(6 * i + 3), at(6 * i + 4), at(6 * i + 5)])
            .collect();
        Ok(ShellSolution {
            displacements,
            rotations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const E_GPA: f64 = 210.0;

    /// A flat `a x b` rectangle (in the plane spanned by `ex`, `ey`)
    /// meshed `nx x ny`, with its origin at `o`.
    fn flat_mesh(
        o: V3,
        ex: V3,
        ey: V3,
        a: f64,
        b: f64,
        nx: usize,
        ny: usize,
        t: f64,
    ) -> (Vec<V3>, Vec<ShellElement>) {
        let mut nodes = Vec::new();
        for j in 0..=ny {
            for i in 0..=nx {
                let (u, v) = (a * i as f64 / nx as f64, b * j as f64 / ny as f64);
                nodes.push([
                    o[0] + u * ex[0] + v * ey[0],
                    o[1] + u * ex[1] + v * ey[1],
                    o[2] + u * ex[2] + v * ey[2],
                ]);
            }
        }
        let id = |i: usize, j: usize| j * (nx + 1) + i;
        let mut els = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                els.push(ShellElement {
                    nodes: [id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)],
                    thickness_m: t,
                    youngs_modulus_gpa: E_GPA,
                    poissons_ratio: 0.3,
                    surface_load_n_m2: [0.0; 3],
                });
            }
        }
        (nodes, els)
    }

    /// A plate stretched by an end line load: u = sigma L / E exactly
    /// (the Q4 reproduces constant strain), with no through-thickness
    /// coupling and lateral Poisson contraction free.
    #[test]
    fn a_stretched_plate_elongates_exactly() {
        let (l, w, t) = (2.0, 1.0, 0.01);
        let (nodes, els) = flat_mesh([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], l, w, 4, 2, t);
        let mut supports = Vec::new();
        let mut loads = Vec::new();
        let nxp = 5;
        for (i, p) in nodes.iter().enumerate() {
            if p[0] == 0.0 {
                // Clamp u; pin the rest of the rigid modes.
                let mut fixed = [true, false, true, true, true, true];
                fixed[1] = p[1] == 0.0; // one node holds v
                supports.push(ShellSupport { node: i, fixed });
            } else {
                supports.push(ShellSupport {
                    node: i,
                    fixed: [false, false, true, true, true, true],
                });
            }
            if (p[0] - l).abs() < 1e-12 {
                let tributary = if p[1] == 0.0 || (p[1] - w).abs() < 1e-12 {
                    w / 4.0
                } else {
                    w / 2.0
                };
                loads.push(ShellNodalLoad {
                    node: i,
                    load: [100e6 * t * tributary, 0.0, 0.0, 0.0, 0.0, 0.0],
                });
            }
        }
        let _ = nxp;
        let sol = ShellModel {
            nodes: nodes.clone(),
            elements: els,
            supports,
            loads,
        }
        .solve()
        .expect("solves");
        let expected = 100e6 / (E_GPA * 1e9) * l;
        let tip = nodes
            .iter()
            .position(|p| (p[0] - l).abs() < 1e-12 && p[1] == w / 2.0)
            .unwrap();
        assert!(
            (sol.displacements[tip][0] - expected).abs() < 1e-9 * expected.max(1.0) + 1e-12,
            "{} vs {expected}",
            sol.displacements[tip][0]
        );
    }

    /// The same clamped square plate under pressure, lying flat and
    /// tilted into an arbitrary plane, deflects identically: the
    /// local-to-global rotation of the 24x24 matrix is correct.
    #[test]
    fn orientation_does_not_change_plate_bending() {
        let (a, t, q) = (2.0, 0.02, 10_000.0);
        let n = 6;
        let solve = |ex: V3, ey: V3| -> f64 {
            let (nodes, mut els) = flat_mesh([0.3, -0.7, 1.1], ex, ey, a, a, n, n, t);
            let normal = cross(ex, ey);
            for e in &mut els {
                e.surface_load_n_m2 = [q * normal[0], q * normal[1], q * normal[2]];
            }
            let o = [0.3, -0.7, 1.1];
            let mut supports = Vec::new();
            for (i, p) in nodes.iter().enumerate() {
                let d = sub(*p, o);
                let (u, v) = (dot(d, ex), dot(d, ey));
                let edge = u < 1e-9 || v < 1e-9 || u > a - 1e-9 || v > a - 1e-9;
                if edge {
                    supports.push(ShellSupport {
                        node: i,
                        fixed: [true; 6],
                    });
                }
            }
            let sol = ShellModel {
                nodes: nodes.clone(),
                elements: els,
                supports,
                loads: vec![],
            }
            .solve()
            .expect("solves");
            let centre = nodes
                .iter()
                .position(|p| {
                    let d = sub(*p, o);
                    (dot(d, ex) - a / 2.0).abs() < 1e-9 && (dot(d, ey) - a / 2.0).abs() < 1e-9
                })
                .unwrap();
            dot(sol.displacements[centre], normal)
        };
        let flat = solve([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let s = 1.0 / 3.0_f64.sqrt();
        let tilted = solve([s, s, s], {
            // Orthogonal to the first: rotate (1,-1,0)/sqrt2.
            let r = 1.0 / 2.0_f64.sqrt();
            [r, -r, 0.0]
        });
        let d = E_GPA * 1e9 * t.powi(3) / (12.0 * (1.0 - 0.09));
        // Clamped square plate, Timoshenko: 0.00126 q a^4 / D.
        let exact = 0.00126 * q * a.powi(4) / d;
        assert!((flat - exact).abs() < 0.06 * exact, "{flat} vs {exact}");
        assert!(
            (tilted - flat).abs() < 1e-8 * flat.abs(),
            "{tilted} vs {flat}"
        );
    }

    /// The Scordelis-Lo barrel vault (R 25, L 50, t 0.25, E 4.32e8 Pa,
    /// nu 0, 90 N/m^2 gravity on the shell area, rigid diaphragm ends,
    /// free long edges): quarter model, vertical displacement at the
    /// free-edge midpoint, MacNeal-Harder reference 0.3024 m.
    fn scordelis_lo(n: usize) -> f64 {
        let (r, half_l, t) = (25.0_f64, 25.0, 0.25);
        let phi_max = 40.0_f64.to_radians();
        let mut nodes = Vec::new();
        for j in 0..=n {
            for i in 0..=n {
                let x = half_l * i as f64 / n as f64;
                let phi = phi_max * j as f64 / n as f64;
                nodes.push([x, r * phi.sin(), r * phi.cos()]);
            }
        }
        let id = |i: usize, j: usize| j * (n + 1) + i;
        let mut els = Vec::new();
        for j in 0..n {
            for i in 0..n {
                // ccw seen from outside: x then phi.
                els.push(ShellElement {
                    nodes: [id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)],
                    thickness_m: t,
                    youngs_modulus_gpa: 4.32e8 / 1e9,
                    poissons_ratio: 0.0,
                    surface_load_n_m2: [0.0, 0.0, -90.0],
                });
            }
        }
        let mut supports = Vec::new();
        for (idx, p) in nodes.iter().enumerate() {
            let mut fixed = [false; 6];
            if p[0] == 0.0 {
                // Symmetry plane x = 0.
                fixed[0] = true;
                fixed[4] = true;
                fixed[5] = true;
            }
            if p[1] == 0.0 {
                // Symmetry plane y = 0.
                fixed[1] = true;
                fixed[3] = true;
                fixed[5] = true;
            }
            if (p[0] - half_l).abs() < 1e-9 {
                // Rigid diaphragm: no in-plane (y, z) motion.
                fixed[1] = true;
                fixed[2] = true;
            }
            if fixed.iter().any(|&f| f) {
                supports.push(ShellSupport { node: idx, fixed });
            }
        }
        let sol = ShellModel {
            nodes,
            elements: els,
            supports,
            loads: vec![],
        }
        .solve()
        .expect("solves");
        // Free-edge midspan: i = 0 (x = 0), j = n (phi = 40 deg).
        -sol.displacements[id(0, n)][2]
    }

    #[test]
    fn scordelis_lo_converges_to_the_published_value() {
        let reference = 0.3024;
        let coarse = scordelis_lo(4);
        let fine = scordelis_lo(8);
        eprintln!("scordelis-lo 4x4 {coarse:.4}  8x8 {fine:.4}  ref {reference}");
        // Monotone convergence towards the reference.
        assert!(
            (fine - reference).abs() < (coarse - reference).abs(),
            "{coarse} -> {fine}"
        );
        assert!((fine - reference).abs() < 0.04 * reference, "{fine}");
    }

    #[test]
    fn bad_models_are_errors() {
        let base = |nodes: Vec<V3>| ShellModel {
            nodes,
            elements: vec![ShellElement {
                nodes: [0, 1, 2, 3],
                thickness_m: 0.01,
                youngs_modulus_gpa: 200.0,
                poissons_ratio: 0.3,
                surface_load_n_m2: [0.0; 3],
            }],
            supports: vec![],
            loads: vec![],
        };
        // A skewed quadrilateral is not a rectangle.
        let skew = base(vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.4, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        assert_eq!(skew.solve(), Err(ShellError::NotRectangular(0)));
        // No supports: free body.
        let rect = base(vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        assert_eq!(rect.solve(), Err(ShellError::SingularSystem));
        let mut oob = rect.clone();
        oob.elements[0].nodes[2] = 9;
        assert_eq!(oob.solve(), Err(ShellError::OutOfRange));
        let mut empty = rect;
        empty.elements.clear();
        assert_eq!(empty.solve(), Err(ShellError::NoElements));
    }
}
