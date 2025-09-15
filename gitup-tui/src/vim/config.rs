use std::collections::HashMap;
use std::path::Path;
use anyhow::{Result, bail};

/// Vim configuration options
#[derive(Debug, Clone)]
pub enum VimOption {
    Boolean(bool),
    Number(i32),
    String(String),
    List(Vec<String>),
}

/// Vim configuration management
pub struct VimConfig {
    // Configuration options
    options: HashMap<String, VimOption>,

    // Key mappings
    keybindings: HashMap<String, String>,

    // Command abbreviations
    abbreviations: HashMap<String, String>,

    // Auto commands (simplified)
    autocmds: Vec<AutoCommand>,
}

#[derive(Debug, Clone)]
pub struct AutoCommand {
    pub event: String,
    pub pattern: String,
    pub command: String,
}

impl Default for VimConfig {
    fn default() -> Self {
        let mut config = Self {
            options: HashMap::new(),
            keybindings: HashMap::new(),
            abbreviations: HashMap::new(),
            autocmds: Vec::new(),
        };

        // Set default options
        config.set_defaults();

        config
    }
}

impl VimConfig {
    /// Set default configuration options
    fn set_defaults(&mut self) {
        // Display options
        self.options.insert("number".to_string(), VimOption::Boolean(true));
        self.options.insert("relativenumber".to_string(), VimOption::Boolean(false));
        self.options.insert("cursorline".to_string(), VimOption::Boolean(true));
        self.options.insert("wrap".to_string(), VimOption::Boolean(false));

        // Search options
        self.options.insert("ignorecase".to_string(), VimOption::Boolean(true));
        self.options.insert("smartcase".to_string(), VimOption::Boolean(true));
        self.options.insert("hlsearch".to_string(), VimOption::Boolean(true));
        self.options.insert("incsearch".to_string(), VimOption::Boolean(true));

        // Behavior options
        self.options.insert("scrolloff".to_string(), VimOption::Number(5));
        self.options.insert("sidescrolloff".to_string(), VimOption::Number(5));
        self.options.insert("timeoutlen".to_string(), VimOption::Number(1000));
        self.options.insert("updatetime".to_string(), VimOption::Number(300));

        // Git-specific options
        self.options.insert("showbranch".to_string(), VimOption::Boolean(true));
        self.options.insert("showauthor".to_string(), VimOption::Boolean(true));
        self.options.insert("showdate".to_string(), VimOption::Boolean(true));
        self.options.insert("dateformat".to_string(), VimOption::String("%Y-%m-%d".to_string()));
        self.options.insert("graphstyle".to_string(), VimOption::String("ascii".to_string()));

        // Fold options
        self.options.insert("foldenable".to_string(), VimOption::Boolean(true));
        self.options.insert("foldmethod".to_string(), VimOption::String("branch".to_string()));
        self.options.insert("foldlevel".to_string(), VimOption::Number(99));

        // Colors
        self.options.insert("colorscheme".to_string(), VimOption::String("default".to_string()));
    }

    /// Load configuration from file (vimrc style)
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config = Self::default();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('"') {
                continue;
            }

