use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;
use notify::{Watcher, RecursiveMode, Event, EventKind, Config};
use anyhow::Result;
use super::event::GraphEvent;

/// Git repository file system watcher
pub struct GitWatcher {
    watcher: Box<dyn Watcher>,
    event_sender: Sender<GraphEvent>,
    repo_path: PathBuf,
}

impl GitWatcher {
    /// Create a new Git watcher
    pub fn new(repo_path: &Path, event_sender: Sender<GraphEvent>) -> Result<Self> {
        let config = Config::default()
            .with_poll_interval(Duration::from_secs(1));

        let tx = event_sender.clone();
        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                handle_fs_event(event, &tx);
            }
        })?;

        Ok(Self {
            watcher: Box::new(watcher),
            event_sender,
            repo_path: repo_path.to_path_buf(),
        })
    }

    /// Start watching the repository
    pub fn watch(&mut self) -> Result<()> {
        let git_dir = self.repo_path.join(".git");

        // Watch the .git directory
        self.watcher.watch(&git_dir, RecursiveMode::Recursive)?;

        // Also watch the working directory for changes
        self.watcher.watch(&self.repo_path, RecursiveMode::NonRecursive)?;

        Ok(())
    }

    /// Stop watching
    pub fn unwatch(&mut self) -> Result<()> {
        let git_dir = self.repo_path.join(".git");
        self.watcher.unwatch(&git_dir)?;
        self.watcher.unwatch(&self.repo_path)?;
        Ok(())
    }

    /// Update the repository path
    pub fn set_repo_path(&mut self, path: &Path) -> Result<()> {
        // Unwatch old path
        self.unwatch()?;

        // Update path
        self.repo_path = path.to_path_buf();

        // Watch new path
        self.watch()
    }
}

/// Handle file system events and convert to graph events
fn handle_fs_event(event: Event, sender: &Sender<GraphEvent>) {
    match event.kind {
        EventKind::Modify(_) => {
            for path in &event.paths {
                if let Some(file_name) = path.file_name() {
                    let name = file_name.to_string_lossy();

                    // Check for specific Git files
                    if name == "HEAD" {
                        // Branch or checkout change
                        let _ = sender.send(GraphEvent::BranchChanged(String::new()));
                    } else if name == "index" {
                        // Working tree change
                        let _ = sender.send(GraphEvent::WorkingTreeChanged);
                    } else if path.parent().and_then(|p| p.file_name())
                        .map(|n| n == "refs").unwrap_or(false) {
                        // Reference update
                        let ref_name = path.strip_prefix(".git/refs/")
                            .ok()
                            .and_then(|p| p.to_str())
                            .unwrap_or("")
                            .to_string();

                        let _ = sender.send(GraphEvent::RefUpdated(
                            super::event::RefUpdate {
                                ref_name,
                                old_oid: None,
                                new_oid: String::new(),
                            }
                        ));
                    } else if name == "COMMIT_EDITMSG" {
                        // New commit being created
                        // We'll detect the actual commit when objects are updated
                    } else if path.parent().and_then(|p| p.file_name())
                        .map(|n| n == "objects").unwrap_or(false) {
                        // New objects (commits, trees, blobs)
                        // This could indicate new commits
                        let _ = sender.send(GraphEvent::RepositoryChanged(
                            super::event::RepositoryChange {
                                change_type: super::event::RepositoryChangeType::Push,
                                path: String::new(),
                            }
                        ));
                    }
                }
            }
        }
        EventKind::Create(_) => {
            // Handle new files
            for path in &event.paths {
                if path.file_name().map(|n| n == "MERGE_HEAD").unwrap_or(false) {
                    // Merge in progress
                    let _ = sender.send(GraphEvent::RepositoryChanged(
                        super::event::RepositoryChange {
                            change_type: super::event::RepositoryChangeType::Merge,
                            path: String::new(),
                        }
                    ));
                } else if path.file_name().map(|n| n == "rebase-merge" || n == "rebase-apply")
                    .unwrap_or(false) {
                    // Rebase in progress
                    let _ = sender.send(GraphEvent::RepositoryChanged(
                        super::event::RepositoryChange {
                            change_type: super::event::RepositoryChangeType::Rebase,
                            path: String::new(),
                        }
                    ));
                }
            }
        }
        EventKind::Remove(_) => {
            // Handle removed files
            for path in &event.paths {
                if path.file_name().map(|n| n == "MERGE_HEAD").unwrap_or(false) {
                    // Merge completed
                    let _ = sender.send(GraphEvent::WorkingTreeChanged);
                } else if path.file_name().map(|n| n == "rebase-merge" || n == "rebase-apply")
                    .unwrap_or(false) {
                    // Rebase completed
                    let _ = sender.send(GraphEvent::WorkingTreeChanged);
                }
            }
        }
        _ => {}
    }
}

/// Debouncer for reducing event frequency
pub struct EventDebouncer {
    last_event_time: std::time::Instant,
    debounce_duration: Duration,
    pending_event: Option<GraphEvent>,
}

impl EventDebouncer {
    pub fn new(debounce_duration: Duration) -> Self {
        Self {
            last_event_time: std::time::Instant::now(),
            debounce_duration,
            pending_event: None,
        }
    }

    /// Add an event to the debouncer
    pub fn add_event(&mut self, event: GraphEvent) {
        self.pending_event = Some(event);
        self.last_event_time = std::time::Instant::now();
    }

    /// Get the pending event if debounce period has passed
    pub fn get_event(&mut self) -> Option<GraphEvent> {
        if self.pending_event.is_some() &&
           self.last_event_time.elapsed() >= self.debounce_duration {
            self.pending_event.take()
        } else {
            None
        }
    }

    /// Check if there's a pending event
    pub fn has_pending(&self) -> bool {
        self.pending_event.is_some()
    }

    /// Force flush the pending event
    pub fn flush(&mut self) -> Option<GraphEvent> {
        self.pending_event.take()
    }
}