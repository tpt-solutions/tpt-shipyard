//! Shared joint-geometry primitives for structural and welding analysis.
//!
//! A [`JointGeometry`] describes the *shape* of the connection: joint kind,
//! groove, thickness, root gap, and fillet legs. From those it derives the
//! quantities downstream physics needs — groove cross-section (weld metal
//! area for the welding crate's heat budgets, effective throat for strength,
//! weld volume for consumables).
//!
//! # Example
//!
//! ```
//! use tpt_yard_joints::{GrooveType, JointGeometry, JointKind};
//!
//! // Single-V butt weld, 60° included angle, 12 mm plate.
//! let butt = JointGeometry::new(JointKind::Butt)
//!     .with_thickness_mm(12.0)
//!     .with_groove(GrooveType::V)
//!     .with_groove_angle_deg(60.0)
//!     .with_root_gap_mm(3.0)
//!     .with_root_face_mm(2.0);
//!
//! let area = butt.weld_area_mm2(); // groove cross-section to fill
//! assert!(area > 0.0);
//!
//! // Fillet on the same plate.
//! let fillet = JointGeometry::new(JointKind::Fillet)
//!     .with_thickness_mm(12.0)
//!     .with_leg_mm(6.0);
//! assert!((fillet.effective_throat_mm() - 6.0 / 2.0f64.sqrt()).abs() < 1e-9);
//! ```

use std::fmt;

/// Joint family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointKind {
    /// Butt joint: plates meet edge to edge.
    Butt,
    /// Fillet joint: plates meet at an angle, weld in the corner.
    Fillet,
    /// Lap joint: overlapping plates, fillet welds at the edges.
    Lap,
    /// T-joint: plate perpendicular to another surface.
    TJoint,
    /// Corner joint: plates meet at their edges forming a corner.
    Corner,
    /// Edge joint: plates parallel, welded along their edges.
    Edge,
}

impl fmt::Display for JointKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            JointKind::Butt => "butt",
            JointKind::Fillet => "fillet",
            JointKind::Lap => "lap",
            JointKind::TJoint => "t-joint",
            JointKind::Corner => "corner",
            JointKind::Edge => "edge",
        };
        f.write_str(s)
    }
}

/// Groove preparation for a butt-type joint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrooveType {
    /// No groove (square edge) — root gap only.
    Square,
    /// Single-V groove.
    V,
    /// Double-V (X) groove.
    DoubleV,
    /// Single bevel.
    Bevel,
    /// K (double bevel) groove.
    K,
    /// J groove.
    J,
    /// U groove.
    U,
}

/// Geometry of a welded joint.
#[derive(Debug, Clone, PartialEq)]
pub struct JointGeometry {
    /// Joint family.
    pub kind: JointKind,
    /// Plate thickness, mm.
    pub thickness_mm: f64,
    /// Groove preparation (butt-type joints).
    pub groove: GrooveType,
    /// Included groove angle, degrees (V, bevel: 30–75 typical).
    pub groove_angle_deg: f64,
    /// Root gap, mm.
    pub root_gap_mm: f64,
    /// Root face (land), mm.
    pub root_face_mm: f64,
    /// Fillet leg length, mm (fillet family).
    pub leg_mm: f64,
    /// Weld length along the seam, mm.
    pub length_mm: f64,
    /// Penetration depth achieved beyond the root, mm (deep-penetrating
    /// processes like SAW; 0 = no extra penetration credit).
    pub penetration_mm: f64,
}

impl JointGeometry {
    /// Creates a joint of the given kind with neutral defaults
    /// (thickness 10 mm, square groove, no gap, weld length 1000 mm).
    pub fn new(kind: JointKind) -> Self {
        Self {
            kind,
            thickness_mm: 10.0,
            groove: GrooveType::Square,
            groove_angle_deg: 0.0,
            root_gap_mm: 0.0,
            root_face_mm: 0.0,
            leg_mm: 0.0,
            length_mm: 1000.0,
            penetration_mm: 0.0,
        }
    }

    /// Builder: plate thickness, mm.
    #[must_use]
    pub fn with_thickness_mm(mut self, t: f64) -> Self {
        self.thickness_mm = t;
        self
    }

    /// Builder: groove preparation.
    #[must_use]
    pub fn with_groove(mut self, g: GrooveType) -> Self {
        self.groove = g;
        self
    }

    /// Builder: included groove angle, degrees.
    #[must_use]
    pub fn with_groove_angle_deg(mut self, deg: f64) -> Self {
        self.groove_angle_deg = deg;
        self
    }

    /// Builder: root gap, mm.
    #[must_use]
    pub fn with_root_gap_mm(mut self, gap: f64) -> Self {
        self.root_gap_mm = gap;
        self
    }

    /// Builder: root face, mm.
    #[must_use]
    pub fn with_root_face_mm(mut self, face: f64) -> Self {
        self.root_face_mm = face;
        self
    }

    /// Builder: fillet leg, mm.
    #[must_use]
    pub fn with_leg_mm(mut self, leg: f64) -> Self {
        self.leg_mm = leg;
        self
    }

    /// Builder: seam length, mm.
    #[must_use]
    pub fn with_length_mm(mut self, length: f64) -> Self {
        self.length_mm = length;
        self
    }

    /// Builder: penetration credit, mm.
    #[must_use]
    pub fn with_penetration_mm(mut self, pen: f64) -> Self {
        self.penetration_mm = pen;
        self
    }

    /// Effective throat of a fillet weld, mm: `leg / √2` (equal-leg fillet),
    /// with penetration credit added.
    pub fn effective_throat_mm(&self) -> f64 {
        if self.leg_mm <= 0.0 {
            return 0.0;
        }
        self.leg_mm / 2.0f64.sqrt() + self.penetration_mm
    }

