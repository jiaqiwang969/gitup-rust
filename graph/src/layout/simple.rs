use crate::core::{Dag, CommitNode};
use crate::layout::{Row, Lane, LaneIdx};
use std::collections::HashMap;

/// Simple graph builder that focuses on continuous lines
pub struct SimpleGraphBuilder {
    max_lanes: usize,
}

impl SimpleGraphBuilder {
    pub fn new(max_lanes: usize) -> Self {
        Self { max_lanes }
    }

    pub fn build_rows(&mut self, dag: &Dag) -> Vec<Row> {
        let mut rows = Vec::new();

        // Sort commits by timestamp (newest first)
        let mut commits: Vec<&CommitNode> = dag.nodes.values().collect();
        commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if commits.is_empty() {
            return rows;
        }

        // Track which lanes are actively used
        let mut active_lanes: Vec<Option<String>> = vec![None; self.max_lanes];
        let mut commit_lanes: HashMap<String, LaneIdx> = HashMap::new();

        // Build each row
        for commit in &commits {
            // Find or allocate a lane for this commit
            let primary_lane = if let Some(existing_lane) = commit_lanes.get(&commit.id) {
                *existing_lane
            } else {
                // Find first free lane
                let lane = active_lanes
                    .iter()
                    .position(|l| l.is_none())
                    .unwrap_or(0);
                commit_lanes.insert(commit.id.clone(), lane);
                lane
            };

            // Create the lanes array for this row
            let mut lanes = vec![Lane::Empty; self.max_lanes];

            // First, mark all lanes that have active commits passing through
            for (lane_idx, active_commit) in active_lanes.iter().enumerate() {
                if let Some(active_id) = active_commit {
                    if active_id != &commit.id {
                        // This lane has a different commit, draw a line through
                        lanes[lane_idx] = Lane::Pass;
                    }
                }
            }

            // Set the current commit
            lanes[primary_lane] = Lane::Commit;
            active_lanes[primary_lane] = Some(commit.id.clone());

            // Reserve lanes for parents
            for (i, parent_id) in commit.parents.iter().enumerate() {
                if !commit_lanes.contains_key(parent_id) {
                    // Allocate a lane for this parent
                    if i == 0 && active_lanes[primary_lane] == Some(commit.id.clone()) {
                        // First parent inherits the same lane
                        commit_lanes.insert(parent_id.clone(), primary_lane);
                    } else {
                        // Other parents get new lanes
                        if let Some(free_lane) = active_lanes.iter().position(|l| l.is_none()) {
                            commit_lanes.insert(parent_id.clone(), free_lane);
                            active_lanes[free_lane] = Some(parent_id.clone());
                            lanes[free_lane] = Lane::Pass;
                        }
                    }
                }
            }

            // Handle merge visualization
            if commit.parents.len() > 1 {
                let mut merge_targets = Vec::new();
                for parent_id in &commit.parents[1..] {
                    if let Some(&parent_lane) = commit_lanes.get(parent_id) {
                        if parent_lane != primary_lane {
                            merge_targets.push(parent_lane);
                            // Make sure the merge target lane is marked
                            if lanes[parent_lane] == Lane::Empty {
                                lanes[parent_lane] = Lane::Pass;
                            }
                        }
                    }
                }
                if !merge_targets.is_empty() {
                    lanes[primary_lane] = Lane::Merge(merge_targets);
                }
            }


            rows.push(Row {
                commit_id: commit.id.clone(),
                commit: (*commit).clone(),
                lanes,
                primary_lane,
            });

            // After processing, update active lanes for parent continuity
            if commit.parents.len() == 1 {
                // Single parent continues in the same lane
                active_lanes[primary_lane] = Some(commit.parents[0].clone());
            } else if commit.parents.is_empty() {
                // No parents, free the lane
                active_lanes[primary_lane] = None;
            }
            // For merge commits, the lane continues with first parent
            else if !commit.parents.is_empty() {
                active_lanes[primary_lane] = Some(commit.parents[0].clone());
            }
        }

        rows
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_simple_linear() {
        let mut dag = Dag::new();

        // Create linear history: c1 <- c2 <- c3
        let c1 = CommitNode::new(
            "c1".to_string(),
            vec![],
            Utc::now(),
            "Author".to_string(),
            "First commit".to_string(),
        );
        let c2 = CommitNode::new(
            "c2".to_string(),
            vec!["c1".to_string()],
            Utc::now(),
            "Author".to_string(),
            "Second commit".to_string(),
        );
        let c3 = CommitNode::new(
            "c3".to_string(),
            vec!["c2".to_string()],
            Utc::now(),
            "Author".to_string(),
            "Third commit".to_string(),
        );

        dag.add_node(c1);
        dag.add_node(c2);
        dag.add_node(c3);

        let mut builder = SimpleGraphBuilder::new(4);
        let rows = builder.build_rows(&dag);

        assert_eq!(rows.len(), 3);
        // All commits should be in lane 0
        for row in rows {
            assert_eq!(row.primary_lane, 0);
        }
    }
}