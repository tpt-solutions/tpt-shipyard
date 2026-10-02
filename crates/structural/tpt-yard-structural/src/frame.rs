//! 2D frame (beam) elements — review 7H roadmap: "beam/shell elements".
//!
//! [`FrameModel`] solves plane frames (beam + column members with axial,
//! shear and bending stiffness) by the same penalized sparse CG path as
//! the truss model. Each node carries three DOFs: axial ux, transverse uz
//! and rotation theta. Shell elements remain roadmap.
//!
//! All members are Euler-Bernoulli beams; with end loads (or the
//! consistent load vector of a uniform member load on a horizontal
//! member) the Hermite-cubic shape functions reproduce the classical
//! closed forms exactly at the nodes, which the tests exploit: cantilever
//! tip deflection `P L^3 / 3EI`, simply-supported midspan `P L^3 / 48EI`,
//! uniform-load midspan `5 w L^4 / 384 EI`.

/// A node of a plane frame: position in the x-z plane, m.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameNode {
    /// Position, m (x longitudinal, z vertical).
    pub position: (f64, f64),
}

/// A frame member between two nodes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameElement {
    /// Node indices `[from, to]`.
    pub nodes: [usize; 2],
    /// Cross-section area, m².
    pub area_m2: f64,
    /// Second moment of area about the bending axis, m⁴.
    pub inertia_m4: f64,
    /// Young's modulus, GPa.
    pub youngs_modulus_gpa: f64,
    /// Density for self weight, kg/m³ (0 = ignore). Lumped half to each
    /// end node in global -z.
    pub density_kg_m3: f64,
}

/// Support conditions per DOF.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameSupport {
    /// Node index.
    pub node: usize,
    /// Fix longitudinal displacement (ux).
    pub fix_x: bool,
    /// Fix transverse displacement (uz).
    pub fix_z: bool,
    /// Fix rotation (theta).
    pub fix_rot: bool,
}

impl FrameSupport {
    /// A fully fixed (encastré) node.
    pub fn fixed(node: usize) -> Self {
        Self {
            node,
            fix_x: true,
            fix_z: true,
            fix_rot: true,
        }
    }

    /// A pinned node (translations fixed, rotation free).
    pub fn pinned(node: usize) -> Self {
        Self {
            node,
            fix_x: true,
            fix_z: true,
            fix_rot: false,
        }
    }

    /// A roller: transverse support only.
    pub fn roller_z(node: usize) -> Self {
        Self {
            node,
            fix_x: false,
            fix_z: true,
            fix_rot: false,
        }
    }
}

/// A nodal load: forces in N, moment in N·m.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameLoad {
    /// Node index.
    pub node: usize,
    /// Longitudinal force, N.
    pub fx: f64,
    /// Transverse force, N.
    pub fz: f64,
    /// Moment, N·m.
    pub moment_nm: f64,
}

/// A uniform transverse load on a member, N/m (positive in global +z).
/// For horizontal members the consistent load vector is exact; for
/// inclined members the load is projected on the member's local
/// transverse axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemberLoad {
    /// Element index.
    pub element: usize,
    /// Uniform load per metre, N/m.
    pub w_n_m: f64,
}

/// Solution of a frame model: displacements per node and member end
/// forces.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameSolution {
    /// Displacement per node: (ux, uz, theta).
    pub displacements: Vec<(f64, f64, f64)>,
    /// Per member: (axial force N, transverse force V, end moments M1,
    /// M2). Axial tension positive; moments positive sagging.
    pub member_forces: Vec<(f64, f64, f64, f64)>,
    /// Largest translation magnitude, m.
    pub max_displacement_m: f64,
}

/// Errors from the frame solver (the same singular verdicts as the truss
/// model).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// The structure is a mechanism at this support set.
    SingularSystem,
    /// A node or element index is out of range, or a member has
    /// non-physical section properties.
    OutOfRange,
    /// No members.
    NoMembers,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::SingularSystem => {
                f.write_str("singular frame stiffness: structure is a mechanism")
            }
            FrameError::OutOfRange => f.write_str("node or element index out of range"),
            FrameError::NoMembers => f.write_str("no frame members"),
        }
    }
}

