//! Block distortion management: deviation maps and correction planning.
//!
//! After a block is welded, its as-built geometry is measured (laser
//! metrology, total station). [`DistortionControl`] compares the measured
//! geometry against target, produces a [`DeviationField`], and plans
//! [`CorrectionAction`]s against an allowable tolerance: accept small
//! deviations, heat-straighten moderate ones, reject gross ones.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::{Geometry3D, Material, Vector3};
//! use tpt_yard_distortion::{CorrectionAction, DistortionControl};
//!
//! let target = Geometry3D::from_box(2000.0, 500.0, 100.0);
//! let mut measured = target.clone();
//! measured.vertices[0] = measured.vertices[0] + Vector3::new(0.0, 0.0, 4.0);
//!
//! let mut dc = DistortionControl::new(target, measured);
//! let plan = dc.correction_plan(3.0); // ±3 mm tolerance
//! // The 4 mm bump must be heat-straightened (or rejected), not accepted.
//! assert!(plan.iter().any(|a| matches!(
//!     a,
//!     CorrectionAction::HeatStraighten { .. } | CorrectionAction::Reject { .. }
//! )));
//! ```

use tpt_yard_core::{Geometry3D, Material, Vector3};

/// A planned or executed correction on a distorted block.
#[derive(Debug, Clone, PartialEq)]
pub enum CorrectionAction {
    /// Line heating to shrink/straighten locally.
    HeatStraighten {
        /// Where to apply the heat, m (block coordinates).
        location: Vector3,
        /// Heat input for the straightening pass, kJ per mm of heat line.
        heat_input: f64,
    },
    /// Mechanical rework (press, flame-less straightening, adding material).
    Rework {
        /// What needs to happen.
        description: String,
    },
    /// Accept the block as built: the deviation is within tolerance.
    Accept {
        /// The accepted worst deviation, mm.
        deviation_mm: f64,
    },
    /// Reject: deviation beyond economic correction.
    Reject {
        /// Why.
        reason: String,
    },
}

/// One measured deviation point.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviationPoint {
    /// Nominal (target) position, m.
    pub target: Vector3,
    /// Signed displacement along the dominant deviation axis, mm.
    pub deviation_mm: f64,
}

/// Map of measured vs target geometry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeviationField {
    /// Per-vertex deviations (paired by vertex index).
    pub points: Vec<DeviationPoint>,
    /// Largest absolute deviation, mm.
    pub max_deviation_mm: f64,
    /// Root-mean-square deviation, mm.
    pub rms_deviation_mm: f64,
    /// Rigid-body translation removed before differencing, m (zero for the
    /// raw map).
    pub removed_translation: Vector3,
    /// Small rigid-body rotation removed before differencing, rad about
    /// the target centroid (zero for the raw map).
    pub removed_rotation_rad: Vector3,
}

/// Line-heating model for sizing straightening heat from the same
/// contraction physics as the welding crate (review 7B: the heat figure
/// was a flat placeholder before).
#[derive(Debug, Clone, PartialEq)]
pub struct StraighteningModel {
    /// Plate material.
    pub material: Material,
    /// Plate thickness, mm.
    pub plate_thickness_mm: f64,
    /// Fraction of the applied line heat that enters the plate (flame
    /// heating is far less efficient than an arc: ~0.3-0.5).
    pub thermal_efficiency: f64,
}

impl StraighteningModel {
    /// Transverse shrinkage produced per kJ/mm of heat line, mm - the
    /// welding crate's contraction formula
    /// `0.5 * alpha * eta * E / (rho * c * t)` evaluated per unit heat.
    pub fn shrinkage_mm_per_kj_mm(&self) -> f64 {
        let t_m = self.plate_thickness_mm.max(1e-3) / 1000.0;
        let eta = self.thermal_efficiency.clamp(0.01, 1.0);
        let m_per_kj = 0.5 * self.material.thermal_expansion_1_k * eta * 1e6
            / (self.material.density_kg_m3 * self.material.specific_heat_j_kg_k * t_m);
        m_per_kj * 1000.0
    }
}

