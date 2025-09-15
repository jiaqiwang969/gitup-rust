# Terminal UI Enhancement Plan - Git Graph Visualization (Enhanced)

## 📊 Deep Analysis of Reference Projects

### GitUp (macOS Native)
**Architecture Highlights:**
- **Graph Generation**: `GIGraph` class builds commit DAG with topological sorting
- **Layer System**: Assigns commits to horizontal lanes to minimize crossings
- **Optimization**: Skips stale branches and standalone tags to reduce noise
- **Memory Efficient**: Uses mapping arrays for fast lookups

### VSCode GitLens (Deep Dive)
**Advanced Features Discovered:**

#### 1. **Event-Driven Architecture**
```typescript
// Real-time updates on Git changes
- Repository change events (RepositoryChangeEvent)
- File system watchers for .git directory
- Debounced updates for performance
- Cancellation tokens for long operations
```

#### 2. **Interactive Command System**
```typescript
// Context-aware actions from graph
- Double-click handlers for refs and rows
- Context menus with item-specific actions
- Direct Git operations: merge, rebase, cherry-pick
- Undo/redo support for operations
```

#### 3. **State Management**
```typescript
// Sophisticated state synchronization
- WebView IPC protocol for UI updates
- Incremental loading with virtual scrolling
- Search results overlay on graph
- Multi-selection with batch operations
```

#### 4. **Performance Optimizations**
- Lazy loading of commit details
- Avatar caching system
- Deferred stats loading
- Pagination with cursor-based navigation
- Background metadata fetching

## 🎯 Enhanced Terminal UI Design

### Core Architecture Components

#### 1. **Event System (New)**
```rust
// Event-driven updates inspired by GitLens
pub enum GraphEvent {
    // Repository events
    RepositoryChanged(RepositoryChange),
    BranchChanged(String),
    WorkingTreeChanged,

    // User interactions
    NodeSelected(String),
    NodeActivated(String), // Double-click equivalent
    ContextMenuRequested(Position),

    // Graph operations
    SearchInitiated(String),
    FilterApplied(FilterCriteria),
    ViewModeChanged(ViewMode),
}

pub struct EventBus {
    subscribers: Vec<Box<dyn EventHandler>>,
    pending_events: VecDeque<GraphEvent>,
}
```

#### 2. **Interactive Operations Manager**
```rust
pub struct OperationsManager {
    repository: Repository,
    operation_queue: VecDeque<Operation>,
    undo_stack: Vec<Operation>,

    pub fn execute(&mut self, op: Operation) -> Result<()> {
        match op {
            Operation::Merge(ref_) => self.merge_branch(ref_),
            Operation::Rebase(ref_) => self.rebase_onto(ref_),
            Operation::CherryPick(commits) => self.cherry_pick(commits),
            Operation::Reset(ref_, mode) => self.reset_to(ref_, mode),
            // ... more operations
        }
    }
}
```

#### 3. **Advanced Graph Renderer with Vim Integration**
```rust
pub struct GraphRenderer {
    layout_engine: LayoutEngine,
    style_manager: StyleManager,
    cache: RenderCache,
    vim_state: VimState,

    pub fn render(&mut self, graph: &GitGraph, area: Rect) -> Buffer {
        // Multi-pass rendering for optimal display
        let layout = self.layout_engine.compute(graph);
        let styled = self.style_manager.apply(layout);

        // Apply Vim mode highlighting
        let highlighted = self.apply_vim_highlights(styled);

        // Render mode line
        self.render_mode_line(area);

        self.render_to_buffer(highlighted, area)
    }

    fn render_mode_line(&self, area: Rect) -> String {
        match self.vim_state.mode {
            VimMode::Normal => "-- NORMAL --",
            VimMode::Insert => "-- INSERT --",
            VimMode::Visual => "-- VISUAL --",
            VimMode::VisualLine => "-- VISUAL LINE --",
            VimMode::VisualBlock => "-- VISUAL BLOCK --",
            VimMode::Command => format!(":{}", self.vim_state.command_buffer),
            VimMode::Search => format!("/{}", self.vim_state.search_pattern),
            VimMode::Operator => format!("-- {} --", self.vim_state.operator),
        }
    }
}

// Advanced layout algorithms
pub enum LayoutAlgorithm {
    Compact,      // Minimize vertical space
    Chronological, // Time-based positioning
    Topological,  // Branch-focused layout
    Hybrid,       // Smart combination
}

// Vim-specific rendering helpers
impl GraphRenderer {
    fn apply_vim_highlights(&self, graph: &mut GitGraph) -> &GitGraph {
        match self.vim_state.mode {
            VimMode::Visual | VimMode::VisualLine | VimMode::VisualBlock => {
                self.highlight_selection(graph);
            }
            VimMode::Search => {
                self.highlight_search_matches(graph);
            }
            _ => {}
        }

        // Always highlight cursor position
        self.highlight_cursor(graph);

        // Show marks
        self.render_marks(graph);

        graph
    }
}

## 🎮 Vim-Style Modal System

### Modal States
```rust
pub enum VimMode {
    Normal,     // Default navigation and commands
    Insert,     // Text input (commit messages, search)
    Visual,     // Selection mode (single)
    VisualLine, // Line selection mode
    VisualBlock,// Block selection mode
    Command,    // Command line mode (:commands)
    Search,     // Search mode (/pattern)
    Operator,   // Pending operator (d, y, c)
}

