# Terminal UI Enhancement Plan - Git Graph Visualization

## 📊 Analysis of Reference Projects

### GitUp (macOS Native)
**Key Features:**
1. **Graph Visualization Architecture**
   - Uses `GIGraph` class for commit graph generation
   - Implements layers, nodes, lines, and branches
   - Supports various display options (virtual tips, stale branches, standalone tags)
   - Optimized for performance with position computation and color assignment

2. **Visual Components**
   - **Nodes**: Represent commits with parent-child relationships
   - **Lines**: Connect commits to show relationships
   - **Branches**: Visual representation of branch paths
   - **Layers**: Horizontal arrangement of parallel development lines

3. **Display Options**
   - Show/hide virtual branch tips
   - Skip stale branches (based on time)
   - Skip standalone tags
   - Skip standalone remote branches
   - Preserve upstream remote branch tips

### VSCode GitLens
**Key Features:**
1. **Graph Data Model**
   - Uses `GitGraph` interface with rows, avatars, stats
   - Supports different node types (commit, merge, stash, work-dir-changes)
   - Implements streaming/pagination for large repositories

2. **Rich Metadata**
   - Avatars for contributors
   - Branch/tag decorations
   - Remote information
   - Worktree integration
   - Stats (additions/deletions)

3. **Interactive Features**
   - Hover information
   - Context menus
   - Search and filtering
   - Collapsible sections

## 🎯 Terminal UI Enhancement Design

### Core Concepts for Terminal Graph Visualization

#### 1. ASCII Art Graph Rendering
```
* [main] Latest commit
|\
| * [feature] Feature work
| * Another feature commit
|/
* Merge commit
|\
| * [bugfix] Bug fix
|/
* Base commit
```

#### 2. Color Coding System
- **Branches**: Different colors for each branch line
- **HEAD**: Special highlight color
- **Tags**: Distinct color/symbol
- **Stash**: Different symbol/color
- **Remote branches**: Italics or different shade

#### 3. Compact vs Expanded Views
- **Compact**: One line per commit with essential info
- **Expanded**: Multi-line with full commit message, author, date

## 🏗️ Proposed Architecture

### Module Structure
```
gitup-ui/
├── src/
│   ├── tui.rs           # Main TUI orchestrator
│   ├── graph/           # Graph visualization module
│   │   ├── mod.rs       # Graph module interface
│   │   ├── builder.rs   # Graph construction logic
│   │   ├── renderer.rs  # ASCII rendering engine
│   │   ├── layout.rs    # Layout calculations
│   │   └── style.rs     # Color and style definitions
│   ├── components/      # UI components
│   │   ├── graph_view.rs    # Graph display component
│   │   ├── commit_list.rs   # Commit list view
│   │   ├── branch_view.rs   # Branch management
│   │   ├── diff_view.rs     # Diff viewer
│   │   └── status_view.rs   # Working directory status
│   └── widgets/         # Custom widgets
│       ├── scrollable.rs    # Enhanced scrolling
│       ├── searchable.rs    # Search functionality
│       └── interactive.rs   # Interactive elements
```

### Data Structures

```rust
// Graph representation
pub struct GitGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub lanes: Vec<Lane>,
    pub layout: GraphLayout,
}

pub struct GraphNode {
    pub id: String,           // Commit SHA
    pub position: Position,   // (x, y) in graph
    pub commit: CommitInfo,    // Commit details
    pub refs: Vec<RefInfo>,    // Branches, tags
    pub node_type: NodeType,   // Regular, Merge, Branch point
    pub symbol: char,          // Visual symbol (* o + etc)
}

pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub lane: usize,
    pub edge_type: EdgeType,  // Parent, Merge
    pub style: LineStyle,     // Straight, Curved
}

pub struct Lane {
    pub id: usize,
    pub color: Color,
    pub active_ranges: Vec<Range>,
}

pub enum NodeType {
    Regular,
    Merge,
    Branch,
    Initial,
    Stash,
    WorkingDirectory,
}

pub struct RefInfo {
    pub ref_type: RefType,    // Branch, Tag, Remote
    pub name: String,
    pub color: Color,
    pub is_head: bool,
}
```