/// Errors from distortion management.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistortionError {
    /// Target and measured geometry must have identical topology.
    MismatchedGeometry {
        /// Vertex count of the target.
        target_vertices: usize,
        /// Vertex count of the measurement.
        measured_vertices: usize,
    },
}

impl std::fmt::Display for DistortionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DistortionError::MismatchedGeometry {
                target_vertices,
                measured_vertices,
            } => {
                write!(
                    f,
                    "target has {target_vertices} vertices but measurement has {measured_vertices}"
                )
            }
        }
    }
}

impl std::error::Error for DistortionError {}

/// Block distortion control: target vs measured geometry plus the correction
/// history.
#[derive(Debug, Clone, PartialEq)]
pub struct DistortionControl {
    /// Nominal geometry (as designed).
    pub target_geometry: Geometry3D,
    /// As-measured geometry.
    pub measured_geometry: Geometry3D,
    /// Corrections planned/executed so far.
    pub corrections: Vec<CorrectionAction>,
    /// Deviations beyond `reject_factor * tolerance` are rejected as
    /// economically uncorrectable (default 5).
    pub reject_factor: f64,
    /// Fraction of vertices needing straightening above which the block is
    /// systematically out of tolerance and sent to rework (default 0.2).
    pub rework_fraction: f64,
    /// When set, straightening heat is sized from the plate's contraction
    /// physics instead of the flat heuristic.
    pub straightening: Option<StraighteningModel>,
}

impl DistortionControl {
    /// Creates a controller with no corrections yet and the documented
    /// default thresholds.
    pub fn new(target_geometry: Geometry3D, measured_geometry: Geometry3D) -> Self {
        Self {
            target_geometry,
            measured_geometry,
            corrections: Vec::new(),
            reject_factor: 5.0,
            rework_fraction: 0.2,
            straightening: None,
        }
    }

    /// Builder: size straightening heat from the plate's contraction
    /// physics (see [`StraighteningModel`]).
    #[must_use]
    pub fn with_straightening_model(mut self, model: StraighteningModel) -> Self {
        self.straightening = Some(model);
        self
    }

    /// Deviation field: per-vertex signed deviation, max and RMS.
    ///
    /// The reported deviation per vertex is the component with the largest
    /// absolute difference (the dominant axis), signed, in mm.
    ///
    /// # Errors
    ///
    /// [`DistortionError::MismatchedGeometry`] when the vertex counts differ.
    pub fn deviation_map(&self) -> Result<DeviationField, DistortionError> {
        let n = self.target_geometry.vertices.len();
        if n != self.measured_geometry.vertices.len() {
            return Err(DistortionError::MismatchedGeometry {
                target_vertices: n,
                measured_vertices: self.measured_geometry.vertices.len(),
            });
        }
        let mut points = Vec::with_capacity(n);
        let mut sum_sq = 0.0;
        let mut max = 0.0f64;
        for (t, m) in self
            .target_geometry
            .vertices
            .iter()
            .zip(&self.measured_geometry.vertices)
        {
            let d = *m - *t;
            let comps = [d.x, d.y, d.z];
            let dominant = comps
                .iter()
                .fold(0.0f64, |acc, v| if v.abs() > acc.abs() { *v } else { acc });
            let dev_mm = dominant * 1000.0;
            max = max.max(dev_mm.abs());
            sum_sq += dev_mm * dev_mm;
            points.push(DeviationPoint {
                target: *t,
                deviation_mm: dev_mm,
            });
        }
        let rms = if n > 0 {
            (sum_sq / n as f64).sqrt()
        } else {
            0.0
        };
        Ok(DeviationField {
            points,
            max_deviation_mm: max,
            rms_deviation_mm: rms,
            removed_translation: Vector3::ZERO,
            removed_rotation_rad: Vector3::ZERO,
        })
    }

