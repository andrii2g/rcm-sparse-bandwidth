use crate::graph::Graph;

#[derive(Debug, PartialEq, Eq)]
pub struct Metrics {
    pub bandwidth: usize,
    pub profile: usize,
}

impl Metrics {
    pub fn measure(graph: &Graph) -> Self {
        let mut bandwidth = 0;
        // The implicit diagonal anchors each row, including rows of isolated vertices.
        let mut first_column: Vec<_> = (0..graph.vertices().len()).collect();
        for &[a, b] in graph.edges() {
            bandwidth = bandwidth.max(b - a);
            first_column[b] = first_column[b].min(a);
        }
        let profile = first_column
            .into_iter()
            .enumerate()
            .map(|(row, column)| row - column)
            .sum();
        Self { bandwidth, profile }
    }
}
