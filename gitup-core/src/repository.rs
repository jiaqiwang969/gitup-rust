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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefType {
    Head,
    Branch,
    Remote,
    Tag,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefInfo {
    pub name: String,
    pub ref_type: RefType,
    pub is_head: bool,
    pub is_remote: bool,
}

/// Commit info with parent relationships for graph building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitWithParents {
    pub id: String,
    pub parents: Vec<String>,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
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
        // Default revision walk from HEAD
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

    /// Get recent commits with their parent commit ids (topological + time order)
    pub fn get_commits_with_parents(&self, count: usize) -> Result<Vec<CommitWithParents>> {
        use git2::Sort;

        let mut revwalk = self.git_repo.revwalk()?;
        // Ensure stable topology ordering for graph rendering
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        for (i, oid_res) in revwalk.enumerate() {
            if i >= count { break; }
            let oid = oid_res?;
            let commit = self.git_repo.find_commit(oid)?;

            // Collect parent ids
            let mut parents = Vec::with_capacity(commit.parent_count() as usize);
            for p in commit.parents() {
                parents.push(p.id().to_string());
            }

            commits.push(CommitWithParents {
                id: oid.to_string(),
                parents,
                message: commit.summary().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                email: commit.author().email().unwrap_or("").to_string(),
                timestamp: commit.time().seconds(),
            });
        }

        Ok(commits)
    }

    /// List all refs grouped by target OID (hex string)
    pub fn list_refs_by_oid(&self) -> Result<std::collections::HashMap<String, Vec<RefInfo>>> {
        use git2::BranchType;
        use std::collections::HashMap;

        let mut map: HashMap<String, Vec<RefInfo>> = HashMap::new();

        // HEAD
        if let Ok(head) = self.git_repo.head() {
            if let Some(target) = head.target() {
                map.entry(target.to_string()).or_default().push(RefInfo {
                    name: head.shorthand().unwrap_or("HEAD").to_string(),
                    ref_type: RefType::Head,
                    is_head: true,
                    is_remote: false,
                });
            }
        }

        // Local branches
        for br in self.git_repo.branches(Some(BranchType::Local))? {
            let (branch, _) = br?;
            if let Some(name) = branch.name()? {
                if let Some(target) = branch.get().target() {
                    map.entry(target.to_string()).or_default().push(RefInfo {
                        name: name.to_string(),
                        ref_type: RefType::Branch,
                        is_head: false,
                        is_remote: false,
                    });
                }
            }
        }

        // Remote branches
        for br in self.git_repo.branches(Some(BranchType::Remote))? {
            let (branch, _) = br?;
            if let Some(name) = branch.name()? {
                if let Some(target) = branch.get().target() {
                    map.entry(target.to_string()).or_default().push(RefInfo {
                        name: name.to_string(),
                        ref_type: RefType::Remote,
                        is_head: false,
                        is_remote: true,
                    });
                }
            }
        }

        // Tags
        self.git_repo.tag_foreach(|oid, name| {
            if let Ok(name_str) = std::str::from_utf8(name) {
                if let Some(tag_name) = name_str.strip_prefix("refs/tags/") {
                    map.entry(oid.to_string()).or_default().push(RefInfo {
                        name: tag_name.to_string(),
                        ref_type: RefType::Tag,
                        is_head: false,
                        is_remote: false,
                    });
                }
            }
            true
        })?;

        Ok(map)
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
    use std::fs;
    use std::io::Write;

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

    /// Helper to write a file
    fn write_file<P: AsRef<std::path::Path>>(p: P, content: &str) {
        let mut f = fs::File::create(p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    /// Make a commit with a new or updated file
    fn make_commit(repo: &Repository, workdir: &std::path::Path, name: &str, content: &str, msg: &str) -> String {
        write_file(workdir.join(name), content);
        let inner = Commit::new(&repo.git_repo);
        inner.stage_file(name).unwrap();
        repo.commit(msg, "Tester", "tester@example.com").unwrap()
    }

    #[test]
    fn test_get_commits_with_parents_linear() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // First commit (root)
        let _c1 = make_commit(&repo, temp_dir.path(), "a.txt", "1", "c1");
        // Second commit
        let c2 = make_commit(&repo, temp_dir.path(), "a.txt", "2", "c2");

        let commits = repo.get_commits_with_parents(10).unwrap();
        assert!(!commits.is_empty());
        // HEAD first
        assert_eq!(commits[0].id.len(), 40);
        assert_eq!(commits[0].message, "c2");
        // HEAD should have 1 parent in linear history
        assert_eq!(commits[0].parents.len(), 1);
        // Root commit should have 0 parents somewhere in the list
        assert!(commits.iter().any(|c| c.parents.is_empty()));
    }

    #[test]
    fn test_get_commits_with_parents_merge() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // c1 on main
        let _c1 = make_commit(&repo, temp_dir.path(), "f.txt", "1", "c1");

        // create branch feature from HEAD
        {
            let git = &repo.git_repo;
            let head_commit = git.head().unwrap().peel_to_commit().unwrap();
            git.branch("feature", &head_commit, false).unwrap();
        }

        // checkout feature
        repo.checkout_branch("feature").unwrap();
        let _c2 = make_commit(&repo, temp_dir.path(), "f.txt", "2", "c2-feature");

        // checkout main and add c2m
        repo.checkout_branch("master").ok(); // some git init default may be "master" or "main"
        // If master not exist, try "main"
        if repo.git_repo.find_branch("master", git2::BranchType::Local).is_err() {
            // attempt to set HEAD symbolic name to main if exists
            // No-op; continue assuming default branch name
        }
        let _c2m = make_commit(&repo, temp_dir.path(), "f.txt", "3", "c2-main");

        // merge feature into current branch (fast-forward or merge)
        {
            let git = &repo.git_repo;
            let mut idx = git.merge_commits(
                &git.head().unwrap().peel_to_commit().unwrap(),
                &git.find_branch("feature", git2::BranchType::Local).unwrap().get().peel_to_commit().unwrap(),
                None,
            ).unwrap();
            assert!(idx.has_conflicts() == false);
            // Write the merge tree
            let tree_id = idx.write_tree_to(git).unwrap();
            let tree = git.find_tree(tree_id).unwrap();
            let sig = git2::Signature::now("Tester", "tester@example.com").unwrap();
            let head = git.head().unwrap().peel_to_commit().unwrap();
            let feature = git.find_branch("feature", git2::BranchType::Local).unwrap().get().peel_to_commit().unwrap();
            let oid = git.commit(Some("HEAD"), &sig, &sig, "merge feature", &tree, &[&head, &feature]).unwrap();
            assert_eq!(oid.to_string().len(), 40);
        }

        let commits = repo.get_commits_with_parents(10).unwrap();
        // At least one merge commit with two parents
        assert!(commits.iter().any(|c| c.parents.len() >= 2));
    }

    #[test]
    fn test_list_refs_by_oid_basic() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // first commit
        let c1 = make_commit(&repo, temp_dir.path(), "g.txt", "1", "init");

        // create branch 'feature'
        {
            let git = &repo.git_repo;
            let head_commit = git.head().unwrap().peel_to_commit().unwrap();
            git.branch("feature", &head_commit, false).unwrap();
        }
        // create tag 'v1'
        {
            let git = &repo.git_repo;
            let oid = git2::Oid::from_str(&c1).unwrap();
            let obj = git.find_object(oid, Some(git2::ObjectType::Commit)).unwrap();
            let sig = git2::Signature::now("Tester", "tester@example.com").unwrap();
            git.tag("v1", &obj, &sig, "tag v1", false).unwrap();
        }

        let map = repo.list_refs_by_oid().unwrap();
        // Should contain at least HEAD entry and tag/branch entries
        assert!(!map.is_empty());
        assert!(map.values().flatten().any(|r| r.ref_type == RefType::Head));
        assert!(map.values().flatten().any(|r| matches!(r.ref_type, RefType::Branch)));
        assert!(map.values().flatten().any(|r| matches!(r.ref_type, RefType::Tag)));
    }
}