pub struct VimState {
    mode: VimMode,
    operator: Option<Operator>,
    count: Option<usize>,
    register: char,
    marks: HashMap<char, String>, // mark -> commit SHA
    macros: HashMap<char, Vec<KeyEvent>>,
    last_command: Vec<KeyEvent>,
    search_pattern: String,
    command_history: VecDeque<String>,
    visual_anchor: Option<String>,
}
```

### Normal Mode Keybindings

| Key | Action | Description |
|-----|--------|-------------|
| **Navigation** | | |
| `j` / `k` | Next/Previous commit | Move through graph |
| `h` / `l` | Parent/Child commit | Navigate relationships |
| `gg` / `G` | First/Last commit | Jump to extremes |
| `{` / `}` | Previous/Next branch | Branch navigation |
| `[c` / `]c` | Previous/Next conflict | Conflict navigation |
| `[m` / `]m` | Previous/Next merge | Merge navigation |
| `<C-d>` / `<C-u>` | Half page down/up | Scroll viewport |
| `<C-f>` / `<C-b>` | Full page down/up | Page navigation |
| `H` / `M` / `L` | Top/Middle/Bottom | Viewport jumps |
| **Marks & Jumps** | | |
| `m{a-z}` | Set mark | Mark commit |
| `'{a-z}` | Jump to mark | Go to marked commit |
| `''` | Jump to last position | Previous location |
| `<C-o>` / `<C-i>` | Jump back/forward | Navigation history |
| **Selection** | | |
| `v` | Visual mode | Start selection |
| `V` | Visual line mode | Select full commits |
| `<C-v>` | Visual block mode | Column selection |
| `gv` | Reselect | Last selection |
| **Operations** | | |
| `d` | Delete/Drop | Remove commits (rebase) |
| `y` | Yank | Copy commit SHA |
| `p` / `P` | Pick/Put | Cherry-pick after/before |
| `c` | Change | Reword commit |
| `r` | Replace | Reset to commit |
| `x` | Delete | Drop single commit |
| **Git Actions** | | |
| `gb` | Checkout branch | Switch to branch |
| `gB` | Create branch | New branch at commit |
| `gc` | Checkout commit | Detached HEAD |
| `gC` | Cherry-pick | Pick commit |
| `gm` | Merge | Merge into current |
| `gr` | Rebase | Rebase onto commit |
| `gR` | Interactive rebase | Edit commits |
| `gs` | Squash | Squash with previous |
| `gf` | Fixup | Fixup with previous |
| `gt` | Create tag | Tag commit |
| `gT` | Delete tag | Remove tag |
| **View Controls** | | |
| `zo` / `zc` | Open/Close fold | Expand/collapse branch |
| `za` | Toggle fold | Toggle branch |
| `zR` / `zM` | Open/Close all | Expand/collapse all |
| `]` / `[` | Indent/Outdent | Adjust graph width |
| **Search** | | |
| `/` | Search forward | Find commits |
| `?` | Search backward | Reverse search |
| `n` / `N` | Next/Previous match | Search navigation |
| `*` / `#` | Search word | Current SHA/message |

### Visual Mode Operations

| Key | Action | Description |
|-----|--------|-------------|
| `d` | Delete selected | Drop commits |
| `y` | Yank selected | Copy SHAs |
| `c` | Change selected | Reword commits |
| `s` | Squash selected | Combine commits |
| `f` | Fixup selected | Fixup commits |
| `>` / `<` | Indent/Outdent | Reorder commits |
| `J` | Join | Squash together |
| `gC` | Cherry-pick range | Pick selection |
| `o` / `O` | Other end | Move anchor |

