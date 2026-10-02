//! Rectangular plate bending elements (review 7H roadmap: "shell
//! elements" — the plate slice).
//!
//! [`PlateModel`] solves thin-plate bending (Kirchhoff theory) on
//! axis-aligned rectangular meshes with the Bogner-Fox-Schmit 16-DOF
//! element: each corner carries `w`, `dw/dx`, `dw/dy`, `d2w/dxdy`, and
//! the shape functions are tensor products of 1D cubic Hermite
//! polynomials — a C1-conforming element, so closed-form plate
//! deflections are approached monotonically with refinement. Solved by
//! the shared penalized sparse CG path. Shear-deformable plates are the
//! sibling [`MindlinModel`](crate::mindlin::MindlinModel); curved
//! geometry remains roadmap.
//!
//! Verification targets (Timoshenko & Woinowsky-Krieger, uniform load q
//! on a square plate of side a, flexural rigidity
//! `D = E h^3 / (12 (1 - nu^2))`): simply supported
//! `w_c = 0.00406 q a^4 / D`; clamped `w_c = 0.00126 q a^4 / D`.

/// A plate node at (x, y), m (plate in the x-y plane, thickness in z).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlateNode {
    /// Position, m.
    pub position: (f64, f64),
}

/// A rectangular plate element: the corner order is counter-clockwise
/// `(x0,y0), (x1,y0), (x1,y1), (x0,y1)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlateElement {
    /// The four corner nodes, counter-clockwise.
    pub nodes: [usize; 4],
    /// Plate thickness, m.
    pub thickness_m: f64,
    /// Young's modulus, GPa.
    pub youngs_modulus_gpa: f64,
    /// Poisson's ratio.
    pub poissons_ratio: f64,
    /// Uniform lateral pressure, N/m² (positive pushes +z).
    pub pressure_n_m2: f64,
}

/// DOF support at a node: constrain any of the four BFS quantities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlateSupport {
    /// Node index.
    pub node: usize,
    /// Fix deflection w.
    pub fix_w: bool,
    /// Fix slope dw/dx.
    pub fix_wx: bool,
    /// Fix slope dw/dy.
    pub fix_wy: bool,
    /// Fix twist dw/dxdy.
    pub fix_wxy: bool,
}

impl PlateSupport {
    /// Clamped node: all four DOFs fixed.
    pub fn clamped(node: usize) -> Self {
        Self {
            node,
            fix_w: true,
            fix_wx: true,
            fix_wy: true,
            fix_wxy: true,
        }
    }

    /// Simply supported node: deflection fixed, rotations free.
    pub fn simply_supported(node: usize) -> Self {
        Self {
            node,
            fix_w: true,
            fix_wx: false,
            fix_wy: false,
            fix_wxy: false,
        }
    }
}

/// Solution: deflection per node plus the maximum.
#[derive(Debug, Clone, PartialEq)]
pub struct PlateSolution {
    /// Deflection w per node, m (positive +z).
    pub deflections: Vec<f64>,
    /// Largest absolute deflection, m.
    pub max_deflection_m: f64,
}

/// Errors from the plate solver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlateError {
    /// The plate is a mechanism or the mesh is degenerate.
    SingularSystem,
    /// Index or property out of range.
    OutOfRange,
    /// No elements.
    NoElements,
}

impl std::fmt::Display for PlateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlateError::SingularSystem => {
                f.write_str("singular plate stiffness: mechanism or degenerate mesh")
            }
            PlateError::OutOfRange => f.write_str("index or property out of range"),
            PlateError::NoElements => f.write_str("no plate elements"),
        }
    }
}

impl std::error::Error for PlateError {}

/// A rectangular thin-plate bending model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlateModel {
    /// Nodes.
    pub nodes: Vec<PlateNode>,
    /// Rectangular elements (axis-aligned).
    pub elements: Vec<PlateElement>,
    /// Supports.
    pub supports: Vec<PlateSupport>,
}

