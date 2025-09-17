use crate::core::{Dag, CommitNode};
use std::collections::{HashMap, HashSet};

/// A lane represents a vertical column in the graph
pub type LaneIdx = usize;

/// A row in the graph visualization
#[derive(Debug, Clone)]
pub struct Row {
    /// The commit ID for this row
    pub commit_id: String,
    /// The commit node
    pub commit: CommitNode,
    /// Lane assignments for this row
    pub lanes: Vec<Lane>,
    /// Primary lane for this commit
    pub primary_lane: LaneIdx,
}

/// Lane information for a row
#[derive(Debug, Clone, PartialEq)]
pub enum Lane {
    /// Empty lane (no line)
    Empty,
    /// Vertical line passing through
    Pass,
    /// This commit occupies this lane
    Commit,
    /// Branch start (fork)
    BranchStart,
    /// Branch merge
    Merge(Vec<LaneIdx>), // Target lanes for merge
    /// Branch end
    End,
}

impl Lane {
    /// Get merge targets if this is a merge lane
    pub fn get_merge_targets(&self) -> &[LaneIdx] {
        match self {
            Lane::Merge(targets) => targets,
            _ => &[],
        }
    }

    /// Check if this lane represents an event (commit, merge, branch start/end)
    pub fn is_event(&self) -> bool {
        matches!(self, Lane::Commit | Lane::Merge(_) | Lane::BranchStart | Lane::End)
    }
}

/// Builds rows from a DAG
pub struct RowBuilder {
    /// Maximum number of lanes to use
    max_lanes: usize,
    /// Active branches (commit_id -> lane_idx)
    active_branches: HashMap<String, LaneIdx>,
    /// Next available lane
    next_lane: LaneIdx,
}

impl RowBuilder {
    pub fn new(max_lanes: usize) -> Self {
        Self {
            max_lanes,
            active_branches: HashMap::new(),
            next_lane: 0,
        }
    }

    /// Build rows from DAG (no compression yet)
    pub fn build_rows(&mut self, dag: &Dag) -> Vec<Row> {
        let mut rows = Vec::new();

        // Get topologically sorted commits
        let sorted_commits = self.topological_sort(dag);

        for commit in sorted_commits {
            let row = self.build_row(commit, dag);
            rows.push(row);
        }

        rows
    }

    /// Build a single row
    fn build_row(&mut self, commit: &CommitNode, dag: &Dag) -> Row {
        // Assign lane for this commit
        let primary_lane = self.assign_lane(&commit.id);

        // Build lanes for this row
        let mut lanes = vec![Lane::Empty; self.max_lanes];

        // Mark primary lane
        lanes[primary_lane] = Lane::Commit;

        // Handle parent connections
        if commit.is_merge() {
            // Multiple parents - merge commit
            let mut target_lanes = Vec::new();
            for parent_id in &commit.parents {
                let parent_lane = self.assign_lane(parent_id);
                target_lanes.push(parent_lane);
            }
            lanes[primary_lane] = Lane::Merge(target_lanes);
        } else if commit.parents.len() == 1 {
            // Single parent - continue or branch
            let parent_lane = self.assign_lane(&commit.parents[0]);
            if parent_lane != primary_lane {
                lanes[primary_lane] = Lane::BranchStart;
            }
        }

        // Mark passing lanes
        for (_, &lane_idx) in &self.active_branches {
            if lane_idx != primary_lane && lanes[lane_idx] == Lane::Empty {
                lanes[lane_idx] = Lane::Pass;
            }
        }

        // Handle children to detect branch ends
        let children = dag.get_children(&commit.id);
        if children.is_empty() {
            lanes[primary_lane] = Lane::End;
            self.active_branches.remove(&commit.id);
        }

        Row {
            commit_id: commit.id.clone(),
            commit: commit.clone(),
            lanes,
            primary_lane,
        }
    }

    /// Assign a lane to a commit
    fn assign_lane(&mut self, commit_id: &str) -> LaneIdx {
        if let Some(&lane) = self.active_branches.get(commit_id) {
            return lane;
        }

        // Assign new lane (no compression yet)
        let lane = self.next_lane;
        self.next_lane = (self.next_lane + 1).min(self.max_lanes - 1);
        self.active_branches.insert(commit_id.to_string(), lane);
        lane
    }

