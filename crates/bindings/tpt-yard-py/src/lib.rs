//! Python bindings for tpt-shipyard (review 7H roadmap: PyO3 bindings,
//! first slice).
//!
//! Two entry points: a construction digital twin
//! ([`DigitalTwin`] — load a project JSON, drive the build phases,
//! read the weight report) and a prismatic hull form
//! ([`HullForm`] — hydrostatics and intact GZ curves; [`OffsetsHull`] — the
//! same from a real offsets table). Errors surface
//! as `ValueError` with the engine's message.
//!
#![allow(clippy::useless_conversion)]
// pyo3's #[pymethods] wrapper appends an `.into()` on the already-
// correct PyResult return, which clippy's useless_conversion flags
// inside the macro expansion (pyo3 0.22 + clippy 1.98) — not ours.

//! ```python
//! import tpt_yard_py
//! twin = tpt_yard_py.DigitalTwin.from_project_json(open("project.json").read())
//! twin.advance_phase(1)
//! report = twin.weight_report()
//! print(report["total_weight_kg"])
//! ```

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Activity tuple as passed from Python: `(id, duration_h, dep_ids,
/// resources)` with resources as `(kind_name, capacity)` pairs.
type PyActivity = (u64, f64, Vec<u64>, Vec<(String, f64)>);
/// Levelled-schedule return: `(makespan_h, order_ids, peaks, notes)`.
type PyLevelled = (f64, Vec<u64>, Vec<(String, f64)>, Vec<String>);

/// A construction digital twin over a vessel project.
#[pyclass]
struct DigitalTwin {
    inner: tpt_yard_digital_twin::DigitalTwin,
}

#[pymethods]
impl DigitalTwin {
    /// Loads a twin from a vessel-project JSON document (the strict
    /// engine loader — inconsistent files are rejected, not defaulted).
    #[staticmethod]
    fn from_project_json(project_json: &str) -> PyResult<Self> {
        let value = tpt_yard_core::json::Value::parse(project_json)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let project = tpt_yard_core::VesselProject::from_json_value(&value)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: tpt_yard_digital_twin::DigitalTwin::new(project),
        })
    }

    /// Completes `activity_id` and advances the build phase when its
    /// phase is finished. Activities ahead of the current phase are
    /// rejected by the engine.
    fn advance_phase(&mut self, activity_id: u64) -> PyResult<()> {
        self.inner
            .advance_phase(&tpt_yard_core::ActivityId(activity_id))
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// The twin state as a JSON document (round-trips through
    /// `from_project_json`-style strict loading via the engine's
    /// persistence format).
    fn to_json(&self) -> PyResult<String> {
        Ok(self
            .inner
            .to_json()
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .to_string_compact())
    }

    /// The weight report: total/installed/design weights, deviation,
    /// margin and centre of gravity of the best-estimate total.
    fn weight_report(&self) -> HashMap<String, f64> {
        let r = self.inner.weight_model.weight_report();
        HashMap::from([
            ("total_weight_kg".to_string(), r.total_weight_kg),
            ("installed_weight_kg".to_string(), r.installed_weight_kg),
            ("design_weight_kg".to_string(), r.design_weight_kg),
            ("deviation_kg".to_string(), r.deviation_kg),
            ("margin_kg".to_string(), r.margin_kg),
            ("cog_x_m".to_string(), r.cog.x),
            ("cog_y_m".to_string(), r.cog.y),
            ("cog_z_m".to_string(), r.cog.z),
        ])
    }
}

/// A prismatic hull form for hydrostatics and intact stability.
#[pyclass]
struct HullForm {
    inner: tpt_yard_hydrostatics::HullForm,
}

#[pymethods]
impl HullForm {
    /// Creates a hull from its principal dimensions: length and beam
    /// overall, block and waterplane coefficients.
    #[new]
    #[pyo3(signature = (loa_m, boa_m, cb, cwp))]
    fn new(loa_m: f64, boa_m: f64, cb: f64, cwp: f64) -> Self {
        Self {
            inner: tpt_yard_hydrostatics::HullForm {
                loa_m,
                boa_m,
                cb,
                cwp,
            },
        }
    }

