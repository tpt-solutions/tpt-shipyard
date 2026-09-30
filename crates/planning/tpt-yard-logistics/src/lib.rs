//! Material and resource logistics: delivery, staging, and transport
//! scheduling.
//!
//! Yard logistics is a pull system: every material item is needed by an
//! assembly activity at a computable date (the CPM schedule), orders have
//! lead times, and staging space is finite. [`MaterialFlow`] holds the
//! manifest; [`MaterialFlow::schedule_deliveries`] computes order-by and
//! arrive-by dates, respecting a staging dwell limit — material that
//! arrives too early clogs the laydown area.
//!
//! # Example
//!
//! ```
//! use tpt_yard_assembly::ActivityId;
//! use tpt_yard_core::{ActivityType, AssemblyActivity};
//! use tpt_yard_logistics::{MaterialFlow, MaterialItem};
//!
//! let acts = vec![AssemblyActivity::new(
//!     ActivityId(1), "Erect", ActivityType::JoinBlock, 8.0,
//! )];
//! let mut flow = MaterialFlow::new();
//! flow.add_item(MaterialItem {
//!     id: 1,
//!     name: "Block 212 steel".into(),
//!     quantity_t: 142.0,
//!     needed_for: ActivityId(1),
//!     lead_time_days: 30.0,
//!     footprint_m2: 200.0,
//! });
//! // Activity 1 starts at calendar day 0 (30 working days of lead time
//! // before need expand to ~42 calendar days; plus the 5-day buffer).
//! let deliveries = flow.schedule_deliveries(&acts, 5.0, 1_000.0, None).unwrap();
//! assert_eq!(deliveries.len(), 1);
//! assert!((deliveries[0].quantity_t - 142.0).abs() < 1e-9);
//! ```
#![allow(clippy::doc_markdown)]

use std::fmt;

use tpt_yard_assembly::{ActivityGraph, ActivityId, GraphError};
use tpt_yard_core::AssemblyActivity;

/// One material item on the manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialItem {
    /// Item id.
    pub id: u64,
    /// Name.
    pub name: String,
    /// Quantity, tonnes.
    pub quantity_t: f64,
    /// The activity that consumes it.
    pub needed_for: ActivityId,
    /// Supplier lead time, days.
    pub lead_time_days: f64,
    /// Staging footprint, m².
    pub footprint_m2: f64,
}

/// A computed delivery.
#[derive(Debug, Clone, PartialEq)]
pub struct Delivery {
    /// Item id.
    pub item_id: u64,
    /// Day the order must be placed (negative = before project start).
    pub order_by_day: f64,
    /// Day the material arrives at staging.
    pub arrive_day: f64,
    /// Day the consuming activity starts.
    pub need_day: f64,
    /// Days the item waits in staging.
    pub dwell_days: f64,
    /// Tonnes delivered (mirrors the manifest item).
    pub quantity_t: f64,
}

/// Errors from logistics scheduling.
#[derive(Debug, Clone, PartialEq)]
pub enum LogisticsError {
    /// The activity network is malformed.
    Graph(GraphError),
    /// A manifest item references an unknown activity.
    UnknownActivity(ActivityId),
    /// The staging area cannot hold the concurrent footprint.
    StagingOverflow {
        /// Required staging area, m².
        required_m2: f64,
        /// Available staging area, m².
        available_m2: f64,
    },
    /// A delivery would wait in staging longer than its dwell limit.
    DwellExceeded {
        /// The item.
        item_id: u64,
        /// Computed dwell, days.
        dwell_days: f64,
        /// The limit, days.
        limit_days: f64,
    },
}

impl fmt::Display for LogisticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogisticsError::Graph(e) => write!(f, "graph: {e}"),
            LogisticsError::UnknownActivity(a) => write!(f, "unknown activity {a}"),
            LogisticsError::StagingOverflow {
                required_m2,
                available_m2,
            } => write!(
                f,
                "staging overflow: {required_m2:.0} m² needed, {available_m2:.0} m² available"
            ),
            LogisticsError::DwellExceeded {
                item_id,
                dwell_days,
                limit_days,
            } => write!(
                f,
                "item {item_id} would dwell {dwell_days:.1} d in staging, over the {limit_days:.1} d limit"
            ),
        }
    }
}

impl std::error::Error for LogisticsError {}

impl From<GraphError> for LogisticsError {
    fn from(e: GraphError) -> Self {
        LogisticsError::Graph(e)
    }
}

