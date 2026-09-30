//! Hull construction and block management for sea shipyards.
//!
//! [`HullConstruction::block_division`] divides the hull into erection
//! blocks under crane and workshop constraints; [`HullConstruction::
//! erection_sequence`] orders the block joins bottom-tier-first, from
//! midship outwards — the classic stability-first erection doctrine (see
//! RFC 0002 for the sequencing model).
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::Dimensions;
//! use tpt_yard_hull::{HullConstruction, HullGeometry};
//!
//! let hull = HullConstruction::new(HullGeometry {
//!     loa_m: 140.0,
//!     boa_m: 22.0,
//!     depth_m: 12.0,
//!     areal_density_kg_m2: 180.0,
//!     depth_bands: 2,
//! });
//!
//! let blocks = hull.block_division(
//!     40_000.0,                  // crane capacity, kN
//!     Dimensions::new(24.0, 30.0, 14.0), // workshop
//! );
//! assert!(!blocks.is_empty());
//! // Every block must be liftable and must fit the workshop.
//! assert!(blocks.iter().all(|b| b.weight_kn() <= 40_000.0));
//!
//! let joins = hull.erection_sequence(&blocks);
//! // Bottom tier goes in first.
//! assert!(joins.windows(2).all(|w| w[0].z_band <= w[1].z_band));
//! ```

use std::fmt;

use tpt_yard_core::{BlockId, Dimensions, Vector3};

/// Principal particulars and steel density of the hull form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HullGeometry {
    /// Length overall, m.
    pub loa_m: f64,
    /// Breadth (moulded), m.
    pub boa_m: f64,
    /// Depth to main deck, m.
    pub depth_m: f64,
    /// Average areal steel density of the moulded surface, kg/m²
    /// (hull envelope + typical structure; 150–250 for merchant ships).
    pub areal_density_kg_m2: f64,
    /// Number of vertical tiers the depth is split into.
    pub depth_bands: u32,
}

/// Fabrication status of a hull block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStatus {
    /// In design, not yet planned for production.
    Design,
    /// Steel cut.
    Cutting,
    /// Sub-assembly in the workshop.
    Assembly,
    /// Block welding in progress.
    Welding,
    /// Outfitting installed.
    Outfitting,
    /// Painted / primed.
    Painted,
    /// Staged for erection.
    ReadyForErection,
    /// Erected in the dock / on the ways.
    Erected,
}

impl BlockStatus {
    /// True once the block is staged or built into the hull.
    pub fn is_erection_ready(self) -> bool {
        matches!(self, BlockStatus::ReadyForErection | BlockStatus::Erected)
    }
}

impl fmt::Display for BlockStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BlockStatus::Design => "design",
            BlockStatus::Cutting => "cutting",
            BlockStatus::Assembly => "assembly",
            BlockStatus::Welding => "welding",
            BlockStatus::Outfitting => "outfitting",
            BlockStatus::Painted => "painted",
            BlockStatus::ReadyForErection => "ready",
            BlockStatus::Erected => "erected",
        };
        f.write_str(s)
    }
}

/// One erection block.
#[derive(Debug, Clone, PartialEq)]
pub struct HullBlock {
    /// Block identifier.
    pub id: BlockId,
    /// Name (yard convention, e.g. "B212").
    pub name: String,
    /// Block geometry (bounding mesh in hull coordinates).
    pub geometry: Geometry,
    /// Position of the block centre in hull coordinates, m.
    pub cog: Vector3,
    /// Design weight, kg.
    pub weight_kg: f64,
    /// Longitudinal band index (0 = aft).
    pub x_band: u32,
    /// Vertical tier index (0 = keel tier).
    pub z_band: u32,
    /// Fabrication status.
    pub status: BlockStatus,
}

impl HullBlock {
    /// Block weight in kN (gravity 9.81).
    pub fn weight_kn(&self) -> f64 {
        self.weight_kg * 9.81 / 1000.0
    }

    /// Block bounding dimensions, m.
    pub fn dimensions(&self) -> Dimensions {
        self.geometry.dimensions
    }
}

/// Bounding geometry of a block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    /// Centre position, m.
    pub centre: Vector3,
    /// Full extents, m.
    pub dimensions: Dimensions,
}

/// A planned join between two blocks (or the dock bottom for first-tier
/// blocks).
#[derive(Debug, Clone, PartialEq)]
pub struct BlockJoin {
    /// The block being erected.
    pub block: BlockId,
    /// The block it lands against (`None` for the first block of a tier on
    /// the dock floor).
    pub onto: Option<BlockId>,
    /// The seam type between the blocks.
    pub seam: SeamType,
    /// Position in the erection sequence (0-based).
    pub sequence_index: usize,
    /// Vertical tier of the erected block.
    pub z_band: u32,
}

