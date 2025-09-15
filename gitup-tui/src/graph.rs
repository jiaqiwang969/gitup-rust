use std::collections::HashMap;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};
use crate::vim::{Position, VimHandler, VimMode, VimAction};

/// Git graph visualization with Vim navigation
pub struct GraphView {
    /// The git graph data
    graph: GitGraph,

    /// Vim handler for keyboard input
    vim_handler: VimHandler,

    /// Current viewport
    viewport: Viewport,

    /// Rendering configuration
    config: GraphConfig,

    /// Selection state
    selection: SelectionState,

    /// Search state
    search: Option<SearchState>,
}

/// Git graph data structure
pub struct GitGraph {
    /// All nodes in the graph
    pub nodes: Vec<GraphNode>,

    /// Edges connecting nodes
    pub edges: Vec<GraphEdge>,

    /// Lane assignments for rendering
    pub lanes: Vec<Lane>,

    /// Branch references
    pub branches: HashMap<String, String>, // name -> commit_sha

    /// Tag references
    pub tags: HashMap<String, String>, // name -> commit_sha
}

/// A single node in the graph
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// Commit SHA
    pub id: String,

    /// Short commit message
    pub message: String,

    /// Author name
    pub author: String,

    /// Commit date
    pub date: String,

    /// Position in the graph
    pub position: GraphPosition,

    /// Node type
    pub node_type: NodeType,

    /// References at this node
    pub refs: Vec<RefInfo>,
}

/// Position in the graph
#[derive(Debug, Clone, Copy)]
pub struct GraphPosition {
    /// Row index (0 = newest)
    pub row: usize,

    /// Lane index (0 = leftmost)
    pub lane: usize,
}

/// Type of graph node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Regular,
    Merge,
    Branch,
    Initial,
    Current, // HEAD
    Stash,
    WorkingDirectory,
}

/// Reference information
#[derive(Debug, Clone)]
pub struct RefInfo {
    pub ref_type: RefType,
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefType {
    Branch,
    Tag,
    Remote,
    Head,
}

/// Edge connecting two nodes
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub lane: usize,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Direct,
    Merge,
    Fork,
}

/// Lane for rendering parallel branches
#[derive(Debug, Clone)]
pub struct Lane {
    pub index: usize,
    pub color: Color,
    pub active: bool,
}

/// Viewport for scrolling
#[derive(Debug, Clone)]
pub struct Viewport {
    pub top: usize,
    pub height: usize,
    pub left: usize,
    pub width: usize,
}

/// Graph rendering configuration
#[derive(Debug, Clone)]
pub struct GraphConfig {
    pub show_author: bool,
    pub show_date: bool,
    pub show_hash: bool,
    pub hash_length: usize,
    pub colors: GraphColors,
    pub symbols: GraphSymbols,
}

/// Color scheme for the graph
#[derive(Debug, Clone)]
pub struct GraphColors {
    pub lanes: Vec<Color>,
    pub head: Color,
    pub branch: Color,
    pub remote_branch: Color,
    pub tag: Color,
    pub stash: Color,
}

impl Default for GraphColors {
    fn default() -> Self {
        Self {
            lanes: vec![
                Color::Cyan,
                Color::Green,
                Color::Yellow,
                Color::Magenta,
                Color::Blue,
                Color::Red,
            ],
            head: Color::LightCyan,
            branch: Color::Green,
            remote_branch: Color::Blue,
            tag: Color::Yellow,
            stash: Color::Gray,
        }
    }
}

/// ASCII symbols for graph rendering
#[derive(Debug, Clone)]
pub struct GraphSymbols {
    pub commit: char,
    pub commit_head: char,
    pub commit_merge: char,
    pub commit_initial: char,
    pub commit_stash: char,

    pub line_vertical: char,
    pub line_horizontal: char,
    pub line_diagonal_up: char,
    pub line_diagonal_down: char,

    pub branch_start: char,
    pub branch_merge: char,
    pub branch_cross: char,
}

impl Default for GraphSymbols {
    fn default() -> Self {
        Self {
            commit: '●',
            commit_head: '◉',
            commit_merge: '◈',
            commit_initial: '◎',
            commit_stash: '◊',

            line_vertical: '│',
            line_horizontal: '─',
            line_diagonal_up: '╱',
            line_diagonal_down: '╲',

            branch_start: '├',
            branch_merge: '┤',
            branch_cross: '┼',
        }
    }
}