### Command Mode (`:` prefix)

```vim
:q              " Quit
:w              " Write/Save changes
:wq             " Write and quit
:e <branch>     " Edit/checkout branch
:split <ref>    " Split view with ref
:vsplit <ref>   " Vertical split
:diffget        " Get changes from diff
:diffput        " Put changes to diff
:merge <branch> " Merge branch
:rebase <ref>   " Rebase onto ref
:cherry-pick <range> " Pick commit range
:reset [--hard|--soft] <ref> " Reset to ref
:stash [save|pop|apply] " Stash operations
:tag <name>     " Create tag
:branch <name>  " Create branch
:remote <cmd>   " Remote operations
:fetch [remote] " Fetch from remote
:pull [remote]  " Pull from remote
:push [remote]  " Push to remote
:%s/old/new/g   " Bulk commit message edit
:g/pattern/d    " Delete matching commits
:help           " Show help
:set <option>   " Set configuration
```

### Search Mode (`/` and `?`)

```vim
/keyword        " Search in commit messages
/^feat:         " Search commit prefixes
/@author        " Search by author
/#issue-123     " Search issue references
/\d{4}-\d{2}    " Regex pattern search
/<C-r>"         " Paste from register
/<Up>/<Down>    " Search history
```

### Interactive Features Matrix

| Feature | Normal Mode | Visual Mode | Command Mode | Action |
|---------|------------|-------------|--------------|--------|
| Navigate | `j/k/h/l` | Extend selection | - | Move through graph |
| Select | `v/V/<C-v>` | `o/O` anchor | - | Selection modes |
| Operate | `d/y/c/p` | Batch operation | `:command` | Git operations |
| Search | `//?*#` | Extend to match | `:g/pattern/` | Find commits |
| Jump | `gg/G/{}` | Extend range | `:123` | Quick navigation |

### Git Operations Integration

#### 1. **Direct Operations from Graph**
```rust
// Inspired by GitLens command system
pub enum QuickAction {
    // Branch operations
    CheckoutBranch,
    CreateBranch,
    DeleteBranch,
    RenameBranch,

    // Commit operations
    CherryPick,
    Revert,
    Reset,
    Fixup,
    Squash,

    // Advanced operations
    InteractiveRebase,
    Merge(MergeStrategy),
    RebaseOnto,

    // Stash operations
    StashChanges,
    ApplyStash,
    PopStash,
}

impl GraphView {
    pub fn handle_input(&mut self, key: KeyEvent) -> Result<()> {
        // Update count for numeric prefix
        if let KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() && self.vim_state.mode == VimMode::Normal {
                let digit = c.to_digit(10).unwrap() as usize;
                self.vim_state.count = Some(self.vim_state.count.unwrap_or(0) * 10 + digit);
                return Ok(());
            }
        }

        match self.vim_state.mode {
            VimMode::Normal => self.handle_normal_mode(key)?,
            VimMode::Insert => self.handle_insert_mode(key)?,
            VimMode::Visual | VimMode::VisualLine | VimMode::VisualBlock => {
                self.handle_visual_mode(key)?
            }
            VimMode::Command => self.handle_command_mode(key)?,
            VimMode::Search => self.handle_search_mode(key)?,
            VimMode::Operator => self.handle_operator_pending(key)?,
        }

        // Reset count after command
        if self.vim_state.mode == VimMode::Normal {
            self.vim_state.count = None;
        }

        Ok(())
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<()> {
        let count = self.vim_state.count.unwrap_or(1);

        match key.code {
            // Navigation
            KeyCode::Char('j') => self.move_down(count),
            KeyCode::Char('k') => self.move_up(count),
            KeyCode::Char('h') => self.move_to_parent(),
            KeyCode::Char('l') => self.move_to_child(),
            KeyCode::Char('g') => {
                if self.vim_state.last_key == Some('g') {
                    self.move_to_first();
                } else {
                    self.vim_state.last_key = Some('g');
                    return Ok(());
                }
            }
            KeyCode::Char('G') => self.move_to_last(),

            // Marks
            KeyCode::Char('m') => {
                self.vim_state.mode = VimMode::Operator;
                self.vim_state.operator = Some(Operator::Mark);
            }
            KeyCode::Char('\'') => {
                self.vim_state.mode = VimMode::Operator;
                self.vim_state.operator = Some(Operator::JumpToMark);
            }

            // Visual modes
            KeyCode::Char('v') => {
                self.vim_state.mode = VimMode::Visual;
                self.vim_state.visual_anchor = Some(self.current_commit());
            }
            KeyCode::Char('V') => {
                self.vim_state.mode = VimMode::VisualLine;
                self.vim_state.visual_anchor = Some(self.current_commit());
            }
            KeyCode::Ctrl('v') => {
                self.vim_state.mode = VimMode::VisualBlock;
                self.vim_state.visual_anchor = Some(self.current_commit());
            }

            // Git operations
            KeyCode::Char('d') => {
                self.vim_state.mode = VimMode::Operator;
                self.vim_state.operator = Some(Operator::Delete);
            }
            KeyCode::Char('y') => {
                self.vim_state.mode = VimMode::Operator;
                self.vim_state.operator = Some(Operator::Yank);
            }
            KeyCode::Char('p') => self.cherry_pick_after()?,
            KeyCode::Char('P') => self.cherry_pick_before()?,

            // Command and search
            KeyCode::Char(':') => {
                self.vim_state.mode = VimMode::Command;
                self.vim_state.command_buffer.clear();
            }
            KeyCode::Char('/') => {
                self.vim_state.mode = VimMode::Search;
                self.vim_state.search_pattern.clear();
            }
            KeyCode::Char('?') => {
                self.vim_state.mode = VimMode::Search;
                self.vim_state.search_backward = true;
                self.vim_state.search_pattern.clear();
            }

            _ => {}
        }

        self.vim_state.last_key = None;
        Ok(())
    }
}
```

