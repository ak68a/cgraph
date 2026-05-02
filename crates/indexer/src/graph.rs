use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet};
use cgraph_core::{SymbolNode, EdgeKind};

/// CodeGraph wraps petgraph's DiGraph with a HashMap index for O(1) symbol lookup by ID.
/// This is the single source of truth for the in-memory code graph (D-45, D-46).
pub struct CodeGraph {
    /// The petgraph directed graph (public for algorithm access by analysis module).
    pub graph: DiGraph<SymbolNode, EdgeKind>,
    /// Maps symbol_id -> NodeIndex for O(1) lookup.
    node_index: HashMap<String, NodeIndex>,
    /// File paths of barrel files (side-channel per RESEARCH.md A2).
    barrel_files: HashSet<String>,
}

impl CodeGraph {
    /// Create an empty CodeGraph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
            barrel_files: HashSet::new(),
        }
    }

    /// Insert a symbol node into the graph, updating the HashMap index.
    /// Returns the petgraph NodeIndex for the inserted node.
    pub fn add_symbol(&mut self, node: SymbolNode) -> NodeIndex {
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.node_index.insert(id, idx);
        idx
    }

    /// Add an edge between two symbols identified by their string IDs.
    /// If either source or target ID is not found in the graph, the edge is
    /// silently skipped (D-13: warn and continue). This prevents panics from
    /// unresolved references (e.g., third-party imports).
    pub fn add_edge(&mut self, source_id: &str, target_id: &str, kind: EdgeKind) {
        if let (Some(&src), Some(&tgt)) = (
            self.node_index.get(source_id),
            self.node_index.get(target_id),
        ) {
            self.graph.add_edge(src, tgt, kind);
        }
    }

    /// Look up a symbol's NodeIndex by its string ID.
    pub fn get_index(&self, symbol_id: &str) -> Option<NodeIndex> {
        self.node_index.get(symbol_id).copied()
    }

    /// Check whether a file path has been marked as a barrel file.
    pub fn is_barrel_file(&self, file_path: &str) -> bool {
        self.barrel_files.contains(file_path)
    }

    /// Mark a file path as a barrel file.
    pub fn mark_barrel_file(&mut self, file_path: String) {
        self.barrel_files.insert(file_path);
    }

    /// Count unique file paths across all symbol nodes in the graph.
    pub fn file_count(&self) -> usize {
        let files: HashSet<&str> = self
            .graph
            .node_weights()
            .map(|n| n.file_path.as_str())
            .collect();
        files.len()
    }

    /// Number of symbol nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgraph_core::{SymbolKind, Language};

    /// Helper to create a SymbolNode with sensible defaults for testing.
    fn make_test_node(id: &str, file_path: &str) -> SymbolNode {
        SymbolNode {
            id: id.to_string(),
            name: id.split("::").last().unwrap_or(id).to_string(),
            kind: SymbolKind::Function,
            file_path: file_path.to_string(),
            language: Language::TypeScript,
            line_start: 1,
            line_end: 10,
            is_exported: true,
        }
    }

    #[test]
    fn test_add_symbol_and_lookup() {
        let mut graph = CodeGraph::new();
        let node = make_test_node("src/main.ts::hello", "src/main.ts");
        graph.add_symbol(node);

        assert!(graph.get_index("src/main.ts::hello").is_some());
        assert!(graph.get_index("nonexistent::symbol").is_none());
    }

    #[test]
    fn test_add_edge_known_nodes() {
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_test_node("a.ts::foo", "a.ts"));
        graph.add_symbol(make_test_node("b.ts::bar", "b.ts"));

        graph.add_edge("a.ts::foo", "b.ts::bar", EdgeKind::Import);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_add_edge_unknown_target() {
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_test_node("a.ts::foo", "a.ts"));

        // Adding edge to unknown target should silently skip, not panic
        graph.add_edge("a.ts::foo", "unknown::bar", EdgeKind::Import);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_file_count() {
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_test_node("a.ts::foo", "a.ts"));
        graph.add_symbol(make_test_node("a.ts::bar", "a.ts"));
        graph.add_symbol(make_test_node("b.ts::baz", "b.ts"));

        // 3 symbols from 2 files
        assert_eq!(graph.file_count(), 2);
    }

    #[test]
    fn test_barrel_file_tracking() {
        let mut graph = CodeGraph::new();
        graph.mark_barrel_file("src/index.ts".to_string());

        assert!(graph.is_barrel_file("src/index.ts"));
        assert!(!graph.is_barrel_file("src/main.ts"));
    }
}
