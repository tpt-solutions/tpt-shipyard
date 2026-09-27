//! Quality control and inspection: NDT plans and defect tracking.
//!
//! [`QualityManagement::generate_inspection_plan`] turns the build plan
//! into NDT inspection points — each weld-bearing activity gets a method
//! and coverage by criticality (class-society practice: full-penetration
//! butts get radiography/ultrasound, fillets get magnetic particle or dye
//! penetrant, pressure boundaries get 100 %). [`QualityManagement::
//! defect_tracking`] aggregates inspection findings into a repair-rate
//! report.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::{ActivityId, ActivityType, BuildPhase, PhaseId};
//! use tpt_yard_quality::{NdtMethod, QualityManagement};
//!
//! let mut phase = BuildPhase::new(PhaseId(1), "Panel line", 5.0);
//! phase.activities.push(tpt_yard_core::AssemblyActivity::new(
//!     ActivityId(1), "Weld butt seam", ActivityType::WeldBlock, 8.0,
//! ));
//! let qm = QualityManagement::default();
//! let plan = qm.generate_inspection_plan(&[phase]);
//! assert_eq!(plan.len(), 1);
//! assert_eq!(plan[0].method, NdtMethod::UltrasonicTesting);
//! ```
#![allow(clippy::doc_markdown)]

use tpt_yard_core::{ActivityId, ActivityType, BuildPhase};

/// Non-destructive testing methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NdtMethod {
    /// Ultrasonic testing (volumetric).
    UltrasonicTesting,
    /// Radiographic testing (volumetric).
    RadiographicTesting,
    /// Magnetic particle testing (surface/near-surface, ferromagnetic).
    MagneticParticleTesting,
    /// Dye penetrant testing (surface).
    DyePenetrantTesting,
    /// Eddy current testing (surface).
    EddyCurrentTesting,
    /// Visual inspection (always the first gate).
    VisualInspection,
    /// Vacuum-box testing (tank bottoms).
    VacuumTesting,
    /// Pressure / leak testing.
    PressureTesting,
}

impl NdtMethod {
    /// Short name for reports.
    pub fn code(self) -> &'static str {
        match self {
            NdtMethod::UltrasonicTesting => "UT",
            NdtMethod::RadiographicTesting => "RT",
            NdtMethod::MagneticParticleTesting => "MT",
            NdtMethod::DyePenetrantTesting => "PT",
            NdtMethod::EddyCurrentTesting => "ET",
            NdtMethod::VisualInspection => "VT",
            NdtMethod::VacuumTesting => "VAC",
            NdtMethod::PressureTesting => "PRESS",
        }
    }
}

/// Acceptance criteria for an inspection point.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptanceCriteria {
    /// Standard reference (e.g. "IACS Rec.47", "NASA-STD-5009").
    pub standard: String,
    /// Maximum acceptable defect indication, mm.
    pub max_indication_mm: f64,
    /// Required coverage, percent (100 for pressure boundaries).
    pub coverage_pct: f64,
}

impl Default for AcceptanceCriteria {
    fn default() -> Self {
        AcceptanceCriteria {
            standard: "IACS Rec.47".into(),
            max_indication_mm: 2.0,
            coverage_pct: 100.0,
        }
    }
}

/// One planned inspection.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectionPoint {
    /// Point identifier (sequential).
    pub id: u64,
    /// The activity inspected.
    pub activity: ActivityId,
    /// Phase the inspection belongs to.
    pub phase: String,
    /// NDT method.
    pub method: NdtMethod,
    /// Required coverage, percent.
    pub coverage_pct: f64,
    /// Weld criticality driving the method choice (I-IV, I = highest).
    pub criticality: u8,
    /// Timing: after which step of the activity ("post-weld", "post-paint").
    pub timing: String,
}

/// A tracked defect.
#[derive(Debug, Clone, PartialEq)]
pub struct DefectRecord {
    /// Defect id.
    pub id: u64,
    /// The inspection point that found it.
    pub inspection_point: u64,
    /// Defect type.
    pub defect_type: DefectType,
    /// Indication size, mm.
    pub size_mm: f64,
    /// Disposition.
    pub disposition: Disposition,
}

