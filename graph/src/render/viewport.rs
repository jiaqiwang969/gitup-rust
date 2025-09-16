use crate::layout::Row;
use crate::render::TuiRenderer;
use std::cmp;

/// Viewport represents the visible area of the graph
#[derive(Debug, Clone)]
pub struct Viewport {
    /// Top row index (0-based)
    pub top: usize,
    /// Number of visible rows
    pub height: usize,
    /// Current cursor/selected row (absolute index)
    pub cursor: usize,
    /// Total number of rows
    pub total_rows: usize,
}

impl Viewport {
    pub fn new(height: usize, total_rows: usize) -> Self {
        Self {
            top: 0,
            height,
            cursor: 0,
            total_rows,
        }
    }

    /// Get the visible range of rows
    pub fn visible_range(&self) -> (usize, usize) {
        let start = self.top;
        let end = cmp::min(self.top + self.height, self.total_rows);
        (start, end)
    }

    /// Check if a row is visible
    pub fn is_visible(&self, row_idx: usize) -> bool {
        row_idx >= self.top && row_idx < self.top + self.height
    }

    /// Scroll up by n rows
    pub fn scroll_up(&mut self, n: usize) {
        if self.top >= n {
            self.top -= n;
        } else {
            self.top = 0;
        }
    }

    /// Scroll down by n rows
    pub fn scroll_down(&mut self, n: usize) {
        let max_top = self.total_rows.saturating_sub(self.height);
        self.top = cmp::min(self.top + n, max_top);
    }

    /// Page up (scroll by viewport height)
    pub fn page_up(&mut self) {
        self.scroll_up(self.height);
    }

    /// Page down (scroll by viewport height)
    pub fn page_down(&mut self) {
        self.scroll_down(self.height);
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            // Auto-scroll if cursor moves out of view
            if self.cursor < self.top {
                self.top = self.cursor;
            }
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        if self.cursor < self.total_rows - 1 {
            self.cursor += 1;
            // Auto-scroll if cursor moves out of view
            if self.cursor >= self.top + self.height {
                self.top = self.cursor - self.height + 1;
            }
        }
    }

    /// Jump to a specific row
    pub fn jump_to(&mut self, row_idx: usize) {
        self.cursor = cmp::min(row_idx, self.total_rows.saturating_sub(1));
        self.center_on_cursor();
    }

    /// Center viewport on cursor
    pub fn center_on_cursor(&mut self) {
        if self.total_rows <= self.height {
            self.top = 0;
        } else {
            let center_offset = self.height / 2;
            if self.cursor < center_offset {
                self.top = 0;
            } else if self.cursor + center_offset >= self.total_rows {
                self.top = self.total_rows - self.height;
            } else {
                self.top = self.cursor - center_offset;
            }
        }
    }

    /// Jump to top
    pub fn jump_to_top(&mut self) {
        self.cursor = 0;
        self.top = 0;
    }

    /// Jump to bottom
    pub fn jump_to_bottom(&mut self) {
        self.cursor = self.total_rows.saturating_sub(1);
        self.top = self.total_rows.saturating_sub(self.height);
    }

    /// Get progress percentage
    pub fn progress(&self) -> f32 {
        if self.total_rows == 0 {
            return 0.0;
        }
        (self.cursor as f32 / self.total_rows as f32) * 100.0
    }
}

/// Virtual scrolling graph renderer
pub struct VirtualRenderer {
    viewport: Viewport,
    renderer: TuiRenderer,
    rows: Vec<Row>,
}

impl VirtualRenderer {
    pub fn new(rows: Vec<Row>, viewport_height: usize, graph_width: usize) -> Self {
        let total_rows = rows.len();
        Self {
            viewport: Viewport::new(viewport_height, total_rows),
            renderer: TuiRenderer::new(graph_width),
            rows,
        }
    }

