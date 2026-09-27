//! Weight and centre-of-gravity management for vessels under construction.
//!
//! A [`WeightModel`] is the single source of truth for mass properties. Every
//! measurable mass on the vessel is a [`WeightItem`] with a status
//! ([`ItemStatus`]) that advances as procurement and installation proceed.
//! The model answers the two questions a shipyard asks continuously: *how
//! much does it weigh now* ([`WeightModel::total_weight`]) and *where is the
//! centre of gravity* ([`WeightModel::centre_of_gravity`]).
//!
//! # Semantics
//!
//! - [`WeightModel::total_weight`] follows the master-plan convention: items
//!   that are [`ItemStatus::Installed`] count at their **as-built** weight,
//!   everything still in play (`Design`, `Ordered`, `Received`) counts at its
//!   **predicted** weight — i.e. the current best estimate of the lightship
//!   weight.
//! - [`WeightModel::installed_weight`] counts only what is physically in the
//!   ship (as-built weight to date).
//! - [`WeightModel::weight_deviation`] compares the best estimate against the
//!   contractual [`WeightModel::design_weight_kg`].
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::{ItemId, Vector3};
//! use tpt_yard_weight::{ItemStatus, WeightItem, WeightModel};
//!
//! let mut model = WeightModel::new(10_000.0, Vector3::new(60.0, 0.0, 6.0));
//! model.add_item(WeightItem {
//!     id: ItemId(1),
//!     name: "Block 211".into(),
//!     group: "hull".into(),
//!     weight_kg: 120.0,
//!     cog: Vector3::new(10.0, 0.0, 5.0),
//!     status: ItemStatus::Installed,
//!     margin_pct: 2.0,
//!     installed_by: None,
//! });
//! assert_eq!(model.installed_weight(), 120.0);
//! ```

use tpt_yard_core::{ItemId, MassProperties, Vector3};

/// Procurement / installation status of a weight item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    /// Predicted in design; not yet ordered.
    Design,
    /// Ordered from the supplier.
    Ordered,
    /// Delivered to the yard, not yet installed.
    Received,
    /// Physically installed in the vessel (as-built weight is now known).
    Installed,
    /// Installed and later swapped out (kept for history, not counted).
    Replaced,
}

impl ItemStatus {
    /// True if the item is physically in the vessel.
    pub fn is_installed(self) -> bool {
        matches!(self, ItemStatus::Installed)
    }
}

/// One measurable mass on the vessel.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightItem {
    /// Item identifier.
    pub id: ItemId,
    /// Item name.
    pub name: String,
    /// Grouping key for reports: system or zone (e.g. "hull", "machinery",
    /// "zone 3"). Choose one convention per model.
    pub group: String,
    /// Mass, kg. For `Installed` items this is the *as-built* (weighed or
    /// updated) mass; otherwise the predicted mass.
    pub weight_kg: f64,
    /// Centre of gravity, m, in the ship coordinate system.
    pub cog: Vector3,
    /// Current lifecycle status.
    pub status: ItemStatus,
    /// Growth margin held on this item, percent of `weight_kg`.
    pub margin_pct: f64,
    /// The assembly activity that installs this item, if tracked. The digital
    /// twin flips `status` to [`ItemStatus::Installed`] when that activity
    /// completes.
    pub installed_by: Option<tpt_yard_assembly::ActivityId>,
}

impl WeightItem {
    /// Effective mass including the growth margin, kg.
    pub fn weight_with_margin_kg(&self) -> f64 {
        self.weight_kg * (1.0 + self.margin_pct / 100.0)
    }
}

/// Aggregated weight report.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightReport {
    /// Current best-estimate total (installed + predicted remainder), kg.
    pub total_weight_kg: f64,
    /// Physically installed weight, kg.
    pub installed_weight_kg: f64,
    /// Contractual design weight, kg.
    pub design_weight_kg: f64,
    /// `total_weight_kg - design_weight_kg`; positive means overweight.
    pub deviation_kg: f64,
    /// Total held margin, kg.
    pub margin_kg: f64,
    /// Centre of gravity of the best-estimate total, m.
    pub cog: Vector3,
    /// Count of items still not installed.
    pub outstanding_items: usize,
    /// Weight by group key (system or zone), kg, descending.
    pub by_group: Vec<(String, f64)>,
}

/// Errors returned by weight-model operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeightError {
    /// The referenced item does not exist.
    UnknownItem(ItemId),
    /// The model holds no countable mass, so a CoG cannot be defined.
    EmptyModel,
}

impl std::fmt::Display for WeightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeightError::UnknownItem(id) => write!(f, "unknown weight item {id}"),
            WeightError::EmptyModel => {
                write!(f, "no countable items: centre of gravity undefined")
            }
        }
    }
}

impl std::error::Error for WeightError {}

/// The weight model: all items plus the contractual design target.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightModel {
    /// Contractual design (lightship) weight, kg.
    pub design_weight_kg: f64,
    /// Contractual design centre of gravity, m.
    pub design_cog: Vector3,
    /// All tracked items.
    pub items: Vec<WeightItem>,
}

