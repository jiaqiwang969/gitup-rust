use anyhow::Result;
use git2::Repository as Git2Repository;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::diff::{Diff, FileDiff, DiffStats};
use crate::commit::{Commit, Status, FileStatus};

pub struct Repository {
    path: PathBuf,
    git_repo: Git2Repository,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub commit_id: String,
    pub is_head: bool,
    pub is_remote: bool,
}

impl Repository {
    /// Open an existing repository
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let git_repo = Git2Repository::open(&path)?;

        Ok(Repository {
            path,
            git_repo,
        })
    }

    /// Initialize a new repository
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let git_repo = Git2Repository::init(&path)?;

        Ok(Repository {
            path,
            git_repo,
        })
    }

    /// Get repository status
    pub fn is_clean(&self) -> Result<bool> {
        let statuses = self.git_repo.statuses(None)?;
        Ok(statuses.is_empty())
    }

    /// List all branches
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let mut branches = Vec::new();
        let head = self.git_repo.head()?;
        let head_name = head.shorthand().unwrap_or("");

        // Local branches
        for branch in self.git_repo.branches(Some(git2::BranchType::Local))? {
            let (branch, _) = branch?;
            let name = branch.name()?.unwrap_or("").to_string();
            let commit = branch.get().peel_to_commit()?;

            branches.push(BranchInfo {
                name: name.clone(),
                commit_id: commit.id().to_string(),
                is_head: name == head_name,
                is_remote: false,
            });
        }

        // Remote branches
        for branch in self.git_repo.branches(Some(git2::BranchType::Remote))? {
            let (branch, _) = branch?;
            let name = branch.name()?.unwrap_or("").to_string();
            let commit = branch.get().peel_to_commit()?;

            branches.push(BranchInfo {
                name,
                commit_id: commit.id().to_string(),
                is_head: false,
                is_remote: true,
            });
        }

        Ok(branches)
    }

    /// Get recent commits
    pub fn get_commits(&self, count: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.git_repo.revwalk()?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        for (i, oid) in revwalk.enumerate() {
            if i >= count {
                break;
            }

            let oid = oid?;
            let commit = self.git_repo.find_commit(oid)?;

            commits.push(CommitInfo {
                id: oid.to_string(),
                message: commit.summary().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                email: commit.author().email().unwrap_or("").to_string(),
                timestamp: commit.time().seconds(),
            });
        }

        Ok(commits)
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str, target: Option<&str>) -> Result<()> {
        let commit = if let Some(target) = target {
            let oid = git2::Oid::from_str(target)?;
            self.git_repo.find_commit(oid)?
        } else {
            self.git_repo.head()?.peel_to_commit()?
        };

        self.git_repo.branch(name, &commit, false)?;
        Ok(())
    }

    /// Checkout a branch
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        let branch = self.git_repo.find_branch(name, git2::BranchType::Local)?;
        let reference = branch.get();
        let object = reference.peel(git2::ObjectType::Commit)?;

        self.git_repo.checkout_tree(&object, None)?;
        self.git_repo.set_head(reference.name().unwrap())?;

        Ok(())
    }

    /// Get diff between working directory and index
    pub fn diff_workdir_to_index(&self) -> Result<Vec<FileDiff>> {
        let diff = Diff::new(&self.git_repo);
        diff.workdir_to_index()
    }

    /// Get diff between index and HEAD
    pub fn diff_index_to_head(&self) -> Result<Vec<FileDiff>> {
        let diff = Diff::new(&self.git_repo);
        diff.index_to_head()
    }

    /// Get diff for a specific commit
    pub fn diff_for_commit(&self, commit_id: &str) -> Result<Vec<FileDiff>> {
        let diff = Diff::new(&self.git_repo);
        diff.for_commit(commit_id)
    }

    /// Get diff between two commits
    pub fn diff_between_commits(&self, old: &str, new: &str) -> Result<Vec<FileDiff>> {
        let diff = Diff::new(&self.git_repo);
        diff.between_commits(old, new)
    }

    /// Get diff statistics for working directory
    pub fn diff_stats(&self) -> Result<DiffStats> {
        let diffs = self.diff_workdir_to_index()?;
        Ok(DiffStats::from_diffs(&diffs))
    }

    /// Stage a file
    pub fn stage_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let commit = Commit::new(&self.git_repo);
        commit.stage_file(path)
    }

    /// Stage all files
    pub fn stage_all(&self) -> Result<()> {
        let commit = Commit::new(&self.git_repo);
        commit.stage_all()
    }

    /// Unstage a file
    pub fn unstage_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let commit = Commit::new(&self.git_repo);
        commit.unstage_file(path)
    }

    /// Reset all staged files
    pub fn reset_index(&self) -> Result<()> {
        let commit = Commit::new(&self.git_repo);
        commit.reset_index()
    }

    /// Create a commit
    pub fn commit(&self, message: &str, author_name: &str, author_email: &str) -> Result<String> {
        let commit = Commit::new(&self.git_repo);
        commit.create(message, author_name, author_email)
    }

    /// Amend the last commit
    pub fn amend_commit(&self, message: Option<&str>) -> Result<String> {
        let commit = Commit::new(&self.git_repo);
        commit.amend(message)
    }

    /// Get file statuses
    pub fn get_status(&self) -> Result<Vec<FileStatus>> {
        let status = Status::new(&self.git_repo);
        status.get_all()
    }

    /// Check if there are staged changes
    pub fn has_staged_changes(&self) -> Result<bool> {
        let status = Status::new(&self.git_repo);
        status.has_staged_changes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_repository() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        assert!(repo.is_clean().unwrap());
    }

    #[test]
    fn test_open_repository() {
        let temp_dir = TempDir::new().unwrap();
        Repository::init(temp_dir.path()).unwrap();

        let repo = Repository::open(temp_dir.path()).unwrap();
        assert!(repo.is_clean().unwrap());
    }
}