    /// Render only the visible portion
    pub fn render(&self) -> String {
        let (start, end) = self.viewport.visible_range();

        if start >= self.rows.len() {
            return String::new();
        }

        let visible_rows = &self.rows[start..end.min(self.rows.len())];
        let mut output = String::new();

        for (idx, row) in visible_rows.iter().enumerate() {
            let absolute_idx = start + idx;
            let cells = self.renderer.render_row(row);

            // Add cursor indicator
            if absolute_idx == self.viewport.cursor {
                output.push_str("\x1b[7m"); // Reverse video
            }

            // Render cells
            for cell in &cells {
                output.push_str(cell.color.to_ansi());
                output.push(cell.ch);
            }
            output.push_str("\x1b[0m"); // Reset

            // Add commit info
            output.push_str(" ");
            if absolute_idx == self.viewport.cursor {
                output.push_str("\x1b[1m"); // Bold
            }
            output.push_str(&row.commit_id[..8.min(row.commit_id.len())]);
            output.push_str(" ");
            output.push_str(&row.commit.message);
            if absolute_idx == self.viewport.cursor {
                output.push_str("\x1b[0m"); // Reset
            }
            output.push('\n');
        }

        // Add status line
        output.push_str(&format!(
            "\n[{}/{}] {:.0}% | Use arrows/hjkl to navigate, g/G for top/bottom",
            self.viewport.cursor + 1,
            self.viewport.total_rows,
            self.viewport.progress()
        ));

        output
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: char) -> bool {
        match key {
            // Vim-style navigation
            'k' => self.viewport.cursor_up(),
            'j' => self.viewport.cursor_down(),
            'g' => self.viewport.jump_to_top(),
            'G' => self.viewport.jump_to_bottom(),
            '\x15' => self.viewport.page_up(), // Ctrl-U
            '\x04' => self.viewport.page_down(), // Ctrl-D
            'z' => self.viewport.center_on_cursor(),
            'q' => return false, // Quit
            _ => {}
        }
        true
    }

    /// Get current viewport
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Update viewport dimensions
    pub fn resize(&mut self, new_height: usize) {
        self.viewport.height = new_height;
        self.viewport.center_on_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CommitNode;
    use chrono::Utc;

    fn create_test_rows(count: usize) -> Vec<Row> {
        (0..count)
            .map(|i| {
                let commit = CommitNode::new(
                    format!("commit{}", i),
                    vec![],
                    Utc::now(),
                    "Test".to_string(),
                    format!("Commit {}", i),
                );
                Row {
                    commit_id: format!("commit{}", i),
                    commit,
                    lanes: vec![crate::layout::Lane::Commit],
                    primary_lane: 0,
                }
            })
            .collect()
    }

    #[test]
    fn test_viewport_scrolling() {
        let mut viewport = Viewport::new(10, 100);

        assert_eq!(viewport.visible_range(), (0, 10));

        viewport.scroll_down(5);
        assert_eq!(viewport.visible_range(), (5, 15));

        viewport.page_down();
        assert_eq!(viewport.visible_range(), (15, 25));

        viewport.scroll_up(20);
        assert_eq!(viewport.visible_range(), (0, 10));
    }

    #[test]
    fn test_cursor_movement() {
        let mut viewport = Viewport::new(10, 100);

        assert_eq!(viewport.cursor, 0);

        viewport.cursor_down();
        assert_eq!(viewport.cursor, 1);

        viewport.jump_to(50);
        assert_eq!(viewport.cursor, 50);
        assert!(viewport.is_visible(50));

        viewport.jump_to_bottom();
        assert_eq!(viewport.cursor, 99);
    }

    #[test]
    fn test_virtual_renderer() {
        let rows = create_test_rows(20);
        let renderer = VirtualRenderer::new(rows, 10, 5);

        let output = renderer.render();
        assert!(!output.is_empty());

        // Should only render 10 rows
        let lines: Vec<_> = output.lines().collect();
        assert!(lines.len() <= 12); // 10 rows + status line + empty line
    }
}