use std::fmt;

/// Vim operators that can be combined with motions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    // Standard Vim operators
    Delete,
    Yank,
    Change,
    Indent,
    Outdent,
    Format,
    SwapCase,
    Uppercase,
    Lowercase,

    // Git-specific operators
    CherryPick,
    Revert,
    Reset,
    Squash,
    Fixup,
    Drop,
    Reword,
    Edit,

    // Special operators
    Mark,
    JumpToMark,
    Comment,
    Uncomment,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Delete => write!(f, "DELETE"),
            Operator::Yank => write!(f, "YANK"),
            Operator::Change => write!(f, "CHANGE"),
            Operator::Indent => write!(f, "INDENT"),
            Operator::Outdent => write!(f, "OUTDENT"),
            Operator::Format => write!(f, "FORMAT"),
            Operator::SwapCase => write!(f, "SWAP CASE"),
            Operator::Uppercase => write!(f, "UPPERCASE"),
            Operator::Lowercase => write!(f, "LOWERCASE"),

            Operator::CherryPick => write!(f, "CHERRY-PICK"),
            Operator::Revert => write!(f, "REVERT"),
            Operator::Reset => write!(f, "RESET"),
            Operator::Squash => write!(f, "SQUASH"),
            Operator::Fixup => write!(f, "FIXUP"),
            Operator::Drop => write!(f, "DROP"),
            Operator::Reword => write!(f, "REWORD"),
            Operator::Edit => write!(f, "EDIT"),

            Operator::Mark => write!(f, "MARK"),
            Operator::JumpToMark => write!(f, "JUMP TO MARK"),
            Operator::Comment => write!(f, "COMMENT"),
            Operator::Uncomment => write!(f, "UNCOMMENT"),
        }
    }
}

impl Operator {
    /// Check if this operator modifies content
    pub fn is_modifying(&self) -> bool {
        !matches!(self, Operator::Yank | Operator::Mark | Operator::JumpToMark)
    }

    /// Check if this operator requires entering insert mode after
    pub fn enters_insert_mode(&self) -> bool {
        matches!(self, Operator::Change | Operator::Reword)
    }

    /// Check if this is a Git-specific operator
    pub fn is_git_operator(&self) -> bool {
        matches!(
            self,
            Operator::CherryPick | Operator::Revert | Operator::Reset |
            Operator::Squash | Operator::Fixup | Operator::Drop |
            Operator::Reword | Operator::Edit
        )
    }

    /// Get the default register for this operator
    pub fn default_register(&self) -> char {
        match self {
            Operator::Delete => '"', // Default register
            Operator::Yank => '0',   // Yank register
            _ => '"',
        }
    }

    /// Check if operator can be repeated with dot command
    pub fn is_repeatable(&self) -> bool {
        self.is_modifying()
    }

    /// Get keybinding hints for this operator
    pub fn get_hints(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Operator::Delete | Operator::Yank | Operator::Change => vec![
                ("w", "Word"),
                ("$", "To line end"),
                ("0", "To line start"),
                ("gg", "To file start"),
                ("G", "To file end"),
                ("}", "Paragraph"),
                ("i{", "Inner block"),
                ("a{", "Around block"),
            ],

            Operator::CherryPick => vec![
                ("j", "Next commit"),
                ("k", "Previous commit"),
                ("V", "Select range"),
            ],

            Operator::Mark => vec![
                ("a-z", "Local mark"),
                ("A-Z", "Global mark"),
            ],

            Operator::JumpToMark => vec![
                ("a-z", "Jump to local"),
                ("A-Z", "Jump to global"),
                ("'", "Last position"),
            ],

            _ => vec![],
        }
    }
}