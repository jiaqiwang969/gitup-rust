use crate::layout::{Row, Lane};

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
}

impl TuiRenderer {
    pub fn new(graph_width: usize) -> Self {
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
                    cells[pos] = Cell::new(chars::SPACE, color);
                    cells[pos + 1] = Cell::new(chars::SPACE, color);
                }
                Lane::Pass => {
                    cells[pos] = Cell::new(chars::VERTICAL, color);
                    cells[pos + 1] = Cell::new(chars::SPACE, color);
                }
                Lane::Commit => {
                    let ch = if lane_idx == row.primary_lane {
                        chars::COMMIT
                    } else {
                        chars::COMMIT_EMPTY
                    };
                    cells[pos] = Cell::new(ch, color);
                    cells[pos + 1] = Cell::new(chars::SPACE, color);
                }
                Lane::BranchStart => {
                    cells[pos] = Cell::new(chars::BRANCH_UP_RIGHT, color);
                    cells[pos + 1] = Cell::new(chars::HORIZONTAL, color);
                }
                Lane::Merge(targets) => {
                    // Simple merge rendering
                    cells[pos] = Cell::new(chars::MERGE_LEFT, color);
                    cells[pos + 1] = Cell::new(chars::HORIZONTAL, color);

                    // Draw lines to merge targets
                    for &target in targets {
                        if target < self.graph_width && target != lane_idx {
                            let target_pos = target * 2;
                            if target_pos < cells.len() {
                                cells[target_pos] = Cell::new(chars::VERTICAL, self.lane_colors[target]);
                            }
                        }
                    }
                }
                Lane::End => {
                    cells[pos] = Cell::new(chars::BRANCH_DOWN_LEFT, color);
                    cells[pos + 1] = Cell::new(chars::SPACE, color);
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
        let renderer = TuiRenderer::new(5);
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