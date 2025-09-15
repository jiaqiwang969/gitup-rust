use git2::{Repository, Oid, BranchType};
use std::collections::{HashMap, HashSet, VecDeque};
use anyhow::Result;
use super::graph::{GitGraph, GraphNode, GraphEdge, GraphPosition, NodeType, RefInfo, RefType, Lane, EdgeType};

/// Builder for constructing a Git graph from a repository
pub struct GraphBuilder {
    repository: Repository,
    max_count: usize,
    show_stashes: bool,
    show_remotes: bool,
}

impl GraphBuilder {
    /// Create a new graph builder
    pub fn new(repo_path: &str) -> Result<Self> {
        let repository = Repository::open(repo_path)?;
        Ok(Self {
            repository,
            max_count: 500, // Default limit
            show_stashes: true,
            show_remotes: true,
        })
    }

    /// Set maximum number of commits to load
    pub fn max_count(mut self, count: usize) -> Self {
        self.max_count = count;
        self
    }

    /// Set whether to show stashes
    pub fn show_stashes(mut self, show: bool) -> Self {
        self.show_stashes = show;
        self
    }

    /// Set whether to show remote branches
    pub fn show_remotes(mut self, show: bool) -> Self {
        self.show_remotes = show;
        self
    }

    /// Build the graph
    pub fn build(&self) -> Result<GitGraph> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut lanes = Vec::new();
        let mut branches = HashMap::new();
        let mut tags = HashMap::new();

        // Get HEAD
        let head = self.repository.head()?;
        let head_oid = head.target().unwrap();
        let head_name = head.shorthand().unwrap_or("HEAD");

        // Collect all refs
        let refs = self.collect_refs()?;

        // Walk commits
        let mut revwalk = self.repository.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        revwalk.push_head()?;

        // Also include all branches
        for branch in self.repository.branches(Some(BranchType::Local))? {
            let (branch, _) = branch?;
            if let Some(target) = branch.get().target() {
                revwalk.push(target)?;
            }
        }

        // Build commit graph
        let mut commit_map: HashMap<Oid, usize> = HashMap::new();
        let mut lane_manager = LaneManager::new();
        let mut count = 0;

        for oid in revwalk {
            if count >= self.max_count {
                break;
            }

            let oid = oid?;
            let commit = self.repository.find_commit(oid)?;

            // Determine node type
            let node_type = if oid == head_oid {
                NodeType::Current
            } else if commit.parent_count() > 1 {
                NodeType::Merge
            } else if commit.parent_count() == 0 {
                NodeType::Initial
            } else {
                NodeType::Regular
            };

            // Get refs at this commit
            let node_refs = refs.get(&oid).cloned().unwrap_or_default();

            // Assign lane
            let lane = lane_manager.assign_lane(&oid, &commit);

            // Create node
            let node = GraphNode {
                id: oid.to_string(),
                message: commit.summary().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                date: format_timestamp(commit.time().seconds()),
                position: GraphPosition {
                    row: count,
                    lane,
                },
                node_type,
                refs: node_refs,
            };

            commit_map.insert(oid, count);
            nodes.push(node);

            // Create edges to parents
            for parent in commit.parents() {
                let parent_oid = parent.id();
                edges.push(GraphEdge {
                    from: oid.to_string(),
                    to: parent_oid.to_string(),
                    lane,
                    edge_type: if commit.parent_count() > 1 {
                        EdgeType::Merge
                    } else {
                        EdgeType::Direct
                    },
                });
            }

            count += 1;
        }

        // Create lanes
        lanes = lane_manager.get_lanes();

