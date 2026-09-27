//! Drydock flooding and ballast sequencing with stability at every water
//! level.
//!
//! Flooding a building dock is a controlled stability exercise: as the water
//! rises, the vessel takes load off the keel blocks, starts to float, and
//! must be upright and stable at *every* intermediate level before the
//! caisson opens. [`Drydock::flooding_sequence`] computes the level-by-level
//! state (draft, displaced mass, metacentric height) and flags any unstable
//! step.
//!
//! # Example
//!
//! ```
//! use tpt_yard_drydock::{DockedVessel, Drydock};
//!
//! let dock = Drydock { length_m: 200.0, width_m: 30.0, depth_m: 10.0 };
//! let vessel = DockedVessel {
//!     launch_weight_kg: 6_000_000.0,   // 6,000 t at launch
//!     cog_above_keel_m: 6.0,           // KG
//!     length_m: 140.0,
//!     breadth_m: 22.0,
//!     block_coefficient: 0.75,
//!     ballast_tanks: vec![],
//! };
//! let seq = dock.flooding_sequence(&vessel, 5_000.0).unwrap(); // m³/h
//! assert!(seq.steps.len() > 3);
//! assert!(seq.stable_at_every_level);
//! ```
#![allow(clippy::doc_markdown)]

use std::fmt;

/// The building dock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drydock {
    /// Dock length, m.
    pub length_m: f64,
    /// Dock width, m.
    pub width_m: f64,
    /// Dock depth (caisson sill to cope), m.
    pub depth_m: f64,
}

/// A vessel sitting on the dock blocks, awaiting float-out.
#[derive(Debug, Clone, PartialEq)]
pub struct DockedVessel {
    /// Launch weight (as-built at float-out), kg.
    pub launch_weight_kg: f64,
    /// Centre of gravity above keel, m.
    pub cog_above_keel_m: f64,
    /// Length between perpendiculars, m.
    pub length_m: f64,
    /// Moulded breadth, m.
    pub breadth_m: f64,
    /// Block coefficient at launch draft.
    pub block_coefficient: f64,
    /// Ballast tank capacities, m³ (reserved for ballast sequencing).
    pub ballast_tanks: Vec<f64>,
}

/// State of dock + vessel at one water level.
#[derive(Debug, Clone, PartialEq)]
pub struct FloodingStep {
    /// Water level above the dock floor, m.
    pub water_level_m: f64,
    /// Vessel draft at this level, m (0 while aground).
    pub draft_m: f64,
    /// Mass supported by buoyancy, kg (0 while aground).
    pub displaced_mass_kg: f64,
    /// Metacentric height GM, m (`None` while aground).
    pub gm_m: Option<f64>,
    /// Hours of flooding from the previous step at the given rate.
    pub elapsed_hours: f64,
    /// True if the vessel is stable (or still aground) at this level.
    pub stable: bool,
    /// Findings for this level.
    pub notes: Vec<String>,
}

/// The planned flooding sequence.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FloodingSequence {
    /// One step per water level (bottom-up, final step = vessel afloat).
    pub steps: Vec<FloodingStep>,
    /// Total flooding time, hours.
    pub total_time_hours: f64,
    /// Every level is either aground or GM ≥ minimum.
    pub stable_at_every_level: bool,
    /// Minimum GM encountered once afloat, m.
    pub min_gm_m: Option<f64>,
}

/// Errors from docking computations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockError {
    /// The vessel cannot float inside this dock (too large, or the dock is
    /// too shallow for the launch draft).
    VesselDoesNotFit,
    /// Launch weight or geometry must be positive.
    InvalidVessel,
    /// Flood rate must be positive.
    InvalidFloodRate,
}

impl fmt::Display for DockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DockError::VesselDoesNotFit => {
                f.write_str("vessel dimensions or draft exceed the dock")
            }
            DockError::InvalidVessel => f.write_str("invalid vessel geometry or weight"),
            DockError::InvalidFloodRate => f.write_str("flood rate must be > 0"),
        }
    }
}

impl std::error::Error for DockError {}

const RHO_SEA: f64 = 1025.0; // kg/m³
/// Minimum GM once afloat (class screening value), m.
pub const MIN_GM_M: f64 = 0.15;

