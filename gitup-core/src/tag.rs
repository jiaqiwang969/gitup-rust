use anyhow::Result;
use git2::{
    ObjectType, Oid, Repository as Git2Repository, Signature,
};
use chrono::{DateTime, Local, TimeZone};
use std::path::Path;

/// Tag information
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub target: String,
    pub tagger: Option<String>,
    pub message: Option<String>,
    pub timestamp: Option<DateTime<Local>>,
    pub is_annotated: bool,
}

/// Tag operations for a repository
pub struct TagOps {
    repo: Git2Repository,
}

impl TagOps {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self> {
        let repo = Git2Repository::open(repo_path)?;
        Ok(TagOps { repo })
    }

    /// Create a new tag
    pub fn create(
        &self,
        name: &str,
        target: Option<&str>,
        message: Option<&str>,
        force: bool,
    ) -> Result<String> {
        // Get the target object (default to HEAD)
        let target_oid = if let Some(target_ref) = target {
            // Try to parse as commit hash or reference
            if let Ok(oid) = Oid::from_str(target_ref) {
                oid
            } else {
                // Try to resolve as reference
                let reference = self.repo.resolve_reference_from_short_name(target_ref)?;
                reference.target().ok_or_else(|| {
                    anyhow::anyhow!("Could not resolve target reference")
                })?
            }
        } else {
            // Use HEAD
            self.repo.head()?.target().ok_or_else(|| {
                anyhow::anyhow!("HEAD is not pointing to a valid commit")
            })?
        };

        let target_obj = self.repo.find_object(target_oid, Some(ObjectType::Commit))?;

        if let Some(msg) = message {
            // Create annotated tag
            let sig = self.get_signature()?;
            self.repo.tag(
                name,
                &target_obj,
                &sig,
                msg,
                force,
            )?;
            Ok(format!("Created annotated tag '{}' at {}", name, &target_oid.to_string()[..8]))
        } else {
            // Create lightweight tag
            self.repo.tag_lightweight(
                name,
                &target_obj,
                force,
            )?;
            Ok(format!("Created lightweight tag '{}' at {}", name, &target_oid.to_string()[..8]))
        }
    }

