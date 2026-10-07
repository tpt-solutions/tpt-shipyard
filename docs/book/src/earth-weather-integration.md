# Earth/Weather Integration

`tpt-yard-earth-link` gates launch decisions on the sea state — the
`tpt-earth`-shaped forecast vendored until the substrate publishes.

## Launch windows

A [`SeaStateForecast`](tpt_yard_earth_link::SeaStateForecast) carries
Douglas-scale sea state per forecast hour. Each launch method has a
screening limit:

| Method | Max sea state |
|---|---|
| Side launch | 1 |
| Slipway end launch | 2 |
| Shiplift | 2 |
| Drydock flooding | 3 |

[`plan_launch_window`](tpt_yard_earth_link::plan_launch_window) returns the
longest calm run: first safe hour, exclusive end, the governing limit, and
a plain-language note. Never-safe forecasts are an explicit error, not an
empty window.

## Verification

`storm_splits_the_day` puts a storm across the middle of the day and asserts
the window opens at the longer calm run; method-limit differences are tested
(the same forecast clears dock flooding but blocks a side launch).

## From wave height

`tpt-earth`'s ocean-waves model reports significant wave height rather than a
Douglas state, and the crate is not on crates.io, so the bridge takes the
number instead of the dependency:
[`douglas_from_hs`](tpt_yard_earth_link::douglas_from_hs) applies the WMO
sea-state table (0 calm at 0 m; 1 to 0.1 m; 2 to 0.5 m; 3 to 1.25 m; 4 to
2.5 m; 5 to 4 m; 6 to 6 m; 7 to 9 m; 8 to 14 m; 9 above, upper bounds
inclusive), and
[`SeaStateForecast::from_significant_wave_heights`](tpt_yard_earth_link::SeaStateForecast::from_significant_wave_heights)
converts a whole forecast. Forecast files may carry `"hourly_hs_m": [...]`
instead of `hourly_sea_state` (which wins when both are present), so
`tpt-yard risk --weather forecast.json` accepts either. Negative or
non-finite heights are typed errors naming the hour.
