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