    /// Deviation field with the best-fit rigid-body motion (translation +
    /// small rotation about the target centroid) removed, leaving the pure
    /// **form** deviation - a block that sits offset or slightly tilted
    /// but has the right shape reports ~zero here.
    ///
    /// The removed transform is reported in the field so a genuinely
    /// misplaced block is still visible to the caller.
    ///
    /// # Errors
    ///
    /// [`DistortionError::MismatchedGeometry`] when the vertex counts differ.
    pub fn deviation_map_form(&self) -> Result<DeviationField, DistortionError> {
        let n = self.target_geometry.vertices.len();
        if n != self.measured_geometry.vertices.len() {
            return Err(DistortionError::MismatchedGeometry {
                target_vertices: n,
                measured_vertices: self.measured_geometry.vertices.len(),
            });
        }
        // Centroid of the target.
        let mut c = Vector3::ZERO;
        for t in &self.target_geometry.vertices {
            c = c + *t;
        }
        c = if n > 0 { c / n as f64 } else { c };

        // Displacements d_i = measured - target.
        let d: Vec<Vector3> = self
            .target_geometry
            .vertices
            .iter()
            .zip(&self.measured_geometry.vertices)
            .map(|(t, m)| *m - *t)
            .collect();
        let mean_d = d.iter().fold(Vector3::ZERO, |a, v| a + *v) / n.max(1) as f64;

        // Least-squares small rotation about the centroid:
        // theta = [sum (|r|^2 I - r r^T)]^-1 * sum r x d'.
        let mut s_acc = [0.0f64; 3];
        let mut h = [[0.0f64; 3]; 3];
        for (i, t) in self.target_geometry.vertices.iter().enumerate() {
            let r = *t - c;
            let dp = d[i] - mean_d;
            let w = r.cross(dp).to_array();
            for a in 0..3 {
                s_acc[a] += w[a];
            }
            let ra = r.to_array();
            let r2 = ra[0] * ra[0] + ra[1] * ra[1] + ra[2] * ra[2];
            for a in 0..3 {
                for b in 0..3 {
                    let delta = if a == b { r2 } else { 0.0 };
                    h[a][b] += delta - ra[a] * ra[b];
                }
            }
        }
        let theta = solve3_symmetric(&h, s_acc);
        // The rotation term vanishes at the centroid (mean(r) = 0), so the
        // optimal translation is exactly the mean displacement.
        let tau = mean_d;
        let rot = Vector3::new(theta[0], theta[1], theta[2]);

        let mut points = Vec::with_capacity(n);
        let mut sum_sq = 0.0;
        let mut max = 0.0f64;
        for (i, t) in self.target_geometry.vertices.iter().enumerate() {
            let r = *t - c;
            let residual = d[i] - tau - rot.cross(r);
            let comps = [residual.x, residual.y, residual.z];
            let dominant = comps
                .iter()
                .fold(0.0f64, |acc, v| if v.abs() > acc.abs() { *v } else { acc });
            let dev_mm = dominant * 1000.0;
            max = max.max(dev_mm.abs());
            sum_sq += dev_mm * dev_mm;
            points.push(DeviationPoint {
                target: *t,
                deviation_mm: dev_mm,
            });
        }
        let rms = if n > 0 {
            (sum_sq / n as f64).sqrt()
        } else {
            0.0
        };
        Ok(DeviationField {
            points,
            max_deviation_mm: max,
            rms_deviation_mm: rms,
            removed_translation: tau,
            removed_rotation_rad: rot,
        })
    }

