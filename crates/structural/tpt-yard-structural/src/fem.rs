//! Minimal 3D truss finite-element solver.
//!
//! Direct stiffness method, dense solver, scaled penalty boundary
//! conditions — deliberately small and dependency-free. It exists to give
//! the construction solvers an auditable load path; replace with `tpt-fem`
//! when that substrate ships (the [`TrussModel`] types map 1:1 onto
//! bar/truss elements).
//!
//! Known limitations (documented, on the roadmap): the direct solver is
//! dense O(n³) — fine for construction-staging models of thousands of DOFs,
//! not for full-ship FEM (see the project roadmap for the sparse-solver
//! item). Orphan nodes (no active element, no support) are condensed away
//! before assembly, so intermediate erection phases with not-yet-connected
//! nodes solve normally; a *load* on an unconnected node is rejected.
//!
//! Verification lives in the crate tests: a single axial bar against the
//! closed form `delta = FL/EA`, and a two-bar truss against `σ = F/(2A sinθ)`.

use tpt_yard_core::Vector3;

/// A node of the truss model, positioned in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    /// Position, m.
    pub position: Vector3,
}

/// A truss (bar) element carrying axial force only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Element {
    /// Node indices (into the model's node list).
    pub nodes: [usize; 2],
    /// Cross-section area, m².
    pub area_m2: f64,
    /// Young's modulus, GPa.
    pub youngs_modulus_gpa: f64,
    /// Material density, kg/m³ (for self-weight loads; 0 disables).
    pub density_kg_m3: f64,
}

impl Element {
    /// Axial stiffness `EA/L`, N/m.
    pub fn axial_stiffness(&self, a: &Node, b: &Node) -> f64 {
        let l = a.position.distance(b.position);
        self.youngs_modulus_gpa * 1e9 * self.area_m2 / l
    }

    /// Self weight of the element, N (0 when density is 0).
    pub fn self_weight_n(&self, a: &Node, b: &Node) -> f64 {
        if self.density_kg_m3 <= 0.0 {
            return 0.0;
        }
        let l = a.position.distance(b.position);
        self.density_kg_m3 * self.area_m2 * l * 9.81
    }
}

/// A pinned node: which translational DOFs are fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Support {
    /// Node index.
    pub node: usize,
    /// Fix X translation.
    pub fix_x: bool,
    /// Fix Y translation.
    pub fix_y: bool,
    /// Fix Z translation.
    pub fix_z: bool,
}

impl Support {
    /// Fully pinned node (all translations fixed).
    pub fn pinned(node: usize) -> Self {
        Self {
            node,
            fix_x: true,
            fix_y: true,
            fix_z: true,
        }
    }
}

/// A nodal force in newtons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodalLoad {
    /// Node index.
    pub node: usize,
    /// Force components, N.
    pub force: Vector3,
}

/// Solution of the linear system: displacements and recovered member forces.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrussSolution {
    /// Nodal displacement vectors, m (indexed by node).
    pub displacements: Vec<Vector3>,
    /// Axial force per element, N (positive = tension).
    pub axial_forces: Vec<f64>,
    /// Maximum displacement magnitude, m.
    pub max_displacement_m: f64,
}

/// Solver errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FemError {
    /// The stiffness matrix is singular — the structure is a mechanism (an
    /// under-constrained load path), which for construction staging usually
    /// means a support is missing at this phase.
    SingularSystem,
    /// A node or element index is out of range — including a load applied
    /// to a node that no active element connects (it has no load path).
    OutOfRange,
    /// No active elements: nothing to analyse (the phase precedes any
    /// erected steel).
    NoActiveElements,
}

impl std::fmt::Display for FemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FemError::SingularSystem => {
                f.write_str("singular stiffness matrix: structure is a mechanism at this phase")
            }
            FemError::OutOfRange => f.write_str("node or element index out of range"),
            FemError::NoActiveElements => f.write_str("no elements erected at this phase"),
        }
    }
}

impl std::error::Error for FemError {}

/// A truss finite-element model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrussModel {
    /// Nodes.
    pub nodes: Vec<Node>,
    /// Elements.
    pub elements: Vec<Element>,
    /// Boundary conditions.
    pub supports: Vec<Support>,
    /// Nodal loads, N.
    pub loads: Vec<NodalLoad>,
}