impl std::error::Error for FrameError {}

/// The assembled penalized system returned by `FrameModel::assemble`.
struct AssembledFrame {
    /// DOF count (3 per node).
    dof: usize,
    /// Row-major penalized stiffness matrix.
    k: Vec<f64>,
    /// Load vector.
    f: Vec<f64>,
    /// Stiffness scale for the solver's pivot thresholds.
    k_scale: f64,
    /// Which DOFs carry penalties (excluded from CG's residual test).
    penalty_mask: Vec<bool>,
    /// Per-member local transverse load (moment recovery).
    w_local_by_element: Vec<f64>,
}

/// A plane frame finite-element model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameModel {
    /// Nodes.
    pub nodes: Vec<FrameNode>,
    /// Members.
    pub elements: Vec<FrameElement>,
    /// Supports.
    pub supports: Vec<FrameSupport>,
    /// Nodal loads.
    pub loads: Vec<FrameLoad>,
    /// Uniform member loads.
    pub member_loads: Vec<MemberLoad>,
}

/// Local 6x6 Euler-Bernoulli stiffness in DOF order
/// `[u1, w1, t1, u2, w2, t2]`.
fn local_stiffness(ea: f64, ei: f64, l: f64) -> [[f64; 6]; 6] {
    let mut kl = [[0.0_f64; 6]; 6];
    let a = 12.0 * ei / (l * l * l);
    let b = 6.0 * ei / (l * l);
    let c = 4.0 * ei / l;
    let d = 2.0 * ei / l;
    let ax = ea / l;
    kl[0][0] = ax;
    kl[0][3] = -ax;
    kl[3][0] = -ax;
    kl[3][3] = ax;
    kl[1][1] = a;
    kl[1][2] = b;
    kl[2][1] = b;
    kl[1][4] = -a;
    kl[4][1] = -a;
    kl[1][5] = b;
    kl[5][1] = b;
    kl[2][2] = c;
    kl[2][4] = -b;
    kl[4][2] = -b;
    kl[2][5] = d;
    kl[5][2] = d;
    kl[4][4] = a;
    kl[4][5] = -b;
    kl[5][4] = -b;
    kl[5][5] = c;
    kl
}

impl FrameModel {
    /// Assembles and solves the frame with penalty BCs over the sparse CG
    /// solver shared with the truss model.
    ///
    /// # Errors
    ///
    /// [`FrameError`] on singular systems, bad indices, or an empty
    /// member set.
    pub fn solve(&self) -> Result<FrameSolution, FrameError> {
        let a = self.assemble()?;
        let x = crate::fem::pcg_solve(a.dof, &a.k, &a.f, a.k_scale, &a.penalty_mask)
            .map_err(|_| FrameError::SingularSystem)?;
        self.recover(a.dof / 3, &x, &a.w_local_by_element)
    }

    /// Assembles and solves the frame with a dense Cholesky over the
    /// penalized system. For **closed rings and arcs**: the CG path's
    /// free-row residual test is blind to rigid modes restrained at a
    /// single penalty DOF, and self-equilibrated loads leave exactly such
    /// modes — the dense solve removes the conditioning gamble (the same
    /// call the plate module made; DOF stays O(mesh²)).
    ///
    /// # Errors
    ///
    /// [`FrameError`] on singular systems, bad indices, or an empty
    /// member set.
    pub fn solve_dense(&self) -> Result<FrameSolution, FrameError> {
        let mut a = self.assemble()?;
        let x = crate::fem::dense_cholesky_solve(a.dof, &mut a.k, &a.f, a.k_scale)
            .map_err(|_| FrameError::SingularSystem)?;
        self.recover(a.dof / 3, &x, &a.w_local_by_element)
    }