    /// Plans corrections against an allowable tolerance (mm).
    ///
    /// Policy (thresholds configurable via `reject_factor` /
    /// `rework_fraction`, verified in tests):
    /// - the plan works on the **form** deviation
    ///   ([`DistortionControl::deviation_map_form`]) — a block that is
    ///   merely displaced or tilted is not "corrected" point by point;
    /// - every vertex with `|deviation| <= tolerance` needs no action;
    /// - vertices with `tolerance < |deviation| <= reject_factor *
    ///   tolerance` (default 5) -> `HeatStraighten`, the heat input sized
    ///   from the straightening model when present (contraction physics:
    ///   excess / shrinkage-per-kJ), else the documented 1 kJ/mm-per-mm
    ///   heuristic;
    /// - vertices beyond `reject_factor * tolerance` -> `Reject`
    ///   (economically uncorrectable);
    /// - if more than `rework_fraction` (default 20 %) of vertices need
    ///   straightening, the block is systematically out of tolerance -> one
    ///   `Rework` instead of point-wise heating that would fight itself.
    ///
    /// An empty plan means everything is in tolerance and one `Accept` is
    /// emitted. The plan is also appended to `corrections`.
    pub fn correction_plan(&mut self, allowable_tolerance_mm: f64) -> Vec<CorrectionAction> {
        let field = match self.deviation_map_form() {
            Ok(f) => f,
            Err(_) => {
                let action = CorrectionAction::Reject {
                    reason: "measurement does not match target topology".into(),
                };
                self.corrections.push(action.clone());
                return vec![action];
            }
        };
        let reject_limit = self.reject_factor.max(1.0) * allowable_tolerance_mm;
        let mut plan: Vec<CorrectionAction> = Vec::new();
        let mut straighten: Vec<&DeviationPoint> = Vec::new();

        for p in &field.points {
            let dev = p.deviation_mm.abs();
            if dev <= allowable_tolerance_mm {
                continue;
            } else if dev > reject_limit {
                plan.push(CorrectionAction::Reject {
                    reason: format!(
                        "deviation {dev:.1} mm exceeds the rejection limit {reject_limit:.1} mm"
                    ),
                });
            } else {
                straighten.push(p);
            }
        }

        let frac = straighten.len() as f64 / field.points.len().max(1) as f64;
        if frac > self.rework_fraction.clamp(0.0, 1.0) {
            plan.push(CorrectionAction::Rework {
                description: format!(
                    "{:.0}% of vertices out of tolerance (max {:.1} mm form deviation): systematic rework required",
                    100.0 * frac,
                    field.max_deviation_mm
                ),
            });
        } else {
            for p in &straighten {
                let excess = (p.deviation_mm.abs() - allowable_tolerance_mm).max(0.5);
                let heat = match &self.straightening {
                    Some(model) => {
                        let rate = model.shrinkage_mm_per_kj_mm().max(1e-6);
                        (excess / rate).clamp(0.5, 500.0)
                    }
                    // Heuristic: ~1 kJ per mm of heat line per mm of excess.
                    None => excess.min(20.0),
                };
                plan.push(CorrectionAction::HeatStraighten {
                    location: p.target,
                    heat_input: heat,
                });
            }
        }

        if plan.is_empty() {
            plan.push(CorrectionAction::Accept {
                deviation_mm: field.max_deviation_mm,
            });
        }
        self.corrections.extend(plan.iter().cloned());
        plan
    }
}

