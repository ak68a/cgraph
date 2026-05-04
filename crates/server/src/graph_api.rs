use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use cgraph_core::{EdgeKind, SymbolKind};
use cgraph_indexer::{CodeGraph, DeadCodeResult};

use crate::static_assets;

// ─── Response Types ───────────────────────────────────────────────────────────

/// Export counts broken down by symbol kind for a single file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportCounts {
    pub functions: u32,
    pub classes: u32,
    pub types: u32,
    pub interfaces: u32,
    pub hooks: u32,
    pub enums: u32,
    pub total: u32,
}

/// A file-level graph node (D-53, D-54).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    /// Equals file_path — used as node ID in the graph.
    pub id: String,
    /// Relative file path.
    pub path: String,
    /// Basename, or "parent/basename" when duplicates exist (D-54). Truncated at 20 chars.
    pub filename: String,
    pub export_counts: ExportCounts,
    /// Visual radius: 8px (1 export) to 24px (20+ exports) — D-53.
    pub radius: f32,
    /// Number of incoming file-level edges (for tooltip — D-55).
    pub incoming: usize,
    /// Number of outgoing file-level edges (for tooltip — D-55).
    pub outgoing: usize,
}

/// A deduplicated file-level graph edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdge {
    /// Source file path (node ID).
    pub source: String,
    /// Target file path (node ID).
    pub target: String,
}

/// A symbol-level graph node with dead code flags (D-81, VIZN-03).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNodeDto {
    pub id: String,
    pub name: String,
    /// Lowercase symbol kind: "function", "class", "type", "interface", "hook", "enum", "module".
    pub kind: String,
    pub file_path: String,
    pub is_dead_code: bool,
    pub dead_code_confidence: Option<String>,
}

/// A typed edge carrying the relationship kind between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedEdge {
    pub source: String,
    pub target: String,
    /// Edge kind: "import", "call", "type_ref", "re_export".
    pub edge_type: String,
}

/// Enriched /api/graph response: file nodes + symbol nodes + typed edges + dead code flags (D-81).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedGraphResponse {
    pub nodes: Vec<FileNode>,
    pub edges: Vec<TypedEdge>,
    pub symbols: Vec<SymbolNodeDto>,
    pub stats: ScanStats,
    pub project_name: String,
}

/// Summary statistics included in every /api/graph response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    pub files: usize,
    pub symbols: usize,
    pub edges: usize,
    pub elapsed_ms: u64,
}

/// Top-level /api/graph response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGraphResponse {
    pub nodes: Vec<FileNode>,
    pub edges: Vec<FileEdge>,
    pub stats: ScanStats,
    pub project_name: String,
}

// ─── Application State ────────────────────────────────────────────────────────

/// Shared application state: the pre-computed enriched graph wrapped in Arc for cheap cloning.
#[derive(Clone)]
pub struct AppState {
    pub file_graph: Arc<EnrichedGraphResponse>,
}

// ─── File-Level Projection ────────────────────────────────────────────────────

