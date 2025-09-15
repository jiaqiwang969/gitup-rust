use std::collections::VecDeque;
use anyhow::{Result, bail};

/// Command palette for Vim-style ex commands
pub struct CommandPalette {
    // Command history
    history: VecDeque<String>,
    history_index: Option<usize>,

    // Command abbreviations
    abbreviations: Vec<(String, String)>,

    // Command aliases
    aliases: Vec<(String, String)>,
}

impl CommandPalette {
    pub fn new() -> Self {
        let mut palette = Self {
            history: VecDeque::with_capacity(100),
            history_index: None,
            abbreviations: Vec::new(),
            aliases: Vec::new(),
        };

        // Add default abbreviations
        palette.add_default_abbreviations();

        palette
    }

    fn add_default_abbreviations(&mut self) {
        self.abbreviations = vec![
            ("q".to_string(), "quit".to_string()),
            ("w".to_string(), "write".to_string()),
            ("wq".to_string(), "write|quit".to_string()),
            ("x".to_string(), "write|quit".to_string()),
            ("e".to_string(), "edit".to_string()),
            ("b".to_string(), "branch".to_string()),
            ("t".to_string(), "tag".to_string()),
            ("cp".to_string(), "cherry-pick".to_string()),
            ("h".to_string(), "help".to_string()),
        ];
    }

    /// Parse and execute a command
    pub fn execute(&mut self, command: &str) -> Result<CommandResult> {
        // Add to history
        self.add_to_history(command.to_string());

        // Expand abbreviations
        let command = self.expand_abbreviation(command);

        // Split by pipe for command chaining
        let commands: Vec<&str> = command.split('|').collect();

        let mut last_result = CommandResult::None;
        for cmd in commands {
            last_result = self.execute_single(cmd.trim())?;
            if matches!(last_result, CommandResult::Quit) {
                break;
            }
        }

        Ok(last_result)
    }

    fn execute_single(&self, command: &str) -> Result<CommandResult> {
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(CommandResult::None);
        }