    /// Hydrostatics at a draft: displacement, KB, KM, TPC, MCT1cm and
    /// the waterplane area, keyed by name.
    fn hydrostatics(&self, draft_m: f64) -> HashMap<String, f64> {
        let h = self.inner.hydrostatics(draft_m);
        HashMap::from([
            ("draft_m".to_string(), h.draft_m),
            ("displacement_t".to_string(), h.displacement_t),
            ("kb_m".to_string(), h.kb_m),
            ("km_m".to_string(), h.km_m),
            ("tpc_t_cm".to_string(), h.tpc_t_cm),
            ("mct1cm_tm_cm".to_string(), h.mct1cm_tm_cm),
            ("waterplane_area_m2".to_string(), h.waterplane_area_m2),
        ])
    }

    /// The wall-sided GZ curve from 0 to `to_deg` with the
    /// free-surface correction: a list of `(heel_deg, gz_m)` pairs.
    fn gz_curve(&self, draft_m: f64, kg_m: f64, to_deg: f64) -> Vec<(f64, f64)> {
        let loading = tpt_yard_hydrostatics::LoadingCondition::new(draft_m, kg_m);
        self.inner
            .gz_curve(loading, to_deg)
            .points
            .iter()
            .map(|p| (p.heel_deg, p.gz_m))
            .collect()
    }

    /// The IMO 2008 IS Code general-criteria verdict at a draft:
    /// `passed` plus each criterion value.
    fn imo_2008_check(&self, draft_m: f64, kg_m: f64) -> HashMap<String, f64> {
        let loading = tpt_yard_hydrostatics::LoadingCondition::new(draft_m, kg_m);
        let gz = self.inner.gz_curve(loading, 80.0);
        let v = self.inner.imo_2008_general(&gz, draft_m);
        HashMap::from([
            ("passed".to_string(), if v.passed { 1.0 } else { 0.0 }),
            ("area_to_30_deg_mrad".to_string(), v.area_to_30_deg),
            ("area_to_40_deg_mrad".to_string(), v.area_to_40_deg),
            ("area_30_to_40_deg_mrad".to_string(), v.area_30_to_40_deg),
            ("gz_at_30_deg_m".to_string(), v.gz_at_30_deg_m),
            ("max_gz_heel_deg".to_string(), v.max_gz_heel_deg),
            ("max_gz_m".to_string(), v.max_gz_m),
            ("gm_corrected_m".to_string(), v.gm_corrected_m),
        ])
    }
}

/// A hull defined by an offsets table (the lines-plan form behind
/// `tpt-yard stability`): hydrostatics, exact cross-curve GZ curves and the
/// IMO 2008 criteria for a real hull form rather than a prismatic screen.
#[pyclass]
struct OffsetsHull {
    inner: tpt_yard_hydrostatics::Bonjean,
}

#[pymethods]
impl OffsetsHull {
    /// Parses an offsets CSV (`station_x_m, draft_m, half_breadth_m`; see
    /// `schemas/hull-offsets.table-schema.json`).
    #[staticmethod]
    fn from_csv(text: &str) -> PyResult<Self> {
        let inner =
            tpt_yard_hydrostatics::parse_offsets_csv(text).map_err(PyValueError::new_err)?;
        Ok(Self { inner })
    }