/// Project the symbol-level `CodeGraph` onto a file-level view.
///
/// Each unique file becomes one `FileNode`; symbol-level edges are lifted to
/// file-level edges and deduplicated. Self-loops (edges between symbols in the
/// same file) are excluded (D-42).
pub fn file_level_projection(
    graph: &CodeGraph,
    stats: ScanStats,
    project_name: String,
) -> FileGraphResponse {
    // Step 1: Collect all unique file paths and accumulate export counts.
    let mut file_exports: HashMap<String, ExportCounts> = HashMap::new();

    for node in graph.graph.node_weights() {
        let counts = file_exports
            .entry(node.file_path.clone())
            .or_insert_with(ExportCounts::default);

        if node.is_exported {
            match node.kind {
                SymbolKind::Function => {
                    counts.functions += 1;
                    counts.total += 1;
                }
                SymbolKind::Class => {
                    counts.classes += 1;
                    counts.total += 1;
                }
                SymbolKind::Type => {
                    counts.types += 1;
                    counts.total += 1;
                }
                SymbolKind::Interface => {
                    counts.interfaces += 1;
                    counts.total += 1;
                }
                SymbolKind::Hook => {
                    counts.hooks += 1;
                    counts.total += 1;
                }
                SymbolKind::Enum => {
                    counts.enums += 1;
                    counts.total += 1;
                }
                SymbolKind::Module => {
                    // Skip Module per spec
                }
            }
        }
    }

    // Step 2: Detect duplicate basenames for D-54 disambiguation.
    let mut basename_to_paths: HashMap<String, Vec<String>> = HashMap::new();
    for file_path in file_exports.keys() {
        let basename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path)
            .to_string();
        basename_to_paths
            .entry(basename)
            .or_default()
            .push(file_path.clone());
    }

    // Build a path -> display filename map.
    let mut filename_map: HashMap<String, String> = HashMap::new();
    for (basename, paths) in &basename_to_paths {
        if paths.len() > 1 {
            // Disambiguate with parent directory prefix.
            for file_path in paths {
                let parent = std::path::Path::new(file_path)
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let display = if parent.is_empty() {
                    basename.clone()
                } else {
                    format!("{}/{}", parent, basename)
                };
                filename_map.insert(file_path.clone(), display);
            }
        } else {
            filename_map.insert(paths[0].clone(), basename.clone());
        }
    }

    // Step 3: Build deduplicated file-level edge set (exclude self-loops).
    let mut file_edges_set: HashSet<(String, String)> = HashSet::new();
    for edge_idx in graph.graph.edge_indices() {
        if let Some((src_idx, tgt_idx)) = graph.graph.edge_endpoints(edge_idx) {
            let src_file = graph.graph[src_idx].file_path.clone();
            let tgt_file = graph.graph[tgt_idx].file_path.clone();
            if src_file != tgt_file {
                file_edges_set.insert((src_file, tgt_file));
            }
        }
    }

    // Step 4: Compute per-file incoming/outgoing counts.
    let mut incoming_counts: HashMap<String, usize> = HashMap::new();
    let mut outgoing_counts: HashMap<String, usize> = HashMap::new();
    for (src, tgt) in &file_edges_set {
        *outgoing_counts.entry(src.clone()).or_insert(0) += 1;
        *incoming_counts.entry(tgt.clone()).or_insert(0) += 1;
    }

    // Step 5: Build FileNode list.
    let nodes: Vec<FileNode> = file_exports
        .into_iter()
        .map(|(file_path, counts)| {
            let total = counts.total;
            let radius = compute_radius(total);
            let filename = filename_map
                .get(&file_path)
                .cloned()
                .unwrap_or_else(|| file_path.clone());
            let incoming = incoming_counts.get(&file_path).copied().unwrap_or(0);
            let outgoing = outgoing_counts.get(&file_path).copied().unwrap_or(0);
            FileNode {
                id: file_path.clone(),
                path: file_path,
                filename,
                export_counts: counts,
                radius,
                incoming,
                outgoing,
            }
        })
        .collect();

    // Step 6: Build FileEdge list.
    let edges: Vec<FileEdge> = file_edges_set
        .into_iter()
        .map(|(source, target)| FileEdge { source, target })
        .collect();

    FileGraphResponse {
        nodes,
        edges,
        stats,
        project_name,
    }
}

