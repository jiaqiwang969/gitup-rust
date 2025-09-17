use crate::layout::{Row, Lane};
use crate::render::router::{CharsetProfile, ConflictResolver};

/// Box drawing characters for graph rendering
pub mod chars {
    pub const VERTICAL: char = '│';
    pub const HORIZONTAL: char = '─';
    pub const COMMIT: char = '●';
    pub const COMMIT_EMPTY: char = '○';

    pub const BRANCH_UP_RIGHT: char = '┌';
    pub const BRANCH_DOWN_RIGHT: char = '└';
    pub const BRANCH_UP_LEFT: char = '┐';
    pub const BRANCH_DOWN_LEFT: char = '┘';

    pub const MERGE_LEFT: char = '┤';
    pub const MERGE_RIGHT: char = '├';
    pub const MERGE_UP: char = '┴';
    pub const MERGE_DOWN: char = '┬';

    pub const CROSS: char = '┼';
    pub const PASS_VERTICAL: char = '│';
    pub const SPACE: char = ' ';
}

/// Terminal color codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Default,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Color {
    pub fn to_ansi(&self) -> &str {
        match self {
            Color::Default => "\x1b[0m",
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Magenta => "\x1b[35m",
            Color::Cyan => "\x1b[36m",
            Color::White => "\x1b[37m",
        }
    }
}

/// A cell in the rendered grid
#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub color: Color,
}

impl Cell {
    pub fn new(ch: char, color: Color) -> Self {
        Self { ch, color }
    }

    pub fn empty() -> Self {
        Self {
            ch: chars::SPACE,
            color: Color::Default,
        }
    }
}

/// TUI renderer for commit graph
pub struct TuiRenderer {
    /// Width of the graph area (in characters)
    graph_width: usize,
    /// Colors assigned to lanes
    lane_colors: Vec<Color>,
    /// Charset profile for glyph selection/merging
    profile: CharsetProfile,
    /// Conflict resolver used for Z-merge when multiple strokes touch same cell
    resolver: ConflictResolver,
}

impl TuiRenderer {
    pub fn new(graph_width: usize, profile: CharsetProfile) -> Self {
        // Pre-assign colors to lanes
        let colors = vec![
            Color::Blue,
            Color::Green,
            Color::Red,
            Color::Yellow,
            Color::Magenta,
            Color::Cyan,
        ];

        let mut lane_colors = Vec::new();
        for i in 0..graph_width {
            lane_colors.push(colors[i % colors.len()]);
        }

        Self {
            graph_width,
            lane_colors,
            profile,
            resolver: ConflictResolver::new(profile),
        }
    }

    /// Expose the configured graph width (lanes)
    pub fn graph_width(&self) -> usize {
        self.graph_width
    }

    /// Write a char into the cell grid with conflict-aware merge
    #[inline]
    fn put_char(&self, cells: &mut [Cell], idx: usize, ch: char, color: Color) {
        if idx >= cells.len() { return; }
        if cells[idx].ch == chars::SPACE {
            cells[idx] = Cell::new(ch, color);
        } else {
            // Merge with existing glyph using resolver
            let merged = self.resolver.merge_chars(cells[idx].ch, ch);
            cells[idx] = Cell::new(merged, color);
        }
    }

