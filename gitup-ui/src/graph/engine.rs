use anyhow::Result;
use std::collections::HashMap;
use ratatui::style::Color;

use gitup_core::{Repository, CommitWithParents, RefInfo, RefType};

use super::types::*;
use super::lane_manager::LaneManager;

/// Graph engine entrypoint (placeholder):
/// builds a trivial single-lane graph to unblock subsequent work.
pub struct GraphEngine {
    pub max_count: usize,
}

impl Default for GraphEngine {
    fn default() -> Self { Self { max_count: 500 } }
}

impl GraphEngine {
    pub fn build(&self, repo: &Repository) -> Result<GitGraph> {
        let commits: Vec<CommitWithParents> = repo.get_commits_with_parents(self.max_count)?;
        let refs_by_oid = repo.list_refs_by_oid()?;

        let mut nodes = Vec::with_capacity(commits.len());
        let mut edges = Vec::new();
        let mut lane_mgr = LaneManager::new();
        let mut branches: HashMap<String, String> = HashMap::new();
        let mut tags: HashMap<String, String> = HashMap::new();
        let mut branch_colors: HashMap<String, Color> = HashMap::new();

        for (row, c) in commits.iter().enumerate() {
            // assign lane based on active expectations (set by previous child's parents)
            let lane = lane_mgr.assign_lane(&c.id);
            let ntype = match c.parents.len() {
                0 => NodeType::Initial,
                _ => NodeType::Regular,
            };
            // determine primary branch for this commit (prefer local branch)
            let primary_branch: Option<String> = refs_by_oid.get(&c.id)
                .and_then(|infos| {
                    infos.iter()
                        .find(|r| matches!(r.ref_type, RefType::Branch) && !r.is_remote)
                        .map(|r| r.name.clone())
                        .or_else(|| infos.iter()
                            .find(|r| matches!(r.ref_type, RefType::Remote))
                            .map(|r| r.name.clone()))
                });
            nodes.push(GraphNode {
                id: c.id.clone(),
                message: c.message.clone(),
                author: c.author.clone(),
                date: c.timestamp,
                position: GraphPosition { row, lane },
                node_type: ntype,
                primary_branch,
            });

            for p in &c.parents {
                edges.push(GraphEdge { from: c.id.clone(), to: p.clone(), lane });
            }

            // advance lanes to parent expectations
            lane_mgr.post_commit_update(lane, &c.parents);
        }

        // refs
        for (oid, infos) in refs_by_oid {
            for info in infos {
                match info.ref_type {
                    RefType::Branch => { branches.insert(info.name.clone(), oid.clone()); },
                    RefType::Tag => { tags.insert(info.name.clone(), oid.clone()); },
                    _ => {},
                }
            }
        }

        // lanes output
        let palette = vec![
            Color::Cyan, Color::Green, Color::Yellow, Color::Magenta, Color::Blue, Color::Red,
            Color::LightCyan, Color::LightGreen, Color::LightYellow, Color::LightMagenta, Color::LightBlue, Color::LightRed,
        ];
        let mut lanes = Vec::new();
        for i in 0..lane_mgr.lane_count() {
            lanes.push(Lane { index: i, color: palette[i % palette.len()], active: true });
        }

        // assign stable colors to branches
        fn stable_color_index(name: &str, modulo: usize) -> usize {
            let mut hash: u64 = 1469598103934665603; // FNV-1a
            for b in name.as_bytes() { hash ^= *b as u64; hash = hash.wrapping_mul(1099511628211); }
            (hash as usize) % modulo
        }
        for name in branches.keys() {
            let idx = stable_color_index(name, palette.len());
            branch_colors.insert(name.clone(), palette[idx]);
        }

        Ok(GitGraph { nodes, edges, lanes, branches, tags, branch_colors })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::io::Write;
    use std::fs;

    fn write_file<P: AsRef<std::path::Path>>(p: P, s: &str) { let mut f = fs::File::create(p).unwrap(); f.write_all(s.as_bytes()).unwrap(); }

    #[test]
    fn build_trivial_graph() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        // one commit
        write_file(dir.path().join("x.txt"), "1");
        repo.stage_file("x.txt").unwrap();
        repo.commit("c1", "A", "a@a").unwrap();

        let engine = GraphEngine::default();
        let g = engine.build(&repo).unwrap();
        assert!(!g.nodes.is_empty());
        assert_eq!(g.lanes.len(), 1);
        assert!(g.edges.iter().all(|e| e.lane == 0));
    }
}
