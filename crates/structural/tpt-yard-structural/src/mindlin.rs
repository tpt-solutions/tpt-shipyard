//! Mindlin (shear-deformable) plate elements — the MITC4 ansatz of
//! Bathe & Dvorkin — on the same axis-aligned rectangular meshes as the
//! thin-plate [`PlateModel`](crate::plates::PlateModel).
//!
//! Each node carries `(w, theta_x, theta_y)`; the kinematics are
//! `u = z·theta_y`, `v = -z·theta_x`, `u_z = w(x, y)`. The curvatures
//! are `kappa_x = theta_y,x`, `kappa_y = -theta_x,y`,
//! `kappa_xy = theta_y,y - theta_x,x`, and the transverse shears would
//! lock if interpolated from `grad w` directly — the MITC4 assumed-strain
//! interpolation samples the covariant shears at the four edge
//! midsides instead,
//!
//! ```text
//! gamma_xz^A = (w2-w1)/dx + (theta_y1 + theta_y2)/2      (edge 1-2)
//! gamma_xz^C = (w3-w4)/dx + (theta_y3 + theta_y4)/2      (edge 3-4)
//! gamma_yz^B = (w3-w2)/dy - (theta_x2 + theta_x3)/2      (edge 2-3)
//! gamma_yz^D = (w4-w1)/dy - (theta_x1 + theta_x4)/2      (edge 1-4)
//! ```
//!
//! and bilinearly interpolates each component between its two edges.
//! Because every edge value depends only on that edge's own nodes, the
//! assumed shear field is conforming, and the element reproduces the
//! thin-plate (Kirchhoff) limit without shear locking.
//!
//! Conventions (the failure mode of the previous attempt): with
//! `u = z·theta_y`, the thin-limit constraint `gamma -> 0` forces
//! `theta_y = -w,x` and `theta_x = w,y` — the rotation DOFs carry the
//! *negative* x-slope. The edge directions above follow the
//! counter-clockwise node order (1-2 and 4-3 along x, 1-4 and 2-3
//! along y), which is what makes the signs in the midsides formulas
//! work out.
//!
//! Verification: the thin-plate limit against the Timoshenko values
//! (0.00406 and 0.00126 `q a^4/D`) on the same meshes as BFS, an
//! instrumented Mindlin-vs-BFS comparison, strain-free rigid/linear
//! fields, and element-matrix symmetry/PSD probes.

use crate::plates::{PlateElement, PlateError, PlateNode, PlateSolution};

/// Transverse shear correction factor for a homogeneous rectangular
/// section (the standard 5/6).
pub const SHEAR_CORRECTION: f64 = 5.0 / 6.0;

/// DOF support at a node: constrain any of the three Mindlin quantities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindlinSupport {
    /// Node index.
    pub node: usize,
    /// Fix deflection w.
    pub fix_w: bool,
    /// Fix rotation theta_x (about the x axis).
    pub fix_thx: bool,
    /// Fix rotation theta_y (about the y axis).
    pub fix_thy: bool,
}

impl MindlinSupport {
    /// Clamped node: deflection and both rotations fixed.
    pub fn clamped(node: usize) -> Self {
        Self {
            node,
            fix_w: true,
            fix_thx: true,
            fix_thy: true,
        }
    }

    /// "Soft" simply supported node: deflection fixed, rotations free —
    /// the support whose thin limit is the Kirchhoff simply supported
    /// plate behind the 0.00406 target.
    pub fn simply_supported(node: usize) -> Self {
        Self {
            node,
            fix_w: true,
            fix_thx: false,
            fix_thy: false,
        }
    }
}

/// A rectangular shear-deformable plate model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MindlinModel {
    /// Nodes.
    pub nodes: Vec<PlateNode>,
    /// Rectangular elements (axis-aligned; ccw corner order).
    pub elements: Vec<PlateElement>,
    /// Supports.
    pub supports: Vec<MindlinSupport>,
}