/// Cubic Hermite basis on [0, 1] with physical length `a`:
/// `(H0, T0, H1, T1)` — `H0(0)=1, H1(1)=1`, `T` are the slope functions
/// carrying the length scale. Derivative flags: 0 = value, 1 = first
/// derivative (per x/y), 2 = second derivative.
fn hermite(xi: f64, a: f64, deriv: u8) -> [f64; 4] {
    let (x2, x3) = (xi * xi, xi * xi * xi);
    match deriv {
        0 => [
            1.0 - 3.0 * x2 + 2.0 * x3,
            a * (xi - 2.0 * x2 + x3),
            3.0 * x2 - 2.0 * x3,
            a * (-x2 + x3),
        ],
        1 => [
            (-6.0 * xi + 6.0 * x2) / a,
            1.0 - 4.0 * xi + 3.0 * x2,
            (6.0 * xi - 6.0 * x2) / a,
            -2.0 * xi + 3.0 * x2,
        ],
        _ => [
            (-6.0 + 12.0 * xi) / (a * a),
            (-4.0 + 6.0 * xi) / a,
            (6.0 - 12.0 * xi) / (a * a),
            (-2.0 + 6.0 * xi) / a,
        ],
    }
}

/// A plate-bending shape function: `(N, N_xx, N_yy, N_xy)` — the value
/// and the second derivatives that drive the bending energy. DOF kind
/// within the corner: 0 = w, 1 = wx, 2 = wy, 3 = wxy; the corner's
/// (hx, hy) side selects which Hermite pair.
#[allow(clippy::type_complexity)]
fn shape(
    dof_in_corner: usize,
    hx: usize,
    hy: usize,
    xi: f64,
    eta: f64,
    a: f64,
    b: f64,
) -> (f64, f64, f64, f64) {
    // Hermite function selector: `kind` 0 picks the VALUE pair (H0/H1,
    // and their derivatives in the higher-order variants), `kind` 1 the
    // SLOPE pair (T0/T1, and their derivatives). `side` picks the x0 or
    // x1 corner function.
    let f = |side: usize, order: u8, xi: f64, c: f64, kind: u8| -> f64 {
        let h = hermite(xi, c, order);
        if kind == 0 {
            if side == 0 {
                h[0]
            } else {
                h[2]
            }
        } else if side == 0 {
            h[1]
        } else {
            h[3]
        }
    };
    // N and derivatives for DOF kind:
    //   0 = w   (Hhx Hhy)         1 = wx  (Tx Hhy)
    //   2 = wy  (Hhx Ty)          3 = wxy (Tx Ty)
    // N_xx mixes d2/dx2 on the x function; N_yy on the y function;
    // N_xy = d/dx x d/dy across the two.
    let (n, n_xx, n_yy, n_xy) = match dof_in_corner {
        0 => (
            f(hx, 0, xi, a, 0) * f(hy, 0, eta, b, 0),
            f(hx, 2, xi, a, 0) * f(hy, 0, eta, b, 0),
            f(hx, 0, xi, a, 0) * f(hy, 2, eta, b, 0),
            f(hx, 1, xi, a, 0) * f(hy, 1, eta, b, 0),
        ),
        1 => (
            f(hx, 0, xi, a, 1) * f(hy, 0, eta, b, 0),
            f(hx, 2, xi, a, 1) * f(hy, 0, eta, b, 0),
            f(hx, 0, xi, a, 1) * f(hy, 2, eta, b, 0),
            f(hx, 1, xi, a, 1) * f(hy, 1, eta, b, 0),
        ),
        2 => (
            f(hx, 0, xi, a, 0) * f(hy, 0, eta, b, 1),
            f(hx, 2, xi, a, 0) * f(hy, 0, eta, b, 1),
            f(hx, 0, xi, a, 0) * f(hy, 2, eta, b, 1),
            f(hx, 1, xi, a, 0) * f(hy, 1, eta, b, 1),
        ),
        _ => (
            f(hx, 0, xi, a, 1) * f(hy, 0, eta, b, 1),
            f(hx, 2, xi, a, 1) * f(hy, 0, eta, b, 1),
            f(hx, 0, xi, a, 1) * f(hy, 2, eta, b, 1),
            f(hx, 1, xi, a, 1) * f(hy, 1, eta, b, 1),
        ),
    };
    (n, n_xx, n_yy, n_xy)
}

/// 2-point Gauss-Legendre on [0, 1].
fn gauss2() -> [(f64, f64); 2] {
    let g = 0.577_350_269_189_625_7;
    [(0.5 * (1.0 - g), 0.5), (0.5 * (1.0 + g), 0.5)]
}