#### 2. **Interactive Rebase Editor with Vim Bindings**
```rust
pub struct RebaseEditor {
    commits: Vec<RebaseCommit>,
    cursor: usize,
    vim_state: VimState,

    pub fn handle_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.vim_state.mode {
            VimMode::Normal => match key.code {
                KeyCode::Char('j') => self.cursor = (self.cursor + 1).min(self.commits.len() - 1),
                KeyCode::Char('k') => self.cursor = self.cursor.saturating_sub(1),
                KeyCode::Char('p') => self.commits[self.cursor].action = RebaseAction::Pick,
                KeyCode::Char('r') => self.commits[self.cursor].action = RebaseAction::Reword,
                KeyCode::Char('e') => self.commits[self.cursor].action = RebaseAction::Edit,
                KeyCode::Char('s') => self.commits[self.cursor].action = RebaseAction::Squash,
                KeyCode::Char('f') => self.commits[self.cursor].action = RebaseAction::Fixup,
                KeyCode::Char('d') => self.commits[self.cursor].action = RebaseAction::Drop,
                KeyCode::Char('J') => self.move_commit_down(),
                KeyCode::Char('K') => self.move_commit_up(),
                KeyCode::Char('i') => self.vim_state.mode = VimMode::Insert,
                KeyCode::Char(':') => self.vim_state.mode = VimMode::Command,
                _ => {}
            },
            VimMode::Insert => match key.code {
                KeyCode::Esc => self.vim_state.mode = VimMode::Normal,
                _ => self.edit_commit_message(key),
            },
            _ => {}
        }
        Ok(())
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Mode line
        let mode_str = match self.vim_state.mode {
            VimMode::Normal => "-- NORMAL -- (p)ick (r)eword (e)dit (s)quash (f)ixup (d)rop J/K:move",
            VimMode::Insert => "-- INSERT -- (ESC to exit)",
            _ => "",
        };
        buf.set_string(area.x, area.y, mode_str, Style::default().fg(Color::Yellow));

        // Visual rebase todo editor with Vim highlights
        for (i, commit) in self.commits.iter().enumerate() {
            let action = match commit.action {
                RebaseAction::Pick => "pick",
                RebaseAction::Reword => "reword",
                RebaseAction::Edit => "edit",
                RebaseAction::Squash => "squash",
                RebaseAction::Fixup => "fixup",
                RebaseAction::Drop => "drop",
            };

            let style = if i == self.cursor {
                Style::default().bg(Color::Blue)
            } else {
                Style::default()
            };

            buf.set_string(
                area.x,
                area.y + i as u16,
                format!("{} {} {}", action, &commit.sha[..7], commit.message),
                style,
            );
        }
    }
}
```

