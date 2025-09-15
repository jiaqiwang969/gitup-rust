use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gitup_core::{Repository, CommitInfo, BranchInfo, CommitFileStatus, StatusType};
use crate::simple_graph::{SimpleGraph, SimpleGraphWidget};
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

/// Vim mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    Command,
    Search,
}

impl VimMode {
    pub fn to_string(&self) -> &str {
        match self {
            VimMode::Normal => "NORMAL",
            VimMode::Insert => "INSERT",
            VimMode::Visual => "VISUAL",
            VimMode::Command => "COMMAND",
            VimMode::Search => "SEARCH",
        }
    }
}

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

    // Graph visualization
    pub show_graph: bool,
    pub simple_graph: SimpleGraph,

    // Vim mode support
    pub vim_mode: VimMode,
    pub command_buffer: String,
    pub search_buffer: String,
    pub visual_start: Option<usize>,
    pub count: Option<usize>,
    // Navigation context
    pub viewing_commit: Option<String>,  // Currently viewing commit's files
    pub previous_tab: Option<usize>,     // For Esc navigation
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

            // Graph visualization
            show_graph: false,
            simple_graph: SimpleGraph::new(),

            // Initialize Vim mode
            vim_mode: VimMode::Normal,
            command_buffer: String::new(),
            search_buffer: String::new(),
            visual_start: None,
            count: None,
            // Navigation context
            viewing_commit: None,
            previous_tab: None,
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
        // Don't auto-load diff when just navigating
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
        // Don't auto-load diff when just navigating
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

    fn load_commit_files(&mut self) {
        // Load files changed in the selected commit
        if let Some(i) = self.selected_commit.selected() {
            if let Some(commit) = self.commits.get(i) {
                self.viewing_commit = Some(commit.id.clone());

                // Get files changed in this commit
                if let Ok(diffs) = self.repository.diff_for_commit(&commit.id) {
                    self.status_files = diffs.into_iter().map(|diff| {
                        let status = match diff.file.status {
                            gitup_core::FileStatus::Added => StatusType::New,
                            gitup_core::FileStatus::Modified => StatusType::Modified,
                            gitup_core::FileStatus::Deleted => StatusType::Deleted,
                            gitup_core::FileStatus::Renamed => StatusType::Renamed,
                            _ => StatusType::Modified,
                        };
                        (diff.file.path, status)
                    }).collect();

                    // Reset file selection
                    self.selected_file = ListState::default();
                    if !self.status_files.is_empty() {
                        self.selected_file.select(Some(0));
                    }
                }
            }
        }
    }

    fn load_commit_file_diff(&mut self, file_path: &str) {
        // Load diff for a specific file in the viewing commit
        if let Some(commit_id) = &self.viewing_commit {
            if let Ok(diffs) = self.repository.diff_for_commit(commit_id) {
                // Find the specific file's diff
                for diff in diffs {
                    if diff.file.path == file_path {
                        let mut content = String::new();
                        content.push_str(&format!("Commit: {}\n", &commit_id[..8.min(commit_id.len())]));
                        content.push_str(&format!("File: {}\n", file_path));
                        content.push_str(&format!("diff --git a/{} b/{}\n", file_path, file_path));
                        content.push_str(&format!("--- a/{}\n", file_path));
                        content.push_str(&format!("+++ b/{}\n", file_path));

                        // Add hunks
                        for hunk in &diff.hunks {
                            content.push_str(&hunk.header);
                        }

                        // Add lines
                        for line in &diff.lines {
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

                        self.diff_content = content;
                        self.diff_line_count = self.diff_content.lines().count();
                        self.scroll_position = 0;
                        self.scroll_state = self.scroll_state
                            .position(0)
                            .content_length(self.diff_line_count);
                        return;
                    }
                }
            }
        }
    }

    fn load_file_diff(&mut self, file_path: &str) {
        // Try to get diff based on file status
        let mut diff_result = None;
        let mut is_staged = false;
        let mut is_new_file = false;

        // Check file status
        if let Some(i) = self.selected_file.selected() {
            if let Some((_, status)) = self.status_files.get(i) {
                match status {
                    StatusType::New => {
                        // New staged file - try to get staged diff
                        diff_result = self.repository.diff_staged_file(file_path).ok();
                        is_staged = true;
                        is_new_file = true;
                    }
                    StatusType::Modified => {
                        // Modified file - get working directory diff first
                        diff_result = self.repository.diff_file(file_path).ok();
                    }
                    _ => {
                        // Try generic diff
                        diff_result = self.repository.diff_file(file_path).ok();
                    }
                }
            }
        }

        // If no diff yet, try working directory diff
        if diff_result.is_none() && !is_staged {
            diff_result = self.repository.diff_file(file_path).ok();
        }

        match diff_result {
            Some(diff) => {
                let mut content = String::new();

                // Add header
                if is_new_file {
                    content.push_str(&format!("New file (staged): {}\n", file_path));
                } else if is_staged {
                    content.push_str(&format!("Staged changes for: {}\n", file_path));
                } else {
                    content.push_str(&format!("Working directory changes for: {}\n", file_path));
                }

                content.push_str(&format!("diff --git a/{} b/{}\n", file_path, file_path));

                if is_new_file {
                    content.push_str("--- /dev/null\n");
                    content.push_str(&format!("+++ b/{}\n", file_path));
                } else {
                    content.push_str(&format!("--- a/{}\n", file_path));
                    content.push_str(&format!("+++ b/{}\n", file_path));
                }

                // Add hunks
                for hunk in &diff.hunks {
                    content.push_str(&hunk.header);
                }

                // Add lines - for new files, all lines should be additions
                for line in &diff.lines {
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

                if diff.lines.is_empty() && !diff.binary {
                    content.push_str("\n(Empty file)\n");
                } else if diff.binary {
                    content.push_str("\n(Binary file)\n");
                }

                self.diff_content = content;
                self.diff_line_count = self.diff_content.lines().count();
                self.scroll_position = 0;
                self.scroll_state = self.scroll_state
                    .position(0)
                    .content_length(self.diff_line_count);
            }
            None => {
                // No diff available, show file status info
                let mut content = String::new();
                content.push_str(&format!("File: {}\n\n", file_path));

                if let Some(i) = self.selected_file.selected() {
                    if let Some((_, status)) = self.status_files.get(i) {
                        match status {
                            StatusType::Untracked => {
                                content.push_str("Status: Untracked (new file)\n\n");
                                content.push_str("This file is not yet tracked by Git.\n");
                                content.push_str("Use 's' to stage this file.\n\n");
                                content.push_str("Once staged, you'll be able to see the content here.\n");
                            }
                            StatusType::New => {
                                content.push_str("Status: New file (staged)\n\n");
                                content.push_str("Unable to load diff. The file might be binary or empty.\n");
                                content.push_str("Use 'u' to unstage this file.\n");
                            }
                            StatusType::Modified => {
                                content.push_str("Status: Modified\n\n");
                                content.push_str("Unable to load diff.\n");
                            }
                            _ => {
                                content.push_str(&format!("Status: {:?}\n\n", status));
                                content.push_str("No diff information available.\n");
                            }
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
    // Don't auto-load diff on startup - let user decide what to view

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
                // Only handle key press events to avoid repeats/releases triggering twice
                if key.kind == KeyEventKind::Press {
                    handle_key_event(app, key);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_key_event(app: &mut App, key: crossterm::event::KeyEvent) {
    match app.vim_mode {
        VimMode::Normal => handle_normal_mode(app, key),
        VimMode::Insert => handle_insert_mode(app, key),
        VimMode::Visual => handle_visual_mode(app, key),
        VimMode::Command => handle_command_mode(app, key),
        VimMode::Search => handle_search_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: crossterm::event::KeyEvent) {
    // Handle count prefix (1-9)
    if let KeyCode::Char(c) = key.code {
        if c.is_ascii_digit() && c != '0' {
            let digit = c.to_digit(10).unwrap() as usize;
            app.count = Some(app.count.unwrap_or(0) * 10 + digit);
            return;
        }
    }

    let count = app.count.unwrap_or(1);

    match key.code {
        // Vim navigation
        KeyCode::Char('h') | KeyCode::Left => {
            if app.current_tab > 0 {
                app.current_tab -= 1;
            } else {
                app.current_tab = 3;
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.current_tab = (app.current_tab + 1) % 4;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            for _ in 0..count {
                if app.current_tab == 3 {
                    app.scroll_down(1);
                } else {
                    app.next_item();
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            for _ in 0..count {
                if app.current_tab == 3 {
                    app.scroll_up(1);
                } else {
                    app.previous_item();
                }
            }
        }

        // Toggle graph visualization
        KeyCode::Char('v') if app.current_tab == 0 => {
            app.show_graph = !app.show_graph;
            app.message = Some((
                format!("Graph view: {}", if app.show_graph { "ON" } else { "OFF" }),
                Instant::now()
            ));
        }

        // Quick navigation
        KeyCode::Char('g') if app.count.is_none() => {
            // gg - go to top
            match app.current_tab {
                0 => app.selected_commit.select(Some(0)),
                1 => app.selected_branch.select(Some(0)),
                2 => app.selected_file.select(Some(0)),
                3 => app.scroll_to_top(),
                _ => {}
            }
        }
        KeyCode::Char('G') => {
            // G - go to bottom or line N
            if let Some(n) = app.count {
                match app.current_tab {
                    0 => app.selected_commit.select(Some((n - 1).min(app.commits.len() - 1))),
                    1 => app.selected_branch.select(Some((n - 1).min(app.branches.len() - 1))),
                    2 => app.selected_file.select(Some((n - 1).min(app.status_files.len() - 1))),
                    _ => {}
                }
            } else {
                match app.current_tab {
                    0 => app.selected_commit.select(Some(app.commits.len().saturating_sub(1))),
                    1 => app.selected_branch.select(Some(app.branches.len().saturating_sub(1))),
                    2 => app.selected_file.select(Some(app.status_files.len().saturating_sub(1))),
                    3 => app.scroll_to_bottom(),
                    _ => {}
                }
            }
        }

        // Tab navigation with numbers
        KeyCode::Char('1') => app.current_tab = 0,
        KeyCode::Char('2') => app.current_tab = 1,
        KeyCode::Char('3') => app.current_tab = 2,
        KeyCode::Char('4') => app.current_tab = 3,

        // Additional navigation shortcuts
        KeyCode::Char('H') => app.current_tab = 0,  // Shift+H to first tab
        KeyCode::Char('L') => app.current_tab = 3,  // Shift+L to last tab
        KeyCode::Char('M') => {
            // M - go to middle
            match app.current_tab {
                0 => app.selected_commit.select(Some(app.commits.len() / 2)),
                1 => app.selected_branch.select(Some(app.branches.len() / 2)),
                2 => app.selected_file.select(Some(app.status_files.len() / 2)),
                _ => {}
            }
        }

        // Quick actions
        KeyCode::Char('o') => {
            // Open/view diff (similar to old Enter behavior)
            match app.current_tab {
                0 => {
                    app.load_commit_diff();
                    // Stay in current tab
                }
                _ => {}
            }
        }
        KeyCode::Char('O') => {
            // Open diff in new tab (load diff and switch)
            match app.current_tab {
                0 => {
                    app.load_commit_diff();
                    app.previous_tab = Some(0);
                    app.current_tab = 3;
                }
                _ => {}
            }
        }
        KeyCode::Char('x') if app.current_tab == 2 => {
            // TODO: Implement discard changes
            app.message = Some(("Discard changes not yet implemented".to_string(), Instant::now()));
        }
        KeyCode::Char('a') if app.current_tab == 2 => {
            // Stage all files
            let files = app.status_files.clone();
            let mut staged_count = 0;
            for (path, _) in &files {
                if app.repository.stage_file(path).is_ok() {
                    staged_count += 1;
                }
            }
            app.message = Some((format!("Staged {} files", staged_count), Instant::now()));
            let _ = app.refresh();
        }
        KeyCode::Char('A') if app.current_tab == 2 => {
            // Unstage all files - unstage one by one
            let files = app.status_files.clone();
            let mut unstaged_count = 0;
            for (path, _) in &files {
                if app.repository.unstage_file(path).is_ok() {
                    unstaged_count += 1;
                }
            }
            app.message = Some((format!("Unstaged {} files", unstaged_count), Instant::now()));
            let _ = app.refresh();
        }

        // Mode switches
        KeyCode::Char('i') => app.vim_mode = VimMode::Insert,
        KeyCode::Char('v') => {
            app.vim_mode = VimMode::Visual;
            app.visual_start = match app.current_tab {
                0 => app.selected_commit.selected(),
                1 => app.selected_branch.selected(),
                2 => app.selected_file.selected(),
                _ => None,
            };
        }
        KeyCode::Char(':') => {
            app.vim_mode = VimMode::Command;
            app.command_buffer.clear();
        }
        KeyCode::Char('/') => {
            app.vim_mode = VimMode::Search;
            app.search_buffer.clear();
        }

        // Git operations
        KeyCode::Char('s') if app.current_tab == 2 => app.stage_selected_file(),
        KeyCode::Char('u') if app.current_tab == 2 => app.unstage_selected_file(),
        KeyCode::Char('c') if app.current_tab == 1 => app.checkout_selected_branch(),
        KeyCode::Enter => {
            match app.current_tab {
                0 => {
                    // Commits tab: Load commit's files and switch to Status tab
                    app.load_commit_files();
                    app.previous_tab = Some(0);  // Remember we came from Commits
                    app.current_tab = 2;  // Switch to Status tab
                }
                1 => app.checkout_selected_branch(),
                2 => {
                    // Status tab: Load file diff and switch to Diff tab
                    if let Some(i) = app.selected_file.selected() {
                        if let Some((path, status)) = app.status_files.get(i).cloned() {
                            // Check if we're viewing a commit's files
                            if app.viewing_commit.is_some() {
                                // Load diff from the commit context
                                app.load_commit_file_diff(&path);
                                app.previous_tab = Some(2);  // Remember we came from Status
                                app.current_tab = 3;
                            } else {
                                // Load working directory diff
                                match status {
                                    StatusType::Modified | StatusType::New | StatusType::Deleted => {
                                        app.load_file_diff(&path);
                                        app.previous_tab = Some(2);  // Remember we came from Status
                                        app.current_tab = 3;
                                    }
                                    StatusType::Untracked => {
                                        app.load_file_diff(&path);
                                        app.previous_tab = Some(2);  // Remember we came from Status
                                        app.current_tab = 3;
                                    }
                                    _ => {
                                        app.stage_selected_file();
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Other commands
        KeyCode::Char('r') => {
            if let Err(e) = app.refresh() {
                app.message = Some((format!("Refresh failed: {}", e), Instant::now()));
            }
        }
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => {
            // Navigation chain: Diff -> Status -> Commits
            match app.current_tab {
                3 => {
                    // Diff tab: Go back to previous tab (usually Status)
                    if let Some(prev) = app.previous_tab {
                        app.current_tab = prev;
                        app.previous_tab = None;  // Clear after use
                    } else {
                        app.current_tab = 2;  // Default to Status
                    }
                }
                2 => {
                    // Status tab: Go back to Commits and clear viewing context
                    app.viewing_commit = None;
                    app.current_tab = 0;
                    // Reload working directory status
                    let _ = app.refresh();
                }
                _ => {
                    // Other tabs: just clear count and message
                }
            }
            // Clear count and message
            app.count = None;
            app.message = None;
        }

        // Page navigation
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_down(10);
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_up(10);
        }
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_down(20);
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_up(20);
        }

        _ => {}
    }

    // Clear count after command
    if !matches!(key.code, KeyCode::Char(c) if c.is_ascii_digit() && c != '0') {
        app.count = None;
    }
}

fn handle_insert_mode(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.vim_mode = VimMode::Normal;
        }
        // In insert mode, allow normal operations
        _ => handle_normal_mode(app, key),
    }
}

fn handle_visual_mode(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.vim_mode = VimMode::Normal;
            app.visual_start = None;
        }
        // Allow navigation in visual mode
        KeyCode::Char('j') | KeyCode::Down => app.next_item(),
        KeyCode::Char('k') | KeyCode::Up => app.previous_item(),

        // Batch operations in visual mode
        KeyCode::Char('s') if app.current_tab == 2 => {
            // Stage all selected files
            if let (Some(start), Some(end)) = (app.visual_start, app.selected_file.selected()) {
                let (start, end) = if start <= end { (start, end) } else { (end, start) };
                let files_to_stage: Vec<String> = app.status_files[start..=end]
                    .iter()
                    .map(|(path, _)| path.clone())
                    .collect();

                for path in files_to_stage {
                    let _ = app.repository.stage_file(&path);
                }
                app.message = Some((format!("Staged {} files", end - start + 1), Instant::now()));
                let _ = app.refresh();
            }
            app.vim_mode = VimMode::Normal;
            app.visual_start = None;
        }
        _ => {}
    }
}

fn handle_command_mode(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.vim_mode = VimMode::Normal;
            app.command_buffer.clear();
        }
        KeyCode::Enter => {
            execute_command(app);
            app.vim_mode = VimMode::Normal;
            app.command_buffer.clear();
        }
        KeyCode::Backspace => {
            app.command_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.command_buffer.push(c);
        }
        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.vim_mode = VimMode::Normal;
            app.search_buffer.clear();
        }
        KeyCode::Enter => {
            search_items(app);
            app.vim_mode = VimMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.search_buffer.push(c);
        }
        _ => {}
    }
}

fn execute_command(app: &mut App) {
    let parts: Vec<&str> = app.command_buffer.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "q" | "quit" => app.should_quit = true,
        "w" | "write" => {
            // Commit staged changes
            if parts.len() > 1 {
                let message = parts[1..].join(" ");
                // Use default author info for now
                if let Err(e) = app.repository.commit(&message, "GitUp User", "user@gitup.app") {
                    app.message = Some((format!("Commit failed: {}", e), Instant::now()));
                } else {
                    app.message = Some((format!("Committed: {}", message), Instant::now()));
                    let _ = app.refresh();
                }
            }
        }
        "wq" => {
            execute_command(app);
            app.should_quit = true;
        }
        "e" | "edit" => {
            if let Err(e) = app.refresh() {
                app.message = Some((format!("Refresh failed: {}", e), Instant::now()));
            }
        }
        "branch" if parts.len() > 1 => {
            // Create new branch at current HEAD
            if let Err(e) = app.repository.create_branch(parts[1], None) {
                app.message = Some((format!("Failed to create branch: {}", e), Instant::now()));
            } else {
                app.message = Some((format!("Created branch: {}", parts[1]), Instant::now()));
                let _ = app.refresh();
            }
        }
        "checkout" | "co" if parts.len() > 1 => {
            // Checkout branch
            if let Err(e) = app.repository.checkout_branch(parts[1]) {
                app.message = Some((format!("Checkout failed: {}", e), Instant::now()));
            } else {
                app.message = Some((format!("Checked out: {}", parts[1]), Instant::now()));
                let _ = app.refresh();
            }
        }
        _ => {
            app.message = Some((format!("Unknown command: {}", parts[0]), Instant::now()));
        }
    }
}

fn search_items(app: &mut App) {
    if app.search_buffer.is_empty() {
        return;
    }

    let search = app.search_buffer.to_lowercase();

    match app.current_tab {
        0 => {
            // Search commits
            for (i, commit) in app.commits.iter().enumerate() {
                if commit.message.to_lowercase().contains(&search) ||
                   commit.author.to_lowercase().contains(&search) {
                    app.selected_commit.select(Some(i));
                    app.load_commit_diff();
                    break;
                }
            }
        }
        1 => {
            // Search branches
            for (i, branch) in app.branches.iter().enumerate() {
                if branch.name.to_lowercase().contains(&search) {
                    app.selected_branch.select(Some(i));
                    break;
                }
            }
        }
        2 => {
            // Search files
            for (i, (path, _)) in app.status_files.iter().enumerate() {
                if path.to_lowercase().contains(&search) {
                    app.selected_file.select(Some(i));
                    break;
                }
            }
        }
        _ => {}
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
    if app.show_graph {
        // Use graph visualization
        let widget = SimpleGraphWidget::new(
            &app.simple_graph,
            &app.commits,
            &app.branches,
        ).selected(app.selected_commit.selected());
        f.render_widget(widget, area);
    } else {
        // Use traditional list view
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

    // Show different title based on whether we're viewing commit files or working directory
    let title = if let Some(commit_id) = &app.viewing_commit {
        format!("Commit Files [{}]", &commit_id[..8.min(commit_id.len())])
    } else {
        "Working Directory".to_string()
    };

    let files_list = List::new(files)
        .block(Block::default().borders(Borders::ALL).title(title))
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
    // Split status bar into mode indicator and help text
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12),  // Mode indicator (smaller)
            Constraint::Min(0),       // Command/Search/Help text
        ])
        .split(area);

    // Draw mode indicator - simple text style
    let mode_text = if let Some(count) = app.count {
        format!("[{} {}]", app.vim_mode.to_string(), count)
    } else {
        format!("[{}]", app.vim_mode.to_string())
    };

    let mode_color = match app.vim_mode {
        VimMode::Normal => Color::Cyan,
        VimMode::Insert => Color::Green,
        VimMode::Visual => Color::Yellow,
        VimMode::Command => Color::Magenta,
        VimMode::Search => Color::Blue,
    };

    let mode_widget = Paragraph::new(mode_text)
        .style(Style::default().fg(mode_color))
        .block(Block::default().borders(Borders::TOP));

    f.render_widget(mode_widget, chunks[0]);

    // Draw command/search buffer or help text
    let text = match app.vim_mode {
        VimMode::Command => format!(":{}", app.command_buffer),
        VimMode::Search => format!("/{}", app.search_buffer),
        _ => {
            if let Some((msg, _)) = &app.message {
                msg.clone()
            } else {
                // Vim-style help text
                match app.current_tab {
                    0 => "j/k: ↑↓ | Enter: view files | /: search | :: command | q: quit",
                    1 => "j/k: ↑↓ | c/Enter: checkout | b: create branch | /: search",
                    2 => {
                        if app.viewing_commit.is_some() {
                            "j/k: ↑↓ | Enter: view diff | Esc: back to commits"
                        } else {
                            "j/k: ↑↓ | s: stage | u: unstage | Enter: view diff | v: visual"
                        }
                    },
                    3 => "j/k: scroll | gg/G: top/bottom | Ctrl-d/u: page | Esc: back | q: quit",
                    _ => "h/j/k/l: navigate | :: command | /: search | q: quit",
                }.to_string()
            }
        }
    };

    let text_widget = Paragraph::new(text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::TOP))
        .alignment(Alignment::Left);

    f.render_widget(text_widget, chunks[1]);
}
