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
//!     lcg_from_midship_m: 0.0,         // balanced
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

/// A bottom ballast tank available during undocking (review 7H: ballast
/// sequencing). The tank centroid is taken on the keel line: filling it
/// adds mass at KG = 0 and at its longitudinal position.
#[derive(Debug, Clone, PartialEq)]
pub struct BallastTank {
    /// Tank designation (e.g. "FB WBT 3").
    pub name: String,
    /// Capacity, m³.
    pub volume_m3: f64,
    /// Tank centroid longitudinal position from midship, m (+ forward).
    pub x_from_midship_m: f64,
}

/// A vessel sitting on the dock blocks, awaiting float-out.
#[derive(Debug, Clone, PartialEq)]
pub struct DockedVessel {
    /// Launch weight (as-built at float-out), kg.
    pub launch_weight_kg: f64,
    /// Centre of gravity above keel, m.
    pub cog_above_keel_m: f64,
    /// Longitudinal centre of gravity from midship, m (+ forward). The
    /// keel-reaction distribution leans on this; 0.0 = perfectly balanced.
    pub lcg_from_midship_m: f64,
    /// Length between perpendiculars, m.
    pub length_m: f64,
    /// Moulded breadth, m.
    pub breadth_m: f64,
    /// Block coefficient at launch draft.
    pub block_coefficient: f64,
    /// Bottom ballast tanks available for the undocking.
    pub ballast_tanks: Vec<BallastTank>,
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
    /// The ballast tanks cannot deliver the target GM even when full.
    InsufficientBallast,
}

