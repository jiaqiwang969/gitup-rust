use std::io;
use std::time::Duration;
use std::sync::mpsc;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Frame, Terminal,
};

use crate::{
    graph::{GraphView, GitGraph},
    graph_builder::GraphBuilder,
    event::{EventBus, GraphEvent},
    watcher::GitWatcher,
    operations::OperationsManager,
};

/// Main TUI application
pub struct App {
    /// Graph view with Vim navigation
    graph_view: GraphView,

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
}

impl App {
    /// Create a new application
    pub fn new(repo_path: &str) -> Result<Self> {
        // Build initial graph
        let graph = GraphBuilder::new(repo_path)?
            .max_count(500)
            .build()?;

        // Create graph view
        let graph_view = GraphView::new(graph);

        // Create event bus
        let event_bus = EventBus::new();

        // Create operations manager
        let operations = OperationsManager::new(repo_path)?;

        Ok(Self {
            graph_view,
            event_bus,
            watcher: None,
            operations,
            repo_path: repo_path.to_string(),
            should_quit: false,
        })
    }

    /// Initialize file watcher
    pub fn init_watcher(&mut self) -> Result<()> {
        let sender = self.event_bus.get_sender();
        let mut watcher = GitWatcher::new(std::path::Path::new(&self.repo_path), sender)?;
        watcher.watch()?;
        self.watcher = Some(watcher);
        Ok(())
    }

    /// Run the application
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

            // Check if should quit
            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Draw the UI
    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([
                Constraint::Min(10),     // Main graph area
                Constraint::Length(1),   // Status line
            ])
            .split(frame.size());

        // Draw graph
        let graph_block = Block::default()
            .borders(Borders::NONE);

        let graph_area = graph_block.inner(chunks[0]);
        frame.render_widget(graph_block, chunks[0]);

        // Render graph view
        self.graph_view.render(graph_area, frame.buffer_mut());

        // Draw status line
        let status = format!(
            " {} | {} commits | {} ",
            self.repo_path,
            self.graph_view.node_count(),
            self.graph_view.mode_line()
        );

        frame.render_widget(
            ratatui::widgets::Paragraph::new(status)
                .style(ratatui::style::Style::default()
                    .bg(ratatui::style::Color::DarkGray)
                    .fg(ratatui::style::Color::White)),
            chunks[1],
        );
    }

    /// Handle keyboard and other events
    fn handle_events(&mut self) -> Result<bool> {
        // Poll for events with timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Check for quit in normal mode
                if key.code == KeyCode::Char('q') &&
                   self.graph_view.current_mode() == crate::vim::VimMode::Normal {
                    return Ok(true);
                }

                // Handle vim input
                self.graph_view.handle_input(key)?;

                // Check for :q command
                if self.graph_view.current_mode() == crate::vim::VimMode::Normal {
                    // TODO: Check if last command was :q
                }
            }
        }

        Ok(false)
    }

    /// Reload the graph
    pub fn reload_graph(&mut self) -> Result<()> {
        let graph = GraphBuilder::new(&self.repo_path)?
            .max_count(500)
            .build()?;

        self.graph_view = GraphView::new(graph);
        Ok(())
    }
}

/// Setup terminal
pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

/// Restore terminal
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Run the TUI application
pub fn run_tui(repo_path: &str) -> Result<()> {
    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create and run app
    let app = App::new(repo_path)?;
    let res = app.run(&mut terminal);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    res
}