        // Collect branches and tags
        for branch in self.repository.branches(None)? {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                if let Some(target) = branch.get().target() {
                    branches.insert(name.to_string(), target.to_string());
                }
            }
        }

        self.repository.tag_foreach(|oid, name| {
            // name is &[u8], convert to string
            if let Ok(name_str) = std::str::from_utf8(name) {
                if let Some(tag_name) = name_str.strip_prefix("refs/tags/") {
                    tags.insert(tag_name.to_string(), oid.to_string());
                }
            }
            true
        })?;

        Ok(GitGraph {
            nodes,
            edges,
            lanes,
            branches,
            tags,
        })
    }

    /// Collect all refs in the repository
    fn collect_refs(&self) -> Result<HashMap<Oid, Vec<RefInfo>>> {
        let mut refs_map: HashMap<Oid, Vec<RefInfo>> = HashMap::new();

        // HEAD
        if let Ok(head) = self.repository.head() {
            if let Some(target) = head.target() {
                refs_map.entry(target).or_default().push(RefInfo {
                    ref_type: RefType::Head,
                    name: "HEAD".to_string(),
                    is_head: true,
                    is_remote: false,
                });
            }
        }

        // Branches
        for branch_type in &[BranchType::Local, BranchType::Remote] {
            for branch in self.repository.branches(Some(*branch_type))? {
                let (branch, _) = branch?;
                if let Some(name) = branch.name()? {
                    if let Some(target) = branch.get().target() {
                        let is_remote = *branch_type == BranchType::Remote;

                        // Skip remote branches if not showing them
                        if is_remote && !self.show_remotes {
                            continue;
                        }

                        refs_map.entry(target).or_default().push(RefInfo {
                            ref_type: if is_remote { RefType::Remote } else { RefType::Branch },
                            name: name.to_string(),
                            is_head: false,
                            is_remote,
                        });
                    }
                }
            }
        }

        // Tags
        self.repository.tag_foreach(|oid, name| {
            // name is &[u8], convert to string
            if let Ok(name_str) = std::str::from_utf8(name) {
                if let Some(tag_name) = name_str.strip_prefix("refs/tags/") {
                    refs_map.entry(oid).or_default().push(RefInfo {
                        ref_type: RefType::Tag,
                        name: tag_name.to_string(),
                        is_head: false,
                        is_remote: false,
                    });
                }
            }
            true
        })?;

        Ok(refs_map)
    }
}

/// Lane manager for assigning lanes to commits
struct LaneManager {
    lanes: Vec<LaneState>,
    next_color_index: usize,
}

struct LaneState {
    oid: Option<Oid>,
    color_index: usize,
}

impl LaneManager {
    fn new() -> Self {
        Self {
            lanes: Vec::new(),
            next_color_index: 0,
        }
    }

    fn assign_lane(&mut self, oid: &Oid, commit: &git2::Commit) -> usize {
        // Find existing lane or create new one
        for (i, lane) in self.lanes.iter_mut().enumerate() {
            if lane.oid == Some(*oid) || lane.oid.is_none() {
                lane.oid = Some(*oid);
                return i;
            }
        }

        // Create new lane
        let lane_index = self.lanes.len();
        self.lanes.push(LaneState {
            oid: Some(*oid),
            color_index: self.next_color_index,
        });
        self.next_color_index = (self.next_color_index + 1) % 6; // Cycle through 6 colors
        lane_index
    }

    fn get_lanes(&self) -> Vec<Lane> {
        let colors = vec![
            ratatui::style::Color::Cyan,
            ratatui::style::Color::Green,
            ratatui::style::Color::Yellow,
            ratatui::style::Color::Magenta,
            ratatui::style::Color::Blue,
            ratatui::style::Color::Red,
        ];

        self.lanes
            .iter()
            .enumerate()
            .map(|(i, state)| Lane {
                index: i,
                color: colors[state.color_index],
                active: state.oid.is_some(),
            })
            .collect()
    }
}

/// Format timestamp to human-readable date
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Local, TimeZone};

    let datetime = Local.timestamp_opt(timestamp, 0).unwrap();
    let now = Local::now();
    let duration = now.signed_duration_since(datetime);

    if duration.num_days() == 0 {
        if duration.num_hours() == 0 {
            format!("{} mins ago", duration.num_minutes())
        } else {
            format!("{} hours ago", duration.num_hours())
        }
    } else if duration.num_days() < 7 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_days() < 30 {
        format!("{} weeks ago", duration.num_days() / 7)
    } else if duration.num_days() < 365 {
        format!("{} months ago", duration.num_days() / 30)
    } else {
        datetime.format("%Y-%m-%d").to_string()
    }
}