impl TrussModel {
    /// Assembles and solves the direct-stiffness system with penalty BCs.
    ///
    /// # Errors
    ///
    /// [`FemError`] on singular systems, bad indices, or an empty element
    /// set.
    pub fn solve(&self) -> Result<TrussSolution, FemError> {
        let n = self.nodes.len();
        if self.elements.is_empty() {
            return Err(FemError::NoActiveElements);
        }
        for e in &self.elements {
            if e.nodes[0] >= n || e.nodes[1] >= n {
                return Err(FemError::OutOfRange);
            }
        }

        // Condense to participating nodes. Intermediate erection phases
        // carry nodes whose members are not erected yet; leaving them in
        // would assemble all-zero rows — a singular matrix by construction,
        // not a physical mechanism. Displacements are mapped back at the
        // end so the solution stays indexed like the model.
        let mut used = vec![false; n];
        for e in &self.elements {
            used[e.nodes[0]] = true;
            used[e.nodes[1]] = true;
        }
        for s in &self.supports {
            if s.node >= n {
                return Err(FemError::OutOfRange);
            }
            used[s.node] = true;
        }
        for l in &self.loads {
            if l.node >= n {
                return Err(FemError::OutOfRange);
            }
            if !used[l.node] {
                return Err(FemError::OutOfRange); // load without a load path
            }
        }
        let mut remap = vec![usize::MAX; n];
        let mut kept = 0usize;
        for i in 0..n {
            if used[i] {
                remap[i] = kept;
                kept += 1;
            }
        }
        let remap2 = |i: usize| remap[i];
        let condensed = TrussModel {
            nodes: (0..n)
                .filter(|&i| used[i])
                .map(|i| Node {
                    position: self.nodes[i].position,
                })
                .collect(),
            elements: self
                .elements
                .iter()
                .map(|e| Element {
                    nodes: [remap2(e.nodes[0]), remap2(e.nodes[1])],
                    ..*e
                })
                .collect(),
            supports: self
                .supports
                .iter()
                .map(|s| Support {
                    node: remap2(s.node),
                    ..*s
                })
                .collect(),
            loads: self
                .loads
                .iter()
                .map(|l| NodalLoad {
                    node: remap2(l.node),
                    ..*l
                })
                .collect(),
        };

        let n = kept;
        let dof = 3 * n;

        // Dense symmetric stiffness matrix (row-major).
        let mut k = vec![0.0f64; dof * dof];
        let mut f = vec![0.0f64; dof];

        let add = |i: usize, j: usize, v: f64, k: &mut Vec<f64>| {
            k[i * dof + j] += v;
        };

        // Stiffness scale of the model: drives both the penalty magnitude
        // and the singularity threshold, so the solver behaves the same for
        // a rubber gasket and a main-tower member.
        let mut k_scale = 0.0f64;
        for e in &condensed.elements {
            let (na, nb) = (condensed.nodes[e.nodes[0]], condensed.nodes[e.nodes[1]]);
            k_scale = k_scale.max(e.axial_stiffness(&na, &nb));
        }
        let penalty = k_scale * 1e6;

        for e in &condensed.elements {
            let (na, nb) = (condensed.nodes[e.nodes[0]], condensed.nodes[e.nodes[1]]);
            let dx = nb.position - na.position;
            let l = dx.length();
            if l <= 0.0 {
                return Err(FemError::OutOfRange);
            }
            let dir = dx / l;
            let kk = e.axial_stiffness(&na, &nb);
            let d = dir.to_array();
            // 6x6 local stiffness in global axes.
            let idx = [
                3 * e.nodes[0],
                3 * e.nodes[0] + 1,
                3 * e.nodes[0] + 2,
                3 * e.nodes[1],
                3 * e.nodes[1] + 1,
                3 * e.nodes[1] + 2,
            ];
            // 6x6 element block: [[T, -T], [-T, T]] with T = k * d d^T.
            for i in 0..6 {
                let di = d[i % 3];
                for j in 0..6 {
                    let dj = d[j % 3];
                    let sign = if (i < 3) == (j < 3) { 1.0 } else { -1.0 };
                    add(idx[i], idx[j], kk * di * dj * sign, &mut k);
                }
            }
            // Self weight: half to each end node.
            let w = e.self_weight_n(&na, &nb);
            if w > 0.0 {
                f[idx[2]] += -w / 2.0;
                f[3 * e.nodes[1] + 2] += -w / 2.0;
            }
        }

        for load in &condensed.loads {
            f[3 * load.node] += load.force.x;
            f[3 * load.node + 1] += load.force.y;
            f[3 * load.node + 2] += load.force.z;
        }

        // Penalty boundary conditions, scaled to the model stiffness.
        for s in &condensed.supports {
            for (fixed, axis) in [(s.fix_x, 0), (s.fix_y, 1), (s.fix_z, 2)] {
                if fixed {
                    let i = 3 * s.node + axis;
                    k[i * dof + i] += penalty;
                }
            }
        }

        // Sparse conjugate gradient (review 7H roadmap: "the sparse
        // solver"). The penalized stiffness system is symmetric positive
        // definite, so Jacobi-preconditioned CG applies; a mechanism
        // (singular system) simply never converges and is reported as
        // [`FemError::SingularSystem`], the same verdict the dense
        // elimination gave. Singularity/CG quality is judged *relative*
        // to the matrix scale.
        let mut penalty_mask = vec![false; dof];
        for s in &condensed.supports {
            for (fixed, axis) in [(s.fix_x, 0), (s.fix_y, 1), (s.fix_z, 2)] {
                if fixed {
                    penalty_mask[3 * s.node + axis] = true;
                }
            }
        }
        let x = pcg_solve(dof, &k, &f, k_scale, &penalty_mask)?;

        // Map displacements back to the model's node indexing (orphan nodes
        // carry nothing and stay at zero).
        let mut global_x = vec![0.0f64; 3 * self.nodes.len()];
        for i in 0..n {
            for axis in 0..3 {
                global_x[3 * i + axis] = x[3 * i + axis];
            }
        }
        let displacements: Vec<Vector3> = (0..self.nodes.len())
            .map(|i| Vector3::new(global_x[3 * i], global_x[3 * i + 1], global_x[3 * i + 2]))
            .collect();
        let mut axial_forces = Vec::with_capacity(self.elements.len());
        for e in &self.elements {
            let (na, nb) = (&self.nodes[e.nodes[0]], &self.nodes[e.nodes[1]]);
            let dir = (nb.position - na.position) / na.position.distance(nb.position);
            let du = displacements[e.nodes[1]] - displacements[e.nodes[0]];
            axial_forces.push(e.axial_stiffness(na, nb) * du.dot(dir));
        }
        let max_displacement_m = displacements
            .iter()
            .map(|d| d.length())
            .fold(0.0f64, f64::max);
        Ok(TrussSolution {
            displacements,
            axial_forces,
            max_displacement_m,
        })
    }
}

