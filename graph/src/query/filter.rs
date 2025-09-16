use crate::core::{Dag, CommitNode};

pub enum FilterMode {
    Author(String),
    Message(String),
    Path(String),
    DateRange(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>),
}

pub struct DagFilter {
    mode: FilterMode,
}

impl DagFilter {
    pub fn filter(&self, dag: &Dag) -> Dag {
        let mut filtered = Dag::new();

        for (_, node) in &dag.nodes {
            if self.matches(node) {
                filtered.add_node(node.clone());
            }
        }

        filtered
    }

    fn matches(&self, node: &CommitNode) -> bool {
        match &self.mode {
            FilterMode::Author(pattern) => node.author.contains(pattern),
            FilterMode::Message(pattern) => node.message.contains(pattern),
            _ => true,
        }
    }
}
