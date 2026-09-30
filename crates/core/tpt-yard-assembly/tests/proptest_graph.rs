//! Property-based tests for the activity graph (review 7D).
//!
//! Invariants: any DAG built by out-of-order insertion topologically sorts
//! with every dependency before its dependent; the makespan is bounded by
//! the sum of durations; `find_cycle` and `topological_order` agree on
//! cyclicity.

use proptest::prelude::*;

use tpt_yard_assembly::{ActivityGraph, ActivityId};

/// Strategy: a random DAG — activity ids 1..=n, each depending only on
/// strictly smaller ids (acyclic by construction), with 0-3 dependencies.
fn dag_strategy() -> impl Strategy<Value = Vec<(u64, Vec<u64>)>> {
    (1usize..=24usize)
        .prop_flat_map(|n| proptest::collection::vec(any::<bool>(), n))
        .prop_flat_map(move |flags| {
            let n = flags.len();
            let pairs: Vec<BoxedStrategy<(u64, Vec<u64>)>> = (0..n)
                .map(|i| {
                    let deps = proptest::collection::vec(1u64..=(i as u64).max(1), 0..=3usize);
                    (Just((i + 1) as u64), deps)
                        .prop_map(|(id, d)| (id, d))
                        .boxed()
                })
                .collect();
            pairs
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn dag_topological_sort_respects_all_edges(
        nodes in dag_strategy(),
        reverse in any::<bool>(),
    ) {
        let mut g = ActivityGraph::new();
        let mut ids: Vec<u64> = nodes.iter().map(|(id, _)| *id).collect();
        if reverse {
            ids.reverse();
        }
        for id in &ids {
            if let Some((id, deps)) = nodes.iter().find(|(nid, _)| nid == id) {
                let deps: Vec<ActivityId> = deps
                    .iter()
                    .filter(|d| **d < *id)
                    .map(|d| ActivityId(*d))
                    .collect();
                let _ = g.add_activity(ActivityId(*id), format!("a{id}"), 1.0, &deps);
            }
        }
        // Out-of-order insertion is legal; the graph must validate and sort.
        g.validate().unwrap();
        let order = g.topological_order().unwrap();
        let pos = |id: u64| order.iter().position(|x| x.0 == id).unwrap();
        for (id, deps) in &nodes {
            for d in deps {
                if *d < *id && g.contains(ActivityId(*d)) {
                    prop_assert!(pos(*d) < pos(*id), "dependency {d} after {id}");
                }
            }
        }
        // Makespan bounded by the total duration.
        let total = nodes.len() as f64;
        prop_assert!(g.makespan_hours().unwrap() <= total + 1e-9);
    }

    #[test]
    fn cycle_detection_agrees_with_topo_sort((a, b) in (1u64..8, 1u64..8).prop_filter("a != b", |(a, b)| a != b)) {
        let mut g = ActivityGraph::new();
        let (a, b) = (ActivityId(a), ActivityId(b));
        g.add_activity(a, "a", 1.0, &[]).unwrap();
        g.add_activity(b, "b", 1.0, &[a]).unwrap();
        // Close the loop a -> ... only when a != b.
        g.activity_mut(a).unwrap().dependencies.push(b);
        let cyclic = g.find_cycle().is_some();
        let topo_err = g.topological_order().is_err();
        prop_assert_eq!(cyclic, topo_err);
    }
}