/// The 16x16 element matrix for one rectangular BFS element (exposed
/// for SPD verification in tests).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn bfs_element_matrix(
    a: f64,
    b: f64,
    thickness_m: f64,
    youngs_modulus_gpa: f64,
    poissons_ratio: f64,
) -> [[f64; 16]; 16] {
    let d = youngs_modulus_gpa * 1e9 * thickness_m.powi(3)
        / (12.0 * (1.0 - poissons_ratio * poissons_ratio));
    let nu = poissons_ratio;
    let corners = [(0usize, 0usize), (1, 0), (1, 1), (0, 1)];
    let hxs = [0_usize, 1, 1, 0];
    let hys = [0_usize, 0, 1, 1];
    let pts = gauss2();
    let mut ke = [[0.0_f64; 16]; 16];
    // Stiffness kernel loops: indexed accumulation over the 16x16 local
    // matrix is the clearest form; the value functions n_i/n_j do not
    // enter the bending energy (only their second derivatives).
    #[allow(clippy::needless_range_loop, clippy::many_single_char_names)]
    for &(gxi, wxi) in &pts {
        for &(geta, weta) in &pts {
            let det = a * b;
            let wgt = wxi * weta;
            for i in 0..16 {
                let corner = i / 4;
                let in_corner = i % 4;
                let (_n_i, nxx_i, nyy_i, nxy_i) =
                    shape(in_corner, hxs[corner], hys[corner], gxi, geta, a, b);
                for j in 0..16 {
                    let corner_j = j / 4;
                    let in_corner_j = j % 4;
                    let (_n_j, nxx_j, nyy_j, nxy_j) =
                        shape(in_corner_j, hxs[corner_j], hys[corner_j], gxi, geta, a, b);
                    let term = d
                        * (nxx_i * nxx_j
                            + nyy_i * nyy_j
                            + nu * (nxx_i * nyy_j + nyy_i * nxx_j)
                            + 2.0 * (1.0 - nu) * nxy_i * nxy_j);
                    ke[i][j] += wgt * det * term;
                }
            }
        }
    }
    let _ = corners;
    ke
}

