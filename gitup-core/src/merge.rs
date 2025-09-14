use anyhow::Result;
use git2::{
    Repository as Git2Repository, Oid, AnnotatedCommit, MergeOptions,
    build::CheckoutBuilder, Reference, BranchType,
};
use std::path::Path;

/// Merge result information
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub success: bool,
    pub message: String,
    pub conflicts: Vec<String>,
    pub merged_commit: Option<String>,
}

/// Merge operations for a repository
pub struct MergeOps {
    repo: Git2Repository,
}

impl MergeOps {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self> {
        let repo = Git2Repository::open(repo_path)?;
        Ok(MergeOps { repo })
    }

    /// Merge a branch into the current branch
    pub fn merge_branch(&self, branch_name: &str, message: Option<&str>) -> Result<MergeResult> {
        // Get the branch to merge
        let branch = self.repo.find_branch(branch_name, BranchType::Local)?;
        let branch_commit = branch.get().peel_to_commit()?;
        let branch_oid = branch_commit.id();

        // Get current HEAD
        let head = self.repo.head()?;
        let head_commit = head.peel_to_commit()?;

        // Create annotated commit for the merge
        let annotated_commit = self.repo.find_annotated_commit(branch_oid)?;

        // Perform merge analysis
        let (merge_analysis, _) = self.repo.merge_analysis(&[&annotated_commit])?;

        if merge_analysis.is_fast_forward() {
            // Fast-forward merge
            self.fast_forward_merge(&head, &branch_commit)?;
            Ok(MergeResult {
                success: true,
                message: format!("Fast-forwarded to {}", branch_name),
                conflicts: vec![],
                merged_commit: Some(branch_oid.to_string()),
            })
        } else if merge_analysis.is_normal() {
            // Normal merge
            self.normal_merge(&annotated_commit, &head_commit, &branch_commit, branch_name, message)
        } else if merge_analysis.is_up_to_date() {
            Ok(MergeResult {
                success: true,
                message: format!("Already up to date with {}", branch_name),
                conflicts: vec![],
                merged_commit: None,
            })
        } else {
            Err(anyhow::anyhow!("Cannot merge {} - unhandled merge scenario", branch_name))
        }
    }

    /// Perform a fast-forward merge
    fn fast_forward_merge(&self, head: &Reference, target_commit: &git2::Commit) -> Result<()> {
        let target_oid = target_commit.id();
        let refname = head.name().ok_or_else(|| anyhow::anyhow!("Invalid reference name"))?;

        // Create a new reference at the target commit
        let mut reference = self.repo.find_reference(refname)?;
        reference.set_target(target_oid, "Fast-forward merge")?;

        // Checkout the new HEAD
        self.repo.set_head(refname)?;
        self.repo.checkout_head(Some(CheckoutBuilder::new().force()))?;

        Ok(())
    }

    /// Perform a normal (non-fast-forward) merge
    fn normal_merge(
        &self,
        annotated_commit: &AnnotatedCommit,
        head_commit: &git2::Commit,
        branch_commit: &git2::Commit,
        branch_name: &str,
        message: Option<&str>,
    ) -> Result<MergeResult> {
        // Perform the merge
        let mut merge_options = MergeOptions::new();
        let mut checkout_builder = CheckoutBuilder::new();

        self.repo.merge(
            &[annotated_commit],
            Some(&mut merge_options),
            Some(&mut checkout_builder),
        )?;

        // Check for conflicts
        let index = self.repo.index()?;
        let has_conflicts = index.has_conflicts();

        if has_conflicts {
            // Get list of conflicted files
            let conflicts = self.get_conflicts()?;

            Ok(MergeResult {
                success: false,
                message: format!("Merge conflict in {} files", conflicts.len()),
                conflicts,
                merged_commit: None,
            })
        } else {
            // Create merge commit
            let merged_commit = self.create_merge_commit(
                head_commit,
                branch_commit,
                branch_name,
                message,
            )?;

            Ok(MergeResult {
                success: true,
                message: format!("Merged branch '{}'", branch_name),
                conflicts: vec![],
                merged_commit: Some(merged_commit),
            })
        }
    }

    /// Create a merge commit
    fn create_merge_commit(
        &self,
        head_commit: &git2::Commit,
        branch_commit: &git2::Commit,
        branch_name: &str,
        message: Option<&str>,
    ) -> Result<String> {
        // Get the merged tree
        let mut index = self.repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        // Create commit message
        let default_message = format!("Merge branch '{}'", branch_name);
        let commit_message = message.unwrap_or(&default_message);

        // Get signature
        let sig = self.get_signature()?;

        // Create the merge commit with both parents
        let commit_oid = self.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            commit_message,
            &tree,
            &[head_commit, branch_commit],
        )?;

        // Clean up merge state
        self.repo.cleanup_state()?;

