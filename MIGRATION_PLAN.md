# GitUp to Rust Migration Plan

## 📊 Current Status
- **Progress**: Phase 2 - Incremental UI Migration (30% complete)
- **Last Updated**: 2025-09-14
- **Status**: 🟢 Active Development

### Completed Features
#### Phase 1: Core Git Operations ✅
- ✅ Basic repository operations (open, init, status)
- ✅ Branch listing and management
- ✅ Commit history retrieval
- ✅ CLI tool with basic commands
- ✅ FFI bridge foundation
- ✅ Project structure and build system
- ✅ Diff operations (all types)
- ✅ Commit operations (stage, commit, amend)

#### Phase 2: UI Migration 🚧
- ✅ **Terminal UI (ratatui)** (NEW - Enhanced)
  - Interactive 4-tab interface
  - Commit history browser
  - Branch management
  - Working directory status
  - Diff viewer with syntax highlighting
  - Keyboard navigation
  - Real-time updates
  - **Improved scroll functionality** (NEW)
    - Proper line count tracking
    - Multiple scroll methods (arrows, j/k, Page Up/Down, Home/End)
    - Scrollbar with position indicator
    - Current position display in title bar

### Next Steps
1. Implement merge and rebase operations
2. Add tag and stash management
3. Build Objective-C wrapper classes
4. Create native macOS UI integration

## Overview
This document outlines the phased migration strategy for porting GitUp from Objective-C to Rust.

## Architecture Overview

### Current GitUp Architecture (Objective-C)
```
GitUp (macOS App)
    ├── GitUpKit (Framework)
    │   ├── Core (Git operations via libgit2)
    │   ├── Extensions (Convenience features)
    │   ├── Interface (Low-level views)
    │   ├── Utilities (Helper classes)
    │   ├── Components (Single-view controllers)
    │   └── Views (Multi-view controllers)
    └── Application (Main app)
```

### Target Rust Architecture
```
gitup-rs (Main binary)
    ├── gitup-core (Git operations)
    │   ├── repository (Repo management)
    │   ├── commits (Commit operations)
    │   ├── branches (Branch management)
    │   ├── diff (Diff computation)
    │   └── history (Graph/history)
    ├── gitup-ui (UI layer)
    │   ├── native (macOS native UI)
    │   ├── web (Tauri-based UI)
    │   └── terminal (TUI fallback)
    └── gitup-ffi (Foreign Function Interface)
        ├── objc_bridge (Objective-C compatibility)
        └── c_api (C API for plugins)
```

## Migration Phases

### Phase 0: Setup & Foundation (✅ Completed)
- [x] Setup Rust project structure
- [x] Configure workspace and dependencies
- [x] Create basic build system
- [x] Setup FFI bridge framework
- [x] Implement basic repository operations
- [x] Create CLI tool with basic commands

### Phase 1: Core Git Operations (Weeks 1-2)
Replace GitUpKit/Core with Rust implementation while keeping Objective-C UI.

#### Tasks:
1. **Repository Management** ✅
   - [x] Implement GCRepository equivalent in Rust
   - [x] Create FFI bindings for repository operations
   - [x] Add repository discovery and validation

2. **Basic Git Operations** ✅ Completed
   - [x] List branches
   - [x] Get commit history
   - [x] Diff operations
   - [x] Implement commit operations
   - [x] Stage/unstage files
   - [ ] Create tag handling
   - [ ] Implement stash operations

3. **Advanced Features**
   - [x] Diff computation
   - [ ] History graph generation
   - [ ] Conflict resolution
   - [ ] Rebase engine

4. **Integration**
   - [x] Create basic FFI bridge structure
   - [ ] Create Objective-C wrapper classes
   - [ ] Replace Core folder calls with Rust FFI calls
   - [ ] Maintain API compatibility

### Phase 2: Incremental UI Migration 🚧 In Progress (25% complete)
Start replacing UI components with Rust alternatives.

#### Tasks:
1. **Terminal UI (Quick Win)** ✅
   - [x] Create CLI version using ratatui
   - [x] Implement basic navigation
   - [x] Add commit/branch visualization
   - [x] Add diff viewer
   - [x] Keyboard shortcuts and commands

2. **Web-based UI (Tauri)**
   - [ ] Setup Tauri project
   - [ ] Create main window structure
   - [ ] Implement repository view
   - [ ] Add diff viewer
   - [ ] Create commit interface