/// The 12x12 element matrix for one rectangular MITC4 element (exposed
/// for the strain-free and PSD probes in tests). Node order ccw:
/// `(x0,y0), (x1,y0), (x1,y1), (x0,y1)`; DOFs per node `(w, tx, ty)`.
pub(crate) fn mindlin_element_matrix(
    dx: f64,
    dy: f64,
    thickness_m: f64,
    youngs_modulus_gpa: f64,
    poissons_ratio: f64,
) -> [[f64; 12]; 12] {
    let hx = dx / 2.0;
    let hy = dy / 2.0;
    let t = thickness_m;
    let e = youngs_modulus_gpa * 1e9;
    let nu = poissons_ratio;
    let d_b = e * t.powi(3) / (12.0 * (1.0 - nu * nu));
    let g = e / (2.0 * (1.0 + nu));
    let d_s = SHEAR_CORRECTION * g * t;

    // Bilinear shape function derivatives at a point (xi, eta).
    let n_xi = |i: usize, _xi: f64, eta: f64| -> f64 {
        let (s, r) = match i {
            0 => (-1.0, -1.0),
            1 => (1.0, -1.0),
            2 => (1.0, 1.0),
            _ => (-1.0, 1.0),
        };
        s * (1.0 + eta * r) / 4.0
    };
    let n_eta = |i: usize, xi: f64, _eta: f64| -> f64 {
        let (s, r) = match i {
            0 => (-1.0, -1.0),
            1 => (1.0, -1.0),
            2 => (1.0, 1.0),
            _ => (-1.0, 1.0),
        };
        (1.0 + xi * s) * r / 4.0
    };

    // The covariant midsides shear values as 12-vectors over the
    // element DOFs (node i owns DOFs 3i..3i+2): gamma components are
    // linear in the DOFs, so each midsides value is one 12-vector.
    let mut a = [0.0_f64; 12]; // gamma_xz at edge 1-2 (nodes 0,1)
    let mut c = [0.0_f64; 12]; // gamma_xz at edge 3-4 (nodes 2,3)
    let mut b = [0.0_f64; 12]; // gamma_yz at edge 2-3 (nodes 1,2)
    let mut dd = [0.0_f64; 12]; // gamma_yz at edge 1-4 (nodes 0,3)
                                // gamma_xz^A = (w2 - w1)/dx + (theta_y1 + theta_y2)/2
    a[0] = -1.0 / dx;
    a[2] = 0.5;
    a[3] = 1.0 / dx;
    a[5] = 0.5;
    // gamma_xz^C = (w3 - w4)/dx + (theta_y3 + theta_y4)/2
    c[6] = 1.0 / dx;
    c[8] = 0.5;
    c[9] = -1.0 / dx;
    c[11] = 0.5;
    // gamma_yz^B = (w3 - w2)/dy - (theta_x2 + theta_x3)/2
    b[3] = -1.0 / dy;
    b[4] = -0.5;
    b[6] = 1.0 / dy;
    b[7] = -0.5;
    // gamma_yz^D = (w4 - w1)/dy - (theta_x1 + theta_x4)/2
    dd[0] = -1.0 / dy;
    dd[1] = -0.5;
    dd[9] = 1.0 / dy;
    dd[10] = -0.5;

    let g2 = 0.577_350_269_189_625_7_f64;
    let mut ke = [[0.0_f64; 12]; 12];
    for &xi in &[-g2, g2] {
        for &eta in &[-g2, g2] {
            // Assumed shear field: each physical component interpolates
            // between its two edges.
            let mut bs = [[0.0_f64; 12]; 2]; // rows: gamma_xz, gamma_yz
            for dof in 0..12 {
                bs[0][dof] = ((1.0 - eta) / 2.0) * a[dof] + ((1.0 + eta) / 2.0) * c[dof];
                bs[1][dof] = ((1.0 - xi) / 2.0) * dd[dof] + ((1.0 + xi) / 2.0) * b[dof];
            }
            // Bending curvature rows: [theta_y,x; -theta_x,y;
            // theta_y,y - theta_x,x].
            let mut bb = [[0.0_f64; 12]; 3];
            for i in 0..4 {
                let (dxi, deta) = (n_xi(i, xi, eta), n_eta(i, xi, eta));
                bb[0][3 * i + 2] = dxi / hx; // theta_y,x
                bb[1][3 * i + 1] = -deta / hy; // -theta_x,y
                bb[2][3 * i + 2] = deta / hy; // theta_y,y
                bb[2][3 * i + 1] = -dxi / hx; // -theta_x,x
            }
            let det = hx * hy;
            let db_mat = [
                [d_b, d_b * nu, 0.0],
                [d_b * nu, d_b, 0.0],
                [0.0, 0.0, d_b * (1.0 - nu) / 2.0],
            ];
            for p in 0..12 {
                for q in 0..12 {
                    let mut term = 0.0;
                    for (r, row) in bb.iter().enumerate() {
                        for (s2, col) in bb.iter().enumerate() {
                            term += row[p] * db_mat[r][s2] * col[q];
                        }
                    }
                    term += d_s * (bs[0][p] * bs[0][q] + bs[1][p] * bs[1][q]);
                    ke[p][q] += det * term;
                }
            }
        }
    }
    ke
}