/// Seam between adjoining blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeamType {
    /// Butt seam (transverse/longitudinal plate joint).
    ButtSeam,
    /// Fillet seam ( girder to plate).
    FilletSeam,
    /// Dock joint on the keel blocks (first tier).
    DockJoint,
}

/// The hull construction: form geometry, blocks, and the join plan.
#[derive(Debug, Clone, PartialEq)]
pub struct HullConstruction {
    /// Principal particulars.
    pub hull_form: HullGeometry,
    /// Blocks (filled by [`HullConstruction::block_division`]).
    pub blocks: Vec<HullBlock>,
    /// The planned join sequence (filled by
    /// [`HullConstruction::erection_sequence`]).
    pub block_join_sequence: Vec<BlockJoin>,
}

impl HullConstruction {
    /// Creates an empty construction for the given hull form.
    pub fn new(hull_form: HullGeometry) -> Self {
        Self {
            hull_form,
            blocks: Vec::new(),
            block_join_sequence: Vec::new(),
        }
    }

    /// Divides the hull into erection blocks.
    ///
    /// Constraints (all verified by tests):
    /// - block weight ≤ crane capacity (`crane_capacity_kn` at 100 %, minus
    ///   a 10 % rigging allowance);
    /// - block length ≤ workshop length, block height ≤ workshop depth, and
    ///   the moulded breadth fits the workshop width (division is
    ///   longitudinal only — a too narrow workshop means the block cannot
    ///   be built, which the caller must catch).
    /// - the depth is split into `depth_bands` tiers of equal height
    ///   (`depth_bands == 0` yields no blocks).
    ///
    /// Steel weight is the classic pre-design envelope estimate: areal
    /// density × (bottom/deck + shell sides) per tier, times an
    /// **internal-structure factor** of 4 covering decks, transverse and
    /// longitudinal bulkheads, floors, girders and foundation steel — a bare
    /// shell envelope runs ~4× light against real cargo-ship steel weights.
    pub fn block_division(
        &self,
        crane_capacity_kn: f64,
        workshop_dimensions: Dimensions,
    ) -> Vec<HullBlock> {
        if self.hull_form.depth_bands == 0 {
            return Vec::new();
        }
        let g = self.hull_form;
        let crane_limit_kn = crane_capacity_kn * 0.9; // rigging allowance
        let band_height = g.depth_m / g.depth_bands as f64;

        /// Shell envelope → full structure multiplier (pre-design estimate;
        /// see the method doc).
        const INTERNAL_STRUCTURE_FACTOR: f64 = 4.0;

        // Per-metre of length, one tier weighs roughly:
        // factor · areal·(bottom/deck + 2 shell sides of the tier)
        let kg_per_m_per_tier =
            INTERNAL_STRUCTURE_FACTOR * g.areal_density_kg_m2 * (g.boa_m + 2.0 * band_height);

        // Max length per block from the crane (weight limit) and workshop.
        // No upward clamp: a crane that cannot lift even a 1 m block makes
        // the division infeasible, and clamping would hide that.
        let max_len_weight = crane_limit_kn * 1000.0 / 9.81 / kg_per_m_per_tier.max(1e-6);
        let max_len = max_len_weight.min(workshop_dimensions.length).max(1e-3);
        let n_x = (g.loa_m / max_len).ceil().max(1.0) as u32;
        let block_len = g.loa_m / n_x as f64;

        let mut blocks = Vec::new();
        let mut id = BlockId(1);
        for z in 0..g.depth_bands {
            for x in 0..n_x {
                let centre = Vector3::new(
                    (x as f64 + 0.5) * block_len,
                    0.0,
                    (z as f64 + 0.5) * band_height,
                );
                let weight_kg = kg_per_m_per_tier * block_len;
                blocks.push(HullBlock {
                    id,
                    name: format!("B{}Z{}", 100 + x, z + 1),
                    geometry: Geometry {
                        centre,
                        dimensions: Dimensions::new(block_len, g.boa_m, band_height),
                    },
                    cog: centre,
                    weight_kg,
                    x_band: x,
                    z_band: z,
                    status: BlockStatus::Design,
                });
                id = BlockId(id.0 + 1);
            }
        }
        blocks
    }

