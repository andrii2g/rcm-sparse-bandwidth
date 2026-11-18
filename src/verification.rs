use std::collections::{BTreeSet, HashMap};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::graph::{Graph, MAX_VERTICES, MIN_VERTICES};
use crate::ordering::rcm;
use crate::permutation::{Permutation, shuffled_indices};
use crate::sampling::sample_edges;

pub fn verify(graph_count: usize, seed: u64) -> Result<(), String> {
    if graph_count == 0 {
        return Err("graph count must be positive".into());
    }

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for graph_index in 0..graph_count {
        let size = match graph_index {
            0 => MIN_VERTICES,
            1 => MAX_VERTICES,
            _ => rng.random_range(MIN_VERTICES..=MAX_VERTICES),
        };
        let edges = fixture_edges(graph_index, size, &mut rng);
        let vertices: Vec<_> = shuffled_indices(size, &mut rng)
            .into_iter()
            .map(|index| index as u32 + 1)
            .collect();

        verify_graph(&vertices, &edges).map_err(|error| {
            let graph_number = graph_index + 1;
            let input = serde_json::json!({ "vertices": vertices, "edges": edges });
            format!("seed {seed}, graph {graph_number}: {error}\n{input}")
        })?;
    }
    Ok(())
}

fn fixture_edges(graph_index: usize, size: usize, rng: &mut impl Rng) -> Vec<[u32; 2]> {
    let last_label = size as u32;
    let max_edges = size * (size - 1) / 2;
    match graph_index % 7 {
        0 => Vec::new(),
        1 => (1..last_label).map(|left| [left, left + 1]).collect(),
        2 => {
            // Two paths of at least two vertices and one isolate.
            let split = last_label / 2;
            (1..split)
                .chain(split + 1..last_label - 1)
                .map(|left| [left, left + 1])
                .collect()
        }
        3 => sample_edges(size, max_edges * 3 / 4, rng),
        4 => (1..last_label)
            .map(|left| [left, left + 1])
            .chain([[1, last_label]])
            .collect(),
        _ => {
            let edge_count = rng.random_range(1..=max_edges.min(3 * size));
            sample_edges(size, edge_count, rng)
        }
    }
}

fn verify_graph(vertices: &[u32], edges: &[[u32; 2]]) -> Result<(), String> {
    let graph = Graph::new(vertices.to_vec(), edges.to_vec())?;
    let ordering = rcm(&graph);
    let size = vertices.len();
    let stats = ordering.stats;
    if stats.enqueued != size
        || stats.dequeued != size
        || stats.adjacency_scans != 2 * edges.len()
        || !(1..=size).contains(&stats.peak_queue)
    {
        return Err(format!(
            "unexpected BFS counts (enqueued {}, dequeued {}, scanned {}, peak queue {})",
            stats.enqueued, stats.dequeued, stats.adjacency_scans, stats.peak_queue
        ));
    }

    let reversed_edges = edges.iter().rev().map(|&[a, b]| [b, a]).collect();
    let equivalent = Graph::new(vertices.to_vec(), reversed_edges)?;
    if rcm(&equivalent).new_to_old != ordering.new_to_old {
        return Err("RCM order changed after reversing edge order and endpoints".into());
    }

    let permutation = Permutation::new(ordering.new_to_old, size)?;
    let new_to_old = permutation.new_to_old();
    let old_to_new = permutation.old_to_new();
    if new_to_old.len() != size
        || old_to_new.len() != size
        || new_to_old
            .iter()
            .enumerate()
            .any(|(new, &old)| old_to_new.get(old) != Some(&new))
    {
        return Err("permutation and inverse are inconsistent".into());
    }

    let reordered = graph.permuted(&permutation)?;
    let expected_labels: Vec<_> = new_to_old.iter().map(|&old| vertices[old]).collect();
    if reordered.vertices() != expected_labels {
        return Err("reordered vertex labels do not match the permutation".into());
    }

    // Build the oracle from the input labels and edges, independent of Graph's storage.
    let original_indices: HashMap<_, _> = vertices
        .iter()
        .enumerate()
        .map(|(position, &label)| (label, position))
        .collect();
    let mut original_entries = vec![false; size * size];
    let mut original_edges = BTreeSet::new();
    let vector: Vec<_> = (0..size as i64).map(|i| 2 * i - size as i64 + 1).collect();
    let mut product = vec![0; size];
    for &[a, b] in edges {
        let (row, column) = (original_indices[&a], original_indices[&b]);
        original_edges.insert(canonical_pair(row, column));
        original_entries[row * size + column] = true;
        original_entries[column * size + row] = true;
        product[row] += vector[column];
        product[column] += vector[row];
    }

    let mut recovered_edges = BTreeSet::new();
    for &[row, column] in reordered.edges() {
        let (Some(&old_row), Some(&old_column)) = (new_to_old.get(row), new_to_old.get(column))
        else {
            return Err("reordered edge has an out-of-range endpoint".into());
        };
        recovered_edges.insert(canonical_pair(old_row, old_column));
    }
    if reordered.edges().len() != edges.len() || recovered_edges != original_edges {
        return Err("reordered edge list did not preserve the original edge set".into());
    }

    let permuted_vector: Vec<_> = new_to_old.iter().map(|&old| vector[old]).collect();
    let mut actual_row = vec![false; size];
    for (new_row, &old_row) in new_to_old.iter().enumerate() {
        actual_row.fill(false);
        let mut row_product = 0;
        for &new_column in reordered.neighbors(new_row) {
            if new_column >= size || actual_row[new_column] {
                return Err(format!("invalid adjacency entry ({new_row}, {new_column})"));
            }
            actual_row[new_column] = true;
            row_product += permuted_vector[new_column];
        }

        for (new_column, &old_column) in new_to_old.iter().enumerate() {
            if actual_row[new_column] != original_entries[old_row * size + old_column] {
                return Err(format!(
                    "symmetric permutation produced the wrong entry at ({new_row}, {new_column})"
                ));
            }
        }
        if row_product != product[old_row] {
            return Err(format!(
                "matrix-vector product mismatch at reordered row {new_row}"
            ));
        }
    }
    Ok(())
}

fn canonical_pair(a: usize, b: usize) -> [usize; 2] {
    [a.min(b), a.max(b)]
}