#### 3. **Conflict Resolution UI with Vim Navigation**
```rust
pub struct ConflictResolver {
    conflicts: Vec<ConflictFile>,
    current: usize,
    vim_state: VimState,
    resolution_mode: ResolutionMode,

    pub fn handle_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Vim navigation
            KeyCode::Char('j') => self.next_conflict(),
            KeyCode::Char('k') => self.previous_conflict(),
            KeyCode::Char(']') if self.vim_state.last_key == Some(']') => self.next_section(),
            KeyCode::Char('[') if self.vim_state.last_key == Some('[') => self.previous_section(),

            // Resolution commands (Vim diff style)
            KeyCode::Char('d') => {
                if self.vim_state.last_key == Some('d') {
                    match self.vim_state.last_key2 {
                        Some('o') => self.use_ours(),     // ddo = diffget ours
                        Some('t') => self.use_theirs(),   // ddt = diffget theirs
                        Some('b') => self.use_both(),     // ddb = diffget both
                        _ => self.delete_conflict(),      // dd = delete
                    }
                }
            }

            // Vim diff navigation
            KeyCode::Char(']') if self.vim_state.last_key == Some(']') => {
                if self.vim_state.last_key2 == Some('c') {
                    self.next_change();  // ]c = next change
                }
            }
            KeyCode::Char('[') if self.vim_state.last_key == Some('[') => {
                if self.vim_state.last_key2 == Some('c') {
                    self.previous_change();  // [c = previous change
                }
            }

            // Edit mode
            KeyCode::Char('i') => self.enter_edit_mode(),
            KeyCode::Char('A') => self.append_to_resolution(),
            KeyCode::Char('o') => self.open_line_below(),
            KeyCode::Char('O') => self.open_line_above(),

            // Save and exit
            KeyCode::Char(':') => self.enter_command_mode(),
            _ => {}
        }
        Ok(())
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let conflict = &self.conflicts[self.current];

        // Vim-style three-way diff display
        let sections = area.height / 3;

        // Ours (top) - green highlight
        self.render_section(
            Rect { y: area.y, height: sections, ..area },
            buf,
            "<<<<<<< OURS (ddo)",
            &conflict.ours,
            Color::Green,
        );

        // Separator with conflict markers
        buf.set_string(
            area.x,
            area.y + sections,
            "======= CONFLICT =======",
            Style::default().fg(Color::Yellow),
        );

        // Theirs (middle) - blue highlight
        self.render_section(
            Rect { y: area.y + sections + 1, height: sections - 1, ..area },
            buf,
            ">>>>>>> THEIRS (ddt)",
            &conflict.theirs,
            Color::Blue,
        );

        // Resolution (bottom) - current edit
        self.render_section(
            Rect { y: area.y + sections * 2, height: sections, ..area },
            buf,
            "RESOLUTION (edit with 'i')",
            &conflict.resolution,
            Color::White,
        );

        // Vim mode line
        let mode_line = format!(
            "[{}/{}] {} -- {} -- ]c/[c:navigate ddo/ddt/ddb:resolve",
            self.current + 1,
            self.conflicts.len(),
            conflict.path,
            match self.vim_state.mode {
                VimMode::Normal => "NORMAL",
                VimMode::Insert => "INSERT",
                VimMode::Command => "COMMAND",
                _ => "CONFLICT",
            }
        );

        buf.set_string(
            area.x,
            area.y + area.height - 1,
            mode_line,
            Style::default().bg(Color::DarkGray),
        );
    }
}
```

### Real-time Updates System

#### 1. **File System Watcher**
```rust
use notify::{Watcher, RecursiveMode, watcher};

pub struct GitWatcher {
    watcher: Box<dyn Watcher>,
    tx: mpsc::Sender<GraphEvent>,

    pub fn watch_repository(&mut self, repo_path: &Path) -> Result<()> {
        let git_dir = repo_path.join(".git");
        self.watcher.watch(&git_dir, RecursiveMode::Recursive)?;
        Ok(())
    }

    fn handle_event(&self, event: notify::Event) {
        match event.kind {
            EventKind::Modify(_) => {
                if event.paths.iter().any(|p| p.ends_with("HEAD")) {
                    self.tx.send(GraphEvent::BranchChanged).unwrap();
                }
                if event.paths.iter().any(|p| p.ends_with("index")) {
                    self.tx.send(GraphEvent::WorkingTreeChanged).unwrap();
                }
            }
            _ => {}
        }
    }
}
```