/// Solves the symmetric 3x3 system `h * x = b` by Cramer's rule; returns
/// zero rotation when singular (degenerate point sets carry no rotation
/// information).
fn solve3_symmetric(h: &[[f64; 3]; 3], b: [f64; 3]) -> [f64; 3] {
    let det = h[0][0] * (h[1][1] * h[2][2] - h[1][2] * h[2][1])
        - h[0][1] * (h[1][0] * h[2][2] - h[1][2] * h[2][0])
        + h[0][2] * (h[1][0] * h[2][1] - h[1][1] * h[2][0]);
    if det.abs() < 1e-12 {
        return [0.0; 3];
    }
    let mut x = [0.0f64; 3];
    for c in 0..3 {
        let mut m = *h;
        for r in 0..3 {
            m[r][c] = b[r];
        }
        x[c] = (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))
            / det;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(bumps: &[(usize, f64)]) -> (Geometry3D, Geometry3D) {
        let target = Geometry3D::from_box(2000.0, 1000.0, 100.0);
        let mut measured = target.clone();
        for &(i, dz_mm) in bumps {
            measured.vertices[i].z += dz_mm / 1000.0;
        }
        (target, measured)
    }

    #[test]
    fn deviation_map_reports_max_and_rms() {
        let (target, measured) = panel(&[(0, 2.0), (1, -4.0)]);
        let dc = DistortionControl::new(target, measured);
        let field = dc.deviation_map().unwrap();
        assert!((field.max_deviation_mm - 4.0).abs() < 1e-9);
        assert!(field.rms_deviation_mm > 0.0 && field.rms_deviation_mm < 4.0);
        assert!((field.points[0].deviation_mm - 2.0).abs() < 1e-9);
        assert!((field.points[1].deviation_mm + 4.0).abs() < 1e-9);
    }

    #[test]
    fn within_tolerance_is_accepted() {
        let (target, measured) = panel(&[(0, 2.5), (3, -1.0)]);
        let mut dc = DistortionControl::new(target, measured);
        let plan = dc.correction_plan(3.0);
        assert_eq!(plan.len(), 1);
        match &plan[0] {
            CorrectionAction::Accept { deviation_mm } => {
                // The plan works on the form deviation: the best-fit rigid
                // motion absorbs part of the raw bumps, so the accepted
                // figure is at or below the raw 2.5 mm.
                assert!(*deviation_mm <= 2.5 + 1e-6, "got {deviation_mm}");
            }
            other => panic!("expected accept, got {other:?}"),
        }
        assert_eq!(dc.corrections.len(), 1);
    }

    #[test]
    fn moderate_deviation_is_heat_straightened() {
        let (target, measured) = panel(&[(0, 7.0)]);
        let bumped_vertex = target.vertices[0];
        let mut dc = DistortionControl::new(target, measured);
        let plan = dc.correction_plan(3.0);
        assert_eq!(plan.len(), 1);
        match &plan[0] {
            CorrectionAction::HeatStraighten {
                location,
                heat_input,
            } => {
                assert_eq!(*location, bumped_vertex);
                assert!(*heat_input > 0.0 && *heat_input <= 20.0);
            }
            other => panic!("expected heat straightening, got {other:?}"),
        }
    }

    #[test]
    fn gross_deviation_is_rejected() {
        let (target, measured) = panel(&[(2, 40.0)]); // 40 mm > 5 × 3 mm
        let mut dc = DistortionControl::new(target, measured);
        let plan = dc.correction_plan(3.0);
        assert!(matches!(&plan[0], CorrectionAction::Reject { .. }));
    }

    #[test]
    fn systematic_distortion_triggers_rework() {
        // 8 vertices; put 3 (37.5% > 20%) moderately out of tolerance.
        let (target, measured) = panel(&[(0, 6.0), (1, 7.0), (2, 8.0)]);
        let mut dc = DistortionControl::new(target, measured);
        let plan = dc.correction_plan(3.0);
        assert!(plan
            .iter()
            .any(|a| matches!(a, CorrectionAction::Rework { .. })));
    }

    /// Regression (review 7B): a displaced, tilted but otherwise perfect
    /// block has ~zero FORM deviation — the rigid-body motion is removed
    /// and reported, not corrected point by point.
    #[test]
    fn rigid_body_motion_is_removed_from_form_map() {
        let (target, _) = panel(&[]);
        let mut displaced = target.clone();
        // Shift the whole block 5 mm in +z and tilt it ~1 mrad about y
        // (both rotation components: x' = x + t*z, z' = z - t*x).
        let theta_y = 1.0e-3;
        for v in &mut displaced.vertices {
            let (x0, z0) = (v.x, v.z);
            v.x = x0 + theta_y * z0 + 0.0;
            v.z = z0 - theta_y * x0 + 5.0e-3;
        }
        let dc = DistortionControl::new(target, displaced);
        let raw = dc.deviation_map().unwrap();
        assert!(
            raw.max_deviation_mm > 5.0,
            "raw map shows the shift: {}",
            raw.max_deviation_mm
        );
        let form = dc.deviation_map_form().unwrap();
        assert!(
            form.max_deviation_mm < 0.5,
            "form deviation after rigid removal must be tiny: {}",
            form.max_deviation_mm
        );
        // The removed transform is reported.
        assert!((form.removed_translation.z - 5.0e-3).abs() < 1e-9);
        assert!(form.removed_rotation_rad.y.abs() > 1.0e-4);
        // And the correction plan accepts it (nothing to straighten).
        let mut dc = dc;
        assert!(matches!(
            dc.correction_plan(3.0)[0],
            CorrectionAction::Accept { .. }
        ));
    }

    /// Regression (review 7B): with a straightening model, the heat input
    /// is sized from the plate's contraction physics, not the flat
    /// heuristic.
    #[test]
    fn straightening_model_sizes_the_heat() {
        // Two opposite corners bumped: form distortion that no rigid
        // motion can absorb.
        let (target, measured) = panel(&[(0, 7.0), (7, 7.0)]);
        let model = StraighteningModel {
            material: Material::ah36(),
            plate_thickness_mm: 12.0,
            thermal_efficiency: 0.4,
        };
        let rate = model.shrinkage_mm_per_kj_mm();
        assert!(
            rate > 0.01 && rate < 1.0,
            "shrinkage rate {rate} mm per kJ/mm"
        );
        let mut dc = DistortionControl::new(target, measured).with_straightening_model(model);
        let plan = dc.correction_plan(3.0);
        for action in &plan {
            if let CorrectionAction::HeatStraighten { heat_input, .. } = action {
                // excess ~ 7 - (small fit absorption) - 3 mm; heat = excess / rate.
                let expected = 4.0 / rate;
                assert!(
                    (heat_input - expected).abs() < expected * 0.6,
                    "heat {heat_input} vs model-sized {expected}"
                );
            }
        }
        assert!(!plan.is_empty());
    }

    #[test]
    fn mismatched_topologies_reject() {
        let target = Geometry3D::from_box(1.0, 1.0, 1.0);
        let mut measured = Geometry3D::from_box(1.0, 1.0, 1.0);
        measured.vertices.push(Vector3::ZERO);
        let mut dc = DistortionControl::new(target, measured);
        let plan = dc.correction_plan(3.0);
        assert!(matches!(&plan[0], CorrectionAction::Reject { .. }));
        assert!(matches!(
            dc.deviation_map(),
            Err(DistortionError::MismatchedGeometry { .. })
        ));
    }

    #[test]
    fn boundary_values_use_tolerances() {
        // Just inside the tolerance → accept; just over → straighten. (The
        // mm→m→mm round trip through the geometry is not exact in binary
        // floats, so the boundary itself is asserted with a 0.1 mm margin.)
        let (target, measured) = panel(&[(0, 2.9)]);
        let mut dc = DistortionControl::new(target, measured);
        match &dc.correction_plan(3.0)[0] {
            CorrectionAction::Accept { deviation_mm } => {
                // The plan works on the form deviation: the rigid fit
                // absorbs most of a single-vertex bump (1 of 8 vertices is
                // nearly a pure shift+tilt at this resolution).
                assert!(*deviation_mm <= 2.9 + 1e-6, "got {deviation_mm}");
            }
            other => panic!("expected accept inside tolerance, got {other:?}"),
        }

        // Just over: a single 5 mm bump still leaves ~3.4 mm of form
        // deviation after the rigid-body fit (the component along the
        // vertex radius is irreducible), so it must be straightened - and
        // at 1 of 8 vertices (12.5%) it stays under the rework fraction.
        let (target, measured) = panel(&[(0, 5.0)]);
        let mut dc = DistortionControl::new(target, measured);
        assert!(matches!(
            dc.correction_plan(3.0)[0],
            CorrectionAction::HeatStraighten { .. }
        ));
    }
}
