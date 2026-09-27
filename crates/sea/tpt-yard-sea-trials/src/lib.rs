//! Post-launch sea-trial test planning and acceptance-criteria evaluation.
//!
//! A [`TrialProgram`] lists the trials the vessel must pass before
//! delivery — speed trials, manoeuvring, crash stop, seakeeping, inclining,
//! noise & vibration, endurance, and the class-society acceptance protocol.
//! [`TrialProgram::evaluate`] checks measured values against the acceptance
//! criteria and produces the delivery-facing [`TrialReport`].
//!
//! # Example
//!
//! ```
//! use tpt_yard_sea_trials::{Acceptance, Metric, Trial, TrialKind, TrialProgram};
//!
//! let mut program = TrialProgram::new();
//! program.push(Trial {
//!     id: 1,
//!     name: "Speed trial".into(),
//!     kind: TrialKind::Speed,
//!     acceptance: Acceptance { metric: Metric::SpeedKn, minimum: Some(15.0), maximum: None },
//!     measured: Some(15.6),
//! });
//! let report = program.evaluate();
//! assert!(report.all_passed);
//! assert_eq!(report.results[0].status, Ok(()));
//! ```

use std::fmt;

/// Quantities a trial is judged on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Metric {
    /// Speed over ground, knots.
    SpeedKn,
    /// Turning circle diameter, ship lengths.
    TurningCircleLengths,
    /// Crash-stop distance, ship lengths.
    StoppingDistanceLengths,
    /// RMS vertical acceleration at the bridge, g.
    VerticalAccelerationG,
    /// A-weighted noise level in cabins, dB.
    CabinNoiseDb,
    /// Fuel consumption over the endurance run, tonnes/day.
    FuelConsumptionTD,
    /// Initial GM from the inclining experiment, m.
    GmM,
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Metric::SpeedKn => "speed [kn]",
            Metric::TurningCircleLengths => "turning circle [L]",
            Metric::StoppingDistanceLengths => "stopping distance [L]",
            Metric::VerticalAccelerationG => "vertical acceleration [g]",
            Metric::CabinNoiseDb => "cabin noise [dB(A)]",
            Metric::FuelConsumptionTD => "fuel consumption [t/d]",
            Metric::GmM => "GM [m]",
        };
        f.write_str(s)
    }
}

/// Trial categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialKind {
    /// Speed/power trial on the measured mile.
    Speed,
    /// Manoeuvring (turning circles, zig-zag).
    Maneuvering,
    /// Crash stop.
    CrashStop,
    /// Seakeeping (accelerations, slamming).
    Seakeeping,
    /// Noise & vibration survey.
    NoiseVibration,
    /// Endurance run at service power.
    Endurance,
    /// Inclining experiment (lightship weight & CoG).
    Inclining,
    /// Class-society acceptance protocol.
    ClassAcceptance,
}

/// Acceptance window on a metric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Acceptance {
    /// The judged quantity.
    pub metric: Metric,
    /// Inclusive lower bound, if any.
    pub minimum: Option<f64>,
    /// Inclusive upper bound, if any.
    pub maximum: Option<f64>,
}

impl Acceptance {
    /// True if `value` lies inside the window.
    pub fn accepts(&self, value: f64) -> bool {
        let above = self.minimum.is_none_or(|min| value >= min);
        let below = self.maximum.is_none_or(|max| value <= max);
        above && below
    }
}

/// One trial of the program.
#[derive(Debug, Clone, PartialEq)]
pub struct Trial {
    /// Trial identifier.
    pub id: u64,
    /// Name.
    pub name: String,
    /// Category.
    pub kind: TrialKind,
    /// Acceptance window.
    pub acceptance: Acceptance,
    /// Measured value (`None` = not yet performed).
    pub measured: Option<f64>,
}

/// Outcome of one evaluated trial.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialOutcome {
    /// Trial id.
    pub trial_id: u64,
    /// `Ok(())` passed; `Err(reason)` failed; `Err("not performed")` when
    /// no measurement exists.
    pub status: Result<(), String>,
    /// The measured value, if any.
    pub measured: Option<f64>,
    /// Human-readable summary.
    pub summary: String,
}

/// The delivery-facing report.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrialReport {
    /// One outcome per trial, in program order.
    pub results: Vec<TrialOutcome>,
    /// True only if every trial passed.
    pub all_passed: bool,
    /// Count passed / total.
    pub passed_of: (usize, usize),
}

