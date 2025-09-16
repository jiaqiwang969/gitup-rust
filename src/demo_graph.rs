use graph::{GitWalker, RowBuilder, TuiRenderer, CharsetProfile};

fn main() {
    println!("GitUp-Rust Graph Demo");
    println!("=====================\n");

    // Load repository
    let walker = match GitWalker::new(None) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error loading repository: {}", e);
            return;
        }
    };

    // Build DAG
    let dag = match walker.into_dag(Some(20)) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error building DAG: {}", e);
            return;
        }
    };

    println!("Repository statistics:");
    let stats = dag.stats();
    println!("  Total commits: {}", stats.total_commits);
    println!("  Merge commits: {}", stats.merge_commits);
    println!("  Root commits: {}", stats.root_commits);
    println!();

    // Build layout
    let mut builder = RowBuilder::new(10);
    let rows = builder.build_rows(&dag);

    // Render
    let renderer = TuiRenderer::new(10, CharsetProfile::Utf8Straight);
    let output = renderer.render_rows(&rows, Some(15));

    println!("Commit Graph:");
    println!("─────────────");
    print!("{}", output);
}
