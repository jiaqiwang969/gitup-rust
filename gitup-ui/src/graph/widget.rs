use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::Widget,
    style::{Style, Color},
};

use super::types::*;
use super::row_edges::{ProcessedRow, RowEdges};

pub struct AdvancedGraphWidget<'a> {
    pub graph: &'a GitGraph,
    pub rows: &'a [ProcessedRow],
    pub top: usize,
    pub ascii_only: bool,
}

impl<'a> AdvancedGraphWidget<'a> {
    pub fn new(graph: &'a GitGraph, rows: &'a [ProcessedRow]) -> Self {
        Self { graph, rows, top: 0, ascii_only: false }
    }

    pub fn top(mut self, top: usize) -> Self { self.top = top; self }
    pub fn ascii(mut self, ascii: bool) -> Self { self.ascii_only = ascii; self }

    fn lane_node_x(&self, area: Rect, lane: usize) -> u16 { area.x + (lane as u16) * 2 }
    fn lane_line_x(&self, area: Rect, lane: usize) -> u16 { self.lane_node_x(area, lane) }

    fn char_vert(&self) -> char { if self.ascii_only { '|' } else { '│' } }
    fn char_diag_up(&self) -> char { if self.ascii_only { '/' } else { '╱' } }
    fn char_diag_dn(&self) -> char { if self.ascii_only { '\\' } else { '╲' } }
    fn char_node(&self, is_head: bool) -> char { if self.ascii_only { if is_head { 'o' } else { '*' } } else { if is_head { '◉' } else { '●' } } }

    fn color_for_node(&self, id: &str) -> Color {
        if let Some(node) = self.graph.nodes.iter().find(|n| n.id == id) {
            if let Some(name) = &node.primary_branch {
                if let Some(c) = self.graph.branch_colors.get(name) { return *c; }
            }
            return self.graph.lanes.get(node.position.lane).map(|l| l.color).unwrap_or(Color::White);
        }
        Color::White
    }

    fn is_head(&self, id: &str) -> bool {
        // If any ref is HEAD -> id, mark as head
        // branches map holds name->oid; not enough. For now, treat newest row (top==0) as head if ids match.
        self.graph.nodes.first().map(|n| n.id == id).unwrap_or(false)
    }
}