/// Defect categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefectType {
    /// Gas porosity.
    Porosity,
    /// Lack of fusion.
    LackOfFusion,
    /// Lack of penetration.
    LackOfPenetration,
    /// Cracking (always reject).
    Cracking,
    /// Undercut.
    Undercut,
    /// Slag inclusion.
    SlagInclusion,
    /// Misalignment / fit-up.
    Misalignment,
}

/// Dispositions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Accepted as-is (within criteria).
    AcceptAsIs,
    /// Repair (grind-out and re-weld).
    Repair,
    /// Repair and re-examine.
    RepairAndReinspect,
    /// Rejected — engineering review.
    Reject,
}

/// Aggregate defect report.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DefectReport {
    /// Total defects found.
    pub total: usize,
    /// Count per defect type, descending.
    pub by_type: Vec<(DefectType, usize)>,
    /// Repairs over findings (repair rate), 0-1.
    pub repair_rate: f64,
    /// Rejected to engineering.
    pub rejected: usize,
    /// Most frequent defect type, if any.
    pub dominant_defect: Option<DefectType>,
}

/// The quality management system.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityManagement {
    /// Methods available in the yard.
    pub ndt_methods: Vec<NdtMethod>,
    /// Acceptance criteria.
    pub acceptance_criteria: AcceptanceCriteria,
}

impl Default for QualityManagement {
    fn default() -> Self {
        Self {
            ndt_methods: vec![
                NdtMethod::VisualInspection,
                NdtMethod::UltrasonicTesting,
                NdtMethod::RadiographicTesting,
                NdtMethod::MagneticParticleTesting,
                NdtMethod::DyePenetrantTesting,
                NdtMethod::VacuumTesting,
                NdtMethod::PressureTesting,
            ],
            acceptance_criteria: AcceptanceCriteria::default(),
        }
    }
}

impl QualityManagement {
    /// Generates the inspection plan for the given build phases.
    ///
    /// Method selection by criticality (class-society practice):
    /// - `WeldBlock` / `JoinBlock` (structural butts): I — UT with RT
    ///   sample, 100 % coverage on pressure boundaries, 30 % otherwise;
    /// - `Outfit { Piping {..} }`: II — PT or ET at joints, 100 % on
    ///   pressure tests;
    /// - everything else: III — visual only.
    pub fn generate_inspection_plan(&self, build_phases: &[BuildPhase]) -> Vec<InspectionPoint> {
        let mut plan = Vec::new();
        let mut id = 1u64;
        for phase in build_phases {
            for a in &phase.activities {
                let (method, coverage, criticality): (NdtMethod, f64, u8) = match &a.activity_type {
                    ActivityType::WeldBlock | ActivityType::JoinBlock => {
                        (NdtMethod::UltrasonicTesting, 100.0, 1)
                    }
                    ActivityType::Outfit {
                        system: tpt_yard_core::OutfitSystem::Piping { .. },
                    } => (NdtMethod::DyePenetrantTesting, 100.0, 2),
                    ActivityType::Test {
                        test_type:
                            tpt_yard_core::TestType::Pressure | tpt_yard_core::TestType::Hydrostatic,
                    } => (NdtMethod::PressureTesting, 100.0, 1),
                    _ => (NdtMethod::VisualInspection, 100.0, 3),
                };
                plan.push(InspectionPoint {
                    id,
                    activity: a.id,
                    phase: phase.name.clone(),
                    method,
                    coverage_pct: coverage,
                    criticality,
                    timing: "post-weld".into(),
                });
                id += 1;
            }
        }
        plan
    }

