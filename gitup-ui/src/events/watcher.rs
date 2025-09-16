#![cfg(feature = "watch")]
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::sync::mpsc::Sender;
use notify::{RecommendedWatcher, Watcher, RecursiveMode, Event, EventKind, Config};
use anyhow::Result;
use super::types::GraphEvent;

pub struct GitWatcher {
    watcher: RecommendedWatcher,
    repo_path: PathBuf,
    sender: Sender<GraphEvent>,
}

impl GitWatcher {
    pub fn new<P: AsRef<Path>>(path: P, sender: Sender<GraphEvent>) -> Result<Self> {
        let repo_path = path.as_ref().to_path_buf();
        let sender_clone = sender.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(ev) = res {
                // Coarse mapping
                match ev.kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                        let _ = sender_clone.send(GraphEvent::RepositoryChanged);
                    }
                    _ => {}
                }
            }
        })?;
        watcher.configure(Config::default().with_poll_interval(Duration::from_millis(500)))?;
        Ok(Self { watcher, repo_path, sender })
    }

    pub fn watch(&mut self) -> Result<()> {
        let gitdir = self.repo_path.join(".git");
        self.watcher.watch(&gitdir, RecursiveMode::Recursive)?;
        self.watcher.watch(&self.repo_path, RecursiveMode::NonRecursive)?;
        Ok(())
    }

    pub fn unwatch(&mut self) -> Result<()> {
        let gitdir = self.repo_path.join(".git");
        self.watcher.unwatch(&gitdir)?;
        self.watcher.unwatch(&self.repo_path)?;
        Ok(())
    }
}

