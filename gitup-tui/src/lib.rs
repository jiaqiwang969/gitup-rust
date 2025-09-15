//! GitUp Terminal UI with Vim-style modal interface
//!
//! This module provides a terminal user interface for Git operations
//! with comprehensive Vim keybindings and modal editing support.

pub mod vim;
pub mod event;
pub mod watcher;
pub mod operations;

// Re-export main types
pub use vim::{
    VimHandler,
    VimMode,
    VimAction,
    Motion,
    Operator,
    RegisterManager,
    MarkManager,
    MacroRecorder,
    GitTextObject,
    CommandPalette,
    VimConfig,
};

pub use event::{
    GraphEvent,
    EventBus,
    EventHandler,
    EventLogger,
    EventFilter,
};

pub use watcher::{
    GitWatcher,
    EventDebouncer,
};

pub use operations::{
    OperationsManager,
    Operation,
    OperationResult,
    ResetMode,
    MergeStrategy,
};