//! Property-based tests for the SOLAS probabilistic pieces (the 7D
//! verification culture extended to r/v-era modules).
//!
//! Invariants: the penetration factor r is monotone in the penetration
//! depth and bounded by [0, 1] for any zone; the multi-zone group
//! factor never exceeds the union-span p and stays positive; the
//! s-factor is monotone in GZmax and range and gated by heel; and the
//! v factor is monotone in deck height within the printed range.

use proptest::prelude::*;

use tpt_yard_hydrostatics::{
    cross_flooding_time, multi_zone_p_factor, p_factor, r_factor, s_final_factor,
    s_intermediate_factor, v_factor, DamageLengthDensity,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// r grows with penetration depth, stays in [0, 1], and reaches 1
    /// exactly at full penetration — for any zone along any ship.
    #[test]
    fn r_factor_monotone_in_penetration(
        ls in 90.0f64..350.0,
        x1 in 0.05f64..0.6,
        w in 0.05f64..0.3,
        b in 0.2f64..11.0,
    ) {
        let d = DamageLengthDensity::for_subdivision_length(ls).unwrap();
        let breadth = 22.0;
        let x1m = x1 * ls;
        let x2m = x1m + w.min(1.0 - x1) * ls;
        let mut prev = 0.0;
        for k in 0..=10 {
            let depth = b * k as f64 / 10.0;
            let r = r_factor(&d, ls, breadth, x1m, x2m, depth).unwrap();
            prop_assert!((0.0..=1.0).contains(&r), "r {r} out of range");
            prop_assert!(r >= prev - 1e-9, "r fell at depth {depth}");
            prev = r;
        }
        let full = r_factor(&d, ls, breadth, x1m, x2m, breadth / 2.0).unwrap();
        prop_assert!((full - 1.0).abs() < 1e-9, "full penetration r {full}");
    }

    /// A multi-zone group factor is non-negative, never above the
    /// union-span p (pure form), and a bulkhead reduces it below the
    /// pure form at practical depths.
    #[test]
    fn multi_zone_bounded_by_the_union(
        ls in 90.0f64..350.0,
        start in 0.1f64..0.5,
        zw in 0.05f64..0.12,
        b in 0.5f64..8.0,
    ) {
        let d = DamageLengthDensity::for_subdivision_length(ls).unwrap();
        let zones: Vec<(f64, f64)> = (0..3)
            .map(|k| {
                let a = (start + k as f64 * zw) * ls;
                (a, a + zw * ls)
            })
            .collect();
        let union = p_factor(&d, ls, zones[0].0, zones[2].1).unwrap();
        let pure = multi_zone_p_factor(&d, ls, 22.0, &zones, None).unwrap();
        prop_assert!(pure >= -1e-9, "negative pure combination {pure}");
        prop_assert!(pure <= union + 1e-9, "pure {pure} above union {union}");
        let with = multi_zone_p_factor(&d, ls, 22.0, &zones, Some(b)).unwrap();
        prop_assert!(with <= pure + 1e-9, "bulkhead grew the group: {with} vs {pure}");
        // The full-penetration limit: with the barrier at B/2 every r
        // bracket is 1, so the group equals the pure combination again.
        let full = multi_zone_p_factor(&d, ls, 22.0, &zones, Some(11.0)).unwrap();
        prop_assert!((full - pure).abs() < 1e-9, "B/2 barrier must be transparent");
    }

    /// The final-stage s-factor is non-decreasing in GZmax and range
    /// and non-increasing in heel — for both ship types.
    #[test]
    fn s_final_monotone_in_its_arguments(
        heel1 in 0.0f64..30.0,
        d_heel in 0.1f64..5.0,
        gz in 0.01f64..0.6,
        dgz in 0.01f64..0.2,
        range in 2.0f64..15.0,
        drange in 0.5f64..5.0,
    ) {
        for &(passenger, ro_ro) in &[(false, false), (true, false), (true, true)] {
            let s1 = s_final_factor(heel1, gz, range, passenger, ro_ro);
            let s2 = s_final_factor(heel1 + d_heel, gz, range, passenger, ro_ro);
            prop_assert!(s2 <= s1 + 1e-12, "s rose with heel");
            let s3 = s_final_factor(heel1, gz + dgz, range, passenger, ro_ro);
            prop_assert!(s3 >= s1 - 1e-12, "s fell with GZmax");
            let s4 = s_final_factor(heel1, gz, range + drange, passenger, ro_ro);
            prop_assert!(s4 >= s1 - 1e-12, "s fell with range");
            prop_assert!((0.0..=1.0).contains(&s1));
        }
    }

    /// The intermediate factor sits in [0, 1], is capped at the 0.05 m /
    /// 7 deg anchors, and is zero past the heel gate.
    #[test]
    fn s_intermediate_capped_and_gated(
        heel in 0.0f64..45.0,
        gz in 0.001f64..0.3,
        range in 0.5f64..14.0,
    ) {
        for passenger in [false, true] {
            let s = s_intermediate_factor(heel, gz, range, passenger);
            prop_assert!((0.0..=1.0).contains(&s));
            let gate = if passenger { 15.0 } else { 30.0 };
            if heel > gate {
                prop_assert_eq!(s, 0.0);
            }
        }
    }

    /// The cross-flooding time grows with the initial head, shrinks
    /// with duct area, and equals zero for a zero head.
    #[test]
    fn cross_flooding_monotone(
        h0 in 0.5f64..8.0,
        a1 in 0.05f64..0.5,
        da in 0.05f64..0.4,
    ) {
        let t1 = cross_flooding_time(h0, 0.0, a1, 0.6, 50.0, 60.0).unwrap();
        let t2 = cross_flooding_time(h0, 0.0, a1 + da, 0.6, 50.0, 60.0).unwrap();
        prop_assert!(t2 < t1, "bigger duct took longer: {t2} vs {t1}");
        let t0 = cross_flooding_time(0.0, 0.0, a1, 0.6, 50.0, 60.0).unwrap();
        prop_assert_eq!(t0, 0.0);
    }

    /// The horizontal-deck v factor stays inside [0, 1] and is
    /// non-decreasing in deck height.
    #[test]
    fn v_factor_monotone_in_deck_height(
        draft in 2.0f64..12.0,
        h1 in 8.0f64..25.0,
        dh in 0.1f64..5.0,
    ) {
        let v1 = v_factor(h1, draft);
        let v2 = v_factor(h1 + dh, draft);
        prop_assert!((0.0..=1.0).contains(&v1), "v {v1} out of range");
        prop_assert!(v2 >= v1 - 1e-12, "v fell with deck height");
    }
}
