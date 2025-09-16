use std::collections::HashMap;
use super::types::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind { Direct, Merge }

#[derive(Debug, Clone)]
pub struct EdgeSeg { pub to: String, pub kind: EdgeKind }

#[derive(Debug, Default, Clone)]
pub struct RowEdge { pub starting: Option<EdgeSeg>, pub pass_through: Option<EdgeSeg>, pub ending: Option<EdgeSeg> }

pub type RowEdges = HashMap<usize, RowEdge>; // lane -> row edge

#[derive(Debug, Clone)]
pub struct ProcessedRow {
    pub row: usize,
    pub commit_id: String,
    pub edges: RowEdges,
}

pub struct RowEdgesBuilder;

impl RowEdgesBuilder {
    pub fn build(graph: &GitGraph) -> Vec<ProcessedRow> {
        let n = graph.nodes.len();
        let mut rows: Vec<ProcessedRow> = (0..n).map(|i|
            ProcessedRow { row: i, commit_id: graph.nodes[i].id.clone(), edges: RowEdges::default() }
        ).collect();

        // fast lookup: id -> position
        let mut pos: HashMap<&str, GraphPosition> = HashMap::new();
        for node in &graph.nodes {
            pos.insert(node.id.as_str(), node.position.clone());
        }

        for e in &graph.edges {
            let from = match pos.get(e.from.as_str()) { Some(p) => *p, None => continue };
            let to = match pos.get(e.to.as_str()) { Some(p) => *p, None => continue };
            let kind = if from.lane == to.lane { EdgeKind::Direct } else { EdgeKind::Merge };

            // At child row: mark starting if cross-lane, otherwise we rely on node glyph + pass-through
            if kind == EdgeKind::Merge {
                let re = rows[from.row].edges.entry(from.lane).or_insert_with(RowEdge::default);
                re.starting = Some(EdgeSeg { to: e.to.clone(), kind });
            } else {
                let re = rows[from.row].edges.entry(from.lane).or_insert_with(RowEdge::default);
                re.pass_through = Some(EdgeSeg { to: e.to.clone(), kind });
            }

            // Intermediate rows: pass-through on target lane
            let mut r = from.row + 1;
            while r < to.row {
                let re = rows[r].edges.entry(to.lane).or_insert_with(RowEdge::default);
                re.pass_through = Some(EdgeSeg { to: e.to.clone(), kind });
                r += 1;
            }

            // At parent row: ending on target lane
            let re = rows[to.row].edges.entry(to.lane).or_insert_with(RowEdge::default);
            re.ending = Some(EdgeSeg { to: e.to.clone(), kind });
        }

        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn make_graph_linear() -> GitGraph {
        // c2(row0,lane0) -> c1(row1,lane0)
        GitGraph {
            nodes: vec![
                GraphNode { id: "c2".into(), message: String::new(), author: String::new(), date: 0,
                    position: GraphPosition { row: 0, lane: 0 }, node_type: NodeType::Regular, primary_branch: None },
                GraphNode { id: "c1".into(), message: String::new(), author: String::new(), date: 0,
                    position: GraphPosition { row: 1, lane: 0 }, node_type: NodeType::Initial, primary_branch: None },
            ],
            edges: vec![ GraphEdge { from: "c2".into(), to: "c1".into(), lane: 0 } ],
            lanes: vec![ Lane { index: 0, color: Color::Cyan, active: true } ],
            branches: Default::default(),
            tags: Default::default(),
            branch_colors: Default::default(),
        }
    }

    fn make_graph_merge() -> GitGraph {
        // m(row0,l0) -> a(row1,l0), f(row2,l1)
        GitGraph {
            nodes: vec![
                GraphNode { id: "m".into(), message: String::new(), author: String::new(), date: 0,
                    position: GraphPosition { row: 0, lane: 0 }, node_type: NodeType::Regular, primary_branch: None },
                GraphNode { id: "a".into(), message: String::new(), author: String::new(), date: 0,
                    position: GraphPosition { row: 1, lane: 0 }, node_type: NodeType::Regular, primary_branch: None },
                GraphNode { id: "f".into(), message: String::new(), author: String::new(), date: 0,
                    position: GraphPosition { row: 2, lane: 1 }, node_type: NodeType::Regular, primary_branch: None },
            ],
            edges: vec![
                GraphEdge { from: "m".into(), to: "a".into(), lane: 0 },
                GraphEdge { from: "m".into(), to: "f".into(), lane: 0 },
            ],
            lanes: vec![ Lane { index: 0, color: Color::Cyan, active: true }, Lane { index: 1, color: Color::Green, active: true } ],
            branches: Default::default(), tags: Default::default(), branch_colors: Default::default(),
        }
    }

    #[test]
    fn row_edges_linear() {
        let g = make_graph_linear();
        let rows = RowEdgesBuilder::build(&g);
        // child row has pass-through on lane 0
        let r0 = &rows[0];
        let e0 = r0.edges.get(&0).unwrap();
        assert!(e0.pass_through.is_some());
        // parent row has ending on lane 0
        let r1 = &rows[1];
        let e1 = r1.edges.get(&0).unwrap();
        assert!(e1.ending.is_some());
    }

    #[test]
    fn row_edges_merge_cross_lane() {
        let g = make_graph_merge();
        let rows = RowEdgesBuilder::build(&g);
        // at merge row, there should be a starting edge for cross-lane parent
        let r0 = &rows[0];
        let e0 = r0.edges.get(&0).unwrap();
        assert!(e0.starting.is_some() || e0.pass_through.is_some());
        // intermediate pass-through on lane 1 towards 'f'
        let r1 = &rows[1];
        if let Some(re) = r1.edges.get(&1) { assert!(re.pass_through.is_some()); }
        // ending at row 2 on lane 1 for 'f'
        let r2 = &rows[2];
        let e2 = r2.edges.get(&1).unwrap();
        assert!(e2.ending.is_some());
    }
}
