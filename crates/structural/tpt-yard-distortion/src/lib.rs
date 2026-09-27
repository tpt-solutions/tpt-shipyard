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
//! use tpt_yard_core::{Geometry3D, Vector3};
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

use tpt_yard_core::{Geometry3D, Vector3};

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
}

impl DistortionControl {
    /// Creates a controller with no corrections yet.
    pub fn new(target_geometry: Geometry3D, measured_geometry: Geometry3D) -> Self {
        Self {
            target_geometry,
            measured_geometry,
            corrections: Vec::new(),
        }
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
        })
    }

    /// Plans corrections against an allowable tolerance (mm).
    ///
    /// Policy (thresholds verified in tests):
    /// - every vertex with `|deviation| ≤ tolerance` needs no action;
    /// - vertices with `tolerance < |deviation| ≤ 5 × tolerance` →
    ///   `HeatStraighten` at the vertex location, heat input scaled to the
    ///   excess;
    /// - vertices beyond `5 × tolerance` → `Reject` (economically
    ///   uncorrectable);
    /// - if more than 20% of vertices need straightening, the block is
    ///   systematically out of tolerance → one `Rework` instead of
    ///   point-wise heating that would fight itself.
    ///
    /// An empty plan means everything is in tolerance and one `Accept` is
    /// emitted. The plan is also appended to `corrections`.
    pub fn correction_plan(&mut self, allowable_tolerance_mm: f64) -> Vec<CorrectionAction> {
        let field = match self.deviation_map() {
            Ok(f) => f,
            Err(_) => {
                let action = CorrectionAction::Reject {
                    reason: "measurement does not match target topology".into(),
                };
                self.corrections.push(action.clone());
                return vec![action];
            }
        };
        let reject_limit = 5.0 * allowable_tolerance_mm;
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

        if straighten.len() * 5 > field.points.len() {
            plan.push(CorrectionAction::Rework {
                description: format!(
                    "{:.0}% of vertices out of tolerance (max {:.1} mm): systematic rework required",
                    100.0 * straighten.len() as f64 / field.points.len() as f64,
                    field.max_deviation_mm
                ),
            });
        } else {
            for p in &straighten {
                let excess = (p.deviation_mm.abs() - allowable_tolerance_mm).max(0.5);
                plan.push(CorrectionAction::HeatStraighten {
                    location: p.target,
                    // ~1 kJ per mm of heat line per mm of excess (heuristic).
                    heat_input: excess.min(20.0),
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
                assert!((deviation_mm - 2.5).abs() < 1e-6, "got {deviation_mm}");
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
                assert!((deviation_mm - 2.9).abs() < 1e-6, "got {deviation_mm}");
            }
            other => panic!("expected accept inside tolerance, got {other:?}"),
        }

        let (target, measured) = panel(&[(0, 3.1)]);
        let mut dc = DistortionControl::new(target, measured);
        assert!(matches!(
            dc.correction_plan(3.0)[0],
            CorrectionAction::HeatStraighten { .. }
        ));
    }
}
