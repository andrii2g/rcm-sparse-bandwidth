use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rcm_sparse_bandwidth::benchmark;
use rcm_sparse_bandwidth::graph::Graph;
use rcm_sparse_bandwidth::metrics::Metrics;
use rcm_sparse_bandwidth::ordering::{BfsStats, TraceEvent, rcm, trace_rcm};
use rcm_sparse_bandwidth::permutation::{Permutation, shuffled_indices};
use rcm_sparse_bandwidth::svg;
use rcm_sparse_bandwidth::verification::verify;

const DEFAULT_SEED: u64 = 42;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare the input, shuffled, and RCM orderings.
    Analyze {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
    },
    /// Show component roots, queue operations, and the resulting orders.
    Trace {
        #[arg(long)]
        input: PathBuf,
    },
    /// Check graph, permutation, and matrix-vector invariants on generated fixtures.
    Verify {
        #[arg(long, default_value_t = 1000)]
        graphs: usize,
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
    },
    /// Benchmark RCM and seeded shuffles on generated sparse graphs.
    Benchmark {
        #[arg(long, value_delimiter = ',', default_value = "25,50,100,200")]
        vertices: Vec<usize>,
        #[arg(long, default_value_t = 100)]
        trials: usize,
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
    },
    /// Render original, shuffled, and RCM adjacency matrices as one SVG.
    Render {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::Analyze { input, seed } => analyze(&read_graph(&input)?, seed),
        Command::Trace { input } => {
            trace(&read_graph(&input)?);
            Ok(())
        }
        Command::Verify { graphs, seed } => {
            verify(graphs, seed)?;
            println!("Verified {graphs} graphs with seed {seed}.");
            Ok(())
        }
        Command::Benchmark {
            vertices,
            trials,
            seed,
        } => benchmark::run(&vertices, trials, seed, &mut io::stdout().lock()),
        Command::Render {
            input,
            output,
            seed,
        } => render(&read_graph(&input)?, &output, seed),
    }
}

fn read_graph(path: &Path) -> Result<Graph, String> {
    let json = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Graph::from_json(&json).map_err(|error| format!("invalid graph in {}: {error}", path.display()))
}

fn analyze(graph: &Graph, seed: u64) -> Result<(), String> {
    let shuffled = shuffle(graph, seed)?;
    let started = Instant::now();
    let order = rcm(graph);
    let ordering_time = started.elapsed();
    let started = Instant::now();
    let reordered = apply_order(graph, order.new_to_old)?;
    let permutation_time = started.elapsed();

    println!(
        "{} vertices, {} edges, shuffle seed {seed}",
        graph.vertices().len(),
        graph.edges().len()
    );
    println!("{:<12} {:>10} {:>10}", "Ordering", "Bandwidth", "Profile");
    for (label, matrix) in [
        ("Original", graph),
        ("Shuffled", &shuffled),
        ("RCM", &reordered),
    ] {
        let Metrics { bandwidth, profile } = Metrics::measure(matrix);
        println!("{label:<12} {bandwidth:>10} {profile:>10}");
    }
    println!("RCM vertex order: {:?}", reordered.vertices());
    print_bfs_stats(&order.stats);
    println!(
        "RCM ordering time: {:.3} us",
        ordering_time.as_secs_f64() * 1e6
    );
    println!(
        "RCM sparse permutation time: {:.3} us",
        permutation_time.as_secs_f64() * 1e6
    );
    Ok(())
}

fn trace(graph: &Graph) {
    let ids = graph.vertices();
    let order = trace_rcm(graph, |event| match event {
        TraceEvent::Component { root } => println!("Component root {}", ids[root]),
        TraceEvent::Visit { vertex, enqueued } => {
            let enqueued: Vec<_> = enqueued.iter().map(|&index| ids[index]).collect();
            println!("  pop {:>3}  enqueue {enqueued:?}", ids[vertex]);
        }
    });
    let rcm: Vec<_> = order.new_to_old.iter().map(|&index| ids[index]).collect();
    let cm: Vec<_> = rcm.iter().rev().copied().collect();
    println!("CM vertex order: {cm:?}");
    println!("RCM vertex order: {rcm:?}");
    print_bfs_stats(&order.stats);
}

fn print_bfs_stats(stats: &BfsStats) {
    println!(
        "BFS: {} enqueues, {} dequeues, {} adjacency entries scanned, peak queue {}",
        stats.enqueued, stats.dequeued, stats.adjacency_scans, stats.peak_queue
    );
}

fn shuffle(graph: &Graph, seed: u64) -> Result<Graph, String> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    apply_order(graph, shuffled_indices(graph.vertices().len(), &mut rng))
}

fn apply_order(graph: &Graph, new_to_old: Vec<usize>) -> Result<Graph, String> {
    let permutation = Permutation::new(new_to_old, graph.vertices().len())?;
    graph.permuted(&permutation)
}

fn render(graph: &Graph, path: &Path, seed: u64) -> Result<(), String> {
    let shuffled = shuffle(graph, seed)?;
    let reordered = apply_order(graph, rcm(graph).new_to_old)?;
    let image = svg::render(graph, &shuffled, &reordered);
    if let Some(directory) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(directory)
            .map_err(|error| format!("cannot create directory {}: {error}", directory.display()))?;
    }
    fs::write(path, image).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    println!("Wrote {} (shuffle seed {seed})", path.display());
    Ok(())
}