### Rendering Algorithm

```rust
// Simplified rendering logic
fn render_graph(graph: &GitGraph, area: Rect, buf: &mut Buffer) {
    for (y, node) in graph.nodes.iter().enumerate() {
        // Draw node symbol
        let x = node.position.x * 2; // 2 chars spacing
        buf.set_string(x, y, &node.symbol.to_string(), node.style);

        // Draw refs (branches/tags)
        if !node.refs.is_empty() {
            let refs_str = format_refs(&node.refs);
            buf.set_string(x + 3, y, &refs_str, ref_style);
        }

        // Draw commit message
        let msg = truncate_message(&node.commit.message, area.width - x - 20);
        buf.set_string(x + 20, y, &msg, Style::default());

        // Draw edges to parents
        for edge in &node.edges {
            draw_edge(buf, edge, &graph.lanes);
        }
    }
}
```

## 📋 Implementation Plan

### Phase 1: Core Graph Data Structure (Week 1)
1. **Graph Builder Module**
   - [ ] Implement commit graph traversal
   - [ ] Build node and edge relationships
   - [ ] Calculate lane assignments
   - [ ] Handle merge commits and branches

2. **Layout Engine**
   - [ ] Position calculation for nodes
   - [ ] Lane management for parallel branches
   - [ ] Collision detection and resolution
   - [ ] Optimize for terminal constraints

### Phase 2: ASCII Rendering (Week 1-2)
1. **Symbol System**
   ```
   * = Regular commit
   ◉ = HEAD commit
   ⊗ = Merge commit
   ○ = Branch point
   ⊙ = Tagged commit
   $ = Stashed changes
   ? = Working directory
   ```

2. **Line Drawing**
   ```
   │ = Vertical line
   ─ = Horizontal line
   ╱ = Diagonal up
   ╲ = Diagonal down
   ├ = Branch start
   ┤ = Branch merge
   ```

3. **Color Scheme**
   - Main branch: Cyan
   - Feature branches: Green, Yellow, Magenta (rotating)
   - Remote branches: Blue
   - Tags: Yellow background
   - HEAD: Bold white
   - Stash: Gray

### Phase 3: Interactive Features (Week 2)
1. **Navigation**
   - [ ] Keyboard navigation (j/k, arrows)
   - [ ] Jump to branch/tag
   - [ ] Search commits
   - [ ] Fold/unfold branches

2. **Actions**
   - [ ] Checkout from graph
   - [ ] Create branch from node
   - [ ] Cherry-pick from graph
   - [ ] Rebase visualization

3. **Information Display**
   - [ ] Hover/select for details
   - [ ] Diff preview
   - [ ] Author/date info
   - [ ] Stats (additions/deletions)

### Phase 4: Advanced Visualization (Week 2-3)
1. **Multiple Views**
   - **Compact Graph**: One-line commits
   - **Detailed Graph**: Multi-line with full info
   - **Branch-focused**: Highlight specific branch
   - **Time-based**: Chronological layout

2. **Filtering**
   - [ ] By author
   - [ ] By date range
   - [ ] By branch
   - [ ] By file path

3. **Performance Optimization**
   - [ ] Virtual scrolling for large repos
   - [ ] Lazy loading of commit details
   - [ ] Caching of graph calculations
   - [ ] Incremental rendering

### Phase 5: Integration (Week 3)
1. **UI Integration**
   - [ ] Add graph as new tab
   - [ ] Sync with other views
   - [ ] Status bar information
   - [ ] Help overlay

2. **Git Operations**
   - [ ] Real-time updates on changes
   - [ ] Show ongoing operations
   - [ ] Conflict visualization
   - [ ] Rebase/merge progress

## 🎨 UI Mockups