/// The material flow manifest.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MaterialFlow {
    /// Manifest items.
    pub items: Vec<MaterialItem>,
}

impl MaterialFlow {
    /// Creates an empty manifest.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an item.
    pub fn add_item(&mut self, item: MaterialItem) {
        self.items.push(item);
    }

    /// The CPM start of every activity, in **calendar** days.
    ///
    /// The CPM pass runs in working hours (8-hour days); lead times and
    /// buffers are supplier/calendar quantities, so the schedule is
    /// expanded to calendar days over a 5-day week before any lead-time
    /// arithmetic mixes the two (working day `n` lands on calendar day
    /// `7*floor(n/5) + n mod 5`).
    fn activity_start_days(
        acts: &[AssemblyActivity],
    ) -> Result<BTreeMap<ActivityId, f64>, LogisticsError> {
        let mut g = ActivityGraph::new();
        for a in acts {
            if !g.contains(a.id) {
                g.add_activity(a.id, a.name.clone(), a.duration_hours, &a.dependencies)?;
            }
        }
        g.validate()?;
        let finish = g.earliest_finish_times()?;
        let mut starts = BTreeMap::new();
        for (&id, &f) in &finish {
            let dur = g.activity(id).expect("exists").duration_hours;
            let working_day = (f - dur) / 8.0;
            starts.insert(id, Self::working_to_calendar_days(working_day));
        }
        Ok(starts)
    }

    /// Expands a working-day index to a calendar-day index (5-day week).
    fn working_to_calendar_days(working_day: f64) -> f64 {
        let whole = working_day.floor();
        let frac = working_day - whole;
        let weeks = (whole / 5.0).floor();
        let rem = whole - weeks * 5.0;
        (weeks * 7.0 + rem + frac).max(0.0)
    }

    /// Schedules deliveries against the activity schedule.
    ///
    /// Each item orders `lead_time + buffer` calendar days before its
    /// activity starts and arrives `buffer` days before it. When
    /// `max_dwell_days` is set, a delivery that would wait longer than the
    /// limit in staging is rejected ([`LogisticsError::DwellExceeded`]).
    /// The concurrent staging footprint is validated against
    /// `staging_area_m2`; deliveries carry their tonnage.
    ///
    /// # Errors
    ///
    /// [`LogisticsError`] on unknown activities or staging overflow.
    pub fn schedule_deliveries(
        &self,
        acts: &[AssemblyActivity],
        buffer_days: f64,
        staging_area_m2: f64,
        max_dwell_days: Option<f64>,
    ) -> Result<Vec<Delivery>, LogisticsError> {
        let starts = Self::activity_start_days(acts)?;
        let mut deliveries = Vec::with_capacity(self.items.len());
        // (arrive_day, footprint) pairs for the staging overlap check.
        let mut arrivals: Vec<(f64, f64, f64)> = Vec::new(); // arrive, need, footprint
        for item in &self.items {
            let Some(&need_day) = starts.get(&item.needed_for) else {
                return Err(LogisticsError::UnknownActivity(item.needed_for));
            };
            let arrive = need_day - buffer_days;
            let order = arrive - item.lead_time_days;
            let dwell = need_day - arrive;
            if let Some(limit) = max_dwell_days {
                if dwell > limit {
                    return Err(LogisticsError::DwellExceeded {
                        item_id: item.id,
                        dwell_days: dwell,
                        limit_days: limit,
                    });
                }
            }
            deliveries.push(Delivery {
                item_id: item.id,
                order_by_day: order,
                arrive_day: arrive,
                need_day,
                dwell_days: dwell,
                quantity_t: item.quantity_t,
            });
            arrivals.push((arrive, need_day, item.footprint_m2));
        }
        deliveries.sort_by(|a, b| a.arrive_day.total_cmp(&b.arrive_day));

        // Staging occupancy: max concurrent footprint (sweep by arrival and
        // consumption events).
        let mut events: Vec<(f64, f64)> = Vec::new(); // (day, +footprint/-footprint)
        for &(arrive, need, fp) in &arrivals {
            events.push((arrive, fp));
            events.push((need, -fp));
        }
        events.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.1.total_cmp(&a.1)));
        let mut occupancy = 0.0;
        let mut peak = 0.0f64;
        for (_, delta) in events {
            occupancy += delta;
            peak = peak.max(occupancy);
        }
        if peak > staging_area_m2 {
            return Err(LogisticsError::StagingOverflow {
                required_m2: peak,
                available_m2: staging_area_m2,
            });
        }
        Ok(deliveries)
    }
}