    /// The erection sequence for the given blocks: bottom tier first
    /// (stability on the keel blocks), within a tier from midship outwards
    /// (limits cumulative weld shrinkage at the ends and keeps the CoG near
    /// midship). The returned joins are in erection order.
    ///
    /// Each block lands on an already-erected support: a lateral neighbour
    /// in its own tier when one exists, otherwise — for the first block of
    /// an upper tier — the block directly below it. Only the very first
    /// keel-tier block lands on nothing (`onto: None`, the dock floor).
    pub fn erection_sequence(&self, blocks: &[HullBlock]) -> Vec<BlockJoin> {
        let loa = self.hull_form.loa_m;
        let mut ordered: Vec<&HullBlock> = blocks.iter().collect();
        ordered.sort_by(|a, b| {
            a.z_band
                .cmp(&b.z_band)
                .then(midship_distance(a, loa).total_cmp(&midship_distance(b, loa)))
                .then(a.x_band.cmp(&b.x_band))
        });

        let mut joins = Vec::with_capacity(ordered.len());
        let mut erected: Vec<&HullBlock> = Vec::new();
        for (index, block) in ordered.into_iter().enumerate() {
            let lateral = erected
                .iter()
                .filter(|b| b.z_band == block.z_band)
                .min_by_key(|b| (b.x_band as i64 - block.x_band as i64).abs())
                .map(|b| b.id);
            // Upper tiers always have a landed support: the tier below is
            // fully erected first (the ordering guarantees it for complete
            // divisions), so the first block of a tier lands on the block
            // directly beneath it.
            let below = if lateral.is_none() && block.z_band > 0 {
                erected
                    .iter()
                    .find(|b| b.z_band + 1 == block.z_band && b.x_band == block.x_band)
                    .map(|b| b.id)
            } else {
                None
            };
            let onto = lateral.or(below);
            let seam = match (onto, block.z_band) {
                (None, 0) => SeamType::DockJoint,
                (None, _) | (Some(_), _) => SeamType::ButtSeam,
            };
            joins.push(BlockJoin {
                block: block.id,
                onto,
                seam,
                sequence_index: index,
                z_band: block.z_band,
            });
            erected.push(block);
        }
        joins
    }

    /// Total steel weight of all blocks, kg.
    pub fn total_steel_kg(&self) -> f64 {
        self.blocks.iter().map(|b| b.weight_kg).sum()
    }

    /// Checks that a block of this hull physically fits the workshop
    /// cross-section: every band block is the full beam wide and one band
    /// high, so the workshop breadth must admit `boa_m` and its depth the
    /// band height. (Length is a *division* constraint handled inside
    /// [`block_division`](Self::block_division); a cross-section that
    /// cannot fit is a yard infeasibility and must be surfaced, not
    /// silently divided around.)
    ///
    /// # Errors
    ///
    /// A message naming each violated workshop dimension.
    pub fn check_workshop(&self, workshop_dimensions: Dimensions) -> Result<(), String> {
        let g = self.hull_form;
        let band_height = if g.depth_bands > 0 {
            g.depth_m / g.depth_bands as f64
        } else {
            0.0
        };
        let mut violations = Vec::new();
        if workshop_dimensions.breadth + 1e-9 < g.boa_m {
            violations.push(format!(
                "workshop breadth {:.1} m < block width (beam) {:.1} m",
                workshop_dimensions.breadth, g.boa_m
            ));
        }
        if g.depth_bands > 0 && workshop_dimensions.depth + 1e-9 < band_height {
            violations.push(format!(
                "workshop depth {:.1} m < band height {:.1} m ({} bands over {:.1} m)",
                workshop_dimensions.depth, band_height, g.depth_bands, g.depth_m
            ));
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations.join("; "))
        }
    }
}