3. **Native macOS Widgets**
   - [ ] Use objc crate for native integration
   - [ ] Create custom views in Rust
   - [ ] Integrate with existing Objective-C UI

### Phase 3: Complete Migration (Weeks 5-6)
Full replacement of Objective-C code with Rust.

#### Tasks:
1. **Remove Objective-C Dependencies**
   - [ ] Replace all GitUpKit components
   - [ ] Port Application folder to Rust
   - [ ] Remove Xcode project files

2. **Performance Optimization**
   - [ ] Profile and optimize hot paths
   - [ ] Implement parallel operations
   - [ ] Add caching layers

3. **Testing & Polish**
   - [ ] Comprehensive test suite
   - [ ] UI/UX refinement
   - [ ] Documentation

## Implementation Strategy

### 1. Bottom-up Approach
Start with core Git operations and work up to UI:
- Core Git functionality first
- Then UI components
- Finally, application shell

### 2. Parallel Development
Keep existing GitUp working while building Rust version:
- Use FFI to gradually replace components
- Test each component in isolation
- Maintain backward compatibility

### 3. Testing Strategy
- Unit tests for each Rust module
- Integration tests for FFI bridge
- End-to-end tests comparing with original GitUp
- Performance benchmarks

## Technical Decisions

### Git Library Choice
- **Primary**: git2-rs (libgit2 bindings) - familiar, mature
- **Alternative**: gitoxide - pure Rust, better performance
- **Strategy**: Start with git2-rs, migrate to gitoxide later

### UI Framework
- **Phase 1**: FFI bridge to existing Objective-C UI
- **Phase 2**: Tauri for cross-platform web-based UI
- **Phase 3**: Native macOS using objc crate or SwiftUI bridge

### FFI Strategy
- Use safer-ffi for type-safe bindings
- Generate C headers with cbindgen
- Create Objective-C wrapper classes

## File Structure

```
gitup-rust/
├── Cargo.toml
├── gitup-core/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── repository.rs
│   │   ├── commit.rs
│   │   ├── branch.rs
│   │   ├── diff.rs
│   │   └── history.rs
│   └── tests/
├── gitup-ui/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── native/
│   │   ├── web/
│   │   └── terminal/
│   └── assets/
├── gitup-ffi/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── objc_bridge.rs
│   │   └── c_api.rs
│   ├── include/
│   │   └── gitup.h
│   └── build.rs
└── src/
    └── main.rs
```

## Next Steps

1. **Immediate Actions**
   - [ ] Create basic repository struct in gitup-core
   - [ ] Implement simple Git operations (status, log)
   - [ ] Setup FFI bridge with a simple example
   - [ ] Create Objective-C test harness

2. **Week 1 Goals**
   - Complete basic repository operations
   - FFI bridge working with GitUpKit
   - Replace one Core class with Rust

3. **Success Metrics**
   - Performance: Equal or better than original
   - Memory usage: Lower than Objective-C version
   - API compatibility: Drop-in replacement
   - Test coverage: >80%

## Commands

### Build
```bash
# Build all
cargo build --release

# Build specific workspace member
cargo build -p gitup-core --release

# Generate C headers
cargo build -p gitup-ffi --release
cbindgen --config gitup-ffi/cbindgen.toml --crate gitup-ffi --output include/gitup.h
```

### Test
```bash
# Run all tests
cargo test --all

# Run with coverage
cargo tarpaulin --all-features
```

### Integration
```bash
# Build FFI library
cargo build -p gitup-ffi --release

# Copy to Xcode project
cp target/release/libgitup_ffi.a GitUp/GitUpKit/Third-Party/

# Update Xcode project to link with libgitup_ffi.a
```

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| FFI complexity | High | Start with simple functions, gradually increase complexity |
| Performance regression | Medium | Benchmark early and often |
| UI framework limitations | Medium | Keep multiple UI options open |
| Memory management issues | High | Use safer-ffi, extensive testing |
| API incompatibility | High | Maintain compatibility layer |

## Resources

- [git2-rs documentation](https://docs.rs/git2)
- [gitoxide documentation](https://docs.rs/gitoxide)
- [Tauri guides](https://tauri.app/guides/)
- [safer-ffi tutorial](https://github.com/getditto/safer_ffi)
- [Objective-C interop](https://docs.rs/objc/)