/// Project the symbol-level `CodeGraph` onto an enriched view combining file nodes,
/// symbol-level nodes with dead code flags, and typed edges (D-81, VIZN-03).
///
/// File nodes and file-level edge deduplication logic is identical to
/// `file_level_projection`. Additionally:
/// - Symbol nodes include dead code flags sourced from `dead_result`.
/// - Edges include both symbol-level typed edges AND deduplicated file-level edges.
/// - Only exported symbols are included in `symbols` (VIZN-03: expand shows exports).
pub fn enriched_projection(
    graph: &CodeGraph,
    dead_result: &DeadCodeResult,
    stats: ScanStats,
    project_name: String,
) -> EnrichedGraphResponse {
    // ── File node construction (same as file_level_projection) ──────────────

    let mut file_exports: HashMap<String, ExportCounts> = HashMap::new();
    for node in graph.graph.node_weights() {
        let counts = file_exports
            .entry(node.file_path.clone())
            .or_insert_with(ExportCounts::default);
        if node.is_exported {
            match node.kind {
                SymbolKind::Function => { counts.functions += 1; counts.total += 1; }
                SymbolKind::Class    => { counts.classes += 1;   counts.total += 1; }
                SymbolKind::Type     => { counts.types += 1;     counts.total += 1; }
                SymbolKind::Interface => { counts.interfaces += 1; counts.total += 1; }
                SymbolKind::Hook     => { counts.hooks += 1;     counts.total += 1; }
                SymbolKind::Enum     => { counts.enums += 1;     counts.total += 1; }
                SymbolKind::Module   => {}
            }
        }
    }

    let mut basename_to_paths: HashMap<String, Vec<String>> = HashMap::new();
    for file_path in file_exports.keys() {
        let basename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path)
            .to_string();
        basename_to_paths.entry(basename).or_default().push(file_path.clone());
    }
    let mut filename_map: HashMap<String, String> = HashMap::new();
    for (basename, paths) in &basename_to_paths {
        if paths.len() > 1 {
            for file_path in paths {
                let parent = std::path::Path::new(file_path)
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let display = if parent.is_empty() { basename.clone() } else { format!("{}/{}", parent, basename) };
                filename_map.insert(file_path.clone(), display);
            }
        } else {
            filename_map.insert(paths[0].clone(), basename.clone());
        }
    }

    let mut file_edges_set: HashSet<(String, String)> = HashSet::new();
    let mut file_edges_typed: HashSet<(String, String, String)> = HashSet::new();
    for edge_idx in graph.graph.edge_indices() {
        if let Some((src_idx, tgt_idx)) = graph.graph.edge_endpoints(edge_idx) {
            let src_file = graph.graph[src_idx].file_path.clone();
            let tgt_file = graph.graph[tgt_idx].file_path.clone();
            if src_file != tgt_file {
                file_edges_set.insert((src_file.clone(), tgt_file.clone()));
                let edge_kind = &graph.graph[edge_idx];
                let et = match edge_kind {
                    EdgeKind::Import   => "import",
                    EdgeKind::Call     => "call",
                    EdgeKind::TypeRef  => "type_ref",
                    EdgeKind::ReExport => "re_export",
                }.to_string();
                file_edges_typed.insert((src_file, tgt_file, et));
            }
        }
    }

    let mut incoming_counts: HashMap<String, usize> = HashMap::new();
    let mut outgoing_counts: HashMap<String, usize> = HashMap::new();
    for (src, tgt) in &file_edges_set {
        *outgoing_counts.entry(src.clone()).or_insert(0) += 1;
        *incoming_counts.entry(tgt.clone()).or_insert(0) += 1;
    }

    let nodes: Vec<FileNode> = file_exports
        .into_iter()
        .map(|(file_path, counts)| {
            let total = counts.total;
            let radius = compute_radius(total);
            let filename = filename_map.get(&file_path).cloned()
                .unwrap_or_else(|| file_path.clone());
            let incoming = incoming_counts.get(&file_path).copied().unwrap_or(0);
            let outgoing = outgoing_counts.get(&file_path).copied().unwrap_or(0);
            FileNode { id: file_path.clone(), path: file_path, filename, export_counts: counts, radius, incoming, outgoing }
        })
        .collect();

    // ── Dead code lookup sets ────────────────────────────────────────────────

    let confirmed_ids: HashSet<&str> = dead_result.confirmed.iter()
        .map(|e| e.symbol_id.as_str())
        .collect();
    let suspicious_ids: HashSet<&str> = dead_result.suspicious.iter()
        .map(|e| e.symbol_id.as_str())
        .collect();

    // ── Symbol nodes (exported only, VIZN-03) ───────────────────────────────

    let symbols: Vec<SymbolNodeDto> = graph.graph.node_weights()
        .filter(|n| n.is_exported)
        .map(|n| {
            let kind = match n.kind {
                SymbolKind::Function  => "function",
                SymbolKind::Class     => "class",
                SymbolKind::Type      => "type",
                SymbolKind::Interface => "interface",
                SymbolKind::Hook      => "hook",
                SymbolKind::Enum      => "enum",
                SymbolKind::Module    => "module",
            }.to_string();
            let (is_dead_code, dead_code_confidence) = if confirmed_ids.contains(n.id.as_str()) {
                (true, Some("confirmed".to_string()))
            } else if suspicious_ids.contains(n.id.as_str()) {
                (true, Some("suspicious".to_string()))
            } else {
                (false, None)
            };
            SymbolNodeDto {
                id: n.id.clone(),
                name: n.name.clone(),
                kind,
                file_path: n.file_path.clone(),
                is_dead_code,
                dead_code_confidence,
            }
        })
        .collect();

    // ── Typed edges: symbol-level + deduplicated file-level ─────────────────

    let mut edges: Vec<TypedEdge> = Vec::new();

    // Symbol-level edges (carry actual symbol IDs and edge kind)
    for edge_idx in graph.graph.edge_indices() {
        if let Some((src_idx, tgt_idx)) = graph.graph.edge_endpoints(edge_idx) {
            let edge_kind = &graph.graph[edge_idx];
            let edge_type = match edge_kind {
                EdgeKind::Import   => "import",
                EdgeKind::Call     => "call",
                EdgeKind::TypeRef  => "type_ref",
                EdgeKind::ReExport => "re_export",
            }.to_string();
            edges.push(TypedEdge {
                source: graph.graph[src_idx].id.clone(),
                target: graph.graph[tgt_idx].id.clone(),
                edge_type,
            });
        }
    }

    // Deduplicated file-level edges with preserved edge types
    for (src_file, tgt_file, edge_type) in file_edges_typed {
        edges.push(TypedEdge {
            source: src_file,
            target: tgt_file,
            edge_type,
        });
    }

    EnrichedGraphResponse { nodes, edges, symbols, stats, project_name }
}

