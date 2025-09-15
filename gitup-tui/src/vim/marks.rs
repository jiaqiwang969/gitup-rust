use std::collections::{HashMap, VecDeque};
use super::Position;

/// Marks for quick navigation
pub struct MarkManager {
    // Local marks (a-z) - specific to current view/buffer
    local_marks: HashMap<char, Mark>,

    // Global marks (A-Z) - across all views
    global_marks: HashMap<char, GlobalMark>,

    // Special marks
    special_marks: HashMap<char, Mark>,

    // Jump list for Ctrl-O/Ctrl-I navigation
    jump_list: VecDeque<JumpEntry>,
    jump_index: usize,

    // Change list for g; and g, navigation
    change_list: VecDeque<Position>,
    change_index: usize,
}

#[derive(Debug, Clone)]
pub struct Mark {
    pub position: Position,
    pub commit_sha: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GlobalMark {
    pub view: String,
    pub position: Position,
    pub commit_sha: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JumpEntry {
    pub position: Position,
    pub view: String,
    pub timestamp: std::time::Instant,
}

impl MarkManager {
    pub fn new() -> Self {
        let mut special_marks = HashMap::new();

        // Initialize special marks
        // ' - Position before last jump
        // ` - Position before last jump (exact column)
        // [ - Start of last change/yank
        // ] - End of last change/yank
        // < - Start of last visual selection
        // > - End of last visual selection
        // . - Position of last change
        // ^ - Position of last insert

        Self {
            local_marks: HashMap::new(),
            global_marks: HashMap::new(),
            special_marks,
            jump_list: VecDeque::with_capacity(100),
            jump_index: 0,
            change_list: VecDeque::with_capacity(100),
            change_index: 0,
        }
    }

    /// Set a local mark (a-z)
    pub fn set_local_mark(&mut self, mark: char, position: Position, commit_sha: Option<String>) {
        if mark.is_ascii_lowercase() {
            self.local_marks.insert(mark, Mark { position, commit_sha });
        }
    }

    /// Set a global mark (A-Z)
    pub fn set_global_mark(&mut self, mark: char, view: String, position: Position, commit_sha: Option<String>) {
        if mark.is_ascii_uppercase() {
            self.global_marks.insert(mark, GlobalMark { view, position, commit_sha });
        }
    }

    /// Set a special mark
    pub fn set_special_mark(&mut self, mark: char, position: Position, commit_sha: Option<String>) {
        self.special_marks.insert(mark, Mark { position, commit_sha });
    }

    /// Get a local mark position
    pub fn get_local_mark(&self, mark: char) -> Option<&Mark> {
        self.local_marks.get(&mark)
    }

    /// Get a global mark
    pub fn get_global_mark(&self, mark: char) -> Option<&GlobalMark> {
        self.global_marks.get(&mark)
    }

    /// Get a special mark position
    pub fn get_special_mark(&self, mark: char) -> Option<&Mark> {
        self.special_marks.get(&mark)
    }

    /// Jump to a mark and return the position
    pub fn jump_to_mark(&mut self, mark: char, current_pos: Position, current_view: &str) -> Option<Position> {
        // Add current position to jump list before jumping
        self.add_jump(current_pos, current_view);

        match mark {
            'a'..='z' => self.local_marks.get(&mark).map(|m| m.position),
            'A'..='Z' => self.global_marks.get(&mark).map(|m| m.position),
            '\'' | '`' => self.special_marks.get(&mark).map(|m| m.position),
            _ => None,
        }
    }

    /// Add a position to the jump list
    pub fn add_jump(&mut self, position: Position, view: &str) {
        // Truncate jump list if we're not at the end
        if self.jump_index < self.jump_list.len() {
            self.jump_list.truncate(self.jump_index);
        }

        // Add new jump
        self.jump_list.push_back(JumpEntry {
            position,
            view: view.to_string(),
            timestamp: std::time::Instant::now(),
        });

        // Limit jump list size
        if self.jump_list.len() > 100 {
            self.jump_list.pop_front();
        } else {
            self.jump_index = self.jump_list.len();
        }
    }

    /// Jump backward in jump list (Ctrl-O)
    pub fn jump_backward(&mut self) -> Option<&JumpEntry> {
        if self.jump_index > 0 {
            self.jump_index -= 1;
            self.jump_list.get(self.jump_index)
        } else {
            None
        }
    }

    /// Jump forward in jump list (Ctrl-I)
    pub fn jump_forward(&mut self) -> Option<&JumpEntry> {
        if self.jump_index < self.jump_list.len() - 1 {
            self.jump_index += 1;
            self.jump_list.get(self.jump_index)
        } else {
            None
        }
    }

    /// Add a position to the change list
    pub fn add_change(&mut self, position: Position) {
        // Truncate change list if we're not at the end
        if self.change_index < self.change_list.len() {
            self.change_list.truncate(self.change_index);
        }

        // Add new change
        self.change_list.push_back(position);

        // Limit change list size
        if self.change_list.len() > 100 {
            self.change_list.pop_front();
        } else {
            self.change_index = self.change_list.len();
        }

        // Also update the '.' special mark
        self.set_special_mark('.', position, None);
    }

    /// Jump to previous change (g;)
    pub fn prev_change(&mut self) -> Option<Position> {
        if self.change_index > 0 {
            self.change_index -= 1;
            self.change_list.get(self.change_index).copied()
        } else {
            None
        }
    }

    /// Jump to next change (g,)
    pub fn next_change(&mut self) -> Option<Position> {
        if self.change_index < self.change_list.len() - 1 {
            self.change_index += 1;
            self.change_list.get(self.change_index).copied()
        } else {
            None
        }
    }

    /// Clear all local marks
    pub fn clear_local_marks(&mut self) {
        self.local_marks.clear();
    }

    /// Get all marks for display
    pub fn list_marks(&self) -> Vec<(char, String, Position)> {
        let mut marks = Vec::new();

        // Local marks
        for (&mark, m) in &self.local_marks {
            let desc = format!("local: {}", m.commit_sha.as_ref().map(|s| &s[..7]).unwrap_or(""));
            marks.push((mark, desc, m.position));
        }

        // Global marks
        for (&mark, m) in &self.global_marks {
            let desc = format!("global: {} @ {}", m.view, m.commit_sha.as_ref().map(|s| &s[..7]).unwrap_or(""));
            marks.push((mark, desc, m.position));
        }

        // Special marks
        for (&mark, m) in &self.special_marks {
            let desc = match mark {
                '\'' | '`' => "last jump".to_string(),
                '.' => "last change".to_string(),
                '[' => "last yank/change start".to_string(),
                ']' => "last yank/change end".to_string(),
                '<' => "visual start".to_string(),
                '>' => "visual end".to_string(),
                '^' => "last insert".to_string(),
                _ => "special".to_string(),
            };
            marks.push((mark, desc, m.position));
        }

        marks.sort_by_key(|(m, _, _)| *m);
        marks
    }
}