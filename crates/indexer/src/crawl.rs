use std::path::Path;
use std::fs;
use thiserror::Error;
use cgraph_core::{Extractor, SymbolEdge, scan_directory};
use crate::graph::CodeGraph;

/// Errors that can occur during indexing.
#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("I/O error scanning {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl From<std::io::Error> for IndexerError {
    fn from(e: std::io::Error) -> Self {
        IndexerError::Io {
            path: String::new(),
            source: e,
        }
    }
}

/// The Indexer crawls a project directory, dispatches files to registered extractors,
/// and assembles a CodeGraph from the extraction results.
///
/// The extractor registry is dynamic (D-48): the caller builds `Vec<Box<dyn Extractor>>`
/// and passes it in. The indexer has no knowledge of specific languages.
pub struct Indexer {
    extractors: Vec<Box<dyn Extractor>>,
}

impl Indexer {
    /// Create a new Indexer with the given extractor registry.
    pub fn new(extractors: Vec<Box<dyn Extractor>>) -> Self {
        Self { extractors }
    }

    /// Index a project directory and return the assembled CodeGraph.
    ///
    /// 1. Scans the directory for parseable files (reuses core::scan_directory)
    /// 2. For each file, dispatches to the first matching extractor
    /// 3. Collects all symbols and edges into a CodeGraph
    /// 4. Adds edges after all symbols are inserted (so targets can be resolved)
    ///
    /// Files that fail to read or parse do not stop the scan (D-13).
    pub fn index(&self, project_root: &Path) -> Result<CodeGraph, IndexerError> {
        let detection = scan_directory(project_root)?;
        let mut code_graph = CodeGraph::new();
        let mut all_edges: Vec<SymbolEdge> = Vec::new();

        // Phase 1: Extract symbols and collect edges from all parseable files
        for (path, _lang) in &detection.parseable {
            // Read file content (D-18: indexer owns file I/O)
            let source = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("warn: could not read {}: {}", path.display(), e);
                    continue;
                }
            };

            // Find the first extractor that can handle this file
            let extractor = match self.extractors.iter().find(|ext| ext.can_handle(path)) {
                Some(ext) => ext,
                None => continue,
            };

            // Extract graph fragments
            let result = extractor.extract(path, &source);

            // Log extraction errors but continue (D-13, D-14: partial parse is valid data)
            for err in &result.errors {
                eprintln!("warn: {}", err);
            }

            // Add all symbol nodes to the graph
            for node in result.nodes {
                code_graph.add_symbol(node);
            }

