use crate::layout::{Lane, LaneIdx};
use smallvec::SmallVec;

/// Column state carried over from previous row
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnState {
    /// Lane index in the grid
    pub lane_idx: LaneIdx,
    /// Type of line entering from above
    pub entering_type: EnteringType,
    /// Color for this lane
    pub color_idx: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnteringType {
    /// No line from above
    None,
    /// Vertical line continuing down
    Vertical,
    /// Diagonal from left
    DiagonalLeft,
    /// Diagonal from right
    DiagonalRight,
    /// Merge line
    Merge,
}

/// State carried between viewport boundaries
#[derive(Debug, Clone)]
pub struct ViewportCarryOver {
    /// States for each column
    pub columns: SmallVec<[Option<ColumnState>; 16]>,
    /// Previous row's lanes for continuity
    pub prev_lanes: Option<Vec<Lane>>,
}

impl ViewportCarryOver {
    pub fn new(width: usize) -> Self {
        Self {
            columns: SmallVec::from_vec(vec![None; width]),
            prev_lanes: None,
        }
    }

    /// Calculate carry-over from a row that's above the viewport
    pub fn from_row(row: &crate::layout::Row, width: usize) -> Self {
        let mut columns = SmallVec::from_vec(vec![None; width]);

        for (idx, lane) in row.lanes.iter().enumerate() {
            if idx >= width {
                break;
            }

            let state = match lane {
                Lane::Pass | Lane::Commit => Some(ColumnState {
                    lane_idx: idx,
                    entering_type: EnteringType::Vertical,
                    color_idx: idx % 6,
                }),
                Lane::BranchStart => Some(ColumnState {
                    lane_idx: idx,
                    entering_type: EnteringType::DiagonalRight,
                    color_idx: idx % 6,
                }),
                Lane::Merge(_) => Some(ColumnState {
                    lane_idx: idx,
                    entering_type: EnteringType::Merge,
                    color_idx: idx % 6,
                }),
                Lane::Empty | Lane::End => None,
            };

            columns[idx] = state;
        }

        Self {
            columns,
            prev_lanes: Some(row.lanes.clone()),
        }
    }

    /// Apply carry-over to the first visible row
    pub fn apply_to_first_row(&self, cells: &mut [crate::render::Cell], width: usize) {
        for (idx, col_state) in self.columns.iter().enumerate() {
            if idx >= width * 2 {
                break;
            }

            if let Some(state) = col_state {
                let pos = idx * 2;
                let color = crate::render::Color::from_index(state.color_idx);

                // Draw entering lines based on type
                match state.entering_type {
                    EnteringType::Vertical => {
                        // Continue vertical line from above
                        if pos < cells.len() && cells[pos].ch == ' ' {
                            cells[pos] = crate::render::Cell::new('│', color);
                        }
                    }
                    EnteringType::DiagonalLeft => {
                        // Diagonal entering from left above
                        if pos > 0 && pos - 1 < cells.len() {
                            cells[pos - 1] = crate::render::Cell::new('\\', color);
                        }
                    }
                    EnteringType::DiagonalRight => {
                        // Diagonal entering from right above
                        if pos + 1 < cells.len() {
                            cells[pos + 1] = crate::render::Cell::new('/', color);
                        }
                    }
                    EnteringType::Merge => {
                        // Complex merge pattern
                        if pos < cells.len() {
                            cells[pos] = crate::render::Cell::new('┬', color);
                        }
                    }
                    EnteringType::None => {}
                }
            }
        }
    }
}

/// Extended viewport with carry-over support
pub struct SeamlessViewport {
    pub top: usize,
    pub height: usize,
    pub cursor: usize,
    pub total_rows: usize,
    /// Carry-over state from row above viewport
    pub carry_over: Option<ViewportCarryOver>,
}

impl SeamlessViewport {
    pub fn new(height: usize, total_rows: usize) -> Self {
        Self {
            top: 0,
            height,
            cursor: 0,
            total_rows,
            carry_over: None,
        }
    }

    /// Update carry-over when viewport moves
    pub fn update_carry_over(&mut self, rows: &[crate::layout::Row], width: usize) {
        if self.top > 0 && self.top - 1 < rows.len() {
            // Get the row just above the viewport
            let above_row = &rows[self.top - 1];
            self.carry_over = Some(ViewportCarryOver::from_row(above_row, width));
        } else {
            self.carry_over = None;
        }
    }

    /// Scroll operations with carry-over update
    pub fn scroll_up(&mut self, n: usize, rows: &[crate::layout::Row], width: usize) {
        if self.top >= n {
            self.top -= n;
        } else {
            self.top = 0;
        }
        self.update_carry_over(rows, width);
    }

    pub fn scroll_down(&mut self, n: usize, rows: &[crate::layout::Row], width: usize) {
        let max_top = self.total_rows.saturating_sub(self.height);
        self.top = std::cmp::min(self.top + n, max_top);
        self.update_carry_over(rows, width);
    }

    pub fn visible_range(&self) -> (usize, usize) {
        let start = self.top;
        let end = std::cmp::min(self.top + self.height, self.total_rows);
        (start, end)
    }
}

/// Color extension for proper indexing
impl crate::render::Color {
    pub fn from_index(idx: usize) -> Self {
        const COLORS: &[crate::render::Color] = &[
            crate::render::Color::Blue,
            crate::render::Color::Green,
            crate::render::Color::Red,
            crate::render::Color::Yellow,
            crate::render::Color::Magenta,
            crate::render::Color::Cyan,
        ];
        COLORS[idx % COLORS.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CommitNode;
    use crate::layout::Row;
    use chrono::Utc;

    fn create_test_row_with_lanes(lanes: Vec<Lane>) -> Row {
        let commit = CommitNode::new(
            "test".to_string(),
            vec![],
            Utc::now(),
            "Test".to_string(),
            "Test".to_string(),
        );

        Row {
            commit_id: "test".to_string(),
            commit,
            lanes,
            primary_lane: 0,
        }
    }

    #[test]
    fn test_carry_over_vertical() {
        let row = create_test_row_with_lanes(vec![
            Lane::Pass,
            Lane::Commit,
            Lane::Empty,
        ]);

        let carry = ViewportCarryOver::from_row(&row, 3);

        assert!(carry.columns[0].is_some());
        assert_eq!(carry.columns[0].as_ref().unwrap().entering_type, EnteringType::Vertical);
        assert!(carry.columns[1].is_some());
        assert_eq!(carry.columns[1].as_ref().unwrap().entering_type, EnteringType::Vertical);
        assert!(carry.columns[2].is_none());
    }

    #[test]
    fn test_seamless_viewport_scrolling() {
        let rows = vec![
            create_test_row_with_lanes(vec![Lane::Pass, Lane::Empty]),
            create_test_row_with_lanes(vec![Lane::Commit, Lane::Pass]),
            create_test_row_with_lanes(vec![Lane::Pass, Lane::Commit]),
        ];

        let mut viewport = SeamlessViewport::new(2, 3);

        // Scroll down - should capture carry-over from row 0
        viewport.scroll_down(1, &rows, 2);
        assert!(viewport.carry_over.is_some());

        let carry = viewport.carry_over.as_ref().unwrap();
        assert!(carry.columns[0].is_some());
    }
}