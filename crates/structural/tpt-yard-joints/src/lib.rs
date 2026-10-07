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

/// Why a joint geometry cannot be evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointError {
    /// A dimension is NaN, infinite or negative, or the thickness is not
    /// positive.
    InvalidDimension,
    /// The groove angle is outside `[0, 180)` degrees (180 degrees is an
    /// infinitely wide groove), or a V-family groove has no angle.
    InvalidAngle,
}

impl fmt::Display for JointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JointError::InvalidDimension => f.write_str(
                "joint dimensions must be finite, non-negative and the thickness positive",
            ),
            JointError::InvalidAngle => f.write_str("the groove angle must be in [0, 180) degrees"),
        }
    }
}

impl std::error::Error for JointError {}

/// Groove preparation for a butt-type joint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrooveType {
    /// No groove (square edge) — root gap only.
    Square,
    /// Single-V groove.
    V,
    /// Double-V (X) groove.
    DoubleV,
    /// Single bevel (one plate square, the other bevelled; the groove
    /// angle is the bevel angle).
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

    /// Checks the dimensions are finite and non-negative (thickness
    /// positive) and the groove angle lies in `[0, 180)` degrees.
    ///
    /// # Errors
    ///
    /// [`JointError`] naming the class of problem.
    pub fn validate(&self) -> Result<(), JointError> {
        let dims = [
            self.thickness_mm,
            self.groove_angle_deg,
            self.root_gap_mm,
            self.root_face_mm,
            self.leg_mm,
            self.length_mm,
            self.penetration_mm,
        ];
        if dims.iter().any(|d| !d.is_finite() || *d < 0.0) || self.thickness_mm <= 0.0 {
            return Err(JointError::InvalidDimension);
        }
        if self.groove_angle_deg >= 180.0 {
            return Err(JointError::InvalidAngle);
        }
        let angled = matches!(
            self.groove,
            GrooveType::V | GrooveType::DoubleV | GrooveType::Bevel | GrooveType::K
        );
        if angled
            && matches!(
                self.kind,
                JointKind::Butt | JointKind::TJoint | JointKind::Corner
            )
            && self.groove_angle_deg <= 0.0
        {
            return Err(JointError::InvalidAngle);
        }
        Ok(())
    }

    /// Cross-sectional area of weld metal deposited per mm of seam, mm²,
    /// or the validation error. For butt joints this is the groove profile
    /// plus the root gap over the full thickness (`gap x t`); for the
    /// fillet family it is `leg² / 2`.
    ///
    /// Groove profiles (`d` = thickness minus root face, `α` the groove
    /// angle): V `d² tan(α/2)`; single bevel `d² tan(α) / 2`; double-V
    /// `2 (d/2)² tan(α/2)` and K `2 (d/2)² tan(α) / 2` (both sides
    /// included); J/U rounded roots. A square groove is the gap alone.
    ///
    /// # Errors
    ///
    /// [`JointError`] for invalid dimensions or angle.
    pub fn try_weld_area_mm2(&self) -> Result<f64, JointError> {
        self.validate()?;
        Ok(match self.kind {
            JointKind::Fillet | JointKind::Lap | JointKind::Edge => self.leg_mm * self.leg_mm / 2.0,
            // A T-joint is fillet-welded when a leg is given, groove-welded
            // (full penetration) otherwise.
            JointKind::TJoint if self.leg_mm > 0.0 => self.leg_mm * self.leg_mm / 2.0,
            JointKind::Butt | JointKind::TJoint | JointKind::Corner => self.groove_area_mm2(),
        })
    }

    /// [`Self::try_weld_area_mm2`], with `NaN` for an invalid geometry so a
    /// bad joint poisons downstream numbers loudly instead of passing.
    pub fn weld_area_mm2(&self) -> f64 {
        self.try_weld_area_mm2().unwrap_or(f64::NAN)
    }

    /// Total weld metal area for the joint, mm²: the full cross-section
    /// including both sides of a double-sided preparation (the double-V
    /// and K profiles already count both sides, so they are *not* doubled
    /// again); lap joints and fillet-welded T-joints get two fillets.
    pub fn weld_area_mm2_total(&self) -> f64 {
        let single = self.weld_area_mm2();
        match (self.kind, self.groove) {
            (JointKind::Lap, _) => 2.0 * single,
            (JointKind::TJoint, GrooveType::Square) if self.leg_mm > 0.0 => 2.0 * single,
            _ => single,
        }
    }

    /// Groove cross-section area for butt-type joints, mm² (validated
    /// geometry only).
    fn groove_area_mm2(&self) -> f64 {
        let t = self.thickness_mm;
        let face = self.root_face_mm.min(t);
        let groove_depth = (t - face).max(0.0);
        let alpha = self.groove_angle_deg.to_radians();
        let half_angle = alpha / 2.0;
        // The root gap runs the full thickness: area = gap x t.
        let gap_area = self.root_gap_mm * t;
        // Deep penetration extends the fused zone below the root; its added
        // area is the penetration depth continuing through the gap channel.
        let penetration_area = self.penetration_mm * self.root_gap_mm;
        let profile = match self.groove {
            GrooveType::Square => 0.0,
            GrooveType::V => groove_depth * groove_depth * half_angle.tan(),
            // One plate square, one bevelled: a right triangle.
            GrooveType::Bevel => 0.5 * groove_depth * groove_depth * alpha.tan(),
            // DoubleV / K: the depth is shared by two opposing sides (each
            // welded over half the thickness); both sides are counted here.
            GrooveType::DoubleV => {
                let per_side = groove_depth / 2.0;
                2.0 * per_side * per_side * half_angle.tan()
            }
            GrooveType::K => {
                let per_side = groove_depth / 2.0;
                2.0 * 0.5 * per_side * per_side * alpha.tan()
            }
            // J and U grooves have a rounded root, not a sharp wedge: a
            // radius slot (root radius ≈ groove_depth/3, typical per
            // ISO 9692-1) plus a narrow flare. J has one vertical face and
            // a quarter-round root; U has a half-round root flaring both
            // ways. (J/U preparations pair with much smaller included
            // angles than V — the caller supplies the real prep angle.)
            GrooveType::U => {
                let r = (groove_depth / 3.0).max(1.0);
                let flare = (groove_depth - r).max(0.0);
                let top = 2.0 * r + 2.0 * flare * half_angle.tan();
                std::f64::consts::FRAC_PI_2 * r * r + (2.0 * r + top) * 0.5 * flare
            }
            GrooveType::J => {
                let r = (groove_depth / 3.0).max(1.0);
                let flare = (groove_depth - r).max(0.0);
                std::f64::consts::FRAC_PI_4 * r * r
                    + r * flare
                    + (r + flare * half_angle.tan()) * 0.5 * flare
            }
        };
        profile + gap_area + penetration_area
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
        // wedge = (12-2)²·tan(30°) = 57.735, gap = 3 mm over the full
        // 12 mm thickness = 36
        let j = JointGeometry::new(JointKind::Butt)
            .with_thickness_mm(12.0)
            .with_groove(GrooveType::V)
            .with_groove_angle_deg(60.0)
            .with_root_face_mm(2.0)
            .with_root_gap_mm(3.0);
        assert!(
            close(j.weld_area_mm2(), 57.735 + 36.0, 0.01),
            "got {}",
            j.weld_area_mm2()
        );
    }

    /// Regression (review 7B): the groove families must have distinct
    /// geometry — DoubleV/K weld half the depth per side (≈¼ the wedge area
    /// of a V), and J/U have rounded roots instead of a sharp V wedge.
    #[test]
    fn groove_families_differ() {
        let base = || {
            JointGeometry::new(JointKind::Butt)
                .with_thickness_mm(20.0)
                .with_groove_angle_deg(60.0)
                .with_root_face_mm(2.0)
                .with_root_gap_mm(3.0)
        };
        let v = base().with_groove(GrooveType::V).weld_area_mm2();
        let double_v = base().with_groove(GrooveType::DoubleV).weld_area_mm2();
        let k = base().with_groove(GrooveType::K).weld_area_mm2();
        // Wedge share: V = d²·tan, DoubleV = d²/2·tan (d = 18); the root
        // gap (3 mm over the 20 mm thickness = 60 mm²) is common to both.
        let gap = 60.0;
        assert!(
            close(double_v - gap, (v - gap) / 2.0, 1e-6),
            "{double_v} vs {v}"
        );
        // K = two bevels: 2 · ½ · (d/2)² · tan(60°) = d²/4 · tan(60°).
        let d = 18.0_f64;
        assert!(close(
            k - gap,
            d * d / 4.0 * 60.0_f64.to_radians().tan(),
            1e-9
        ));
        assert!(k != double_v);
        // Rounded roots differ from the sharp V wedge.
        let u = base().with_groove(GrooveType::U).weld_area_mm2();
        let j = base().with_groove(GrooveType::J).weld_area_mm2();
        assert!((u - v).abs() > 1.0, "U must not equal V: {u} vs {v}");
        assert!((j - v).abs() > 1.0, "J must not equal V: {j} vs {v}");
        assert!((u - j).abs() > 1.0, "U and J must differ: {u} vs {j}");
    }

    #[test]
    fn square_groove_is_the_gap_over_the_thickness() {
        let j = JointGeometry::new(JointKind::Butt)
            .with_thickness_mm(8.0)
            .with_root_gap_mm(2.0);
        assert!(close(j.weld_area_mm2(), 16.0, 1e-12));
        // A closed square butt deposits nothing.
        let closed = JointGeometry::new(JointKind::Butt).with_thickness_mm(8.0);
        assert!(close(closed.weld_area_mm2(), 0.0, 1e-12));
    }

    /// Hand values for the bevel (right triangle d²·tan(α)/2) and the
    /// double sides of a double-V (counted once, not twice).
    #[test]
    fn bevel_and_double_sided_profiles() {
        let bevel = JointGeometry::new(JointKind::Butt)
            .with_thickness_mm(10.0)
            .with_groove(GrooveType::Bevel)
            .with_groove_angle_deg(45.0);
        assert!(close(bevel.weld_area_mm2(), 0.5 * 100.0, 1e-9));
        let x = JointGeometry::new(JointKind::Butt)
            .with_thickness_mm(20.0)
            .with_groove(GrooveType::DoubleV)
            .with_groove_angle_deg(60.0);
        // 2 · (10)² · tan(30°) = 115.47
        assert!(close(
            x.weld_area_mm2(),
            200.0 * 30.0_f64.to_radians().tan(),
            1e-9
        ));
        assert!(close(x.weld_area_mm2_total(), x.weld_area_mm2(), 1e-12));
    }

    #[test]
    fn invalid_geometry_is_an_error_not_infinite_area() {
        let v = |deg: f64| {
            JointGeometry::new(JointKind::Butt)
                .with_thickness_mm(12.0)
                .with_groove(GrooveType::V)
                .with_groove_angle_deg(deg)
        };
        assert_eq!(v(180.0).try_weld_area_mm2(), Err(JointError::InvalidAngle));
        assert_eq!(v(0.0).try_weld_area_mm2(), Err(JointError::InvalidAngle));
        assert!(v(180.0).weld_area_mm2().is_nan());
        assert_eq!(
            v(60.0).with_root_gap_mm(f64::NAN).try_weld_area_mm2(),
            Err(JointError::InvalidDimension)
        );
        assert_eq!(
            v(60.0).with_thickness_mm(0.0).try_weld_area_mm2(),
            Err(JointError::InvalidDimension)
        );
        assert!(v(60.0).try_weld_area_mm2().is_ok());
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

    /// Regression (Phase 8, 8A1): the double-V profile already covers both
    /// sides; the total must not double it again (welding was given 2x the
    /// shrinkage and angular distortion).
    #[test]
    fn double_sided_totals_are_not_doubled() {
        let v = JointGeometry::new(JointKind::Butt)
            .with_thickness_mm(20.0)
            .with_groove(GrooveType::V)
            .with_groove_angle_deg(60.0);
        let x = v.clone().with_groove(GrooveType::DoubleV);
        assert!(close(x.weld_area_mm2_total(), x.weld_area_mm2(), 1e-12));
        // A double-V takes less metal than a single V of the same angle.
        assert!(x.weld_area_mm2_total() < v.weld_area_mm2());
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