/// The sea-trial program for a vessel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrialProgram {
    /// Trials, in planned execution order.
    pub trials: Vec<Trial>,
}

impl TrialProgram {
    /// Creates an empty program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a trial.
    pub fn push(&mut self, trial: Trial) {
        self.trials.push(trial);
    }

    /// Evaluates every trial against its acceptance window.
    pub fn evaluate(&self) -> TrialReport {
        let mut results = Vec::with_capacity(self.trials.len());
        let mut passed = 0;
        for t in &self.trials {
            let (status, summary) = match t.measured {
                None => (
                    Err("not performed".to_string()),
                    format!("{}: not performed", t.name),
                ),
                Some(v) if t.acceptance.accepts(v) => (
                    Ok(()),
                    format!(
                        "{}: {v} {} within [{:?}, {:?}]",
                        t.name, t.acceptance.metric, t.acceptance.minimum, t.acceptance.maximum
                    ),
                ),
                Some(v) => (
                    Err(format!(
                        "{v} {} outside [{:?}, {:?}]",
                        t.acceptance.metric, t.acceptance.minimum, t.acceptance.maximum
                    )),
                    format!("{}: FAILED ({v})", t.name),
                ),
            };
            if status.is_ok() {
                passed += 1;
            }
            results.push(TrialOutcome {
                trial_id: t.id,
                status,
                measured: t.measured,
                summary,
            });
        }
        let total = self.trials.len();
        TrialReport {
            results,
            all_passed: passed == total,
            passed_of: (passed, total),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program() -> TrialProgram {
        let mut p = TrialProgram::new();
        p.push(Trial {
            id: 1,
            name: "Speed trial".into(),
            kind: TrialKind::Speed,
            acceptance: Acceptance {
                metric: Metric::SpeedKn,
                minimum: Some(15.0),
                maximum: None,
            },
            measured: Some(15.6),
        });
        p.push(Trial {
            id: 2,
            name: "Turning circle".into(),
            kind: TrialKind::Maneuvering,
            acceptance: Acceptance {
                metric: Metric::TurningCircleLengths,
                minimum: None,
                maximum: Some(4.5),
            },
            measured: Some(4.1),
        });
        p.push(Trial {
            id: 3,
            name: "Cabin noise".into(),
            kind: TrialKind::NoiseVibration,
            acceptance: Acceptance {
                metric: Metric::CabinNoiseDb,
                minimum: None,
                maximum: Some(55.0),
            },
            measured: Some(52.0),
        });
        p
    }

    #[test]
    fn acceptance_windows_evaluate() {
        let report = program().evaluate();
        assert!(report.all_passed);
        assert_eq!(report.passed_of, (3, 3));
        assert!(report.results[0].status.is_ok());
    }

    #[test]
    fn failing_trial_is_reported() {
        let mut p = program();
        p.trials[0].measured = Some(14.2); // below 15 kn
        let report = p.evaluate();
        assert!(!report.all_passed);
        assert_eq!(report.passed_of, (2, 3));
        assert!(report.results[0]
            .status
            .as_ref()
            .err()
            .unwrap()
            .contains("outside"));
    }

    #[test]
    fn unperformed_trial_blocks_delivery() {
        let mut p = program();
        p.trials[2].measured = None;
        let report = p.evaluate();
        assert!(!report.all_passed);
        assert_eq!(
            report.results[2].status.clone(),
            Err("not performed".to_string())
        );
    }

    #[test]
    fn inclusive_bounds_hold() {
        let a = Acceptance {
            metric: Metric::SpeedKn,
            minimum: Some(15.0),
            maximum: Some(16.0),
        };
        assert!(a.accepts(15.0));
        assert!(a.accepts(16.0));
        assert!(!a.accepts(14.999));
        assert!(!a.accepts(16.001));
    }

    #[test]
    fn inclining_trial_gates_gm() {
        let mut p = TrialProgram::new();
        p.push(Trial {
            id: 9,
            name: "Inclining".into(),
            kind: TrialKind::Inclining,
            acceptance: Acceptance {
                metric: Metric::GmM,
                minimum: Some(0.15),
                maximum: None,
            },
            measured: Some(1.4),
        });
        assert!(p.evaluate().all_passed);
    }
}
