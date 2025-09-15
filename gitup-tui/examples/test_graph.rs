use anyhow::Result;
use gitup_tui::graph_builder::GraphBuilder;

fn main() -> Result<()> {
    // Get repository path from args or use current directory
    let repo_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".".to_string());

    println!("Building graph for repository: {}", repo_path);

    // Build the graph
    let graph = GraphBuilder::new(&repo_path)?
        .max_count(10)
        .build()?;

    // Print graph info
    println!("\nGraph Statistics:");
    println!("  Nodes: {}", graph.nodes.len());
    println!("  Edges: {}", graph.edges.len());
    println!("  Lanes: {}", graph.lanes.len());
    println!("  Branches: {}", graph.branches.len());
    println!("  Tags: {}", graph.tags.len());

    // Print first few nodes
    println!("\nFirst {} commits:", graph.nodes.len().min(5));
    for (i, node) in graph.nodes.iter().take(5).enumerate() {
        println!("  {}. [{}] {} - {} ({})",
            i + 1,
            &node.id[..7],
            node.message,
            node.author,
            node.date
        );

        // Print refs if any
        if !node.refs.is_empty() {
            for ref_info in &node.refs {
                println!("      -> {:?}: {}", ref_info.ref_type, ref_info.name);
            }
        }
    }

    // Print lane assignments
    println!("\nLane assignments:");
    for (i, lane) in graph.lanes.iter().enumerate() {
        println!("  Lane {}: color={:?}, active={}",
            i,
            lane.color,
            lane.active
        );
    }

    Ok(())
}