        match parts[0] {
            // File/Buffer commands
            "quit" | "q" => Ok(CommandResult::Quit),
            "write" | "w" => Ok(CommandResult::Write),
            "edit" | "e" => {
                if let Some(target) = parts.get(1) {
                    Ok(CommandResult::Edit(target.to_string()))
                } else {
                    bail!("Usage: :edit <branch|commit>")
                }
            }

            // Git branch commands
            "branch" | "b" => {
                if let Some(name) = parts.get(1) {
                    Ok(CommandResult::CreateBranch(name.to_string()))
                } else {
                    Ok(CommandResult::ListBranches)
                }
            }
            "checkout" | "co" => {
                if let Some(target) = parts.get(1) {
                    Ok(CommandResult::Checkout(target.to_string()))
                } else {
                    bail!("Usage: :checkout <branch|commit>")
                }
            }

            // Git tag commands
            "tag" | "t" => {
                if let Some(name) = parts.get(1) {
                    Ok(CommandResult::CreateTag(name.to_string()))
                } else {
                    Ok(CommandResult::ListTags)
                }
            }

            // Git merge/rebase commands
            "merge" => {
                if let Some(branch) = parts.get(1) {
                    Ok(CommandResult::Merge(branch.to_string()))
                } else {
                    bail!("Usage: :merge <branch>")
                }
            }
            "rebase" => {
                if let Some(target) = parts.get(1) {
                    let interactive = parts.get(2) == Some(&"-i");
                    Ok(CommandResult::Rebase {
                        target: target.to_string(),
                        interactive,
                    })
                } else {
                    bail!("Usage: :rebase <target> [-i]")
                }
            }

            // Git cherry-pick
            "cherry-pick" | "cp" => {
                if let Some(range) = parts.get(1) {
                    Ok(CommandResult::CherryPick(range.to_string()))
                } else {
                    bail!("Usage: :cherry-pick <commit|range>")
                }
            }

            // Git reset
            "reset" => {
                let mode = parts.get(1).map(|s| s.to_string()).unwrap_or_else(|| "--mixed".to_string());
                let target = parts.get(2).map(|s| s.to_string()).unwrap_or_else(|| "HEAD".to_string());
                Ok(CommandResult::Reset {
                    mode,
                    target,
                })
            }

            // Git stash commands
            "stash" => {
                match parts.get(1).map(|s| *s) {
                    Some("save") => {
                        let message = parts[2..].join(" ");
                        Ok(CommandResult::StashSave(Some(message)))
                    }
                    Some("pop") => Ok(CommandResult::StashPop),
                    Some("apply") => {
                        let index = parts.get(2).and_then(|s| s.parse().ok());
                        Ok(CommandResult::StashApply(index))
                    }
                    Some("list") | None => Ok(CommandResult::StashList),
                    Some("drop") => {
                        let index = parts.get(2).and_then(|s| s.parse().ok());
                        Ok(CommandResult::StashDrop(index))
                    }
                    _ => bail!("Unknown stash command")
                }
            }

            // Git remote commands
            "fetch" => Ok(CommandResult::Fetch(parts.get(1).map(|s| s.to_string()))),
            "pull" => Ok(CommandResult::Pull(parts.get(1).map(|s| s.to_string()))),
            "push" => Ok(CommandResult::Push {
                remote: parts.get(1).map(|s| s.to_string()),
                force: parts.contains(&"--force") || parts.contains(&"-f"),
            }),

            // Search/Replace
            "substitute" | "s" => {
                // Parse :%s/pattern/replacement/flags
                let args = parts[1..].join(" ");
                Ok(CommandResult::Substitute(args))
            }
            "global" | "g" => {
                // Parse :g/pattern/command
                let args = parts[1..].join(" ");
                Ok(CommandResult::Global(args))
            }

            // Settings
            "set" => {
                if let Some(option) = parts.get(1) {
                    Ok(CommandResult::Set(option.to_string()))
                } else {
                    Ok(CommandResult::ShowSettings)
                }
            }

            // Help
            "help" | "h" => {
                Ok(CommandResult::Help(parts.get(1).map(|s| s.to_string())))
            }

            // Line navigation
            _ if parts[0].chars().all(|c| c.is_ascii_digit()) => {
                if let Ok(line) = parts[0].parse::<usize>() {
                    Ok(CommandResult::GoToLine(line))
                } else {
                    bail!("Invalid line number")
                }
            }

            _ => bail!("Unknown command: {}", parts[0])
        }
    }

    /// Expand command abbreviations
    fn expand_abbreviation(&self, command: &str) -> String {
        for (abbr, full) in &self.abbreviations {
            if command == abbr {
                return full.clone();
            }
        }
        command.to_string()
    }

    /// Add command to history
    fn add_to_history(&mut self, command: String) {
        // Don't add duplicate commands
        if self.history.back() != Some(&command) {
            self.history.push_back(command);
            if self.history.len() > 100 {
                self.history.pop_front();
            }
        }
        self.history_index = None;
    }

    /// Get previous command from history
    pub fn previous_history(&mut self) -> Option<&str> {
        if self.history.is_empty() {
            return None;
        }

        self.history_index = Some(match self.history_index {
            None => self.history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        });

        self.history.get(self.history_index.unwrap()).map(|s| s.as_str())
    }

    /// Get next command from history
    pub fn next_history(&mut self) -> Option<&str> {
        if self.history.is_empty() {
            return None;
        }

        self.history_index = self.history_index.and_then(|i| {
            if i < self.history.len() - 1 {
                Some(i + 1)
            } else {
                None
            }
        });

        self.history_index.and_then(|i| self.history.get(i).map(|s| s.as_str()))
    }

    /// Get command suggestions for completion
    pub fn get_suggestions(&self, partial: &str) -> Vec<String> {
        let commands = vec![
            "quit", "write", "edit", "branch", "checkout", "tag",
            "merge", "rebase", "cherry-pick", "reset", "stash",
            "fetch", "pull", "push", "substitute", "global",
            "set", "help",
        ];

        commands
            .into_iter()
            .filter(|c| c.starts_with(partial))
            .map(String::from)
            .collect()
    }
}

/// Result of executing a command
#[derive(Debug, Clone)]
pub enum CommandResult {
    None,
    Quit,
    Write,
    Edit(String),

    // Git operations
    Checkout(String),
    CreateBranch(String),
    ListBranches,
    CreateTag(String),
    ListTags,
    Merge(String),
    Rebase { target: String, interactive: bool },
    CherryPick(String),
    Reset { mode: String, target: String },

    // Stash operations
    StashSave(Option<String>),
    StashPop,
    StashApply(Option<usize>),
    StashList,
    StashDrop(Option<usize>),

    // Remote operations
    Fetch(Option<String>),
    Pull(Option<String>),
    Push { remote: Option<String>, force: bool },

    // Search/Replace
    Substitute(String),
    Global(String),

    // Settings
    Set(String),
    ShowSettings,

    // Navigation
    GoToLine(usize),

    // Help
    Help(Option<String>),
}