            // Parse the line
            if line.starts_with("set ") {
                config.parse_set(&line[4..])?;
            } else if line.starts_with("map ") {
                config.parse_map(&line[4..])?;
            } else if line.starts_with("abbr ") || line.starts_with("abbreviate ") {
                let start = if line.starts_with("abbr ") { 5 } else { 11 };
                config.parse_abbr(&line[start..])?;
            } else if line.starts_with("autocmd ") {
                config.parse_autocmd(&line[8..])?;
            }
        }

        Ok(config)
    }

    /// Parse a set command
    fn parse_set(&mut self, args: &str) -> Result<()> {
        let args = args.trim();

        if args.contains('=') {
            // Option with value
            let parts: Vec<&str> = args.splitn(2, '=').collect();
            if parts.len() != 2 {
                bail!("Invalid set command: {}", args);
            }

            let name = parts[0].trim();
            let value = parts[1].trim();

            // Try to parse as different types
            if let Ok(num) = value.parse::<i32>() {
                self.options.insert(name.to_string(), VimOption::Number(num));
            } else if value == "true" || value == "yes" {
                self.options.insert(name.to_string(), VimOption::Boolean(true));
            } else if value == "false" || value == "no" {
                self.options.insert(name.to_string(), VimOption::Boolean(false));
            } else {
                self.options.insert(name.to_string(), VimOption::String(value.to_string()));
            }
        } else if args.starts_with("no") {
            // Boolean option set to false
            let name = &args[2..];
            self.options.insert(name.to_string(), VimOption::Boolean(false));
        } else {
            // Boolean option set to true
            self.options.insert(args.to_string(), VimOption::Boolean(true));
        }

        Ok(())
    }

    /// Parse a map command
    fn parse_map(&mut self, args: &str) -> Result<()> {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() != 2 {
            bail!("Invalid map command: {}", args);
        }

        self.keybindings.insert(parts[0].to_string(), parts[1].to_string());
        Ok(())
    }

    /// Parse an abbreviation command
    fn parse_abbr(&mut self, args: &str) -> Result<()> {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() != 2 {
            bail!("Invalid abbreviation command: {}", args);
        }

        self.abbreviations.insert(parts[0].to_string(), parts[1].to_string());
        Ok(())
    }

    /// Parse an autocmd
    fn parse_autocmd(&mut self, args: &str) -> Result<()> {
        let parts: Vec<&str> = args.splitn(3, ' ').collect();
        if parts.len() != 3 {
            bail!("Invalid autocmd: {}", args);
        }

        self.autocmds.push(AutoCommand {
            event: parts[0].to_string(),
            pattern: parts[1].to_string(),
            command: parts[2].to_string(),
        });

        Ok(())
    }

    /// Get an option value
    pub fn get_option(&self, name: &str) -> Option<&VimOption> {
        self.options.get(name)
    }

    /// Set an option value
    pub fn set_option(&mut self, name: &str, value: VimOption) {
        self.options.insert(name.to_string(), value);
    }

    /// Get a boolean option
    pub fn get_bool(&self, name: &str) -> bool {
        match self.options.get(name) {
            Some(VimOption::Boolean(b)) => *b,
            _ => false,
        }
    }

    /// Get a number option
    pub fn get_number(&self, name: &str) -> i32 {
        match self.options.get(name) {
            Some(VimOption::Number(n)) => *n,
            _ => 0,
        }
    }

    /// Get a string option
    pub fn get_string(&self, name: &str) -> String {
        match self.options.get(name) {
            Some(VimOption::String(s)) => s.clone(),
            _ => String::new(),
        }
    }

    /// Get key mapping
    pub fn get_mapping(&self, key: &str) -> Option<&str> {
        self.keybindings.get(key).map(|s| s.as_str())
    }

    /// Get abbreviation
    pub fn get_abbreviation(&self, abbr: &str) -> Option<&str> {
        self.abbreviations.get(abbr).map(|s| s.as_str())
    }

    /// Get autocmds for an event
    pub fn get_autocmds(&self, event: &str) -> Vec<&AutoCommand> {
        self.autocmds
            .iter()
            .filter(|cmd| cmd.event == event)
            .collect()
    }

    /// Generate vimrc content
    pub fn to_vimrc(&self) -> String {
        let mut lines = Vec::new();

        lines.push("\" GitUp Vim configuration".to_string());
        lines.push("".to_string());

        // Options
        lines.push("\" Options".to_string());
        for (name, value) in &self.options {
            let line = match value {
                VimOption::Boolean(true) => format!("set {}", name),
                VimOption::Boolean(false) => format!("set no{}", name),
                VimOption::Number(n) => format!("set {}={}", name, n),
                VimOption::String(s) => format!("set {}={}", name, s),
                VimOption::List(l) => format!("set {}={}", name, l.join(",")),
            };
            lines.push(line);
        }
        lines.push("".to_string());

        // Key mappings
        if !self.keybindings.is_empty() {
            lines.push("\" Key mappings".to_string());
            for (key, mapping) in &self.keybindings {
                lines.push(format!("map {} {}", key, mapping));
            }
            lines.push("".to_string());
        }

        // Abbreviations
        if !self.abbreviations.is_empty() {
            lines.push("\" Abbreviations".to_string());
            for (abbr, expansion) in &self.abbreviations {
                lines.push(format!("abbr {} {}", abbr, expansion));
            }
            lines.push("".to_string());
        }

        // Autocmds
        if !self.autocmds.is_empty() {
            lines.push("\" Auto commands".to_string());
            for cmd in &self.autocmds {
                lines.push(format!("autocmd {} {} {}", cmd.event, cmd.pattern, cmd.command));
            }
        }

        lines.join("\n")
    }
}