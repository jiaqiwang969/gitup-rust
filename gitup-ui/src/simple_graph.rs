use gitup_core::{CommitInfo, BranchInfo};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};
use std::collections::HashMap;

/// Simple graph visualization for git commits
pub struct SimpleGraph {
    /// Graph symbols
    pub symbols: GraphSymbols,
}

/// Basic graph symbols
pub struct GraphSymbols {
    pub commit: char,
    pub merge: char,
    pub branch: char,
    pub vertical: char,
    pub horizontal: char,
    pub cross: char,
}

impl Default for GraphSymbols {
    fn default() -> Self {
        Self {
            commit: '●',
            merge: '◉',
            branch: '├',
            vertical: '│',
            horizontal: '─',
            cross: '┼',
        }
    }
}

impl SimpleGraph {
    pub fn new() -> Self {
        Self {
            symbols: GraphSymbols::default(),
        }
    }

    /// Render the graph for a list of commits
    pub fn render_commits(
        &self,
        commits: &[CommitInfo],
        branches: &[BranchInfo],
        area: Rect,
        buf: &mut Buffer,
        selected: Option<usize>,
    ) {
        // Find HEAD branch
        let head_commit = branches.iter()
            .find(|b| b.is_head)
            .map(|b| &b.commit_id);

        // Simple rendering: one lane with continuous vertical lines
        for (i, commit) in commits.iter().enumerate() {
            if i >= area.height as usize {
                break;
            }

            let y = area.y + i as u16;
            let mut x = area.x;

            // Draw vertical line (including at the commit position)
            let cell = &mut buf[(x, y)];
            if i == 0 {
                // First commit - just the node
                cell.set_char(self.symbols.commit);
            } else {
                // All other rows - vertical line
                cell.set_char(self.symbols.vertical);
            }
            cell.set_style(Style::default().fg(Color::Blue));

            // Draw commit node (overwrite the vertical line)
            x += 0; // Stay at same position
            let cell = &mut buf[(x, y)];
            let is_head = head_commit == Some(&commit.id);
            let symbol = if is_head {
                '◎' // HEAD commit
            } else {
                self.symbols.commit
            };
            cell.set_char(symbol);
            cell.set_style(Style::default().fg(
                if is_head { Color::Green }
                else if selected == Some(i) { Color::Yellow }
                else { Color::White }
            ));

            // Draw commit info
            x += 2;
            let hash = &commit.id[..8.min(commit.id.len())];
            let message = commit.message.lines().next().unwrap_or("");
            let text = format!("{} {}", hash, message);

            for (j, ch) in text.chars().enumerate() {
                if x + j as u16 >= area.x + area.width {
                    break;
                }
                let cell = &mut buf[(x + j as u16, y)];
                cell.set_char(ch);
                cell.set_style(Style::default().fg(
                    if j < 8 { Color::Cyan } else { Color::White }
                ));
            }

            // If not the last commit, draw vertical line in between
            if i < commits.len() - 1 && i < area.height as usize - 1 {
                // Draw an intermediate vertical line between commits
                let between_y = y + 1;
                if between_y < area.y + area.height {
                    let cell = &mut buf[(area.x, between_y)];
                    // Only draw if we're not about to draw another commit there
                    if i + 1 < commits.len() && i + 1 < area.height as usize {
                        // Next iteration will handle this
                    }
                }
            }
        }
    }
}

/// Widget wrapper for SimpleGraph
pub struct SimpleGraphWidget<'a> {
    graph: &'a SimpleGraph,
    commits: &'a [CommitInfo],
    branches: &'a [BranchInfo],
    selected: Option<usize>,
}

impl<'a> SimpleGraphWidget<'a> {
    pub fn new(
        graph: &'a SimpleGraph,
        commits: &'a [CommitInfo],
        branches: &'a [BranchInfo],
    ) -> Self {
        Self {
            graph,
            commits,
            branches,
            selected: None,
        }
    }

    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }
}

impl<'a> Widget for SimpleGraphWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Draw border
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Git Graph");
        let inner = block.inner(area);
        block.render(area, buf);

        // Render the graph
        self.graph.render_commits(
            self.commits,
            self.branches,
            inner,
            buf,
            self.selected,
        );
    }
}