#### 2. **Incremental Graph Updates**
```rust
pub struct IncrementalGraphBuilder {
    cache: GraphCache,

    pub fn update_incrementally(&mut self, changes: &[Change]) -> GitGraph {
        // Only recompute affected parts
        for change in changes {
            match change {
                Change::NewCommit(sha) => {
                    self.add_commit_to_graph(sha);
                }
                Change::BranchMoved(name, new_target) => {
                    self.update_branch_position(name, new_target);
                }
                Change::RefDeleted(name) => {
                    self.remove_ref_from_graph(name);
                }
            }
        }

        self.cache.get_graph()
    }
}
```

### Advanced Search and Filter

#### 1. **Multi-criteria Search**
```rust
pub struct SearchEngine {
    index: SearchIndex,

    pub fn search(&self, query: &SearchQuery) -> SearchResults {
        match query {
            SearchQuery::Text(pattern) => self.search_text(pattern),
            SearchQuery::Author(name) => self.search_author(name),
            SearchQuery::Date(range) => self.search_date_range(range),
            SearchQuery::File(path) => self.search_file_changes(path),
            SearchQuery::Combined(queries) => self.search_combined(queries),
        }
    }

    pub fn highlight_results(&self, graph: &mut GitGraph, results: &SearchResults) {
        for node in &mut graph.nodes {
            if results.matches.contains(&node.id) {
                node.highlight = true;
                node.highlight_color = Color::Yellow;
            }
        }
    }
}
```

#### 2. **Smart Filters**
```rust
pub struct FilterManager {
    active_filters: Vec<Filter>,

    pub fn apply_filters(&self, graph: &mut GitGraph) {
        for filter in &self.active_filters {
            match filter {
                Filter::HideStale(days) => {
                    self.hide_stale_branches(graph, *days);
                }
                Filter::ShowOnlyBranch(name) => {
                    self.filter_to_branch(graph, name);
                }
                Filter::HideMergeCommits => {
                    self.hide_merge_commits(graph);
                }
                Filter::ShowOnlyMyCommits(author) => {
                    self.filter_by_author(graph, author);
                }
            }
        }
    }
}
```

### Performance Optimizations

#### 1. **Virtual Scrolling**
```rust
pub struct VirtualScroller {
    total_items: usize,
    viewport_size: usize,
    scroll_position: usize,
    rendered_items: HashMap<usize, RenderedItem>,

    pub fn get_visible_items(&self) -> Vec<&RenderedItem> {
        let start = self.scroll_position;
        let end = (start + self.viewport_size).min(self.total_items);

        (start..end)
            .filter_map(|i| self.rendered_items.get(&i))
            .collect()
    }

    pub fn prerender_nearby(&mut self) {
        // Prerender items just outside viewport
        let prerender_range = 10;
        let start = self.scroll_position.saturating_sub(prerender_range);
        let end = (self.scroll_position + self.viewport_size + prerender_range)
            .min(self.total_items);

        for i in start..end {
            if !self.rendered_items.contains_key(&i) {
                self.render_item(i);
            }
        }
    }
}
```

#### 2. **Lazy Loading with Pagination**
```rust
pub struct PaginatedGraph {
    loaded_ranges: Vec<Range<usize>>,
    total_commits: usize,
    page_size: usize,

    pub async fn load_page(&mut self, page: usize) -> Result<Vec<GraphNode>> {
        let start = page * self.page_size;
        let end = ((page + 1) * self.page_size).min(self.total_commits);

        if !self.is_loaded(start..end) {
            let nodes = self.fetch_commits(start, end).await?;
            self.loaded_ranges.push(start..end);
            self.merge_nodes(nodes);
        }

        Ok(self.get_nodes(start..end))
    }
}
```

### UI Components Hierarchy

```
GraphView (Main Container)
├── HeaderBar
│   ├── RepositorySelector
│   ├── BranchSelector
│   └── SearchBar
├── GraphCanvas
│   ├── GraphRenderer
│   ├── VirtualScroller
│   └── InteractionLayer
├── SidePanel (Collapsible)
│   ├── CommitDetails
│   ├── FileChanges
│   └── Actions
└── StatusBar
    ├── Statistics
    ├── CurrentOperation
    └── KeyHints
```

## 📋 Enhanced Implementation Plan with Vim Focus

