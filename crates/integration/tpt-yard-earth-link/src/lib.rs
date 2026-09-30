//! Bridge between weather and sea-state data (`tpt-earth`) and launch
//! planning.
//!
//! The `tpt-earth` substrate is not published yet, so this crate vendors
//! the minimal sea-state forecast type ([`SeaStateForecast`], Douglas
//! scale) behind the shape the substrate will use. [`plan_launch_window`]
//! gates launch methods on sea state and returns the safe window.
//!
//! # Example
//!
//! ```
//! use tpt_yard_core::{MassProperties, Vector3};
//! use tpt_yard_earth_link::{plan_launch_window, SeaStateForecast};
//! use tpt_yard_launch::{LaunchAnalysis, LaunchMethod, SiteConditions}; // SiteConditions used in tests via super
//!
//! let analysis = LaunchAnalysis {
//!     launch_method: LaunchMethod::Slipway { slope_deg: 3.0, ways: 2 },
//!     vessel_weight: MassProperties {
//!         mass_kg: 4_000_000.0,
//!         cog: Vector3::new(70.0, 0.0, 6.0),
//!     },
//!     way_length_m: 120.0,
//!     way_width_m: 2.0,
//!     friction_coefficient: 0.02,
//!     poppet_to_cog_m: 70.0,
//!     end_bearing_m: 20.0,
//!     immersion_length_m: 90.0,
//!     block_coefficient: 0.8,
//!     breadth_m: 20.0,
//!     site: SiteConditions { max_sea_state: 3 },
//! };
//!
//! // Calm at both ends of the day, a storm in the middle.
//! let forecast = SeaStateForecast {
//!     hourly_sea_state: (0..24)
//!         .map(|h| if (9..17).contains(&h) { 4.0 } else { 1.0 })
//!         .collect(),
//! };
//! let window = plan_launch_window(&analysis, &forecast).unwrap();
//! assert_eq!(window.earliest_hour, 0); // morning run is longest
//! assert!(window.latest_hour <= 9.0);
//! ```
#![allow(clippy::doc_markdown)]

use std::fmt;

#[allow(unused_imports)]
use tpt_yard_core::Vector3;
use tpt_yard_launch::LaunchAnalysis;
#[allow(unused_imports)]
use tpt_yard_launch::{LaunchMethod, SiteConditions};

/// Sea-state forecast (stand-in for `tpt_earth::ocean::SeaState`):
/// Douglas-scale values, one per hour of the forecast horizon.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SeaStateForecast {
    /// Douglas sea state (0-9) per forecast hour.
    pub hourly_sea_state: Vec<f64>,
}

/// The safe launch window.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchWindow {
    /// First hour the launch is safe.
    pub earliest_hour: usize,
    /// Last hour the launch remains safe (exclusive bound on the calm run).
    pub latest_hour: f64,
    /// The governing sea-state limit of the launch method.
    pub sea_state_limit: f64,
    /// Total calm hours available.
    pub calm_hours: usize,
    /// Findings.
    pub notes: Vec<String>,
}

/// Errors from launch-window planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchWindowError {
    /// No calm hours in the forecast: the launch cannot proceed.
    NoSafeWindow,
    /// The forecast is empty.
    EmptyForecast,
}

impl fmt::Display for LaunchWindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaunchWindowError::NoSafeWindow => {
                f.write_str("no hours below the sea-state limit in the forecast")
            }
            LaunchWindowError::EmptyForecast => f.write_str("empty forecast"),
        }
    }
}

impl std::error::Error for LaunchWindowError {}

/// The sea-state limit by launch method (yard practice screening):
/// slipway end launch <= 2, side launch <= 1, dock flooding <= 3, shiplift <= 2.
/// (Combined with the site's operational limit in `plan_launch_window`.)
fn method_sea_state_limit(analysis: &LaunchAnalysis) -> f64 {
    match analysis.launch_method {
        LaunchMethod::Slipway { .. } => 2.0,
        LaunchMethod::SideLaunch => 1.0,
        LaunchMethod::DrydockFlooding => 3.0,
        LaunchMethod::Shiplift { .. } => 2.0,
    }
}