impl<'a> Widget for AdvancedGraphWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let height = area.height as usize;
        // Index nodes by id for quick lookup
        let mut node_lane_by_id = std::collections::HashMap::new();
        for n in &self.graph.nodes {
            node_lane_by_id.insert(n.id.as_str(), n.position.lane);
        }

        // Render visible rows
        for i in 0..height {
            let row_idx = self.top + i;
            if row_idx >= self.rows.len() { break; }
            let screen_y = area.y + i as u16;
            let prow = &self.rows[row_idx];

            // 1) draw pass-through / starting / ending segments
            for (lane, re) in &prow.edges {
                let x_line = self.lane_line_x(area, *lane);
                let color = self.graph.lanes.get(*lane).map(|l| l.color).unwrap_or(Color::Blue);
                // pass-through vertical
                if re.pass_through.is_some() {
                    let cell = buf.get_mut(x_line, screen_y);
                    cell.set_char(self.char_vert());
                    cell.set_style(Style::default().fg(color));
                }
                // starting diagonal (towards parent lane): draw one diagonal glyph at current lane column
                if let Some(seg) = &re.starting {
                    // detect direction using parent lane
                    if let Some(&to_lane) = node_lane_by_id.get(seg.to.as_str()) {
                        let ch = if to_lane < *lane { self.char_diag_up() } else { self.char_diag_dn() };
                        let cell = buf.get_mut(x_line, screen_y);
                        cell.set_char(ch);
                        cell.set_style(Style::default().fg(color));
                    }
                }
                // ending marker: ensure a vertical at parent lane for clarity
                if re.ending.is_some() {
                    let cell = buf.get_mut(x_line, screen_y);
                    cell.set_char(self.char_vert());
                    cell.set_style(Style::default().fg(color));
                }
            }

            // 2) draw node at its lane
            if let Some(&lane) = node_lane_by_id.get(prow.commit_id.as_str()) {
                let x_node = self.lane_node_x(area, lane);
                let is_head = self.is_head(&prow.commit_id);
                let cell = buf.get_mut(x_node, screen_y);
                cell.set_char(self.char_node(is_head));
                let col = if is_head { Color::Green } else { self.color_for_node(&prow.commit_id) };
                cell.set_style(Style::default().fg(col));

                // 3) draw text summary to the right of max lanes with proper CJK handling
                let lanes_width = self.lane_node_x(area, self.graph.lanes.len().saturating_sub(1)) + 1 - area.x;
                let text_x = area.x + lanes_width + 2;
                if text_x < area.x + area.width {
                    let hash = &prow.commit_id;
                    let short = &hash[..hash.len().min(8)];
                    // find message
                    let message = self.graph.nodes.iter().find(|n| n.id == prow.commit_id)
                        .map(|n| n.message.as_str()).unwrap_or("");

                    // Format and truncate text with proper CJK handling
                    let text = format!("{} {}", short, message);
                    let available_width = (area.width - (text_x - area.x)) as usize;

                    // Use grapheme clusters for proper text handling
                    use unicode_segmentation::UnicodeSegmentation;
                    use unicode_width::UnicodeWidthStr;

                    let mut x = text_x;
                    let mut width_used = 0;

                    for grapheme in text.graphemes(true) {
                        // Calculate the display width of this grapheme
                        let width = UnicodeWidthStr::width(grapheme);

                        // Check if we have space for this grapheme
                        if width_used + width > available_width {
                            break;
                        }

                        // For multi-width characters (CJK, emoji), handle carefully
                        if width > 0 {
                            // Get the first character of the grapheme for rendering
                            if let Some(ch) = grapheme.chars().next() {
                                if x < area.x + area.width {
                                    let c = buf.get_mut(x, screen_y);
                                    c.set_char(ch);

                                    // For wide characters, skip the next cell
                                    if width == 2 && x + 1 < area.x + area.width {
                                        // Mark the next cell as continuation of wide char
                                        let c_next = buf.get_mut(x + 1, screen_y);
                                        c_next.set_char(' '); // Use space as placeholder
                                    }

                                    x += width as u16;
                                    width_used += width;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;
    use ratatui::buffer::Buffer;
    use ratatui::style::Color;
    use super::super::row_edges::RowEdgesBuilder;

    fn make_linear_graph() -> GitGraph {
        GitGraph {
            nodes: vec![
                GraphNode { id: "c2".into(), message: "msg2".into(), author: String::new(), date: 0,
                    position: GraphPosition { row: 0, lane: 0 }, node_type: NodeType::Regular, primary_branch: None },
                GraphNode { id: "c1".into(), message: "msg1".into(), author: String::new(), date: 0,
                    position: GraphPosition { row: 1, lane: 0 }, node_type: NodeType::Initial, primary_branch: None },
            ],
            edges: vec![ GraphEdge { from: "c2".into(), to: "c1".into(), lane: 0 } ],
            lanes: vec![ Lane { index: 0, color: Color::Cyan, active: true } ],
            branches: Default::default(),
            tags: Default::default(),
            branch_colors: Default::default(),
        }
    }

    #[test]
    fn render_linear_ascii() {
        let g = make_linear_graph();
        let rows = RowEdgesBuilder::build(&g);
        let area = Rect { x: 0, y: 0, width: 40, height: 3 };
        let mut buf = Buffer::empty(area);
        let w = AdvancedGraphWidget::new(&g, &rows).ascii(true);
        w.render(area, &mut buf);
        // Check that row 0 col 0 has node (head in ascii = 'o')
        assert_eq!(buf.get(0,0).symbol(), "o");
        // Row 1 is the parent commit row; node '*' should be at same column (col=0)
        assert_eq!(buf.get(0,1).symbol(), "*");
    }
}
