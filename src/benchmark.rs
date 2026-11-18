use std::cmp::Ordering;
use std::hint::black_box;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::graph::{Graph, MAX_VERTICES, MIN_VERTICES};
use crate::metrics::Metrics;
use crate::ordering::rcm;
use crate::permutation::{Permutation, shuffled_indices};
use crate::sampling::sparse_graph;

pub fn run(
    vertex_counts: &[usize],
    trials: usize,
    seed: u64,
    output: &mut impl Write,
) -> Result<(), String> {
    if vertex_counts.is_empty()
        || vertex_counts
            .iter()
            .any(|n| !(MIN_VERTICES..=MAX_VERTICES).contains(n))
    {
        return Err(format!(
            "benchmark requires vertex counts in {MIN_VERTICES}..={MAX_VERTICES}"
        ));
    }
    if trials == 0 {
        return Err("benchmark requires at least one trial".into());
    }

    writeln!(
        output,
        "Random sparse graphs with one graph per size, seed {seed}, {trials} shuffles per graph"
    )
    .map_err(write_error)?;
    writeln!(
        output,
        "Permutation timing includes validating the order, building its inverse, and rebuilding the sparse graph."
    )
    .map_err(write_error)?;

    for &n in vertex_counts {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        // Each size uses its own stream so the graph is independent of the other sizes and trials.
        rng.set_stream(n as u64);
        let graph = sparse_graph(n, 2 * n, &mut rng)?;
        let original = Metrics::measure(&graph);
        let ordered = rcm(&graph);
        let stats = ordered.stats;
        let reordered = Metrics::measure(&permute(&graph, ordered.new_to_old)?);

        let mut bandwidths = Vec::with_capacity(trials);
        let mut profiles = Vec::with_capacity(trials);
        let mut shuffle_times = Vec::with_capacity(trials);
        let mut shuffle_permute_times = Vec::with_capacity(trials);
        let mut rcm_times = Vec::with_capacity(trials);
        let mut rcm_permute_times = Vec::with_capacity(trials);

        for _ in 0..trials {
            let (order, elapsed) = timed(|| shuffled_indices(n, &mut rng));
            shuffle_times.push(elapsed);
            let (shuffled, elapsed) = timed(|| permute(black_box(&graph), order));
            shuffle_permute_times.push(elapsed);
            let metrics = Metrics::measure(&shuffled?);
            bandwidths.push(metrics.bandwidth);
            profiles.push(metrics.profile);

            let (order, elapsed) = timed(|| rcm(black_box(&graph)).new_to_old);
            rcm_times.push(elapsed);
            let (permuted, elapsed) = timed(|| permute(black_box(&graph), order));
            rcm_permute_times.push(elapsed);
            drop(permuted?);
        }

        let bandwidth_summary = Summary::from_samples(&mut bandwidths, reordered.bandwidth);
        let profile_summary = Summary::from_samples(&mut profiles, reordered.profile);
        let [shuffle_us, shuffle_permute_us, rcm_us, rcm_permute_us] = [
            median_microseconds(&mut shuffle_times),
            median_microseconds(&mut shuffle_permute_times),
            median_microseconds(&mut rcm_times),
            median_microseconds(&mut rcm_permute_times),
        ];

        writeln!(
            output,
            "\n{n} vertices, {} edges\
             \n  Original  bandwidth {}, profile {}\
             \n  Shuffled  bandwidth {}, profile {} (min/median/max)\
             \n  RCM       bandwidth {}, profile {}\
             \n  RCM vs original   bandwidth {}, profile {}\
             \n  RCM vs shuffles   bandwidth {}, profile {} (lower/equal/higher)\
             \n  Median shuffle time (µs)  ordering {shuffle_us:.3}, permutation {shuffle_permute_us:.3}\
             \n  Median RCM time (µs)      ordering {rcm_us:.3}, permutation {rcm_permute_us:.3}\
             \n  RCM BFS operations  {} enqueues, {} dequeues, {} adjacency scans, {} peak queue",
            graph.edges().len(),
            original.bandwidth,
            original.profile,
            bandwidth_summary.range(),
            profile_summary.range(),
            reordered.bandwidth,
            reordered.profile,
            relative(reordered.bandwidth, original.bandwidth),
            relative(reordered.profile, original.profile),
            bandwidth_summary.comparison_counts(),
            profile_summary.comparison_counts(),
            stats.enqueued,
            stats.dequeued,
            stats.adjacency_scans,
            stats.peak_queue,
        )
        .map_err(write_error)?;
    }
    Ok(())
}

fn permute(graph: &Graph, order: Vec<usize>) -> Result<Graph, String> {
    graph.permuted(&Permutation::new(order, graph.vertices().len())?)
}

fn timed<T>(work: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let result = black_box(work());
    (result, start.elapsed())
}

fn median_microseconds(times: &mut [Duration]) -> f64 {
    times.sort_unstable();
    let low = times[(times.len() - 1) / 2].as_secs_f64();
    let high = times[times.len() / 2].as_secs_f64();
    (low + high) * 500_000.0
}

struct Summary {
    min: usize,
    median: f64,
    max: usize,
    lower: usize,
    equal: usize,
    higher: usize,
}

impl Summary {
    fn from_samples(samples: &mut [usize], rcm: usize) -> Self {
        samples.sort_unstable();
        let lower_samples = samples.partition_point(|&value| value < rcm);
        let at_most_rcm = samples.partition_point(|&value| value <= rcm);
        let low = samples[(samples.len() - 1) / 2];
        let high = samples[samples.len() / 2];
        Self {
            min: samples[0],
            median: (low as f64 + high as f64) / 2.0,
            max: samples[samples.len() - 1],
            lower: samples.len() - at_most_rcm,
            equal: at_most_rcm - lower_samples,
            higher: lower_samples,
        }
    }

    fn range(&self) -> String {
        format!("{}/{:.1}/{}", self.min, self.median, self.max)
    }

    fn comparison_counts(&self) -> String {
        format!("{}/{}/{}", self.lower, self.equal, self.higher)
    }
}

fn relative(rcm: usize, original: usize) -> &'static str {
    match rcm.cmp(&original) {
        Ordering::Less => "lower",
        Ordering::Equal => "equal",
        Ordering::Greater => "higher",
    }
}

fn write_error(error: io::Error) -> String {
    format!("could not write benchmark results: {error}")
}