/// Finds the first safe window in the forecast for the launch method.
///
/// The window spans the longest (and first) run of hours at or below the
/// sea-state limit - the *stricter* of the launch-method practice limit and
/// the site's operational `max_sea_state` (the site limit was ignored
/// before, review 7B); the `earliest_hour` is that run's start.
///
/// # Errors
///
/// [`LaunchWindowError`] when the forecast is empty or never safe.
pub fn plan_launch_window(
    analysis: &LaunchAnalysis,
    forecast: &SeaStateForecast,
) -> Result<LaunchWindow, LaunchWindowError> {
    if forecast.hourly_sea_state.is_empty() {
        return Err(LaunchWindowError::EmptyForecast);
    }
    let limit = method_sea_state_limit(analysis).min(analysis.site.max_sea_state as f64);

    // Longest calm run.
    let (mut best_start, mut best_len) = (0usize, 0usize);
    let (mut run_start, mut run_len) = (0usize, 0usize);
    for (h, &state) in forecast.hourly_sea_state.iter().enumerate() {
        if state <= limit {
            if run_len == 0 {
                run_start = h;
            }
            run_len += 1;
            if run_len > best_len {
                best_len = run_len;
                best_start = run_start;
            }
        } else {
            run_len = 0;
        }
    }
    if best_len == 0 {
        return Err(LaunchWindowError::NoSafeWindow);
    }

    let latest = (best_start + best_len) as f64;
    Ok(LaunchWindow {
        earliest_hour: best_start,
        latest_hour: latest,
        sea_state_limit: limit,
        calm_hours: best_len,
        notes: vec![format!(
            "safe from hour {} to {} ({} h) at sea state <= {limit}",
            best_start,
            best_start + best_len,
            best_len
        )],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_yard_core::MassProperties;

    fn slipway() -> LaunchAnalysis {
        LaunchAnalysis {
            launch_method: LaunchMethod::Slipway {
                slope_deg: 3.0,
                ways: 2,
            },
            vessel_weight: MassProperties {
                mass_kg: 4_000_000.0,
                cog: Vector3::new(70.0, 0.0, 6.0),
            },
            way_length_m: 120.0,
            way_width_m: 2.0,
            friction_coefficient: 0.02,
            poppet_to_cog_m: 70.0,
            end_bearing_m: 20.0,
            immersion_length_m: 90.0,
            block_coefficient: 0.8,
            breadth_m: 20.0,
            site: SiteConditions { max_sea_state: 3 },
        }
    }

    #[test]
    fn storm_splits_the_day() {
        // Storm 5..17: the evening run (17..24) is longer than the morning
        // one (0..5), so the window opens at hour 17.
        let forecast = SeaStateForecast {
            hourly_sea_state: (0..24)
                .map(|h| if (5..17).contains(&h) { 4.0 } else { 1.0 })
                .collect(),
        };
        let window = plan_launch_window(&slipway(), &forecast).unwrap();
        assert_eq!(window.earliest_hour, 17);
        assert_eq!(window.calm_hours, 7); // hours 17..24
        assert_eq!(window.sea_state_limit, 2.0);
    }

    /// Regression (review 7B): the site's operational sea-state limit is
    /// enforced, not just the per-method practice limit.
    #[test]
    fn site_limit_stricter_than_method_limit() {
        // Sheltered site: slipway practice allows SS 2, the site only SS 1.
        let mut a = slipway();
        a.site.max_sea_state = 1;
        let forecast = SeaStateForecast {
            hourly_sea_state: (0..24).map(|h| if h < 6 { 2.0 } else { 1.0 }).collect(),
        };
        let window = plan_launch_window(&a, &forecast).unwrap();
        // SS 2 hours would pass a bare method limit of 2 but must wait.
        assert_eq!(window.earliest_hour, 6);
        assert_eq!(window.sea_state_limit, 1.0);
    }

    #[test]
    fn calm_day_opens_at_once() {
        let forecast = SeaStateForecast {
            hourly_sea_state: vec![1.0; 24],
        };
        let window = plan_launch_window(&slipway(), &forecast).unwrap();
        assert_eq!(window.earliest_hour, 0);
        assert_eq!(window.calm_hours, 24);
    }

    #[test]
    fn never_safe_is_reported() {
        let forecast = SeaStateForecast {
            hourly_sea_state: vec![5.0; 24],
        };
        assert_eq!(
            plan_launch_window(&slipway(), &forecast),
            Err(LaunchWindowError::NoSafeWindow)
        );
    }

    #[test]
    fn method_limits_differ() {
        // Sea state 2: fine for dock flooding (<= 3), too rough for a
        // slipway launch (<= 2 inclusive? boundary: 2 <= 2 passes) — use 3.
        let forecast = SeaStateForecast {
            hourly_sea_state: vec![3.0; 10],
        };
        let mut dock = slipway();
        dock.launch_method = LaunchMethod::DrydockFlooding;
        assert!(plan_launch_window(&dock, &forecast).is_ok());
        let mut side = slipway();
        side.launch_method = LaunchMethod::SideLaunch;
        assert!(plan_launch_window(&side, &forecast).is_err());
    }

    #[test]
    fn empty_forecast_rejected() {
        assert_eq!(
            plan_launch_window(&slipway(), &SeaStateForecast::default()),
            Err(LaunchWindowError::EmptyForecast)
        );
    }
}
