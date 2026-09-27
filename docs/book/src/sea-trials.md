# Sea Trials

`tpt-yard-sea-trials` plans the post-launch trial program and judges the
measurements against acceptance criteria.

## Trials

A [`Trial`](tpt_yard_sea_trials::Trial) pairs a category (speed,
manoeuvring, crash stop, seakeeping, noise & vibration, endurance,
inclining, class acceptance) with an
[`Acceptance`](tpt_yard_sea_trials::Acceptance) window on a metric:

```rust
Acceptance { metric: Metric::SpeedKn, minimum: Some(15.0), maximum: None }
```

Bounds are inclusive; a one-sided window leaves the other end `None`.

## Evaluation

[`TrialProgram::evaluate`](tpt_yard_sea_trials::TrialProgram::evaluate)
checks every trial's measured value: `Ok(())` inside the window,
`Err(reason)` outside, and `Err("not performed")` when no measurement
exists — an incomplete program never passes, which is exactly what the
delivery gate needs. The resulting [`TrialReport`](tpt_yard_sea_trials::TrialReport)
carries per-trial outcomes and the passed/total count.