impl Drydock {
    /// Water volume in the dock at a given level, m³.
    pub fn water_volume_m3(&self, level_m: f64) -> f64 {
        self.length_m * self.width_m * level_m.min(self.depth_m).max(0.0)
    }

    /// Vessel draft when the dock water stands at `level_m` above the dock
    /// floor (keel assumed at the floor; docking blocks neglected).
    ///
    /// While aground, draft = level. Afloat, the draft solves the
    /// displacement equation for the launch weight:
    /// `d = W / (ρ · C_b · L · B)`.
    fn draft_at(&self, vessel: &DockedVessel, level_m: f64) -> f64 {
        let floating_draft = floating_draft_m(vessel);
        level_m.min(floating_draft)
    }

    /// Computes the level-by-level flooding sequence.
    ///
    /// # Errors
    ///
    /// [`DockError`] for invalid inputs or a vessel that cannot float out.
    pub fn flooding_sequence(
        &self,
        vessel: &DockedVessel,
        flood_rate_m3_hr: f64,
    ) -> Result<FloodingSequence, DockError> {
        if flood_rate_m3_hr <= 0.0 {
            return Err(DockError::InvalidFloodRate);
        }
        if vessel.launch_weight_kg <= 0.0
            || vessel.length_m <= 0.0
            || vessel.breadth_m <= 0.0
            || vessel.block_coefficient <= 0.0
        {
            return Err(DockError::InvalidVessel);
        }
        let floating_draft = floating_draft_m(vessel);
        if vessel.breadth_m > self.width_m
            || vessel.length_m > self.length_m
            || floating_draft + 0.5 > self.depth_m
        {
            return Err(DockError::VesselDoesNotFit);
        }

        // Level schedule: 0 → float-off level → floating_draft + under-keel
        // clearance → (dock depth available). ~12 steps minimum.
        let final_level = (floating_draft + 1.0).min(self.depth_m);
        let n_levels = 12;
        let mut steps = Vec::with_capacity(n_levels + 1);
        let mut stable_at_every_level = true;
        let mut min_gm = None;
        let mut total_hours = 0.0;

        for i in 0..=n_levels {
            let level = final_level * i as f64 / n_levels as f64;
            let draft = self.draft_at(vessel, level);
            let displaced = displaced_mass_kg(vessel, draft);
            let aground = displaced < vessel.launch_weight_kg - 1.0;

            let (gm, stable, notes) = if aground {
                (
                    None,
                    true,
                    vec![format!(
                        "aground: keel blocks carry {:.0} t",
                        (vessel.launch_weight_kg - displaced) / 1000.0
                    )],
                )
            } else {
                let gm = gm_m(vessel, draft);
                let stable = gm >= MIN_GM_M;
                let notes = if stable {
                    vec![format!("afloat, GM {gm:.2} m ≥ {MIN_GM_M} m")]
                } else {
                    vec![format!("UNSTABLE: GM {gm:.2} m below {MIN_GM_M} m")]
                };
                min_gm = Some(min_gm.map_or(gm, |m: f64| m.min(gm)));
                (Some(gm), stable, notes)
            };
            if !stable {
                stable_at_every_level = false;
            }

            let volume = self.water_volume_m3(level);
            let hours = volume / flood_rate_m3_hr;
            let elapsed = hours - total_hours;
            total_hours = hours;
            steps.push(FloodingStep {
                water_level_m: level,
                draft_m: draft,
                displaced_mass_kg: displaced,
                gm_m: gm,
                elapsed_hours: elapsed.max(0.0),
                stable,
                notes,
            });
        }

        Ok(FloodingSequence {
            steps,
            total_time_hours: total_hours,
            stable_at_every_level,
            min_gm_m: min_gm,
        })
    }
}

/// Draft at which the vessel floats at its launch weight, m.
pub fn floating_draft_m(vessel: &DockedVessel) -> f64 {
    vessel.launch_weight_kg
        / (RHO_SEA * vessel.block_coefficient * vessel.length_m * vessel.breadth_m)
}

/// Displacement at a given draft, kg (rectangular block approximation).
pub fn displaced_mass_kg(vessel: &DockedVessel, draft_m: f64) -> f64 {
    RHO_SEA * vessel.block_coefficient * vessel.length_m * vessel.breadth_m * draft_m.max(0.0)
}

