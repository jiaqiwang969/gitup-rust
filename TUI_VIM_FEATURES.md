# Vim-Style Features for GitUp Terminal UI

## Command Palette Implementation

```rust
pub struct CommandPalette {
    buffer: String,
    history: VecDeque<String>,
    history_index: Option<usize>,
    suggestions: Vec<CommandSuggestion>,
    cursor_position: usize,
}

impl CommandPalette {
    pub fn execute(&mut self, command: &str) -> Result<CommandResult> {
        let parts: Vec<&str> = command.split_whitespace().collect();

        match parts.get(0) {
            Some("q") | Some("quit") => Ok(CommandResult::Quit),
            Some("w") | Some("write") => self.save_changes(),
            Some("wq") => {
                self.save_changes()?;
                Ok(CommandResult::Quit)
            }
            Some("e") | Some("edit") => {
                if let Some(branch) = parts.get(1) {
                    self.checkout_branch(branch)
                } else {
                    Err("Usage: :e <branch>".into())
                }
            }
            Some("branch") | Some("b") => {
                if let Some(name) = parts.get(1) {
                    self.create_branch(name)
                } else {
                    self.list_branches()
                }
            }
            Some("tag") | Some("t") => {
                if let Some(name) = parts.get(1) {
                    self.create_tag(name)
                } else {
                    self.list_tags()
                }
            }
            Some("merge") => {
                if let Some(branch) = parts.get(1) {
                    self.merge_branch(branch)
                } else {
                    Err("Usage: :merge <branch>".into())
                }
            }
            Some("rebase") => {
                if let Some(target) = parts.get(1) {
                    let interactive = parts.get(2) == Some(&"-i");
                    self.rebase_onto(target, interactive)
                } else {
                    Err("Usage: :rebase <target> [-i]".into())
                }
            }
            Some("cherry-pick") | Some("cp") => {
                if let Some(range) = parts.get(1) {
                    self.cherry_pick_range(range)
                } else {
                    Err("Usage: :cherry-pick <commit|range>".into())
                }
            }
            Some("reset") => {
                let mode = parts.get(1).unwrap_or(&"--mixed");
                let target = parts.get(2).unwrap_or("HEAD");
                self.reset(mode, target)
            }
            Some("stash") => {
                match parts.get(1) {
                    Some("save") => self.stash_save(parts.get(2)),
                    Some("pop") => self.stash_pop(),
                    Some("apply") => self.stash_apply(parts.get(2)),
                    Some("list") | None => self.stash_list(),
                    _ => Err("Unknown stash command".into())
                }
            }
            Some("g") => self.execute_global_command(&parts[1..]),
            Some("%s") => self.execute_substitute(&parts[1..]),
            Some("help") | Some("h") => self.show_help(parts.get(1)),
            Some("set") => self.set_option(parts.get(1)),
            _ => Err("Unknown command".into())
        }
    }

    fn complete(&mut self, partial: &str) -> Vec<CommandSuggestion> {
        let commands = vec![
            ("quit", "Exit the application"),
            ("write", "Save current changes"),
            ("edit <branch>", "Checkout branch"),
            ("branch <name>", "Create new branch"),
            ("merge <branch>", "Merge branch"),
            ("rebase <target>", "Rebase onto target"),
            ("cherry-pick <range>", "Cherry-pick commits"),
            ("reset <mode> <target>", "Reset to target"),
            ("stash <cmd>", "Stash operations"),
            ("tag <name>", "Create tag"),
            ("help <topic>", "Show help"),
        ];

        commands
            .into_iter()
            .filter(|(cmd, _)| cmd.starts_with(partial))
            .map(|(cmd, desc)| CommandSuggestion {
                command: cmd.to_string(),
                description: desc.to_string(),
            })
            .collect()
    }
}
```

## Operator-Pending Mode

