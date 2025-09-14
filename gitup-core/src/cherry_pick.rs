use anyhow::Result;
use git2::{
    Repository as Git2Repository, Oid, build::CheckoutBuilder,
    CherrypickOptions, Index,
};
use std::path::Path;

/// Cherry-pick result information
#[derive(Debug, Clone)]
pub struct CherryPickResult {
    pub success: bool,
    pub message: String,
    pub picked_commit: Option<String>,
    pub conflicts: Vec<String>,
}

/// Cherry-pick operations for a repository
pub struct CherryPickOps {
    repo: Git2Repository,
}

impl CherryPickOps {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self> {
        let repo = Git2Repository::open(repo_path)?;
        Ok(CherryPickOps { repo })
    }

    /// Cherry-pick a single commit
    pub fn pick_commit(&self, commit_ref: &str) -> Result<CherryPickResult> {
        // Find the commit to cherry-pick
        let commit = self.find_commit_from_ref(commit_ref)?;
        let commit_id = commit.id();

        // Set up cherry-pick options
        let mut opts = CherrypickOptions::new();

        // Perform the cherry-pick
        self.repo.cherrypick(&commit, Some(&mut opts))?;

        // Check for conflicts
        let index = self.repo.index()?;
        if index.has_conflicts() {
            let conflicts = self.get_conflicts(&index)?;
            Ok(CherryPickResult {
                success: false,
                message: format!("Cherry-pick conflict with commit {}", &commit_id.to_string()[..8]),
                picked_commit: None,
                conflicts,
            })
        } else {
            // Create the commit
            let picked = self.complete_cherry_pick(&commit)?;
            Ok(CherryPickResult {
                success: true,
                message: format!("Successfully cherry-picked commit {}", &commit_id.to_string()[..8]),
                picked_commit: Some(picked),
                conflicts: vec![],
            })
        }
    }

    /// Cherry-pick multiple commits
    pub fn pick_range(&self, start_ref: &str, end_ref: &str) -> Result<Vec<CherryPickResult>> {
        // Find the commit range
        let start_commit = self.find_commit_from_ref(start_ref)?;
        let end_commit = self.find_commit_from_ref(end_ref)?;

        // Build list of commits to cherry-pick
        let mut commits = Vec::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(end_commit.id())?;

        for oid in revwalk {
            let oid = oid?;
            if oid == start_commit.id() {
                break;
            }
            commits.push(oid);
        }

        // Reverse to apply in chronological order
        commits.reverse();

        // Cherry-pick each commit
        let mut results = Vec::new();
        for oid in commits {
            let commit_ref = oid.to_string();
            let result = self.pick_commit(&commit_ref)?;

            if !result.success {
                // Stop on first conflict
                results.push(result);
                break;
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Continue a cherry-pick after resolving conflicts
    pub fn continue_pick(&self) -> Result<CherryPickResult> {
        // Check repository state
        if self.repo.state() != git2::RepositoryState::CherryPick {
            return Err(anyhow::anyhow!("No cherry-pick in progress"));
        }

        // Check for remaining conflicts
        let index = self.repo.index()?;
        if index.has_conflicts() {
            let conflicts = self.get_conflicts(&index)?;
            return Ok(CherryPickResult {
                success: false,
                message: "Conflicts must be resolved before continuing".to_string(),
                picked_commit: None,
                conflicts,
            });
        }

        // Read CHERRY_PICK_HEAD to get the commit being picked
        let cherry_pick_head_path = self.repo.path().join("CHERRY_PICK_HEAD");
        let cherry_pick_oid = if cherry_pick_head_path.exists() {
            let content = std::fs::read_to_string(&cherry_pick_head_path)?;
            Oid::from_str(content.trim())?
        } else {
            return Err(anyhow::anyhow!("Cannot find CHERRY_PICK_HEAD"));
        };

        let commit = self.repo.find_commit(cherry_pick_oid)?;

        // Complete the cherry-pick
        let picked = self.complete_cherry_pick(&commit)?;

        // Clean up state
        self.repo.cleanup_state()?;

        Ok(CherryPickResult {
            success: true,
            message: format!("Cherry-pick completed for commit {}",
                &cherry_pick_oid.to_string()[..8]),
            picked_commit: Some(picked),
            conflicts: vec![],
        })
    }

    /// Abort a cherry-pick in progress
    pub fn abort_pick(&self) -> Result<String> {
        // Check repository state
        if self.repo.state() != git2::RepositoryState::CherryPick {
            return Err(anyhow::anyhow!("No cherry-pick in progress"));
        }

        // Reset to HEAD
        let head = self.repo.head()?.peel_to_commit()?;
        self.repo.reset(
            head.as_object(),
            git2::ResetType::Hard,
            Some(CheckoutBuilder::new().remove_untracked(true)),
        )?;

        // Clean up state
        self.repo.cleanup_state()?;

        Ok("Cherry-pick aborted".to_string())
    }

    /// Get cherry-pick status
    pub fn pick_status(&self) -> Result<String> {
        if self.repo.state() == git2::RepositoryState::CherryPick {
            let index = self.repo.index()?;
            if index.has_conflicts() {
                let conflicts = self.get_conflicts(&index)?;
                Ok(format!("Cherry-picking with {} conflicts", conflicts.len()))
            } else {
                Ok("Cherry-pick in progress (no conflicts)".to_string())
            }
        } else {
            Ok("No cherry-pick in progress".to_string())
        }
    }

    /// Complete a cherry-pick by creating the commit
    fn complete_cherry_pick(&self, original_commit: &git2::Commit) -> Result<String> {
        // Get the tree from the index
        let mut index = self.repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;

        // Get HEAD as parent
        let head = self.repo.head()?.peel_to_commit()?;

        // Create commit message
        let message = format!(
            "{}\n\n(cherry picked from commit {})",
            original_commit.message().unwrap_or(""),
            original_commit.id()
        );

        // Get signature
        let sig = self.get_signature()?;

        // Create the commit
        let commit_oid = self.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &message,
            &tree,
            &[&head],
        )?;

        Ok(commit_oid.to_string())
    }

    /// Find commit from reference string
    fn find_commit_from_ref(&self, reference: &str) -> Result<git2::Commit> {
        // Try as commit SHA
        if let Ok(oid) = Oid::from_str(reference) {
            return Ok(self.repo.find_commit(oid)?);
        }

        // Try as reference
        if let Ok(reference) = self.repo.find_reference(reference) {
            return Ok(reference.peel_to_commit()?);
        }

        // Try revparse
        let obj = self.repo.revparse_single(reference)?;
        obj.peel_to_commit()
            .map_err(|e| anyhow::anyhow!("Failed to resolve '{}' to a commit: {}", reference, e))
    }

    /// Get list of conflicted files
    fn get_conflicts(&self, index: &Index) -> Result<Vec<String>> {
        let mut conflicts = Vec::new();

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

    /// Get signature for cherry-pick operations
    fn get_signature(&self) -> Result<git2::Signature<'static>> {
        let config = self.repo.config()?;

        let name = config.get_string("user.name")
            .unwrap_or_else(|_| "GitUp User".to_string());
        let email = config.get_string("user.email")
            .unwrap_or_else(|_| "gitup@local".to_string());

        Ok(git2::Signature::now(&name, &email)?)
    }
}