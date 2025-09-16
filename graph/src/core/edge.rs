/// An edge connecting two commits
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edge {
    /// Source commit ID (child)
    pub from: String,
    /// Target commit ID (parent)
    pub to: String,
    /// Edge type (for future extensions)
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeType {
    /// Regular parent-child relationship
    Regular,
    /// Merge edge (from merge commit to parent)
    Merge,
    /// Cherry-pick relationship (future)
    CherryPick,
}

impl Edge {
    pub fn new(from: String, to: String) -> Self {
        Self {
            from,
            to,
            edge_type: EdgeType::Regular,
        }
    }

    pub fn merge(from: String, to: String) -> Self {
        Self {
            from,
            to,
            edge_type: EdgeType::Merge,
        }
    }
}