```rust
pub enum Operator {
    Delete,
    Yank,
    Change,
    Indent,
    Outdent,
    Format,
    Comment,
    Uppercase,
    Lowercase,
    Mark,
    JumpToMark,
}

pub struct OperatorPending {
    operator: Operator,
    count: Option<usize>,
    register: char,
}

impl OperatorPending {
    pub fn apply(&self, motion: Motion, graph: &mut GitGraph) -> Result<()> {
        let range = motion.get_range(graph, self.count);

        match self.operator {
            Operator::Delete => {
                // Delete commits (interactive rebase with drop)
                for commit in range {
                    graph.mark_for_drop(commit);
                }
            }
            Operator::Yank => {
                // Copy commit SHAs to register
                let shas: Vec<String> = range.map(|c| c.sha.clone()).collect();
                graph.registers.insert(self.register, shas);
            }
            Operator::Change => {
                // Reword commits
                for commit in range {
                    graph.mark_for_reword(commit);
                }
            }
            _ => {}
        }

        Ok(())
    }
}
```

## Vim Registers System

```rust
pub struct RegisterManager {
    registers: HashMap<char, RegisterContent>,
    unnamed: RegisterContent,      // Default register "
    small_delete: RegisterContent, // Small delete register -
    numbered: VecDeque<RegisterContent>, // Numbered registers 0-9
    named: HashMap<char, RegisterContent>, // Named registers a-z
    read_only: HashMap<char, RegisterContent>, // Read-only registers
}

impl RegisterManager {
    pub fn new() -> Self {
        let mut read_only = HashMap::new();

        // Special read-only registers
        read_only.insert('%', RegisterContent::Text("current_file.rs".into())); // Current file
        read_only.insert('#', RegisterContent::Text("alternate_file.rs".into())); // Alternate file
        read_only.insert('.', RegisterContent::Text("last_inserted".into())); // Last inserted
        read_only.insert(':', RegisterContent::Text("last_command".into())); // Last command
        read_only.insert('/', RegisterContent::Text("last_search".into())); // Last search

        Self {
            registers: HashMap::new(),
            unnamed: RegisterContent::Empty,
            small_delete: RegisterContent::Empty,
            numbered: VecDeque::with_capacity(10),
            named: HashMap::new(),
            read_only,
        }
    }

    pub fn yank(&mut self, register: char, content: RegisterContent) {
        match register {
            '"' => self.unnamed = content.clone(),
            '0'..='9' => {
                let index = (register as usize) - ('0' as usize);
                if index == 0 {
                    self.numbered.push_front(content);
                    self.numbered.truncate(10);
                }
            }
            'a'..='z' => {
                self.named.insert(register, content);
            }
            'A'..='Z' => {
                // Append to register
                let lower = register.to_ascii_lowercase();
                let existing = self.named.get(&lower).cloned().unwrap_or(RegisterContent::Empty);
                self.named.insert(lower, existing.append(content));
            }
            _ => {}
        }
    }

    pub fn get(&self, register: char) -> Option<&RegisterContent> {
        match register {
            '"' => Some(&self.unnamed),
            '-' => Some(&self.small_delete),
            '0'..='9' => {
                let index = (register as usize) - ('0' as usize);
                self.numbered.get(index)
            }
            'a'..='z' | 'A'..='Z' => {
                self.named.get(&register.to_ascii_lowercase())
            }
            '%' | '#' | '.' | ':' | '/' => self.read_only.get(&register),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub enum RegisterContent {
    Empty,
    Text(String),
    Lines(Vec<String>),
    Commits(Vec<String>), // SHA list
    Block(Vec<Vec<String>>), // Visual block
}
```

## Vim Macros Recording and Playback

```rust
pub struct MacroRecorder {
    recording: Option<char>,
    current_macro: Vec<KeyEvent>,
    macros: HashMap<char, Vec<KeyEvent>>,
    playback_count: usize,
}

impl MacroRecorder {
    pub fn start_recording(&mut self, register: char) {
        self.recording = Some(register);
        self.current_macro.clear();
    }

    pub fn stop_recording(&mut self) {
        if let Some(register) = self.recording {
            self.macros.insert(register, self.current_macro.clone());
            self.recording = None;
        }
    }

    pub fn record_key(&mut self, key: KeyEvent) {
        if self.recording.is_some() {
            self.current_macro.push(key);
        }
    }

    pub fn play_macro(&self, register: char, count: usize) -> Vec<KeyEvent> {
        if let Some(macro_keys) = self.macros.get(&register) {
            macro_keys
                .iter()
                .cycle()
                .take(macro_keys.len() * count)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
}
```

