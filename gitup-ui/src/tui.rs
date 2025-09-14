use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gitup_core::{Repository, CommitInfo, BranchInfo, CommitFileStatus, StatusType};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Application state
pub struct App {
    pub repository: Repository,
    pub current_tab: usize,
    pub commits: Vec<CommitInfo>,
    pub branches: Vec<BranchInfo>,
    pub status_files: Vec<(String, StatusType)>,
    pub selected_commit: ListState,
    pub selected_branch: ListState,
    pub selected_file: ListState,
    pub diff_content: String,
    pub diff_line_count: usize,
    pub scroll_state: ScrollbarState,
    pub scroll_position: u16,
    pub should_quit: bool,
    pub message: Option<(String, Instant)>,
}

impl App {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self> {
        let repository = Repository::open(repo_path)?;
        let commits = repository.get_commits(50)?;
        let branches = repository.list_branches()?;
        let status = repository.get_status()?;

        let status_files: Vec<(String, StatusType)> = status
            .into_iter()
            .map(|f| (f.path, f.status))
            .collect();

        let mut selected_commit = ListState::default();
        selected_commit.select(Some(0));

        let mut selected_branch = ListState::default();
        selected_branch.select(Some(0));

        let mut selected_file = ListState::default();
        if !status_files.is_empty() {
            selected_file.select(Some(0));
        }

        Ok(App {
            repository,
            current_tab: 0,
            commits,
            branches,
            status_files,
            selected_commit,
            selected_branch,
            selected_file,
            diff_content: String::new(),
            diff_line_count: 0,
            scroll_state: ScrollbarState::default(),
            scroll_position: 0,
            should_quit: false,
            message: None,
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.commits = self.repository.get_commits(50)?;
        self.branches = self.repository.list_branches()?;

        let status = self.repository.get_status()?;
        self.status_files = status
            .into_iter()
            .map(|f| (f.path, f.status))
            .collect();

        Ok(())
    }

    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % 4;
    }

    pub fn previous_tab(&mut self) {
        if self.current_tab > 0 {
            self.current_tab -= 1;
        } else {
            self.current_tab = 3;
        }
    }

    pub fn next_item(&mut self) {
        match self.current_tab {
            0 => self.next_commit(),
            1 => self.next_branch(),
            2 => self.next_file(),
            _ => {}
        }
    }

    pub fn previous_item(&mut self) {
        match self.current_tab {
            0 => self.previous_commit(),
            1 => self.previous_branch(),
            2 => self.previous_file(),
            _ => {}
        }
    }

