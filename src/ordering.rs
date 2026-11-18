use std::collections::VecDeque;

use crate::graph::Graph;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct BfsStats {
    pub enqueued: usize,
    pub dequeued: usize,
    pub adjacency_scans: usize,
    pub peak_queue: usize,
}

#[derive(Debug)]
pub struct Ordering {
    pub new_to_old: Vec<usize>,
    pub stats: BfsStats,
}

pub enum TraceEvent<'a> {
    Component {
        root: usize,
    },
    Visit {
        vertex: usize,
        enqueued: &'a [usize],
    },
}

pub fn rcm(graph: &Graph) -> Ordering {
    trace_rcm(graph, |_| {})
}

pub fn trace_rcm(graph: &Graph, mut emit: impl FnMut(TraceEvent<'_>)) -> Ordering {
    let n = graph.vertices().len();
    let key = |&vertex: &usize| (graph.neighbors(vertex).len(), graph.vertices()[vertex]);

    let mut roots: Vec<_> = (0..n).collect();
    roots.sort_unstable_by_key(key);
    let ordered_neighbors: Vec<_> = (0..n)
        .map(|vertex| {
            let mut neighbors = graph.neighbors(vertex).to_vec();
            neighbors.sort_unstable_by_key(key);
            neighbors
        })
        .collect();

    let mut seen = vec![false; n];
    let mut queue = VecDeque::new();
    let mut order = Vec::with_capacity(n);
    let mut enqueued = Vec::new();
    let mut stats = BfsStats::default();

    for root in roots {
        if seen[root] {
            continue;
        }
        emit(TraceEvent::Component { root });
        seen[root] = true;
        queue.push_back(root);
        stats.enqueued += 1;
        stats.peak_queue = stats.peak_queue.max(queue.len());

        while let Some(vertex) = queue.pop_front() {
            order.push(vertex);
            stats.dequeued += 1;
            let neighbors = &ordered_neighbors[vertex];
            stats.adjacency_scans += neighbors.len();
            enqueued.clear();
            for &neighbor in neighbors {
                if !seen[neighbor] {
                    seen[neighbor] = true;
                    queue.push_back(neighbor);
                    enqueued.push(neighbor);
                }
            }
            stats.enqueued += enqueued.len();
            stats.peak_queue = stats.peak_queue.max(queue.len());
            emit(TraceEvent::Visit {
                vertex,
                enqueued: &enqueued,
            });
        }
    }

    order.reverse();
    Ordering {
        new_to_old: order,
        stats,
    }
}
