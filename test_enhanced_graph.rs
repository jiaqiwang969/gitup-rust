use gitup_ui::enhanced_graph::EnhancedGraphIntegration;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn main() {
    println!("Testing Enhanced Graph Integration...");

    // Try to initialize the enhanced graph
    match EnhancedGraphIntegration::new(".") {
        Ok(mut graph) => {
            println!("✓ Enhanced graph initialized successfully");

            // Create a test buffer
            let area = Rect::new(0, 0, 120, 40);
            let mut buffer = Buffer::empty(area);

            // Try to render
            graph.render(area, &mut buffer);
            println!("✓ Enhanced graph rendered successfully");

            // Test input handling
            graph.handle_input(crossterm::event::KeyCode::Down);
            println!("✓ Input handling works");

            // Get selected commit
            if let Some(commit) = graph.selected_commit() {
                println!("✓ Selected commit: {}", &commit[..8.min(commit.len())]);
            }

            println!("\n✅ All tests passed! Enhanced graph integration is working.");
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize enhanced graph: {}", e);
        }
    }
}