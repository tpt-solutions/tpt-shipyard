//! Weld-procedure advisor (review 7H): carbon equivalents, t8/5 cooling
//! windows and WPS/PQR qualification mapping.
//!
//! Companion to the crate's existing `SteelChemistry` /
//! [`crate::advise_preheat`] screening: this module adds the EN 1011-2
//! CET and Ito–Bessyo Pcm equivalents, the Graville class, the closed-form
//! preheat that drives the Rosenthal t8/5 to a target cooling time, an
//! end-to-end [`advise`] screening against a supplier t8/5 window, and the
//! WPS-inside-PQR envelope checks in the spirit of ISO 15614-1 / ASME IX.
//!
//! Everything here is a screening aid for planning, not a substitute for
//! the welding procedure specification or the qualification record itself.

use crate::{SteelChemistry, WeldProcedure, WeldProcess};
use tpt_yard_core::Material;

/// Graville weldability class: whether cold cracking is dominated by
/// hydrogen and heat input (class I) or by hardenability (class III).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GravilleClass {
    /// C < 0.09 %; chemistry cannot cause HAZ cracking on its own.
    ClassI,
    /// C ≥ 0.09 % with CE(IIW) < 0.40; moderate sensitivity.
    ClassII,
    /// CE(IIW) ≥ 0.40; preheat and heat-input management required.
    ClassIII,
}

impl SteelChemistry {
    /// EN 1011-2 carbon equivalent CET (better fit for low-carbon TMCP
    /// steels): `CET = C + (Mn + Mo)/10 + (Cr + Cu)/20 + Ni/40`.
    pub fn cet(&self) -> f64 {
        self.c + (self.mn + self.mo) / 10.0 + (self.cr + self.cu) / 20.0 + self.ni / 40.0
    }

    /// Ito–Bessyo crack parameter Pcm (cold-cracking sensitivity of
    /// low-carbon steels):
    /// `Pcm = C + Si/30 + (Mn + Cu + Cr)/20 + Ni/60 + Mo/15 + V/10 + 5B`,
    /// with boron taken as 0 (trace levels; the chemistry struct does not
    /// carry it).
    pub fn pcm(&self) -> f64 {
        self.c
            + self.si / 30.0
            + (self.mn + self.cu + self.cr) / 20.0
            + self.ni / 60.0
            + self.mo / 15.0
            + self.v / 10.0
    }

    /// Graville (carbon equivalent) class — see [`GravilleClass`].
    pub fn graville_class(&self) -> GravilleClass {
        if self.c < 0.09 {
            GravilleClass::ClassI
        } else if self.carbon_equivalent_iiw() < 0.40 {
            GravilleClass::ClassII
        } else {
            GravilleClass::ClassIII
        }
    }
}

/// Acceptable t8/5 cooling window: below `min_t8_5_s` the HAZ cools too
/// fast and risks hardenability (cold) cracking; above `max_t8_5_s` the
/// grain coarsens and HAZ toughness suffers. Both limits come from the
/// steel supplier's weldability datasheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoolingWindow {
    /// Minimum acceptable 800→500 °C cooling time, s.
    pub min_t8_5_s: f64,
    /// Maximum acceptable 800→500 °C cooling time, s.
    pub max_t8_5_s: f64,
}

