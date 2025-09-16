use std::collections::HashMap;
use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Regular,
    Merge,
    Initial,
    Current,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphPosition {
    pub row: usize,
    pub lane: usize,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub message: String,
    pub author: String,
    pub date: i64,
    pub position: GraphPosition,
    pub node_type: NodeType,
    pub primary_branch: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub lane: usize,
}

#[derive(Debug, Clone)]
pub struct Lane {
    pub index: usize,
    pub color: Color,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct GitGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub lanes: Vec<Lane>,
    pub branches: HashMap<String, String>,
    pub tags: HashMap<String, String>,
    pub branch_colors: HashMap<String, Color>,
}