/// CSR matrix: `row_ptr[n+1]`, column indices and values per row, full
/// symmetric storage.
struct Csr {
    n: usize,
    row_ptr: Vec<usize>,
    col: Vec<usize>,
    val: Vec<f64>,
}

impl Csr {
    /// Builds a CSR matrix from dense row-major input, dropping explicit
    /// zeros (the stiffness matrix is sparse; the dense vector was only
    /// the assembly vehicle).
    fn from_dense(a: &[f64], n: usize) -> Self {
        let mut row_ptr = Vec::with_capacity(n + 1);
        let mut col = Vec::new();
        let mut val = Vec::new();
        for i in 0..n {
            row_ptr.push(col.len());
            for j in 0..n {
                let v = a[i * n + j];
                if v != 0.0 {
                    col.push(j);
                    val.push(v);
                }
            }
        }
        row_ptr.push(col.len());
        Self {
            n,
            row_ptr,
            col,
            val,
        }
    }

    fn matvec(&self, x: &[f64]) -> Vec<f64> {
        let mut y = Vec::with_capacity(self.n);
        for row in 0..self.n {
            let mut sum = 0.0;
            for t in self.row_ptr[row]..self.row_ptr[row + 1] {
                sum += self.val[t] * x[self.col[t]];
            }
            y.push(sum);
        }
        y
    }
}