### Compact View
```
┌─ GitUp Graph ────────────────────────────────────────────────┐
│ ◉ [HEAD -> main] 3d794a6 Implement merge operations         │
│ │ John Doe, 2 hours ago                                      │
│ * 728bc17 Implement stash operations                         │
│ ├─⊙ [v0.1.0] Initial release                                │
│ │ * [feature/ui] a5c3d1f Add color support                  │
│ │ * 9b2e4f8 Improve scrolling                               │
│ ├─┤ Merge feature/ui into main                              │
│ * │ 6687b5f Fix SSH authentication                          │
│ │ │ * [origin/feature/api] 4d5e6f7 Add API endpoints       │
│ ├─┴─┤ Merge origin/feature/api                              │
│ * 2a3b4c5 Initial commit                                     │
└───────────────────────────────────────────────────────────────┘
[Tab] Switch View | [/] Search | [b] Branches | [Enter] Details
```

### Detailed View
```
┌─ GitUp Graph (Detailed) ─────────────────────────────────────┐
│ ◉ 3d794a6 ────────────────────────────────────────────────  │
│ │ Author: John Doe <john@example.com>                       │
│ │ Date:   2025-09-15 10:30:45 +0800                        │
│ │ Refs:   HEAD -> main, origin/main                         │
│ │                                                            │
│ │ Implement merge operations                                │
│ │                                                            │
│ │ - Created comprehensive merge module                       │
│ │ - Added fast-forward and normal merge                     │
│ │ - Implemented conflict detection                          │
│ │                                                            │
│ │ 12 files changed, 544 insertions(+), 2 deletions(-)      │
│ │                                                            │
│ * 728bc17 ────────────────────────────────────────────────  │
│ │ Author: John Doe <john@example.com>                       │
│ │ Date:   2025-09-15 09:15:30 +0800                        │
│ │                                                            │
│ │ Implement stash operations                                │
│ │                                                            │
└───────────────────────────────────────────────────────────────┘
[Tab] Compact | [d] Diff | [c] Checkout | [Space] Select
```

## 🔧 Technical Considerations

### Performance
1. **Large Repositories**
   - Limit initial graph depth (e.g., 500 commits)
   - Load more on demand
   - Use virtual scrolling
   - Cache computed layouts

2. **Real-time Updates**
   - Watch for .git changes
   - Incremental graph updates
   - Debounce rapid changes

### Terminal Constraints
1. **Limited Colors**
   - Fallback for 8-color terminals
   - Use different symbols when colors unavailable
   - ASCII-only mode for compatibility

2. **Character Width**
   - Handle Unicode properly
   - Account for emoji in commit messages
   - CJK character width handling

### User Experience
1. **Responsive Design**
   - Adapt to terminal resize
   - Minimum width requirements
   - Graceful degradation

2. **Accessibility**
   - Screen reader friendly output
   - Keyboard-only navigation
   - High contrast mode

## 📈 Success Metrics

1. **Performance**
   - Graph generation < 100ms for 500 commits
   - Smooth scrolling at 60 FPS
   - Memory usage < 50MB for typical repo

2. **Usability**
   - All operations accessible via keyboard
   - Clear visual hierarchy
   - Intuitive navigation

3. **Functionality**
   - Support all common Git workflows
   - Real-time synchronization
   - Accurate graph representation

## 🎯 MVP Features

### Must Have (Week 1)
- Basic graph rendering with ASCII art
- Commit nodes and edges
- Branch/tag labels
- Scrolling navigation
- Current implementation integration

### Should Have (Week 2)
- Colors for branches
- Interactive selection
- Basic filtering
- Compact/expanded toggle

### Nice to Have (Week 3+)
- Advanced filtering
- Search functionality
- Diff preview
- Performance optimizations
- Multiple layout algorithms

## 🚀 Next Steps

1. **Prototype Development**
   - Create basic graph builder
   - Implement ASCII renderer
   - Test with real repositories

2. **User Testing**
   - Gather feedback on visualization
   - Iterate on design
   - Performance profiling

3. **Integration**
   - Merge with existing TUI
   - Add configuration options
   - Documentation

This plan provides a comprehensive roadmap for implementing Git graph visualization in the terminal UI, taking inspiration from both GitUp and GitLens while adapting to terminal constraints.