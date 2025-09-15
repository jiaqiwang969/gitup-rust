use anyhow::Result;
use git2::Repository as Git2Repository;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::diff::{Diff, FileDiff, DiffStats};
use crate::commit::{Commit, Status, FileStatus};
use crate::remote::{RemoteInfo, RemoteOps};
use crate::stash::{StashInfo, StashOps};
use crate::tag::{TagInfo, TagOps};
use crate::merge::{MergeOps, MergeResult, ConflictResolution};
use crate::rebase::{RebaseOps, RebaseResult};
use crate::cherry_pick::{CherryPickOps, CherryPickResult};

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

    /// Get diff for a specific file in working directory
    pub fn diff_file<P: AsRef<Path>>(&self, path: P) -> Result<FileDiff> {
        let diff = Diff::new(&self.git_repo);
        diff.file_diff(path.as_ref())
    }

    /// Get staged diff for a specific file
    pub fn diff_staged_file<P: AsRef<Path>>(&self, path: P) -> Result<FileDiff> {
        let diff = Diff::new(&self.git_repo);
        diff.staged_file_diff(path.as_ref())
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

    // Remote operations

    /// Get remote operations handler
    pub fn remote_ops(&self) -> RemoteOps {
        RemoteOps::new(&self.git_repo)
    }

    /// List all remotes
    pub fn list_remotes(&self) -> Result<Vec<RemoteInfo>> {
        self.remote_ops().list_remotes()
    }

    /// Add a remote
    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        self.remote_ops().add_remote(name, url)
    }

    /// Remove a remote
    pub fn remove_remote(&self, name: &str) -> Result<()> {
        self.remote_ops().remove_remote(name)
    }

    /// Fetch from remote
    pub fn fetch(&self, remote_name: &str) -> Result<String> {
        self.remote_ops().fetch(remote_name, &[], None)
    }

    /// Pull from remote
    pub fn pull(&self, remote_name: &str, branch_name: &str) -> Result<String> {
        self.remote_ops().pull(remote_name, branch_name, None)
    }

    /// Push to remote
    pub fn push(&self, remote_name: &str) -> Result<String> {
        self.remote_ops().push(remote_name, &[], None)
    }

    /// Get upstream for current branch
    pub fn get_upstream(&self) -> Result<Option<(String, String)>> {
        self.remote_ops().get_upstream()
    }

    /// Set upstream for current branch
    pub fn set_upstream(&self, remote_name: &str, branch_name: &str) -> Result<()> {
        self.remote_ops().set_upstream(remote_name, branch_name)
    }

    // Stash operations

    /// Save changes to stash
    pub fn stash_save(&self, message: Option<&str>, include_untracked: bool) -> Result<String> {
        let mut ops = StashOps::new(&self.path)?;
        ops.save(message, include_untracked)
    }

    /// List all stashes
    pub fn stash_list(&self) -> Result<Vec<StashInfo>> {
        let mut ops = StashOps::new(&self.path)?;
        ops.list()
    }

    /// Apply a stash
    pub fn stash_apply(&self, index: usize) -> Result<String> {
        let mut ops = StashOps::new(&self.path)?;
        ops.apply(index)
    }

    /// Pop a stash (apply and remove)
    pub fn stash_pop(&self, index: Option<usize>) -> Result<String> {
        let index = index.unwrap_or(0);
        let mut ops = StashOps::new(&self.path)?;
        ops.pop(index)
    }

    /// Drop a stash
    pub fn stash_drop(&self, index: usize) -> Result<String> {
        let mut ops = StashOps::new(&self.path)?;
        ops.drop(index)
    }

    /// Clear all stashes
    pub fn stash_clear(&self) -> Result<String> {
        let mut ops = StashOps::new(&self.path)?;
        ops.clear()
    }

    /// Show a stash
    pub fn stash_show(&self, index: usize) -> Result<String> {
        let mut ops = StashOps::new(&self.path)?;
        ops.show(index)
    }

    /// Check if there are any stashes
    pub fn has_stashes(&self) -> Result<bool> {
        let mut ops = StashOps::new(&self.path)?;
        ops.has_stashes()
    }

    // Tag operations

    /// Create a tag
    pub fn tag_create(
        &self,
        name: &str,
        target: Option<&str>,
        message: Option<&str>,
        force: bool,
    ) -> Result<String> {
        let ops = TagOps::new(&self.path)?;
        ops.create(name, target, message, force)
    }

    /// List all tags
    pub fn tag_list(&self, pattern: Option<&str>) -> Result<Vec<TagInfo>> {
        let ops = TagOps::new(&self.path)?;
        ops.list(pattern)
    }

    /// Delete a tag
    pub fn tag_delete(&self, name: &str) -> Result<String> {
        let mut ops = TagOps::new(&self.path)?;
        ops.delete(name)
    }

    /// Show tag details
    pub fn tag_show(&self, name: &str) -> Result<String> {
        let ops = TagOps::new(&self.path)?;
        ops.show(name)
    }

    /// Push tags to remote
    pub fn tag_push(&self, remote_name: &str, tag_name: Option<&str>, force: bool) -> Result<String> {
        let ops = TagOps::new(&self.path)?;
        ops.push(remote_name, tag_name, force)
    }

    /// Check if a tag exists
    pub fn tag_exists(&self, name: &str) -> Result<bool> {
        let ops = TagOps::new(&self.path)?;
        Ok(ops.exists(name))
    }

    // Merge operations

    /// Merge a branch into the current branch
    pub fn merge_branch(&self, branch_name: &str, message: Option<&str>) -> Result<MergeResult> {
        let ops = MergeOps::new(&self.path)?;
        ops.merge_branch(branch_name, message)
    }

    /// Abort an in-progress merge
    pub fn merge_abort(&self) -> Result<String> {
        let ops = MergeOps::new(&self.path)?;
        ops.abort_merge()
    }

    /// Continue an in-progress merge
    pub fn merge_continue(&self, message: Option<&str>) -> Result<MergeResult> {
        let ops = MergeOps::new(&self.path)?;
        ops.continue_merge(message)
    }

    /// Get merge status
    pub fn merge_status(&self) -> Result<String> {
        let ops = MergeOps::new(&self.path)?;
        ops.merge_status()
    }

    /// Get list of conflicted files
    pub fn merge_conflicts(&self) -> Result<Vec<String>> {
        let ops = MergeOps::new(&self.path)?;
        ops.get_conflicts()
    }

    /// Resolve a conflict
    pub fn merge_resolve_conflict(&self, file_path: &str, resolution: ConflictResolution) -> Result<String> {
        let ops = MergeOps::new(&self.path)?;
        ops.resolve_conflict(file_path, resolution)
    }

    // Rebase operations

    /// Rebase current branch onto another branch
    pub fn rebase_onto(&self, target_branch: &str) -> Result<RebaseResult> {
        let ops = RebaseOps::new(&self.path)?;
        ops.rebase_onto(target_branch)
    }

    /// Start an interactive rebase
    pub fn rebase_interactive(&self, upstream: &str, onto: Option<&str>) -> Result<RebaseResult> {
        let ops = RebaseOps::new(&self.path)?;
        ops.start_interactive(upstream, onto)
    }

    /// Continue an in-progress rebase
    pub fn rebase_continue(&self) -> Result<RebaseResult> {
        let mut ops = RebaseOps::new(&self.path)?;
        ops.continue_rebase()
    }

    /// Abort an in-progress rebase
    pub fn rebase_abort(&self) -> Result<String> {
        let ops = RebaseOps::new(&self.path)?;
        ops.abort_rebase()
    }

    /// Skip current commit in rebase
    pub fn rebase_skip(&self) -> Result<RebaseResult> {
        let mut ops = RebaseOps::new(&self.path)?;
        ops.skip_commit()
    }

    /// Get rebase status
    pub fn rebase_status(&self) -> Result<String> {
        let ops = RebaseOps::new(&self.path)?;
        ops.rebase_status()
    }

    // Cherry-pick operations

    /// Cherry-pick a single commit
    pub fn cherry_pick(&self, commit_ref: &str) -> Result<CherryPickResult> {
        let ops = CherryPickOps::new(&self.path)?;
        ops.pick_commit(commit_ref)
    }

    /// Cherry-pick a range of commits
    pub fn cherry_pick_range(&self, start_ref: &str, end_ref: &str) -> Result<Vec<CherryPickResult>> {
        let ops = CherryPickOps::new(&self.path)?;
        ops.pick_range(start_ref, end_ref)
    }

    /// Continue a cherry-pick after resolving conflicts
    pub fn cherry_pick_continue(&self) -> Result<CherryPickResult> {
        let ops = CherryPickOps::new(&self.path)?;
        ops.continue_pick()
    }

    /// Abort a cherry-pick in progress
    pub fn cherry_pick_abort(&self) -> Result<String> {
        let ops = CherryPickOps::new(&self.path)?;
        ops.abort_pick()
    }

    /// Get cherry-pick status
    pub fn cherry_pick_status(&self) -> Result<String> {
        let ops = CherryPickOps::new(&self.path)?;
        ops.pick_status()
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