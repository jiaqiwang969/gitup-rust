use anyhow::Result;
use git2::{
    BranchType, Cred, Direction, FetchOptions, PushOptions, RemoteCallbacks,
    Repository as Git2Repository,
};
use std::path::Path;

/// Remote repository information
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub fetch_url: Option<String>,
    pub push_url: Option<String>,
}

/// Transfer progress information
#[derive(Debug, Clone)]
pub struct TransferProgress {
    pub total_objects: usize,
    pub indexed_objects: usize,
    pub received_objects: usize,
    pub local_objects: usize,
    pub total_deltas: usize,
    pub indexed_deltas: usize,
    pub received_bytes: usize,
}

impl From<git2::Progress<'_>> for TransferProgress {
    fn from(progress: git2::Progress) -> Self {
        TransferProgress {
            total_objects: progress.total_objects(),
            indexed_objects: progress.indexed_objects(),
            received_objects: progress.received_objects(),
            local_objects: progress.local_objects(),
            total_deltas: progress.total_deltas(),
            indexed_deltas: progress.indexed_deltas(),
            received_bytes: progress.received_bytes(),
        }
    }
}

/// Remote operations for a repository
pub struct RemoteOps<'a> {
    repo: &'a Git2Repository,
}

impl<'a> RemoteOps<'a> {
    pub fn new(repo: &'a Git2Repository) -> Self {
        RemoteOps { repo }
    }

    /// List all remotes
    pub fn list_remotes(&self) -> Result<Vec<RemoteInfo>> {
        let remotes = self.repo.remotes()?;
        let mut result = Vec::new();

        for remote_name in remotes.iter().flatten() {
            if let Ok(remote) = self.repo.find_remote(remote_name) {
                result.push(RemoteInfo {
                    name: remote_name.to_string(),
                    url: remote.url().unwrap_or("").to_string(),
                    fetch_url: remote.url().map(|s| s.to_string()),
                    push_url: remote.pushurl().map(|s| s.to_string()),
                });
            }
        }

        Ok(result)
    }