impl WeightModel {
    /// Creates an empty model with the contractual design target.
    pub fn new(design_weight_kg: f64, design_cog: Vector3) -> Self {
        Self {
            design_weight_kg,
            design_cog,
            items: Vec::new(),
        }
    }

    /// Adds an item to the model.
    pub fn add_item(&mut self, item: WeightItem) {
        self.items.push(item);
    }

    /// Marks an item installed; the as-built weight becomes authoritative.
    ///
    /// # Errors
    ///
    /// [`WeightError::UnknownItem`] if the id is not in the model.
    pub fn mark_installed(&mut self, id: ItemId, as_built_kg: f64) -> Result<(), WeightError> {
        let item = self
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(WeightError::UnknownItem(id))?;
        item.status = ItemStatus::Installed;
        item.weight_kg = as_built_kg;
        Ok(())
    }

    /// Items with a given status.
    pub fn items_with_status(&self, status: ItemStatus) -> impl Iterator<Item = &WeightItem> {
        self.items.iter().filter(move |i| i.status == status)
    }

    /// Current best-estimate weight, kg: installed items at as-built mass,
    /// all other non-replaced items at predicted mass.
    pub fn total_weight(&self) -> f64 {
        self.items
            .iter()
            .filter(|i| !matches!(i.status, ItemStatus::Replaced))
            .map(|i| i.weight_kg)
            .sum()
    }

    /// Weight physically installed to date, kg.
    pub fn installed_weight(&self) -> f64 {
        self.items
            .iter()
            .filter(|i| i.status.is_installed())
            .map(|i| i.weight_kg)
            .sum()
    }

    /// Centre of gravity of the current best-estimate set, m.
    ///
    /// # Errors
    ///
    /// [`WeightError::EmptyModel`] when nothing countable is present.
    pub fn centre_of_gravity(&self) -> Result<Vector3, WeightError> {
        let mut moment = Vector3::ZERO;
        let mut total = 0.0;
        for i in &self.items {
            if matches!(i.status, ItemStatus::Replaced) {
                continue;
            }
            moment = moment + i.cog * i.weight_kg;
            total += i.weight_kg;
        }
        if total <= 0.0 {
            return Err(WeightError::EmptyModel);
        }
        Ok(moment / total)
    }

    /// Centre of gravity of the installed subset only (as-built CoG), m.
    ///
    /// # Errors
    ///
    /// [`WeightError::EmptyModel`] if nothing is installed yet.
    pub fn installed_centre_of_gravity(&self) -> Result<Vector3, WeightError> {
        let mut moment = Vector3::ZERO;
        let mut total = 0.0;
        for i in self.items.iter().filter(|i| i.status.is_installed()) {
            moment = moment + i.cog * i.weight_kg;
            total += i.weight_kg;
        }
        if total <= 0.0 {
            return Err(WeightError::EmptyModel);
        }
        Ok(moment / total)
    }

    /// Mass properties of the current best-estimate set.
    pub fn mass_properties(&self) -> MassProperties {
        MassProperties {
            mass_kg: self.total_weight(),
            cog: self.centre_of_gravity().unwrap_or(Vector3::ZERO),
        }
    }

    /// Best-estimate total minus the contractual design weight, kg (positive
    /// = overweight).
    pub fn weight_deviation(&self) -> f64 {
        self.total_weight() - self.design_weight_kg
    }

    /// Sum of margins over countable items, kg.
    pub fn total_margin_kg(&self) -> f64 {
        self.items
            .iter()
            .filter(|i| !matches!(i.status, ItemStatus::Replaced))
            .map(|i| i.weight_kg * i.margin_pct / 100.0)
            .sum()
    }