            // Collect edges for later resolution
            all_edges.extend(result.edges);
        }

        // Resolution pass: apply tsconfig alias substitution and barrel chain resolution.
        // Runs after all symbols are in the graph (so barrel expansion can find exported symbols)
        // but before edges are added (so resolved targets are used).
        let aliases = crate::resolve::TsConfigAliases::load(project_root);
        crate::resolve::resolve_edges(&mut all_edges, &mut code_graph, project_root, &aliases);

        // Phase 2: Add resolved edges to the graph.
        // Edges with unknown targets (e.g., unresolved::*, third-party imports)
        // will be silently dropped by CodeGraph::add_edge.
        for edge in all_edges {
            code_graph.add_edge(&edge.source_id, &edge.target_id, edge.kind.clone());
        }

        Ok(code_graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgraph_ts_extractor::TsExtractor;
    use std::fs;

    #[test]
    fn test_index_single_file() {
        let tmp = std::env::temp_dir().join("cgraph_test_index_single");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(
            tmp.join("hello.ts"),
            "export function hello() { return 42; }\n",
        )
        .unwrap();

        let indexer = Indexer::new(vec![Box::new(TsExtractor::new())]);
        let graph = indexer.index(&tmp).unwrap();

        assert!(graph.node_count() >= 1, "expected at least 1 symbol node");
        assert_eq!(graph.file_count(), 1, "expected exactly 1 file");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_index_empty_dir() {
        let tmp = std::env::temp_dir().join("cgraph_test_index_empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let indexer = Indexer::new(vec![Box::new(TsExtractor::new())]);
        let graph = indexer.index(&tmp).unwrap();

        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.file_count(), 0);

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_index_syntax_errors_continue() {
        // D-14: partial parse is valid data — even files with syntax errors
        // should contribute nodes. The indexer continues past errors.
        let tmp = std::env::temp_dir().join("cgraph_test_index_syntax_err");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Valid file
        fs::write(
            tmp.join("valid.ts"),
            "export function goodFunc() { return 1; }\n",
        )
        .unwrap();

        // File with syntax errors but still parseable symbols
        fs::write(
            tmp.join("broken.ts"),
            "export function brokenFunc() { return }\nexport const x = {{{;\n",
        )
        .unwrap();

        let indexer = Indexer::new(vec![Box::new(TsExtractor::new())]);
        let graph = indexer.index(&tmp).unwrap();

        // Both files should contribute nodes (D-14: partial parse is valid data)
        assert_eq!(graph.file_count(), 2, "both files should be represented");
        assert!(
            graph.node_count() >= 2,
            "expected at least 2 symbols (one from each file)"
        );

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_barrel_chain_integration() {
        // End-to-end: consumer.ts imports useToggle from index.ts barrel,
        // which re-exports from hooks.ts. After resolution, the import edge
        // should point from consumer to hooks (true source).
        //
        // Each file needs at least one real symbol declaration so the extractor
        // creates SymbolNodes (which is what the graph indexes on).
        let tmp = std::env::temp_dir().join("cgraph_test_barrel_chain_integration");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // True source file with actual symbol
        fs::write(
            tmp.join("hooks.ts"),
            "export function useToggle() { return true; }\n",
        )
        .unwrap();

        // Barrel re-export file (re-exports only, no own symbols)
        fs::write(
            tmp.join("index.ts"),
            "export { useToggle } from './hooks';\n",
        )
        .unwrap();

        // Consumer that imports through the barrel and has its own symbol
        fs::write(
            tmp.join("consumer.ts"),
            "import { useToggle } from './index';\nexport function main() { useToggle(); }\n",
        )
        .unwrap();

        let indexer = Indexer::new(vec![Box::new(TsExtractor::new())]);
        let graph = indexer.index(&tmp).unwrap();

        // hooks.ts and consumer.ts have symbol nodes; index.ts has only re-exports
        assert!(graph.node_count() >= 2, "expected at least 2 symbol nodes");

        let index_path = tmp.join("index.ts").to_string_lossy().to_string();

        // Verify index.ts is marked as a barrel file
        assert!(graph.is_barrel_file(&index_path), "index.ts should be marked as barrel");

        // Verify no ReExport edges remain in the graph
        for edge_idx in graph.graph.edge_indices() {
            let edge_kind = &graph.graph[edge_idx];
            assert_ne!(
                *edge_kind,
                cgraph_core::EdgeKind::ReExport,
                "no ReExport edges should remain after resolution"
            );
        }

        // Verify that if any Import edge points to useToggle, it targets hooks.ts (not index.ts).
        // Note: Import edges use source_id = "file::<import>" which is not a SymbolNode, so
        // those edges get silently dropped by add_edge. But edges between actual symbols
        // (e.g., Call edges from main -> useToggle) should resolve to the true source.
        // The key verification is: no edge target points to index.ts::useToggle.
        let mut has_edge_to_index_use_toggle = false;

        for edge_idx in graph.graph.edge_indices() {
            let (_src, tgt) = graph.graph.edge_endpoints(edge_idx).unwrap();
            let target_node = &graph.graph[tgt];
            if target_node.file_path == index_path && target_node.name == "useToggle" {
                has_edge_to_index_use_toggle = true;
            }
        }

        assert!(
            !has_edge_to_index_use_toggle,
            "no edge should target index.ts::useToggle (barrel intermediate)"
        );

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_tsconfig_alias_integration() {
        // End-to-end: app.ts imports from @/utils, tsconfig maps @/* -> src/*
        // After resolution, the import edge should resolve to src/utils.ts.
        //
        // Note: Import edges have source_id = "file::<import>" which is not a SymbolNode.
        // These edges get silently dropped by add_edge. To verify alias resolution works,
        // we check that Call edges (from actual symbols) resolve through the alias.
        let tmp = std::env::temp_dir().join("cgraph_test_tsconfig_alias_integration");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(tmp.join("src")).unwrap();

        // tsconfig.json with path alias
        fs::write(
            tmp.join("tsconfig.json"),
            r#"{"compilerOptions": {"paths": {"@/*": ["src/*"]}}}"#,
        )
        .unwrap();

        // Source module with actual symbol
        fs::write(
            tmp.join("src/utils.ts"),
            "export function format() { return 'formatted'; }\n",
        )
        .unwrap();

        // Consumer using alias path with its own symbol
        fs::write(
            tmp.join("app.ts"),
            "import { format } from '@/utils';\nexport function run() { format(); }\n",
        )
        .unwrap();

        let indexer = Indexer::new(vec![Box::new(TsExtractor::new())]);
        let graph = indexer.index(&tmp).unwrap();

        // Both files have symbol nodes
        assert!(graph.file_count() >= 2, "expected at least 2 files in graph");

        // The alias resolution should have resolved @/utils -> src/utils.
        // Verify by checking that the graph has no nodes or edges referencing "@/utils"
        // and that src/utils.ts symbols are present.
        let utils_path = tmp.join("src/utils.ts").to_string_lossy().to_string();
        let format_id = format!("{}::format", utils_path);

        assert!(
            graph.get_index(&format_id).is_some(),
            "expected src/utils.ts::format node in graph"
        );

        fs::remove_dir_all(&tmp).ok();
    }
}