    /// Add a new remote
    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        self.repo.remote(name, url)?;
        Ok(())
    }

    /// Remove a remote
    pub fn remove_remote(&self, name: &str) -> Result<()> {
        self.repo.remote_delete(name)?;
        Ok(())
    }

    /// Rename a remote
    pub fn rename_remote(&self, old_name: &str, new_name: &str) -> Result<()> {
        self.repo.remote_rename(old_name, new_name)?;
        Ok(())
    }

    /// Set remote URL
    pub fn set_remote_url(&self, name: &str, url: &str) -> Result<()> {
        self.repo.remote_set_url(name, url)?;
        Ok(())
    }

    /// Fetch from remote
    pub fn fetch(
        &self,
        remote_name: &str,
        refspecs: &[&str],
        progress_callback: Option<Box<dyn FnMut(TransferProgress) + '_>>,
    ) -> Result<String> {
        let mut remote = self.repo.find_remote(remote_name)?;
        let mut callbacks = RemoteCallbacks::new();

        // Set up progress callback if provided
        if let Some(mut callback) = progress_callback {
            callbacks.transfer_progress(move |progress| {
                callback(progress.into());
                true
            });
        }

        // Set up authentication
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            // Try different SSH key types
            if let Ok(home) = std::env::var("HOME") {
                let ssh_dir = Path::new(&home).join(".ssh");

                // Try ed25519 key first (modern and secure)
                let ed25519_key = ssh_dir.join("id_ed25519");
                if ed25519_key.exists() {
                    return Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        &ed25519_key,
                        None,
                    );
                }

                // Try RSA key
                let rsa_key = ssh_dir.join("id_rsa");
                if rsa_key.exists() {
                    return Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        &rsa_key,
                        None,
                    );
                }

                // Try ECDSA key
                let ecdsa_key = ssh_dir.join("id_ecdsa");
                if ecdsa_key.exists() {
                    return Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        &ecdsa_key,
                        None,
                    );
                }
            }

            // Fall back to SSH agent
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        // Perform fetch
        remote.fetch(refspecs, Some(&mut fetch_options), None)?;

        // Get fetch head information
        let stats = remote.stats();
        let msg = format!(
            "Fetched {} objects, {} bytes",
            stats.received_objects(),
            stats.received_bytes()
        );

        Ok(msg)
    }

    /// Pull from remote (fetch + merge)
    pub fn pull(
        &self,
        remote_name: &str,
        branch_name: &str,
        progress_callback: Option<Box<dyn FnMut(TransferProgress) + '_>>,
    ) -> Result<String> {
        // First fetch
        let fetch_msg = self.fetch(remote_name, &[], progress_callback)?;

        // Get current branch
        let head = self.repo.head()?;
        let current_branch_name = head
            .shorthand()
            .ok_or_else(|| anyhow::anyhow!("Could not get current branch name"))?;

        // Find the remote branch
        let remote_branch_name = format!("{}/{}", remote_name, branch_name);
        let remote_branch = self.repo.find_branch(&remote_branch_name, BranchType::Remote)?;
        let remote_commit = remote_branch.get().peel_to_commit()?;

        // Get current commit
        let head_commit = head.peel_to_commit()?;

        // Check if we can fast-forward
        let merge_base = self.repo.merge_base(head_commit.id(), remote_commit.id())?;

        if merge_base == remote_commit.id() {
            // Already up-to-date
            Ok(format!("Already up-to-date. {}", fetch_msg))
        } else if merge_base == head_commit.id() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", current_branch_name);
            let mut reference = self.repo.find_reference(&refname)?;
            reference.set_target(
                remote_commit.id(),
                &format!("Fast-forward to {}", remote_commit.id()),
            )?;

            // Checkout the new commit
            self.repo.checkout_head(Some(
                git2::build::CheckoutBuilder::new()
                    .force()
                    .remove_untracked(false),
            ))?;

            Ok(format!(
                "Fast-forwarded to {}. {}",
                &remote_commit.id().to_string()[..8],
                fetch_msg
            ))
        } else {
            // Would need a real merge - not implemented yet
            Err(anyhow::anyhow!(
                "Non-fast-forward merge required. Please merge manually."
            ))
        }
    }

    /// Push to remote
    pub fn push(
        &self,
        remote_name: &str,
        refspecs: &[&str],
        progress_callback: Option<Box<dyn FnMut(TransferProgress) + '_>>,
    ) -> Result<String> {
        let mut remote = self.repo.find_remote(remote_name)?;
        let mut callbacks = RemoteCallbacks::new();

        // Set up progress callback if provided
        if let Some(mut callback) = progress_callback {
            callbacks.transfer_progress(move |progress| {
                callback(progress.into());
                true
            });
        }

        // Set up authentication
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            // Try different SSH key types
            if let Ok(home) = std::env::var("HOME") {
                let ssh_dir = Path::new(&home).join(".ssh");

                // Try ed25519 key first (modern and secure)
                let ed25519_key = ssh_dir.join("id_ed25519");
                if ed25519_key.exists() {
                    return Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        &ed25519_key,
                        None,
                    );
                }

                // Try RSA key
                let rsa_key = ssh_dir.join("id_rsa");
                if rsa_key.exists() {
                    return Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        &rsa_key,
                        None,
                    );
                }

                // Try ECDSA key
                let ecdsa_key = ssh_dir.join("id_ecdsa");
                if ecdsa_key.exists() {
                    return Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        &ecdsa_key,
                        None,
                    );
                }
            }

            // Fall back to SSH agent
            Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        callbacks.push_update_reference(|refname, status| {
            if let Some(msg) = status {
                eprintln!("Push failed for {}: {}", refname, msg);
            }
            Ok(())
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        // If no refspecs provided, push current branch
        let refspecs_to_push: Vec<String>;
        let final_refspecs = if refspecs.is_empty() {
            let head = self.repo.head()?;
            let branch_name = head
                .shorthand()
                .ok_or_else(|| anyhow::anyhow!("Could not get current branch name"))?;
            refspecs_to_push = vec![format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name)];
            refspecs_to_push.iter().map(|s| s.as_str()).collect::<Vec<_>>()
        } else {
            refspecs.to_vec()
        };

        // Perform push
        remote.push(&final_refspecs, Some(&mut push_options))?;

        Ok(format!("Pushed to {}", remote_name))
    }

    /// Get remote tracking branch for current branch
    pub fn get_upstream(&self) -> Result<Option<(String, String)>> {
        let head = self.repo.head()?;
        if !head.is_branch() {
            return Ok(None);
        }

        let branch = self.repo.find_branch(
            head.shorthand().unwrap_or(""),
            BranchType::Local,
        )?;

        match branch.upstream() {
            Ok(upstream) => {
                let name = upstream.name()?.unwrap_or("").to_string();
                // Parse remote and branch from name like "origin/main"
                if let Some(slash_pos) = name.find('/') {
                    let remote = &name[..slash_pos];
                    let branch = &name[slash_pos + 1..];
                    Ok(Some((remote.to_string(), branch.to_string())))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }

    /// Set upstream for current branch
    pub fn set_upstream(&self, remote_name: &str, branch_name: &str) -> Result<()> {
        let mut branch = self.repo.find_branch(
            self.repo.head()?.shorthand().unwrap_or(""),
            BranchType::Local,
        )?;

        let upstream_name = format!("{}/{}", remote_name, branch_name);
        branch.set_upstream(Some(&upstream_name))?;

        Ok(())
    }
}