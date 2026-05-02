use petgraph::Direction::Incoming;
use std::collections::HashSet;
use std::path::Path;
use cgraph_core::SymbolKind;
use crate::graph::CodeGraph;

/// Confidence tier for dead code detection (D-41).
#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    /// Exported, zero incoming edges, not an entry point file, not re-exported by any barrel.
    Confirmed,
    /// Zero direct edges but demoted by heuristic -- String explains why.
    Suspicious(String),
}

/// A single dead code entry with location and confidence information.
#[derive(Debug, Clone)]
pub struct DeadCodeEntry {
    pub symbol_id: String,
    pub file_path: String,
    pub symbol_name: String,
    pub kind: SymbolKind,
    pub line_start: u32,
    pub line_end: u32,
    pub confidence: Confidence,
}

/// Result of dead code analysis: confirmed dead and suspicious entries.
#[derive(Debug, Default)]
pub struct DeadCodeResult {
    pub confirmed: Vec<DeadCodeEntry>,
    pub suspicious: Vec<DeadCodeEntry>,
}

/// Result of file-level circular dependency detection.
#[derive(Debug, Clone)]
pub struct CycleResult {
    /// Each cycle is an ordered list of file paths forming the import loop.
    pub cycles: Vec<Vec<String>>,
}

/// Entry point filename patterns (just the filename, not full path).
const ENTRY_FILENAMES: &[&str] = &["App.tsx", "App.ts"];

/// Entry point file stems (matched via Path::file_stem()).
const ENTRY_STEMS: &[&str] = &["setup", "config"];

/// Root-only entry filenames (only count as entry points at the project root).
const ROOT_ENTRY_FILENAMES: &[&str] = &["main.ts", "main.tsx", "index.ts", "index.tsx"];

/// Directory names that indicate test files.
const TEST_DIR_NAMES: &[&str] = &["test", "tests", "__tests__", "__test__"];

/// Check if a file is an entry point (D-40).
fn is_entry_point(file_path: &str, project_root: &Path) -> bool {
    let path = Path::new(file_path);
    let file_name = match path.file_name().and_then(|f| f.to_str()) {
        Some(name) => name,
        None => return false,
    };

    // Check if file is in a test directory
    for component in path.components() {
        if let Some(name) = component.as_os_str().to_str() {
            if TEST_DIR_NAMES.contains(&name) {
                return true;
            }
        }
    }

    // Check App.tsx / App.ts
    if ENTRY_FILENAMES.contains(&file_name) {
        return true;
    }

    // Check setup.* / config.* (file stem match)
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        if ENTRY_STEMS.contains(&stem) {
            return true;
        }
    }

    // Check root-only entry filenames (main.ts, index.ts at project root)
    if ROOT_ENTRY_FILENAMES.contains(&file_name) {
        // Check if the file is at the project root by seeing if the parent
        // matches the project root or if the file_path has no directory component
        // beyond the project root
        let file_path_obj = Path::new(file_path);
        if let Some(parent) = file_path_obj.parent() {
            let root_str = project_root.to_str().unwrap_or("");
            // file_path is either at root level (parent is "" or matches project_root)
            if parent.as_os_str().is_empty() || parent == project_root {
                return true;
            }
            // Also check if parent path string matches the root string
            if let Some(parent_str) = parent.to_str() {
                if parent_str == root_str {
                    return true;
                }
            }
        } else {
            // No parent means file is at root
            return true;
        }
    }

    false
}

