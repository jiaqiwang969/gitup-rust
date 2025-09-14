use anyhow::Result;
use git2::{
    Repository as Git2Repository, Signature, StashApplyOptions, StashFlags, Oid,
};
use chrono::{DateTime, Local, TimeZone};
use std::path::Path;
use std::str::FromStr;

/// Stash entry information
#[derive(Debug, Clone)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
    pub oid: String,
    pub timestamp: DateTime<Local>,
}

/// Stash operations for a repository
pub struct StashOps {
    repo: Git2Repository,
}

impl StashOps {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self> {
        let repo = Git2Repository::open(repo_path)?;
        Ok(StashOps { repo })
    }

    /// Save current changes to stash
    pub fn save(&mut self, message: Option<&str>, include_untracked: bool) -> Result<String> {
        // Get signature for stash
        let sig = self.get_signature()?;

        // Determine stash flags
        let flags = if include_untracked {
            StashFlags::INCLUDE_UNTRACKED
        } else {
            StashFlags::DEFAULT
        };

        // Create stash
        let _stash_oid = self.repo.stash_save2(
            &sig,
            message,
            Some(flags),
        )?;

        let msg = message.unwrap_or("WIP on current branch");
        Ok(format!("Saved working directory and index state: {}", msg))
    }

    /// List all stashes
    pub fn list(&mut self) -> Result<Vec<StashInfo>> {
        let mut stashes = Vec::new();

        self.repo.stash_foreach(|index, name, oid| {
            // Parse stash message
            let message = name.split(": ").nth(1).unwrap_or(name).to_string();

            stashes.push(StashInfo {
                index,
                message,
                oid: oid.to_string(),
                timestamp: Local::now(), // We'll get the timestamp separately
            });

            true // Continue iteration
        })?;

        // Update timestamps
        for stash in &mut stashes {
            if let Ok(oid) = Oid::from_str(&stash.oid) {
                if let Ok(commit) = self.repo.find_commit(oid) {
                    stash.timestamp = Local.timestamp_opt(commit.time().seconds(), 0).single()
                        .unwrap_or_else(Local::now);
                }
            }
        }

        Ok(stashes)
    }

    /// Apply a stash (without removing it)
    pub fn apply(&mut self, index: usize) -> Result<String> {
        let mut opts = StashApplyOptions::new();

        // Apply the stash
        self.repo.stash_apply(index, Some(&mut opts))?;

        Ok(format!("Applied stash@{{{}}}", index))
    }

    /// Pop a stash (apply and remove)
    pub fn pop(&mut self, index: usize) -> Result<String> {
        // First apply the stash
        self.apply(index)?;

        // Then drop it
        self.drop(index)?;

        Ok(format!("Dropped and applied stash@{{{}}}", index))
    }

    /// Drop (remove) a stash
    pub fn drop(&mut self, index: usize) -> Result<String> {
        self.repo.stash_drop(index)?;
        Ok(format!("Dropped stash@{{{}}}", index))
    }

    /// Clear all stashes
    pub fn clear(&mut self) -> Result<String> {
        // Get count before clearing
        let _count = self.list()?.len();

        // Clear all stashes
        let mut dropped = 0;
        while self.list()?.len() > 0 {
            self.repo.stash_drop(0)?;
            dropped += 1;
        }

        Ok(format!("Dropped {} stash entries", dropped))
    }

    /// Show diff of a stash
    pub fn show(&mut self, index: usize) -> Result<String> {
        // Get the stash commit
        let mut stash_oid = None;
        self.repo.stash_foreach(|idx, _name, oid| {
            if idx == index {
                stash_oid = Some(*oid);
                false // Stop iteration
            } else {
                true // Continue
            }
        })?;

        let oid = stash_oid.ok_or_else(|| anyhow::anyhow!("Stash not found"))?;
        let commit = self.repo.find_commit(oid)?;

        // Get the diff between stash and its parent
        let parent = commit.parent(0)?;
        let stash_tree = commit.tree()?;
        let parent_tree = parent.tree()?;

        let diff = self.repo.diff_tree_to_tree(
            Some(&parent_tree),
            Some(&stash_tree),
            None,
        )?;

        // Format diff output
        let mut output = String::new();
        output.push_str(&format!("stash@{{{}}}: {}\n", index, commit.message().unwrap_or("")));
        output.push_str(&format!("Author: {}\n", commit.author().name().unwrap_or("Unknown")));
        output.push_str(&format!("Date: {}\n\n",
            Local.timestamp_opt(commit.time().seconds(), 0).single()
                .unwrap_or_else(Local::now)));

        // Add diff statistics
        let stats = diff.stats()?;
        output.push_str(&format!(" {} files changed, {} insertions(+), {} deletions(-)\n",
            stats.files_changed(),
            stats.insertions(),
            stats.deletions()));

        Ok(output)
    }

    /// Get signature for stash operations
    fn get_signature(&self) -> Result<Signature<'static>> {
        // Try to get from git config
        let config = self.repo.config()?;

        let name = config.get_string("user.name")
            .unwrap_or_else(|_| "GitUp User".to_string());
        let email = config.get_string("user.email")
            .unwrap_or_else(|_| "gitup@local".to_string());

        Ok(Signature::now(&name, &email)?)
    }

    /// Check if there are any stashes
    pub fn has_stashes(&mut self) -> Result<bool> {
        let mut has_stash = false;
        self.repo.stash_foreach(|_index, _name, _oid| {
            has_stash = true;
            false // Stop after first stash
        })?;
        Ok(has_stash)
    }

    /// Get a specific stash by index
    pub fn get(&mut self, index: usize) -> Result<Option<StashInfo>> {
        let stashes = self.list()?;
        Ok(stashes.into_iter().find(|s| s.index == index))
    }
}