/// Selection state
#[derive(Debug, Clone)]
pub struct SelectionState {
    pub cursor: Position,
    pub visual_anchor: Option<Position>,
    pub selected_commits: Vec<String>,
}

/// Search state
#[derive(Debug, Clone)]
pub struct SearchState {
    pub pattern: String,
    pub matches: Vec<Position>,
    pub current_match: usize,
}

impl GraphView {
    pub fn new(graph: GitGraph) -> Self {
        let config = GraphConfig {
            show_author: true,
            show_date: true,
            show_hash: true,
            hash_length: 7,
            colors: GraphColors::default(),
            symbols: GraphSymbols::default(),
        };

        Self {
            graph,
            vim_handler: VimHandler::new(),
            viewport: Viewport {
                top: 0,
                height: 0,
                left: 0,
                width: 0,
            },
            config,
            selection: SelectionState {
                cursor: Position::new(0, 0),
                visual_anchor: None,
                selected_commits: Vec::new(),
            },
            search: None,
        }
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.graph.nodes.len()
    }

    /// Get the current vim mode line
    pub fn mode_line(&self) -> String {
        self.vim_handler.mode_line()
    }

    /// Get the current vim mode
    pub fn current_mode(&self) -> crate::vim::VimMode {
        self.vim_handler.current_mode()
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        let action = self.vim_handler.handle_key(key)?;

        match action {
            VimAction::Move(motion) => {
                // Apply motion to cursor
                let context = GraphMotionContext::new(&self.graph);
                let new_pos = motion.apply(self.selection.cursor, &context);
                self.selection.cursor = new_pos;

                // Update viewport if needed
                self.ensure_cursor_visible();
            }

            VimAction::ModeChange(mode) => {
                // Handle mode changes
                match mode {
                    VimMode::Visual | VimMode::VisualLine => {
                        self.selection.visual_anchor = Some(self.selection.cursor);
                    }
                    VimMode::Normal => {
                        self.selection.visual_anchor = None;
                    }
                    _ => {}
                }
            }

            VimAction::GitOp(op) => {
                // Handle git operations
                self.handle_git_operation(op)?;
            }

            VimAction::Search(direction, pattern) => {
                // Perform search
                self.search_commits(&pattern, direction)?;
            }

            _ => {}
        }

        Ok(())
    }

    /// Ensure cursor is visible in viewport
    fn ensure_cursor_visible(&mut self) {
        let row = self.selection.cursor.row;

        if row < self.viewport.top {
            self.viewport.top = row;
        } else if row >= self.viewport.top + self.viewport.height {
            self.viewport.top = row - self.viewport.height + 1;
        }
    }

    /// Handle git operations
    fn handle_git_operation(&mut self, op: crate::vim::GitOperation) -> anyhow::Result<()> {
        // TODO: Implement git operations
        Ok(())
    }

    /// Search for commits
    fn search_commits(&mut self, pattern: &str, direction: crate::vim::SearchDirection) -> anyhow::Result<()> {
        // TODO: Implement search
        Ok(())
    }

    /// Render the graph
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Calculate visible range
        let visible_start = self.viewport.top;
        let visible_end = (self.viewport.top + area.height as usize).min(self.graph.nodes.len());

        // Render each visible node
        for (i, node) in self.graph.nodes[visible_start..visible_end].iter().enumerate() {
            let y = area.y + i as u16;
            self.render_node(node, y, area, buf);
        }

        // Render cursor
        if self.selection.cursor.row >= visible_start && self.selection.cursor.row < visible_end {
            let cursor_y = area.y + (self.selection.cursor.row - visible_start) as u16;
            self.highlight_line(cursor_y, area, buf);
        }

