//! User-supplied rule tables for minimum scantlings.
//!
//! Classification-society minimum thicknesses are rule content: they differ
//! between societies, change with every edition, and carry the society's
//! copyright. This engine therefore ships **no** class values. A yard loads
//! its own society's table from JSON and the engine only applies it, so the
//! rule data stays in the user's hands and never goes stale in the repo.
//!
//! A rule is the common closed form
//!
//! ```text
//! t_min = base_mm + coeff_mm * L^length_exponent    (then clamped to [min_mm, max_mm])
//! ```
//!
//! in the ship length `L` (m), tagged *net* (the corrosion addition is still
//! to be added) or *gross* (as built). A constant minimum omits `coeff_mm`.
//!
//! ```json
//! { "society": "Your society", "edition": "2026",
//!   "min_thickness": [
//!     { "member": "bottom shell", "basis": "net", "base_mm": 5.0,
//!       "coeff_mm": 0.04, "length_exponent": 1.0, "max_mm": 16.0 },
//!     { "member": "tank boundary", "basis": "gross", "base_mm": 7.5 } ] }
//! ```
//!
//! The loader is strict (no silent defaults): every field above is required
//! except `coeff_mm` / `length_exponent` (a pair: both or neither) and the
//! `min_mm` / `max_mm` clamps. Duplicate members, negative or non-finite
//! numbers and out-of-range lengths are typed errors.

use std::fmt;

use tpt_yard_core::json::Value;

use crate::LocalPlateScantling;

/// Whether a rule minimum excludes or includes the corrosion addition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThicknessBasis {
    /// Net thickness: the corrosion addition is added on top.
    Net,
    /// Gross (as-built) thickness: already includes any corrosion margin.
    Gross,
}

/// One minimum-thickness rule.
#[derive(Debug, Clone, PartialEq)]
pub struct MinThicknessRule {
    /// Member name, matched exactly (after trimming) by lookups.
    pub member: String,
    /// Net or gross.
    pub basis: ThicknessBasis,
    /// Constant term, mm.
    pub base_mm: f64,
    /// `(coeff_mm, length_exponent)` of the length-dependent term.
    pub length_term: Option<(f64, f64)>,
    /// Lower clamp, mm.
    pub min_mm: Option<f64>,
    /// Upper clamp, mm.
    pub max_mm: Option<f64>,
}

impl MinThicknessRule {
    /// The rule minimum for a ship length, mm (clamps applied).
    #[must_use]
    pub fn thickness_mm(&self, length_m: f64) -> f64 {
        let mut t = self.base_mm;
        if let Some((coeff, exponent)) = self.length_term {
            t += coeff * length_m.powf(exponent);
        }
        if let Some(lo) = self.min_mm {
            t = t.max(lo);
        }
        if let Some(hi) = self.max_mm {
            t = t.min(hi);
        }
        t
    }
}

/// Errors from loading or applying a rule table.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleTableError {
    /// The document is malformed; the message names the field.
    Malformed(String),
    /// No rule for this member.
    UnknownMember(String),
    /// The ship length is not a positive finite number.
    InvalidLength,
}

impl fmt::Display for RuleTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleTableError::Malformed(m) => write!(f, "rule table: {m}"),
            RuleTableError::UnknownMember(m) => write!(f, "rule table has no rule for '{m}'"),
            RuleTableError::InvalidLength => f.write_str("ship length must be positive and finite"),
        }
    }
}

impl std::error::Error for RuleTableError {}

/// Which side set the required thickness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleGoverned {
    /// The calculated (bending/buckling) scantling.
    Calculation,
    /// The rule minimum.
    RuleMinimum,
}

/// A calculated scantling checked against a rule minimum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuleCheckedScantling {
    /// Calculated gross thickness (governing net + corrosion), mm.
    pub calculated_gross_mm: f64,
    /// Rule minimum as a gross thickness, mm.
    pub rule_minimum_gross_mm: f64,
    /// The larger of the two, mm.
    pub required_gross_mm: f64,
    /// Which one set it.
    pub governed_by: RuleGoverned,
}

/// A user-supplied set of minimum-thickness rules.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleTable {
    /// Society name (free text, for reports).
    pub society: String,
    /// Edition label (free text, for reports).
    pub edition: String,
    /// The rules.
    pub rules: Vec<MinThicknessRule>,
}