/// Compute visual radius per D-53: 8px at 1 export, up to 24px at 20+ exports.
fn compute_radius(total_exports: u32) -> f32 {
    8.0 + (total_exports as f32 / 20.0 * 16.0).min(16.0)
}

/// Truncate a string to at most `max_chars` characters.
// ─── Port Discovery ───────────────────────────────────────────────────────────

/// Find an available TCP port starting from `start`, binding to 127.0.0.1 (T-04-02).
pub async fn find_available_port(start: u16) -> Result<(u16, tokio::net::TcpListener), std::io::Error> {
    for port in start..=65535 {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            Ok(listener) => return Ok((port, listener)),
            Err(_) => {
                if port < 65535 {
                    eprintln!("Port {} in use, trying {}...", port, port + 1);
                }
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrNotAvailable,
        format!("No available port found in range {}..65535", start),
    ))
}

// ─── Axum Handlers ────────────────────────────────────────────────────────────

pub async fn graph_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json((*state.file_graph).clone())
}

pub async fn index_handler() -> impl IntoResponse {
    static_assets::index_handler().await
}

pub async fn static_handler_route(Path(path): Path<String>) -> impl IntoResponse {
    static_assets::static_handler(Path(path)).await
}

// ─── Router ──────────────────────────────────────────────────────────────────