/// Longitudinal distance of a block centre from midship, m (the hull runs
/// x ∈ [0, LOA], so midship is at LOA/2 — measuring from the origin would
/// order the sequence end-to-end instead of midship-outward).
fn midship_distance(b: &HullBlock, loa_m: f64) -> f64 {
    (b.geometry.centre.x - loa_m / 2.0).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container_ship() -> HullConstruction {
        HullConstruction::new(HullGeometry {
            loa_m: 140.0,
            boa_m: 22.0,
            depth_m: 12.0,
            areal_density_kg_m2: 180.0,
            depth_bands: 2,
        })
    }

    /// Review 7B leftover: an explicit workshop *cross-section* check —
    /// a band block is the full beam wide and one band high, so a narrow
    /// or shallow workshop cannot build the blocks even when their
    /// length divides cleanly.
    #[test]
    fn workshop_cross_section_check() {
        let hull = container_ship();
        // The reference workshop (30 x 14 m) admits the 22 m beam.
        assert!(hull
            .check_workshop(Dimensions::new(24.0, 30.0, 14.0))
            .is_ok());

        // Too narrow for the beam.
        let err = hull
            .check_workshop(Dimensions::new(24.0, 18.0, 14.0))
            .unwrap_err();
        assert!(err.contains("breadth"), "{err}");
        assert!(err.contains("22.0"), "{err}");

        // Too shallow for the 6 m band height.
        let err = hull
            .check_workshop(Dimensions::new(24.0, 30.0, 4.0))
            .unwrap_err();
        assert!(err.contains("depth"), "{err}");

        // Both violated at once.
        let err = hull
            .check_workshop(Dimensions::new(24.0, 10.0, 2.0))
            .unwrap_err();
        assert!(err.contains("breadth") && err.contains("depth"), "{err}");
    }

    /// Verification: block division respects crane and workshop constraints.
    #[test]
    fn block_division_respects_constraints() {
        let hull = container_ship();
        let crane_kn = 40_000.0;
        let workshop = Dimensions::new(24.0, 30.0, 14.0);
        let blocks = hull.block_division(crane_kn, workshop);
        assert!(!blocks.is_empty());

        // Every block liftable with rigging allowance.
        for b in &blocks {
            assert!(
                b.weight_kn() <= crane_kn * 0.9 + 1e-6,
                "{} weighs {}",
                b.name,
                b.weight_kn()
            );
            // Every block fits the workshop (length & height).
            assert!(b.dimensions().length <= workshop.length + 1e-9);
            assert!(b.dimensions().depth <= workshop.depth + 1e-9);
        }

        // Full coverage: total block length per tier = LOA.
        let tier_len: f64 = blocks
            .iter()
            .filter(|b| b.z_band == 0)
            .map(|b| b.dimensions().length)
            .sum();
        assert!((tier_len - hull.hull_form.loa_m).abs() < 1e-6);

        // Strict crane limit forces more (shorter) blocks once the crane —
        // not the workshop — is the binding constraint (80 m workshop).
        let wide_workshop = Dimensions::new(80.0, 30.0, 14.0);
        let tight = hull.block_division(2_000.0, wide_workshop); // 30 m crane limit
        let loose = hull.block_division(40_000.0, wide_workshop); // 80 m workshop limit
        assert!(tight.len() > loose.len());
        for b in &tight {
            assert!(b.weight_kn() <= 2_000.0 * 0.9 + 1e-6);
        }
    }

    /// Verification: erection sequence builds the bottom tier first and
    /// grows from midship outwards.
    #[test]
    fn erection_sequence_is_stability_first() {
        let hull = container_ship();
        let blocks = hull.block_division(40_000.0, Dimensions::new(24.0, 30.0, 14.0));
        let joins = hull.erection_sequence(&blocks);
        assert_eq!(joins.len(), blocks.len());

        // Tiers never interleave: all z_band 0 before any z_band 1.
        let first_tier1 = joins.iter().position(|j| j.z_band == 1).unwrap();
        assert!(joins[..first_tier1].iter().all(|j| j.z_band == 0));

        // Within the bottom tier, distance from midship is non-decreasing
        // (review 7D: inline arithmetic, not the function under test).
        let by_id = |id: BlockId| blocks.iter().find(|b| b.id == id).unwrap();
        let tier0: Vec<&BlockJoin> = joins[..first_tier1].iter().collect();
        for w in tier0.windows(2) {
            let d0 = (by_id(w[0].block).geometry.centre.x - 70.0).abs();
            let d1 = (by_id(w[1].block).geometry.centre.x - 70.0).abs();
            assert!(d0 <= d1 + 1e-9, "midship-outward order violated");
        }

        // First join is a dock joint on the keel tier.
        assert_eq!(joins[0].seam, SeamType::DockJoint);
        assert_eq!(joins[0].onto, None);

        // Later blocks in a tier land against an erected neighbour.
        let join_with_neighbour = joins
            .iter()
            .find(|j| j.z_band == 0 && j.onto.is_some())
            .expect("subsequent tier-0 blocks join neighbours");
        assert!(by_id(join_with_neighbour.onto.unwrap()).z_band == 0);

        // Regression (review 7A/A7): the first block of an upper tier must
        // land on the block directly below it, not hang from `onto: None`.
        let first_tier1_join = &joins[first_tier1];
        let below = by_id(
            first_tier1_join
                .onto
                .expect("upper tier lands on tier below"),
        );
        assert_eq!(below.z_band, 0);
        assert_eq!(below.x_band, by_id(first_tier1_join.block).x_band);
        // Every upper-tier block has a support.
        assert!(joins[first_tier1..].iter().all(|j| j.onto.is_some()));
    }

    #[test]
    fn total_steel_matches_block_sum() {
        let mut hull = container_ship();
        hull.blocks = hull.block_division(40_000.0, Dimensions::new(24.0, 30.0, 14.0));
        let sum: f64 = hull.blocks.iter().map(|b| b.weight_kg).sum();
        assert!((hull.total_steel_kg() - sum).abs() < 1e-6);
        assert!(hull.total_steel_kg() > 0.0);
    }

    #[test]
    fn status_transitions() {
        assert!(!BlockStatus::Design.is_erection_ready());
        assert!(BlockStatus::ReadyForErection.is_erection_ready());
        assert!(BlockStatus::Erected.is_erection_ready());
        assert_eq!(BlockStatus::Welding.to_string(), "welding");
    }
}
