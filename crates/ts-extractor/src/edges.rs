use tree_sitter::{Query, QueryCursor, Node, StreamingIterator};
use cgraph_core::{SymbolEdge, EdgeKind};

/// Extract all edges (imports, calls, type refs, re-exports) from the AST.
/// Pass 2 of the two-pass extraction algorithm.
///
/// Edge ID conventions:
/// - Import: source_id = "file_path::<import>", target_id = "raw_path::symbol_name"
/// - Call: source_id = "file_path::<call>", target_id = "unresolved::called_name"
/// - TypeRef: source_id = "file_path::source_type", target_id = "unresolved::referenced_type"
/// - ReExport: source_id = "file_path::specifier" or "file_path::*", target_id = "raw_path::specifier" or "raw_path::*"
pub fn extract_edges(
    root: Node,
    source: &str,
    file_path: &str,
    import_query: &Query,
    call_query: &Query,
    type_ref_query: &Query,
    type_ann_query: &Query,
    reexport_query: &Query,
    member_ref_query: &Query,
) -> Vec<SymbolEdge> {
    let mut edges = Vec::new();

    extract_imports(root, source, file_path, import_query, &mut edges);
    extract_calls(root, source, file_path, call_query, &mut edges);
    extract_type_refs(root, source, file_path, type_ref_query, &mut edges);
    extract_type_annotations(root, source, file_path, type_ann_query, &mut edges);
    extract_reexports(root, source, file_path, reexport_query, &mut edges);
    extract_member_refs(root, source, file_path, member_ref_query, &mut edges);

    edges
}

/// Extract import edges. Each imported symbol gets its own edge.
/// source_id = "file_path::<import>" (file-level import context)
/// target_id = "raw_import_path::imported_name"
fn extract_imports(
    root: Node,
    source: &str,
    file_path: &str,
    query: &Query,
    edges: &mut Vec<SymbolEdge>,
) {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());

    // Get capture indices by name (returns Option<u32>)
    let import_name_idx = query.capture_index_for_name("import_name");
    let default_import_idx = query.capture_index_for_name("default_import_name");
    let namespace_idx = query.capture_index_for_name("namespace_name");
    let path_idx = query.capture_index_for_name("import_path");

    while let Some(m) = matches.next() {
        let mut import_path: Option<&str> = None;
        let mut imported_names: Vec<(&str, u32)> = Vec::new();

        for cap in m.captures {
            if path_idx.is_some_and(|idx| cap.index == idx) {
                // string_fragment node contains the raw path without quotes
                import_path = Some(&source[cap.node.byte_range()]);
            } else if import_name_idx.is_some_and(|idx| cap.index == idx) {
                let name = &source[cap.node.byte_range()];
                let line = cap.node.start_position().row as u32 + 1;
                imported_names.push((name, line));
            } else if default_import_idx.is_some_and(|idx| cap.index == idx) {
                let name = &source[cap.node.byte_range()];
                let line = cap.node.start_position().row as u32 + 1;
                imported_names.push((name, line));
            } else if namespace_idx.is_some_and(|idx| cap.index == idx) {
                let name = &source[cap.node.byte_range()];
                let line = cap.node.start_position().row as u32 + 1;
                imported_names.push((name, line));
            }
        }

        if let Some(path) = import_path {
            for (name, line) in imported_names {
                edges.push(SymbolEdge {
                    source_id: format!("{}::<import>", file_path),
                    target_id: format!("{}::{}", path, name),
                    kind: EdgeKind::Import,
                    source_location: line,
                });
            }
        }
    }
}

/// Extract call edges. Only direct named calls (identifier in function position).
/// Per D-30: skip obj.method(), dynamic dispatch, callbacks, IIFE.
/// The CALL_QUERY_SRC naturally filters member expressions by requiring
/// `function: (identifier)` — member expressions produce `function: (member_expression)`.
/// source_id = "file_path::<call>"
/// target_id = "unresolved::called_name"
fn extract_calls(
    root: Node,
    source: &str,
    file_path: &str,
    query: &Query,
    edges: &mut Vec<SymbolEdge>,
) {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());

    let target_idx = query.capture_index_for_name("call_target");

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if target_idx.is_some_and(|idx| cap.index == idx) {
                let name = &source[cap.node.byte_range()];
                let line = cap.node.start_position().row as u32 + 1;
                edges.push(SymbolEdge {
                    source_id: format!("{}::<call>", file_path),
                    target_id: format!("unresolved::{}", name),
                    kind: EdgeKind::Call,
                    source_location: line,
                });
            }
        }
    }
}

