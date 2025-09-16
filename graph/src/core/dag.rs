use super::{node::CommitNode, edge::Edge};
use std::collections::HashMap;

/// Directed Acyclic Graph representing commit history
#[derive(Debug, Clone)]
pub struct Dag {
    /// All nodes indexed by commit ID
    pub nodes: HashMap<String, CommitNode>,
    /// All edges in the graph
    pub edges: Vec<Edge>,
    /// Quick lookup: commit ID -> children IDs
    pub children: HashMap<String, Vec<String>>,
}

impl Dag {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            children: HashMap::new(),
        }
    }

    /// Add a commit node to the DAG
    pub fn add_node(&mut self, node: CommitNode) {
        let id = node.id.clone();

        // Add edges for each parent
        for parent_id in &node.parents {
            let edge = if node.parents.len() > 1 {
                Edge::merge(id.clone(), parent_id.clone())
            } else {
                Edge::new(id.clone(), parent_id.clone())
            };
            self.edges.push(edge);

            // Update children map
            self.children
                .entry(parent_id.clone())
                .or_insert_with(Vec::new)
                .push(id.clone());
        }

        self.nodes.insert(id, node);
    }

    /// Get all root commits (no parents)
    pub fn roots(&self) -> Vec<&CommitNode> {
        self.nodes
            .values()
            .filter(|node| node.is_root())
            .collect()
    }

    /// Get all leaf commits (no children)
    pub fn leaves(&self) -> Vec<&CommitNode> {
        self.nodes
            .values()
            .filter(|node| !self.children.contains_key(&node.id))
            .collect()
    }

    /// Get children of a commit
    pub fn get_children(&self, commit_id: &str) -> Vec<&CommitNode> {
        self.children
            .get(commit_id)
            .map(|child_ids| {
                child_ids
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get parents of a commit
    pub fn get_parents(&self, commit_id: &str) -> Vec<&CommitNode> {
        self.nodes
            .get(commit_id)
            .map(|node| {
                node.parents
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Count of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Count of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if DAG contains orphan branches
    pub fn has_orphan_branches(&self) -> bool {
        self.roots().len() > 1
    }

    /// Get statistics about the DAG
    pub fn stats(&self) -> DagStats {
        let merge_commits = self.nodes.values().filter(|n| n.is_merge()).count();
        let root_commits = self.roots().len();
        let leaf_commits = self.leaves().len();

        DagStats {
            total_commits: self.nodes.len(),
            total_edges: self.edges.len(),
            merge_commits,
            root_commits,
            leaf_commits,
            has_orphans: self.has_orphan_branches(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DagStats {
    pub total_commits: usize,
    pub total_edges: usize,
    pub merge_commits: usize,
    pub root_commits: usize,
    pub leaf_commits: usize,
    pub has_orphans: bool,
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}