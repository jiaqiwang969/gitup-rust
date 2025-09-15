# GitUp Terminal UI - TODO Tracker

## Code TODOs from Phase 1

### vim/state.rs
1. **Line 258**: Implement undo operation
   - Connect to OperationsManager::undo()
   - Return appropriate VimAction

2. **Line 260**: Implement redo operation
   - Connect to OperationsManager::redo()
   - Return appropriate VimAction

3. **Line 313**: Implement delete operation in visual mode
   - Get selection range
   - Apply delete to selected commits
   - Update registers

4. **Line 320**: Implement yank operation in visual mode
   - Get selection range
   - Copy commit SHAs to register
   - Exit visual mode

5. **Line 327**: Implement change operation in visual mode
   - Get selection range
   - Delete and enter insert mode
   - Update registers

6. **Line 401**: Implement mark setting
   - Call MarkManager::set_local_mark() or set_global_mark()
   - Store current position and commit SHA

7. **Line 409**: Implement jump to mark
   - Call MarkManager::jump_to_mark()
   - Update cursor position
   - Add to jump list

### vim/motion.rs
8. **Line 185**: Implement remaining motions
   - ParagraphForward/Backward
   - SectionForward/Backward
   - FindChar/TillChar motions
   - Text object motions (InnerWord, AroundWord, etc.)
   - Git-specific motions (NextBranch, PrevBranch, NextMerge, etc.)

## Phase 2 Implementation Plan

### Week 1: Graph Rendering Core
- [ ] Create GraphRenderer component
- [ ] Implement ASCII art symbols for commits
- [ ] Build lane management for parallel branches
- [ ] Add branch/tag decorators
- [ ] Implement color system

### Week 2: Vim Integration
- [ ] Fix all Phase 1 TODOs
- [ ] Connect VimHandler to graph view
- [ ] Implement motion context for graph
- [ ] Add visual selection rendering
- [ ] Enable operator-motion combinations

### Week 3: Interactive Features
- [ ] Implement quick actions (g-prefix commands)
- [ ] Add context menus
- [ ] Enable multi-selection
- [ ] Implement search highlighting
- [ ] Add fold/expand for branches

### Week 4: Git Operations
- [ ] Connect operations to actual git2 implementation
- [ ] Implement interactive rebase UI
- [ ] Add conflict resolution interface
- [ ] Enable cherry-pick/revert from graph
- [ ] Add stash management UI

## Priority Order

### High Priority (Block Phase 2)
1. Fix visual mode operations (delete/yank/change)
2. Implement mark setting/jumping
3. Complete motion implementations for graph navigation

### Medium Priority (Enhance usability)
4. Implement undo/redo
5. Add operator-motion combinations
6. Complete text object implementations

### Low Priority (Nice to have)
7. Advanced motions (paragraph, section)
8. Find/till character motions
9. Macro improvements

## Implementation Notes

### For Visual Mode Operations
```rust
// In state.rs handle_visual_key()
KeyCode::Char('d') => {
    let selection = self.get_selection();
    // Get commit SHAs in range
    let commits = context.get_commits_in_range(selection);
    // Store in register
    registers.delete(self.register, RegisterContent::Commits(commits));
    self.mode = VimMode::Normal;
    self.visual_anchor = None;
    Ok(VimAction::GitOp(GitOperation::Drop(commits)))
}
```

### For Mark Operations
```rust
// In state.rs handle_operator_key()
Operator::Mark => {
    if let KeyCode::Char(c) = key.code {
        marks.set_local_mark(c, self.cursor, context.get_current_commit_sha());
        VimAction::None
    }
}
```

### For Motion Context
```rust
// Create GraphMotionContext implementing MotionContext trait
impl MotionContext for GraphMotionContext {
    fn next_commit(&self, from: Position) -> Option<Position> {
        // Find next commit in graph
        self.graph.find_next_commit(from)
    }
    // ... other methods
}
```