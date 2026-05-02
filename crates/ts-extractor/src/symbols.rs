use tree_sitter::{Query, QueryCursor, Node, StreamingIterator};
use cgraph_core::{SymbolNode, Language, SymbolKind};
use crate::classify::classify_function;

/// Extract all exported symbols from the AST root node.
/// Pass 1 of the two-pass extraction algorithm (PARS-01).
///
/// Uses the pre-compiled symbol_query to match export_statement patterns.
/// Also extracts non-exported top-level function/variable declarations for
/// intra-file call edge resolution in Pass 2.
pub fn extract_symbols(
    root: Node,
    source: &str,
    file_path: &str,
    language: Language,
    symbol_query: &Query,
) -> Vec<SymbolNode> {
    let mut nodes = Vec::new();
    let mut cursor = QueryCursor::new();
    let mut query_matches = cursor.matches(symbol_query, root, source.as_bytes());

    let name_idx = symbol_query.capture_index_for_name("symbol_name");
    let export_stmt_idx = symbol_query.capture_index_for_name("export_stmt");

    while let Some(m) = query_matches.next() {
        let pattern_idx = m.pattern_index;

        // Map pattern_index to SymbolKind (per queries.rs)
        // Pattern 0: exported function declaration -> Function
        // Pattern 1: exported arrow function (const) -> Function
        // Pattern 2: exported interface -> Interface
        // Pattern 3: exported type alias -> Type
        // Pattern 4: exported class -> Class
        // Pattern 5: exported enum -> Enum
        // Pattern 6: export default identifier -> skip (no name capture to extract)
        let base_kind = match pattern_idx {
            0 => SymbolKind::Function,
            1 => SymbolKind::Function,
            2 => SymbolKind::Interface,
            3 => SymbolKind::Type,
            4 => SymbolKind::Class,
            5 => SymbolKind::Enum,
            _ => continue,
        };

        let capture_idx = match name_idx {
            Some(idx) => idx,
            None => continue,
        };

        // Find the @export_stmt capture to get the full declaration span.
        // The @symbol_name capture is just the identifier token (single line),
        // so we use the parent export_statement node for accurate line_start/line_end.
        let (span_start, span_end) = export_stmt_idx
            .and_then(|eidx| {
                m.captures.iter().find(|c| c.index == eidx).map(|c| {
                    (
                        c.node.start_position().row as u32 + 1,
                        c.node.end_position().row as u32 + 1,
                    )
                })
            })
            .unwrap_or((0, 0)); // fallback populated below per-capture

        for cap in m.captures {
            if cap.index == capture_idx {
                let name = &source[cap.node.byte_range()];

                // Reclassify Function as Hook if it follows the use* convention (D-32)
                let kind = if matches!(base_kind, SymbolKind::Function) {
                    classify_function(name)
                } else {
                    base_kind.clone()
                };

                // Use the export_statement span if available, otherwise fall back
                // to the identifier node span (should not happen with current queries).
                let (line_start, line_end) = if span_start > 0 {
                    (span_start, span_end)
                } else {
                    (
                        cap.node.start_position().row as u32 + 1,
                        cap.node.end_position().row as u32 + 1,
                    )
                };

                nodes.push(SymbolNode {
                    id: format!("{}::{}", file_path, name),
                    name: name.to_string(),
                    kind,
                    file_path: file_path.to_string(),
                    language: language.clone(),
                    line_start,
                    line_end,
                    is_exported: true,
                });
                break; // one symbol per match
            }
        }
    }

    // Deduplicate symbols with the same id (handles TypeScript function overloads).
    // Keep the first occurrence (which has the broadest line span for overloads).
    // T-02-05-02: O(n) HashSet allocation — bounded by file size (tree-sitter handles large files)
    let mut seen_ids = std::collections::HashSet::new();
    nodes.retain(|node| seen_ids.insert(node.id.clone()));

    // Also extract non-exported top-level functions for intra-file call edges
    extract_non_exported_functions(root, source, file_path, language, &mut nodes);

    nodes
}

/// Extract non-exported top-level function declarations and arrow functions.
/// These are needed so that intra-file call edges can reference them.
fn extract_non_exported_functions(
    root: Node,
    source: &str,
    file_path: &str,
    language: Language,
    nodes: &mut Vec<SymbolNode>,
) {
    let mut tree_cursor = root.walk();
    // Walk top-level children only (no deep recursion)
    if tree_cursor.goto_first_child() {
        loop {
            let node = tree_cursor.node();

            // Skip export_statements (already captured above)
            if node.kind() == "function_declaration" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &source[name_node.byte_range()];
                    // Only add if not already captured as an export
                    let already_exists = nodes.iter().any(|n| n.name == name);
                    if !already_exists {
                        let kind = classify_function(name);
                        nodes.push(SymbolNode {
                            id: format!("{}::{}", file_path, name),
                            name: name.to_string(),
                            kind,
                            file_path: file_path.to_string(),
                            language: language.clone(),
                            line_start: node.start_position().row as u32 + 1,
                            line_end: node.end_position().row as u32 + 1,
                            is_exported: false,
                        });
                    }
                }
            } else if node.kind() == "lexical_declaration" {
                // Non-exported const arrow functions
                let mut decl_cursor = node.walk();
                if decl_cursor.goto_first_child() {
                    loop {
                        let child = decl_cursor.node();
                        if child.kind() == "variable_declarator" {
                            if let Some(name_node) = child.child_by_field_name("name") {
                                if let Some(value_node) = child.child_by_field_name("value") {
                                    if value_node.kind() == "arrow_function" {
                                        let name = &source[name_node.byte_range()];
                                        let already_exists = nodes.iter().any(|n| n.name == name);
                                        if !already_exists {
                                            let kind = classify_function(name);
                                            nodes.push(SymbolNode {
                                                id: format!("{}::{}", file_path, name),
                                                name: name.to_string(),
                                                kind,
                                                file_path: file_path.to_string(),
                                                language: language.clone(),
                                                line_start: child.start_position().row as u32 + 1,
                                                line_end: child.end_position().row as u32 + 1,
                                                is_exported: false,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        if !decl_cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
            }

            if !tree_cursor.goto_next_sibling() {
                break;
            }
        }
    }
}
