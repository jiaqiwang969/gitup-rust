# GitUp-RS

A Rust implementation of GitUp - A fast Git client for macOS.

## Overview

This project is a phased migration of GitUp from Objective-C to Rust, aiming to:
- Improve performance with Rust's zero-cost abstractions
- Enhance memory safety
- Enable cross-platform support
- Maintain compatibility with existing GitUp features

## Project Structure

```
gitup-rust/
├── gitup-core/     # Core Git operations library
├── gitup-ui/       # UI layer (Terminal UI, future GUI)
├── gitup-ffi/      # Foreign Function Interface for Objective-C integration
└── src/            # Main CLI application
```

## Features

### Completed ✅
- Repository management (open, init, status)
- Branch operations (list, create, checkout)
- Commit history viewing
- Diff operations (working dir, staged, commits)
- CLI interface

### In Progress 🚧
- Commit creation
- Staging/unstaging files
- Terminal UI with ratatui

### Planned 📋
- Merge and rebase operations
- Conflict resolution
- Tag management
- Stash operations
- macOS native UI integration

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test --all
```

## Usage

```bash
# Check repository status
gitup status

# View commit history
gitup log --count 10

# Show diff
gitup diff
gitup diff --stat
gitup diff --staged
gitup diff --commit <hash>

# List branches
gitup branches
```

## Migration Plan

See [MIGRATION_PLAN.md](MIGRATION_PLAN.md) for detailed migration strategy from Objective-C to Rust.

## License

This project maintains compatibility with the original GitUp license.