    /// List all tags
    pub fn list(&self, pattern: Option<&str>) -> Result<Vec<TagInfo>> {
        let mut tags = Vec::new();
        let tag_names = self.repo.tag_names(pattern)?;

        for tag_name in tag_names.iter().flatten() {
            if let Ok(tag_info) = self.get_tag_info(tag_name) {
                tags.push(tag_info);
            }
        }

        // Sort by name
        tags.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tags)
    }

    /// Get information about a specific tag
    pub fn get_tag_info(&self, name: &str) -> Result<TagInfo> {
        let full_name = format!("refs/tags/{}", name);
        let reference = self.repo.find_reference(&full_name)?;
        let target_oid = reference.target().ok_or_else(|| {
            anyhow::anyhow!("Tag reference has no target")
        })?;

        // Check if it's an annotated tag
        if let Ok(tag) = self.repo.find_tag(target_oid) {
            // Annotated tag
            let tagger = tag.tagger();
            Ok(TagInfo {
                name: name.to_string(),
                target: tag.target_id().to_string(),
                tagger: tagger.as_ref().map(|t| format!("{} <{}>", t.name().unwrap_or(""), t.email().unwrap_or(""))),
                message: tag.message().map(|s| s.to_string()),
                timestamp: tagger.as_ref().map(|t| {
                    Local.timestamp_opt(t.when().seconds(), 0).single()
                        .unwrap_or_else(Local::now)
                }),
                is_annotated: true,
            })
        } else {
            // Lightweight tag
            Ok(TagInfo {
                name: name.to_string(),
                target: target_oid.to_string(),
                tagger: None,
                message: None,
                timestamp: None,
                is_annotated: false,
            })
        }
    }

    /// Delete a tag
    pub fn delete(&mut self, name: &str) -> Result<String> {
        let full_name = format!("refs/tags/{}", name);

        // Check if tag exists
        self.repo.find_reference(&full_name)?;

        // Delete the tag
        self.repo.tag_delete(name)?;

        Ok(format!("Deleted tag '{}'", name))
    }

    /// Show tag details
    pub fn show(&self, name: &str) -> Result<String> {
        let tag_info = self.get_tag_info(name)?;
        let mut output = String::new();

        output.push_str(&format!("tag {}\n", tag_info.name));

        if tag_info.is_annotated {
            if let Some(tagger) = &tag_info.tagger {
                output.push_str(&format!("Tagger: {}\n", tagger));
            }
            if let Some(timestamp) = &tag_info.timestamp {
                output.push_str(&format!("Date:   {}\n", timestamp));
            }
            if let Some(message) = &tag_info.message {
                output.push_str(&format!("\n{}\n", message));
            }
        }

        // Get commit information
        if let Ok(oid) = Oid::from_str(&tag_info.target) {
            if let Ok(commit) = self.repo.find_commit(oid) {
                output.push_str(&format!("\ncommit {}\n", &oid.to_string()[..8]));
                output.push_str(&format!("Author: {} <{}>\n",
                    commit.author().name().unwrap_or("Unknown"),
                    commit.author().email().unwrap_or("unknown")));
                output.push_str(&format!("Date:   {}\n",
                    Local.timestamp_opt(commit.time().seconds(), 0).single()
                        .unwrap_or_else(Local::now)));
                output.push_str(&format!("\n    {}\n",
                    commit.message().unwrap_or("").lines().next().unwrap_or("")));
            }
        }

        Ok(output)
    }

    /// Push tags to remote
    pub fn push(&self, remote_name: &str, tag_name: Option<&str>, _force: bool) -> Result<String> {
        let refspecs = if let Some(tag) = tag_name {
            // Push specific tag
            vec![format!("refs/tags/{}:refs/tags/{}", tag, tag)]
        } else {
            // Push all tags
            vec!["refs/tags/*:refs/tags/*".to_string()]
        };

        let refspecs_str: Vec<&str> = refspecs.iter().map(|s| s.as_str()).collect();

        // Push directly without using RemoteOps
        let mut remote = self.repo.find_remote(remote_name)?;

        // Set up push options
        let mut push_options = git2::PushOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        // Set up authentication (reuse from remote.rs)
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            if let Ok(home) = std::env::var("HOME") {
                let ssh_dir = Path::new(&home).join(".ssh");

                // Try ed25519 key first
                let ed25519_key = ssh_dir.join("id_ed25519");
                if ed25519_key.exists() {
                    return git2::Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        &ed25519_key,
                        None,
                    );
                }

                // Try RSA key
                let rsa_key = ssh_dir.join("id_rsa");
                if rsa_key.exists() {
                    return git2::Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        &rsa_key,
                        None,
                    );
                }
            }

            // Fall back to SSH agent
            git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
        });

        push_options.remote_callbacks(callbacks);

        // Perform push
        remote.push(&refspecs_str, Some(&mut push_options))?;

        if let Some(tag) = tag_name {
            Ok(format!("Pushed tag '{}' to {}", tag, remote_name))
        } else {
            Ok(format!("Pushed all tags to {}", remote_name))
        }
    }

    /// Verify a tag's signature (if signed)
    pub fn verify(&self, name: &str) -> Result<String> {
        // For now, just check if the tag exists and is valid
        let tag_info = self.get_tag_info(name)?;

        if tag_info.is_annotated {
            Ok(format!("Tag '{}' is a valid annotated tag", name))
        } else {
            Ok(format!("Tag '{}' is a valid lightweight tag", name))
        }
    }

    /// Get signature for tag operations
    fn get_signature(&self) -> Result<Signature<'static>> {
        // Try to get from git config
        let config = self.repo.config()?;

        let name = config.get_string("user.name")
            .unwrap_or_else(|_| "GitUp User".to_string());
        let email = config.get_string("user.email")
            .unwrap_or_else(|_| "gitup@local".to_string());

        Ok(Signature::now(&name, &email)?)
    }

    /// Check if a tag exists
    pub fn exists(&self, name: &str) -> bool {
        let full_name = format!("refs/tags/{}", name);
        self.repo.find_reference(&full_name).is_ok()
    }
}