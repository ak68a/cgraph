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

        // Phase 2: Add edges after all symbols are in the graph.
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
}
