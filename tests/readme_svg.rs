use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rcm_sparse_bandwidth::graph::Graph;
use rcm_sparse_bandwidth::ordering::rcm;
use rcm_sparse_bandwidth::permutation::{Permutation, shuffled_indices};
use rcm_sparse_bandwidth::svg::render;

#[test]
fn readme_svg_matches_the_rendered_grid() {
    let graph = Graph::from_json(include_str!("../examples/scrambled-grid.json")).unwrap();
    let size = graph.vertices().len();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let shuffle = Permutation::new(shuffled_indices(size, &mut rng), size).unwrap();
    let permutation = Permutation::new(rcm(&graph).new_to_old, size).unwrap();

    let svg = render(
        &graph,
        &graph.permuted(&shuffle).unwrap(),
        &graph.permuted(&permutation).unwrap(),
    );
    let documented = include_str!("../docs/images/scrambled-grid.svg").replace("\r\n", "\n");
    assert!(
        svg == documented,
        "regenerate the grid SVG using the README command"
    );
}
