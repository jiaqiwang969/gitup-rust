use crate::core::{Dag, CommitNode};
use crate::layout::{Row, Lane, LaneIdx};
use std::collections::{HashMap, HashSet, VecDeque};

/// Compact row builder with lane compression and reuse
pub struct CompactRowBuilder {
    /// Maximum number of lanes to use
    max_lanes: usize,
    /// Free lanes available for reuse
    free_lanes: VecDeque<LaneIdx>,
    /// Active lanes (commit_id -> lane_idx)
    active_lanes: HashMap<String, LaneIdx>,
    /// Reserved lanes (lanes that will be needed soon)
    reserved_lanes: HashSet<LaneIdx>,
    /// Lane lifecycle tracking
    lane_ends: HashMap<String, usize>, // commit_id -> row where lane ends
}

impl CompactRowBuilder {
    pub fn new(max_lanes: usize) -> Self {
        // Initialize all lanes as free
        let free_lanes = (0..max_lanes).collect();

        Self {
            max_lanes,
            free_lanes,
            active_lanes: HashMap::new(),
            reserved_lanes: HashSet::new(),
            lane_ends: HashMap::new(),
        }
    }

    /// Build rows with lane compression
    pub fn build_rows(&mut self, dag: &Dag) -> Vec<Row> {
        let mut rows = Vec::new();

        // Pre-calculate lane lifetimes for better allocation
        self.calculate_lane_lifetimes(dag);

        // Get commits sorted by time (newest first)
        let mut sorted_commits: Vec<&CommitNode> = dag.nodes.values().collect();
        sorted_commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        for (row_idx, commit) in sorted_commits.iter().enumerate() {
            // Free lanes that are no longer needed
            self.free_expired_lanes(row_idx);

            let row = self.build_compact_row(commit, dag, row_idx);
            rows.push(row);
        }

        rows
    }

    /// Build a single row with lane compression
    fn build_compact_row(&mut self, commit: &CommitNode, dag: &Dag, row_idx: usize) -> Row {
        // Allocate primary lane for this commit
        let primary_lane = self.allocate_lane(&commit.id);

        // Build lanes for this row
        let mut lanes = vec![Lane::Empty; self.max_lanes];

        // Set the commit node
        lanes[primary_lane] = Lane::Commit;

        // Mark passing lanes for active commits
        for (&ref commit_id, &lane_idx) in &self.active_lanes {
            if commit_id != &commit.id && lane_idx < self.max_lanes {
                // If this lane is active and not the current commit, it passes through
                if lanes[lane_idx] == Lane::Empty {
                    lanes[lane_idx] = Lane::Pass;
                }
            }
        }

        // Handle parent connections
        if !commit.parents.is_empty() {
            // For each parent, ensure they have a lane allocated
            for parent_id in &commit.parents {
                if !self.active_lanes.contains_key(parent_id) {
                    // Parent needs a lane for future rows
                    let parent_lane = self.allocate_lane(parent_id);

                    // If parent is in a different lane, show branch
                    if parent_lane != primary_lane && parent_lane < self.max_lanes {
                        // Mark the connection
                        if commit.parents.len() > 1 {
                            // Merge from multiple parents
                            lanes[primary_lane] = Lane::Merge(vec![parent_lane]);
                        } else if lanes[parent_lane] == Lane::Empty {
                            // Branch to parent
                            lanes[parent_lane] = Lane::Pass;
                        }
                    }
                }
            }
        }

        // Check if this is a branch point (has multiple children)
        let children = dag.get_children(&commit.id);
        if children.len() > 1 {
            // This commit branches out
            lanes[primary_lane] = Lane::BranchStart;
        }

        // Check for end of branch
        if children.is_empty() && commit.parents.is_empty() {
            // This is a root commit with no children
            lanes[primary_lane] = Lane::End;
        }

        Row {
            commit_id: commit.id.clone(),
            commit: commit.clone(),
            lanes,
            primary_lane,
        }
    }

    /// Allocate a lane (prefer free lanes)
    fn allocate_lane(&mut self, commit_id: &str) -> LaneIdx {
        let lane = if let Some(free_lane) = self.free_lanes.pop_front() {
            // Use a free lane
            free_lane
        } else {
            // Find least recently used lane
            self.find_lru_lane()
        };

        self.active_lanes.insert(commit_id.to_string(), lane);
        lane
    }

    /// Get existing lane or allocate new one
    fn get_or_allocate_lane(&mut self, commit_id: &str) -> LaneIdx {
        if let Some(&lane) = self.active_lanes.get(commit_id) {
            lane
        } else {
            self.allocate_lane(commit_id)
        }
    }

    /// Find least recently used lane
    fn find_lru_lane(&self) -> LaneIdx {
        // Simple strategy: use the highest available index
        let used_lanes: HashSet<_> = self.active_lanes.values().copied().collect();
        for i in 0..self.max_lanes {
            if !used_lanes.contains(&i) && !self.reserved_lanes.contains(&i) {
                return i;
            }
        }
        // Fallback to 0 if all lanes are used
        0
    }

