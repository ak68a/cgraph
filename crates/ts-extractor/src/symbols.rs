use tree_sitter::{Query, QueryCursor, Node, StreamingIterator};
use cgraph_core::{SymbolNode, Language, SymbolKind};
use crate::classify::classify_function;

/// Extract all exported symbols from the AST root node.
/// Pass 1 of the two-pass extraction algorithm.
pub fn extract_symbols(
    root: Node,
    source: &str,
    file_path: &str,
    language: Language,
    symbol_query: &Query,
) -> Vec<SymbolNode> {
    Vec::new() // Stub - implemented in Plan 02
}