/// Detect dead code in the graph (ANLS-01, ANLS-02).
///
/// Finds exported symbols with zero incoming edges, excluding:
/// - Entry point file symbols (D-40)
/// - Barrel file symbols
/// - Non-exported symbols
///
/// Two-tier confidence (D-41):
/// - Confirmed: zero edges, not entry, not barrel
/// - Suspicious: zero edges but demoted by heuristics
pub fn dead_code(graph: &CodeGraph, project_root: &Path) -> DeadCodeResult {
    let mut result = DeadCodeResult::default();

    // Collect all edge target IDs that contain "unresolved::" for heuristic check
    let mut unresolved_targets: HashSet<String> = HashSet::new();
    for edge_idx in graph.graph.edge_indices() {
        if let Some((_, tgt)) = graph.graph.edge_endpoints(edge_idx) {
            let tgt_id = &graph.graph[tgt].id;
            if tgt_id.contains("unresolved::") {
                // Extract the symbol name after "unresolved::"
                if let Some(name) = tgt_id.split("unresolved::").nth(1) {
                    unresolved_targets.insert(name.to_string());
                }
            }
        }
    }

    // Collect actual unresolved call target names from edge target_ids
    // Look at all edges where the target node ID starts with "unresolved::"
    // We need to check edge source/target for unresolved patterns
    let mut unresolved_call_names: HashSet<String> = HashSet::new();
    for edge_idx in graph.graph.edge_indices() {
        if let Some((src, tgt)) = graph.graph.edge_endpoints(edge_idx) {
            let edge_kind = &graph.graph[edge_idx];
            let src_node = &graph.graph[src];
            let tgt_node = &graph.graph[tgt];
            // Check if target ID contains "unresolved::" pattern
            if tgt_node.id.starts_with("unresolved::") {
                if let Some(name) = tgt_node.id.strip_prefix("unresolved::") {
                    unresolved_call_names.insert(name.to_string());
                }
            }
            // Also check: if there's a Call edge with target_id containing unresolved
            if matches!(edge_kind, cgraph_core::EdgeKind::Call) && tgt_node.id.contains("unresolved::") {
                if let Some(name) = tgt_node.id.split("unresolved::").nth(1) {
                    unresolved_call_names.insert(name.to_string());
                }
            }
            // Check for namespace import (import * as X) -- edge targeting file::*
            let _ = (src_node, tgt_node); // used above
        }
    }

    // Collect files with namespace imports (edges targeting file_path::*)
    let mut namespace_import_files: HashSet<(String, String)> = HashSet::new(); // (target_file, importing_file)
    for edge_idx in graph.graph.edge_indices() {
        let edge_kind = &graph.graph[edge_idx];
        if matches!(edge_kind, cgraph_core::EdgeKind::Import) {
            if let Some((src, tgt)) = graph.graph.edge_endpoints(edge_idx) {
                let tgt_id = &graph.graph[tgt].id;
                if tgt_id.ends_with("::*") {
                    let target_file = tgt_id.trim_end_matches("::*").to_string();
                    let importing_file = graph.graph[src].file_path.clone();
                    namespace_import_files.insert((target_file, importing_file));
                }
            }
        }
    }

    for node_idx in graph.graph.node_indices() {
        let node = &graph.graph[node_idx];

        // Only check exported symbols
        if !node.is_exported {
            continue;
        }

        // Check if has incoming edges -- if so, not dead
        if graph.graph.neighbors_directed(node_idx, Incoming).next().is_some() {
            continue;
        }

        // Zero incoming edges. Check exclusions:

        // D-40: Entry point file exclusion
        if is_entry_point(&node.file_path, project_root) {
            continue;
        }

        // Barrel file exclusion
        if graph.is_barrel_file(&node.file_path) {
            continue;
        }

        // Heuristic demotion checks (D-41):
        let mut demotion_reason: Option<String> = None;

        // Check 1: unresolved call target matching this symbol's name
        if unresolved_call_names.contains(&node.name) {
            demotion_reason = Some("referenced as unresolved call target".to_string());
        }

        // Check 2: namespace import from another file that could access this symbol
        if demotion_reason.is_none() {
            for (target_file, importing_file) in &namespace_import_files {
                if *target_file == node.file_path {
                    demotion_reason = Some(format!(
                        "namespace import from {} could access this symbol",
                        importing_file
                    ));
                    break;
                }
            }
        }

        let entry = DeadCodeEntry {
            symbol_id: node.id.clone(),
            file_path: node.file_path.clone(),
            symbol_name: node.name.clone(),
            kind: node.kind.clone(),
            line_start: node.line_start,
            line_end: node.line_end,
            confidence: match demotion_reason {
                Some(reason) => Confidence::Suspicious(reason),
                None => Confidence::Confirmed,
            },
        };

        match &entry.confidence {
            Confidence::Confirmed => result.confirmed.push(entry),
            Confidence::Suspicious(_) => result.suspicious.push(entry),
        }
    }

    // Sort by file_path for deterministic output
    result.confirmed.sort_by(|a, b| a.file_path.cmp(&b.file_path).then(a.symbol_id.cmp(&b.symbol_id)));
    result.suspicious.sort_by(|a, b| a.file_path.cmp(&b.file_path).then(a.symbol_id.cmp(&b.symbol_id)));

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgraph_core::{Language, SymbolKind, EdgeKind};

    /// Helper to build a SymbolNode with test defaults.
    fn make_node(id: &str, file_path: &str, name: &str, kind: SymbolKind, exported: bool) -> cgraph_core::SymbolNode {
        cgraph_core::SymbolNode {
            id: id.to_string(),
            name: name.to_string(),
            kind,
            file_path: file_path.to_string(),
            language: Language::TypeScript,
            line_start: 1,
            line_end: 10,
            is_exported: exported,
        }
    }

    #[test]
    fn test_dead_code_confirmed() {
        let mut graph = CodeGraph::new();
        // Node A (exported) in src/utils.ts, Node B (exported) in src/app.ts
        // B imports A, so A has incoming edges, B does not
        graph.add_symbol(make_node("src/utils.ts::helperFn", "src/utils.ts", "helperFn", SymbolKind::Function, true));
        graph.add_symbol(make_node("src/app.ts::appInit", "src/app.ts", "appInit", SymbolKind::Function, true));
        graph.add_edge("src/app.ts::appInit", "src/utils.ts::helperFn", EdgeKind::Import);

        let result = dead_code(&graph, Path::new(""));
        // A has incoming edge (from B), so A is NOT dead.
        // B has zero incoming edges, so B IS dead (confirmed).
        assert_eq!(result.confirmed.len(), 1);
        assert_eq!(result.confirmed[0].symbol_id, "src/app.ts::appInit");
    }

    #[test]
    fn test_dead_code_not_flagged_with_incoming() {
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_node("src/utils.ts::helperFn", "src/utils.ts", "helperFn", SymbolKind::Function, true));
        graph.add_symbol(make_node("src/app.ts::appInit", "src/app.ts", "appInit", SymbolKind::Function, true));
        // appInit imports helperFn, so helperFn has incoming edges
        graph.add_edge("src/app.ts::appInit", "src/utils.ts::helperFn", EdgeKind::Import);

        let result = dead_code(&graph, Path::new(""));
        // helperFn should NOT be in results (has incoming edge)
        let all_ids: Vec<&str> = result.confirmed.iter().chain(result.suspicious.iter())
            .map(|e| e.symbol_id.as_str()).collect();
        assert!(!all_ids.contains(&"src/utils.ts::helperFn"));
    }

    #[test]
    fn test_dead_code_entry_point_exclusion() {
        let mut graph = CodeGraph::new();
        // Node in main.ts at project root with zero incoming edges
        graph.add_symbol(make_node("main.ts::main", "main.ts", "main", SymbolKind::Function, true));

        let result = dead_code(&graph, Path::new(""));
        // main.ts at root is an entry point -- should NOT be flagged
        assert!(result.confirmed.is_empty());
        assert!(result.suspicious.is_empty());
    }

    #[test]
    fn test_dead_code_app_tsx_exclusion() {
        let mut graph = CodeGraph::new();
        // Node in App.tsx with zero incoming edges
        graph.add_symbol(make_node("src/App.tsx::App", "src/App.tsx", "App", SymbolKind::Function, true));

        let result = dead_code(&graph, Path::new(""));
        // App.tsx is an entry point -- should NOT be flagged
        assert!(result.confirmed.is_empty());
        assert!(result.suspicious.is_empty());
    }

    #[test]
    fn test_dead_code_test_file_exclusion() {
        let mut graph = CodeGraph::new();
        // Node in __tests__/utils.test.ts with zero incoming edges
        graph.add_symbol(make_node(
            "__tests__/utils.test.ts::testHelper",
            "__tests__/utils.test.ts",
            "testHelper",
            SymbolKind::Function,
            true,
        ));

        let result = dead_code(&graph, Path::new(""));
        // Test file -- should NOT be flagged
        assert!(result.confirmed.is_empty());
        assert!(result.suspicious.is_empty());
    }

    #[test]
    fn test_dead_code_barrel_exclusion() {
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_node("src/index.ts::exported", "src/index.ts", "exported", SymbolKind::Function, true));
        graph.mark_barrel_file("src/index.ts".to_string());

        let result = dead_code(&graph, Path::new(""));
        // Barrel file -- should NOT be flagged
        assert!(result.confirmed.is_empty());
        assert!(result.suspicious.is_empty());
    }

    #[test]
    fn test_dead_code_non_exported_not_flagged() {
        let mut graph = CodeGraph::new();
        // Non-exported symbol with zero incoming edges
        graph.add_symbol(make_node("src/utils.ts::internal", "src/utils.ts", "internal", SymbolKind::Function, false));

        let result = dead_code(&graph, Path::new(""));
        // Non-exported symbols are not considered for dead code analysis
        assert!(result.confirmed.is_empty());
        assert!(result.suspicious.is_empty());
    }

    #[test]
    fn test_dead_code_suspicious_unresolved_call() {
        let mut graph = CodeGraph::new();
        // Node A (exported, zero incoming) with name "helperFn"
        graph.add_symbol(make_node("src/utils.ts::helperFn", "src/utils.ts", "helperFn", SymbolKind::Function, true));
        // An unresolved call target node matching A's name
        graph.add_symbol(make_node("unresolved::helperFn", "unresolved", "helperFn", SymbolKind::Function, false));
        // A call edge targeting the unresolved node
        graph.add_symbol(make_node("src/other.ts::caller", "src/other.ts", "caller", SymbolKind::Function, true));
        graph.add_edge("src/other.ts::caller", "unresolved::helperFn", EdgeKind::Call);

        let result = dead_code(&graph, Path::new(""));
        // helperFn should be in suspicious (not confirmed) because of unresolved call reference
        assert!(result.confirmed.iter().all(|e| e.symbol_id != "src/utils.ts::helperFn"));
        assert_eq!(result.suspicious.len(), 1);
        assert_eq!(result.suspicious[0].symbol_id, "src/utils.ts::helperFn");
        assert!(matches!(result.suspicious[0].confidence, Confidence::Suspicious(ref reason) if reason.contains("unresolved call target")));
    }
}