/// The Rosenthal t8/5 closed form inverted for preheat:
/// `t8/5 = Q/(2·pi·k·v) · (1/(500−T0) − 1/(800−T0))` is monotone in T0,
/// so the preheat that drives a given net heat input to `target_t8_5_s`
/// has a unique solution between the ambient temperature and 350 °C
/// (bisection).
///
/// - `q_net_w`: net plate power `η·H·v` (see
///   [`crate::WeldingSimulation::net_power_w`]);
/// - `k_w_mk`: conductivity, W/(m·K);
/// - `v_ms`: travel speed, m/s;
/// - `ambient_c`: the unwelded plate temperature (the search floor — the
///   answer never comes back below `max(ambient, 20) °C`).
///
/// Returns `None` if the target is unreachable even at the 350 °C ceiling.
pub fn preheat_for_target_t8_5(
    q_net_w: f64,
    k_w_mk: f64,
    v_ms: f64,
    ambient_c: f64,
    target_t8_5_s: f64,
) -> Option<f64> {
    let c = q_net_w / (2.0 * std::f64::consts::PI * k_w_mk * v_ms);
    let t8_5 = |t0: f64| c * (1.0 / (500.0 - t0) - 1.0 / (800.0 - t0));
    // Monotone increasing in T0; already slow enough -> hold the floor.
    if t8_5(ambient_c) >= target_t8_5_s {
        return Some(ambient_c.max(20.0));
    }
    if t8_5(350.0) < target_t8_5_s {
        return None;
    }
    let (mut lo, mut hi) = (ambient_c, 350.0_f64);
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if t8_5(mid) < target_t8_5_s {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

/// Outcome of the procedure advisory ([`advise`]).
#[derive(Debug, Clone, PartialEq)]
pub struct AdvisorReport {
    /// IIW carbon equivalent of the base metal.
    pub ce_iiw: f64,
    /// EN 1011-2 CET of the base metal.
    pub cet: f64,
    /// Ito–Bessyo Pcm of the base metal.
    pub pcm: f64,
    /// Graville class.
    pub graville: GravilleClass,
    /// Screening minimum preheat (from [`crate::advise_preheat`]).
    pub min_preheat_c: f64,
    /// t8/5 of the proposed procedure at mid-plate, if computable.
    pub computed_t8_5_s: Option<f64>,
    /// Preheat that would centre the t8/5 in the requested window, if the
    /// window is not met as-proposed.
    pub recommended_preheat_c: Option<f64>,
    /// Findings; empty means the procedure screens clean.
    pub findings: Vec<String>,
}

/// Screens a WPS against the steel's weldability and an optional supplier
/// t8/5 window.
///
/// `combined_thickness_mm` is the EN 1011-2 3-D joint thickness sum (a
/// butt joint in plate of thickness t is commonly screened at 3·t); a
/// low-hydrogen process moves the preheat table band like
/// [`crate::advise_preheat`] does. The t8/5 is evaluated at mid-plate
/// (half the joint thickness), where a single-pass model is most
/// representative.
pub fn advise(
    procedure: &WeldProcedure,
    material: &Material,
    thickness_mm: f64,
    chemistry: &SteelChemistry,
    combined_thickness_mm: f64,
    low_hydrogen: bool,
    window: Option<CoolingWindow>,
) -> AdvisorReport {
    let mut findings = Vec::new();
    let graville = chemistry.graville_class();
    let preheat_advice = crate::advise_preheat(chemistry, combined_thickness_mm, low_hydrogen);
    let min_preheat_c = preheat_advice.recommended_preheat_c;
    if procedure.preheat_temp_c + 1e-9 < min_preheat_c {
        findings.push(format!(
            "preheat {:.0} °C below the screening minimum {:.0} °C ({})",
            procedure.preheat_temp_c, min_preheat_c, preheat_advice.rationale
        ));
    }

    // t8/5 of the proposal at mid-plate.
    let sim = crate::WeldingSimulation::new(
        procedure.clone(),
        material.clone(),
        tpt_yard_joints::JointGeometry::new(tpt_yard_joints::JointKind::Butt)
            .with_thickness_mm(thickness_mm),
    );
    let computed = sim
        .thermal_cycle(thickness_mm / 2.0)
        .ok()
        .and_then(|c| c.t8_5_s);

    let mut recommended_preheat = None;
    if let Some(w) = window {
        match computed {
            Some(t85) if t85 < w.min_t8_5_s => {
                findings.push(format!(
                    "t8/5 {t85:.1} s below the {:.1} s hardenability floor: raise preheat or heat input",
                    w.min_t8_5_s
                ));
                // Window centre as the target cooling time.
                let target = 0.5 * (w.min_t8_5_s + w.max_t8_5_s);
                if let Ok(q) = sim.net_power_w() {
                    recommended_preheat = preheat_for_target_t8_5(
                        q,
                        material.conductivity_w_mk,
                        procedure.travel_speed_mm_s / 1000.0,
                        procedure.preheat_temp_c,
                        target,
                    );
                }
                match recommended_preheat {
                    Some(t0) => findings.push(format!(
                        "preheat ≈ {t0:.0} °C reaches the {target:.1} s window centre"
                    )),
                    None => findings.push(
                        "the window centre is unreachable even at 350 °C: increase heat input"
                            .into(),
                    ),
                }
            }
            Some(t85) if t85 > w.max_t8_5_s => {
                findings.push(format!(
                    "t8/5 {t85:.1} s above the {:.1} s toughness ceiling: reduce heat input or preheat",
                    w.max_t8_5_s
                ));
            }
            Some(t85) => findings.push(format!(
                "t8/5 {t85:.1} s inside the {:.1}-{:.1} s window",
                w.min_t8_5_s, w.max_t8_5_s
            )),
            None => findings.push("t8/5 not computable for this procedure".into()),
        }
    }

    AdvisorReport {
        ce_iiw: chemistry.carbon_equivalent_iiw(),
        cet: chemistry.cet(),
        pcm: chemistry.pcm(),
        graville,
        min_preheat_c,
        computed_t8_5_s: computed,
        recommended_preheat_c: recommended_preheat,
        findings,
    }
}

/// Welding-qualification standards the [`Pqr`] mapping covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualificationStandard {
    /// ISO 15614-1 (steel, arc welding; thickness rule 0.5·t–2·t).
    Iso15614Part1,
    /// ASME BPVC Section IX (the same screening envelope; P-number and
    /// deposit-thickness rules are finer and mapped outside this screen).
    AsmeIx,
}

/// Qualified essential-variable envelope of one procedure qualification
/// record.
#[derive(Debug, Clone, PartialEq)]
pub struct WeldEnvelope {
    /// Qualified process.
    pub process: WeldProcess,
    /// Qualified arc-energy range, kJ/mm (min, max).
    pub heat_input_kj_mm: (f64, f64),
    /// Qualified preheat range, °C (min, max).
    pub preheat_c: (f64, f64),
    /// Qualified maximum interpass temperature, °C.
    pub interpass_max_c: f64,
    /// Qualified test-coupon thickness, mm — the qualified range is
    /// 0.5·t to 2·t per ISO 15614-1 Table 5 (the screening rule applied
    /// to both standards here; ASME IX deposit rules are finer).
    pub coupon_thickness_mm: f64,
    /// Qualified filler metal designation.
    pub filler_metal: String,
}

/// A procedure qualification record: the envelope a WPS must sit inside.
#[derive(Debug, Clone, PartialEq)]
pub struct Pqr {
    /// Record identifier (e.g. "PQR-2026-017").
    pub id: String,
    /// Qualification standard.
    pub standard: QualificationStandard,
    /// The qualified essential variables.
    pub envelope: WeldEnvelope,
}

impl Pqr {
    /// Checks a WPS (and the joint thickness it is written for) against
    /// the qualified envelope. `Ok(())` means every essential variable is
    /// covered; `Err(violations)` lists each one outside the envelope.
    pub fn qualifies(&self, wps: &WeldProcedure, thickness_mm: f64) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();
        let e = &self.envelope;
        if wps.process != e.process {
            violations.push(format!(
                "process {:?} not qualified (PQR covers {:?})",
                wps.process, e.process
            ));
        }
        if !(e.heat_input_kj_mm.0 <= wps.heat_input_kj_mm
            && wps.heat_input_kj_mm <= e.heat_input_kj_mm.1)
        {
            violations.push(format!(
                "heat input {:.2} kJ/mm outside the qualified {:.2}-{:.2} kJ/mm",
                wps.heat_input_kj_mm, e.heat_input_kj_mm.0, e.heat_input_kj_mm.1
            ));
        }
        if wps.preheat_temp_c < e.preheat_c.0 {
            violations.push(format!(
                "preheat {:.0} °C below the qualified minimum {:.0} °C (a lower preheat is an essential-variable change)",
                wps.preheat_temp_c, e.preheat_c.0
            ));
        }
        if wps.interpass_temp_c > e.interpass_max_c {
            violations.push(format!(
                "interpass {:.0} °C above the qualified maximum {:.0} °C",
                wps.interpass_temp_c, e.interpass_max_c
            ));
        }
        let (t_min, t_max) = (0.5 * e.coupon_thickness_mm, 2.0 * e.coupon_thickness_mm);
        if !(t_min <= thickness_mm && thickness_mm <= t_max) {
            violations.push(format!(
                "thickness {thickness_mm:.1} mm outside the qualified {t_min:.1}-{t_max:.1} mm \
                 (0.5-2.0 x coupon {:.1} mm)",
                e.coupon_thickness_mm
            ));
        }
        if wps.filler_metal != e.filler_metal {
            violations.push(format!(
                "filler '{}' not the qualified '{}' (F-number mapping is a manual step)",
                wps.filler_metal, e.filler_metal
            ));
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Residual-free binary Fe-C-Mn chemistry for the textbook closed forms.
    fn comp(c: f64, mn: f64) -> SteelChemistry {
        SteelChemistry {
            c,
            si: 0.4,
            mn,
            cr: 0.0,
            mo: 0.0,
            ni: 0.0,
            cu: 0.0,
            v: 0.0,
        }
    }

    /// IIW closed form on a known chemistry: S355-ish C 0.14 / Mn 1.45
    /// gives CE = 0.14 + 1.45/6 = 0.3817, CET = 0.14 + 1.45/10 = 0.285.
    #[test]
    fn carbon_equivalents_match_closed_forms() {
        let s = comp(0.14, 1.45);
        assert!((s.carbon_equivalent_iiw() - 0.38167).abs() < 1e-4);
        assert!((s.cet() - 0.285).abs() < 1e-9);
        // Pcm hand-check: C + Si/30 + Mn/20 (residual-free).
        let s2 = comp(0.08, 1.50);
        assert!((s2.pcm() - (0.08 + 0.4 / 30.0 + 1.50 / 20.0)).abs() < 1e-12);
        // The crate's AH36-like ladle: CE = 0.16 + 1.4/6
        //   + (0.02+0+0.005)/5 + (0.02+0.02)/15 = 0.401.
        assert!((SteelChemistry::ah36_like().carbon_equivalent_iiw() - 0.401).abs() < 1e-3);
    }

    #[test]
    fn graville_classes_split_on_carbon_and_ce() {
        assert_eq!(comp(0.05, 1.20).graville_class(), GravilleClass::ClassI);
        assert_eq!(comp(0.14, 1.45).graville_class(), GravilleClass::ClassII);
        // CE = 0.16 + 1.6/6 = 0.4267 -> III.
        assert_eq!(comp(0.16, 1.60).graville_class(), GravilleClass::ClassIII);
    }

    /// The inversion recovers the preheat: build the target from the
    /// closed form at a known T0, invert, and get that T0 back.
    #[test]
    fn preheat_inversion_recovers_the_target() {
        let q = 0.95 * 12.0 * 1000.0 * 8.0; // SAW 12 kJ/mm at 8 mm/s -> 91.2 kW
        let k = 50.0;
        let v = 0.008;
        let c = q / (2.0 * std::f64::consts::PI * k * v);
        let t0_known = 150.0;
        let target = c * (1.0 / (500.0 - t0_known) - 1.0 / (800.0 - t0_known));
        let t0 = preheat_for_target_t8_5(q, k, v, 20.0, target).expect("reachable");
        assert!((t0 - t0_known).abs() < 1e-6, "inverted {t0} vs {t0_known}");

        // An unreachable target (cooling always too fast) reports None...
        assert_eq!(preheat_for_target_t8_5(q, k, v, 20.0, 1e6), None);
        // ...and an already-slow-enough plate stays at the ambient floor.
        let slow = c * (1.0 / (500.0 - 20.0) - 1.0 / (800.0 - 20.0));
        assert_eq!(preheat_for_target_t8_5(q, k, v, 20.0, slow), Some(20.0));
    }

    fn procedure(preheat: f64, heat: f64) -> WeldProcedure {
        WeldProcedure {
            process: WeldProcess::Saw,
            heat_input_kj_mm: heat,
            travel_speed_mm_s: 8.0,
            preheat_temp_c: preheat,
            interpass_temp_c: 150.0,
            filler_metal: "S2Si2 / SA AB1 47".into(),
            sequence: vec![],
        }
    }

    /// End-to-end: a cold, low-heat SAW on 40 mm CE-0.427 plate screens
    /// dirty (preheat below the SEW 088 floor, t8/5 below the
    /// hardenability window) and the advisor proposes a preheat that
    /// lands the cooling time inside the window.
    #[test]
    fn advisor_flags_and_recommends() {
        // CE = 0.16 + 1.6/6 = 0.4267; low-hydrogen SAW, t = 40 mm:
        // table row 1, column 40 mm -> 100 °C screening floor.
        let steel = comp(0.16, 1.60);
        let wps = procedure(20.0, 6.0);
        let window = CoolingWindow {
            min_t8_5_s: 25.0,
            max_t8_5_s: 60.0,
        };
        let report = advise(
            &wps,
            &Material::ah36(),
            40.0,
            &steel,
            40.0,
            true,
            Some(window),
        );
        assert_eq!(report.graville, GravilleClass::ClassIII);
        assert_eq!(report.min_preheat_c, 100.0);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.contains("below the screening minimum")),
            "cold preheat must be flagged: {:?}",
            report.findings
        );
        assert!(
            report.computed_t8_5_s.unwrap() < 25.0,
            "6 kJ/mm at 20 °C cools below the hardenability floor: {:?}",
            report.findings
        );
        assert!(report.recommended_preheat_c.is_some());
        let fixed_preheat = report.recommended_preheat_c.unwrap();

        // The recommendation actually lands the t8/5 inside the window...
        let wps2 = procedure(fixed_preheat, 6.0);
        let report2 = advise(
            &wps2,
            &Material::ah36(),
            40.0,
            &steel,
            40.0,
            true,
            Some(window),
        );
        let t85 = report2.computed_t8_5_s.unwrap();
        assert!(
            (25.0..=60.0).contains(&t85),
            "recommended preheat {fixed_preheat:.0} °C lands t8/5 at {t85:.1} s"
        );
        // ...and respects the screening floor (no preheat finding left).
        assert!(wps2.preheat_temp_c >= report2.min_preheat_c);
        assert!(!report2
            .findings
            .iter()
            .any(|f| f.contains("below the screening minimum")));
    }

    fn pqr() -> Pqr {
        Pqr {
            id: "PQR-2026-017".into(),
            standard: QualificationStandard::Iso15614Part1,
            envelope: WeldEnvelope {
                process: WeldProcess::Saw,
                heat_input_kj_mm: (10.0, 14.0),
                preheat_c: (50.0, 200.0),
                interpass_max_c: 180.0,
                coupon_thickness_mm: 20.0,
                filler_metal: "S2Si2 / SA AB1 47".into(),
            },
        }
    }

    #[test]
    fn pqr_envelope_accepts_inside_and_rejects_outside() {
        let p = pqr();
        // Inside everywhere: a 20 mm coupon qualifies 10-40 mm, here 16 mm.
        assert!(p.qualifies(&procedure(80.0, 12.0), 16.0).is_ok());

        // Every essential variable violated at once.
        let mut bad = procedure(20.0, 25.0);
        bad.process = WeldProcess::Gmaw;
        bad.interpass_temp_c = 220.0;
        bad.filler_metal = "E71T-1".into();
        let err = p.qualifies(&bad, 60.0).unwrap_err();
        assert_eq!(
            err.len(),
            6,
            "all five variables + thickness flagged: {err:?}"
        );
    }

    /// ISO 15614-1 Table 5 thickness rule: the 20 mm coupon qualifies
    /// 10-40 mm; 9.9 mm and 40.1 mm are outside.
    #[test]
    fn pqr_thickness_rule_is_half_to_double_the_coupon() {
        let p = pqr();
        assert!(p.qualifies(&procedure(80.0, 12.0), 10.0).is_ok());
        assert!(p.qualifies(&procedure(80.0, 12.0), 40.0).is_ok());
        assert!(p.qualifies(&procedure(80.0, 12.0), 9.9).is_err());
        assert!(p.qualifies(&procedure(80.0, 12.0), 40.1).is_err());
    }
}
