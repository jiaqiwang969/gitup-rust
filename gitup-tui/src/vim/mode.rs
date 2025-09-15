use std::fmt;

/// Vim operation modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    /// Normal mode - navigation and commands
    Normal,

    /// Insert mode - text insertion
    Insert,

    /// Visual mode - character selection
    Visual,

    /// Visual line mode - line selection
    VisualLine,

    /// Visual block mode - rectangular selection
    VisualBlock,

    /// Command mode - ex commands
    Command,

    /// Search mode - pattern search
    Search,

    /// Operator-pending mode - waiting for motion
    Operator,
}

impl fmt::Display for VimMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VimMode::Normal => write!(f, "NORMAL"),
            VimMode::Insert => write!(f, "INSERT"),
            VimMode::Visual => write!(f, "VISUAL"),
            VimMode::VisualLine => write!(f, "VISUAL LINE"),
            VimMode::VisualBlock => write!(f, "VISUAL BLOCK"),
            VimMode::Command => write!(f, "COMMAND"),
            VimMode::Search => write!(f, "SEARCH"),
            VimMode::Operator => write!(f, "OPERATOR"),
        }
    }
}

impl VimMode {
    /// Check if mode is a visual mode
    pub fn is_visual(&self) -> bool {
        matches!(
            self,
            VimMode::Visual | VimMode::VisualLine | VimMode::VisualBlock
        )
    }

    /// Check if mode allows text input
    pub fn is_input(&self) -> bool {
        matches!(self, VimMode::Insert | VimMode::Command | VimMode::Search)
    }

    /// Check if mode is waiting for additional input
    pub fn is_pending(&self) -> bool {
        matches!(self, VimMode::Operator)
    }

    /// Get the default mode
    pub fn default() -> Self {
        VimMode::Normal
    }

    /// Transition rules for mode changes
    pub fn can_transition_to(&self, target: VimMode) -> bool {
        match (self, target) {
            // From Normal, can go to any mode
            (VimMode::Normal, _) => true,

            // From Insert, can only go to Normal (via Esc)
            (VimMode::Insert, VimMode::Normal) => true,
            (VimMode::Insert, _) => false,

            // From Visual modes, can go to Normal or other visual modes
            (mode, VimMode::Normal) if mode.is_visual() => true,
            (mode, target) if mode.is_visual() && target.is_visual() => true,
            (mode, _) if mode.is_visual() => false,

            // From Command/Search, can only go to Normal
            (VimMode::Command | VimMode::Search, VimMode::Normal) => true,
            (VimMode::Command | VimMode::Search, _) => false,

            // From Operator, can only go to Normal
            (VimMode::Operator, VimMode::Normal) => true,
            (VimMode::Operator, _) => false,

            _ => false,
        }
    }

    /// Get keybinding hints for the current mode
    pub fn get_hints(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            VimMode::Normal => vec![
                ("h/j/k/l", "Navigate"),
                ("i", "Insert"),
                ("v", "Visual"),
                (":", "Command"),
                ("/", "Search"),
                ("g", "Git ops"),
            ],
            VimMode::Insert => vec![
                ("ESC", "Normal mode"),
                ("Ctrl-o", "One command"),
            ],
            VimMode::Visual => vec![
                ("ESC", "Normal mode"),
                ("d", "Delete"),
                ("y", "Yank"),
                ("c", "Change"),
            ],
            VimMode::VisualLine => vec![
                ("ESC", "Normal mode"),
                ("d", "Delete lines"),
                ("y", "Yank lines"),
                ("c", "Change lines"),
            ],
            VimMode::VisualBlock => vec![
                ("ESC", "Normal mode"),
                ("d", "Delete block"),
                ("y", "Yank block"),
                ("I", "Insert block"),
            ],
            VimMode::Command => vec![
                ("ESC", "Cancel"),
                ("Enter", "Execute"),
                ("Tab", "Complete"),
            ],
            VimMode::Search => vec![
                ("ESC", "Cancel"),
                ("Enter", "Search"),
                ("n/N", "Next/Prev"),
            ],
            VimMode::Operator => vec![
                ("ESC", "Cancel"),
                ("Motion", "Apply"),
            ],
        }
    }
}