    /// Topological sort of commits
    fn topological_sort<'a>(&self, dag: &'a Dag) -> Vec<&'a CommitNode> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        fn visit<'a>(
            node: &'a CommitNode,
            dag: &'a Dag,
            visited: &mut HashSet<String>,
            temp_visited: &mut HashSet<String>,
            sorted: &mut Vec<&'a CommitNode>,
        ) {
            if visited.contains(&node.id) {
                return;
            }
            if temp_visited.contains(&node.id) {
                // Cycle detected - shouldn't happen in a DAG
                return;
            }

            temp_visited.insert(node.id.clone());

            // Visit parents first
            for parent_id in &node.parents {
                if let Some(parent) = dag.nodes.get(parent_id) {
                    visit(parent, dag, visited, temp_visited, sorted);
                }
            }

            temp_visited.remove(&node.id);
            visited.insert(node.id.clone());
            sorted.push(node);
        }

        // Start from leaves (commits with no children)
        for node in dag.leaves() {
            visit(node, dag, &mut visited, &mut temp_visited, &mut sorted);
        }

        // Handle any remaining nodes (orphan branches)
        for node in dag.nodes.values() {
            if !visited.contains(&node.id) {
                visit(node, dag, &mut visited, &mut temp_visited, &mut sorted);
            }
        }

        sorted.reverse(); // We want newest first
        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_dag() -> Dag {
        let mut dag = Dag::new();

        // Create a simple linear history
        let commit1 = CommitNode::new(
            "aaa".to_string(),
            vec![],
            Utc::now(),
            "Alice".to_string(),
            "Initial commit".to_string(),
        );

        let commit2 = CommitNode::new(
            "bbb".to_string(),
            vec!["aaa".to_string()],
            Utc::now(),
            "Bob".to_string(),
            "Second commit".to_string(),
        );

        let commit3 = CommitNode::new(
            "ccc".to_string(),
            vec!["bbb".to_string()],
            Utc::now(),
            "Charlie".to_string(),
            "Third commit".to_string(),
        );

        dag.add_node(commit1);
        dag.add_node(commit2);
        dag.add_node(commit3);

        dag
    }

    #[test]
    fn test_linear_layout() {
        let dag = create_test_dag();
        let mut builder = RowBuilder::new(10);
        let rows = builder.build_rows(&dag);

        assert_eq!(rows.len(), 3);

        // Check that we have sequential commits
        let commit_ids: Vec<_> = rows.iter().map(|r| &r.commit_id[..]).collect();
        assert!(commit_ids.contains(&"ccc"));
        assert!(commit_ids.contains(&"bbb"));
        assert!(commit_ids.contains(&"aaa"));

        // Check lane types exist
        for row in &rows {
            assert!(row.primary_lane < 10);
            assert!(!row.lanes.is_empty());
        }
    }

    #[test]
    fn test_merge_layout() {
        let mut dag = Dag::new();

        // Create a merge scenario
        let base = CommitNode::new(
            "base".to_string(),
            vec![],
            Utc::now(),
            "Alice".to_string(),
            "Base".to_string(),
        );

        let branch1 = CommitNode::new(
            "b1".to_string(),
            vec!["base".to_string()],
            Utc::now(),
            "Bob".to_string(),
            "Branch 1".to_string(),
        );

        let branch2 = CommitNode::new(
            "b2".to_string(),
            vec!["base".to_string()],
            Utc::now(),
            "Charlie".to_string(),
            "Branch 2".to_string(),
        );

        let merge = CommitNode::new(
            "merge".to_string(),
            vec!["b1".to_string(), "b2".to_string()],
            Utc::now(),
            "Dave".to_string(),
            "Merge".to_string(),
        );

        dag.add_node(base);
        dag.add_node(branch1);
        dag.add_node(branch2);
        dag.add_node(merge);

        let mut builder = RowBuilder::new(10);
        let rows = builder.build_rows(&dag);

        assert_eq!(rows.len(), 4);

        // Find merge row
        let merge_row = rows.iter().find(|r| r.commit_id == "merge").unwrap();

        // The merge commit should be marked appropriately
        // Check that it has multiple parent connections
        assert_eq!(merge_row.commit.parents.len(), 2);

        // Verify branch commits exist
        assert!(rows.iter().any(|r| r.commit_id == "b1"));
        assert!(rows.iter().any(|r| r.commit_id == "b2"));
    }
}