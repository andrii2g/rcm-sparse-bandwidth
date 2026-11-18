use std::fmt::Write;

use crate::{graph::Graph, metrics::Metrics};

/// Render three orderings of the same graph at a shared scale.
pub fn render(original: &Graph, shuffled: &Graph, reordered: &Graph) -> String {
    let panels = [
        ("Original", original, Metrics::measure(original)),
        ("Shuffled", shuffled, Metrics::measure(shuffled)),
        ("RCM", reordered, Metrics::measure(reordered)),
    ];
    let vertex_count = original.vertices().len();
    let edge_count = original.edges().len();
    let edge_label = if edge_count == 1 { "edge" } else { "edges" };
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1080 478" role="img" aria-labelledby="title description">
  <title id="title">Adjacency matrix reordering</title>
  <desc id="description">Comparison of original, shuffled, and Reverse Cuthill–McKee vertex orders for a graph with {vertex_count} vertices and {edge_count} {edge_label}. Filled cells represent edges. The dashed line marks the main diagonal."##
    );
    for (name, _, metrics) in &panels {
        write!(
            svg,
            " {name} has bandwidth {} and lower profile {}.",
            metrics.bandwidth, metrics.profile
        )
        .unwrap();
    }
    writeln!(
        svg,
        r##"</desc>
  <rect width="1080" height="478" rx="12" fill="#f0f4f8"/>
  <g font-family="system-ui, sans-serif" fill="#172c39">
    <text x="28" y="40" font-size="25" font-weight="650">Adjacency matrix reordering</text>
    <text x="28" y="65" font-size="14" fill="#425b6b">{vertex_count} vertices · {edge_count} undirected {edge_label}</text>
    <rect x="847" y="54" width="11" height="11" fill="#216582"/>
    <text x="865" y="65" font-size="13" fill="#425b6b">Edge</text>
    <path d="M915 61h21" fill="none" stroke="#658496" stroke-dasharray="3 4"/>
    <text x="942" y="65" font-size="13" fill="#425b6b">Main diagonal</text>"##
    )
    .unwrap();

    for ((name, graph, metrics), x) in panels.into_iter().zip([28, 374, 720]) {
        render_panel(&mut svg, x, name, graph, &metrics);
    }
    svg.push_str("  </g>\n</svg>\n");
    svg
}

fn render_panel(svg: &mut String, x: usize, name: &str, graph: &Graph, metrics: &Metrics) {
    let n = graph.vertices().len();
    let Metrics { bandwidth, profile } = metrics;
    writeln!(
        svg,
        r##"    <g transform="translate({x} 88)">
      <rect width="332" height="362" rx="8" fill="#fff" stroke="#cad8e2"/>
      <text x="20" y="31" font-size="16" font-weight="650">{name}</text>
      <text x="312" y="31" text-anchor="end" font-size="13" fill="#425b6b">Bandwidth {bandwidth} · Profile {profile}</text>
      <rect x="20" y="50" width="292" height="292" fill="#f8fafc"/>
      <svg x="20" y="50" width="292" height="292" viewBox="0 0 {n} {n}">"##
    )
    .unwrap();

    if n <= 25 {
        svg.push_str("        <path d=\"");
        for coordinate in 1..n {
            write!(svg, "M{coordinate} 0V{n}M0 {coordinate}H{n}").unwrap();
        }
        svg.push_str(
            "\" fill=\"none\" stroke=\"#dce7ee\" stroke-width=\"0.75\" vector-effect=\"non-scaling-stroke\"/>\n",
        );
    }
    writeln!(
        svg,
        r##"        <path d="M0 0L{n} {n}" fill="none" stroke="#658496" stroke-dasharray="3 4" vector-effect="non-scaling-stroke"/>"##
    )
    .unwrap();

    if !graph.edges().is_empty() {
        svg.push_str("        <path d=\"");
        for &[u, v] in graph.edges() {
            // Matrix rows determine y and columns determine x.
            write!(svg, "M{v} {u}h1v1h-1zM{u} {v}h1v1h-1z").unwrap();
        }
        svg.push_str("\" fill=\"#216582\" shape-rendering=\"crispEdges\"/>\n");
    }

    svg.push_str(
        r##"      </svg>
      <rect x="20" y="50" width="292" height="292" fill="none" stroke="#cad8e2"/>
    </g>
"##,
    );
}
