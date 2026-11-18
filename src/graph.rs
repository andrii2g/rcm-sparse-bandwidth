use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::permutation::Permutation;

pub const MIN_VERTICES: usize = 5;
pub const MAX_VERTICES: usize = 200;

/// A simple undirected graph whose vertex list defines its matrix row and column order.
#[derive(Debug)]
pub struct Graph {
    vertices: Vec<u32>,
    edges: Vec<[usize; 2]>,
    adjacency: Vec<Vec<usize>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphInput {
    vertices: Vec<u32>,
    edges: Vec<[u32; 2]>,
}

impl Graph {
    pub fn from_json(input: &str) -> Result<Self, String> {
        let GraphInput { vertices, edges } =
            serde_json::from_str(input).map_err(|error| error.to_string())?;
        Self::new(vertices, edges)
    }

    pub fn new(vertices: Vec<u32>, edges: Vec<[u32; 2]>) -> Result<Self, String> {
        if !(MIN_VERTICES..=MAX_VERTICES).contains(&vertices.len()) {
            return Err(format!(
                "expected {MIN_VERTICES}–{MAX_VERTICES} vertices, got {}",
                vertices.len()
            ));
        }

        let mut indices = HashMap::with_capacity(vertices.len());
        for (index, &id) in vertices.iter().enumerate() {
            if indices.insert(id, index).is_some() {
                return Err(format!("duplicate vertex ID {id}"));
            }
        }

        let mut unique = HashSet::with_capacity(edges.len());
        let mut indexed_edges = Vec::with_capacity(edges.len());
        for [from, to] in edges {
            let &row = indices
                .get(&from)
                .ok_or_else(|| format!("unknown edge endpoint {from}"))?;
            let &column = indices
                .get(&to)
                .ok_or_else(|| format!("unknown edge endpoint {to}"))?;
            if row == column {
                return Err(format!("self-loop at vertex {from}"));
            }
            let pair = [row.min(column), row.max(column)];
            if !unique.insert(pair) {
                return Err(format!("duplicate undirected edge [{from}, {to}]"));
            }
            indexed_edges.push(pair);
        }

        Ok(Self::from_indexed_edges(vertices, indexed_edges))
    }

    pub fn vertices(&self) -> &[u32] {
        &self.vertices
    }

    /// Each edge is a pair of matrix indices, with the smaller index first.
    pub fn edges(&self) -> &[[usize; 2]] {
        &self.edges
    }

    pub fn neighbors(&self, vertex: usize) -> &[usize] {
        &self.adjacency[vertex]
    }

    /// Permute rows and columns together, preserving both vertex IDs and edges.
    pub fn permuted(&self, permutation: &Permutation) -> Result<Self, String> {
        let new_to_old = permutation.new_to_old();
        if new_to_old.len() != self.vertices.len() {
            return Err(format!(
                "permutation length {} does not match graph size {}",
                new_to_old.len(),
                self.vertices.len()
            ));
        }

        let vertices = new_to_old.iter().map(|&old| self.vertices[old]).collect();
        let old_to_new = permutation.old_to_new();
        let edges = self
            .edges
            .iter()
            .map(|&[row, column]| {
                let a = old_to_new[row];
                let b = old_to_new[column];
                [a.min(b), a.max(b)]
            })
            .collect();
        Ok(Self::from_indexed_edges(vertices, edges))
    }

    fn from_indexed_edges(vertices: Vec<u32>, edges: Vec<[usize; 2]>) -> Self {
        let mut adjacency = vec![Vec::new(); vertices.len()];
        for &[row, column] in &edges {
            adjacency[row].push(column);
            adjacency[column].push(row);
        }
        Self {
            vertices,
            edges,
            adjacency,
        }
    }
}