        // Render mode line
        self.render_mode_line(area, buf);
    }

    /// Render a single node
    fn render_node(&self, node: &GraphNode, y: u16, area: Rect, buf: &mut Buffer) {
        let mut x = area.x;

        // Render lanes and commit symbol
        let lane_width = 2; // characters per lane
        let graph_width = self.graph.lanes.len() * lane_width;

        // Draw lanes before the commit
        for lane_idx in 0..node.position.lane {
            let lane_x = x + (lane_idx * lane_width) as u16;
            let lane = &self.graph.lanes[lane_idx];

            if lane.active {
                buf.set_string(
                    lane_x,
                    y,
                    &self.config.symbols.line_vertical.to_string(),
                    Style::default().fg(lane.color),
                );
            }
        }

        // Draw commit symbol
        let commit_x = x + (node.position.lane * lane_width) as u16;
        let symbol = match node.node_type {
            NodeType::Current => self.config.symbols.commit_head,
            NodeType::Merge => self.config.symbols.commit_merge,
            NodeType::Initial => self.config.symbols.commit_initial,
            NodeType::Stash => self.config.symbols.commit_stash,
            _ => self.config.symbols.commit,
        };

        let commit_color = self.graph.lanes[node.position.lane].color;
        buf.set_string(
            commit_x,
            y,
            &symbol.to_string(),
            Style::default().fg(commit_color).add_modifier(
                if node.node_type == NodeType::Current {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }
            ),
        );

        x += graph_width as u16 + 1;

        // Render hash
        if self.config.show_hash {
            let hash = &node.id[..self.config.hash_length.min(node.id.len())];
            buf.set_string(
                x,
                y,
                hash,
                Style::default().fg(Color::DarkGray),
            );
            x += self.config.hash_length as u16 + 1;
        }

        // Render refs
        if !node.refs.is_empty() {
            for ref_info in &node.refs {
                let (text, color): (&str, Color) = match ref_info.ref_type {
                    RefType::Head => ("HEAD", self.config.colors.head),
                    RefType::Branch => (ref_info.name.as_str(), self.config.colors.branch),
                    RefType::Tag => (ref_info.name.as_str(), self.config.colors.tag),
                    RefType::Remote => (ref_info.name.as_str(), self.config.colors.remote_branch),
                };

                let ref_text = if ref_info.is_head {
                    format!("HEAD -> {}", text)
                } else {
                    text.to_string()
                };

                buf.set_string(
                    x,
                    y,
                    &format!("[{}]", ref_text),
                    Style::default().fg(color),
                );
                x += ref_text.len() as u16 + 3;
            }
        }

        // Render message
        let remaining_width = area.width.saturating_sub(x - area.x);
        let message = if node.message.len() > remaining_width as usize {
            format!("{}...", &node.message[..remaining_width as usize - 3])
        } else {
            node.message.clone()
        };

        buf.set_string(
            x,
            y,
            &message,
            Style::default(),
        );
    }

    /// Highlight a line
    fn highlight_line(&self, y: u16, area: Rect, buf: &mut Buffer) {
        for x in area.x..area.x + area.width {
            let cell = buf.get_mut(x, y);
            cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
        }
    }

    /// Render the mode line
    fn render_mode_line(&self, area: Rect, buf: &mut Buffer) {
        let mode_line = self.vim_handler.mode_line();
        let y = area.y + area.height - 1;

        buf.set_string(
            area.x,
            y,
            &mode_line,
            Style::default().bg(Color::DarkGray).fg(Color::White),
        );
    }
}

/// Motion context for the graph
struct GraphMotionContext<'a> {
    graph: &'a GitGraph,
}

impl<'a> GraphMotionContext<'a> {
    fn new(graph: &'a GitGraph) -> Self {
        Self { graph }
    }
}

impl<'a> crate::vim::motion::MotionContext for GraphMotionContext<'a> {
    fn line_length(&self, row: usize) -> usize {
        80 // Default line length
    }

    fn first_non_blank(&self, row: usize) -> usize {
        0 // Graph always starts at column 0
    }

    fn total_lines(&self) -> usize {
        self.graph.nodes.len()
    }

    fn next_word_start(&self, from: Position) -> Position {
        // Move to next significant element (hash, ref, message)
        Position::new(from.row, from.col + 10)
    }

    fn prev_word_start(&self, from: Position) -> Position {
        Position::new(from.row, from.col.saturating_sub(10))
    }

    fn next_word_end(&self, from: Position) -> Position {
        Position::new(from.row, from.col + 9)
    }

    fn next_commit(&self, from: Position) -> Option<Position> {
        if from.row + 1 < self.graph.nodes.len() {
            Some(Position::new(from.row + 1, 0))
        } else {
            None
        }
    }

    fn prev_commit(&self, from: Position) -> Option<Position> {
        if from.row > 0 {
            Some(Position::new(from.row - 1, 0))
        } else {
            None
        }
    }

    fn parent_commit(&self, from: Position, n: usize) -> Option<Position> {
        // Find parent commit(s) of current commit
        // TODO: Implement actual parent lookup
        self.prev_commit(from)
    }

    fn child_commit(&self, from: Position, n: usize) -> Option<Position> {
        // Find child commit(s) of current commit
        // TODO: Implement actual child lookup
        self.next_commit(from)
    }
}