fn num(v: &Value, field: &str, ctx: &str) -> Result<f64, RuleTableError> {
    v.get(field)
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite())
        .ok_or_else(|| RuleTableError::Malformed(format!("{ctx}: '{field}' must be a number")))
}

fn opt_num(v: &Value, field: &str, ctx: &str) -> Result<Option<f64>, RuleTableError> {
    match v.get(field) {
        None => Ok(None),
        Some(_) => num(v, field, ctx).map(Some),
    }
}

impl RuleTable {
    /// Loads a table from a parsed JSON document (see the module docs).
    ///
    /// # Errors
    ///
    /// [`RuleTableError::Malformed`] naming the offending field.
    pub fn from_json_value(doc: &Value) -> Result<Self, RuleTableError> {
        let text = |f: &str| {
            doc.get(f)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| RuleTableError::Malformed(format!("'{f}' must be a string")))
        };
        let (society, edition) = (text("society")?, text("edition")?);
        let items = doc
            .get("min_thickness")
            .and_then(Value::as_array)
            .ok_or_else(|| RuleTableError::Malformed("'min_thickness' must be an array".into()))?;
        let mut rules: Vec<MinThicknessRule> = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            let ctx = format!("min_thickness[{i}]");
            let member = item
                .get("member")
                .and_then(Value::as_str)
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty())
                .ok_or_else(|| {
                    RuleTableError::Malformed(format!("{ctx}: 'member' must be a name"))
                })?;
            if rules.iter().any(|r| r.member == member) {
                return Err(RuleTableError::Malformed(format!(
                    "{ctx}: duplicate member '{member}'"
                )));
            }
            let basis = match item.get("basis").and_then(Value::as_str) {
                Some("net") => ThicknessBasis::Net,
                Some("gross") => ThicknessBasis::Gross,
                _ => {
                    return Err(RuleTableError::Malformed(format!(
                        "{ctx}: 'basis' must be \"net\" or \"gross\""
                    )))
                }
            };
            let base_mm = num(item, "base_mm", &ctx)?;
            let coeff = opt_num(item, "coeff_mm", &ctx)?;
            let exponent = opt_num(item, "length_exponent", &ctx)?;
            let length_term = match (coeff, exponent) {
                (Some(c), Some(e)) => Some((c, e)),
                (None, None) => None,
                _ => {
                    return Err(RuleTableError::Malformed(format!(
                        "{ctx}: 'coeff_mm' and 'length_exponent' go together"
                    )))
                }
            };
            let (min_mm, max_mm) = (
                opt_num(item, "min_mm", &ctx)?,
                opt_num(item, "max_mm", &ctx)?,
            );
            if base_mm < 0.0
                || coeff.is_some_and(|c| c < 0.0)
                || min_mm.is_some_and(|m| m < 0.0)
                || max_mm.is_some_and(|m| m <= 0.0)
                || matches!((min_mm, max_mm), (Some(lo), Some(hi)) if lo > hi)
            {
                return Err(RuleTableError::Malformed(format!(
                    "{ctx}: thickness terms must be non-negative and min <= max"
                )));
            }
            rules.push(MinThicknessRule {
                member,
                basis,
                base_mm,
                length_term,
                min_mm,
                max_mm,
            });
        }
        Ok(Self {
            society,
            edition,
            rules,
        })
    }

    /// Loads a table from JSON text.
    ///
    /// # Errors
    ///
    /// [`RuleTableError::Malformed`] for a syntax error or a bad field.
    pub fn from_json_str(text: &str) -> Result<Self, RuleTableError> {
        let v = Value::parse(text).map_err(|e| RuleTableError::Malformed(e.to_string()))?;
        Self::from_json_value(&v)
    }

    /// The rule for a member.
    ///
    /// # Errors
    ///
    /// [`RuleTableError::UnknownMember`] when absent.
    pub fn rule(&self, member: &str) -> Result<&MinThicknessRule, RuleTableError> {
        let m = member.trim();
        self.rules
            .iter()
            .find(|r| r.member == m)
            .ok_or_else(|| RuleTableError::UnknownMember(m.to_string()))
    }

    /// The rule minimum for a member and ship length, as `(mm, basis)`.
    ///
    /// # Errors
    ///
    /// [`RuleTableError`] for an unknown member or a bad length.
    pub fn min_thickness_mm(
        &self,
        member: &str,
        length_m: f64,
    ) -> Result<(f64, ThicknessBasis), RuleTableError> {
        if !(length_m.is_finite() && length_m > 0.0) {
            return Err(RuleTableError::InvalidLength);
        }
        let rule = self.rule(member)?;
        Ok((rule.thickness_mm(length_m), rule.basis))
    }

    /// Checks a calculated local plate scantling against the member's rule
    /// minimum: a net rule gets the scantling's own corrosion addition
    /// (`with_corrosion - governing_net`) added; a gross rule is used as is.
    ///
    /// # Errors
    ///
    /// [`RuleTableError`] for an unknown member or a bad length.
    pub fn check_plate(
        &self,
        member: &str,
        length_m: f64,
        scantling: &LocalPlateScantling,
    ) -> Result<RuleCheckedScantling, RuleTableError> {
        let (min, basis) = self.min_thickness_mm(member, length_m)?;
        let corrosion = scantling.with_corrosion_mm - scantling.governing_net_mm;
        let rule_gross = match basis {
            ThicknessBasis::Net => min + corrosion,
            ThicknessBasis::Gross => min,
        };
        let calc = scantling.with_corrosion_mm;
        Ok(RuleCheckedScantling {
            calculated_gross_mm: calc,
            rule_minimum_gross_mm: rule_gross,
            required_gross_mm: calc.max(rule_gross),
            governed_by: if rule_gross > calc {
                RuleGoverned::RuleMinimum
            } else {
                RuleGoverned::Calculation
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{local_plate_scantling, LocalPlateScantlingInput};

    const DOC: &str = r#"{
      "society": "Test society", "edition": "illustrative",
      "min_thickness": [
        { "member": "bottom shell", "basis": "net", "base_mm": 5.0,
          "coeff_mm": 0.04, "length_exponent": 1.0, "max_mm": 16.0 },
        { "member": "tank boundary", "basis": "gross", "base_mm": 7.5 },
        { "member": "deck", "basis": "net", "base_mm": 1.0,
          "coeff_mm": 0.5, "length_exponent": 0.5, "min_mm": 6.0 }
      ] }"#;

    fn plate(pressure: f64, corrosion: f64) -> LocalPlateScantling {
        local_plate_scantling(LocalPlateScantlingInput {
            spacing_m: 0.7,
            long_span_m: None,
            pressure_kn_m2: pressure,
            allowable_bending_mpa: 160.0,
            material_factor_k: 1.0,
            boundary_factor: 1.0,
            corrosion_addition_mm: corrosion,
            applied_compression_mpa: None,
            youngs_modulus_gpa: 206.0,
        })
        .expect("valid")
    }

    #[test]
    fn rules_evaluate_by_hand_with_clamps() {
        let t = RuleTable::from_json_str(DOC).expect("loads");
        // 5 + 0.04 * 140 = 10.6
        let (m, b) = t.min_thickness_mm("bottom shell", 140.0).unwrap();
        assert!((m - 10.6).abs() < 1e-12 && b == ThicknessBasis::Net);
        // capped at 16: 5 + 0.04 * 400 = 21 -> 16
        assert_eq!(t.min_thickness_mm(" bottom shell ", 400.0).unwrap().0, 16.0);
        // constant gross rule
        assert_eq!(
            t.min_thickness_mm("tank boundary", 50.0).unwrap(),
            (7.5, ThicknessBasis::Gross)
        );
        // 1 + 0.5 sqrt(4) = 2 -> floored at 6
        assert_eq!(t.min_thickness_mm("deck", 4.0).unwrap().0, 6.0);
        // 1 + 0.5 sqrt(144) = 7 > floor
        assert!((t.min_thickness_mm("deck", 144.0).unwrap().0 - 7.0).abs() < 1e-12);
    }

    #[test]
    fn the_minimum_governs_a_light_plate_and_the_calculation_a_heavy_one() {
        let t = RuleTable::from_json_str(DOC).unwrap();
        // Light load: the calculation is thin, so the net rule minimum
        // (10.6) plus the 1.5 mm corrosion addition governs.
        let light = plate(10.0, 1.5);
        let r = t.check_plate("bottom shell", 140.0, &light).unwrap();
        assert_eq!(r.governed_by, RuleGoverned::RuleMinimum);
        assert!((r.rule_minimum_gross_mm - 12.1).abs() < 1e-12);
        assert!((r.required_gross_mm - 12.1).abs() < 1e-12);
        // Heavy load: the calculated scantling exceeds the rule minimum.
        let heavy = plate(400.0, 1.5);
        let r = t.check_plate("bottom shell", 140.0, &heavy).unwrap();
        assert_eq!(r.governed_by, RuleGoverned::Calculation);
        assert!((r.required_gross_mm - heavy.with_corrosion_mm).abs() < 1e-12);
        // A gross rule ignores the corrosion addition.
        let r = t.check_plate("tank boundary", 140.0, &light).unwrap();
        assert!((r.rule_minimum_gross_mm - 7.5).abs() < 1e-12);
    }

    /// The shipped sample parses, and the schema declares every key the
    /// loader reads (so the two cannot drift).
    #[test]
    fn the_sample_file_loads_and_matches_the_schema() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");
        let read = |p: &str| std::fs::read_to_string(format!("{root}/{p}")).expect("reads");
        let sample = read("test-data/rules/example-rule-table.json");
        let table = RuleTable::from_json_str(&sample).expect("sample loads");
        assert_eq!(table.rules.len(), 3);
        let schema = Value::parse(&read("schemas/scantling-rule-table.schema.json")).unwrap();
        let declared = |v: &Value| -> Vec<String> {
            v.get("properties")
                .and_then(Value::as_object)
                .expect("properties")
                .iter()
                .map(|(k, _)| k.clone())
                .collect()
        };
        let doc = Value::parse(&sample).unwrap();
        let top = declared(&schema);
        for (k, _) in doc.as_object().unwrap() {
            assert!(top.contains(k), "top-level '{k}' undeclared");
        }
        let item_schema = schema
            .get("properties")
            .and_then(|p| p.get("min_thickness"))
            .and_then(|m| m.get("items"))
            .expect("items");
        let item_keys = declared(item_schema);
        for rule in doc.get("min_thickness").and_then(Value::as_array).unwrap() {
            for (k, _) in rule.as_object().unwrap() {
                assert!(item_keys.contains(k), "rule key '{k}' undeclared");
            }
        }
    }

    #[test]
    fn bad_tables_and_lookups_are_typed_errors() {
        let member =
            |body: &str| format!(r#"{{"society":"s","edition":"e","min_thickness":[{body}]}}"#);
        for (doc, what) in [
            (r#"{}"#.to_string(), "society"),
            (
                r#"{"society":"s","edition":"e"}"#.to_string(),
                "min_thickness",
            ),
            (
                member(r#"{"member":"a","basis":"mean","base_mm":1}"#),
                "basis",
            ),
            (
                member(r#"{"member":"a","basis":"net","base_mm":-1}"#),
                "negative base",
            ),
            (
                member(r#"{"member":"a","basis":"net","base_mm":1,"coeff_mm":0.1}"#),
                "coeff without exponent",
            ),
            (
                member(r#"{"member":"a","basis":"net","base_mm":1,"min_mm":9,"max_mm":3}"#),
                "min above max",
            ),
            (
                member(
                    r#"{"member":"a","basis":"net","base_mm":1},{"member":" a ","basis":"net","base_mm":2}"#,
                ),
                "duplicate",
            ),
            (
                member(r#"{"member":"a","basis":"net","base_mm":"x"}"#),
                "text number",
            ),
            ("not json".to_string(), "syntax"),
        ] {
            assert!(
                matches!(
                    RuleTable::from_json_str(&doc),
                    Err(RuleTableError::Malformed(_))
                ),
                "{what} must be rejected"
            );
        }
        let t = RuleTable::from_json_str(DOC).unwrap();
        assert_eq!(
            t.min_thickness_mm("keel", 100.0),
            Err(RuleTableError::UnknownMember("keel".into()))
        );
        for bad in [0.0, -1.0, f64::NAN] {
            assert_eq!(
                t.min_thickness_mm("deck", bad),
                Err(RuleTableError::InvalidLength)
            );
        }
    }
}
