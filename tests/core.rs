use rcm_sparse_bandwidth::graph::Graph;
use rcm_sparse_bandwidth::metrics::Metrics;
use rcm_sparse_bandwidth::ordering::{BfsStats, rcm};
use rcm_sparse_bandwidth::permutation::Permutation;
use rcm_sparse_bandwidth::verification::verify;

#[test]
fn restores_scrambled_path() {
    let graph = Graph::new(
        vec![1, 4, 2, 5, 3, 6],
        vec![[1, 2], [2, 3], [3, 4], [4, 5], [5, 6]],
    )
    .unwrap();
    let order = rcm(&graph);
    assert_eq!(
        order.stats,
        BfsStats {
            enqueued: 6,
            dequeued: 6,
            adjacency_scans: 10,
            peak_queue: 1,
        },
    );
    let result = graph
        .permuted(&Permutation::new(order.new_to_old, 6).unwrap())
        .unwrap();
    assert_eq!(result.vertices(), &[6, 5, 4, 3, 2, 1]);
    assert_eq!(
        Metrics::measure(&graph),
        Metrics {
            bandwidth: 3,
            profile: 9
        }
    );
    assert_eq!(
        Metrics::measure(&result),
        Metrics {
            bandwidth: 1,
            profile: 5
        }
    );
}

#[test]
fn groups_disconnected_components_and_retains_isolates() {
    let graph = Graph::new(
        vec![1, 5, 3, 7, 2, 6, 4],
        vec![[1, 2], [2, 3], [3, 4], [5, 6]],
    )
    .unwrap();
    let result = graph
        .permuted(&Permutation::new(rcm(&graph).new_to_old, 7).unwrap())
        .unwrap();
    assert_eq!(result.vertices(), &[6, 5, 4, 3, 2, 1, 7]);
    assert!(result.neighbors(6).is_empty());
    assert_eq!(
        Metrics::measure(&result),
        Metrics {
            bandwidth: 1,
            profile: 4
        }
    );
}

#[test]
fn reversing_cm_preserves_bandwidth_but_can_change_profile() {
    let graph = Graph::new(vec![1, 2, 3, 4, 5], vec![[1, 3], [2, 3], [3, 4], [3, 5]]).unwrap();
    let order = rcm(&graph).new_to_old;
    let cm = Permutation::new(order.iter().rev().copied().collect(), 5).unwrap();
    let rcm = Permutation::new(order, 5).unwrap();
    assert_eq!(
        Metrics::measure(&graph),
        Metrics {
            bandwidth: 2,
            profile: 5
        }
    );
    assert_eq!(
        Metrics::measure(&graph.permuted(&cm).unwrap()),
        Metrics {
            bandwidth: 3,
            profile: 7
        },
    );
    assert_eq!(
        Metrics::measure(&graph.permuted(&rcm).unwrap()),
        Metrics {
            bandwidth: 3,
            profile: 4
        },
    );
}

#[test]
fn validates_graphs_and_permutations_before_indexing() {
    let vertices = vec![1, 2, 3, 4, 5];
    assert!(Graph::new(vec![1, 2, 3, 4], vec![]).is_err());
    assert!(Graph::new(vec![1, 2, 3, 4, 4], vec![]).is_err());
    assert!(Graph::new(vertices.clone(), vec![[1, 6]]).is_err());
    assert!(Graph::new(vertices.clone(), vec![[1, 1]]).is_err());
    assert!(Graph::new(vertices.clone(), vec![[1, 2], [2, 1]]).is_err());
    assert!(Permutation::new(vec![0, 1], 3).is_err());
    assert!(Permutation::new(vec![0, 1, 1], 3).is_err());
    assert!(Permutation::new(vec![0, 1, 3], 3).is_err());

    let edgeless = Graph::new(vertices, vec![]).unwrap();
    let short = Permutation::new(vec![0, 1], 2).unwrap();
    assert!(edgeless.permuted(&short).is_err());
    assert_eq!(
        Metrics::measure(&edgeless),
        Metrics {
            bandwidth: 0,
            profile: 0
        }
    );
}

#[test]
fn verifies_seeded_graphs_against_an_independent_matrix_oracle() {
    verify(21, 42).unwrap();
    assert!(verify(0, 42).is_err());
}
