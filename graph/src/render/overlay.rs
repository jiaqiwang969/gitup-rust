use crate::layout::{Row, Lane};
use crate::render::{Cell, Color};
use std::fmt::Write;

/// Debug overlay for visualizing graph internals
pub struct DebugOverlay {
    enabled: bool,
    show_lanes: bool,
    show_directions: bool,
    show_seams: bool,
    show_colors: bool,
}

impl DebugOverlay {
    pub fn new() -> Self {
        Self {
            enabled: false,
            show_lanes: true,
            show_directions: true,
            show_seams: false,
            show_colors: false,
        }
    }

    /// Toggle overlay on/off
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Check if overlay is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Render debug overlay on top of existing cells
    pub fn render_overlay(&self, cells: &mut Vec<Cell>, rows: &[Row], viewport_top: usize, width: usize, height: usize) {
        if !self.enabled {
            return;
        }

        for (row_idx, row) in rows.iter().skip(viewport_top).take(height).enumerate() {
            let y = row_idx;

            // Show lane indices
            if self.show_lanes {
                for (lane_idx, lane) in row.lanes.iter().enumerate() {
                    let x = lane_idx * 2;
                    if x < width {
                        let cell_idx = y * width + x;
                        if cell_idx < cells.len() {
                            // Overlay lane index as small number
                            let digit = (lane_idx % 10).to_string().chars().next().unwrap();
                            cells[cell_idx] = Cell::new(digit, Color::Gray);
                        }
                    }
                }
            }

            // Show commit ID hint
            let info_x = width / 3;
            if info_x < width {
                let cell_idx = y * width + info_x;
                if cell_idx < cells.len() {
                    // Show first 4 chars of commit ID
                    let id_hint = &row.commit_id[..4.min(row.commit_id.len())];
                    for (i, ch) in id_hint.chars().enumerate() {
                        let idx = cell_idx + i;
                        if idx < cells.len() {
                            cells[idx] = Cell::new(ch, Color::Blue);
                        }
                    }
                }
            }
        }
    }

    /// Export debug info as JSON
    pub fn export_debug_info(&self, rows: &[Row]) -> String {
        let mut output = String::new();
        writeln!(&mut output, "{{").unwrap();
        writeln!(&mut output, "  \"rows\": [").unwrap();

        for (i, row) in rows.iter().enumerate() {
            writeln!(&mut output, "    {{").unwrap();
            writeln!(&mut output, "      \"index\": {},", i).unwrap();
            writeln!(&mut output, "      \"commit_id\": \"{}\",", &row.commit_id[..8.min(row.commit_id.len())]).unwrap();
            writeln!(&mut output, "      \"primary_lane\": {},", row.primary_lane).unwrap();
            write!(&mut output, "      \"lanes\": [").unwrap();

            for (j, lane) in row.lanes.iter().enumerate() {
                let lane_str = match lane {
                    Lane::Empty => "Empty",
                    Lane::Pass => "Pass",
                    Lane::Commit => "Commit",
                    Lane::BranchStart => "BranchStart",
                    Lane::Merge(_) => "Merge",
                    Lane::End => "End",
                };
                write!(&mut output, "\"{}\"", lane_str).unwrap();
                if j < row.lanes.len() - 1 {
                    write!(&mut output, ", ").unwrap();
                }
            }

            writeln!(&mut output, "]").unwrap();
            write!(&mut output, "    }}").unwrap();
            if i < rows.len() - 1 {
                writeln!(&mut output, ",").unwrap();
            } else {
                writeln!(&mut output).unwrap();
            }
        }

        writeln!(&mut output, "  ]").unwrap();
        writeln!(&mut output, "}}").unwrap();

        output
    }

    /// Dump grid as text for debugging
    pub fn dump_grid(&self, cells: &[Cell], width: usize, height: usize) -> String {
        let mut output = String::new();

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                if idx < cells.len() {
                    output.push(cells[idx].ch);
                } else {
                    output.push(' ');
                }
            }
            output.push('\n');
        }

        output
    }
}

/// Key handler for debug overlay
pub fn handle_debug_key(overlay: &mut DebugOverlay, key: char) -> bool {
    match key {
        'E' | 'e' => {
            overlay.toggle();
            true
        }
        'l' if overlay.enabled => {
            overlay.show_lanes = !overlay.show_lanes;
            true
        }
        'd' if overlay.enabled => {
            overlay.show_directions = !overlay.show_directions;
            true
        }
        's' if overlay.enabled => {
            overlay.show_seams = !overlay.show_seams;
            true
        }
        'c' if overlay.enabled => {
            overlay.show_colors = !overlay.show_colors;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CommitNode;
    use chrono::Utc;

    #[test]
    fn test_overlay_toggle() {
        let mut overlay = DebugOverlay::new();
        assert!(!overlay.is_enabled());

        overlay.toggle();
        assert!(overlay.is_enabled());

        overlay.toggle();
        assert!(!overlay.is_enabled());
    }

    #[test]
    fn test_export_debug_info() {
        let overlay = DebugOverlay::new();

        let commit = CommitNode::new(
            "abc123".to_string(),
            vec![],
            Utc::now(),
            "Test".to_string(),
            "Test commit".to_string(),
        );

        let rows = vec![Row {
            commit_id: "abc123".to_string(),
            commit,
            lanes: vec![Lane::Commit, Lane::Pass, Lane::Empty],
            primary_lane: 0,
        }];

        let json = overlay.export_debug_info(&rows);
        assert!(json.contains("\"commit_id\": \"abc123\""));
        assert!(json.contains("\"lanes\": [\"Commit\", \"Pass\", \"Empty\"]"));
    }
}