    /// Render a single row to a grid of cells
    pub fn render_row(&self, row: &Row) -> Vec<Cell> {
        let mut cells = vec![Cell::empty(); self.graph_width * 2]; // 2 chars per lane

        for (lane_idx, lane) in row.lanes.iter().enumerate() {
            if lane_idx >= self.graph_width {
                break;
            }

            let color = self.lane_colors[lane_idx % self.lane_colors.len()];
            let pos = lane_idx * 2; // Each lane takes 2 chars

            match lane {
                Lane::Empty => {
                    self.put_char(&mut cells, pos, chars::SPACE, color);
                    self.put_char(&mut cells, pos + 1, chars::SPACE, color);
                }
                Lane::Pass => {
                    self.put_char(&mut cells, pos, chars::VERTICAL, color);
                    self.put_char(&mut cells, pos + 1, chars::SPACE, color);
                }
                Lane::Commit => {
                    let ch = if lane_idx == row.primary_lane {
                        chars::COMMIT
                    } else {
                        chars::COMMIT_EMPTY
                    };
                    // For commit nodes, overlay on top of existing character (preserve background lines)
                    if cells[pos].ch != chars::SPACE {
                        // Merge commit node with existing background glyph
                        let merged = self.resolver.merge_chars(cells[pos].ch, ch);
                        self.put_char(&mut cells, pos, merged, color);
                    } else {
                        self.put_char(&mut cells, pos, ch, color);
                    }
                    self.put_char(&mut cells, pos + 1, chars::SPACE, color);
                }
                Lane::BranchStart => {
                    self.put_char(&mut cells, pos, chars::BRANCH_UP_RIGHT, color);
                    self.put_char(&mut cells, pos + 1, chars::HORIZONTAL, color);
                }
                Lane::Merge(targets) => {
                    // Enhanced merge rendering with proper tee connections
                    // Draw the merge point with proper character selection
                    let merge_char = if targets.iter().any(|&t| t < lane_idx) && targets.iter().any(|&t| t > lane_idx) {
                        chars::CROSS  // Cross if merging from both sides
                    } else if targets.iter().any(|&t| t < lane_idx) {
                        chars::MERGE_LEFT  // Left tee (┤) for merges from left
                    } else {
                        chars::MERGE_RIGHT // Right tee (├) for merges from right
                    };

                    // For merge nodes, overlay on existing character to preserve background
                    if cells[pos].ch != chars::SPACE {
                        let merged = self.resolver.merge_chars(cells[pos].ch, merge_char);
                        self.put_char(&mut cells, pos, merged, color);
                    } else {
                        self.put_char(&mut cells, pos, merge_char, color);
                    }

                    // Draw horizontal line to the right
                    self.put_char(&mut cells, pos + 1, chars::HORIZONTAL, color);

                    // Draw lines to merge targets (ensuring they connect properly)
                    for &target in targets {
                        if target < self.graph_width && target != lane_idx {
                            let target_pos = target * 2;
                            let tcolor = self.lane_colors[target % self.lane_colors.len()];

                            // Ensure target lanes show proper connection
                            if cells[target_pos].ch == chars::SPACE {
                                self.put_char(&mut cells, target_pos, chars::VERTICAL, tcolor);
                            } else {
                                // Merge with existing glyph
                                let merged = self.resolver.merge_chars(cells[target_pos].ch, chars::VERTICAL);
                                self.put_char(&mut cells, target_pos, merged, tcolor);
                            }

                            // Draw horizontal connection lines between merge point and targets
                            let start_x = std::cmp::min(lane_idx * 2, target_pos);
                            let end_x = std::cmp::max(lane_idx * 2, target_pos);
                            for x in ((start_x + 2)..(end_x)).step_by(2) {
                                if x + 1 < cells.len() {
                                    self.put_char(&mut cells, x + 1, chars::HORIZONTAL, color);
                                }
                            }
                        }
                    }
                }
                Lane::End => {
                    self.put_char(&mut cells, pos, chars::BRANCH_DOWN_LEFT, color);
                    self.put_char(&mut cells, pos + 1, chars::SPACE, color);
                }
            }
        }

        cells
    }

    /// Render multiple rows to a string buffer
    pub fn render_rows(&self, rows: &[Row], limit: Option<usize>) -> String {
        let mut buffer = String::new();

        let rows_to_render = if let Some(limit) = limit {
            &rows[..rows.len().min(limit)]
        } else {
            rows
        };

        for row in rows_to_render {
            let cells = self.render_row(row);

            // Render graph
            for cell in &cells {
                buffer.push_str(cell.color.to_ansi());
                buffer.push(cell.ch);
            }
            buffer.push_str(Color::Default.to_ansi());

            // Add commit info
            buffer.push_str(" ");
            buffer.push_str(&row.commit_id[..8.min(row.commit_id.len())]);
            buffer.push_str(" ");
            buffer.push_str(&row.commit.message);
            buffer.push('\n');
        }

        buffer
    }
}

/// ASCII-only renderer (no colors)
pub struct AsciiRenderer {
    graph_width: usize,
}

impl AsciiRenderer {
    pub fn new(graph_width: usize) -> Self {
        Self { graph_width }
    }

    /// Render row to ASCII characters only
    pub fn render_row(&self, row: &Row) -> String {
        let mut line = String::new();

        for (lane_idx, lane) in row.lanes.iter().enumerate() {
            if lane_idx >= self.graph_width {
                break;
            }

            let chars = match lane {
                Lane::Empty => "  ",
                Lane::Pass => "| ",
                Lane::Commit if lane_idx == row.primary_lane => "* ",
                Lane::Commit => "o ",
                Lane::BranchStart => "|-",
                Lane::Merge(_) => "|<",
                Lane::End => "|_",
            };

            line.push_str(chars);
        }

        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CommitNode;
    use chrono::Utc;

    fn create_test_row() -> Row {
        let commit = CommitNode::new(
            "abc123".to_string(),
            vec![],
            Utc::now(),
            "Test".to_string(),
            "Test commit".to_string(),
        );

        Row {
            commit_id: "abc123".to_string(),
            commit,
            lanes: vec![Lane::Commit, Lane::Pass, Lane::Empty],
            primary_lane: 0,
        }
    }

    #[test]
    fn test_tui_renderer() {
        let renderer = TuiRenderer::new(5, CharsetProfile::Utf8Straight);
        let row = create_test_row();
        let cells = renderer.render_row(&row);

        // Should have 10 cells (5 lanes * 2 chars)
        assert_eq!(cells.len(), 10);

        // First cell should be a commit
        assert_eq!(cells[0].ch, chars::COMMIT);

        // Second lane should be a pass
        assert_eq!(cells[2].ch, chars::VERTICAL);
    }

    #[test]
    fn test_ascii_renderer() {
        let renderer = AsciiRenderer::new(5);
        let row = create_test_row();
        let line = renderer.render_row(&row);

        assert!(line.starts_with("* | "));
    }
}
