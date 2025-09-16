use crate::core::{Dag, CommitNode};
use crate::layout::{Row, Lane, LaneIdx};
use std::collections::HashMap;

/// Simple graph builder that focuses on continuous lines
pub struct SimpleGraphBuilder {
    lanes: Vec<Option<String>>, // lane -> commit_id mapping
    max_lanes: usize,
}

impl SimpleGraphBuilder {
    pub fn new(max_lanes: usize) -> Self {
        Self {
            lanes: vec![None; max_lanes],
            max_lanes,
        }
    }

    pub fn build_rows(&mut self, dag: &Dag) -> Vec<Row> {
        let mut rows = Vec::new();

        // Sort commits by timestamp (newest first)
        let mut commits: Vec<&CommitNode> = dag.nodes.values().collect();
        commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Pre-allocate lanes for all commits to ensure continuity
        let mut commit_lanes: HashMap<String, LaneIdx> = HashMap::new();
        let mut next_lane = 0;

        // First pass: assign lanes to ensure linear history stays in same lane
        for commit in &commits {
            if !commit_lanes.contains_key(&commit.id) {
                commit_lanes.insert(commit.id.clone(), next_lane);

                // If this has a single parent, try to keep it in the same lane
                if commit.parents.len() == 1 {
                    let parent_id = &commit.parents[0];
                    if !commit_lanes.contains_key(parent_id) {
                        commit_lanes.insert(parent_id.clone(), next_lane);
                    }
                } else {
                    next_lane = (next_lane + 1) % self.max_lanes;
                }
            }
        }

        // Second pass: build rows with proper lanes
        for commit in &commits {
            let primary_lane = *commit_lanes.get(&commit.id).unwrap_or(&0);

            // Create lanes array
            let mut lanes = vec![Lane::Empty; self.max_lanes];

            // Set current commit
            lanes[primary_lane] = Lane::Commit;

            // Mark lanes that continue through this row
            for other_commit in &commits {
                if other_commit.id != commit.id {
                    if let Some(&other_lane) = commit_lanes.get(&other_commit.id) {
                        // Check if this commit is connected to the other commit
                        let is_parent = commit.parents.contains(&other_commit.id);
                        let is_child = other_commit.parents.contains(&commit.id);

                        if (is_parent || is_child) && lanes[other_lane] == Lane::Empty {
                            lanes[other_lane] = Lane::Pass;
                        }
                    }
                }
            }

            rows.push(Row {
                commit_id: commit.id.clone(),
                commit: (*commit).clone(),
                lanes,
                primary_lane,
            });
        }

        rows
    }

    fn build_simple_row(&mut self, commit: &CommitNode, dag: &Dag) -> Row {
        // Find or allocate a lane for this commit
        let primary_lane = self.find_or_allocate_lane(&commit.id);

        // Create lanes array
        let mut lanes = vec![Lane::Empty; self.max_lanes];

        // Mark all active lanes as passing
        for (idx, lane_commit) in self.lanes.iter().enumerate() {
            if let Some(commit_id) = lane_commit {
                if commit_id == &commit.id {
                    // This is the current commit
                    lanes[idx] = Lane::Commit;
                } else {
                    // This lane continues through
                    lanes[idx] = Lane::Pass;
                }
            }
        }

        // Handle merges (multiple parents)
        if commit.parents.len() > 1 {
            let mut merge_targets = Vec::new();
            for parent_id in &commit.parents[1..] {
                if let Some(parent_lane) = self.find_lane(parent_id) {
                    merge_targets.push(parent_lane);
                }
            }
            if !merge_targets.is_empty() {
                lanes[primary_lane] = Lane::Merge(merge_targets);
            }
        }

        // Handle branches (multiple children)
        let children = dag.get_children(&commit.id);
        if children.len() > 1 && primary_lane < self.max_lanes {
            // This commit spawns branches
            lanes[primary_lane] = Lane::BranchStart;
        }

        Row {
            commit_id: commit.id.clone(),
            commit: commit.clone(),
            lanes,
            primary_lane,
        }
    }

    fn find_or_allocate_lane(&mut self, commit_id: &str) -> LaneIdx {
        // First check if this commit already has a lane
        if let Some(lane) = self.find_lane(commit_id) {
            return lane;
        }

        // Find first available lane
        for (idx, lane) in self.lanes.iter_mut().enumerate() {
            if lane.is_none() {
                *lane = Some(commit_id.to_string());
                return idx;
            }
        }

        // If no free lanes, reuse the first one
        self.lanes[0] = Some(commit_id.to_string());
        0
    }

    fn find_lane(&self, commit_id: &str) -> Option<LaneIdx> {
        self.lanes.iter().position(|l| l.as_ref().map_or(false, |id| id == commit_id))
    }

    fn update_lanes_after_commit(&mut self, commit: &CommitNode) {
        // Clear the lane for this commit
        if let Some(lane_idx) = self.find_lane(&commit.id) {
            self.lanes[lane_idx] = None;

            // Assign lanes to parents
            for (i, parent_id) in commit.parents.iter().enumerate() {
                if i == 0 {
                    // First parent continues in the same lane
                    self.lanes[lane_idx] = Some(parent_id.clone());
                } else {
                    // Other parents need new lanes
                    self.find_or_allocate_lane(parent_id);
                }
            }
        }
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