    fn assemble(&self) -> Result<AssembledFrame, FrameError> {
        let n = self.nodes.len();
        if self.elements.is_empty() {
            return Err(FrameError::NoMembers);
        }
        for e in &self.elements {
            if e.nodes[0] >= n || e.nodes[1] >= n {
                return Err(FrameError::OutOfRange);
            }
        }
        for load in &self.loads {
            if load.node >= n {
                return Err(FrameError::OutOfRange);
            }
        }
        for ml in &self.member_loads {
            if ml.element >= self.elements.len() {
                return Err(FrameError::OutOfRange);
            }
        }
        let dof = 3 * n;
        let mut k = vec![0.0_f64; dof * dof];
        let mut f = vec![0.0_f64; dof];

        let mut k_scale = 0.0_f64;
        // Per-member local transverse load (for the moment recovery).
        let mut w_local_by_element = vec![0.0_f64; self.elements.len()];

        for e in self.elements.iter() {
            let (na, nb) = (&self.nodes[e.nodes[0]], &self.nodes[e.nodes[1]]);
            let dx = nb.position.0 - na.position.0;
            let dz = nb.position.1 - na.position.1;
            let l = (dx * dx + dz * dz).sqrt();
            if l <= 0.0
                || !e.area_m2.is_finite()
                || e.area_m2 <= 0.0
                || !e.inertia_m4.is_finite()
                || e.inertia_m4 <= 0.0
            {
                return Err(FrameError::OutOfRange);
            }
            let ea = e.youngs_modulus_gpa * 1e9 * e.area_m2;
            let ei_val = e.youngs_modulus_gpa * 1e9 * e.inertia_m4;
            k_scale = k_scale.max(ea / l);
            let (c, s) = (dx / l, dz / l);

            let kl = local_stiffness(ea, ei_val, l);
            // Block-diagonal rotation T = diag(R, R), R = [[c, s, 0],
            // [-s, c, 0], [0, 0, 1]]: kg = T^T kl T.
            let rot = [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]];
            // kg = T^T kl T with T = diag(R, R): first kl T (each block
            // column times R), then R^T times each block row. Block-index
            // arithmetic reads clearest as plain indexed loops.
            // kg = T^T kl T with T = diag(R, R) the global->local map
            // (rot[l][g]): the column pass contracts kl's local component
            // index against rot[l][g], the row pass against rot[l][g] on
            // the left. (Both were transposed before review 7H's inclined
            // cantilever probe — horizontal members hide it behind an
            // identity rotation.)
            let mut kt = [[0.0_f64; 6]; 6];
            #[allow(clippy::needless_range_loop)]
            for a in 0..6 {
                for b in 0..6 {
                    let (lg, bb) = (b % 3, b / 3);
                    let mut sum = 0.0;
                    for ll in 0..3 {
                        sum += kl[a][bb * 3 + ll] * rot[ll][lg];
                    }
                    kt[a][b] = sum;
                }
            }
            let mut kg = [[0.0_f64; 6]; 6];
            #[allow(clippy::needless_range_loop)]
            for a in 0..6 {
                let (lg, ba) = (a % 3, a / 3);
                for b in 0..6 {
                    let mut sum = 0.0;
                    for ll in 0..3 {
                        sum += rot[ll][lg] * kt[ba * 3 + ll][b];
                    }
                    kg[a][b] = sum;
                }
            }
            let idx = [
                3 * e.nodes[0],
                3 * e.nodes[0] + 1,
                3 * e.nodes[0] + 2,
                3 * e.nodes[1],
                3 * e.nodes[1] + 1,
                3 * e.nodes[1] + 2,
            ];
            for i in 0..6 {
                for j in 0..6 {
                    k[idx[i] * dof + idx[j]] += kg[i][j];
                }
            }
            // Self weight: lumped half to each end node in global -z.
            if e.density_kg_m3 > 0.0 {
                let w = e.density_kg_m3 * e.area_m2 * l * 9.81;
                f[idx[1]] += -w / 2.0;
                f[idx[4]] += -w / 2.0;
            }
        }

