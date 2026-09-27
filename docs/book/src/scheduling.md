# Scheduling

`tpt-yard-scheduling` answers the planner's three questions on the activity
network.

## Critical path

The CPM pass computes earliest/latest starts and float per activity;
[`critical_path`](tpt_yard_scheduling::ShipyardScheduler::critical_path)
returns the zero-float chain that drives the makespan. Golden reference:
`test-data/golden/planning/critical-path-schedule.json` (34 h network with
a 4 h float branch).

## Objectives

[`optimize_sequence`](tpt_yard_scheduling::ShipyardScheduler::optimize_sequence)
schedules under a [`ScheduleObjective`](tpt_yard_scheduling::ScheduleObjective):
earliest starts for duration/parallelism objectives; a serial
schedule-generation scheme (priority = minimum float first) for the
cost/crane-usage objectives.

## Resource levelling

[`resource_leveling`](tpt_yard_scheduling::ShipyardScheduler::resource_leveling)
places each activity at the earliest time where its resources clash with
nothing already placed — a true serial RCPSP pass. Levelling may legitimately
stretch the makespan to flatten peaks; the golden case trades 32 h -> 40 h
for a crane peak of 2 -> 1 (`resource-leveling.json`).
