use std::collections::{HashMap, VecDeque};

/// Content that can be stored in registers
#[derive(Debug, Clone)]
pub enum RegisterContent {
    /// Empty register
    Empty,

    /// Plain text
    Text(String),

    /// Multiple lines of text
    Lines(Vec<String>),

    /// Git commit SHAs
    Commits(Vec<String>),

    /// Visual block selection
    Block(Vec<Vec<String>>),
}

impl RegisterContent {
    /// Append content to existing content
    pub fn append(&self, other: RegisterContent) -> RegisterContent {
        match (self, &other) {
            (RegisterContent::Empty, _) => other,
            (existing, RegisterContent::Empty) => existing.clone(),

            (RegisterContent::Text(s1), RegisterContent::Text(s2)) => {
                RegisterContent::Text(format!("{}{}", s1, s2))
            }

            (RegisterContent::Lines(l1), RegisterContent::Lines(l2)) => {
                let mut lines = l1.clone();
                lines.extend(l2.clone());
                RegisterContent::Lines(lines)
            }

            (RegisterContent::Commits(c1), RegisterContent::Commits(c2)) => {
                let mut commits = c1.clone();
                commits.extend(c2.clone());
                RegisterContent::Commits(commits)
            }

            _ => other, // For mismatched types, use the new content
        }
    }

    /// Convert content to string for pasting
    pub fn to_string(&self) -> String {
        match self {
            RegisterContent::Empty => String::new(),
            RegisterContent::Text(s) => s.clone(),
            RegisterContent::Lines(lines) => lines.join("\n"),
            RegisterContent::Commits(commits) => commits.join("\n"),
            RegisterContent::Block(block) => {
                block.iter()
                    .map(|row| row.join(""))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }

    /// Check if register is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, RegisterContent::Empty)
    }
}

/// Vim register manager
pub struct RegisterManager {
    // Standard registers
    unnamed: RegisterContent,       // " - default register
    small_delete: RegisterContent,  // - - small delete register
    numbered: VecDeque<RegisterContent>, // 0-9 - numbered registers

    // Named registers
    named: HashMap<char, RegisterContent>, // a-z, A-Z

    // Read-only registers
    read_only: HashMap<char, RegisterContent>,

    // Special registers
    last_search: String,    // / - last search pattern
    last_command: String,   // : - last command
    last_inserted: String,  // . - last inserted text
    current_file: String,   // % - current file name
    alternate_file: String, // # - alternate file name
    expression: String,     // = - expression register
    black_hole: RegisterContent, // _ - black hole register
}

impl RegisterManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            unnamed: RegisterContent::Empty,
            small_delete: RegisterContent::Empty,
            numbered: VecDeque::with_capacity(10),
            named: HashMap::new(),
            read_only: HashMap::new(),
            last_search: String::new(),
            last_command: String::new(),
            last_inserted: String::new(),
            current_file: String::new(),
            alternate_file: String::new(),
            expression: String::new(),
            black_hole: RegisterContent::Empty,
        };

        // Initialize numbered registers
        for _ in 0..10 {
            mgr.numbered.push_back(RegisterContent::Empty);
        }

        mgr
    }

    /// Store content in a register
    pub fn set(&mut self, register: char, content: RegisterContent) {
        match register {
            // Unnamed register
            '"' => {
                self.unnamed = content.clone();
                // Also update numbered register 0 for yanks
                self.numbered[0] = content;
            }

            // Small delete register
            '-' => self.small_delete = content,

            // Numbered registers
            '0'..='9' => {
                let index = (register as usize) - ('0' as usize);
                if index == 0 {
                    // Register 0 is for yanks
                    self.numbered[0] = content;
                } else {
                    // Registers 1-9 are for deletes/changes
                    // Shift existing content
                    for i in (1..index).rev() {
                        if i + 1 < 10 {
                            self.numbered[i + 1] = self.numbered[i].clone();
                        }
                    }
                    self.numbered[index] = content;
                }
            }

            // Named registers (lowercase)
            'a'..='z' => {
                self.named.insert(register, content);
            }

            // Named registers (uppercase - append)
            'A'..='Z' => {
                let lower = register.to_ascii_lowercase();
                let existing = self.named.get(&lower).cloned()
                    .unwrap_or(RegisterContent::Empty);
                self.named.insert(lower, existing.append(content));
            }

            // Special registers
            '/' => self.last_search = content.to_string(),
            ':' => self.last_command = content.to_string(),
            '.' => self.last_inserted = content.to_string(),
            '%' => self.current_file = content.to_string(),
            '#' => self.alternate_file = content.to_string(),
            '=' => self.expression = content.to_string(),

            // Black hole register (discards content)
            '_' => {}

            _ => {} // Ignore invalid registers
        }
    }

    /// Get content from a register
    pub fn get(&self, register: char) -> RegisterContent {
        match register {
            '"' => self.unnamed.clone(),
            '-' => self.small_delete.clone(),

            '0'..='9' => {
                let index = (register as usize) - ('0' as usize);
                self.numbered.get(index)
                    .cloned()
                    .unwrap_or(RegisterContent::Empty)
            }

            'a'..='z' | 'A'..='Z' => {
                let lower = register.to_ascii_lowercase();
                self.named.get(&lower)
                    .cloned()
                    .unwrap_or(RegisterContent::Empty)
            }

            '/' => RegisterContent::Text(self.last_search.clone()),
            ':' => RegisterContent::Text(self.last_command.clone()),
            '.' => RegisterContent::Text(self.last_inserted.clone()),
            '%' => RegisterContent::Text(self.current_file.clone()),
            '#' => RegisterContent::Text(self.alternate_file.clone()),
            '=' => RegisterContent::Text(self.expression.clone()),

            '_' => RegisterContent::Empty, // Black hole always returns empty

            _ => {
                // Check read-only registers
                self.read_only.get(&register)
                    .cloned()
                    .unwrap_or(RegisterContent::Empty)
            }
        }
    }

    /// Yank content to register
    pub fn yank(&mut self, register: char, content: RegisterContent) {
        // Yanks always go to register 0 as well
        self.numbered[0] = content.clone();
        self.set(register, content);
    }

    /// Delete content to register
    pub fn delete(&mut self, register: char, content: RegisterContent, is_small: bool) {
        if is_small {
            // Small deletes go to - register
            self.small_delete = content.clone();
        } else {
            // Large deletes shift numbered registers 1-9
            for i in (1..9).rev() {
                self.numbered[i + 1] = self.numbered[i].clone();
            }
            self.numbered[1] = content.clone();
        }

        // Also set the specified register
        self.set(register, content);
    }

    /// Set a read-only register
    pub fn set_read_only(&mut self, register: char, content: RegisterContent) {
        self.read_only.insert(register, content);
    }

    /// Get all non-empty registers for display
    pub fn list_registers(&self) -> Vec<(char, String)> {
        let mut registers = Vec::new();

        // Check unnamed register
        if !self.unnamed.is_empty() {
            registers.push(('"', self.unnamed.to_string()));
        }

        // Check numbered registers
        for i in 0..10 {
            if !self.numbered[i].is_empty() {
                let reg = (b'0' + i as u8) as char;
                registers.push((reg, self.numbered[i].to_string()));
            }
        }

        // Check named registers
        for (&reg, content) in &self.named {
            if !content.is_empty() {
                registers.push((reg, content.to_string()));
            }
        }

        // Check special registers
        if !self.last_search.is_empty() {
            registers.push(('/', self.last_search.clone()));
        }
        if !self.last_command.is_empty() {
            registers.push((':', self.last_command.clone()));
        }

        registers
    }
}