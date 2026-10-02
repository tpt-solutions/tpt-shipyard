//! Property-based tests for the damage-probability and scantling
//! modules (the 7D verification culture extended to the 2026-10-02
//! slices).
//!
//! Invariants: the SOLAS zone factor `p` is monotone in zone width and
//! bounded by 1; the slab formula round-trips the clamped-plate stress
//! relation for ANY physical input; the Euler buckling thickness is the
//! exact inverse of the Euler stress; the EC3 reduction is monotone
//! non-increasing in slenderness and inside (0, 1]; the stiffener
//! actions are the fixed-fixed closed forms.

use proptest::prelude::*;

use tpt_yard_hydrostatics::{
    p_factor, plate_buckling_check_ec3, plate_buckling_reduction_ec3, plate_buckling_thickness_mm,
    plate_euler_stress_mpa, slab_bending_thickness_mm, stiffener_scantling, DamageLengthDensity,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// `p` grows with zone width, never exceeds 1, and terminal zones
    /// carry at least the mid-ship probability of the same width.
    #[test]
    fn p_factor_monotone_and_bounded(
        ls in 80.0f64..400.0,
        start in 0.02f64..0.3,
        w1 in 0.02f64..0.25,
        dw in 0.01f64..0.3,
    ) {
        let d = DamageLengthDensity::for_subdivision_length(ls).unwrap();
        let x1 = start * ls;
        let p1 = p_factor(&d, ls, x1, x1 + w1 * ls).unwrap();
        let p2 = p_factor(&d, ls, x1, x1 + (w1 + dw).min(1.0 - start) * ls).unwrap();
        prop_assert!(p1 <= p2 + 1e-12, "p shrank with width: {p1} > {p2}");
        prop_assert!((0.0..=1.0).contains(&p1), "p out of range: {p1}");
        // Terminal truncation never loses probability.
        let terminal = p_factor(&d, ls, 0.0, w1 * ls).unwrap();
        prop_assert!(terminal >= p1 - 1e-12, "terminal {terminal} < mid {p1}");
    }

    /// The slab thickness reproduces the clamped-plate stress
    /// sigma = 0.5 p s^2 / t^2 = sigma_allowable / k for any input.
    #[test]
    fn slab_round_trip_for_any_input(
        s in 0.3f64..2.5,
        p in 5.0f64..400.0,
        sigma in 50.0f64..400.0,
        k in 0.7f64..1.0,
        alpha in 1.0f64..1.3,
    ) {
        let t = slab_bending_thickness_mm(s, p, sigma, k, alpha);
        prop_assert!(t > 0.0);
        let t0_m = t / 1000.0 / alpha;
        let sigma_check = 0.5 * (p * 1000.0) * s * s / (t0_m * t0_m) / 1e6;
        prop_assert!((sigma_check - sigma / k).abs() < 1e-6 * sigma,
            "round trip {sigma_check} vs {sigma}");
    }

    /// The buckling thickness is the exact inversion: sigma_E(t_req)
    /// equals the applied demand for any panel and demand.
    #[test]
    fn euler_inversion_exact_for_any_panel(
        short in 400.0f64..2500.0,
        ratio in 1.0f64..6.0,
        applied in 10.0f64..300.0,
    ) {
        let long = short * ratio;
        let t_req = plate_buckling_thickness_mm(short, long, applied, 206_000.0, 0.3)
            .unwrap();
        let back = plate_euler_stress_mpa(t_req, short, long, 206_000.0, 0.3).unwrap();
        prop_assert!((back - applied).abs() < 1e-6 * applied, "{back} vs {applied}");
    }

    /// The EC3 reduction is non-increasing in slenderness, inside
    /// (0, 1], and the check's capacity is rho * fy with utilization
    /// consistent with the demand.
    #[test]
    fn ec3_curve_monotone_and_check_consistent(
        psi in -0.9f64..1.0,
        l1 in 0.55f64..1.4,
        dl in 0.05f64..1.2,
        applied in 20.0f64..400.0,
    ) {
        let rho1 = plate_buckling_reduction_ec3(l1, psi).unwrap();
        let rho2 = plate_buckling_reduction_ec3(l1 + dl, psi).unwrap();
        prop_assert!(rho1 >= rho2, "rho rose: {rho1} -> {rho2} at psi {psi}");
        prop_assert!((0.0..=1.0).contains(&rho1), "rho out of range: {rho1}");
        let check = plate_buckling_check_ec3(
            12.0, 800.0, 3200.0, applied, 355.0, 206_000.0, 0.3,
        )
        .unwrap();
        prop_assert!((check.capacity_mpa - check.reduction * 355.0).abs() < 1e-9);
        prop_assert!((check.utilization - applied / check.capacity_mpa).abs() < 1e-9);
        prop_assert_eq!(check.passes, check.utilization <= 1.0);
    }

    /// The stiffener actions are the exact fixed-fixed closed forms
    /// (end p s l^2 / 12; reaction p s l / 2) and the modulus recovers
    /// the demand stress.
    #[test]
    fn stiffener_closed_forms_for_any_input(
        s in 0.4f64..2.0,
        l in 1.5f64..8.0,
        p in 10.0f64..300.0,
        sigma in 80.0f64..300.0,
        k in 0.7f64..1.0,
    ) {
        let r = stiffener_scantling(s, l, p, sigma, k, None).unwrap();
        let load = k * p * s;
        prop_assert!((r.end_moment_knm - load * l * l / 12.0).abs() < 1e-9);
        prop_assert!((r.reaction_kn - load * l / 2.0).abs() < 1e-9);
        let stress = r.end_moment_knm * 1000.0 / r.required_modulus_cm3;
        prop_assert!((stress - sigma).abs() < 1e-9 * sigma);
    }
}
