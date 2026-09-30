import io
p = "crates/core/tpt-yard-digital-twin/src/lib.rs"
s = io.open(p, encoding="utf-8").read()

anchor = """    /// Serializes the twin's session state to JSON: the vessel project,"""
new = """    /// Ingests a live sensor/scan batch (review 7H roadmap item: "live
    /// digital-twin ingest of sensor and scan-deviation JSON driving
    /// distortion corrections").
    ///
    /// The batch schema (see `test-data/telemetry/sample-batch.json`):
    ///
    /// ```json
    /// { "readings": [{"block": 1, "quantity": "temperature_c", "value": 152.0}],
    ///   "scan_deviations": [{"block": 2, "axis_mm": 4.2}] }
    /// ```
    ///
    /// Readings append to `sensor_data`; scan deviations beyond the
    /// tolerance (a `distortion` correction plan is *not* auto-executed —
    /// the twin only flags) produce quality records with `accepted: false`,
    /// which the handover and dashboards surface as outstanding rework.
    ///
    /// # Errors
    ///
    /// [`TwinError::Malformed`] on a malformed batch.
    pub fn ingest_telemetry(
        &mut self,
        batch: &tpt_yard_core::json::Value,
    ) -> Result<TelemetrySummary, TwinError> {
        use tpt_yard_core::json::Value;
        let malformed = |m: &str| TwinError::Malformed(format!("telemetry: {m}"));
        const TOLERANCE_MM: f64 = 5.0;

        let mut summary = TelemetrySummary::default();

        if let Some(readings) = batch.get("readings").and_then(|r| r.as_array()) {
            for r in readings {
                let block = r
                    .get("block")
                    .and_then(|b| b.as_u64())
                    .ok_or_else(|| malformed("reading missing 'block'"))?;
                let quantity = r
                    .get("quantity")
                    .and_then(|q| q.as_str())
                    .ok_or_else(|| malformed("reading missing 'quantity'"))?
                    .to_string();
                let value = r
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| malformed("reading missing numeric 'value'"))?;
                self.sensor_data.push(SensorReading {
                    activity: ActivityId(block),
                    quantity,
                    value,
                    timestamp: None,
                });
                summary.readings_ingested += 1;
            }
        }

        if let Some(devs) = batch.get("scan_deviations").and_then(|d| d.as_array()) {
            for d in devs {
                let block = d
                    .get("block")
                    .and_then(|b| b.as_u64())
                    .ok_or_else(|| malformed("deviation missing 'block'"))?;
                let dev_mm = d
                    .get("axis_mm")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| malformed("deviation missing numeric 'axis_mm'"))?;
                summary.deviations_scanned += 1;
                if dev_mm.abs() > TOLERANCE_MM {
                    summary.corrections_flagged += 1;
                    self.quality_records.push(QualityRecord {
                        id: self.quality_records.len() as u64 + 1,
                        activity: ActivityId(block),
                        accepted: false,
                        note: format!(
                            "scan deviation {dev_mm:.1} mm exceeds the {TOLERANCE_MM} mm tolerance: flag for heat-straightening"
                        ),
                    });
                }
            }
        }

        Ok(summary)
    }

    /// Serializes the twin's session state to JSON: the vessel project,"""
assert anchor in s
s = s.replace(anchor, new, 1)

# TelemetrySummary struct before TwinError enum.
anchor2 = """pub enum TwinError {"""
summary = """/// Counts from one [`DigitalTwin::ingest_telemetry`] batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TelemetrySummary {
    /// Sensor readings stored.
    pub readings_ingested: usize,
    /// Scan deviations seen.
    pub deviations_scanned: usize,
    /// Deviations beyond tolerance flagged as quality rework.
    pub corrections_flagged: usize,
}

pub enum TwinError {"""
assert anchor2 in s
s = s.replace(anchor2, summary, 1)
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("ingest added")
