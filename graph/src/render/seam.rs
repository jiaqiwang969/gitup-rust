use crate::layout::{Lane, LaneIdx};
use smallvec::SmallVec;

// Direction bit masks
const DIR_N: u8 = 0x01;  // North (up)
const DIR_S: u8 = 0x02;  // South (down)
const DIR_E: u8 = 0x04;  // East (right)
const DIR_W: u8 = 0x08;  // West (left)
const DIR_NE: u8 = 0x10; // Northeast
const DIR_NW: u8 = 0x20; // Northwest
const DIR_SE: u8 = 0x40; // Southeast
const DIR_SW: u8 = 0x80; // Southwest

/// Calculate direction masks for a lane based on its connections
fn calculate_direction_masks(lane: &Lane, row: &crate::layout::Row, col: usize) -> (u8, u8) {
    let mut incoming = 0u8;
    let mut outgoing = 0u8;

    match lane {
        Lane::Pass => {
            // Vertical pass-through
            incoming |= DIR_N;
            outgoing |= DIR_S;
        }
        Lane::Commit => {
            // Commit node - check for incoming from above
            incoming |= DIR_N;
            // Check if continuing down
            if has_continuation_below(row, col) {
                outgoing |= DIR_S;
            }
        }
        Lane::BranchStart => {
            // Branch starting - diagonal connections
            incoming |= DIR_N;
            outgoing |= DIR_S | DIR_SE;
        }
        Lane::Merge(sources) => {
            // Multiple incoming lanes
            incoming |= DIR_N;
            for &src in sources {
                if src < col {
                    incoming |= DIR_NW;
                } else if src > col {
                    incoming |= DIR_NE;
                }
            }
            outgoing |= DIR_S;
        }
        Lane::End => {
            // Ending lane
            incoming |= DIR_N;
        }
        Lane::Empty => {
            // No connections
        }
    }

    (incoming, outgoing)
}

/// Check if a lane continues below (simplified check)
fn has_continuation_below(row: &crate::layout::Row, col: usize) -> bool {
    // In a real implementation, we'd check the next row
    // For now, assume most lanes continue except End
    !matches!(row.lanes.get(col), Some(Lane::End) | None)
}

/// Select appropriate glyph based on direction masks
fn select_glyph_from_dirs(incoming: u8, outgoing: u8, prefer_straight: bool) -> char {
    // Quick paths for common cases
    if incoming == DIR_N && outgoing == DIR_S {
        return '│'; // Straight vertical
    }
    if incoming == DIR_W && outgoing == DIR_E {
        return '─'; // Straight horizontal
    }

    // Count directions
    let in_count = incoming.count_ones();
    let out_count = outgoing.count_ones();

    // Junction detection
    if in_count > 1 && out_count > 1 {
        return '┼'; // Cross junction
    }
    if in_count > 1 {
        // Multiple incoming - merge points
        if incoming & DIR_N != 0 && incoming & (DIR_NE | DIR_NW) != 0 {
            return '┬'; // Top merge
        }
        if incoming & DIR_W != 0 && incoming & DIR_E != 0 {
            return '┤'; // Side merge
        }
    }
    if out_count > 1 {
        // Multiple outgoing - branch points
        if outgoing & DIR_S != 0 && outgoing & (DIR_SE | DIR_SW) != 0 {
            return '┴'; // Bottom branch
        }
        if outgoing & DIR_W != 0 && outgoing & DIR_E != 0 {
            return '├'; // Side branch
        }
    }

    // Corners
    if incoming & DIR_N != 0 && outgoing & DIR_E != 0 { return '└'; }
    if incoming & DIR_N != 0 && outgoing & DIR_W != 0 { return '┘'; }
    if incoming & DIR_S != 0 && outgoing & DIR_E != 0 { return '┌'; }
    if incoming & DIR_S != 0 && outgoing & DIR_W != 0 { return '┐'; }
    if incoming & DIR_W != 0 && outgoing & DIR_N != 0 { return '┘'; }
    if incoming & DIR_W != 0 && outgoing & DIR_S != 0 { return '┐'; }
    if incoming & DIR_E != 0 && outgoing & DIR_N != 0 { return '└'; }
    if incoming & DIR_E != 0 && outgoing & DIR_S != 0 { return '┌'; }

    // Default to simple lines
    if incoming & (DIR_N | DIR_S) != 0 || outgoing & (DIR_N | DIR_S) != 0 {
        return '│';
    }
    if incoming & (DIR_E | DIR_W) != 0 || outgoing & (DIR_E | DIR_W) != 0 {
        return '─';
    }

    ' ' // Empty if no connections
}