        for load in &self.loads {
            f[3 * load.node] += load.fx;
            f[3 * load.node + 1] += load.fz;
            f[3 * load.node + 2] += load.moment_nm;
        }

        // Uniform member loads: the consistent nodal vector of the
        // local transverse component (exact for horizontal members).
        for ml in &self.member_loads {
            let e = &self.elements[ml.element];
            let (na, nb) = (&self.nodes[e.nodes[0]], &self.nodes[e.nodes[1]]);
            let dx = nb.position.0 - na.position.0;
            let dz = nb.position.1 - na.position.1;
            let l = (dx * dx + dz * dz).sqrt();
            let (c, _s) = (dx / l, dz / l);
            let w_local = ml.w_n_m * c;
            w_local_by_element[ml.element] = w_local;
            let i1 = 3 * e.nodes[0] + 1;
            let t1 = 3 * e.nodes[0] + 2;
            let i2 = 3 * e.nodes[1] + 1;
            let t2 = 3 * e.nodes[1] + 2;
            f[i1] += w_local * l / 2.0;
            f[t1] += w_local * l * l / 12.0;
            f[i2] += w_local * l / 2.0;
            f[t2] += -w_local * l * l / 12.0;
        }

        // Penalty BCs.
        let mut penalty_mask = vec![false; dof];
        for sup in &self.supports {
            if sup.node >= n {
                return Err(FrameError::OutOfRange);
            }
            for (fixed, off) in [(sup.fix_x, 0), (sup.fix_z, 1), (sup.fix_rot, 2)] {
                if fixed {
                    let i = 3 * sup.node + off;
                    k[i * dof + i] += k_scale * 1e6;
                    penalty_mask[i] = true;
                }
            }
        }

