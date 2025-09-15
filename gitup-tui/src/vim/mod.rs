// Vim-style modal system for GitUp Terminal UI

pub mod state;
pub mod mode;
pub mod motion;
pub mod operator;
pub mod register;
pub mod marks;
pub mod macros;
pub mod text_objects;
pub mod commands;
pub mod config;

pub use state::VimState;
pub use mode::VimMode;
pub use motion::{Motion, MotionType};
pub use operator::Operator;
pub use register::{RegisterManager, RegisterContent};
pub use marks::MarkManager;
pub use macros::MacroRecorder;
pub use text_objects::GitTextObject;
pub use commands::CommandPalette;
pub use config::VimConfig;

use crossterm::event::KeyEvent;
use anyhow::Result;

/// Main Vim handler that processes all keyboard input
pub struct VimHandler {
    state: VimState,
    registers: RegisterManager,
    marks: MarkManager,
    macros: MacroRecorder,
    commands: CommandPalette,
    config: VimConfig,
}

impl VimHandler {
    pub fn new() -> Self {
        Self {
            state: VimState::new(),
            registers: RegisterManager::new(),
            marks: MarkManager::new(),
            macros: MacroRecorder::new(),
            commands: CommandPalette::new(),
            config: VimConfig::default(),
        }
    }

    /// Process a key event and return the resulting action
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<VimAction> {
        // Record key for macros if recording
        if self.macros.is_recording() {
            self.macros.record_key(key);
        }

        // Handle based on current mode
        let action = match self.state.mode() {
            VimMode::Normal => self.handle_normal_mode(key)?,
            VimMode::Insert => self.handle_insert_mode(key)?,
            VimMode::Visual | VimMode::VisualLine | VimMode::VisualBlock => {
                self.handle_visual_mode(key)?
            }
            VimMode::Command => self.handle_command_mode(key)?,
            VimMode::Search => self.handle_search_mode(key)?,
            VimMode::Operator => self.handle_operator_pending(key)?,
        };

        Ok(action)
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<VimAction> {
        self.state.handle_normal_key(key, &mut self.registers, &mut self.marks)
    }

    fn handle_insert_mode(&mut self, key: KeyEvent) -> Result<VimAction> {
        self.state.handle_insert_key(key)
    }

    fn handle_visual_mode(&mut self, key: KeyEvent) -> Result<VimAction> {
        self.state.handle_visual_key(key, &mut self.registers)
    }

    fn handle_command_mode(&mut self, key: KeyEvent) -> Result<VimAction> {
        self.state.handle_command_key(key, &mut self.commands)
    }

    fn handle_search_mode(&mut self, key: KeyEvent) -> Result<VimAction> {
        self.state.handle_search_key(key)
    }

    fn handle_operator_pending(&mut self, key: KeyEvent) -> Result<VimAction> {
        self.state.handle_operator_key(key)
    }

    /// Get the current mode for display
    pub fn current_mode(&self) -> VimMode {
        self.state.mode()
    }

    /// Get the mode line string for status bar
    pub fn mode_line(&self) -> String {
        self.state.mode_line()
    }

    /// Load configuration from file
    pub fn load_config(&mut self, path: &std::path::Path) -> Result<()> {
        self.config = VimConfig::load_from_file(path)?;
        Ok(())
    }
}

/// Actions that can result from Vim input handling
#[derive(Debug, Clone)]
pub enum VimAction {
    /// No action needed
    None,

    /// Movement actions
    Move(Motion),

    /// Git operations
    GitOp(GitOperation),

    /// Text insertion
    Insert(String),

    /// Mode change
    ModeChange(VimMode),

    /// Command execution
    ExecuteCommand(String),

    /// Search
    Search(SearchDirection, String),

    /// Visual selection changed
    SelectionChanged(SelectionRange),

    /// Quit application
    Quit,
}

#[derive(Debug, Clone)]
pub enum GitOperation {
    Checkout(String),
    CreateBranch(String),
    Merge(String),
    Rebase(String),
    CherryPick(Vec<String>),
    Reset(ResetMode, String),
    Stash(StashOp),
    Tag(String),
}

#[derive(Debug, Clone)]
pub enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

#[derive(Debug, Clone)]
pub enum StashOp {
    Save(Option<String>),
    Pop,
    Apply(Option<usize>),
    List,
}

#[derive(Debug, Clone)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone)]
pub struct SelectionRange {
    pub start: Position,
    pub end: Position,
    pub mode: SelectionMode,
}

#[derive(Debug, Clone, Copy)]
pub enum SelectionMode {
    Character,
    Line,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}