/// Metacentric height GM at draft `d`, m:
/// `GM = KB + BM − KG` with `KB = d/2`, `BM = B²/(12·d)`
/// (rectangular waterplane block approximation).
pub fn gm_m(vessel: &DockedVessel, draft_m: f64) -> f64 {
    let d = draft_m.max(1e-6);
    let kb = d / 2.0;
    let bm = vessel.breadth_m * vessel.breadth_m / (12.0 * d);
    kb + bm - vessel.cog_above_keel_m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn barge() -> DockedVessel {
        DockedVessel {
            launch_weight_kg: 6_000_000.0,
            cog_above_keel_m: 5.5,
            length_m: 140.0,
            breadth_m: 22.0,
            block_coefficient: 0.85,
            ballast_tanks: vec![],
        }
    }

    fn dock() -> Drydock {
        Drydock {
            length_m: 200.0,
            width_m: 30.0,
            depth_m: 10.0,
        }
    }

    #[test]
    fn floating_draft_matches_displacement() {
        let v = barge();
        let d = floating_draft_m(&v);
        // 6,000,000 / (1025 · 0.85 · 140 · 22) = 2.236 m
        assert!((d - 2.236).abs() < 0.001, "draft {d}");
        assert!((displaced_mass_kg(&v, d) - v.launch_weight_kg).abs() < 1.0);
    }

    #[test]
    fn gm_of_a_beamy_barge_is_positive() {
        let v = barge();
        let d = floating_draft_m(&v);
        let gm = gm_m(&v, d);
        assert!(gm > 1.0, "barge GM {gm} m");
    }

    #[test]
    fn tall_cog_capsizes_at_dock() {
        let mut v = barge();
        v.cog_above_keel_m = 25.0; // above the metacentre
        let d = floating_draft_m(&v);
        assert!(gm_m(&v, d) < 0.0);
    }

    #[test]
    fn sequence_floods_to_afloat() {
        let seq = dock().flooding_sequence(&barge(), 5_000.0).unwrap();
        assert_eq!(seq.steps.len(), 13);
        assert!(seq.stable_at_every_level);
        // Early steps aground, final step afloat with positive GM.
        assert!(seq.steps.first().unwrap().gm_m.is_none());
        let last = seq.steps.last().unwrap();
        assert!(last.gm_m.unwrap() > MIN_GM_M);
        assert!((last.displaced_mass_kg - 6_000_000.0).abs() < 1.0);
        assert!(seq.total_time_hours > 0.0);
        // Elapsed hours are consistent with the dock volume.
        let final_volume = dock().water_volume_m3(seq.steps.last().unwrap().water_level_m);
        assert!((seq.total_time_hours - final_volume / 5_000.0).abs() < 1e-9);
    }

    #[test]
    fn vessel_too_large_is_rejected() {
        let mut v = barge();
        v.breadth_m = 40.0; // wider than the dock
        assert_eq!(
            dock().flooding_sequence(&v, 5_000.0),
            Err(DockError::VesselDoesNotFit)
        );
        let deep = DockedVessel {
            launch_weight_kg: 60_000_000.0,
            ..barge()
        };
        // Draft ~22 m > dock depth → does not fit.
        assert_eq!(
            dock().flooding_sequence(&deep, 5_000.0),
            Err(DockError::VesselDoesNotFit)
        );
    }

    #[test]
    fn invalid_inputs_rejected() {
        assert_eq!(
            dock().flooding_sequence(&barge(), 0.0),
            Err(DockError::InvalidFloodRate)
        );
        let mut v = barge();
        v.launch_weight_kg = 0.0;
        assert_eq!(
            dock().flooding_sequence(&v, 100.0),
            Err(DockError::InvalidVessel)
        );
    }

    #[test]
    fn water_volume_saturates_at_depth() {
        let d = dock();
        assert!((d.water_volume_m3(5.0) - 200.0 * 30.0 * 5.0).abs() < 1e-9);
        assert!((d.water_volume_m3(99.0) - 200.0 * 30.0 * 10.0).abs() < 1e-9);
    }
}