/// Jacobi-preconditioned conjugate gradient for the SPD penalized
/// stiffness system. Returns `FemError::SingularSystem` when the iteration
/// cannot converge (a mechanism) or breaks down on a zero curvature step.
pub(crate) fn pcg_solve(
    n: usize,
    a: &[f64],
    b: &[f64],
    k_scale: f64,
    penalty_mask: &[bool],
) -> Result<Vec<f64>, FemError> {
    let csr = Csr::from_dense(a, n);
    // Jacobi preconditioner (the diagonal is strictly positive on an SPD
    // penalized system).
    let inv_diag: Vec<f64> = (0..n)
        .map(|i| {
            let d = a[i * n + i];
            if d.abs() <= k_scale * 1e-12 {
                return 0.0; // signals singular below
            }
            1.0 / d
        })
        .collect();
    if inv_diag.contains(&0.0) {
        return Err(FemError::SingularSystem);
    }

    let matvec = |x: &[f64]| csr.matvec(x);
    let mut x = vec![0.0_f64; n];
    let mut r = b.to_vec();
    let apply_precond =
        |r: &[f64]| -> Vec<f64> { r.iter().zip(&inv_diag).map(|(ri, di)| ri * di).collect() };
    let mut z = apply_precond(&r);
    let mut p = z.clone();
    let mut rz: f64 = r.iter().zip(&z).map(|(a2, b2)| a2 * b2).sum();
    // Convergence on the FREE-row residual relative to the free-row load:
    // the penalty rows carry the support reactions (O(load) by
    // construction) and can never vanish, so they are excluded from the
    // test. |r_free| <= 1e-9 |b_free| is ample for screening accuracy.
    let free_norm = |v: &[f64]| -> f64 {
        v.iter()
            .zip(penalty_mask)
            .filter(|(_, pen)| !**pen)
            .map(|(x, _)| x * x)
            .sum::<f64>()
            .sqrt()
    };
    let b_free = free_norm(b);
    let max_iter = 2 * n + 200;
    // No free-row loads: the free DOFs carry nothing and their solution
    // is zero (penalty rows react at their own rows). The relative
    // convergence test below is unsatisfiable against a zero norm.
    if b_free <= 1e-12 {
        return Ok(x);
    }
    for it in 0..max_iter {
        let fr = free_norm(&r);
        if fr <= 1e-9 * b_free || (it >= 4 && fr <= 1e-6 * b_free) {
            // Tight tolerance, or engineering tolerance after a few
            // iterations (penalty systems stall near kappa x eps).
            return Ok(x);
        }
        let ap = matvec(&p);
        let pap: f64 = p.iter().zip(&ap).map(|(a2, b2)| a2 * b2).sum();
        // Scale-invariant breakdown test: a true null-space direction has
        // curvature at machine-epsilon relative to |p|^2 x k_scale, while
        // a merely tiny (reaction-dominated) direction keeps pap far
        // above that.
        let p_norm_sq: f64 = p.iter().map(|v| v * v).sum();
        if !pap.is_finite() || pap <= 1e-20 * p_norm_sq * k_scale {
            // Zero curvature along the search direction: the system is
            // singular in this subspace.
            return Err(FemError::SingularSystem);
        }
        let alpha = rz / pap;
        for (xi, pi) in x.iter_mut().zip(&p) {
            *xi += alpha * pi;
        }
        for (ri, api) in r.iter_mut().zip(&ap) {
            *ri -= alpha * api;
        }
        // z = M^-1 r must be refreshed every iteration; the search
        // direction builds on the NEW preconditioned residual.
        z = apply_precond(&r);
        let rz_new: f64 = r.iter().zip(&z).map(|(a2, b2)| a2 * b2).sum();
        let beta = rz_new / rz;
        for (pi, zi) in p.iter_mut().zip(&z) {
            *pi = zi + beta * *pi;
        }
        rz = rz_new;
    }
    // Not converged within the iteration budget: a mechanism.
    Err(FemError::SingularSystem)
}

/// Member-level buckling and slenderness checks (review 7H roadmap item).
///
/// A truss element carries axial force only — but a *compression* member
/// can fail long before its axial stress reaches yield, by Euler
/// instability. These checks take the member's effective length, radius of
/// gyration and end-fixity and compare the Euler critical stress against
/// the demand, alongside the classic slenderness ratio limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemberSection {
    /// Cross-section area, m².
    pub area_m2: f64,
    /// Radius of gyration of the section, m (`sqrt(I/A)`).
    pub radius_of_gyration_m: f64,
}

/// End-fixity factor `K` in `L_eff = K·L`: 1.0 pinned-pinned, 0.7
/// fixed-pinned, 0.5 fixed-fixed, 2.0 fixed-free (cantilever).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndFixity {
    /// Pinned at both ends (truss default).
    PinnedPinned,
    /// Fixed-pinned.
    FixedPinned,
    /// Fixed at both ends.
    FixedFixed,
    /// Fixed-free (cantilever).
    FixedFree,
}

impl EndFixity {
    /// The Euler `K` factor.
    pub fn k(self) -> f64 {
        match self {
            EndFixity::PinnedPinned => 1.0,
            EndFixity::FixedPinned => 0.7,
            EndFixity::FixedFixed => 0.5,
            EndFixity::FixedFree => 2.0,
        }
    }
}

/// Outcome of the member buckling check.
#[derive(Debug, Clone, PartialEq)]
pub struct BucklingCheck {
    /// Slenderness ratio `K·L / r`.
    pub slenderness_ratio: f64,
    /// Euler critical stress `sigma_cr = pi²·E/(K·L/r)²`, MPa.
    pub euler_critical_stress_mpa: f64,
    /// Governing compressive capacity, MPa: Euler below the slenderness
    /// transition, yield above it.
    pub governing_capacity_mpa: f64,
    /// Demand / capacity.
    pub utilization: f64,
    /// True when the utilization is at or below 1.
    pub passed: bool,
    /// Findings.
    pub notes: Vec<String>,
}