    /// Reads an offsets CSV file.
    #[staticmethod]
    fn from_csv_file(path: &str) -> PyResult<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| PyValueError::new_err(format!("reading {path}: {e}")))?;
        Self::from_csv(&text)
    }

    /// Hydrostatics at an upright even-keel draft, keyed by name.
    fn hydrostatics(&self, draft_m: f64) -> PyResult<HashMap<String, f64>> {
        let h = self.inner.hydrostatics(draft_m).ok_or_else(|| {
            PyValueError::new_err("the draft must be positive and within the offsets table")
        })?;
        Ok(HashMap::from([
            ("draft_m".to_string(), h.draft_m),
            ("displacement_t".to_string(), h.displacement_t),
            ("kb_m".to_string(), h.kb_m),
            ("km_m".to_string(), h.km_m),
            ("lcb_m".to_string(), h.lcb_m),
            ("lcf_m".to_string(), h.lcf_m),
            ("tpc_t_cm".to_string(), h.tpc_t_cm),
            ("mct1cm_tm_cm".to_string(), h.mct1cm_tm_cm),
            ("waterplane_area_m2".to_string(), h.waterplane_area_m2),
        ]))
    }

    /// The GZ curve from 0 to `to_deg` (every `step_deg`) as
    /// `(heel_deg, gz_m)` pairs.
    #[pyo3(signature = (draft_m, kg_m, to_deg=60.0, step_deg=5.0, free_surface_moment_tm=0.0))]
    fn gz_curve(
        &self,
        draft_m: f64,
        kg_m: f64,
        to_deg: f64,
        step_deg: f64,
        free_surface_moment_tm: f64,
    ) -> PyResult<Vec<(f64, f64)>> {
        Ok(self
            .curve(draft_m, kg_m, to_deg, step_deg, free_surface_moment_tm)?
            .points
            .iter()
            .map(|p| (p.heel_deg, p.gz_m))
            .collect())
    }

    /// The IMO 2008 IS Code general criteria for a loading: `passed`
    /// (1.0/0.0) plus each criterion value.
    #[pyo3(signature = (draft_m, kg_m, free_surface_moment_tm=0.0))]
    fn imo_2008_check(
        &self,
        draft_m: f64,
        kg_m: f64,
        free_surface_moment_tm: f64,
    ) -> PyResult<HashMap<String, f64>> {
        let gz = self.curve(draft_m, kg_m, 60.0, 1.0, free_surface_moment_tm)?;
        let v = tpt_yard_hydrostatics::imo_2008_general_criteria(&gz);
        Ok(HashMap::from([
            ("passed".to_string(), if v.passed { 1.0 } else { 0.0 }),
            ("area_to_30_deg_mrad".to_string(), v.area_to_30_deg),
            ("area_to_40_deg_mrad".to_string(), v.area_to_40_deg),
            ("area_30_to_40_deg_mrad".to_string(), v.area_30_to_40_deg),
            ("gz_at_30_deg_m".to_string(), v.gz_at_30_deg_m),
            ("max_gz_heel_deg".to_string(), v.max_gz_heel_deg),
            ("max_gz_m".to_string(), v.max_gz_m),
            ("gm_corrected_m".to_string(), v.gm_corrected_m),
        ]))
    }
}

impl OffsetsHull {
    fn curve(
        &self,
        draft_m: f64,
        kg_m: f64,
        to_deg: f64,
        step_deg: f64,
        fsm: f64,
    ) -> PyResult<tpt_yard_hydrostatics::GzCurve> {
        if !(fsm.is_finite() && fsm >= 0.0) {
            return Err(PyValueError::new_err(
                "the free-surface moment must be a non-negative number",
            ));
        }
        let loading =
            tpt_yard_hydrostatics::LoadingCondition::new(draft_m, kg_m).with_free_surface_tm(fsm);
        self.inner
            .gz_curve(loading, to_deg, step_deg)
            .ok_or_else(|| {
                PyValueError::new_err(
                    "no GZ curve: check the draft is within the table and 0 < step, to_deg <= 180",
                )
            })
    }
}

/// A construction scheduler over an activity network (review 7H: the
/// planning bindings). Activities come as `(id, duration_h, dep_ids,
/// resources)` tuples with resources as `(kind, capacity)` pairs using
/// the kind names `crane|workshop|drydock|welding|robot|crew|transport`.
#[pyclass]
struct Scheduler {
    inner: tpt_yard_scheduling::ShipyardScheduler,
}

fn resource_kind(name: &str) -> Option<tpt_yard_core::ResourceKind> {
    match name {
        "crane" => Some(tpt_yard_core::ResourceKind::Crane),
        "workshop" => Some(tpt_yard_core::ResourceKind::Workshop),
        "drydock" => Some(tpt_yard_core::ResourceKind::Drydock),
        "welding" => Some(tpt_yard_core::ResourceKind::WeldingStation),
        "robot" => Some(tpt_yard_core::ResourceKind::Robot),
        "crew" => Some(tpt_yard_core::ResourceKind::Crew),
        "transport" => Some(tpt_yard_core::ResourceKind::Transport),
        _ => None,
    }
}

fn kind_name(k: tpt_yard_core::ResourceKind) -> &'static str {
    tpt_yard_scheduling::resource_kind_name(k)
}