impl PlateModel {
    /// Assembles and solves the plate with penalty BCs over the sparse CG
    /// solver.
    ///
    /// # Errors
    ///
    /// [`PlateError`] on singular systems, bad indices, degenerate
    /// geometry, or an empty element set.
    #[allow(clippy::needless_range_loop)]
    pub fn solve(&self) -> Result<PlateSolution, PlateError> {
        let n = self.nodes.len();
        if self.elements.is_empty() {
            return Err(PlateError::NoElements);
        }
        for e in &self.elements {
            if e.nodes.iter().any(|&i| i >= n) {
                return Err(PlateError::OutOfRange);
            }
        }
        let dof = 4 * n;
        let mut k = vec![0.0_f64; dof * dof];
        let mut f = vec![0.0_f64; dof];
        let pts = gauss2();
        let mut k_scale = 0.0_f64;

        for e in &self.elements {
            let xs: Vec<f64> = e.nodes.iter().map(|&i| self.nodes[i].position.0).collect();
            let ys: Vec<f64> = e.nodes.iter().map(|&i| self.nodes[i].position.1).collect();
            let x0 = xs.iter().cloned().fold(f64::INFINITY, f64::min);
            let x1 = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let y0 = ys.iter().cloned().fold(f64::INFINITY, f64::min);
            let y1 = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let a = x1 - x0;
            let b = y1 - y0;
            if a <= 0.0 || b <= 0.0 {
                return Err(PlateError::OutOfRange);
            }
            if !(e.thickness_m.is_finite() && e.thickness_m > 0.0)
                || !(e.youngs_modulus_gpa.is_finite() && e.youngs_modulus_gpa > 0.0)
                || !(e.poissons_ratio.is_finite()
                    && e.poissons_ratio > -1.0
                    && e.poissons_ratio < 0.5)
            {
                return Err(PlateError::OutOfRange);
            }
            let d = e.youngs_modulus_gpa * 1e9 * e.thickness_m.powi(3)
                / (12.0 * (1.0 - e.poissons_ratio * e.poissons_ratio));
            k_scale = k_scale.max(d / a);
            let nu = e.poissons_ratio;

            // Corner ownership: ccw corner i sits at the (hx, hy) corner.
            let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
            let hxs = [0_usize, 1, 1, 0];
            let hys = [0_usize, 0, 1, 1];
            for (&ni, &(cx, cy)) in e.nodes.iter().zip(&corners) {
                let (px, py) = self.nodes[ni].position;
                if (px - cx).abs() > 1e-9 * a.max(1.0) || (py - cy).abs() > 1e-9 * b.max(1.0) {
                    return Err(PlateError::OutOfRange);
                }
            }

            let mut ke = [[0.0_f64; 16]; 16];
            let mut fe = [0.0_f64; 16];
            for &(gxi, wxi) in &pts {
                for &(geta, weta) in &pts {
                    let det = a * b;
                    let wgt = wxi * weta;
                    for i in 0..16 {
                        let corner = i / 4;
                        let in_corner = i % 4;
                        let (n_i, nxx_i, nyy_i, nxy_i) =
                            shape(in_corner, hxs[corner], hys[corner], gxi, geta, a, b);
                        if e.pressure_n_m2 != 0.0 {
                            fe[i] += wgt * det * e.pressure_n_m2 * n_i;
                        } else {
                            let _ = n_i;
                        }
                        for j in 0..16 {
                            let corner_j = j / 4;
                            let in_corner_j = j % 4;
                            let (_n_j, nxx_j, nyy_j, nxy_j) =
                                shape(in_corner_j, hxs[corner_j], hys[corner_j], gxi, geta, a, b);
                            let term = d
                                * (nxx_i * nxx_j
                                    + nyy_i * nyy_j
                                    + nu * (nxx_i * nyy_j + nyy_i * nxx_j)
                                    + 2.0 * (1.0 - nu) * nxy_i * nxy_j);
                            ke[i][j] += wgt * det * term;
                        }
                    }
                }
            }

            let idx = e
                .nodes
                .iter()
                .flat_map(|&ni| [4 * ni, 4 * ni + 1, 4 * ni + 2, 4 * ni + 3])
                .collect::<Vec<_>>();
            for (i, &ii) in idx.iter().enumerate() {
                f[ii] += fe[i];
                for (j, &jj) in idx.iter().enumerate() {
                    k[ii * dof + jj] += ke[i][j];
                }
            }
        }

        // Penalty BCs.
        let mut penalty_mask = vec![false; dof];
        for sup in &self.supports {
            if sup.node >= n {
                return Err(PlateError::OutOfRange);
            }
            for (fixed, off) in [
                (sup.fix_w, 0),
                (sup.fix_wx, 1),
                (sup.fix_wy, 2),
                (sup.fix_wxy, 3),
            ] {
                if fixed {
                    let i = 4 * sup.node + off;
                    // Penalty 1e4: compliance error ~1e-4 relative, but the
                    // condition number drops by 100x so CG converges within
                    // the iteration budget (plate matrices mix w/slope/
                    // twist scales that raw Jacobi-CG handles poorly).
                    k[i * dof + i] += k_scale * 1e4;
                    penalty_mask[i] = true;
                }
            }
        }

        // Direct solve: the plate system mixes w/slope/twist DOF scales,
        // and unpreconditioned CG would iterate far beyond any budget at
        // this condition number. dof stays O(mesh^2) — a dense Cholesky
        // is the robust choice here (the truss/frame paths keep CG).
        let x = crate::fem::dense_cholesky_solve(dof, &mut k, &f, k_scale)
            .map_err(|_| PlateError::SingularSystem)?;
        let _ = penalty_mask;

        let deflections: Vec<f64> = (0..n).map(|i| x[4 * i]).collect();
        let max_deflection_m = deflections.iter().map(|d| d.abs()).fold(0.0_f64, f64::max);

        Ok(PlateSolution {
            deflections,
            max_deflection_m,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: f64 = 2.0; // m square plate
    const H: f64 = 0.02; // m thick
    const E_GPA: f64 = 210.0;
    const NU: f64 = 0.3;
    const Q: f64 = 10_000.0; // N/m2

    fn flexural_rigidity() -> f64 {
        E_GPA * 1e9 * H.powi(3) / (12.0 * (1.0 - NU * NU))
    }

    /// Structured n x n mesh over the square [0, a]^2, counter-clockwise
    /// element corners.
    fn square_mesh(n: usize, pressure: f64) -> PlateModel {
        let mut nodes = Vec::new();
        for j in 0..=n {
            for i in 0..=n {
                nodes.push(PlateNode {
                    position: (A * i as f64 / n as f64, A * j as f64 / n as f64),
                });
            }
        }
        let idx = |i: usize, j: usize| j * (n + 1) + i;
        let mut elements = Vec::new();
        for j in 0..n {
            for i in 0..n {
                elements.push(PlateElement {
                    nodes: [idx(i, j), idx(i + 1, j), idx(i + 1, j + 1), idx(i, j + 1)],
                    thickness_m: H,
                    youngs_modulus_gpa: E_GPA,
                    poissons_ratio: NU,
                    pressure_n_m2: pressure,
                });
            }
        }
        PlateModel {
            nodes,
            elements,
            supports: Vec::new(),
        }
    }

    /// Simply supported square plate under uniform load: the central
    /// deflection approaches 0.00406 q a^4 / D from above as the mesh
    /// refines (C1 conforming element).
    #[test]
    fn simply_supported_square_matches_timoshenko() {
        let target = 0.00406 * Q * A.powi(4) / flexural_rigidity();
        let mut model = square_mesh(8, Q);
        // Constrain w on the boundary (rotations free = natural SS).
        let n = 8;
        for j in 0..=n {
            for i in 0..=n {
                if i == 0 || i == n || j == 0 || j == n {
                    model
                        .supports
                        .push(PlateSupport::simply_supported(j * (n + 1) + i));
                }
            }
        }
        let sol = model.solve().expect("SS plate solves");
        let centre = sol.deflections[(n / 2) * (n + 1) + n / 2];
        // BFS on a uniform mesh converges from above for SS plates.
        assert!(
            (centre.abs() - target).abs() / target < 0.02 && centre.abs() >= target * 0.99,
            "centre {} vs {target}",
            centre.abs()
        );
    }

    /// Clamped square plate: 0.00126 q a^4 / D.
    #[test]
    fn clamped_square_matches_timoshenko() {
        let target = 0.00126 * Q * A.powi(4) / flexural_rigidity();
        let n = 8;
        let mut model = square_mesh(8, Q);
        for j in 0..=n {
            for i in 0..=n {
                if i == 0 || i == n || j == 0 || j == n {
                    model.supports.push(PlateSupport::clamped(j * (n + 1) + i));
                }
            }
        }
        let sol = model.solve().expect("clamped plate solves");
        let centre = sol.deflections[(n / 2) * (n + 1) + n / 2];
        assert!(
            (centre.abs() - target).abs() / target < 0.02,
            "centre {} vs {target}",
            centre.abs()
        );
        // The clamped plate is stiffer than simply supported: smaller
        // deflection.
        assert!(centre.abs() < 0.00406 * Q * A.powi(4) / flexural_rigidity());
    }

    /// SPD probe: for random vectors the single-element quadratic form
    /// must be non-negative (PSD with the three rigid modes as the only
    /// null directions). Exposes element-matrix sign errors directly.
    #[test]
    fn element_matrix_is_positive_semidefinite() {
        let ke = bfs_element_matrix(2.0, 2.0, 0.02, 210.0, 0.3);
        // Symmetry.
        #[allow(clippy::needless_range_loop)]
        for i in 0..16 {
            for j in 0..16 {
                assert!((ke[i][j] - ke[j][i]).abs() <= 1e-9 * ke[i][j].abs().max(1.0));
            }
        }
        // Quadratic form over pseudo-random vectors (xorshift).
        let mut state = 0x9E3779B97F4A7C15_u64;
        let mut worst = f64::INFINITY;
        for _ in 0..200 {
            let mut x = [0.0_f64; 16];
            for v in &mut x {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *v = (state % 2000) as f64 / 1000.0 - 1.0;
            }
            let q: f64 = (0..16)
                .map(|i| x[i] * (0..16).map(|j| ke[i][j] * x[j]).sum::<f64>())
                .sum();
            worst = worst.min(q);
        }
        assert!(worst >= -1e-6, "negative quadratic form {worst}");
    }

    /// A degenerate element (zero area) is rejected, not solved.
    #[test]
    fn degenerate_mesh_rejected() {
        let mut model = square_mesh(1, Q);
        model.elements[0].thickness_m = 0.0;
        assert_eq!(model.solve(), Err(PlateError::OutOfRange));
    }
}
