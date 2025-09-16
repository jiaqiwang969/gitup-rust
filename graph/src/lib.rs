pub mod core;
pub mod git_backend;

pub use core::{Dag, CommitNode, Edge, EdgeType, DagStats};
pub use git_backend::GitWalker;