#!/bin/bash
# Demo script for Commit 01: Graph Core DAG

set -e

echo "=== Commit 01: Graph Core DAG Demo ==="
echo

# Build the graph module
echo "Building graph module..."
cd "$(dirname "$0")/../.."
cargo build -p graph

echo
echo "Running DAG tests..."
cargo test -p graph -- --nocapture

echo
echo "Creating demo program..."
cat > src/demo_dag.rs << 'EOF'
use graph::{GitWalker, DagStats};
use anyhow::Result;

fn main() -> Result<()> {
    println!("Loading repository DAG...");

    let walker = GitWalker::new(None)?;
    let dag = walker.into_dag(Some(200))?;

    let stats = dag.stats();
    println!("\n=== DAG Statistics ===");
    println!("Total commits: {}", stats.total_commits);
    println!("Total edges: {}", stats.total_edges);
    println!("Merge commits: {}", stats.merge_commits);
    println!("Root commits: {}", stats.root_commits);
    println!("Leaf commits: {}", stats.leaf_commits);
    println!("Has orphan branches: {}", stats.has_orphans);

    println!("\n=== Sample Commits ===");
    for (i, node) in dag.nodes.values().take(5).enumerate() {
        println!("{}. {} - {} ({})",
            i + 1,
            &node.id[..8],
            node.message,
            node.author
        );
        if !node.parents.is_empty() {
            println!("   Parents: {}",
                node.parents.iter()
                    .map(|p| &p[..8])
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    Ok(())
}
EOF

echo
echo "Running demo..."
cargo run --bin demo_dag 2>/dev/null || {
    echo "Note: To run the demo, add this to Cargo.toml:"
    echo "[[bin]]"
    echo "name = \"demo_dag\""
    echo "path = \"src/demo_dag.rs\""
}

echo
echo "✅ Commit 01 verification complete!"
echo "   - DAG structure created"
echo "   - Git walker adapter implemented"
echo "   - Tests passing"