        Ok(commit_oid.to_string())
    }

    /// Abort an in-progress merge
    pub fn abort_merge(&self) -> Result<String> {
        // Check if merge is in progress
        if self.repo.state() != git2::RepositoryState::Merge {
            return Err(anyhow::anyhow!("No merge in progress"));
        }

        // Reset to HEAD
        let head = self.repo.head()?.peel_to_commit()?;
        self.repo.reset(
            head.as_object(),
            git2::ResetType::Hard,
            Some(CheckoutBuilder::new().remove_untracked(true)),
        )?;

        // Clean up merge state
        self.repo.cleanup_state()?;

        Ok("Merge aborted".to_string())
    }

    /// Continue an in-progress merge after resolving conflicts
    pub fn continue_merge(&self, message: Option<&str>) -> Result<MergeResult> {
        // Check if merge is in progress
        if self.repo.state() != git2::RepositoryState::Merge {
            return Err(anyhow::anyhow!("No merge in progress"));
        }

        // Check for remaining conflicts
        let index = self.repo.index()?;
        if index.has_conflicts() {
            let conflicts = self.get_conflicts()?;
            return Ok(MergeResult {
                success: false,
                message: "Conflicts still present".to_string(),
                conflicts,
                merged_commit: None,
            });
        }

        // Get merge heads
        let merge_heads = self.get_merge_heads()?;
        if merge_heads.is_empty() {
            return Err(anyhow::anyhow!("No merge heads found"));
        }

        // Get parent commits
        let head = self.repo.head()?.peel_to_commit()?;
        let mut parents = vec![&head];

        let merge_commits: Vec<_> = merge_heads
            .iter()
            .map(|oid| self.repo.find_commit(*oid))
            .collect::<Result<Vec<_>, _>>()?;

        for commit in &merge_commits {
            parents.push(commit);
        }

        // Create the merge commit
        let mut index = self.repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        let commit_message = message.unwrap_or("Merge commit");
        let sig = self.get_signature()?;

        let commit_oid = self.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            commit_message,
            &tree,
            &parents,
        )?;

        // Clean up merge state
        self.repo.cleanup_state()?;

        Ok(MergeResult {
            success: true,
            message: "Merge completed".to_string(),
            conflicts: vec![],
            merged_commit: Some(commit_oid.to_string()),
        })
    }

    /// Get list of conflicted files
    pub fn get_conflicts(&self) -> Result<Vec<String>> {
        let mut conflicts = Vec::new();
        let index = self.repo.index()?;

        for conflict in index.conflicts()? {
            if let Ok(entry) = conflict {
                if let Some(our) = entry.our {
                    let path = std::str::from_utf8(&our.path)
                        .unwrap_or("<invalid utf8>")
                        .to_string();
                    if !conflicts.contains(&path) {
                        conflicts.push(path);
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// Get merge status
    pub fn merge_status(&self) -> Result<String> {
        if self.repo.state() == git2::RepositoryState::Merge {
            let conflicts = self.get_conflicts()?;
            if !conflicts.is_empty() {
                Ok(format!("Merging with {} conflicts", conflicts.len()))
            } else {
                Ok("Merge in progress (no conflicts)".to_string())
            }
        } else {
            Ok("No merge in progress".to_string())
        }
    }

    /// Get merge heads
    fn get_merge_heads(&self) -> Result<Vec<Oid>> {
        // Read MERGE_HEAD file directly
        let merge_head_path = self.repo.path().join("MERGE_HEAD");
        if !merge_head_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(merge_head_path)?;
        let mut merge_heads = Vec::new();

        for line in content.lines() {
            if let Ok(oid) = Oid::from_str(line.trim()) {
                merge_heads.push(oid);
            }
        }

        Ok(merge_heads)
    }

    /// Resolve conflicts by choosing a version
    pub fn resolve_conflict(&self, file_path: &str, resolution: ConflictResolution) -> Result<String> {
        let index = self.repo.index()?;

        // Get conflict entries
        let mut our_entry = None;
        let mut their_entry = None;

        for conflict in index.conflicts()? {
            if let Ok(entry) = conflict {
                if let Some(ref e) = entry.our {
                    if std::str::from_utf8(&e.path)? == file_path {
                        our_entry = entry.our;
                        their_entry = entry.their;
                        break;
                    }
                }
            }
        }

        match resolution {
            ConflictResolution::Ours => {
                if let Some(entry) = our_entry {
                    let mut index = self.repo.index()?;
                    index.add(&entry)?;
                    index.write()?;
                    Ok(format!("Resolved {} using our version", file_path))
                } else {
                    Err(anyhow::anyhow!("No 'ours' version found for {}", file_path))
                }
            }
            ConflictResolution::Theirs => {
                if let Some(entry) = their_entry {
                    let mut index = self.repo.index()?;
                    index.add(&entry)?;
                    index.write()?;
                    Ok(format!("Resolved {} using their version", file_path))
                } else {
                    Err(anyhow::anyhow!("No 'theirs' version found for {}", file_path))
                }
            }
            ConflictResolution::Manual => {
                // User will manually edit and stage the file
                Ok(format!("Please manually resolve {} and stage it", file_path))
            }
        }
    }

    /// Get signature for merge operations
    fn get_signature(&self) -> Result<git2::Signature<'static>> {
        // Try to get from git config
        let config = self.repo.config()?;

        let name = config.get_string("user.name")
            .unwrap_or_else(|_| "GitUp User".to_string());
        let email = config.get_string("user.email")
            .unwrap_or_else(|_| "gitup@local".to_string());

        Ok(git2::Signature::now(&name, &email)?)
    }
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy)]
pub enum ConflictResolution {
    Ours,
    Theirs,
    Manual,
}