use anyhow::Result;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use graph::{
    GitWalker, CompactRowBuilder, TuiRenderer, VirtualRenderer,
    TextLayout, CjkMode, CommitMessageFormatter, SeamlessViewport,
    CellRouter, CharsetProfile, ConflictResolver,
};

use crate::graph::{GitGraph, GraphNode};

/// Enhanced graph widget that uses the new graph rendering engine
pub struct EnhancedGraphWidget {
    /// Virtual renderer with viewport management
    renderer: VirtualRenderer,
    /// Seamless viewport for scroll continuity
    viewport: SeamlessViewport,
    /// Text formatter for CJK support
    formatter: CommitMessageFormatter,
    /// Cell router for character selection
    router: CellRouter,
    /// Conflict resolver
    resolver: ConflictResolver,
    /// Title
    title: String,
}

impl EnhancedGraphWidget {
    /// Create a new enhanced graph widget
    pub fn new(repo_path: &str) -> Result<Self> {
        // Load repository
        let walker = GitWalker::new(Some(repo_path))?;
        let dag = walker.into_dag(None)?;

        // Build compact layout
        let mut builder = CompactRowBuilder::new(10);
        let rows = builder.build_rows(&dag);

        // Determine CJK mode
        let cjk_mode = if TextLayout::detect_cjk_from_locale() {
            CjkMode::Auto
        } else {
            CjkMode::Off
        };

        // Create components
        let renderer = VirtualRenderer::new(rows.clone(), 30, 10);
        let viewport = SeamlessViewport::new(30, rows.len());
        let formatter = CommitMessageFormatter::new(cjk_mode);

        // Determine charset profile based on terminal
        let profile = if is_unicode_terminal() {
            CharsetProfile::Utf8Rounded
        } else {
            CharsetProfile::Ascii
        };

        let router = CellRouter::new(profile);
        let resolver = ConflictResolver::new(profile);

        Ok(Self {
            renderer,
            viewport,
            formatter,
            router,
            resolver,
            title: "Commits".to_string(),
        })
    }

    /// Handle input and update viewport
    pub fn handle_input(&mut self, key: char) -> bool {
        // Use virtual renderer's input handling
        self.renderer.handle_input(key)
    }

    /// Scroll up
    pub fn scroll_up(&mut self, n: usize) {
        self.viewport.scroll_up(n, &[], 10); // TODO: pass actual rows
    }

    /// Scroll down
    pub fn scroll_down(&mut self, n: usize) {
        self.viewport.scroll_down(n, &[], 10); // TODO: pass actual rows
    }

    /// Get current cursor position
    pub fn cursor_position(&self) -> usize {
        self.viewport.cursor
    }
}

impl Widget for EnhancedGraphWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Draw border
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        // Render graph content
        let content = self.renderer.render();

        // Split content into lines and render
        for (y, line) in content.lines().enumerate() {
            if y >= inner.height as usize {
                break;
            }

            let mut x = inner.x;
            for ch in line.chars() {
                if x >= inner.x + inner.width {
                    break;
                }

                // Apply styling based on character type
                let style = if ch == '●' || ch == '○' {
                    Style::default().fg(Color::Yellow)
                } else if "│├┤┌┐└┘┬┴┼─".contains(ch) {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default()
                };

                buf.get_mut(x, inner.y + y as u16)
                    .set_char(ch)
                    .set_style(style);

                // Handle CJK width
                let width = if is_cjk_char(ch) { 2 } else { 1 };
                x += width;
            }
        }
    }
}

/// Check if terminal supports Unicode
fn is_unicode_terminal() -> bool {
    std::env::var("TERM")
        .map(|term| !term.contains("ascii") && !term.contains("vt100"))
        .unwrap_or(true)
}

/// Check if character is CJK
fn is_cjk_char(ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{3040}'..='\u{309F}' |
        '\u{30A0}'..='\u{30FF}' |
        '\u{AC00}'..='\u{D7AF}'
    )
}

/// Integration adapter to convert existing GitGraph to new format
pub struct GraphAdapter;

impl GraphAdapter {
    /// Convert existing GitGraph to DAG format
    pub fn convert_to_dag(graph: &GitGraph) -> Result<graph::Dag> {
        let mut dag = graph::Dag::new();

        for node in &graph.nodes {
            // Extract parent IDs from edges
            let parents: Vec<String> = graph.edges
                .iter()
                .filter(|e| e.child_id == node.id)
                .map(|e| e.parent_id.clone())
                .collect();

            let commit = graph::CommitNode::new(
                node.id.clone(),
                parents,
                chrono::Utc::now(), // TODO: parse actual date
                node.author.clone(),
                node.message.clone(),
            );

            dag.add_node(commit);
        }

        Ok(dag)
    }

    /// Create enhanced widget from existing graph
    pub fn create_widget(graph: &GitGraph) -> Result<EnhancedGraphWidget> {
        let dag = Self::convert_to_dag(graph)?;

        // Build compact layout
        let mut builder = CompactRowBuilder::new(10);
        let rows = builder.build_rows(&dag);

        // Create renderer
        let renderer = VirtualRenderer::new(rows.clone(), 30, 10);
        let viewport = SeamlessViewport::new(30, rows.len());

        let cjk_mode = if TextLayout::detect_cjk_from_locale() {
            CjkMode::Auto
        } else {
            CjkMode::Off
        };

        let formatter = CommitMessageFormatter::new(cjk_mode);
        let router = CellRouter::new(CharsetProfile::Utf8Rounded);
        let resolver = ConflictResolver::new(CharsetProfile::Utf8Rounded);

        Ok(EnhancedGraphWidget {
            renderer,
            viewport,
            formatter,
            router,
            resolver,
            title: "Enhanced Commits".to_string(),
        })
    }
}