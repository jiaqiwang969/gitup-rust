use anyhow::Result;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use graph::{
    GitWalker, SeamlessViewport,
    TextLayout, CjkMode, CommitMessageFormatter,
    CellRouter, CharsetProfile, ConflictResolver,
    ViewportCarryOver, Color as GraphColor,
};

/// Integration to add enhanced graph to existing TUI
pub struct EnhancedGraphIntegration {
    /// The DAG from new graph module
    dag: graph::Dag,
    /// Compact rows with lane compression
    rows: Vec<graph::Row>,
    /// Seamless viewport
    viewport: SeamlessViewport,
    /// Text formatter
    formatter: CommitMessageFormatter,
    /// Cell router - used internally for routing decisions
    #[allow(dead_code)]
    router: CellRouter,
    /// Conflict resolver - used internally for conflict resolution
    #[allow(dead_code)]
    resolver: ConflictResolver,
    /// Renderer
    renderer: graph::TuiRenderer,
}

impl EnhancedGraphIntegration {
    /// Create from repository path
    pub fn new(repo_path: &str) -> Result<Self> {
        // Load repository using our walker
        let walker = GitWalker::new(Some(repo_path))?;
        let dag = walker.into_dag(None)?;

        // Build simple layout with continuous lines
        let mut builder = graph::layout::SimpleGraphBuilder::new(12);
        let rows = builder.build_rows(&dag);

        // Create seamless viewport
        let viewport = SeamlessViewport::new(30, rows.len());

        // CJK support
        let cjk_mode = if TextLayout::detect_cjk_from_locale() {
            CjkMode::Auto
        } else {
            CjkMode::Off
        };
        let formatter = CommitMessageFormatter::new(cjk_mode);

        // Charset based on terminal
        let profile = detect_charset_profile();
        let router = CellRouter::new(profile);
        let resolver = ConflictResolver::new(profile);

        // Create renderer with detected charset profile
        let renderer = graph::TuiRenderer::new(12, profile);

        Ok(Self {
            dag,
            rows,
            viewport,
            formatter,
            router,
            resolver,
            renderer,
        })
    }

    /// Render to a specific area
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Update viewport carry-over
        self.viewport.update_carry_over(&self.rows, area.width as usize / 2);

        // Get visible range
        let (start, end) = self.viewport.visible_range();
        let visible_rows = &self.rows[start.min(self.rows.len())..end.min(self.rows.len())];

        // Apply carry-over to first row if needed
        let mut first_row_cells = None;
        if let Some(carry_over) = &self.viewport.carry_over {
            if !visible_rows.is_empty() {
                let mut cells = self.renderer.render_row(&visible_rows[0]);
                carry_over.apply_to_first_row(&mut cells, area.width as usize / 2);
                first_row_cells = Some(cells);
            }
        }

