use anyhow::Result;
use git2::{
    Repository as Git2Repository, Oid, RebaseOptions,
    build::CheckoutBuilder, BranchType, Signature,
};
use std::path::Path;

/// Rebase result information
#[derive(Debug, Clone)]
pub struct RebaseResult {
    pub success: bool,
    pub message: String,
    pub rebased_commits: Vec<String>,
    pub conflicts: Vec<String>,
}

/// Rebase operation information
#[derive(Debug, Clone)]
pub struct RebaseOperation {
    pub operation_type: String,
    pub commit_id: String,
    pub message: String,
}

/// Rebase operations for a repository
pub struct RebaseOps {
    repo: Git2Repository,
}

impl RebaseOps {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self> {
        let repo = Git2Repository::open(repo_path)?;
        Ok(RebaseOps { repo })
    }

    /// Start an interactive rebase
    pub fn start_interactive(&self, upstream: &str, onto: Option<&str>) -> Result<RebaseResult> {
        // Get the upstream commit
        let upstream_commit = self.find_commit_from_ref(upstream)?;
        let annotated_upstream = self.repo.find_annotated_commit(upstream_commit.id())?;

        // Get the onto commit if specified
        let annotated_onto = if let Some(onto_ref) = onto {
            let onto_commit = self.find_commit_from_ref(onto_ref)?;
            Some(self.repo.find_annotated_commit(onto_commit.id())?)
        } else {
            None
        };

        // Get current branch
        let head = self.repo.head()?;
        let branch = self.repo.find_annotated_commit(head.target().unwrap())?;

        // Initialize rebase
        let mut rebase_opts = RebaseOptions::new();
        let mut rebase = self.repo.rebase(
            Some(&branch),
            Some(&annotated_upstream),
            annotated_onto.as_ref(),
            Some(&mut rebase_opts),
        )?;

        // Get the list of operations
        let mut operations = Vec::new();
        let operation_count = rebase.len();

        for i in 0..operation_count {
            let op = rebase.nth(i);
            if let Some(op) = op {
                operations.push(RebaseOperation {
                    operation_type: format!("{:?}", op.kind()),
                    commit_id: op.id().to_string(),
                    message: self.get_commit_message(op.id())?,
                });
            }
        }

        Ok(RebaseResult {
            success: true,
            message: format!("Interactive rebase initialized with {} commits", operation_count),
            rebased_commits: operations.iter().map(|op| op.commit_id.clone()).collect(),
            conflicts: vec![],
        })
    }