/// Check if one glyph should override another (priority system)
fn should_override(existing: char, new: char) -> bool {
    // Priority order: junctions > corners > lines > space
    let priority = |c: char| match c {
        '┼' | '┬' | '┴' | '├' | '┤' => 4, // Junctions
        '┌' | '┐' | '└' | '┘' => 3,       // Corners
        '│' | '─' => 2,                   // Lines
        '╱' | '╲' => 1,                   // Diagonals
        ' ' => 0,                          // Space
        _ => 1,                            // Other
    };

    priority(new) > priority(existing)
}

/// Column state carried over from previous row
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnState {
    /// Lane index in the grid
    pub lane_idx: LaneIdx,
    /// Type of line entering from above
    pub entering_type: EnteringType,
    /// Color for this lane
    pub color_idx: usize,
    /// Direction mask for incoming edges (bitmask: N,S,E,W,NE,NW,SE,SW)
    pub incoming: u8,
    /// Direction mask for outgoing edges
    pub outgoing: u8,
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

            // Calculate direction masks based on lane type and connections
            let (incoming, outgoing) = calculate_direction_masks(lane, row, idx);

            let state = match lane {
                Lane::Pass | Lane::Commit => Some(ColumnState {
                    lane_idx: idx,
                    entering_type: EnteringType::Vertical,
                    color_idx: idx % 6,
                    incoming,
                    outgoing,
                }),
                Lane::BranchStart => Some(ColumnState {
                    lane_idx: idx,
                    entering_type: EnteringType::DiagonalRight,
                    color_idx: idx % 6,
                    incoming,
                    outgoing,
                }),
                Lane::Merge(_) => Some(ColumnState {
                    lane_idx: idx,
                    entering_type: EnteringType::Merge,
                    color_idx: idx % 6,
                    incoming,
                    outgoing,
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

    /// Apply carry-over to the first visible row with proper glyph selection
    pub fn apply_to_first_row(&self, cells: &mut [crate::render::Cell], width: usize) {
        for (idx, col_state) in self.columns.iter().enumerate() {
            if idx >= width * 2 {
                break;
            }

            if let Some(state) = col_state {
                let pos = idx * 2;
                let color = crate::render::Color::from_index(state.color_idx);

                // Select proper glyph based on direction masks
                let glyph = select_glyph_from_dirs(state.incoming, state.outgoing, true);

                // Apply the glyph if position is valid and empty
                if pos < cells.len() {
                    // Only override if cell is empty or we have a better glyph
                    if cells[pos].ch == ' ' || should_override(cells[pos].ch, glyph) {
                        cells[pos] = crate::render::Cell::new(glyph, color);
                    }
                }

                // Handle diagonal connections
                match state.entering_type {
                    EnteringType::DiagonalLeft => {
                        if pos > 0 && pos - 1 < cells.len() {
                            cells[pos - 1] = crate::render::Cell::new('╲', color);
                        }
                    }
                    EnteringType::DiagonalRight => {
                        if pos + 1 < cells.len() {
                            cells[pos + 1] = crate::render::Cell::new('╱', color);
                        }
                    }
                    _ => {}
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