impl fmt::Display for DockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DockError::VesselDoesNotFit => {
                f.write_str("vessel dimensions or draft exceed the dock")
            }
            DockError::InvalidVessel => f.write_str("invalid vessel geometry or weight"),
            DockError::InvalidFloodRate => f.write_str("flood rate must be > 0"),
            DockError::InsufficientBallast => {
                f.write_str("ballast tanks cannot reach the target GM even when full")
            }
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
        // clearance → (dock depth available). ~12 steps minimum, with the
        // exact float-off instant (buoyancy = weight, keel reaction = 0)
        // always sampled — the grid on its own can straddle the one level
        // that decides whether the float-out is safe.
        let final_level = (floating_draft + 1.0).min(self.depth_m);
        let n_levels = 12;
        let mut levels: Vec<f64> = (0..=n_levels)
            .map(|i| final_level * i as f64 / n_levels as f64)
            .collect();
        if floating_draft < final_level && levels.iter().all(|&l| (l - floating_draft).abs() > 1e-9)
        {
            levels.push(floating_draft);
            levels.sort_by(f64::total_cmp);
        }
        let mut steps = Vec::with_capacity(levels.len());
        let mut stable_at_every_level = true;
        let mut min_gm = None;
        let mut total_hours = 0.0;

        for &level in &levels {
            let draft = self.draft_at(vessel, level);
            let displaced = displaced_mass_kg(vessel, draft);
            let aground = displaced < vessel.launch_weight_kg - 1.0;

            let (gm, stable, notes) = if aground {
                (
                    None,
                    true,
                    vec![format!(
                        "aground: keel blocks carry {:.0} t (independent heeling is restrained by the keel line; stability is judged at the float-off instant)",
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
/// `GM = KB + BM − KG` with `KB = d/2` and the fineness-corrected
/// `BM = Cwp·B²/(12·Cb·d)`, where the waterplane coefficient is the
/// standard approximation `Cwp ≈ (1 + 2·Cb)/3`. (The plain box form
/// `B²/(12·d)` ignores the block coefficient and overstates BM for full
/// hull forms.)
/// Virtual GM while the vessel rests on the keel blocks at touchdown
/// (review 7H roadmap item — the "virtual GM" of grounding/docking).
///
/// When the keel first touches, an upward ground reaction `P` at the keel
/// line effectively stiffens the vessel: any small heel lifts one side off
/// the blocks and the reaction redistributes, acting like a righting
/// couple. The classical docking approximation raises GM by
/// `ΔGM = P·KM / W` (the reaction applied at the keel line acts at the
/// metacentre arm). Equivalently the vessel is *more* stable on the blocks
/// than afloat at the same draft — the danger is the **instant of lifting
/// OFF**, where P → 0 and the virtual GM collapses to the free-floating
/// GM. `P` here is the keel-block share of the weight at the given draft
/// (weight minus buoyancy), floored at zero.
pub fn virtual_gm_touchdown_m(vessel: &DockedVessel, draft_m: f64) -> f64 {
    let hs_weight_kn = vessel.launch_weight_kg * G_ACC / 1000.0;
    let buoyancy_kn = displaced_mass_kg(vessel, draft_m) * G_ACC / 1000.0;
    let reaction_kn = (hs_weight_kn - buoyancy_kn).max(0.0);
    let gm_afloat = gm_m(vessel, draft_m);
    if reaction_kn <= 0.0 || hs_weight_kn <= 0.0 {
        return gm_afloat; // afloat: no virtual rise
    }
    // KM at this draft (GM + KG).
    let km = gm_afloat + vessel.cog_above_keel_m;
    let share = reaction_kn / hs_weight_kn;
    gm_afloat + share * km
}

const G_ACC: f64 = 9.81;

/// One planned ballast movement (review 7H: ballast sequencing).
#[derive(Debug, Clone, PartialEq)]
pub struct BallastStep {
    /// The tank to fill.
    pub tank: String,
    /// Target fill fraction at the end of the step, 0..=1.
    pub fill_frac: f64,
    /// Dock water level at which the fill happens, m. Pre-flood fills sit
    /// at level 0: the keel blocks still carry the full reaction and the
    /// ballast goes in while the bottom is dry.
    pub at_water_level_m: f64,
}

/// The planned ballast outcome: masses to put in before flooding, and the
/// resulting float-off condition.
#[derive(Debug, Clone, PartialEq)]
pub struct BallastPlan {
    /// Fill movements in execution order (all pre-flood in this model).
    pub steps: Vec<BallastStep>,
    /// Total ballast mass the plan puts aboard, kg.
    pub ballast_mass_kg: f64,
    /// Float-off draft with the ballast aboard, m.
    pub float_off_draft_m: f64,
    /// GM at float-off with the ballast aboard, m.
    pub float_off_gm_m: f64,
    /// Findings.
    pub notes: Vec<String>,
}

/// Plans the undocking ballast (review 7H: ballast sequencing).
///
/// The vessel is screened first without ballast: if the float-off GM
/// already reaches `target_gm_m` (default [`MIN_GM_M`]), no ballast is
/// needed. Otherwise bottom tanks are filled in equal increments — keel-
/// line mass lowers the centre of gravity without touching KM — until the
/// float-off GM reaches the target or the tanks run out
/// ([`DockError::InsufficientBallast`]). Filling happens before flooding
/// begins (level 0): while the keel blocks carry the boat, extra mass
/// only loads the blocks; once afloat the ballast is why the boat is
/// stable.
///
/// `target_gm_m = None` means [`MIN_GM_M`].
///
/// # Errors
///
/// [`DockError::InvalidVessel`] on a malformed vessel or tank set, and
/// [`DockError::InsufficientBallast`] when even full tanks cannot reach
/// the target GM.
pub fn ballast_plan(
    vessel: &DockedVessel,
    target_gm_m: Option<f64>,
) -> Result<BallastPlan, DockError> {
    let target = target_gm_m.unwrap_or(MIN_GM_M);
    if vessel.launch_weight_kg <= 0.0
        || vessel.length_m <= 0.0
        || vessel.breadth_m <= 0.0
        || vessel.block_coefficient <= 0.0
    {
        return Err(DockError::InvalidVessel);
    }
    if vessel
        .ballast_tanks
        .iter()
        .any(|t| !(t.volume_m3.is_finite() && t.volume_m3 >= 0.0))
    {
        return Err(DockError::InvalidVessel);
    }

    // Float-off condition for a given ballast mass (all keel-line).
    let float_off = |ballast_kg: f64| -> (f64, f64) {
        let w = vessel.launch_weight_kg + ballast_kg;
        let draft = w / (RHO_SEA * vessel.block_coefficient * vessel.length_m * vessel.breadth_m);
        let kb = draft / 2.0;
        let cwp = (1.0 + 2.0 * vessel.block_coefficient) / 3.0;
        let bm = cwp * vessel.breadth_m * vessel.breadth_m
            / (12.0 * vessel.block_coefficient * draft.max(1e-6));
        let kg = vessel.launch_weight_kg * vessel.cog_above_keel_m / w;
        (draft, kb + bm - kg)
    };

    let (d0, gm0) = float_off(0.0);
    let mut notes = vec![format!(
        "unballasted float-off at draft {d0:.2} m: GM {gm0:.2} m vs target {target:.2} m"
    )];
    if gm0 >= target {
        notes.push("no ballast required".into());
        return Ok(BallastPlan {
            steps: Vec::new(),
            ballast_mass_kg: 0.0,
            float_off_draft_m: d0,
            float_off_gm_m: gm0,
            notes,
        });
    }

    // Greedy equal fills: every tank gains the same increment until the
    // target GM is met (keeps the LCG untouched with a symmetric set, and
    // is simple to execute).
    const STEP: f64 = 0.05;
    let mut fills = vec![0.0_f64; vessel.ballast_tanks.len()];
    loop {
        for f in &mut fills {
            *f = (*f + STEP).min(1.0);
        }
        let mass: f64 = vessel
            .ballast_tanks
            .iter()
            .zip(&fills)
            .map(|(t, f)| RHO_SEA * t.volume_m3 * f)
            .sum();
        let (draft, gm) = float_off(mass);
        let all_full = fills.iter().all(|f| *f >= 1.0);
        if gm >= target || all_full {
            if gm < target {
                return Err(DockError::InsufficientBallast);
            }
            let steps = vessel
                .ballast_tanks
                .iter()
                .zip(&fills)
                .filter(|(_, f)| **f > 0.0)
                .map(|(t, f)| BallastStep {
                    tank: t.name.clone(),
                    fill_frac: *f,
                    at_water_level_m: 0.0,
                })
                .collect();
            notes.push(format!(
                "fill {:.0} t of keel-line ballast: float-off draft {draft:.2} m, GM {gm:.2} m",
                mass / 1000.0
            ));
            return Ok(BallastPlan {
                steps,
                ballast_mass_kg: mass,
                float_off_draft_m: draft,
                float_off_gm_m: gm,
                notes,
            });
        }
    }
}

/// Keel-block reactions for one water level (review 7H: keel-block
/// reaction distribution). The total aground reaction is the ballasted
/// weight minus buoyancy; its longitudinal centre follows from the moment
/// balance about midship (`x_R = W·LCG / R`, the prismatic displacement
/// centroid sitting at midship). The total is spread over `n_blocks`
/// evenly spaced blocks by a linear pressure law, which goes negative at
/// one row end once the reaction centroid passes L/3 from midship — the
/// classical single-end lift-off; `lifted_off_ends` flags it and the
/// pressures are clamped at zero and renormalised.
#[derive(Debug, Clone, PartialEq)]
pub struct KeelReactions {
    /// Total reaction the blocks carry, kN (0 when afloat).
    pub total_kn: f64,
    /// Longitudinal reaction centroid from midship, m.
    pub centroid_from_midship_m: f64,
    /// Per-block reactions, kN, from the aft end of the row.
    pub per_block_kn: Vec<f64>,
    /// True when the linear law wanted negative end pressures (the row is
    /// pivoting about one end).
    pub lifted_off_ends: bool,
}

/// Computes the keel-block reaction distribution — see [`KeelReactions`].
/// `fills` are the fill fractions matching `vessel.ballast_tanks`.
pub fn keel_reaction_distribution(
    vessel: &DockedVessel,
    fills: &[f64],
    water_level_m: f64,
    n_blocks: usize,
) -> KeelReactions {
    let ballast: f64 = vessel
        .ballast_tanks
        .iter()
        .zip(fills.iter().chain(std::iter::repeat(&0.0)))
        .map(|(t, f)| RHO_SEA * t.volume_m3 * f)
        .sum();
    let w = vessel.launch_weight_kg + ballast;
    // While aground the keel sits 1 m over the dock floor on the blocks;
    // the immersed draft is the water level over the keel line, capped by
    // the equilibrium draft (a level above float-off cannot immerse more).
    let floating = w / (RHO_SEA * vessel.block_coefficient * vessel.length_m * vessel.breadth_m);
    let draft = (water_level_m - 1.0).min(floating).max(0.0);
    let displaced = displaced_mass_kg(vessel, draft).min(w);
    let r_kn = (w - displaced) * G_ACC / 1000.0;
    if r_kn <= 1e-9 || n_blocks == 0 {
        return KeelReactions {
            total_kn: 0.0,
            centroid_from_midship_m: 0.0,
            per_block_kn: vec![0.0; n_blocks],
            lifted_off_ends: false,
        };
    }
    // Moment balance about midship (buoyancy centroid at 0 in the
    // prismatic model).
    let x_r = w * vessel.lcg_from_midship_m / (w - displaced);
    let half = vessel.length_m / 2.0;
    let e = (x_r / half).clamp(-1.0, 1.0);
    // p(-1) = R/2 (1 - 3e) goes negative once |x_R| > L/6.
    let lifted = x_r.abs() > half / 3.0;
    // Linear pressure over the row: R_i = R/n * (1 + 3 e xi), xi in
    // [-1, 1] from aft (-1) to forward (+1). (Continuum moment balance:
    // int p xi dxi = 2 beta/3 = R e, so beta = 1.5 R e and the midpoint
    // load on block i is p(xi) * 2/n.)
    let n = n_blocks as f64;
    let mut per: Vec<f64> = (0..n_blocks)
        .map(|i| {
            let xi = 2.0 * (i as f64 + 0.5) / n - 1.0;
            (r_kn / n) * (1.0 + 3.0 * e * xi)
        })
        .collect();
    let wanted_negative = per.iter().any(|p| *p < 0.0);
    if wanted_negative {
        let positive: f64 = per.iter().map(|p| p.max(0.0)).sum();
        if positive > 1e-9 {
            for p in &mut per {
                *p = p.max(0.0) * (r_kn / positive);
            }
        }
    }
    let centroid: f64 = per
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let xi = 2.0 * (i as f64 + 0.5) / n - 1.0;
            p * xi * half
        })
        .sum::<f64>()
        / r_kn;
    KeelReactions {
        total_kn: r_kn,
        centroid_from_midship_m: centroid,
        per_block_kn: per,
        lifted_off_ends: lifted,
    }
}

/// Metacentric height GM at draft `d`, m:
/// `GM = KB + BM − KG` with `KB = d/2` and the fineness-corrected
/// `BM = Cwp·B²/(12·Cb·d)`, where the waterplane coefficient is the
/// standard approximation `Cwp ≈ (1 + 2·Cb)/3`. (The plain box form
/// `B²/(12·d)` ignores the block coefficient and overstates BM for full
/// hull forms.)
pub fn gm_m(vessel: &DockedVessel, draft_m: f64) -> f64 {
    let d = draft_m.max(1e-6);
    let kb = d / 2.0;
    let cwp = (1.0 + 2.0 * vessel.block_coefficient) / 3.0;
    let bm = cwp * vessel.breadth_m * vessel.breadth_m / (12.0 * vessel.block_coefficient * d);
    kb + bm - vessel.cog_above_keel_m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn barge() -> DockedVessel {
        DockedVessel {
            launch_weight_kg: 6_000_000.0,
            cog_above_keel_m: 5.5,
            lcg_from_midship_m: 0.0,
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

    /// Verification (review 7H): virtual GM at touchdown exceeds the
    /// afloat GM, scales with the block reaction share, and collapses to
    /// the afloat GM exactly at float-off.
    #[test]
    fn virtual_gm_stiffens_on_the_blocks() {
        let v = barge();
        // Touchdown-ish: barely aground (draft a hair below floating draft).
        let float_draft = floating_draft_m(&v);
        let aground_draft = float_draft * 0.5;
        let virtual_gm = virtual_gm_touchdown_m(&v, aground_draft);
        let afloat_gm = gm_m(&v, aground_draft);
        assert!(
            virtual_gm > afloat_gm,
            "virtual {virtual_gm} must exceed afloat {afloat_gm}"
        );
        // At half-float draft the reaction carries ~half the weight, so
        // the rise is ~0.5 x KM.
        let km = afloat_gm + v.cog_above_keel_m;
        let expected_rise = 0.5 * km;
        assert!(
            (virtual_gm - afloat_gm - expected_rise).abs() < 0.1,
            "rise {} vs expected {expected_rise}",
            virtual_gm - afloat_gm
        );
        // Deeper aground = stiffer still.
        let deep = virtual_gm_touchdown_m(&v, float_draft * 0.1);
        assert!(deep > virtual_gm);
        // At (or past) float-off: identical to the afloat GM.
        let off = virtual_gm_touchdown_m(&v, float_draft * 1.1);
        assert!((off - gm_m(&v, float_draft * 1.1)).abs() < 1e-9);
    }

    #[test]
    fn sequence_floods_to_afloat() {
        let seq = dock().flooding_sequence(&barge(), 5_000.0).unwrap();
        // 12-level grid + level 0 + the exact float-off instant.
        assert_eq!(seq.steps.len(), 14);
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
        // The float-off instant (water level = floating draft) is sampled
        // explicitly: buoyancy carries the full weight, keel reaction gone.
        let float_off = seq
            .steps
            .iter()
            .find(|s| (s.water_level_m - floating_draft_m(&barge())).abs() < 1e-9)
            .expect("float-off instant must be sampled");
        assert!((float_off.displaced_mass_kg - 6_000_000.0).abs() < 1.0);
        assert!(float_off.gm_m.is_some(), "GM judged at float-off");
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

    /// Review 7H: ballast sequencing — a tender vessel (KG so high the
    /// unballasted float-off GM is negative) is stabilised by keel-line
    /// ballast, planned before flooding begins.
    #[test]
    fn ballast_plan_stabilises_a_tender_vessel() {
        // 4000 t, KG 16 m on a 90 x 20 x Cb 0.8 hull: draft 2.71 m,
        // KM 14.68 m -> GM = -1.32 m, well below the 0.15 m minimum.
        let mut vessel = barge();
        vessel.launch_weight_kg = 4_000_000.0;
        vessel.cog_above_keel_m = 16.0;
        vessel.length_m = 90.0;
        vessel.breadth_m = 20.0;
        vessel.block_coefficient = 0.8;
        vessel.ballast_tanks = vec![
            BallastTank {
                name: "AP WBT".into(),
                volume_m3: 1000.0,
                x_from_midship_m: -35.0,
            },
            BallastTank {
                name: "FP WBT".into(),
                volume_m3: 1000.0,
                x_from_midship_m: 35.0,
            },
        ];

        // Unballasted, the undocking is unstable (screen with the public
        // hydrostatics).
        let draft0 = floating_draft_m(&vessel);
        assert!(gm_m(&vessel, draft0) < MIN_GM_M);

        // The plan fills the tanks and reaches the target GM.
        let plan = ballast_plan(&vessel, None).unwrap();
        assert!(!plan.steps.is_empty());
        assert_eq!(plan.steps.len(), 2, "both tanks used");
        for step in &plan.steps {
            assert_eq!(step.at_water_level_m, 0.0, "ballast goes in pre-flood");
            assert!((0.0..=1.0).contains(&step.fill_frac));
        }
        assert!(
            plan.float_off_gm_m >= MIN_GM_M,
            "GM {:.2}",
            plan.float_off_gm_m
        );
        assert!(plan.ballast_mass_kg > 0.0);
        // Symmetric fills keep the LCG at midship (both tanks equal fill).
        let fills: Vec<f64> = plan.steps.iter().map(|s| s.fill_frac).collect();
        assert!((fills[0] - fills[1]).abs() < 1e-9);

        // A target the tanks cannot reach is refused, not faked.
        assert_eq!(
            ballast_plan(&vessel, Some(5.0)),
            Err(DockError::InsufficientBallast)
        );

        // With the ballast aboard, the flooding sequence is stable at
        // every level including float-off.
        let ballasted_w = vessel.launch_weight_kg + plan.ballast_mass_kg;
        let mut ballasted = vessel.clone();
        ballasted.launch_weight_kg = ballasted_w;
        ballasted.cog_above_keel_m = 4_000_000.0 * 16.0 / ballasted_w;
        ballasted.ballast_tanks = vec![];
        let dock = Drydock {
            length_m: 200.0,
            width_m: 30.0,
            depth_m: 10.0,
        };
        let seq = dock.flooding_sequence(&ballasted, 5_000.0).unwrap();
        assert!(seq.stable_at_every_level);
        assert!((seq.steps.last().unwrap().gm_m.unwrap() - plan.float_off_gm_m).abs() < 1e-6);
    }

    /// Review 7H: keel-block reaction distribution — total from
    /// Archimedes, centroid from the moment balance, linear spread over
    /// the row, single-end lift-off flagged past L/3.
    #[test]
    fn keel_reactions_distribute_and_flag_pivot() {
        let mut vessel = barge(); // 6000 t, L 140, balanced
        vessel.lcg_from_midship_m = 5.0;

        // At level 0 (dry bottom) the blocks carry everything.
        let r = keel_reaction_distribution(&vessel, &[], 0.0, 9);
        assert!((r.total_kn - 6_000_000.0 * 9.81 / 1000.0).abs() < 1e-6);
        let sum: f64 = r.per_block_kn.iter().sum();
        assert!(
            (sum - r.total_kn).abs() < 1e-6,
            "reactions must sum to the total"
        );
        // Midpoint discretization over n=9 blocks gives the exact lever
        // times (1 - 1/n^2): 4.938 m vs the continuum 5.0 m.
        assert!(
            (r.centroid_from_midship_m - 5.0 * (1.0 - 1.0 / 81.0)).abs() < 1e-9,
            "{}",
            r.centroid_from_midship_m
        );
        assert!(r.per_block_kn.iter().all(|p| *p > 0.0));
        assert!(!r.lifted_off_ends);
        // Forward-heavy: the forward blocks carry more.
        assert!(*r.per_block_kn.last().unwrap() > *r.per_block_kn.first().unwrap());

        // Afloat (level past float-off): nothing on the blocks.
        let afloat = keel_reaction_distribution(&vessel, &[], 9.0, 9);
        assert_eq!(afloat.total_kn, 0.0);

        // CoG 50 m from midship on a 70 m half-length: x_R = W*50/W = 50
        // > L/3 = 46.7 -> the linear law wants a negative end pressure.
        let mut eccentric = vessel.clone();
        eccentric.lcg_from_midship_m = 50.0;
        let pivot = keel_reaction_distribution(&eccentric, &[], 0.0, 9);
        assert!(pivot.lifted_off_ends);
        assert!(pivot.per_block_kn.iter().all(|p| *p >= 0.0), "clamped");
        let sum: f64 = pivot.per_block_kn.iter().sum();
        assert!(
            (sum - pivot.total_kn).abs() < 1e-6,
            "clamped set still sums"
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
