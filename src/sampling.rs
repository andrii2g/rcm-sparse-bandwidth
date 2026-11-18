use rand::{Rng, seq::index};

use crate::graph::{Graph, MAX_VERTICES, MIN_VERTICES};

pub fn sparse_graph(vertices: usize, edges: usize, rng: &mut impl Rng) -> Result<Graph, String> {
    if !(MIN_VERTICES..=MAX_VERTICES).contains(&vertices) {
        return Err(format!(
            "vertex count must be between {MIN_VERTICES} and {MAX_VERTICES}, got {vertices}"
        ));
    }

    let max_edges = vertices * (vertices - 1) / 2;
    if edges > max_edges {
        return Err(format!(
            "cannot sample {edges} distinct edges from {vertices} vertices (maximum {max_edges})"
        ));
    }

    let labels: Vec<u32> = (1..=vertices as u32).collect();
    let edges = sample_edges(vertices, edges, rng);
    Graph::new(labels, edges)
}

pub(crate) fn sample_edges(vertices: usize, edges: usize, rng: &mut impl Rng) -> Vec<[u32; 2]> {
    let candidates: Vec<_> = (1..=vertices as u32)
        .flat_map(|left| (left + 1..=vertices as u32).map(move |right| [left, right]))
        .collect();

    index::sample(rng, candidates.len(), edges)
        .into_iter()
        .map(|edge| candidates[edge])
        .collect()
}