    /// Aggregated report by group.
    pub fn weight_report(&self) -> WeightReport {
        let mut by_group: Vec<(String, f64)> = Vec::new();
        for i in &self.items {
            if matches!(i.status, ItemStatus::Replaced) {
                continue;
            }
            match by_group.iter_mut().find(|(g, _)| *g == i.group) {
                Some((_, w)) => *w += i.weight_kg,
                None => by_group.push((i.group.clone(), i.weight_kg)),
            }
        }
        by_group.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        WeightReport {
            total_weight_kg: self.total_weight(),
            installed_weight_kg: self.installed_weight(),
            design_weight_kg: self.design_weight_kg,
            deviation_kg: self.weight_deviation(),
            margin_kg: self.total_margin_kg(),
            cog: self.centre_of_gravity().unwrap_or(Vector3::ZERO),
            outstanding_items: self
                .items
                .iter()
                .filter(|i| !i.status.is_installed() && !matches!(i.status, ItemStatus::Replaced))
                .count(),
            by_group,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_assembly::ActivityId;

    /// Verification: sum of block weights must equal the vessel total.
    #[test]
    fn test_block_weight_sum() {
        let mut model = WeightModel::new(190_000.0, Vector3::new(70.0, 0.0, 7.0));
        // Ten blocks: 8 hull blocks of 15 t + 2 machinery blocks of 35 t.
        for b in 0..8u64 {
            model.add_item(WeightItem {
                id: ItemId(b + 1),
                name: format!("Hull block {b}"),
                group: "hull".into(),
                weight_kg: 15_000.0,
                cog: Vector3::new(10.0 * b as f64, 0.0, 6.0),
                status: ItemStatus::Installed,
                margin_pct: 2.0,
                installed_by: Some(ActivityId(b + 1)),
            });
        }
        for m in 0..2u64 {
            model.add_item(WeightItem {
                id: ItemId(100 + m),
                name: format!("Machinery block {m}"),
                group: "machinery".into(),
                weight_kg: 35_000.0,
                cog: Vector3::new(20.0 + 30.0 * m as f64, 0.0, 4.0),
                status: ItemStatus::Design,
                margin_pct: 5.0,
                installed_by: Some(ActivityId(50 + m)),
            });
        }
        let block_sum: f64 = model.items.iter().map(|i| i.weight_kg).sum();
        assert_eq!(model.total_weight(), block_sum);
        assert_eq!(model.total_weight(), 8.0 * 15_000.0 + 2.0 * 35_000.0);
        assert_eq!(model.installed_weight(), 8.0 * 15_000.0);
    }

    #[test]
    fn cog_is_weighted_mean() {
        let mut model = WeightModel::new(1.0, Vector3::ZERO);
        model.add_item(item(
            1,
            100.0,
            Vector3::new(0.0, 0.0, 0.0),
            ItemStatus::Installed,
        ));
        model.add_item(item(
            2,
            300.0,
            Vector3::new(4.0, 0.0, 0.0),
            ItemStatus::Installed,
        ));
        let cog = model.centre_of_gravity().unwrap();
        assert!((cog.x - 3.0).abs() < 1e-12);
        assert!(cog.y.abs() < 1e-12);
    }

    #[test]
    fn replaced_items_never_count() {
        let mut model = WeightModel::new(0.0, Vector3::ZERO);
        model.add_item(item(1, 100.0, Vector3::ZERO, ItemStatus::Installed));
        model.add_item(item(2, 50.0, Vector3::ZERO, ItemStatus::Replaced));
        assert_eq!(model.total_weight(), 100.0);
        model.items[1].status = ItemStatus::Design;
        assert_eq!(model.total_weight(), 150.0);
    }

    #[test]
    fn empty_model_has_no_cog() {
        let model = WeightModel::new(0.0, Vector3::ZERO);
        assert_eq!(model.centre_of_gravity(), Err(WeightError::EmptyModel));
    }

    #[test]
    fn deviation_and_report() {
        let mut model = WeightModel::new(1_000.0, Vector3::ZERO);
        let mut i1 = item(1, 600.0, Vector3::new(2.0, 0.0, 1.0), ItemStatus::Installed);
        i1.group = "installed".into();
        let mut i2 = item(2, 500.0, Vector3::new(-2.0, 0.0, 1.0), ItemStatus::Design);
        i2.group = "design".into();
        model.add_item(i1);
        model.add_item(i2);
        let report = model.weight_report();
        assert_eq!(report.total_weight_kg, 1_100.0);
        assert_eq!(report.deviation_kg, 100.0); // 100 kg overweight
        assert_eq!(report.installed_weight_kg, 600.0);
        assert_eq!(report.outstanding_items, 1);
        assert_eq!(report.by_group.len(), 2);
        assert_eq!(report.by_group[0].0, "installed"); // descending weight
        let cog = model.installed_centre_of_gravity().unwrap();
        assert!((cog.x - 2.0).abs() < 1e-12);
    }

    #[test]
    fn mark_installed_updates_weight() {
        let mut model = WeightModel::new(500.0, Vector3::ZERO);
        model.add_item(item(
            1,
            300.0,
            Vector3::new(1.0, 0.0, 0.0),
            ItemStatus::Received,
        ));
        model.mark_installed(ItemId(1), 315.0).unwrap();
        assert_eq!(model.installed_weight(), 315.0);
        assert_eq!(model.items[0].weight_kg, 315.0);
        assert!(matches!(
            model.mark_installed(ItemId(99), 1.0),
            Err(WeightError::UnknownItem(_))
        ));
    }

    #[test]
    fn margins_add_up() {
        let mut model = WeightModel::new(0.0, Vector3::ZERO);
        model.add_item(item(1, 100.0, Vector3::ZERO, ItemStatus::Design));
        model.items[0].margin_pct = 5.0;
        model.add_item(item(2, 200.0, Vector3::ZERO, ItemStatus::Design));
        model.items[1].margin_pct = 10.0;
        assert!((model.total_margin_kg() - 25.0).abs() < 1e-9);
        assert!((model.items[0].weight_with_margin_kg() - 105.0).abs() < 1e-9);
    }

    fn item(id: u64, kg: f64, cog: Vector3, status: ItemStatus) -> WeightItem {
        WeightItem {
            id: ItemId(id),
            name: format!("item {id}"),
            group: "a".into(),
            weight_kg: kg,
            cog,
            status,
            margin_pct: 0.0,
            installed_by: None,
        }
    }
}