    /// Aggregates defect records into a report.
    pub fn defect_tracking(&self, records: &[DefectRecord]) -> DefectReport {
        let mut by_type: Vec<(DefectType, usize)> = Vec::new();
        for r in records {
            match by_type.iter_mut().find(|(t, _)| *t == r.defect_type) {
                Some((_, n)) => *n += 1,
                None => by_type.push((r.defect_type, 1)),
            }
        }
        by_type.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(format!("{:?}", a.0).cmp(&format!("{:?}", b.0)))
        });
        let repairs = records
            .iter()
            .filter(|r| {
                matches!(
                    r.disposition,
                    Disposition::Repair | Disposition::RepairAndReinspect
                )
            })
            .count();
        let rejected = records
            .iter()
            .filter(|r| matches!(r.disposition, Disposition::Reject))
            .count();
        DefectReport {
            total: records.len(),
            repair_rate: if records.is_empty() {
                0.0
            } else {
                repairs as f64 / records.len() as f64
            },
            rejected,
            dominant_defect: by_type.first().map(|(t, _)| *t),
            by_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_core::{
        ActivityType, AssemblyActivity, FluidType, OutfitSystem, PhaseId, TestType,
    };

    fn panel_line() -> Vec<BuildPhase> {
        let mut p1 = BuildPhase::new(PhaseId(1), "Panel line", 10.0);
        p1.activities.push(AssemblyActivity::new(
            ActivityId(1),
            "Weld butt",
            ActivityType::WeldBlock,
            8.0,
        ));
        p1.activities.push(AssemblyActivity::new(
            ActivityId(2),
            "Paint",
            ActivityType::Paint,
            4.0,
        ));
        let mut p2 = BuildPhase::new(PhaseId(2), "Outfitting", 20.0);
        p2.activities.push(AssemblyActivity::new(
            ActivityId(3),
            "Sea water line",
            ActivityType::Outfit {
                system: OutfitSystem::Piping {
                    fluid: FluidType::SeaWater,
                    diameter_mm: 200.0,
                },
            },
            6.0,
        ));
        p2.activities.push(AssemblyActivity::new(
            ActivityId(4),
            "Hydro test",
            ActivityType::Test {
                test_type: TestType::Hydrostatic,
            },
            2.0,
        ));
        vec![p1, p2]
    }

    #[test]
    fn inspection_plan_covers_welds_and_tests() {
        let qm = QualityManagement::default();
        let plan = qm.generate_inspection_plan(&panel_line());
        assert_eq!(plan.len(), 4);
        assert_eq!(plan[0].method, NdtMethod::UltrasonicTesting);
        assert_eq!(plan[0].criticality, 1);
        // Paint gets visual only.
        assert_eq!(plan[1].method, NdtMethod::VisualInspection);
        assert_eq!(plan[1].criticality, 3);
        // Piping gets PT at 100 %.
        assert_eq!(plan[2].method, NdtMethod::DyePenetrantTesting);
        // Hydrostatic test gets pressure testing.
        assert_eq!(plan[3].method, NdtMethod::PressureTesting);
        // Phases propagate.
        assert_eq!(plan[2].phase, "Outfitting");
    }

    #[test]
    fn defect_tracking_aggregates() {
        let qm = QualityManagement::default();
        let records = vec![
            DefectRecord {
                id: 1,
                inspection_point: 1,
                defect_type: DefectType::Porosity,
                size_mm: 1.5,
                disposition: Disposition::Repair,
            },
            DefectRecord {
                id: 2,
                inspection_point: 1,
                defect_type: DefectType::Porosity,
                size_mm: 1.0,
                disposition: Disposition::AcceptAsIs,
            },
            DefectRecord {
                id: 3,
                inspection_point: 2,
                defect_type: DefectType::Cracking,
                size_mm: 5.0,
                disposition: Disposition::Reject,
            },
            DefectRecord {
                id: 4,
                inspection_point: 2,
                defect_type: DefectType::Undercut,
                size_mm: 0.8,
                disposition: Disposition::RepairAndReinspect,
            },
        ];
        let report = qm.defect_tracking(&records);
        assert_eq!(report.total, 4);
        assert_eq!(report.dominant_defect, Some(DefectType::Porosity));
        assert_eq!(report.by_type[0], (DefectType::Porosity, 2));
        // 2 repairs of 4 findings.
        assert!((report.repair_rate - 0.5).abs() < 1e-9);
        assert_eq!(report.rejected, 1);
    }

    #[test]
    fn empty_tracking_is_clean() {
        let qm = QualityManagement::default();
        let report = qm.defect_tracking(&[]);
        assert_eq!(report.total, 0);
        assert_eq!(report.repair_rate, 0.0);
        assert_eq!(report.dominant_defect, None);
    }
}
