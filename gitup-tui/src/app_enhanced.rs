use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use crate::{
    graph_enhanced::{EnhancedGraphWidget, GraphAdapter},
    graph::{GraphView, GitGraph},
    graph_builder::GraphBuilder,
    event::{EventBus, GraphEvent},
    watcher::GitWatcher,
    operations::OperationsManager,
};

/// Enhanced TUI application with new graph rendering
pub struct EnhancedApp {
    /// Enhanced graph widget
    graph_widget: EnhancedGraphWidget,

    /// Graph view for compatibility
    graph_view: Option<GraphView>,

    /// Event bus for handling events
    event_bus: EventBus,

    /// Git watcher for real-time updates
    watcher: Option<GitWatcher>,

    /// Operations manager
    operations: OperationsManager,

    /// Repository path
    repo_path: String,

    /// Should quit
    should_quit: bool,

    /// Use enhanced rendering
    use_enhanced: bool,

    /// Status message
    status_message: String,
}

impl EnhancedApp {
    /// Create a new enhanced application
    pub fn new(repo_path: &str) -> Result<Self> {
        // Create enhanced graph widget
        let graph_widget = EnhancedGraphWidget::new(repo_path)?;

        // Create event bus
        let event_bus = EventBus::new();

        // Create operations manager
        let operations = OperationsManager::new(repo_path)?;

        Ok(Self {
            graph_widget,
            graph_view: None,
            event_bus,
            watcher: None,
            operations,
            repo_path: repo_path.to_string(),
            should_quit: false,
            use_enhanced: true,
            status_message: format!("GitUp Enhanced - {}", repo_path),
        })
    }

    /// Toggle between enhanced and legacy rendering
    pub fn toggle_rendering(&mut self) -> Result<()> {
        self.use_enhanced = !self.use_enhanced;

        if !self.use_enhanced && self.graph_view.is_none() {
            // Create legacy graph view if needed
            let graph = GraphBuilder::new(&self.repo_path)?
                .max_count(500)
                .build()?;
            self.graph_view = Some(GraphView::new(graph));
        }

        self.status_message = format!(
            "Rendering: {}",
            if self.use_enhanced { "Enhanced (新)" } else { "Legacy" }
        );

        Ok(())
    }

    /// Initialize file watcher
    pub fn init_watcher(&mut self) -> Result<()> {
        let sender = self.event_bus.get_sender();
        let mut watcher = GitWatcher::new(std::path::Path::new(&self.repo_path), sender)?;
        watcher.watch()?;
        self.watcher = Some(watcher);
        Ok(())
    }

    /// Run the enhanced application
    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Initialize watcher
        self.init_watcher()?;

        // Main loop
        loop {
            // Draw UI
            terminal.draw(|f| self.draw(f))?;

            // Handle events
            if self.handle_events()? {
                break;
            }

            // Process git events
            self.event_bus.process_events()?;

            // Check for refresh
            if self.event_bus.has_event(&GraphEvent::RefreshRequired) {
                self.reload_graph()?;
            }
        }

        Ok(())
    }

    /// Draw the UI
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),      // Main content
                Constraint::Length(3),   // Status bar
            ])
            .split(f.size());

        // Draw main graph
        if self.use_enhanced {
            self.draw_enhanced_graph(f, chunks[0]);
        } else if let Some(ref mut graph_view) = self.graph_view {
            graph_view.render(chunks[0], f.buffer_mut());
        }

        // Draw status bar
        self.draw_status(f, chunks[1]);
    }

    /// Draw enhanced graph
    fn draw_enhanced_graph<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        // Clone the widget (or create a reference wrapper)
        // Note: In real implementation, we'd need to handle this better
        f.render_widget(&self.graph_widget, area);
    }

    /// Draw status bar
    fn draw_status<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let status = Paragraph::new(self.status_message.clone())
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, area);
    }

    /// Handle events
    fn handle_events(&mut self) -> Result<bool> {
        // Poll for events with timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // Quit
                    KeyCode::Char('q') => {
                        return Ok(true);
                    }
                    // Toggle rendering mode
                    KeyCode::Char('e') => {
                        self.toggle_rendering()?;
                    }
                    // Vim-style navigation
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.graph_widget.scroll_down(1);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.graph_widget.scroll_up(1);
                    }
                    KeyCode::Char('g') => {
                        // Jump to top
                        self.graph_widget.scroll_up(9999);
                    }
                    KeyCode::Char('G') => {
                        // Jump to bottom
                        self.graph_widget.scroll_down(9999);
                    }
                    KeyCode::PageDown => {
                        self.graph_widget.scroll_down(10);
                    }
                    KeyCode::PageUp => {
                        self.graph_widget.scroll_up(10);
                    }
                    // Refresh
                    KeyCode::Char('r') => {
                        self.reload_graph()?;
                        self.status_message = "Graph refreshed".to_string();
                    }
                    // Help
                    KeyCode::Char('?') => {
                        self.status_message = "Keys: q=quit, e=toggle, j/k=nav, g/G=top/bot, r=refresh".to_string();
                    }
                    _ => {}
                }
            }
        }

        Ok(false)
    }

    /// Reload the graph
    pub fn reload_graph(&mut self) -> Result<()> {
        // Recreate enhanced widget
        self.graph_widget = EnhancedGraphWidget::new(&self.repo_path)?;

        // Reload legacy view if needed
        if let Some(ref mut graph_view) = self.graph_view {
            let graph = GraphBuilder::new(&self.repo_path)?
                .max_count(500)
                .build()?;
            *graph_view = GraphView::new(graph);
        }

        Ok(())
    }
}

/// Setup terminal for enhanced mode
pub fn setup_enhanced_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

/// Restore terminal from enhanced mode
pub fn restore_enhanced_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}