#[pymethods]
impl Scheduler {
    #[new]
    fn new(activities: Vec<PyActivity>) -> PyResult<Self> {
        let acts = activities
            .iter()
            .map(|(id, duration, deps, resources)| {
                let resources = resources
                    .iter()
                    .map(|(kind_name, capacity)| {
                        let kind = resource_kind(kind_name).ok_or_else(|| {
                            PyValueError::new_err(format!(
                                "unknown resource kind '{kind_name}' (use crane|workshop|drydock|welding|robot|crew|transport)"
                            ))
                        })?;
                        Ok(tpt_yard_core::Resource {
                            name: kind_name.clone(),
                            kind,
                            capacity: *capacity,
                        })
                    })
                    .collect::<PyResult<Vec<_>>>()?;
                Ok(tpt_yard_core::AssemblyActivity::new(
                    tpt_yard_core::ActivityId(*id),
                    format!("activity {id}"),
                    tpt_yard_core::ActivityType::JoinBlock,
                    *duration,
                )
                .with_dependencies(&deps.iter().map(|d| tpt_yard_core::ActivityId(*d)).collect::<Vec<_>>())
                .with_resources(resources))
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(Self {
            inner: tpt_yard_scheduling::ShipyardScheduler::new(acts),
        })
    }

    /// The zero-float critical path as activity ids.
    fn critical_path(&self) -> PyResult<Vec<u64>> {
        Ok(self
            .inner
            .critical_path()
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .into_iter()
            .map(|a| a.0)
            .collect())
    }

    /// Schedules under an objective named `min_duration|min_crane|min_cost|
    /// max_parallel|min_drydock`: returns `(makespan_h, order_ids,
    /// dock_occupancy_h)`.
    fn optimize(&self, objective: &str) -> PyResult<(f64, Vec<u64>, Option<f64>)> {
        use tpt_yard_scheduling::ScheduleObjective as O;
        let objective = match objective {
            "min_duration" => O::MinimizeDuration,
            "min_crane" => O::MinimizeCraneUsage,
            "min_cost" => O::MinimizeCost,
            "max_parallel" => O::MaximizeParallelism,
            "min_drydock" => O::MinimizeDrydockTime,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown objective '{other}'"
                )))
            }
        };
        let r = self
            .inner
            .optimize_sequence(objective)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok((
            r.makespan_hours,
            r.order.iter().map(|a| a.0).collect(),
            r.dock_occupancy_h,
        ))
    }

    /// Capacity-aware resource levelling: `limits` maps kind names to
    /// yard-wide capacities. Returns `(makespan_h, order_ids, peaks,
    /// notes)`.
    fn resource_leveling_with_limits(&self, limits: HashMap<String, f64>) -> PyResult<PyLevelled> {
        let mut limit_kinds = std::collections::BTreeMap::new();
        for (name, value) in &limits {
            let kind = resource_kind(name)
                .ok_or_else(|| PyValueError::new_err(format!("unknown resource kind '{name}'")))?;
            limit_kinds.insert(kind, *value);
        }
        let r = self
            .inner
            .resource_leveling_with_limits(&limit_kinds)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok((
            r.makespan_hours,
            r.order.iter().map(|a| a.0).collect(),
            r.peak_resource_use
                .iter()
                .map(|(k, v)| (kind_name(*k).to_string(), *v))
                .collect(),
            r.notes,
        ))
    }

    /// Monte Carlo schedule risk with triangular duration uncertainty:
    /// returns `(p50_h, p90_h, mean_h)`; seed-deterministic.
    fn monte_carlo_risk(
        &self,
        uncertainty_frac: f64,
        samples: u32,
        seed: u64,
    ) -> PyResult<(f64, f64, f64)> {
        let r = self
            .inner
            .monte_carlo_risk(uncertainty_frac, samples, seed)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok((r.p50_makespan_h, r.p90_makespan_h, r.mean_makespan_h))
    }
}

/// The tpt-shipyard Python module.
#[pymodule]
fn tpt_yard_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DigitalTwin>()?;
    m.add_class::<HullForm>()?;
    m.add_class::<OffsetsHull>()?;
    m.add_class::<Scheduler>()?;
    Ok(())
}