### Phase 1: Core Infrastructure with Vim Foundation (Week 1)
- [x] Basic graph data structure
- [ ] **Vim mode state machine**
- [ ] **Register system implementation**
- [ ] **Motion and text object engine**
- [ ] Event system implementation
- [ ] File system watcher
- [ ] Operations manager
- [ ] Incremental update system

### Phase 2: Vim Modal System (Week 2)
- [ ] **Normal mode with full navigation**
- [ ] **Visual mode (char/line/block)**
- [ ] **Insert mode for text input**
- [ ] **Command mode with palette**
- [ ] **Search mode with patterns**
- [ ] **Operator-pending mode**
- [ ] **Marks and jumps system**
- [ ] **Macro recording/playback**

### Phase 3: Interactive Features with Vim Integration (Week 2-3)
- [ ] Multi-selection support via Visual mode
- [ ] **Vim-style context menus**
- [ ] **Quick actions with g-prefix**
- [ ] **Text objects for Git entities**
- [ ] **Comprehensive Vim shortcuts**
- [ ] **Split window management**

### Phase 4: Git Operations with Vim Commands (Week 3)
- [ ] **Merge UI with :merge command**
- [ ] **Interactive rebase with Vim bindings**
- [ ] **Cherry-pick with motions**
- [ ] **Conflict resolution with diff commands**
- [ ] **Stash management via :stash**
- [ ] **Quickfix integration for errors**

### Phase 5: Advanced Vim Features (Week 3-4)
- [ ] **Custom text objects (commits, branches, hunks)**
- [ ] **Git-specific operators**
- [ ] **Folds for branch visualization**
- [ ] **Vim script configuration**
- [ ] **Plugin system for extensions**
- [ ] Search engine with `/` and `?`
- [ ] Smart filters with `:filter`
- [ ] Virtual scrolling optimization
- [ ] Lazy loading with pagination
- [ ] Performance monitoring

### Phase 6: Polish & Integration (Week 4)
- [ ] **Vim help system (:help)**
- [ ] **Customizable keybindings via :map**
- [ ] **Vimrc-style configuration**
- [ ] **Session management**
- [ ] **Undo tree visualization**
- [ ] Animations and transitions
- [ ] Theme customization
- [ ] Settings persistence
- [ ] Testing and optimization

## 🚀 Key Innovations

### 1. **Terminal-Native Vim Experience**
- Full Vim modal editing system
- Complete motion and operator support
- Text objects for Git entities
- Macro recording and playback
- Register system with Git integration
- Marks for quick navigation

### 2. **Git-Specific Vim Extensions**
- Custom operators for Git operations (gm=merge, gr=rebase, gc=cherry-pick)
- Text objects for commits (ic/ac), branches (ib/ab), hunks (ih/ah)
- Git-aware folds and sections
- Specialized diff mode commands
- Interactive rebase with Vim bindings

### 3. **Real-time Collaboration Features**
- Live updates when others push
- Conflict prevention warnings
- Team activity indicators
- Vim-style collaborative editing

### 4. **AI-Assisted Operations**
- Natural language search via `:ai` command
- Intelligent conflict resolution suggestions
- Commit message generation with `:ai-commit`
- Smart command completion

### 5. **Performance Targets**
- < 50ms graph render for 1000 commits
- < 10ms scroll response
- < 100ms search results
- < 1s initial load for large repos
- Instant Vim command response

## 🎯 Success Metrics

### 1. **Usability**
- All operations accessible within 3 keystrokes
- Context-aware suggestions
- Minimal cognitive load
- **Full Vim muscle memory support**
- **Seamless modal transitions**

### 2. **Performance**
- Smooth 60 FPS scrolling
- Instant search results
- Background operation support
- **Zero-latency Vim commands**
- **Efficient macro playback**

### 3. **Reliability**
- Graceful error handling
- Undo/redo for all operations
- Data integrity guarantees
- **Robust state management**
- **Crash recovery with Vim session**

### 4. **Vim Compliance**
- **90%+ standard Vim commands supported**
- **Compatible motion system**
- **Full operator coverage**
- **Register compatibility**
- **Macro system parity**

This enhanced plan incorporates a complete Vim-style modal system inspired by Vim and Neovim, providing a familiar and powerful interface for Git operations in the terminal. The system maintains the sophisticated event-driven architecture and interactive features while adding comprehensive Vim functionality for efficient keyboard-driven workflows.