    fn next_commit(&mut self) {
        let i = match self.selected_commit.selected() {
            Some(i) => {
                if i >= self.commits.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.selected_commit.select(Some(i));
        self.load_commit_diff();
    }

    fn previous_commit(&mut self) {
        let i = match self.selected_commit.selected() {
            Some(i) => {
                if i == 0 {
                    self.commits.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selected_commit.select(Some(i));
        self.load_commit_diff();
    }

    fn next_branch(&mut self) {
        let i = match self.selected_branch.selected() {
            Some(i) => {
                if i >= self.branches.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.selected_branch.select(Some(i));
    }

    fn previous_branch(&mut self) {
        let i = match self.selected_branch.selected() {
            Some(i) => {
                if i == 0 {
                    self.branches.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selected_branch.select(Some(i));
    }

    fn next_file(&mut self) {
        if self.status_files.is_empty() {
            return;
        }
        let i = match self.selected_file.selected() {
            Some(i) => {
                if i >= self.status_files.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.selected_file.select(Some(i));
    }

    fn previous_file(&mut self) {
        if self.status_files.is_empty() {
            return;
        }
        let i = match self.selected_file.selected() {
            Some(i) => {
                if i == 0 {
                    self.status_files.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selected_file.select(Some(i));
    }

    fn load_commit_diff(&mut self) {
        if let Some(i) = self.selected_commit.selected() {
            if let Some(commit) = self.commits.get(i) {
                if let Ok(diffs) = self.repository.diff_for_commit(&commit.id) {
                    let mut content = String::new();
                    for diff in diffs {
                        content.push_str(&format!("--- {}\n", diff.file.path));
                        for line in diff.lines {
                            let prefix = match line.origin {
                                gitup_core::LineOrigin::Addition => "+",
                                gitup_core::LineOrigin::Deletion => "-",
                                gitup_core::LineOrigin::Context => " ",
                            };
                            content.push_str(&format!("{}{}", prefix, line.content));
                            if !line.content.ends_with('\n') {
                                content.push('\n');
                            }
                        }
                    }
                    self.diff_content = content;
                    self.diff_line_count = self.diff_content.lines().count();
                    self.scroll_position = 0;
                    self.scroll_state = self.scroll_state
                        .position(0)
                        .content_length(self.diff_line_count);
                }
            }
        }
    }

    pub fn stage_selected_file(&mut self) {
        if let Some(i) = self.selected_file.selected() {
            if let Some((path, _)) = self.status_files.get(i) {
                if let Err(e) = self.repository.stage_file(path) {
                    self.message = Some((format!("Failed to stage: {}", e), Instant::now()));
                } else {
                    self.message = Some((format!("Staged: {}", path), Instant::now()));
                    let _ = self.refresh();
                }
            }
        }
    }

    pub fn unstage_selected_file(&mut self) {
        if let Some(i) = self.selected_file.selected() {
            if let Some((path, _)) = self.status_files.get(i) {
                if let Err(e) = self.repository.unstage_file(path) {
                    self.message = Some((format!("Failed to unstage: {}", e), Instant::now()));
                } else {
                    self.message = Some((format!("Unstaged: {}", path), Instant::now()));
                    let _ = self.refresh();
                }
            }
        }
    }

    pub fn checkout_selected_branch(&mut self) {
        if let Some(i) = self.selected_branch.selected() {
            if let Some(branch) = self.branches.get(i) {
                if !branch.is_remote && !branch.is_head {
                    if let Err(e) = self.repository.checkout_branch(&branch.name) {
                        self.message = Some((format!("Failed to checkout: {}", e), Instant::now()));
                    } else {
                        self.message = Some((format!("Checked out: {}", branch.name), Instant::now()));
                        let _ = self.refresh();
                    }
                }
            }
        }
    }

    pub fn scroll_down(&mut self, amount: u16) {
        if self.current_tab == 3 {  // Only scroll in diff tab
            let max_scroll = self.diff_line_count.saturating_sub(10) as u16; // Keep some lines visible
            self.scroll_position = (self.scroll_position + amount).min(max_scroll);
            self.scroll_state = self.scroll_state
                .position(self.scroll_position as usize)
                .content_length(self.diff_line_count);
        }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        if self.current_tab == 3 {  // Only scroll in diff tab
            self.scroll_position = self.scroll_position.saturating_sub(amount);
            self.scroll_state = self.scroll_state
                .position(self.scroll_position as usize)
                .content_length(self.diff_line_count);
        }
    }

    pub fn scroll_to_top(&mut self) {
        if self.current_tab == 3 {
            self.scroll_position = 0;
            self.scroll_state = self.scroll_state
                .position(0)
                .content_length(self.diff_line_count);
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        if self.current_tab == 3 {
            let max_scroll = self.diff_line_count.saturating_sub(10) as u16;
            self.scroll_position = max_scroll;
            self.scroll_state = self.scroll_state
                .position(max_scroll as usize)
                .content_length(self.diff_line_count);
        }
    }
}

/// Run the TUI application
pub fn run_tui<P: AsRef<Path>>(repo_path: P) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(repo_path)?;
    app.load_commit_diff();

    // Main loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Handle messages timeout
        if let Some((_, time)) = &app.message {
            if time.elapsed() > Duration::from_secs(3) {
                app.message = None;
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.previous_tab(),
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.current_tab == 3 {
                            app.scroll_down(1);
                        } else {
                            app.next_item();
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.current_tab == 3 {
                            app.scroll_up(1);
                        } else {
                            app.previous_item();
                        }
                    }
                    KeyCode::PageDown => app.scroll_down(10),
                    KeyCode::PageUp => app.scroll_up(10),
                    KeyCode::Home => app.scroll_to_top(),
                    KeyCode::End => app.scroll_to_bottom(),
                    KeyCode::Char('r') => {
                        if let Err(e) = app.refresh() {
                            app.message = Some((format!("Refresh failed: {}", e), Instant::now()));
                        }
                    }
                    KeyCode::Char('s') if app.current_tab == 2 => app.stage_selected_file(),
                    KeyCode::Char('u') if app.current_tab == 2 => app.unstage_selected_file(),
                    KeyCode::Enter if app.current_tab == 1 => app.checkout_selected_branch(),
                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tabs
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Status bar
        ])
        .split(size);

    // Draw tabs
    let titles = vec!["Commits", "Branches", "Status", "Diff"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("GitUp"))
        .select(app.current_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Black),
        );
    f.render_widget(tabs, chunks[0]);

    // Draw content based on selected tab
    match app.current_tab {
        0 => draw_commits_tab(f, app, chunks[1]),
        1 => draw_branches_tab(f, app, chunks[1]),
        2 => draw_status_tab(f, app, chunks[1]),
        3 => draw_diff_tab(f, app, chunks[1]),
        _ => {}
    }

    // Draw status bar
    draw_status_bar(f, app, chunks[2]);
}

fn draw_commits_tab(f: &mut Frame, app: &App, area: Rect) {
    let commits: Vec<ListItem> = app
        .commits
        .iter()
        .map(|c| {
            let content = vec![
                Line::from(vec![
                    Span::styled(&c.id[..8], Style::default().fg(Color::Yellow)),
                    Span::raw(" - "),
                    Span::raw(&c.author),
                ]),
                Line::from(Span::raw(&c.message)),
            ];
            ListItem::new(content)
        })
        .collect();

    let commits_list = List::new(commits)
        .block(Block::default().borders(Borders::ALL).title("Commit History"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(commits_list, area, &mut app.selected_commit.clone());
}

fn draw_branches_tab(f: &mut Frame, app: &App, area: Rect) {
    let branches: Vec<ListItem> = app
        .branches
        .iter()
        .map(|b| {
            let style = if b.is_head {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else if b.is_remote {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };

            let prefix = if b.is_head { "* " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(&b.name, style),
            ]))
        })
        .collect();

    let branches_list = List::new(branches)
        .block(Block::default().borders(Borders::ALL).title("Branches"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(branches_list, area, &mut app.selected_branch.clone());
}

fn draw_status_tab(f: &mut Frame, app: &App, area: Rect) {
    let files: Vec<ListItem> = app
        .status_files
        .iter()
        .map(|(path, status)| {
            let (symbol, color) = match status {
                StatusType::New => ("+", Color::Green),
                StatusType::Modified => ("M", Color::Yellow),
                StatusType::Deleted => ("-", Color::Red),
                StatusType::Renamed => ("R", Color::Cyan),
                StatusType::Untracked => ("?", Color::Gray),
                _ => ("?", Color::White),
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", symbol), Style::default().fg(color)),
                Span::raw(path),
            ]))
        })
        .collect();

    let files_list = List::new(files)
        .block(Block::default().borders(Borders::ALL).title("Working Directory"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(files_list, area, &mut app.selected_file.clone());
}

fn draw_diff_tab(f: &mut Frame, app: &App, area: Rect) {
    // Split area for content and scrollbar
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let lines: Vec<Line> = app
        .diff_content
        .lines()
        .map(|line| {
            if line.starts_with('+') && !line.starts_with("+++") {
                Line::from(Span::styled(line, Style::default().fg(Color::Green)))
            } else if line.starts_with('-') && !line.starts_with("---") {
                Line::from(Span::styled(line, Style::default().fg(Color::Red)))
            } else if line.starts_with("@@") {
                Line::from(Span::styled(line, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            } else if line.starts_with("diff --git") {
                Line::from(Span::styled(line, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            } else if line.starts_with("---") || line.starts_with("+++") {
                Line::from(Span::styled(line, Style::default().fg(Color::Blue)))
            } else {
                Line::from(Span::raw(line))
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Diff [{}:{}/{}]",
            if app.diff_line_count > 0 {
                app.scroll_position + 1
            } else {
                0
            },
            app.scroll_position + (chunks[0].height.saturating_sub(2)).min(app.diff_line_count as u16),
            app.diff_line_count
        )))
        .scroll((app.scroll_position, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, chunks[0]);

    // Render scrollbar if content is scrollable
    if app.diff_line_count > chunks[0].height as usize {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        let mut scrollbar_state = app.scroll_state.clone();
        scrollbar_state = scrollbar_state
            .position(app.scroll_position as usize)
            .content_length(app.diff_line_count)
            .viewport_content_length(chunks[0].height.saturating_sub(2) as usize);

        f.render_stateful_widget(
            scrollbar,
            chunks[0].inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.current_tab {
        0 => "↑↓: Navigate | Tab: Next Tab | q: Quit | r: Refresh",
        1 => "↑↓: Navigate | Enter: Checkout | Tab: Next Tab | q: Quit",
        2 => "↑↓: Navigate | s: Stage | u: Unstage | Tab: Next Tab | q: Quit",
        3 => "↑↓/j/k: Scroll | PgUp/PgDn: Page | Home/End: Top/Bottom | Tab: Next Tab | q: Quit",
        _ => "Tab: Switch Tab | q: Quit",
    };

    let text = if let Some((msg, _)) = &app.message {
        format!("{} | {}", msg, help_text)
    } else {
        help_text.to_string()
    };

    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}