/// Extract type reference edges: extends, implements.
/// source_id = "file_path::class_or_interface_name"
/// target_id = "unresolved::referenced_type_name"
fn extract_type_refs(
    root: Node,
    source: &str,
    file_path: &str,
    query: &Query,
    edges: &mut Vec<SymbolEdge>,
) {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());

    let class_name_idx = query.capture_index_for_name("class_name");
    let iface_name_idx = query.capture_index_for_name("iface_name");
    let extends_idx = query.capture_index_for_name("extends_target");
    let implements_idx = query.capture_index_for_name("implements_target");

    while let Some(m) = matches.next() {
        let mut source_name: Option<&str> = None;
        let mut targets: Vec<(&str, u32)> = Vec::new();

        for cap in m.captures {
            if class_name_idx.is_some_and(|idx| cap.index == idx) {
                source_name = Some(&source[cap.node.byte_range()]);
            } else if iface_name_idx.is_some_and(|idx| cap.index == idx) {
                source_name = Some(&source[cap.node.byte_range()]);
            } else if extends_idx.is_some_and(|idx| cap.index == idx) {
                let name = &source[cap.node.byte_range()];
                let line = cap.node.start_position().row as u32 + 1;
                targets.push((name, line));
            } else if implements_idx.is_some_and(|idx| cap.index == idx) {
                let name = &source[cap.node.byte_range()];
                let line = cap.node.start_position().row as u32 + 1;
                targets.push((name, line));
            }
        }

        if let Some(src) = source_name {
            for (target, line) in targets {
                edges.push(SymbolEdge {
                    source_id: format!("{}::{}", file_path, src),
                    target_id: format!("unresolved::{}", target),
                    kind: EdgeKind::TypeRef,
                    source_location: line,
                });
            }
        }
    }
}

/// Extract type annotation references (parameter types, return types, generics, unions, etc.).
/// Deduplicates per file: one edge per unique type name.
fn extract_type_annotations(
    root: Node,
    source: &str,
    file_path: &str,
    query: &Query,
    edges: &mut Vec<SymbolEdge>,
) {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());

    let ann_type_idx = query.capture_index_for_name("ann_type");

    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if ann_type_idx.is_some_and(|idx| cap.index == idx) {
                let name = &source[cap.node.byte_range()];
                if seen_names.insert(name.to_string()) {
                    let line = cap.node.start_position().row as u32 + 1;
                    edges.push(SymbolEdge {
                        source_id: format!("{}::<ref>", file_path),
                        target_id: format!("unresolved::{}", name),
                        kind: EdgeKind::TypeRef,
                        source_location: line,
                    });
                }
            }
        }
    }
}