    /// Cross-sectional area of weld metal deposited per mm of seam, mm².
    ///
    /// For butt joints this is the groove profile (trapezoidal V/X family,
    /// rectangular for the gap); for the fillet family it is `leg² / 2`.
    pub fn weld_area_mm2(&self) -> f64 {
        match self.kind {
            JointKind::Fillet | JointKind::Lap | JointKind::Edge => self.leg_mm * self.leg_mm / 2.0,
            // A T-joint is fillet-welded when a leg is given, groove-welded
            // (full penetration) otherwise.
            JointKind::TJoint if self.leg_mm > 0.0 => self.leg_mm * self.leg_mm / 2.0,
            JointKind::Butt | JointKind::TJoint | JointKind::Corner => self.groove_area_mm2(),
        }
    }

    /// Total weld metal area accounting for double-sided work — mm².
    ///
    /// DoubleV/K grooves are welded from both sides; lap joints and
    /// square-groove T-joints get two fillet toes.
    pub fn weld_area_mm2_total(&self) -> f64 {
        let single = self.weld_area_mm2();
        match (self.kind, self.groove) {
            (JointKind::Butt, GrooveType::DoubleV) | (JointKind::Butt, GrooveType::K) => {
                2.0 * single
            }
            (JointKind::Lap, _) => 2.0 * single,
            (JointKind::TJoint, GrooveType::Square) if self.leg_mm > 0.0 => 2.0 * single,
            _ => single,
        }
    }

    /// Groove cross-section area for butt-type joints, mm².
    fn groove_area_mm2(&self) -> f64 {
        let t = self.thickness_mm;
        let face = self.root_face_mm.min(t);
        let groove_depth = (t - face).max(0.0);
        let half_angle = self.groove_angle_deg.to_radians() / 2.0;
        let triangular = match self.groove {
            GrooveType::Square => 0.0,
            GrooveType::V | GrooveType::Bevel | GrooveType::J | GrooveType::U => {
                groove_depth * groove_depth * half_angle.tan()
            }
            GrooveType::DoubleV | GrooveType::K => {
                // Both sides share the depth symmetrically.
                groove_depth * groove_depth * half_angle.tan()
            }
        };
        // The root gap fills across the root-face height: area = gap × face.
        let gap_area = self.root_gap_mm * face;
        // Deep penetration extends the fused zone below the root; its added
        // area is the penetration depth continuing through the gap channel.
        let penetration_area = self.penetration_mm * self.root_gap_mm;
        triangular + gap_area + penetration_area
    }

    /// Volume of weld metal for the whole seam, mm³ (single side).
    pub fn weld_volume_mm3(&self) -> f64 {
        self.weld_area_mm2() * self.length_mm
    }

    /// Mass of weld metal, kg, for a filler of given density (7.85 g/cm³ for
    /// steel wire).
    pub fn weld_mass_kg(&self, density_kg_m3: f64) -> f64 {
        self.weld_volume_mm3() / 1e9 * density_kg_m3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn butt_v_groove_area() {
        // 60° V, 12 mm plate, 2 mm face, 3 mm gap:
        // triangular = (12-2)²·tan(30°) = 57.735, gap area = 3 × 2 = 6
        let j = JointGeometry::new(JointKind::Butt)
            .with_thickness_mm(12.0)
            .with_groove(GrooveType::V)
            .with_groove_angle_deg(60.0)
            .with_root_face_mm(2.0)
            .with_root_gap_mm(3.0);
        assert!(
            close(j.weld_area_mm2(), 57.735 + 6.0, 0.01),
            "got {}",
            j.weld_area_mm2()
        );
    }

    #[test]
    fn square_groove_is_gap_only() {
        let j = JointGeometry::new(JointKind::Butt)
            .with_thickness_mm(8.0)
            .with_root_gap_mm(2.0);
        assert!(close(j.weld_area_mm2(), 0.0, 1e-12));
    }

    #[test]
    fn fillet_throat_and_area() {
        let j = JointGeometry::new(JointKind::Fillet).with_leg_mm(8.0);
        assert!(close(j.effective_throat_mm(), 8.0 / 2.0f64.sqrt(), 1e-12));
        assert!(close(j.weld_area_mm2(), 32.0, 1e-12));
        // Penetration adds to the throat (SAW deep penetration).
        let jp = j.with_penetration_mm(2.0);
        assert!(close(
            jp.effective_throat_mm(),
            8.0 / 2.0f64.sqrt() + 2.0,
            1e-12
        ));
    }

    #[test]
    fn double_sided_totals() {
        let v = JointGeometry::new(JointKind::Butt)
            .with_thickness_mm(20.0)
            .with_groove(GrooveType::V)
            .with_groove_angle_deg(60.0);
        let single = v.weld_area_mm2();
        let x = v.with_groove(GrooveType::DoubleV);
        assert!(close(x.weld_area_mm2_total(), 2.0 * single, 1e-12));
    }

    #[test]
    fn volume_and_mass() {
        let j = JointGeometry::new(JointKind::Fillet)
            .with_leg_mm(6.0)
            .with_length_mm(2000.0);
        assert!(close(j.weld_volume_mm3(), 18.0 * 2000.0, 1e-9));
        assert!(close(
            j.weld_mass_kg(7850.0),
            18.0 * 2000.0 / 1e9 * 7850.0,
            1e-12
        ));
    }

    #[test]
    fn t_joint_double_fillet_totals() {
        let j = JointGeometry::new(JointKind::TJoint)
            .with_thickness_mm(10.0)
            .with_leg_mm(7.0);
        // T-joints with square groove get welded from both sides.
        assert!(close(j.weld_area_mm2_total(), 2.0 * 24.5, 1e-12));
    }
}