impl MindlinModel {
    /// Assembles and solves the plate with penalty BCs (dense Cholesky —
    /// the w/rotation scale mixing rules out raw CG, as for BFS).
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
        let dof = 3 * n;
        let mut k = vec![0.0_f64; dof * dof];
        let mut f = vec![0.0_f64; dof];
        let mut k_scale = 0.0_f64;

        for e in &self.elements {
            let xs: Vec<f64> = e.nodes.iter().map(|&i| self.nodes[i].position.0).collect();
            let ys: Vec<f64> = e.nodes.iter().map(|&i| self.nodes[i].position.1).collect();
            let x0 = xs.iter().cloned().fold(f64::INFINITY, f64::min);
            let x1 = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let y0 = ys.iter().cloned().fold(f64::INFINITY, f64::min);
            let y1 = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let (dx, dy) = (x1 - x0, y1 - y0);
            if dx <= 0.0 || dy <= 0.0 {
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
            let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
            for (&ni, &(cx, cy)) in e.nodes.iter().zip(&corners) {
                let (px, py) = self.nodes[ni].position;
                if (px - cx).abs() > 1e-9 * dx.max(1.0) || (py - cy).abs() > 1e-9 * dy.max(1.0) {
                    return Err(PlateError::OutOfRange);
                }
            }
            let g = e.youngs_modulus_gpa * 1e9 / (2.0 * (1.0 + e.poissons_ratio));
            let d_b = e.youngs_modulus_gpa * 1e9 * e.thickness_m.powi(3)
                / (12.0 * (1.0 - e.poissons_ratio * e.poissons_ratio));
            k_scale = k_scale
                .max(d_b / dx.max(dy).powi(2))
                .max(SHEAR_CORRECTION * g * e.thickness_m);

            let ke = mindlin_element_matrix(
                dx,
                dy,
                e.thickness_m,
                e.youngs_modulus_gpa,
                e.poissons_ratio,
            );
            // Consistent pressure load: the bilinear shape functions
            // integrate to dx*dy/4 per corner; pressure does no work on
            // the rotation DOFs.
            let idx = e
                .nodes
                .iter()
                .flat_map(|&ni| [3 * ni, 3 * ni + 1, 3 * ni + 2])
                .collect::<Vec<_>>();
            for i in 0..4 {
                f[idx[3 * i]] += e.pressure_n_m2 * dx * dy / 4.0;
            }
            for (p, &pi) in idx.iter().enumerate() {
                for (q, &qi) in idx.iter().enumerate() {
                    k[pi * dof + qi] += ke[p][q];
                }
            }
        }

        for sup in &self.supports {
            if sup.node >= n {
                return Err(PlateError::OutOfRange);
            }
            for (fixed, off) in [(sup.fix_w, 0), (sup.fix_thx, 1), (sup.fix_thy, 2)] {
                if fixed {
                    let i = 3 * sup.node + off;
                    k[i * dof + i] += k_scale * 1e4;
                }
            }
        }

        let x = crate::fem::dense_cholesky_solve(dof, &mut k, &f, k_scale)
            .map_err(|_| PlateError::SingularSystem)?;
        let deflections: Vec<f64> = (0..n).map(|i| x[3 * i]).collect();
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
    use crate::plates::PlateSupport;

    const A: f64 = 2.0; // m square plate
    const H_THIN: f64 = 0.02; // m (span/thickness = 100)
    const E_GPA: f64 = 210.0;
    const NU: f64 = 0.3;
    const Q: f64 = 10_000.0; // N/m2

    fn flexural_rigidity() -> f64 {
        E_GPA * 1e9 * H_THIN.powi(3) / (12.0 * (1.0 - NU * NU))
    }

    /// Structured n x n mesh over [0, a]^2 (shared shape with the BFS
    /// tests so both solvers see identical meshes).
    fn square_mesh(
        n: usize,
        thickness: f64,
        pressure: f64,
    ) -> (MindlinModel, crate::plates::PlateModel) {
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
                    thickness_m: thickness,
                    youngs_modulus_gpa: E_GPA,
                    poissons_ratio: NU,
                    pressure_n_m2: pressure,
                });
            }
        }
        let mindlin = MindlinModel {
            nodes: nodes.clone(),
            elements: elements.clone(),
            supports: Vec::new(),
        };
        let bfs = crate::plates::PlateModel {
            nodes,
            elements,
            supports: Vec::new(),
        };
        (mindlin, bfs)
    }

    fn add_supports_mindlin(model: &mut MindlinModel, n: usize, clamped: bool) {
        for j in 0..=n {
            for i in 0..=n {
                if i == 0 || i == n || j == 0 || j == n {
                    let node = j * (n + 1) + i;
                    model.supports.push(if clamped {
                        MindlinSupport::clamped(node)
                    } else {
                        MindlinSupport::simply_supported(node)
                    });
                }
            }
        }
    }

    fn add_supports_bfs(model: &mut crate::plates::PlateModel, n: usize, clamped: bool) {
        for j in 0..=n {
            for i in 0..=n {
                if i == 0 || i == n || j == 0 || j == n {
                    let node = j * (n + 1) + i;
                    model.supports.push(if clamped {
                        PlateSupport::clamped(node)
                    } else {
                        PlateSupport::simply_supported(node)
                    });
                }
            }
        }
    }

    /// THE instrumented convergence study (the first step the review
    /// note prescribes): Mindlin vs BFS on identical meshes in the thin
    /// limit — both must reproduce the Timoshenko values, and the
    /// Mindlin error must shrink with refinement.
    #[test]
    fn thin_plate_converges_to_the_bfs_and_timoshenko_values() {
        let target_ss = 0.00406 * Q * A.powi(4) / flexural_rigidity();
        let target_cl = 0.00126 * Q * A.powi(4) / flexural_rigidity();

        // Simply supported.
        let mut prev_err = f64::INFINITY;
        for n in [4_usize, 8, 16] {
            let (mut m, _) = square_mesh(n, H_THIN, Q);
            add_supports_mindlin(&mut m, n, false);
            let sol = m.solve().expect("SS Mindlin solves");
            let centre = sol.deflections[(n / 2) * (n + 1) + n / 2].abs();
            let err = (centre - target_ss).abs() / target_ss;
            assert!(
                err < 0.03,
                "SS mesh {n}: centre {centre} vs target {target_ss} (err {err})"
            );
            assert!(
                err < prev_err,
                "SS error must shrink with refinement: {err} after {prev_err}"
            );
            prev_err = err;
        }

        // Clamped, with the Mindlin-vs-BFS instrument on the same mesh.
        for n in [4_usize, 8] {
            let (mut m, mut b) = square_mesh(n, H_THIN, Q);
            add_supports_mindlin(&mut m, n, true);
            add_supports_bfs(&mut b, n, true);
            let sol = m.solve().expect("clamped Mindlin solves");
            let bfs = b.solve().expect("clamped BFS solves");
            let centre = sol.deflections[(n / 2) * (n + 1) + n / 2].abs();
            let bfs_centre = bfs.deflections[(n / 2) * (n + 1) + n / 2].abs();
            let err = (centre - target_cl).abs() / target_cl;
            assert!(
                err < 0.05,
                "clamped mesh {n}: centre {centre} vs {target_cl} (err {err})"
            );
            let agree = (centre - bfs_centre).abs() / bfs_centre;
            assert!(
                agree < 0.05,
                "Mindlin {centre} and BFS {bfs_centre} must agree in the thin limit (mesh {n})"
            );
        }
    }

    /// Strain-free fields produce no nodal force: the rigid modes
    /// (constant w) and the linear slope field `w = alpha x` with the
    /// coupled rotations `theta_y = -alpha, theta_x = 0` (the thin-limit
    /// constraint) are both in the element's null space. This probe
    /// kills the DOF-convention coupling sign error directly.
    #[test]
    fn strain_free_fields_exert_no_force() {
        let ke = mindlin_element_matrix(0.5, 0.5, 0.02, 210.0, 0.3);
        let mat_vec = |u: &[f64; 12]| -> [f64; 12] {
            let mut out = [0.0_f64; 12];
            for (p, o) in out.iter_mut().enumerate() {
                *o = (0..12).map(|q| ke[p][q] * u[q]).sum();
            }
            out
        };
        let norm = |v: &[f64; 12]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
        let k_norm = {
            let mut biggest = 0.0_f64;
            for row in &ke {
                for v in row {
                    biggest = biggest.max(v.abs());
                }
            }
            biggest * 12.0
        };
        // Constant deflection (all four w DOFs equal).
        let rigid = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let r = mat_vec(&rigid);
        assert!(norm(&r) < 1e-9 * k_norm, "rigid w residual {}", norm(&r));
        // Linear slope w = alpha x, theta_y = -alpha: zero shear AND zero
        // curvature (alpha * x is linear).
        let alpha = 0.7_f64;
        // Node x positions for a dx = 0.5 element: -0.25, 0.25, 0.25,
        // -0.25 relative to the centre.
        let w = |x: f64| alpha * x;
        let u = [
            w(-0.25),
            0.0,
            -alpha,
            w(0.25),
            0.0,
            -alpha,
            w(0.25),
            0.0,
            -alpha,
            w(-0.25),
            0.0,
            -alpha,
        ];
        let r = mat_vec(&u);
        assert!(
            norm(&r) < 1e-9 * k_norm,
            "linear-slope residual {}",
            norm(&r)
        );
        // Twisting thin-limit mode w = (x-c)(y-c), theta_x = (x-c),
        // theta_y = -(y-c): shear-free (the constraint that kills
        // locking) but NOT strain-free — it carries kappa_xy = -2, so
        // the rotation DOFs legitimately feel the bending force. Two
        // exact assertions: the w rows of K·u vanish (no shear force on
        // the deflections), and the strain energy equals the pure
        // bending value d_b (1-nu) A for kappa_xy = -2. This is the
        // regression for the edge-orientation bug — the linear fields
        // above never exercise the w_y edge terms the gamma_yz midsides
        // carry (the previous attempt's failure mode); a wrong edge
        // operator puts shear strain into this mode and both checks
        // blow up (observed 2000x energy inflation).
        // Element side 0.5 m, node offsets +/- 0.25 from the centre.
        let u_twist = [
            0.0625, -0.25, 0.25, -0.0625, 0.25, 0.25, 0.0625, 0.25, -0.25, -0.0625, -0.25, -0.25,
        ];
        let r = mat_vec(&u_twist);
        let w_residual: f64 = (0..4)
            .map(|node| r[3 * node])
            .map(f64::abs)
            .fold(0.0, f64::max);
        assert!(
            w_residual < 1e-9 * k_norm,
            "twist-mode w-row residual {w_residual} (gamma_yz edge orientation)"
        );
        let energy = 0.5 * (0..12).map(|p| u_twist[p] * r[p]).sum::<f64>();
        let d_b = 210.0e9 * 0.02_f64.powi(3) / (12.0 * (1.0 - 0.3 * 0.3));
        let expected = d_b * (1.0 - 0.3) * 0.25; // x area A = 0.25 m^2
        assert!(
            (energy - expected).abs() < 1e-9 * expected,
            "twist energy {energy} vs pure bending {expected}"
        );
    }

    /// Element matrix is symmetric and positive semidefinite (with the
    /// three rigid modes as the only null space).
    #[test]
    fn element_matrix_is_symmetric_and_psd() {
        let ke = mindlin_element_matrix(0.5, 0.4, 0.05, 210.0, 0.3);
        #[allow(clippy::needless_range_loop)]
        for p in 0..12 {
            for q in 0..12 {
                let scale = ke[p][q].abs().max(1.0);
                assert!((ke[p][q] - ke[q][p]).abs() <= 1e-9 * scale, "{p},{q}");
            }
        }
        let mut state = 0x9E3779B97F4A7C15_u64;
        for _ in 0..200 {
            let mut x = [0.0_f64; 12];
            for v in &mut x {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *v = (state % 2000) as f64 / 1000.0 - 1.0;
            }
            let q_form: f64 = (0..12)
                .map(|i| x[i] * (0..12).map(|j| ke[i][j] * x[j]).sum::<f64>())
                .sum();
            assert!(q_form >= -1e-9, "negative quadratic form {q_form}");
        }
    }

    /// A thick plate (span/thickness = 5) is more flexible than the
    /// thin-plate theory predicts: the shear contribution adds
    /// deflection, and the effect must exceed the discretisation error
    /// to be meaningful.
    #[test]
    fn thick_plate_is_shear_flexible() {
        let n = 8;
        let thick = A / 5.0;
        let (mut m, _) = square_mesh(n, thick, Q);
        add_supports_mindlin(&mut m, n, false);
        let sol = m.solve().expect("thick SS Mindlin solves");
        let centre = sol.deflections[(n / 2) * (n + 1) + n / 2].abs();
        // Kirchhoff value with the THICK plate's rigidity (what a
        // shear-stiff theory would give): shear must push past it.
        let d_thick = E_GPA * 1e9 * thick.powi(3) / (12.0 * (1.0 - NU * NU));
        let kirchhoff = 0.00406 * Q * A.powi(4) / d_thick;
        assert!(
            centre > kirchhoff * 1.05,
            "thick plate {centre} must be visibly more flexible than Kirchhoff {kirchhoff}"
        );
    }

    /// Degenerate meshes and bad properties are rejected, matching the
    /// BFS error contract.
    #[test]
    fn degenerate_mesh_rejected() {
        let (mut model, _) = square_mesh(1, H_THIN, Q);
        model.elements[0].thickness_m = 0.0;
        assert_eq!(model.solve(), Err(PlateError::OutOfRange));
        let (mut model, _) = square_mesh(1, H_THIN, Q);
        model.elements[0].nodes[2] = 99;
        assert_eq!(model.solve(), Err(PlateError::OutOfRange));
        let (model, _) = square_mesh(0, H_THIN, Q);
        assert_eq!(model.solve(), Err(PlateError::NoElements));
    }
}