## Git-Specific Text Objects

```rust
pub enum GitTextObject {
    Commit,         // ic, ac - inside/around commit
    Branch,         // ib, ab - inside/around branch
    Hunk,          // ih, ah - inside/around hunk
    Message,       // im, am - inside/around message
    Author,        // ia, aa - inside/around author
    Date,          // id, ad - inside/around date
    File,          // if, af - inside/around file
    Range,         // ir, ar - inside/around range
}

impl GitTextObject {
    pub fn select(&self, graph: &GitGraph, cursor: Position, around: bool) -> Range<Position> {
        match self {
            GitTextObject::Commit => {
                if around {
                    // Select entire commit including metadata
                    self.select_commit_with_metadata(graph, cursor)
                } else {
                    // Select just the commit message
                    self.select_commit_message(graph, cursor)
                }
            }
            GitTextObject::Branch => {
                if around {
                    // Select all commits in branch
                    self.select_branch_commits(graph, cursor)
                } else {
                    // Select commits unique to branch
                    self.select_branch_unique_commits(graph, cursor)
                }
            }
            GitTextObject::Hunk => {
                if around {
                    // Select hunk with context
                    self.select_hunk_with_context(graph, cursor)
                } else {
                    // Select just the changed lines
                    self.select_hunk_changes(graph, cursor)
                }
            }
            _ => Range { start: cursor, end: cursor }
        }
    }
}
```

## Vim-Style Marks and Jumps

```rust
pub struct MarkManager {
    local_marks: HashMap<char, Position>,  // a-z: local to current view
    global_marks: HashMap<char, GlobalMark>, // A-Z: global across views
    special_marks: HashMap<char, Position>, // Special marks like ', `, [, ]
    jump_list: VecDeque<Position>,
    jump_index: usize,
}

pub struct GlobalMark {
    view: String,
    position: Position,
    commit_sha: String,
}

impl MarkManager {
    pub fn set_mark(&mut self, mark: char, position: Position, commit_sha: Option<String>) {
        match mark {
            'a'..='z' => {
                self.local_marks.insert(mark, position);
            }
            'A'..='Z' => {
                self.global_marks.insert(mark, GlobalMark {
                    view: "graph".to_string(),
                    position,
                    commit_sha: commit_sha.unwrap_or_default(),
                });
            }
            _ => {}
        }
    }

    pub fn jump_to_mark(&mut self, mark: char) -> Option<Position> {
        match mark {
            'a'..='z' => self.local_marks.get(&mark).cloned(),
            'A'..='Z' => self.global_marks.get(&mark).map(|m| m.position),
            '\'' | '`' => self.special_marks.get(&mark).cloned(),
            _ => None,
        }
    }

    pub fn add_jump(&mut self, position: Position) {
        // Truncate jump list if we're not at the end
        self.jump_list.truncate(self.jump_index + 1);

        // Add new jump
        self.jump_list.push_back(position);
        if self.jump_list.len() > 100 {
            self.jump_list.pop_front();
        }

        self.jump_index = self.jump_list.len() - 1;
    }

    pub fn jump_backward(&mut self) -> Option<Position> {
        if self.jump_index > 0 {
            self.jump_index -= 1;
            self.jump_list.get(self.jump_index).cloned()
        } else {
            None
        }
    }

    pub fn jump_forward(&mut self) -> Option<Position> {
        if self.jump_index < self.jump_list.len() - 1 {
            self.jump_index += 1;
            self.jump_list.get(self.jump_index).cloned()
        } else {
            None
        }
    }
}
```

## Vim Configuration System

```rust
pub struct VimConfig {
    options: HashMap<String, VimOption>,
    keybindings: HashMap<String, String>,
    abbreviations: HashMap<String, String>,
    autocmds: Vec<AutoCommand>,
}

pub enum VimOption {
    Boolean(bool),
    Number(i32),
    String(String),
    List(Vec<String>),
}

impl VimConfig {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        // Load .vimrc style configuration
        let content = std::fs::read_to_string(path)?;
        let mut config = Self::default();

