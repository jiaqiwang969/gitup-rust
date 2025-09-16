pub mod core;
pub mod git_backend;
pub mod layout;
pub mod render;

pub use core::{Dag, CommitNode, Edge, EdgeType, DagStats};
pub use git_backend::GitWalker;
pub use layout::{Row, RowBuilder, Lane, LaneIdx, CompactRowBuilder, SimpleGraphBuilder};
pub use render::{
    TuiRenderer, AsciiRenderer, Cell, Color,
    Viewport, VirtualRenderer,
    ViewportCarryOver, SeamlessViewport, ColumnState, EnteringType,
    CellRouter, ConflictResolver, CharsetProfile, EntryDir, ExitDir, LaneType,
    TextLayout, CjkMode, Alignment, CommitMessageFormatter,
};