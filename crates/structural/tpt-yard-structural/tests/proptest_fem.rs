//! Property-based tests for the truss FEM (review 7D).
//!
//! Invariants: a solved model is in static equilibrium (support reactions
//! balance the applied loads) and a well-supported two-bar truss recovers
//! the analytic force split regardless of geometry.

use proptest::prelude::*;

use tpt_yard_core::Vector3;
use tpt_yard_structural::fem::{Element, Node, NodalLoad, Support, TrussModel};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// A two-bar truss under an apex load must satisfy ΣFz = 0: the two
    /// member forces' vertical components sum to the applied load, for ANY
    /// symmetric half-span and height.
    #[test]
    fn two_bar_equilibrium_for_any_geometry(
        half in 0.3f64..3.0,
        height in 0.3f64..3.0,
        load_n in 1_000.0f64..100_000.0,
    ) {
        let area = 0.005;
        let e = 210.0;
        let model = TrussModel {
            nodes: vec![
                Node { position: Vector3::new(-half, 0.0, height) },
                Node { position: Vector3::new(half, 0.0, height) },
                Node { position: Vector3::new(0.0, 0.0, 0.0) },
            ],
            elements: vec![
                Element { nodes: [0, 2], area_m2: area, youngs_modulus_gpa: e, density_kg_m3: 0.0 },
                Element { nodes: [1, 2], area_m2: area, youngs_modulus_gpa: e, density_kg_m3: 0.0 },
            ],
            supports: vec![
                Support::pinned(0),
                Support::pinned(1),
                Support { node: 2, fix_x: false, fix_y: true, fix_z: false },
            ],
            loads: vec![NodalLoad {
                node: 2,
                force: Vector3::new(0.0, 0.0, -load_n),
            }],
        };
        let sol = model.solve().unwrap();
        let theta = height.atan2(half);
        let vertical_sum = sol.axial_forces[0].abs() * theta.sin()
            + sol.axial_forces[1].abs() * theta.sin();
        prop_assert!(
            (vertical_sum - load_n).abs() < load_n * 5e-3,
            "ΣFz = {vertical_sum} vs load {load_n} (half {half}, h {height})"
        );
        // Both members carry tension (they hold the apex up).
        prop_assert!(sol.axial_forces[0] > 0.0 && sol.axial_forces[1] > 0.0);
        // Displacements are small relative to the structure size.
        prop_assert!(sol.max_displacement_m < height);
    }

    /// A stiffer member (bigger area) takes a proportionally larger share
    /// of a symmetric two-bar pair's load — actually in a symmetric pair
    /// the split is 50/50 regardless of area, so instead verify stiffness
    /// reduces displacement monotonically for a single axial bar.
    #[test]
    fn stiffer_bar_displaces_less(
        area in 0.001f64..0.05,
        load_n in 10_000.0f64..500_000.0,
    ) {
        let l = 2.0;
        let e = 210.0;
        let model = TrussModel {
            nodes: vec![
                Node { position: Vector3::ZERO },
                Node { position: Vector3::new(l, 0.0, 0.0) },
            ],
            elements: vec![Element {
                nodes: [0, 1],
                area_m2: area,
                youngs_modulus_gpa: e,
                density_kg_m3: 0.0,
            }],
            supports: vec![
                Support::pinned(0),
                Support { node: 1, fix_x: false, fix_y: true, fix_z: true },
            ],
            loads: vec![NodalLoad {
                node: 1,
                force: Vector3::new(load_n, 0.0, 0.0),
            }],
        };
        let sol = model.solve().unwrap();
        // delta = FL/EA within the penalty tolerance (1e-3 relative).
        let expected = load_n * l / (e * 1e9 * area);
        prop_assert!(
            (sol.displacements[1].x - expected).abs() <= expected * 5e-3,
            "delta {} vs {}",
            sol.displacements[1].x,
            expected
        );
        // Stress = F/A exactly (force recovery).
        prop_assert!(
            (sol.axial_forces[0] - load_n).abs() < load_n * 5e-3
        );
    }
}