use std::collections::BTreeMap;

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_core::ActivityType;

    fn acts() -> Vec<AssemblyActivity> {
        vec![
            AssemblyActivity::new(ActivityId(1), "Erect A", ActivityType::JoinBlock, 16.0),
            AssemblyActivity::new(ActivityId(2), "Erect B", ActivityType::JoinBlock, 16.0)
                .with_dependencies(&[ActivityId(1)]),
        ]
    }

    /// Regression (review 7B): the dwell limit is actually enforced, and
    /// the schedule expands working days to calendar days.
    #[test]
    fn dwell_limit_and_calendar_expansion() {
        let mut flow = MaterialFlow::new();
        flow.add_item(MaterialItem {
            id: 1,
            name: "steel".into(),
            quantity_t: 50.0,
            needed_for: ActivityId(1),
            lead_time_days: 10.0,
            footprint_m2: 100.0,
        });
        // Activity 1 starts at working day 0 -> calendar day 0.
        let d = flow
            .schedule_deliveries(&acts(), 10.0, 1_000.0, Some(15.0))
            .unwrap();
        assert_eq!(d[0].dwell_days, 10.0);
        assert!((d[0].quantity_t - 50.0).abs() < 1e-9);
        // A 5-day limit rejects the 10-day dwell.
        let err = flow
            .schedule_deliveries(&acts(), 10.0, 1_000.0, Some(5.0))
            .unwrap_err();
        assert!(matches!(err, LogisticsError::DwellExceeded { .. }));
        // Calendar expansion: an activity at working day 10 lands on
        // calendar day 14 (two weekends passed).
        let late = vec![AssemblyActivity::new(
            ActivityId(9),
            "L",
            ActivityType::JoinBlock,
            80.0, // starts at working hour 0 after... independent: starts wd 0
        )];
        let starts = MaterialFlow::activity_start_days(&late).unwrap();
        assert!((starts[&ActivityId(9)] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn order_dates_lead_the_need() {
        let mut flow = MaterialFlow::new();
        flow.add_item(MaterialItem {
            id: 1,
            name: "steel A".into(),
            quantity_t: 140.0,
            needed_for: ActivityId(1),
            lead_time_days: 30.0,
            footprint_m2: 100.0,
        });
        flow.add_item(MaterialItem {
            id: 2,
            name: "steel B".into(),
            quantity_t: 140.0,
            needed_for: ActivityId(2),
            lead_time_days: 10.0,
            footprint_m2: 100.0,
        });
        let deliveries = flow
            .schedule_deliveries(&acts(), 5.0, 1_000.0, None)
            .unwrap();
        assert_eq!(deliveries.len(), 2);
        // Activity 1 starts day 0: order at -35 (30 lead + 5 buffer).
        let d1 = &deliveries[0];
        assert!((d1.order_by_day - -35.0).abs() < 1e-9);
        assert!((d1.arrive_day - -5.0).abs() < 1e-9);
        assert!((d1.dwell_days - 5.0).abs() < 1e-9);
        // Activity 2 starts day 2 (16 h = 2 days): order at 2-15 = -13.
        let d2 = &deliveries[1];
        assert!((d2.need_day - 2.0).abs() < 1e-9);
        assert!((d2.order_by_day - -13.0).abs() < 1e-9);
    }

    #[test]
    fn staging_overflow_is_detected() {
        let mut flow = MaterialFlow::new();
        flow.add_item(MaterialItem {
            id: 1,
            name: "huge module".into(),
            quantity_t: 500.0,
            needed_for: ActivityId(1),
            lead_time_days: 1.0,
            footprint_m2: 2_000.0,
        });
        let err = flow
            .schedule_deliveries(&acts(), 5.0, 1_000.0, None)
            .unwrap_err();
        assert!(matches!(err, LogisticsError::StagingOverflow { .. }));
        // With a big enough staging area it passes.
        assert!(flow
            .schedule_deliveries(&acts(), 5.0, 2_000.0, None)
            .is_ok());
    }

    #[test]
    fn unknown_activity_rejected() {
        let mut flow = MaterialFlow::new();
        flow.add_item(MaterialItem {
            id: 1,
            name: "ghost steel".into(),
            quantity_t: 1.0,
            needed_for: ActivityId(99),
            lead_time_days: 1.0,
            footprint_m2: 1.0,
        });
        assert!(matches!(
            flow.schedule_deliveries(&acts(), 5.0, 1_000.0, None),
            Err(LogisticsError::UnknownActivity(_))
        ));
    }
}
