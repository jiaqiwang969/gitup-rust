use std::collections::VecDeque;
use std::sync::mpsc::{Sender, Receiver, channel};
use anyhow::Result;

/// Event types for the Git graph visualization
#[derive(Debug, Clone)]
pub enum GraphEvent {
    // Repository events
    RepositoryChanged(RepositoryChange),
    BranchChanged(String),
    WorkingTreeChanged,
    CommitAdded(String),
    RefUpdated(RefUpdate),

    // User interactions
    NodeSelected(String),
    NodeActivated(String), // Double-click equivalent
    ContextMenuRequested(Position),

    // Graph operations
    SearchInitiated(String),
    FilterApplied(FilterCriteria),
    ViewModeChanged(ViewMode),

    // Vim events
    VimModeChanged(super::VimMode),
    VimCommandExecuted(String),
    VimMacroRecorded(char),
    VimMarkSet(char, Position),

    // UI events
    WindowResized(u16, u16),
    ScrollPositionChanged(usize),
    FocusChanged(bool),
}

#[derive(Debug, Clone)]
pub struct RepositoryChange {
    pub change_type: RepositoryChangeType,
    pub path: String,
}

#[derive(Debug, Clone)]
pub enum RepositoryChangeType {
    Fetch,
    Pull,
    Push,
    Merge,
    Rebase,
    Reset,
    Stash,
}

#[derive(Debug, Clone)]
pub struct RefUpdate {
    pub ref_name: String,
    pub old_oid: Option<String>,
    pub new_oid: String,
}

#[derive(Debug, Clone)]
pub struct FilterCriteria {
    pub author: Option<String>,
    pub date_range: Option<(String, String)>,
    pub branch: Option<String>,
    pub message_pattern: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ViewMode {
    Compact,
    Detailed,
    Chronological,
    Topological,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

/// Event handler trait
pub trait EventHandler: Send {
    fn handle_event(&mut self, event: &GraphEvent) -> Result<()>;
}

/// Event bus for managing event subscriptions and dispatch
pub struct EventBus {
    sender: Sender<GraphEvent>,
    receiver: Receiver<GraphEvent>,
    handlers: Vec<Box<dyn EventHandler>>,
    event_queue: VecDeque<GraphEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            handlers: Vec::new(),
            event_queue: VecDeque::new(),
        }
    }

    /// Get a sender for publishing events
    pub fn get_sender(&self) -> Sender<GraphEvent> {
        self.sender.clone()
    }

    /// Subscribe an event handler
    pub fn subscribe(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    /// Publish an event
    pub fn publish(&mut self, event: GraphEvent) {
        self.event_queue.push_back(event);
    }

    /// Process all pending events
    pub fn process_events(&mut self) -> Result<()> {
        // Process events from channel
        while let Ok(event) = self.receiver.try_recv() {
            self.event_queue.push_back(event);
        }

        // Process queued events
        while let Some(event) = self.event_queue.pop_front() {
            for handler in &mut self.handlers {
                handler.handle_event(&event)?;
            }
        }

        Ok(())
    }

    /// Process events with a limit
    pub fn process_events_limited(&mut self, max_events: usize) -> Result<usize> {
        let mut processed = 0;

        // Process events from channel
        while processed < max_events {
            match self.receiver.try_recv() {
                Ok(event) => {
                    for handler in &mut self.handlers {
                        handler.handle_event(&event)?;
                    }
                    processed += 1;
                }
                Err(_) => break,
            }
        }

        Ok(processed)
    }
}

/// Event logger for debugging
pub struct EventLogger {
    events: VecDeque<(std::time::Instant, GraphEvent)>,
    max_events: usize,
}

impl EventLogger {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(max_events),
            max_events,
        }
    }

    pub fn log(&mut self, event: GraphEvent) {
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back((std::time::Instant::now(), event));
    }

    pub fn get_recent(&self, count: usize) -> Vec<&GraphEvent> {
        self.events
            .iter()
            .rev()
            .take(count)
            .map(|(_, e)| e)
            .collect()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// Event filter for selective processing
pub struct EventFilter {
    allowed_types: Vec<String>,
    blocked_types: Vec<String>,
}

impl EventFilter {
    pub fn new() -> Self {
        Self {
            allowed_types: Vec::new(),
            blocked_types: Vec::new(),
        }
    }

    pub fn allow(&mut self, event_type: String) {
        self.allowed_types.push(event_type);
    }

    pub fn block(&mut self, event_type: String) {
        self.blocked_types.push(event_type);
    }

    pub fn should_process(&self, event: &GraphEvent) -> bool {
        let event_type = format!("{:?}", event).split('(').next().unwrap_or("").to_string();

        if !self.allowed_types.is_empty() {
            return self.allowed_types.contains(&event_type);
        }

        if !self.blocked_types.is_empty() {
            return !self.blocked_types.contains(&event_type);
        }

        true
    }
}