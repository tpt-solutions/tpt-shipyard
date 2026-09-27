# Quality & Inspection

`tpt-yard-quality` generates the NDT inspection plan from the build plan and
tracks defects through disposition.

## Inspection planning

[`generate_inspection_plan`](tpt_yard_quality::QualityManagement::generate_inspection_plan)
maps each activity to an inspection point by criticality (class-society
practice):

| Activity | Method | Criticality |
|---|---|---|
| `WeldBlock` / `JoinBlock` | UT (structural butts) | I |
| `Outfit` piping | PT | II |
| Pressure/hydro tests | Pressure test | I |
| Everything else | Visual | III |

Golden reference: `test-data/golden/quality/ndt-inspection-plan.json`.

## Defect tracking

[`defect_tracking`](tpt_yard_quality::QualityManagement::defect_tracking)
aggregates [`DefectRecord`](tpt_yard_quality::DefectRecord)s into a report:
counts by type (descending), repair rate, rejections to engineering, and the
dominant defect. Golden reference:
`weld-defect-tracking.json`.
