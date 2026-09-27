//! Minimal 3D truss finite-element solver.
//!
//! Direct stiffness method, dense solver, penalty boundary conditions —
//! deliberately small and dependency-free. It exists to give the construction
//! solvers an auditable load path; replace with `tpt-fem` when that substrate
//! ships (the [`TrussModel`] types map 1:1 onto bar/truss elements).
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
    /// A node or element index is out of range.
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
        let dof = 3 * n;
        for e in &self.elements {
            if e.nodes[0] >= n || e.nodes[1] >= n {
                return Err(FemError::OutOfRange);
            }
        }
        for l in &self.loads {
            if l.node >= n {
                return Err(FemError::OutOfRange);
            }
        }

        // Dense symmetric stiffness matrix (row-major).
        let mut k = vec![0.0f64; dof * dof];
        let mut f = vec![0.0f64; dof];

        let add = |i: usize, j: usize, v: f64, k: &mut Vec<f64>| {
            k[i * dof + j] += v;
        };

        for e in &self.elements {
            let (na, nb) = (self.nodes[e.nodes[0]], self.nodes[e.nodes[1]]);
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

        for load in &self.loads {
            f[3 * load.node] += load.force.x;
            f[3 * load.node + 1] += load.force.y;
            f[3 * load.node + 2] += load.force.z;
        }

        // Penalty boundary conditions.
        const PENALTY: f64 = 1e13;
        for s in &self.supports {
            if s.node >= n {
                return Err(FemError::OutOfRange);
            }
            for (fixed, axis) in [(s.fix_x, 0), (s.fix_y, 1), (s.fix_z, 2)] {
                if fixed {
                    let i = 3 * s.node + axis;
                    k[i * dof + i] += PENALTY;
                }
            }
        }

        // Gaussian elimination with partial pivoting.
        let mut a = k;
        let mut x = f;
        for col in 0..dof {
            // Pivot.
            let mut piv = col;
            let mut best = a[col * dof + col].abs();
            for r in col + 1..dof {
                let v = a[r * dof + col].abs();
                if v > best {
                    best = v;
                    piv = r;
                }
            }
            if best < 1e-9 {
                return Err(FemError::SingularSystem);
            }
            if piv != col {
                for c in 0..dof {
                    a.swap(col * dof + c, piv * dof + c);
                }
                x.swap(col, piv);
            }
            let inv = 1.0 / a[col * dof + col];
            for r in col + 1..dof {
                let factor = a[r * dof + col] * inv;
                if factor == 0.0 {
                    continue;
                }
                for c in col..dof {
                    a[r * dof + c] -= factor * a[col * dof + c];
                }
                x[r] -= factor * x[col];
            }
        }
        for r in (0..dof).rev() {
            let mut sum = x[r];
            for c in r + 1..dof {
                sum -= a[r * dof + c] * x[c];
            }
            x[r] = sum / a[r * dof + r];
        }

        let displacements: Vec<Vector3> = (0..n)
            .map(|i| Vector3::new(x[3 * i], x[3 * i + 1], x[3 * i + 2]))
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