/// Build the axum router with all routes wired to the provided state.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/graph", get(graph_handler))
        .route("/{*path}", get(static_handler_route))
        .with_state(state)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cgraph_core::{EdgeKind, Language, SymbolKind, SymbolNode};
    use cgraph_indexer::{DeadCodeResult, DeadCodeEntry, Confidence};

    fn make_symbol(
        id: &str,
        file_path: &str,
        kind: SymbolKind,
        is_exported: bool,
    ) -> SymbolNode {
        SymbolNode {
            id: id.to_string(),
            name: id.split("::").last().unwrap_or(id).to_string(),
            kind,
            file_path: file_path.to_string(),
            language: Language::TypeScript,
            line_start: 1,
            line_end: 10,
            is_exported,
        }
    }

    fn dummy_stats() -> ScanStats {
        ScanStats {
            files: 0,
            symbols: 0,
            edges: 0,
            elapsed_ms: 0,
        }
    }

    #[test]
    fn test_file_level_projection_basic() {
        let mut graph = CodeGraph::new();

        // 2 exported functions in auth.ts
        graph.add_symbol(make_symbol(
            "src/auth.ts::login",
            "src/auth.ts",
            SymbolKind::Function,
            true,
        ));
        graph.add_symbol(make_symbol(
            "src/auth.ts::logout",
            "src/auth.ts",
            SymbolKind::Function,
            true,
        ));
        // 1 exported class in app.ts
        graph.add_symbol(make_symbol(
            "src/app.ts::App",
            "src/app.ts",
            SymbolKind::Class,
            true,
        ));

        // Edge from app.ts symbol -> auth.ts symbol
        graph.add_edge(
            "src/app.ts::App",
            "src/auth.ts::login",
            EdgeKind::Import,
        );

        let response = file_level_projection(&graph, dummy_stats(), "test".to_string());

        assert_eq!(response.nodes.len(), 2, "should have 2 file nodes");
        assert_eq!(response.edges.len(), 1, "should have 1 deduplicated file edge");

        // Find nodes by path
        let auth_node = response
            .nodes
            .iter()
            .find(|n| n.path == "src/auth.ts")
            .expect("auth.ts node should exist");
        let app_node = response
            .nodes
            .iter()
            .find(|n| n.path == "src/app.ts")
            .expect("app.ts node should exist");

        assert_eq!(auth_node.export_counts.functions, 2);
        assert_eq!(app_node.export_counts.classes, 1);
    }

    #[test]
    fn test_duplicate_basename_disambiguation() {
        let mut graph = CodeGraph::new();

        graph.add_symbol(make_symbol(
            "src/utils/index.ts::helper",
            "src/utils/index.ts",
            SymbolKind::Function,
            true,
        ));
        graph.add_symbol(make_symbol(
            "src/hooks/index.ts::useHook",
            "src/hooks/index.ts",
            SymbolKind::Hook,
            true,
        ));

        let response = file_level_projection(&graph, dummy_stats(), "test".to_string());

        assert_eq!(response.nodes.len(), 2);

        let utils_node = response
            .nodes
            .iter()
            .find(|n| n.path == "src/utils/index.ts")
            .expect("utils/index.ts node should exist");
        let hooks_node = response
            .nodes
            .iter()
            .find(|n| n.path == "src/hooks/index.ts")
            .expect("hooks/index.ts node should exist");

        assert_eq!(utils_node.filename, "utils/index.ts");
        assert_eq!(hooks_node.filename, "hooks/index.ts");
    }

    #[test]
    fn test_radius_capping() {
        let mut graph = CodeGraph::new();

        // Add 25 exported symbols to a single file
        for i in 0..25 {
            graph.add_symbol(make_symbol(
                &format!("src/big.ts::fn{}", i),
                "src/big.ts",
                SymbolKind::Function,
                true,
            ));
        }

        let response = file_level_projection(&graph, dummy_stats(), "test".to_string());

        let big_node = response
            .nodes
            .iter()
            .find(|n| n.path == "src/big.ts")
            .expect("big.ts node should exist");

        // 25 exports: radius = 8.0 + (25/20 * 16).min(16) = 8.0 + 16.0 = 24.0
        assert_eq!(big_node.radius, 24.0, "radius should be capped at 24.0");
    }

    #[test]
    fn test_self_edges_excluded() {
        let mut graph = CodeGraph::new();

        graph.add_symbol(make_symbol(
            "src/a.ts::foo",
            "src/a.ts",
            SymbolKind::Function,
            true,
        ));
        graph.add_symbol(make_symbol(
            "src/a.ts::bar",
            "src/a.ts",
            SymbolKind::Function,
            true,
        ));

        // Edge between two symbols in the SAME file — should be excluded
        graph.add_edge("src/a.ts::foo", "src/a.ts::bar", EdgeKind::Call);

        let response = file_level_projection(&graph, dummy_stats(), "test".to_string());

        assert_eq!(
            response.edges.len(),
            0,
            "self-edges (same file) should be excluded"
        );
    }

    #[test]
    fn test_enriched_response_includes_symbols() {
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_symbol("src/a.ts::foo", "src/a.ts", SymbolKind::Function, true));
        graph.add_symbol(make_symbol("src/a.ts::Bar", "src/a.ts", SymbolKind::Class, true));
        let dead_result = DeadCodeResult::default();
        let resp = enriched_projection(&graph, &dead_result, dummy_stats(), "test".to_string());
        assert_eq!(resp.symbols.len(), 2);
        assert!(resp.symbols.iter().any(|s| s.name == "foo" && s.kind == "function"));
        assert!(resp.symbols.iter().any(|s| s.name == "Bar" && s.kind == "class"));
    }

    #[test]
    fn test_enriched_response_dead_code_flags() {
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_symbol("src/a.ts::foo", "src/a.ts", SymbolKind::Function, true));
        graph.add_symbol(make_symbol("src/a.ts::bar", "src/a.ts", SymbolKind::Function, true));
        let dead_result = DeadCodeResult {
            confirmed: vec![DeadCodeEntry {
                symbol_id: "src/a.ts::foo".to_string(),
                file_path: "src/a.ts".to_string(),
                symbol_name: "foo".to_string(),
                kind: SymbolKind::Function,
                line_start: 1,
                line_end: 10,
                confidence: Confidence::Confirmed,
            }],
            suspicious: vec![],
        };
        let resp = enriched_projection(&graph, &dead_result, dummy_stats(), "test".to_string());
        let foo = resp.symbols.iter().find(|s| s.name == "foo").unwrap();
        assert!(foo.is_dead_code);
        assert_eq!(foo.dead_code_confidence, Some("confirmed".to_string()));
        let bar = resp.symbols.iter().find(|s| s.name == "bar").unwrap();
        assert!(!bar.is_dead_code);
        assert_eq!(bar.dead_code_confidence, None);
    }

    #[test]
    fn test_enriched_response_typed_edges() {
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_symbol("src/a.ts::foo", "src/a.ts", SymbolKind::Function, true));
        graph.add_symbol(make_symbol("src/b.ts::bar", "src/b.ts", SymbolKind::Function, true));
        graph.add_edge("src/a.ts::foo", "src/b.ts::bar", EdgeKind::Call);
        let dead_result = DeadCodeResult::default();
        let resp = enriched_projection(&graph, &dead_result, dummy_stats(), "test".to_string());
        // Symbol-level edge
        assert!(resp.edges.iter().any(|e| e.source == "src/a.ts::foo" && e.target == "src/b.ts::bar" && e.edge_type == "call"));
        // File-level edge also present
        assert!(resp.edges.iter().any(|e| e.source == "src/a.ts" && e.target == "src/b.ts"));
    }
}
