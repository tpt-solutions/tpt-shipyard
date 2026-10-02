//! Python bindings for tpt-shipyard (review 7H roadmap: PyO3 bindings,
//! first slice).
//!
//! Two entry points: a construction digital twin
//! ([`DigitalTwin`] — load a project JSON, drive the build phases,
//! read the weight report) and a prismatic hull form
//! ([`HullForm`] — hydrostatics and intact GZ curves). Errors surface
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
            ("max_gz_heel_deg".to_string(), v.max_gz_heel_deg),
            ("max_gz_m".to_string(), v.max_gz_m),
            ("gm_corrected_m".to_string(), v.gm_corrected_m),
        ])
    }
}

/// The tpt-shipyard Python module.
#[pymodule]
fn tpt_yard_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DigitalTwin>()?;
    m.add_class::<HullForm>()?;
    Ok(())
}