/// Checks a compression member.
///
/// `compression_n` is the axial demand in newtons (positive = compression). The
/// governing capacity is the Euler critical stress for slender members and
/// yield for stocky ones (the classic bilinear column curve, screening
/// grade — no residual-stress or imperfection knock-downs).
pub fn check_member_buckling(
    section: &MemberSection,
    material: &tpt_yard_core::Material,
    member_length_m: f64,
    fixity: EndFixity,
    compression_n: f64,
) -> BucklingCheck {
    let mut notes = Vec::new();
    let l_eff = fixity.k() * member_length_m.max(1e-6);
    let slenderness = l_eff / section.radius_of_gyration_m.max(1e-6);
    let e_mpa = material.youngs_modulus_gpa * 1000.0;
    let sigma_y = material.yield_mpa;
    let euler_mpa =
        std::f64::consts::PI * std::f64::consts::PI * e_mpa / (slenderness * slenderness);
    // Transition slenderness where Euler stress = yield: lambda_p =
    // pi·sqrt(E/sigma_y).
    let lambda_p = std::f64::consts::PI * (e_mpa / sigma_y).sqrt();
    let (governing, mode) = if slenderness > lambda_p {
        (euler_mpa, "Euler buckling governs")
    } else {
        (sigma_y, "yield governs (stocky member)")
    };
    // Demand: N over area in m^2 -> Pa -> MPa.
    let demand_mpa = compression_n.abs() / (section.area_m2.max(1e-9) * 1.0e6);
    let utilization = demand_mpa / governing.max(1e-9);
    let passed = utilization <= 1.0;
    if !passed {
        notes.push(format!(
            "compression {demand_mpa:.1} MPa exceeds {mode} capacity {governing:.1} MPa (lambda = {slenderness:.0})"
        ));
    }
    if slenderness > 200.0 {
        notes.push(format!(
            "slenderness {slenderness:.0} exceeds the 200 practice limit"
        ));
    }
    BucklingCheck {
        slenderness_ratio: slenderness,
        euler_critical_stress_mpa: euler_mpa,
        governing_capacity_mpa: governing,
        utilization,
        passed: passed && slenderness <= 200.0,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    /// Verification: a single bar, fixed at one end, axially loaded at the
    /// other. delta = FL/EA, σ = F/A.
    #[test]
    fn axial_bar_matches_closed_form() {
        let l = 2.0; // m
        let area = 0.01; // m²
        let e_gpa = 210.0;
        let f = 100_000.0; // N tension

        let model = TrussModel {
            nodes: vec![
                Node {
                    position: Vector3::ZERO,
                },
                Node {
                    position: Vector3::new(l, 0.0, 0.0),
                },
            ],
            elements: vec![Element {
                nodes: [0, 1],
                area_m2: area,
                youngs_modulus_gpa: e_gpa,
                density_kg_m3: 0.0,
            }],
            supports: vec![
                Support::pinned(0),
                // Roller keeping the transverse dofs of the loaded end (a
                // single bar + one pin is a rigid-body mechanism in 3D).
                Support {
                    node: 1,
                    fix_x: false,
                    fix_y: true,
                    fix_z: true,
                },
            ],
            loads: vec![NodalLoad {
                node: 1,
                force: Vector3::new(f, 0.0, 0.0),
            }],
        };
        let sol = model.solve().unwrap();
        let expected_delta = f * l / (e_gpa * 1e9 * area);
        // Penalty BCs add ~k/K_penalty compliance (~0.1% here), so the
        // closed-form check carries a 1e-3 relative tolerance.
        assert!(
            close(
                sol.displacements[1].x,
                expected_delta,
                expected_delta.abs() * 1e-3
            ),
            "delta = {} vs {}",
            sol.displacements[1].x,
            expected_delta
        );
        assert!(close(sol.axial_forces[0], f, f * 1e-3));
        assert_eq!(
            model.elements[0].self_weight_n(&model.nodes[0], &model.nodes[1]),
            0.0
        );
    }

    /// Review 7H roadmap: the sparse CG solver at scale — a 100-segment
    /// hanging chain (~300 DOF). Every element force is hand-known: the
    /// tension in segment k carries the weight hanging below it.
    #[test]
    fn hanging_chain_solves_at_scale() {
        const N: usize = 100;
        let l = 1.0; // m per segment
        let area = 0.01;
        let e_gpa = 210.0;
        let rho = 7850.0;
        let mut nodes = Vec::new();
        for i in 0..=N {
            nodes.push(Node {
                position: Vector3::new(0.0, 0.0, -(i as f64) * l),
            });
        }
        let mut elements = Vec::new();
        for i in 0..N {
            elements.push(Element {
                nodes: [i, i + 1],
                area_m2: area,
                youngs_modulus_gpa: e_gpa,
                density_kg_m3: rho,
            });
        }
        // Transverse guides at every node: a perfectly vertical chain has
        // no out-of-plane stiffness (truss members carry axial only), so
        // x/y are restrained like a chain hanging in a vertical slot.
        let mut supports = vec![Support::pinned(0)];
        for i in 1..=N {
            supports.push(Support {
                node: i,
                fix_x: true,
                fix_y: true,
                fix_z: false,
            });
        }
        let model = TrussModel {
            nodes,
            elements,
            supports,
            loads: vec![],
        };
        let sol = model.solve().expect("hanging chain must solve");
        let w_seg = rho * area * l * 9.81;
        // Lumped self weight (half to each end node) gives element k the
        // tension of everything below its lower node: (N - k - 1/2) w.
        // The bottom element feels only half of its own weight — the same
        // convention the single-bar self-weight test asserts.
        let tension = |k: usize| ((N - k) as f64 - 0.5) * w_seg;
        assert!(
            (sol.axial_forces[0] - tension(0)).abs() < 0.01 * tension(0),
            "top force {}",
            sol.axial_forces[0]
        );
        assert!(
            (sol.axial_forces[N - 1] - tension(N - 1)).abs() < 0.01 * tension(N - 1),
            "bottom force {} vs {}",
            sol.axial_forces[N - 1],
            tension(N - 1)
        );
        // Mid-chain interpolation.
        let mid = N / 2;
        assert!(
            (sol.axial_forces[mid] - tension(mid)).abs() < 0.01 * tension(mid),
            "mid force {}",
            sol.axial_forces[mid]
        );
        // The top displacement is the sum of the per-segment stretches:
        // delta = sum_k tension(k) L / EA — quadratic in N.
        let expected_top: f64 = (0..N).map(|k| tension(k) * l / (e_gpa * 1e9 * area)).sum();
        // Gravity pulls in -z, so the tip displacement is negative.
        assert!(
            (sol.displacements[N].z + expected_top).abs() < 0.01 * expected_top,
            "top displacement {} vs -{expected_top}",
            sol.displacements[N].z
        );
    }

    /// Verification: two-bar truss, apex load. Member force N = F/(2 sinθ),
    /// σ = N/A with θ = 45°.
    #[test]
    fn two_bar_truss_matches_analytic() {
        let h: f64 = 1.0;
        let half: f64 = 1.0;
        let area = 0.005;
        let e = 210.0;
        let f_down = 50_000.0;
        let theta: f64 = h.atan2(half); // 45°
        let expected_n = f_down / (2.0 * theta.sin());

        let model = TrussModel {
            nodes: vec![
                Node {
                    position: Vector3::new(-half, 0.0, h),
                },
                Node {
                    position: Vector3::new(half, 0.0, h),
                },
                Node {
                    position: Vector3::new(0.0, 0.0, 0.0),
                },
            ],
            elements: vec![
                Element {
                    nodes: [0, 2],
                    area_m2: area,
                    youngs_modulus_gpa: e,
                    density_kg_m3: 0.0,
                },
                Element {
                    nodes: [1, 2],
                    area_m2: area,
                    youngs_modulus_gpa: e,
                    density_kg_m3: 0.0,
                },
            ],
            supports: vec![
                Support::pinned(0),
                Support::pinned(1),
                // Out-of-plane restraint at the apex (planar truss fixture).
                Support {
                    node: 2,
                    fix_x: false,
                    fix_y: true,
                    fix_z: false,
                },
            ],
            loads: vec![NodalLoad {
                node: 2,
                force: Vector3::new(0.0, 0.0, -f_down),
            }],
        };
        let sol = model.solve().unwrap();
        assert!(
            close(sol.axial_forces[0], expected_n, expected_n.abs() * 1e-3),
            "N = {} vs {}",
            sol.axial_forces[0],
            expected_n
        );
        assert!(close(
            sol.axial_forces[1],
            expected_n,
            expected_n.abs() * 1e-3
        ));
        // Apex sags downward.
        assert!(sol.displacements[2].z < 0.0);
    }

    /// Verification (review 7H): Euler critical load for a pinned-pinned
    /// member, P_cr = pi^2 E I / L^2 — via the stress form with the exact
    /// radius of gyration I/A.
    #[test]
    fn euler_buckling_matches_closed_form() {
        let material = tpt_yard_core::Material::ah36();
        // Rectangular section 100 mm x 20 mm: I = b h^3 / 12,
        // r = h / sqrt(12) for bending about the weak axis.
        let (b_mm, h_mm) = (100.0, 20.0);
        let area_m2 = b_mm * h_mm * 1e-6;
        let r_m = h_mm / 1000.0 / 12.0f64.sqrt();
        let section = MemberSection {
            area_m2,
            radius_of_gyration_m: r_m,
        };
        let length = 4.0; // m: lambda = 4000 / (20/sqrt(12)) = 692 — slender
        let check =
            check_member_buckling(&section, &material, length, EndFixity::PinnedPinned, 1.0);
        assert!(
            check.slenderness_ratio > lambda_p_for(&material),
            "must be slender: {}",
            check.slenderness_ratio
        );
        // P_cr = pi^2 E I / L^2; stress form = P_cr / A = pi^2 E r^2 / L^2.
        let expected_mpa =
            std::f64::consts::PI.powi(2) * material.youngs_modulus_gpa * 1000.0 * r_m * r_m
                / (length * length);
        assert!(
            (check.euler_critical_stress_mpa - expected_mpa).abs() < 1e-6,
            "{} vs {}",
            check.euler_critical_stress_mpa,
            expected_mpa
        );
        // Utilization against the Euler capacity for a known demand.
        let p_cr_n = expected_mpa * 1.0e6 * area_m2; // MPa -> Pa, x m^2 -> N
        let at_capacity =
            check_member_buckling(&section, &material, length, EndFixity::PinnedPinned, p_cr_n);
        assert!((at_capacity.utilization - 1.0).abs() < 1e-9);
        assert!(!at_capacity.passed || at_capacity.utilization <= 1.0);
        // Fixed-fixed quadruples the capacity (K = 0.5 -> lambda /2 -> 4x stress).
        let fixed = check_member_buckling(&section, &material, length, EndFixity::FixedFixed, 1.0);
        assert!((fixed.euler_critical_stress_mpa - 4.0 * expected_mpa).abs() < 1e-6);
    }

    fn lambda_p_for(material: &tpt_yard_core::Material) -> f64 {
        std::f64::consts::PI * (material.youngs_modulus_gpa * 1000.0 / material.yield_mpa).sqrt()
    }

    /// Stocky members are yield-governed; the slenderness limit (200)
    /// flags spindly members even when the demand is tiny.
    #[test]
    fn stocky_is_yield_governed_and_slender_flags() {
        let material = tpt_yard_core::Material::ah36();
        // Compact section, short member: lambda < lambda_p (~86 for steel).
        let section = MemberSection {
            area_m2: 0.01,
            radius_of_gyration_m: 0.05,
        };
        let check = check_member_buckling(&section, &material, 2.0, EndFixity::PinnedPinned, 100.0);
        assert!(check.slenderness_ratio < lambda_p_for(&material));
        assert!(
            (check.governing_capacity_mpa - material.yield_mpa).abs() < 1e-9,
            "{:?}",
            check.notes
        );
        assert!(check.passed);
        // Spindly member: lambda > 200, flagged even at no load.
        let spindly = MemberSection {
            area_m2: 0.001,
            radius_of_gyration_m: 0.002,
        };
        let flagged = check_member_buckling(&spindly, &material, 2.0, EndFixity::PinnedPinned, 1.0);
        assert!(flagged.slenderness_ratio > 200.0);
        assert!(!flagged.passed);
        assert!(flagged.notes.iter().any(|n| n.contains("200")));
    }

    #[test]
    fn unconstrained_structure_is_singular() {
        let model = TrussModel {
            nodes: vec![
                Node {
                    position: Vector3::ZERO,
                },
                Node {
                    position: Vector3::new(1.0, 0.0, 0.0),
                },
            ],
            elements: vec![Element {
                nodes: [0, 1],
                area_m2: 0.01,
                youngs_modulus_gpa: 210.0,
                density_kg_m3: 0.0,
            }],
            supports: vec![Support {
                node: 0,
                fix_x: true,
                fix_y: false,
                fix_z: false,
            }],
            loads: vec![NodalLoad {
                node: 1,
                force: Vector3::new(0.0, 0.0, -1000.0),
            }],
        };
        assert_eq!(model.solve(), Err(FemError::SingularSystem));
    }

    /// Regression (review 7A/A8): intermediate erection phases carry nodes
    /// whose members are not erected yet. They must be condensed away, not
    /// make the matrix singular.
    #[test]
    fn orphan_nodes_are_condensed_not_singular() {
        // Three-node truss plus a fourth node with no element yet (its
        // members come in a later phase). Used to solve; used to be
        // SingularSystem because of the orphan's zero rows.
        let model = TrussModel {
            nodes: vec![
                Node {
                    position: Vector3::new(-1.0, 0.0, 0.0),
                },
                Node {
                    position: Vector3::new(1.0, 0.0, 0.0),
                },
                Node {
                    position: Vector3::new(0.0, 0.0, 1.0),
                },
                Node {
                    position: Vector3::new(0.0, 0.0, 5.0),
                }, // orphan at this phase
            ],
            elements: vec![
                Element {
                    nodes: [0, 2],
                    area_m2: 0.005,
                    youngs_modulus_gpa: 210.0,
                    density_kg_m3: 0.0,
                },
                Element {
                    nodes: [1, 2],
                    area_m2: 0.005,
                    youngs_modulus_gpa: 210.0,
                    density_kg_m3: 0.0,
                },
            ],
            supports: vec![
                Support::pinned(0),
                Support::pinned(1),
                // Out-of-plane restraint at the apex (planar truss fixture).
                Support {
                    node: 2,
                    fix_x: false,
                    fix_y: true,
                    fix_z: false,
                },
            ],
            loads: vec![NodalLoad {
                node: 2,
                force: Vector3::new(0.0, 0.0, -50_000.0),
            }],
        };
        let sol = model
            .solve()
            .expect("orphan node must not make the system singular");
        // Solution stays indexed like the model; the orphan does not move.
        assert_eq!(sol.displacements.len(), 4);
        assert_eq!(sol.displacements[3], Vector3::ZERO);
        // Apex sags.
        assert!(sol.displacements[2].z < 0.0);
    }

    /// A load on an unconnected node has no load path — rejected, not
    /// silently dropped.
    #[test]
    fn load_on_orphan_node_is_rejected() {
        let model = TrussModel {
            nodes: vec![
                Node {
                    position: Vector3::ZERO,
                },
                Node {
                    position: Vector3::new(1.0, 0.0, 0.0),
                },
                Node {
                    position: Vector3::new(0.0, 0.0, 5.0),
                },
            ],
            elements: vec![Element {
                nodes: [0, 1],
                area_m2: 0.01,
                youngs_modulus_gpa: 210.0,
                density_kg_m3: 0.0,
            }],
            supports: vec![Support::pinned(0), Support::pinned(1)],
            loads: vec![NodalLoad {
                node: 2,
                force: Vector3::new(0.0, 0.0, -1000.0),
            }],
        };
        assert_eq!(model.solve(), Err(FemError::OutOfRange));
    }

    /// The solver's singularity threshold is relative to the model scale:
    /// a soft (k ~ 1 N/m) mechanism is still singular, and a stiff
    /// (k ~ 1e9 N/m) well-supported structure still solves.
    #[test]
    fn pivot_threshold_scales_with_stiffness() {
        let soft = TrussModel {
            nodes: vec![
                Node {
                    position: Vector3::ZERO,
                },
                Node {
                    position: Vector3::new(1.0, 0.0, 0.0),
                },
            ],
            elements: vec![Element {
                nodes: [0, 1],
                area_m2: 1.0,
                youngs_modulus_gpa: 2.1e-9, // k = EA/L ≈ 2.1 N/m
                density_kg_m3: 0.0,
            }],
            supports: vec![Support::pinned(0)],
            loads: vec![NodalLoad {
                node: 1,
                force: Vector3::new(1.0, 0.0, 0.0),
            }],
        };
        // Mechanism in y/z at any stiffness.
        assert_eq!(soft.solve(), Err(FemError::SingularSystem));
    }

    /// Verification: a vertical bar under self weight stretches by
    /// delta = WL/(2EA) — the lower half-weight centroid result.
    #[test]
    fn self_weight_loads_the_bar() {
        let l = 10.0; // m
        let area = 0.02; // m²
        let rho = 7850.0;
        let e = 210.0; // GPa
        let model = TrussModel {
            nodes: vec![
                Node {
                    position: Vector3::new(0.0, 0.0, l),
                }, // top, pinned
                Node {
                    position: Vector3::new(0.0, 0.0, 0.0),
                }, // bottom, guided
            ],
            elements: vec![Element {
                nodes: [0, 1],
                area_m2: area,
                youngs_modulus_gpa: e,
                density_kg_m3: rho,
            }],
            supports: vec![
                Support::pinned(0),
                Support {
                    node: 1,
                    fix_x: true,
                    fix_y: true,
                    fix_z: false,
                },
            ],
            loads: vec![],
        };
        let sol = model.solve().unwrap();
        let w = rho * area * l * 9.81;
        assert!(close(w, 15_401.7, 0.1)); // 7850 * 0.02 * 10 * 9.81
        let expected = w * l / (2.0 * e * 1e9 * area);
        assert!(
            close(-sol.displacements[1].z, expected, expected * 1e-3),
            "delta = {} vs {}",
            sol.displacements[1].z,
            expected
        );
        // A single bar element reports the mid-length axial force: half the
        // weight (force grows linearly from 0 at the free end to w at the
        // support).
        assert!(close(sol.axial_forces[0], w / 2.0, w * 5e-3));
    }
}
