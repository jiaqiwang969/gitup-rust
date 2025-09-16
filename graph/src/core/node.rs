use chrono::{DateTime, Utc};

/// A commit node in the DAG
#[derive(Debug, Clone)]
pub struct CommitNode {
    /// Unique commit ID (SHA)
    pub id: String,
    /// Parent commit IDs
    pub parents: Vec<String>,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Author name
    pub author: String,
    /// Commit message (short)
    pub message: String,
}

impl CommitNode {
    pub fn new(
        id: String,
        parents: Vec<String>,
        timestamp: DateTime<Utc>,
        author: String,
        message: String,
    ) -> Self {
        Self {
            id,
            parents,
            timestamp,
            author,
            message,
        }
    }

    /// Check if this is a root commit (no parents)
    pub fn is_root(&self) -> bool {
        self.parents.is_empty()
    }

    /// Check if this is a merge commit (multiple parents)
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }
}