use anyhow::Result;
use git2::{IndexAddOption, Repository as Git2Repository, Signature, Time};
use std::path::Path;

pub struct Commit<'repo> {
    repo: &'repo Git2Repository,
}

impl<'repo> Commit<'repo> {
    pub fn new(repo: &'repo Git2Repository) -> Self {
        Commit { repo }
    }

    /// Stage a file to the index
    pub fn stage_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_path(path.as_ref())?;
        index.write()?;
        Ok(())
    }

    /// Stage all files (including untracked)
    pub fn stage_all(&self) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Unstage a file from the index
    pub fn unstage_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut index = self.repo.index()?;

        if let Ok(head) = self.repo.head() {
            // If we have HEAD, reset file to HEAD state
            let head_tree = head.peel_to_tree()?;
            if let Ok(entry) = head_tree.get_path(path.as_ref()) {
                let blob = self.repo.find_blob(entry.id())?;
                index.add(&git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: entry.filemode() as u32,
                    uid: 0,
                    gid: 0,
                    file_size: blob.size() as u32,
                    id: entry.id(),
                    flags: 0,
                    flags_extended: 0,
                    path: path.as_ref().to_string_lossy().into_owned().into_bytes(),
                })?;
            } else {
                // File doesn't exist in HEAD, remove from index
                index.remove_path(path.as_ref())?;
            }
        } else {
            // No HEAD, just remove from index
            index.remove_path(path.as_ref())?;
        }

        index.write()?;
        Ok(())
    }

    /// Reset all staged files
    pub fn reset_index(&self) -> Result<()> {
        let mut index = self.repo.index()?;

        if let Ok(head) = self.repo.head() {
            let tree = head.peel_to_tree()?;
            index.read_tree(&tree)?;
        } else {
            // If no HEAD exists (initial repo), clear the index
            index.clear()?;
        }

        index.write()?;
        Ok(())
    }

    /// Create a new commit
    pub fn create(
        &self,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<String> {
        let signature = Signature::now(author_name, author_email)?;
        self.create_with_signature(message, &signature, &signature)
    }

    /// Create a commit with specific author and committer
    pub fn create_with_signature(
        &self,
        message: &str,
        author: &Signature,
        committer: &Signature,
    ) -> Result<String> {
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let parent_commit = if let Ok(head) = self.repo.head() {
            Some(head.peel_to_commit()?)
        } else {
            None
        };

        let parents = if let Some(ref parent) = parent_commit {
            vec![parent]
        } else {
            vec![]
        };

        let oid = self.repo.commit(
            Some("HEAD"),
            author,
            committer,
            message,
            &tree,
            &parents,
        )?;

        Ok(oid.to_string())
    }

    /// Amend the current commit
    pub fn amend(
        &self,
        message: Option<&str>,
    ) -> Result<String> {
        let head = self.repo.head()?.peel_to_commit()?;

        let message = message.unwrap_or(head.message().unwrap_or(""));
        let author = head.author();
        let committer = Signature::now(
            head.committer().name().unwrap_or(""),
            head.committer().email().unwrap_or(""),
        )?;

        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let oid = head.amend(
            Some("HEAD"),
            Some(&author),
            Some(&committer),
            None,
            Some(message),
            Some(&tree),
        )?;

        Ok(oid.to_string())
    }

    /// Cherry-pick a commit
    pub fn cherry_pick(&self, commit_id: &str) -> Result<()> {
        let oid = git2::Oid::from_str(commit_id)?;
        let commit = self.repo.find_commit(oid)?;

        let mut options = git2::CherrypickOptions::new();
        self.repo.cherrypick(&commit, Some(&mut options))?;

        Ok(())
    }

    /// Revert a commit
    pub fn revert(&self, commit_id: &str) -> Result<()> {
        let oid = git2::Oid::from_str(commit_id)?;
        let commit = self.repo.find_commit(oid)?;

        let mut options = git2::RevertOptions::new();
        self.repo.revert(&commit, Some(&mut options))?;

        Ok(())
    }
}

/// Get the status of files in the repository
pub struct Status<'repo> {
    repo: &'repo Git2Repository,
}

#[derive(Debug, Clone)]
pub struct FileStatus {
    pub path: String,
    pub status: StatusType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusType {
    New,
    Modified,
    Deleted,
    Renamed,
    Copied,
    UpdatedButUnmerged,
    Untracked,
    Ignored,
}

impl<'repo> Status<'repo> {
    pub fn new(repo: &'repo Git2Repository) -> Self {
        Status { repo }
    }

    /// Get the status of all files
    pub fn get_all(&self) -> Result<Vec<FileStatus>> {
        let mut options = git2::StatusOptions::new();
        options.include_untracked(true);
        options.include_ignored(false);

        let statuses = self.repo.statuses(Some(&mut options))?;
        let mut result = Vec::new();

        for entry in statuses.iter() {
            let status = entry.status();
            let path = entry.path().unwrap_or("").to_string();

            let status_type = if status.is_wt_new() || status.is_index_new() {
                StatusType::New
            } else if status.is_wt_modified() || status.is_index_modified() {
                StatusType::Modified
            } else if status.is_wt_deleted() || status.is_index_deleted() {
                StatusType::Deleted
            } else if status.is_wt_renamed() || status.is_index_renamed() {
                StatusType::Renamed
            } else if status.is_wt_typechange() || status.is_index_typechange() {
                StatusType::Copied
            } else if status.is_conflicted() {
                StatusType::UpdatedButUnmerged
            } else if status.is_wt_new() && !status.is_index_new() {
                StatusType::Untracked
            } else if status.is_ignored() {
                StatusType::Ignored
            } else {
                continue;
            };

            result.push(FileStatus {
                path,
                status: status_type,
            });
        }

        Ok(result)
    }

    /// Check if there are any staged changes
    pub fn has_staged_changes(&self) -> Result<bool> {
        let statuses = self.repo.statuses(None)?;

        for entry in statuses.iter() {
            let status = entry.status();
            if status.is_index_new() ||
               status.is_index_modified() ||
               status.is_index_deleted() ||
               status.is_index_renamed() ||
               status.is_index_typechange() {
                return Ok(true);
            }
        }

        Ok(false)
    }
}