    /// Rebase current branch onto another branch
    pub fn rebase_onto(&self, target_branch: &str) -> Result<RebaseResult> {
        // Get target branch
        let target = self.repo.find_branch(target_branch, BranchType::Local)?;
        let target_commit = target.get().peel_to_commit()?;
        let annotated_target = self.repo.find_annotated_commit(target_commit.id())?;

        // Get current branch
        let head = self.repo.head()?;
        let current_commit = head.peel_to_commit()?;
        let annotated_current = self.repo.find_annotated_commit(current_commit.id())?;

        // Find merge base
        let merge_base = self.repo.merge_base(current_commit.id(), target_commit.id())?;
        let annotated_base = self.repo.find_annotated_commit(merge_base)?;

        // Start rebase
        let mut rebase_opts = RebaseOptions::new();
        let mut rebase = self.repo.rebase(
            Some(&annotated_current),
            Some(&annotated_base),
            Some(&annotated_target),
            Some(&mut rebase_opts),
        )?;

        let mut rebased_commits = Vec::new();
        let sig = self.get_signature()?;

        // Process each commit
        while let Some(op) = rebase.next() {
            match op {
                Ok(operation) => {
                    if let Err(e) = rebase.commit(None, &sig, None) {
                        // Check for conflicts
                        if self.repo.index()?.has_conflicts() {
                            let conflicts = self.get_conflicts()?;
                            return Ok(RebaseResult {
                                success: false,
                                message: format!("Rebase conflict at commit {}",
                                    operation.id().to_string()[..8].to_string()),
                                rebased_commits,
                                conflicts,
                            });
                        } else {
                            return Err(anyhow::anyhow!("Failed to commit during rebase: {}", e));
                        }
                    } else {
                        rebased_commits.push(operation.id().to_string());
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Rebase operation failed: {}", e));
                }
            }
        }

        // Finish rebase
        rebase.finish(Some(&sig))?;

        Ok(RebaseResult {
            success: true,
            message: format!("Successfully rebased {} commits onto {}",
                rebased_commits.len(), target_branch),
            rebased_commits,
            conflicts: vec![],
        })
    }

    /// Continue an in-progress rebase
    pub fn continue_rebase(&mut self) -> Result<RebaseResult> {
        // Open existing rebase
        let mut rebase = match self.repo.open_rebase(None) {
            Ok(r) => r,
            Err(_) => return Err(anyhow::anyhow!("No rebase in progress")),
        };

        // Check for conflicts
        let index = self.repo.index()?;
        if index.has_conflicts() {
            let conflicts = self.get_conflicts()?;
            return Ok(RebaseResult {
                success: false,
                message: "Conflicts must be resolved before continuing".to_string(),
                rebased_commits: vec![],
                conflicts,
            });
        }

        let sig = self.get_signature()?;
        let mut rebased_commits = Vec::new();

        // Commit the current operation
        if let Err(e) = rebase.commit(None, &sig, None) {
            return Err(anyhow::anyhow!("Failed to commit: {}", e));
        }

        // Continue with remaining operations
        while let Some(op) = rebase.next() {
            match op {
                Ok(operation) => {
                    if let Err(e) = rebase.commit(None, &sig, None) {
                        if self.repo.index()?.has_conflicts() {
                            let conflicts = self.get_conflicts()?;
                            return Ok(RebaseResult {
                                success: false,
                                message: format!("Rebase conflict at commit {}",
                                    operation.id().to_string()[..8].to_string()),
                                rebased_commits,
                                conflicts,
                            });
                        } else {
                            return Err(anyhow::anyhow!("Failed to commit during rebase: {}", e));
                        }
                    } else {
                        rebased_commits.push(operation.id().to_string());
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Rebase operation failed: {}", e));
                }
            }
        }

        // Finish rebase
        rebase.finish(Some(&sig))?;

        Ok(RebaseResult {
            success: true,
            message: "Rebase completed successfully".to_string(),
            rebased_commits,
            conflicts: vec![],
        })
    }

    /// Abort an in-progress rebase
    pub fn abort_rebase(&self) -> Result<String> {
        // Open existing rebase
        let mut rebase = match self.repo.open_rebase(None) {
            Ok(r) => r,
            Err(_) => return Err(anyhow::anyhow!("No rebase in progress")),
        };

        // Abort the rebase
        rebase.abort()?;

        Ok("Rebase aborted".to_string())
    }

    /// Skip current commit in rebase
    pub fn skip_commit(&mut self) -> Result<RebaseResult> {
        // Open existing rebase
        let mut rebase = match self.repo.open_rebase(None) {
            Ok(r) => r,
            Err(_) => return Err(anyhow::anyhow!("No rebase in progress")),
        };

        // Reset index and working directory
        let head = self.repo.head()?.peel_to_commit()?;
        self.repo.reset(
            head.as_object(),
            git2::ResetType::Hard,
            Some(&mut CheckoutBuilder::new()),
        )?;

        let sig = self.get_signature()?;
        let mut rebased_commits = Vec::new();

        // Continue with next operation
        while let Some(op) = rebase.next() {
            match op {
                Ok(operation) => {
                    if let Err(e) = rebase.commit(None, &sig, None) {
                        if self.repo.index()?.has_conflicts() {
                            let conflicts = self.get_conflicts()?;
                            return Ok(RebaseResult {
                                success: false,
                                message: format!("Rebase conflict at commit {}",
                                    operation.id().to_string()[..8].to_string()),
                                rebased_commits,
                                conflicts,
                            });
                        } else {
                            return Err(anyhow::anyhow!("Failed to commit during rebase: {}", e));
                        }
                    } else {
                        rebased_commits.push(operation.id().to_string());
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Rebase operation failed: {}", e));
                }
            }
        }

        // Finish rebase
        rebase.finish(Some(&sig))?;

        Ok(RebaseResult {
            success: true,
            message: "Skipped commit and completed rebase".to_string(),
            rebased_commits,
            conflicts: vec![],
        })
    }

    /// Get rebase status
    pub fn rebase_status(&self) -> Result<String> {
        match self.repo.open_rebase(None) {
            Ok(mut rebase) => {
                let current = rebase.operation_current();
                let total = rebase.len();

                if let Some(current_op) = current {
                    Ok(format!("Rebasing ({}/{})", current_op + 1, total))
                } else {
                    Ok(format!("Rebase in progress (0/{})", total))
                }
            }
            Err(_) => Ok("No rebase in progress".to_string()),
        }
    }

    /// Helper: Find commit from reference string
    fn find_commit_from_ref(&self, reference: &str) -> Result<git2::Commit> {
        // Try as branch name first
        if let Ok(branch) = self.repo.find_branch(reference, BranchType::Local) {
            return Ok(branch.get().peel_to_commit()?);
        }

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

    /// Get commit message
    fn get_commit_message(&self, oid: Oid) -> Result<String> {
        let commit = self.repo.find_commit(oid)?;
        Ok(commit.summary().unwrap_or("<no message>").to_string())
    }

    /// Get list of conflicted files
    fn get_conflicts(&self) -> Result<Vec<String>> {
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

    /// Get signature for rebase operations
    fn get_signature(&self) -> Result<Signature<'static>> {
        let config = self.repo.config()?;

        let name = config.get_string("user.name")
            .unwrap_or_else(|_| "GitUp User".to_string());
        let email = config.get_string("user.email")
            .unwrap_or_else(|_| "gitup@local".to_string());

        Ok(Signature::now(&name, &email)?)
    }
}