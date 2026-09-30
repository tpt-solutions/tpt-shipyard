import io
p = "crates/integration/tpt-yard-earth-link/src/lib.rs"
s = io.open(p, encoding="utf-8").read()

# The launch window limit = min(method practice limit, site operational limit).
old = """/// The sea-state limit by launch method (yard practice screening):
/// slipway end launch <= 2, side launch <= 1, dock flooding <= 3, shiplift <= 2.
fn sea_state_limit(analysis: &LaunchAnalysis) -> f64 {
    match analysis.launch_method {
        LaunchMethod::Slipway { .. } => 2.0,
        LaunchMethod::SideLaunch => 1.0,
        LaunchMethod::DrydockFlooding => 3.0,
        LaunchMethod::Shiplift { .. } => 2.0,
    }
}"""
new = """/// The sea-state limit by launch method (yard practice screening):
/// slipway end launch <= 2, side launch <= 1, dock flooding <= 3, shiplift <= 2.
fn method_sea_state_limit(analysis: &LaunchAnalysis) -> f64 {
    match analysis.launch_method {
        LaunchMethod::Slipway { .. } => 2.0,
        LaunchMethod::SideLaunch => 1.0,
        LaunchMethod::DrydockFlooding => 3.0,
        LaunchMethod::Shiplift { .. } => 2.0,
    }
}"""
assert old in s, "limit fn not found"
s = s.replace(old, new)

s = s.replace("""/// Finds the first safe window in the forecast for the launch method.
///
/// The window spans the longest (and first) run of hours at or below the
/// method's sea-state limit; the `earliest_hour` is that run's start.""",
"""/// Finds the first safe window in the forecast for the launch method.
///
/// The window spans the longest (and first) run of hours at or below the
/// sea-state limit — the *stricter* of the launch-method practice limit
/// and the site's operational `max_sea_state` (the site limit was ignored
/// before, review 7B); the `earliest_hour` is that run's start.""")

s = s.replace("""    let limit = sea_state_limit(analysis);""",
"""    let limit = method_sea_state_limit(analysis).min(analysis.site.max_sea_state as f64);""")

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("earth-link updated")