/// Extract re-export edges: named re-exports and star re-exports.
/// Named: source_id = "file_path::specifier_name", target_id = "raw_path::specifier_name"
/// Star: source_id = "file_path::*", target_id = "raw_path::*"
/// Per D-25: only single-hop edges; Phase 3 resolves chains.
/// Per D-26: named and star patterns supported.
///
/// The REEXPORT_QUERY_SRC has two patterns:
/// - Pattern 0: named re-export (requires export_clause with export_specifier)
/// - Pattern 1: star re-export (matches any export_statement with a source — including named ones)
///
/// Pattern 1 can match named re-export statements too (since they have a source), so we must
/// check that the matched export_statement node does NOT have an `export_clause` child before
/// emitting a star edge. True star exports (`export * from`) have no export_clause.
fn extract_reexports(
    root: Node,
    source: &str,
    file_path: &str,
    query: &Query,
    edges: &mut Vec<SymbolEdge>,
) {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());

    let specifier_idx = query.capture_index_for_name("specifier_name");
    let alias_idx = query.capture_index_for_name("alias_name");
    let source_path_idx = query.capture_index_for_name("source_path");
    let star_source_idx = query.capture_index_for_name("star_source");

    while let Some(m) = matches.next() {
        if m.pattern_index == 0 {
            // Named re-export: export { foo, bar } from './module'
            // Aliased re-export: export { foo as bar } from './module'
            // Each match covers one specifier (tree-sitter matches per export_specifier)
            let mut specifier_name: Option<(&str, u32)> = None;
            let mut alias_name: Option<&str> = None;
            let mut named_source_path: Option<&str> = None;

            for cap in m.captures {
                if specifier_idx.is_some_and(|idx| cap.index == idx) {
                    let name = &source[cap.node.byte_range()];
                    let line = cap.node.start_position().row as u32 + 1;
                    specifier_name = Some((name, line));
                } else if alias_idx.is_some_and(|idx| cap.index == idx) {
                    alias_name = Some(&source[cap.node.byte_range()]);
                } else if source_path_idx.is_some_and(|idx| cap.index == idx) {
                    named_source_path = Some(&source[cap.node.byte_range()]);
                }
            }

            if let (Some((name, line)), Some(path)) = (specifier_name, named_source_path) {
                // For aliased re-exports (export { foo as bar }), use the alias as the
                // public name for source_id since consumers import "bar", not "foo".
                // The target_id uses the original name since that's what the source module exports.
                let public_name = alias_name.unwrap_or(name);
                edges.push(SymbolEdge {
                    source_id: format!("{}::{}", file_path, public_name),
                    target_id: format!("{}::{}", path, name),
                    kind: EdgeKind::ReExport,
                    source_location: line,
                });
            }
        } else {
            // Pattern 1: potentially a star re-export — but also matches named re-exports
            // (since named re-exports also have a source field). Guard: only emit a star edge
            // if the export_statement node has no `export_clause` child. A star export
            // `export * from './x'` has no export_clause; a named export does.
            let mut star_path: Option<&str> = None;
            let mut star_line: u32 = 0;
            let mut export_stmt_node: Option<Node> = None;

            for cap in m.captures {
                if star_source_idx.is_some_and(|idx| cap.index == idx) {
                    star_path = Some(&source[cap.node.byte_range()]);
                    star_line = cap.node.start_position().row as u32 + 1;
                    // Walk up to find the export_statement ancestor
                    let mut n = cap.node;
                    loop {
                        if n.kind() == "export_statement" {
                            export_stmt_node = Some(n);
                            break;
                        }
                        match n.parent() {
                            Some(p) => n = p,
                            None => break,
                        }
                    }
                }
            }

            // Distinguish three cases for Pattern 1 matches:
            // 1. namespace_export child present: `export * as ns from './module'`
            // 2. No export_clause and no namespace_export: true `export * from './module'`
            // 3. export_clause present: named re-export already handled by Pattern 0 — skip
            let has_export_clause = export_stmt_node.map_or(true, |stmt| {
                let mut cursor2 = stmt.walk();
                stmt.children(&mut cursor2).any(|child| child.kind() == "export_clause")
            });
            let has_namespace_export = export_stmt_node.map_or(false, |stmt| {
                let mut cursor2 = stmt.walk();
                stmt.children(&mut cursor2).any(|child| child.kind() == "namespace_export")
            });

            if has_namespace_export {
                // `export * as ns from './module'` — emit ReExport with namespace name as source
                // T-02-05-01: Validate namespace_export child node exists before extracting identifier
                if let Some(stmt) = export_stmt_node {
                    let mut ns_cursor = stmt.walk();
                    let ns_name = stmt.children(&mut ns_cursor)
                        .filter(|child| child.kind() == "namespace_export")
                        .flat_map(|ns_node| {
                            let mut inner = ns_node.walk();
                            ns_node.children(&mut inner)
                                .filter(|c| c.kind() == "identifier")
                                .map(|c| &source[c.byte_range()])
                                .collect::<Vec<_>>()
                        })
                        .next();
                    if let (Some(ns), Some(path)) = (ns_name, star_path) {
                        edges.push(SymbolEdge {
                            source_id: format!("{}::{}", file_path, ns),
                            target_id: format!("{}::*", path),
                            kind: EdgeKind::ReExport,
                            source_location: star_line,
                        });
                    }
                }
            } else if !has_export_clause {
                // True star export: `export * from './module'`
                if let Some(path) = star_path {
                    edges.push(SymbolEdge {
                        source_id: format!("{}::*", file_path),
                        target_id: format!("{}::*", path),
                        kind: EdgeKind::ReExport,
                        source_location: star_line,
                    });
                }
            }
            // else: has_export_clause — named re-export already handled by Pattern 0; skip
        }
    }
}

/// Extract member expression object references (e.g., `EnumName.Value`).
/// Captures identifiers used as member expression objects, creating edges so
/// intra-file symbol usage (enums, class statics) prevents false dead code flags.
/// Deduplicates per file: one edge per unique identifier name.
fn extract_member_refs(
    root: Node,
    source: &str,
    file_path: &str,
    query: &Query,
    edges: &mut Vec<SymbolEdge>,
) {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());

    let target_idx = query.capture_index_for_name("ref_target");

    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if target_idx.is_some_and(|idx| cap.index == idx) {
                let name = &source[cap.node.byte_range()];
                if seen_names.insert(name.to_string()) {
                    let line = cap.node.start_position().row as u32 + 1;
                    edges.push(SymbolEdge {
                        source_id: format!("{}::<ref>", file_path),
                        target_id: format!("unresolved::{}", name),
                        kind: EdgeKind::Call,
                        source_location: line,
                    });
                }
            }
        }
    }
}