        Ok(AssembledFrame {
            dof,
            k,
            f,
            k_scale,
            penalty_mask,
            w_local_by_element,
        })
    }

    /// Recovers displacements and member end forces from a solved
    /// displacement vector.
    fn recover(
        &self,
        n: usize,
        x: &[f64],
        w_local_by_element: &[f64],
    ) -> Result<FrameSolution, FrameError> {
        let displacements: Vec<(f64, f64, f64)> = (0..n)
            .map(|i| (x[3 * i], x[3 * i + 1], x[3 * i + 2]))
            .collect();

        // Member end forces: internal = k_local u_local - f_eq_local.
        let mut member_forces = Vec::with_capacity(self.elements.len());
        for (ei, e) in self.elements.iter().enumerate() {
            let (na, nb) = (&self.nodes[e.nodes[0]], &self.nodes[e.nodes[1]]);
            let dx = nb.position.0 - na.position.0;
            let dz = nb.position.1 - na.position.1;
            let l = (dx * dx + dz * dz).sqrt();
            let (c, s) = (dx / l, dz / l);
            let ea = e.youngs_modulus_gpa * 1e9 * e.area_m2;
            let ei_val = e.youngs_modulus_gpa * 1e9 * e.inertia_m4;
            let w_local = w_local_by_element[ei];

            // Nodal displacements in the local frame.
            let du1 = x[3 * e.nodes[0]] * c + x[3 * e.nodes[0] + 1] * s;
            let dw1 = -x[3 * e.nodes[0]] * s + x[3 * e.nodes[0] + 1] * c;
            let t1 = x[3 * e.nodes[0] + 2];
            let du2 = x[3 * e.nodes[1]] * c + x[3 * e.nodes[1] + 1] * s;
            let dw2 = -x[3 * e.nodes[1]] * s + x[3 * e.nodes[1] + 1] * c;
            let t2 = x[3 * e.nodes[1] + 2];

            let axial = ea / l * (du2 - du1);
            // End forces from the local stiffness rows: the moment rows
            // carry -6 EI/L^2 on the relative transverse displacement
            // (slope-deflection: -3 psi), +4/+2 on the rotations.
            let shear = ei_val * (12.0 * (dw2 - dw1) / (l * l * l) + 6.0 * (t1 + t2) / (l * l));
            let m1 = ei_val * (-6.0 * (dw2 - dw1) / (l * l) + 4.0 * t1 / l + 2.0 * t2 / l)
                - w_local * l * l / 12.0;
            let m2 = ei_val * (-6.0 * (dw2 - dw1) / (l * l) + 2.0 * t1 / l + 4.0 * t2 / l)
                + w_local * l * l / 12.0;
            member_forces.push((axial, shear, m1, m2));
        }

        let max_displacement_m = displacements
            .iter()
            .map(|(ux, uz, _)| (ux * ux + uz * uz).sqrt())
            .fold(0.0_f64, f64::max);

        Ok(FrameSolution {
            displacements,
            member_forces,
            max_displacement_m,
        })
    }

    /// Appends a circular arch in the x-z plane: `segments` straight
    /// members along the arc of `radius_m` centred at `(cx, cz)`, from
    /// angle `a0` to `a1` (radians from +x, counter-clockwise, x right /
    /// z up), all sharing the given section. Returns the node indices of
    /// the arc's two ends. Faceted arcs are how practice models curved
    /// members with straight frame elements — the membrane verification
    /// below shows the faceting converging to the closed form.
    ///
    /// # Errors
    ///
    /// [`FrameError::OutOfRange`] on a non-positive radius, fewer than
    /// two segments, or a non-physical section.
    pub fn add_arc(
        &mut self,
        cx: f64,
        cz: f64,
        radius_m: f64,
        a0: f64,
        a1: f64,
        segments: u32,
        area_m2: f64,
        inertia_m4: f64,
        youngs_modulus_gpa: f64,
    ) -> Result<(usize, usize), FrameError> {
        if !(radius_m.is_finite() && radius_m > 0.0)
            || segments < 2
            || !(area_m2.is_finite() && area_m2 > 0.0)
            || !(inertia_m4.is_finite() && inertia_m4 > 0.0)
            || !(youngs_modulus_gpa.is_finite() && youngs_modulus_gpa > 0.0)
        {
            return Err(FrameError::OutOfRange);
        }
        let first = self.nodes.len();
        for i in 0..=segments {
            let a = a0 + (a1 - a0) * i as f64 / segments as f64;
            self.nodes.push(FrameNode {
                position: (cx + radius_m * a.cos(), cz + radius_m * a.sin()),
            });
        }
        for i in 0..segments {
            self.elements.push(FrameElement {
                nodes: [first + i as usize, first + i as usize + 1],
                area_m2,
                inertia_m4,
                youngs_modulus_gpa,
                density_kg_m3: 0.0,
            });
        }
        Ok((first, first + segments as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Curved-member verification (review 7H "curved elements"): a ring
    /// under uniform radial pressure is in a pure membrane state — every
    /// section carries the hoop force pR and (essentially) no bending.
    /// Modelled as a half ring with SYMMETRY restraints (uz and rot
    /// fixed on the cut planes, ux fixed at the crown) — restraints that
    /// lie in the null directions of the uniform radial-shrinkage
    /// membrane field, so they cannot self-stress the ring. The faceted
    /// arc converges: member axial approaches pR (compression, negative)
    /// and bending vanishes as the discretisation refines.
    #[test]
    fn faceted_ring_converges_to_the_membrane_hoop_state() {
        let radius = 5.0_f64;
        // Working pressure: far below the ring's classical buckling load
        // 3EI/R^3 = 50.4 kN/m (above it the linear response amplifies
        // the lozenge mode instead — see the buckling test below).
        let pressure = 8_000.0_f64; // N/m2
        let hoop = pressure * radius; // N of hoop compression per m
        let area = 0.01_f64;
        let inertia = 1.0e-5_f64;
        let e = 210.0_f64;
        for &(chords_per_quadrant, axial_tol) in &[(9_u32, 0.03_f64), (36, 0.002)] {
            let per_half = chords_per_quadrant * 2;
            let step = std::f64::consts::PI / per_half as f64;
            let mut model = FrameModel::default();
            // The open half arc: per_half chords over pi, per_half + 1
            // nodes (the cut ends at theta = 0 and pi).
            let (n0, n_last) = model
                .add_arc(
                    0.0,
                    0.0,
                    radius,
                    0.0,
                    std::f64::consts::PI,
                    per_half,
                    area,
                    inertia,
                    e,
                )
                .unwrap();
            // Radial nodal loads: ends carry half their tributary arc
            // (the mirrored half provides the rest), interior nodes the
            // full p R dtheta.
            let node_count = per_half + 1;
            for i in 0..node_count {
                let a = step * i as f64;
                let load = if i == 0 || i == node_count - 1 {
                    pressure * radius * step / 2.0
                } else {
                    pressure * radius * step
                };
                model.loads.push(FrameLoad {
                    node: n0 + i as usize,
                    fx: -load * a.cos(),
                    fz: -load * a.sin(),
                    moment_nm: 0.0,
                });
            }
            // Symmetry restraints on the cut planes and the crown: all
            // perpendicular to the uniform radial-shrinkage field.
            model.supports.push(FrameSupport {
                node: n0,
                fix_x: false,
                fix_z: true,
                fix_rot: true,
            });
            model.supports.push(FrameSupport {
                node: n_last,
                fix_x: false,
                fix_z: true,
                fix_rot: true,
            });
            let crown = n0 + chords_per_quadrant as usize;
            model.supports.push(FrameSupport {
                node: crown,
                fix_x: true,
                fix_z: false,
                fix_rot: false,
            });
            let sol = model.solve_dense().unwrap();
            for (i, (n_f, _v, m1, _m2)) in sol.member_forces.iter().enumerate() {
                eprintln!("dbg: m{i}: N {n_f:.3e} M {m1:.3e}");
            }
            let mut worst_axial = 0.0_f64;
            let mut worst_bending = 0.0_f64;
            for (n_f, _v, m1, m2) in &sol.member_forces {
                worst_axial = worst_axial.max((n_f.abs() - hoop).abs());
                worst_bending = worst_bending.max(m1.abs().max(m2.abs()));
            }
            assert!(
                worst_axial < axial_tol * hoop,
                "chords {per_half}: axial error {} vs pR {hoop}",
                worst_axial
            );
            assert!(
                worst_bending < 0.02 * hoop * radius,
                "chords {per_half}: bending {worst_bending} not small"
            );
        }
    }

    /// An inclined (45 deg) cantilever with a transverse tip load must
    /// deflect exactly P L^3 / 3EI — the transformation sanity check for
    /// non-horizontal members.
    #[test]
    fn inclined_cantilever_matches_the_closed_form() {
        let l = 2.0_f64;
        let mut model = FrameModel::default();
        model.nodes.push(FrameNode {
            position: (0.0, 0.0),
        });
        model.nodes.push(FrameNode {
            position: (l * 0.5f64.sqrt(), l * 0.5f64.sqrt()),
        });
        model.elements.push(FrameElement {
            nodes: [0, 1],
            area_m2: 0.01,
            inertia_m4: 1.0e-5,
            youngs_modulus_gpa: 210.0,
            density_kg_m3: 0.0,
        });
        model.supports.push(FrameSupport::fixed(0));
        // The load is transverse to the member (perpendicular, in -local z
        // direction: for a 45 deg member the local z maps to global
        // (-s, c)).
        let p = 50_000.0_f64;
        model.loads.push(FrameLoad {
            node: 1,
            fx: -p * 0.5f64.sqrt(),
            fz: p * 0.5f64.sqrt(),
            moment_nm: 0.0,
        });
        let sol = model.solve().unwrap();
        // Tip deflection along the load direction = P L^3 / 3EI.
        let expected = p * l.powi(3) / (3.0 * 2.1e11 * 1.0e-5);
        let (ux, uz, _t) = sol.displacements[1];
        let deflection = -ux * 0.5f64.sqrt() + uz * 0.5f64.sqrt();
        assert!(
            (deflection - expected).abs() < 1e-6 * expected,
            "tip {deflection} vs {expected}"
        );
    }

    const L: f64 = 4.0; // m
    const EI: f64 = 2.1e11 * 8.0e-5; // 210 GPa x 8e4 cm4
    const EA: f64 = 2.1e11 * 0.05;

    fn beam_nodes() -> Vec<FrameNode> {
        vec![
            FrameNode {
                position: (0.0, 0.0),
            },
            FrameNode { position: (L, 0.0) },
        ]
    }

    fn member(area: f64, inertia: f64) -> FrameElement {
        FrameElement {
            nodes: [0, 1],
            area_m2: area,
            inertia_m4: inertia,
            youngs_modulus_gpa: 210.0,
            density_kg_m3: 0.0,
        }
    }

    /// Cantilever with a tip point load: deflection P L^3 / 3EI, slope
    /// P L^2 / 2EI, root moment P L — one Euler-Bernoulli element is
    /// exact.
    #[test]
    fn cantilever_tip_load_matches_closed_form() {
        let p = 50_000.0_f64;
        let model = FrameModel {
            nodes: beam_nodes(),
            elements: vec![member(0.05, 8.0e-5)],
            supports: vec![FrameSupport::fixed(0)],
            loads: vec![FrameLoad {
                node: 1,
                fx: 0.0,
                fz: -p,
                moment_nm: 0.0,
            }],
            member_loads: vec![],
        };
        let sol = model.solve().expect("cantilever solves");
        let defl = -sol.displacements[1].1;
        let expected = p * L * L * L / (3.0 * EI);
        assert!(
            (defl - expected).abs() < 1e-6 * expected,
            "deflection {defl} vs {expected}"
        );
        let slope = -sol.displacements[1].2;
        let expected_slope = p * L * L / (2.0 * EI);
        assert!(
            (slope - expected_slope).abs() < 1e-6 * expected_slope,
            "slope {slope} vs {expected_slope}"
        );
        let (_n, _v, m1, _m2) = sol.member_forces[0];
        assert!((m1 - p * L).abs() < 1e-6 * (p * L), "root moment {m1}");
    }

    /// Simply supported with a midspan point load (two elements, load at
    /// the common node): midspan deflection P L^3 / 48 EI, and no axial
    /// force anywhere.
    #[test]
    fn simply_supported_midspan_load_matches_closed_form() {
        let p = 80_000.0_f64;
        let model = FrameModel {
            nodes: vec![
                FrameNode {
                    position: (0.0, 0.0),
                },
                FrameNode {
                    position: (L / 2.0, 0.0),
                },
                FrameNode { position: (L, 0.0) },
            ],
            elements: vec![
                FrameElement {
                    nodes: [0, 1],
                    area_m2: 0.05,
                    inertia_m4: 8.0e-5,
                    youngs_modulus_gpa: 210.0,
                    density_kg_m3: 0.0,
                },
                FrameElement {
                    nodes: [1, 2],
                    area_m2: 0.05,
                    inertia_m4: 8.0e-5,
                    youngs_modulus_gpa: 210.0,
                    density_kg_m3: 0.0,
                },
            ],
            supports: vec![FrameSupport::pinned(0), FrameSupport::roller_z(2)],
            loads: vec![FrameLoad {
                node: 1,
                fx: 0.0,
                fz: -p,
                moment_nm: 0.0,
            }],
            member_loads: vec![],
        };
        let sol = model.solve().expect("SSB solves");
        let defl = -sol.displacements[1].1;
        let expected = p * L * L * L / (48.0 * EI);
        assert!(
            (defl - expected).abs() < 1e-6 * expected,
            "midspan {defl} vs {expected}"
        );
        for (n, _v, _m1, _m2) in &sol.member_forces {
            assert!(n.abs() < 1e-6 * p, "axial {n} should be zero");
        }
    }

    /// Uniform load on a simply supported beam via the consistent load
    /// vector: end rotation w L^3 / 24 EI, and the recovered member end
    /// moments vanish at the simple supports (the fixed-end offset does
    /// its job).
    #[test]
    fn uniform_load_matches_closed_form() {
        let w = 30_000.0_f64; // N/m magnitude, applied downward
        let model = FrameModel {
            nodes: vec![
                FrameNode {
                    position: (0.0, 0.0),
                },
                FrameNode { position: (L, 0.0) },
            ],
            elements: vec![member(0.05, 8.0e-5)],
            supports: vec![FrameSupport::pinned(0), FrameSupport::roller_z(1)],
            loads: vec![],
            member_loads: vec![MemberLoad {
                element: 0,
                w_n_m: -w,
            }],
        };
        let sol = model.solve().expect("uniform beam solves");
        // A single element between the supports has no midspan DOF, so
        // check the end rotation against -w L^3 / 24 EI (a downward load
        // rotates the left end clockwise, negative with CCW positive).
        let expected_slope = -w * L * L * L / (24.0 * EI);
        let slope = sol.displacements[0].2;
        assert!(
            (slope - expected_slope).abs() < 1e-6 * expected_slope.abs(),
            "end slope {slope} vs {expected_slope}"
        );
        let (_n, _v, m1, m2) = sol.member_forces[0];
        assert!(m1.abs() < 1e-6 * w * L * L, "M1 {m1}");
        assert!(m2.abs() < 1e-6 * w * L * L, "M2 {m2}");
    }

    /// Axial behaviour carries over from the truss model: a bar with one
    /// fixed end stretches by P L / E A.
    #[test]
    fn axial_member_stretches() {
        let p = 200_000.0_f64;
        let model = FrameModel {
            nodes: beam_nodes(),
            elements: vec![member(0.05, 8.0e-5)],
            supports: vec![FrameSupport::fixed(0)],
            loads: vec![FrameLoad {
                node: 1,
                fx: p,
                fz: 0.0,
                moment_nm: 0.0,
            }],
            member_loads: vec![],
        };
        let sol = model.solve().expect("axial member solves");
        let expected = p * L / EA;
        assert!(
            (sol.displacements[1].0 - expected).abs() < 1e-6 * expected,
            "stretch {} vs {expected}",
            sol.displacements[1].0
        );
        assert!((sol.member_forces[0].0 - p).abs() < 1e-6 * p);
    }

    /// An inclined pair of very slender members meeting at an apex: with
    /// negligible bending stiffness the frame approaches the truss
    /// solution, apex drop P L / (2 EA sin^2 theta) with
    /// sin(theta) = 3/sqrt(13).
    #[test]
    fn inclined_apex_triangle_carries_axially() {
        let p = 100_000.0_f64;
        let model = FrameModel {
            nodes: vec![
                FrameNode {
                    position: (0.0, 0.0),
                },
                FrameNode {
                    position: (2.0, 3.0),
                },
                FrameNode {
                    position: (4.0, 0.0),
                },
            ],
            elements: vec![
                FrameElement {
                    nodes: [0, 1],
                    area_m2: 0.01,
                    inertia_m4: 1.0e-9,
                    youngs_modulus_gpa: 210.0,
                    density_kg_m3: 0.0,
                },
                FrameElement {
                    nodes: [1, 2],
                    area_m2: 0.01,
                    inertia_m4: 1.0e-9,
                    youngs_modulus_gpa: 210.0,
                    density_kg_m3: 0.0,
                },
            ],
            supports: vec![FrameSupport::pinned(0), FrameSupport::pinned(2)],
            loads: vec![FrameLoad {
                node: 1,
                fx: 0.0,
                fz: -p,
                moment_nm: 0.0,
            }],
            member_loads: vec![],
        };
        let sol = model.solve().expect("triangle solves");
        let leg = 13.0_f64.sqrt();
        let sin = 3.0 / leg;
        let expected = p * leg / (2.0 * 2.1e11 * 0.01 * sin * sin);
        let defl = -sol.displacements[1].1;
        assert!(
            (defl - expected).abs() < 0.05 * expected,
            "apex drop {defl} vs truss approximation {expected}"
        );
    }
}