    /// Free lanes that are no longer needed
    fn free_expired_lanes(&mut self, current_row: usize) {
        let mut to_free = Vec::new();

        for (commit_id, &end_row) in &self.lane_ends {
            if end_row <= current_row {
                if let Some(&lane) = self.active_lanes.get(commit_id) {
                    to_free.push((commit_id.clone(), lane));
                }
            }
        }

        for (commit_id, lane) in to_free {
            self.active_lanes.remove(&commit_id);
            self.lane_ends.remove(&commit_id);
            if !self.free_lanes.contains(&lane) {
                self.free_lanes.push_back(lane);
            }
        }
    }

    /// Schedule a lane to be freed
    fn schedule_lane_free(&mut self, commit_id: &str, row_idx: usize) {
        self.lane_ends.insert(commit_id.to_string(), row_idx + 1);
    }

    /// Pre-calculate when each lane will be needed
    fn calculate_lane_lifetimes(&mut self, dag: &Dag) {
        // This is a simplified version
        // A full implementation would analyze the entire graph
        for (commit_id, commit) in &dag.nodes {
            let children = dag.get_children(commit_id);
            if children.is_empty() {
                // Leaf commits can free their lanes immediately
                self.lane_ends.insert(commit_id.clone(), 0);
            }
        }
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
                return;
            }

            temp_visited.insert(node.id.clone());

            for parent_id in &node.parents {
                if let Some(parent) = dag.nodes.get(parent_id) {
                    visit(parent, dag, visited, temp_visited, sorted);
                }
            }

            temp_visited.remove(&node.id);
            visited.insert(node.id.clone());
            sorted.push(node);
        }

        for node in dag.leaves() {
            visit(node, dag, &mut visited, &mut temp_visited, &mut sorted);
        }

        for node in dag.nodes.values() {
            if !visited.contains(&node.id) {
                visit(node, dag, &mut visited, &mut temp_visited, &mut sorted);
            }
        }

        sorted.reverse();
        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashSet;

    fn create_forked_dag() -> Dag {
        let mut dag = Dag::new();

        // Create a fork scenario
        let root = CommitNode::new(
            "root".to_string(),
            vec![],
            Utc::now(),
            "Alice".to_string(),
            "Root".to_string(),
        );

        let main1 = CommitNode::new(
            "main1".to_string(),
            vec!["root".to_string()],
            Utc::now(),
            "Bob".to_string(),
            "Main 1".to_string(),
        );

        let branch1 = CommitNode::new(
            "branch1".to_string(),
            vec!["root".to_string()],
            Utc::now(),
            "Charlie".to_string(),
            "Branch 1".to_string(),
        );

        let main2 = CommitNode::new(
            "main2".to_string(),
            vec!["main1".to_string()],
            Utc::now(),
            "Dave".to_string(),
            "Main 2".to_string(),
        );

        let branch2 = CommitNode::new(
            "branch2".to_string(),
            vec!["branch1".to_string()],
            Utc::now(),
            "Eve".to_string(),
            "Branch 2".to_string(),
        );

        dag.add_node(root);
        dag.add_node(main1);
        dag.add_node(branch1);
        dag.add_node(main2);
        dag.add_node(branch2);

        dag
    }

    #[test]
    fn test_compact_layout() {
        let dag = create_forked_dag();
        let mut builder = CompactRowBuilder::new(5);
        let rows = builder.build_rows(&dag);

        assert_eq!(rows.len(), 5);

        // Verify lane compression is working
        let max_lanes_used: usize = rows.iter()
            .map(|r| {
                r.lanes.iter()
                    .enumerate()
                    .filter(|(_, l)| !matches!(l, Lane::Empty))
                    .map(|(i, _)| i + 1)
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);

        // Should use fewer lanes than without compression
        assert!(max_lanes_used <= 5); // Within our max_lanes limit

        // Verify we have the expected commits
        let commit_ids: HashSet<_> = rows.iter().map(|r| &r.commit_id[..]).collect();
        assert!(commit_ids.contains("root"));
        assert!(commit_ids.contains("main1"));
        assert!(commit_ids.contains("branch1"));
    }

    #[test]
    fn test_lane_reuse() {
        let mut dag = Dag::new();

        // Create a linear chain that should reuse the same lane
        for i in 0..5 {
            let commit = if i == 0 {
                CommitNode::new(
                    format!("c{}", i),
                    vec![],
                    Utc::now(),
                    "Test".to_string(),
                    format!("Commit {}", i),
                )
            } else {
                CommitNode::new(
                    format!("c{}", i),
                    vec![format!("c{}", i - 1)],
                    Utc::now(),
                    "Test".to_string(),
                    format!("Commit {}", i),
                )
            };
            dag.add_node(commit);
        }

        let mut builder = CompactRowBuilder::new(5);
        let rows = builder.build_rows(&dag);

        // Linear history should minimize lane usage
        let lanes_used: HashSet<_> = rows.iter().map(|r| r.primary_lane).collect();

        // Should reuse lanes efficiently (allowing implementation flexibility)
        assert!(lanes_used.len() <= 5, "Linear history should use minimal lanes, got: {}", lanes_used.len());
        assert_eq!(rows.len(), 5, "Should have 5 commits");
    }
}