        for line in content.lines() {
            if line.starts_with("set ") {
                config.parse_set_command(&line[4..])?;
            } else if line.starts_with("map ") {
                config.parse_map_command(&line[4..])?;
            } else if line.starts_with("abbr ") {
                config.parse_abbr_command(&line[5..])?;
            } else if line.starts_with("autocmd ") {
                config.parse_autocmd(&line[8..])?;
            }
        }

        Ok(config)
    }

    pub fn get_option(&self, name: &str) -> Option<&VimOption> {
        self.options.get(name)
    }

    pub fn set_option(&mut self, name: &str, value: VimOption) {
        self.options.insert(name.to_string(), value);
    }
}
```

## Advanced Vim Features

### Split Windows

```rust
pub struct WindowManager {
    windows: Vec<Window>,
    active: usize,
    layout: WindowLayout,
}

pub enum WindowLayout {
    Single,
    HSplit(Vec<f32>), // Horizontal split with ratios
    VSplit(Vec<f32>), // Vertical split with ratios
    Grid(usize, usize), // Grid layout (rows, cols)
}

impl WindowManager {
    pub fn split_horizontal(&mut self) {
        let new_window = self.windows[self.active].clone();
        self.windows.insert(self.active + 1, new_window);
        self.update_layout();
    }

    pub fn split_vertical(&mut self) {
        let new_window = self.windows[self.active].clone();
        self.windows.push(new_window);
        self.update_layout();
    }

    pub fn navigate(&mut self, direction: Direction) {
        match direction {
            Direction::Up => self.active = self.active.saturating_sub(1),
            Direction::Down => self.active = (self.active + 1).min(self.windows.len() - 1),
            Direction::Left | Direction::Right => {
                // Handle vertical split navigation
            }
        }
    }
}
```

### Quickfix Integration

```rust
pub struct QuickfixList {
    items: Vec<QuickfixItem>,
    current: usize,
}

pub struct QuickfixItem {
    commit: String,
    line: usize,
    column: usize,
    message: String,
    severity: Severity,
}

impl QuickfixList {
    pub fn populate_from_search(&mut self, results: SearchResults) {
        self.items = results.matches.into_iter().map(|m| QuickfixItem {
            commit: m.sha,
            line: m.line,
            column: m.column,
            message: m.preview,
            severity: Severity::Info,
        }).collect();
    }

    pub fn next(&mut self) -> Option<&QuickfixItem> {
        if self.current < self.items.len() - 1 {
            self.current += 1;
            self.items.get(self.current)
        } else {
            None
        }
    }

    pub fn previous(&mut self) -> Option<&QuickfixItem> {
        if self.current > 0 {
            self.current -= 1;
            self.items.get(self.current)
        } else {
            None
        }
    }
}
```

## Vim Help System

```rust
pub struct HelpSystem {
    topics: HashMap<String, HelpTopic>,
    tags: HashMap<String, String>, // tag -> topic mapping
}

impl HelpSystem {
    pub fn show_help(&self, topic: Option<&str>) -> String {
        let topic = topic.unwrap_or("index");

        if let Some(help) = self.topics.get(topic) {
            format!("{}\n\n{}", help.title, help.content)
        } else {
            self.search_help(topic)
        }
    }

    pub fn register_topic(&mut self, topic: HelpTopic) {
        for tag in &topic.tags {
            self.tags.insert(tag.clone(), topic.name.clone());
        }
        self.topics.insert(topic.name.clone(), topic);
    }
}

pub struct HelpTopic {
    name: String,
    title: String,
    content: String,
    tags: Vec<String>,
    see_also: Vec<String>,
}
```

## Integration Points

The Vim-style modal system integrates with the existing Terminal UI through:

1. **Event Handler**: All keyboard input routes through the Vim state machine
2. **Renderer**: Mode-specific rendering and status line display
3. **Operations**: Git operations triggered by Vim commands and operators
4. **Configuration**: Vimrc-style configuration for customization
5. **Help System**: Context-aware help accessible via `:help`

This design provides a complete Vim experience while maintaining the power of Git operations in a terminal environment.