#[derive(Debug, Clone)]
pub enum GraphEvent {
    RepositoryChanged,
    BranchChanged(String),
    WorkingTreeChanged,
    CommitAdded(String),
    RefUpdated(String),
    NodeSelected(String),
    NodeActivated(String),
    ScrollPositionChanged(usize),
}