        // Render each visible row
        for (idx, row) in visible_rows.iter().enumerate() {
            let y = area.y + idx as u16;
            if y >= area.y + area.height {
                break;
            }

            // Get cells for this row
            let cells = if idx == 0 && first_row_cells.is_some() {
                first_row_cells.take().unwrap()
            } else {
                self.renderer.render_row(row)
            };

            // Draw cells to buffer
            let mut x = area.x;
            for cell in cells.iter().take(area.width as usize) {
                if x >= area.x + area.width {
                    break;
                }

                let style = convert_color_to_style(cell.color);
                // For ratatui, we can directly index the buffer
                let buf_cell = &mut buf[(x, y)];
                buf_cell.set_char(cell.ch).set_style(style);

                x += 1;
            }

            // Draw commit info (SHA + message) with proper CJK handling
            let info_x = area.x + (area.width / 3).min(24);
            if info_x < area.x + area.width {
                let sha = &row.commit_id[..8.min(row.commit_id.len())];
                let message = &row.commit.message;

                let info_width = (area.width - (info_x - area.x)) as usize;
                let formatted = self.formatter.format(sha, message, info_width);

                // Use Unicode-aware rendering for CJK text
                use unicode_segmentation::UnicodeSegmentation;
                use unicode_width::UnicodeWidthStr;

                let mut x = info_x;
                let mut width_used = 0;

                for grapheme in formatted.graphemes(true) {
                    // Calculate display width
                    let width = UnicodeWidthStr::width(grapheme);

                    // Check if we have space
                    if width_used + width > info_width {
                        break;
                    }

                    if x >= area.x + area.width {
                        break;
                    }

                    // Highlight if selected
                    let style = if idx == self.viewport.cursor - start {
                        Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    // Render the grapheme
                    if width > 0 {
                        if let Some(ch) = grapheme.chars().next() {
                            if x < area.x + area.width {
                                let buf_cell = &mut buf[(x, y)];
                                buf_cell.set_char(ch).set_style(style);

                                // For wide characters (CJK, emoji), handle the next cell
                                if width == 2 && x + 1 < area.x + area.width {
                                    // Clear the next cell for wide character continuation
                                    let buf_cell_next = &mut buf[(x + 1, y)];
                                    buf_cell_next.set_char(' ');
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

    /// Handle keyboard input
    pub fn handle_input(&mut self, code: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;

        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.viewport.cursor < self.viewport.total_rows - 1 {
                    self.viewport.cursor += 1;
                    if self.viewport.cursor >= self.viewport.top + self.viewport.height {
                        self.viewport.top = self.viewport.cursor - self.viewport.height + 1;
                    }
                }
                self.viewport.update_carry_over(&self.rows, 12);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.viewport.cursor > 0 {
                    self.viewport.cursor -= 1;
                    if self.viewport.cursor < self.viewport.top {
                        self.viewport.top = self.viewport.cursor;
                    }
                }
                self.viewport.update_carry_over(&self.rows, 12);
                true
            }
            KeyCode::Char('g') => {
                self.viewport.cursor = 0;
                self.viewport.top = 0;
                self.viewport.update_carry_over(&self.rows, 12);
                true
            }
            KeyCode::Char('G') => {
                self.viewport.cursor = self.viewport.total_rows.saturating_sub(1);
                self.viewport.top = self.viewport.total_rows.saturating_sub(self.viewport.height);
                self.viewport.update_carry_over(&self.rows, 12);
                true
            }
            KeyCode::PageDown => {
                let page = self.viewport.height;
                self.viewport.scroll_down(page, &self.rows, 12);
                true
            }
            KeyCode::PageUp => {
                let page = self.viewport.height;
                self.viewport.scroll_up(page, &self.rows, 12);
                true
            }
            _ => false,
        }
    }

    /// Get selected commit SHA
    pub fn selected_commit(&self) -> Option<String> {
        if self.viewport.cursor < self.rows.len() {
            Some(self.rows[self.viewport.cursor].commit_id.clone())
        } else {
            None
        }
    }

    /// Refresh graph from repository
    pub fn refresh(&mut self, repo_path: &str) -> Result<()> {
        let walker = GitWalker::new(Some(repo_path))?;
        self.dag = walker.into_dag(None)?;

        let mut builder = graph::layout::SimpleGraphBuilder::new(12);
        self.rows = builder.build_rows(&self.dag);

        self.viewport = SeamlessViewport::new(30, self.rows.len());
        Ok(())
    }
}

/// Convert graph color to ratatui style
fn convert_color_to_style(color: GraphColor) -> Style {
    let fg = match color {
        GraphColor::Red => Color::Red,
        GraphColor::Green => Color::Green,
        GraphColor::Yellow => Color::Yellow,
        GraphColor::Blue => Color::Blue,
        GraphColor::Magenta => Color::Magenta,
        GraphColor::Cyan => Color::Cyan,
        GraphColor::White => Color::White,
        _ => Color::Reset,
    };
    Style::default().fg(fg)
}

/// Detect best charset profile for terminal
fn detect_charset_profile() -> CharsetProfile {
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("256color") || term.contains("truecolor") {
            CharsetProfile::Utf8Rounded
        } else if term.contains("xterm") || term.contains("screen") {
            CharsetProfile::Utf8Straight
        } else {
            CharsetProfile::Ascii
        }
    } else {
        CharsetProfile::Utf8Straight
    }
}
