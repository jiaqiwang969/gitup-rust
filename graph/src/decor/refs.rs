use crate::core::CommitNode;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Decoration {
    pub is_head: bool,
    pub branches: Vec<String>,
    pub tags: Vec<String>,
    pub color_index: usize,
}

pub struct RefDecorator {
    head: Option<String>,
    branches: HashMap<String, Vec<String>>,
    tags: HashMap<String, Vec<String>>,
}

impl RefDecorator {
    pub fn new() -> Self {
        Self {
            head: None,
            branches: HashMap::new(),
            tags: HashMap::new(),
        }
    }

    pub fn set_head(&mut self, commit_id: String) {
        self.head = Some(commit_id);
    }

    pub fn add_branch(&mut self, commit_id: String, branch: String) {
        self.branches.entry(commit_id).or_default().push(branch);
    }

    pub fn decorate(&self, commit: &CommitNode, lane_idx: usize) -> Decoration {
        Decoration {
            is_head: self.head.as_ref() == Some(&commit.id),
            branches: self.branches.get(&commit.id).cloned().unwrap_or_default(),
            tags: self.tags.get(&commit.id).cloned().unwrap_or_default(),
            color_index: lane_idx % 6,
        }
    }
}
