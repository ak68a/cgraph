use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf, Component};
use serde_json::Value;
use cgraph_core::{SymbolEdge, EdgeKind};
use crate::graph::CodeGraph;

/// TsConfigAliases loads and resolves TypeScript path aliases from tsconfig.json (PARS-10, D-28).
///
/// On missing or unparseable tsconfig.json, returns empty aliases (D-13 graceful fallback).
pub struct TsConfigAliases {
    /// Maps alias prefix (e.g., "@/") to real path prefixes (e.g., ["src/"]).
    pub aliases: HashMap<String, Vec<String>>,
    /// tsconfig baseUrl if present.
    pub base_url: Option<String>,
}

impl TsConfigAliases {
    /// Load tsconfig.json from the project root and extract path aliases.
    ///
    /// On file read error or JSON parse error, returns empty aliases (D-13).
    /// T-03-07: Do not expose file contents in error messages.
    pub fn load(project_root: &Path) -> Self {
        let tsconfig_path = project_root.join("tsconfig.json");
        let Ok(content) = std::fs::read_to_string(&tsconfig_path) else {
            return Self { aliases: HashMap::new(), base_url: None };
        };

        // Strip single-line comments before parsing (Pitfall 1: JSONC support).
        // This is a simple pass that removes `// ...` to end-of-line while respecting
        // string literals (tracks whether we're inside a quoted string).
        let stripped = strip_json_comments(&content);

        let Ok(json): Result<Value, _> = serde_json::from_str(&stripped) else {
            eprintln!("warn: could not parse tsconfig.json");
            return Self { aliases: HashMap::new(), base_url: None };
        };

        let compiler_options = &json["compilerOptions"];
        let base_url = compiler_options["baseUrl"].as_str().map(String::from);
        let mut aliases = HashMap::new();

        if let Some(paths) = compiler_options["paths"].as_object() {
            for (alias, targets) in paths {
                // alias like "@/*" -> strip trailing "*" -> "@/"
                let prefix = alias.trim_end_matches('*').to_string();
                let resolved: Vec<String> = targets
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim_end_matches('*').to_string())
                    .collect();
                aliases.insert(prefix, resolved);
            }
        }

        Self { aliases, base_url }
    }

    /// Resolve a raw import path by substituting alias prefixes.
    ///
    /// If no alias matches, returns the path unchanged.
    pub fn resolve(&self, raw_path: &str) -> String {
        for (prefix, targets) in &self.aliases {
            if raw_path.starts_with(prefix.as_str()) {
                if let Some(first_target) = targets.first() {
                    let suffix = &raw_path[prefix.len()..];
                    return format!("{}{}", first_target, suffix);
                }
            }
        }
        raw_path.to_string()
    }
}

/// Strip single-line comments from JSON content (for JSONC/tsconfig support).
/// Handles `//` comments while respecting string literals.
fn strip_json_comments(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            result.push(chars[i]);
            if chars[i] == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if chars[i] == '"' {
                in_string = false;
            }
        } else if chars[i] == '"' {
            in_string = true;
            result.push(chars[i]);
        } else if chars[i] == '/' && i + 1 < len && chars[i + 1] == '/' {
            // Skip to end of line
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        } else {
            result.push(chars[i]);
        }
        i += 1;
    }

    result
}

/// Normalize an import path to a canonical project-relative form.
///
/// For relative paths (starting with `./` or `../`), resolves against
/// the source file's directory. Uses `Path::components()` iteration to
/// handle `..` segments without `canonicalize` (which requires files to exist).
///
/// For non-relative paths (bare modules, already alias-resolved), returns as-is.
pub fn normalize_import_path(source_file: &Path, raw_import: &str, _project_root: &Path) -> PathBuf {
    if raw_import.starts_with("./") || raw_import.starts_with("../") {
        // Relative path: resolve against source file's parent directory
        let base = source_file.parent().unwrap_or(source_file);
        let joined = base.join(raw_import);

        // Normalize path components (handle .. without canonicalize)
        let mut components = Vec::new();
        for comp in joined.components() {
            match comp {
                Component::ParentDir => {
                    components.pop();
                }
                Component::CurDir => {
                    // Skip . components
                }
                _ => {
                    components.push(comp);
                }
            }
        }
        let normalized: PathBuf = components.iter().collect();

        normalized
    } else {
        // Non-relative path (bare module or already resolved)
        PathBuf::from(raw_import)
    }
}

/// Resolve a file path from a raw import: apply alias substitution then normalize.
///
/// T-03-04 mitigation: After alias substitution, verify resolved path does not escape
/// project root. If it escapes, return the raw path (edge will be silently dropped).
pub fn resolve_file_path(
    source_file: &Path,
    raw_import: &str,
    project_root: &Path,
    aliases: &TsConfigAliases,
) -> String {
    // Apply alias resolution first
    let aliased = aliases.resolve(raw_import);

    // Normalize the path
    let normalized = normalize_import_path(source_file, &aliased, project_root);

    // Convert to string with forward slashes (Windows compat)
    let result = normalized.to_string_lossy().replace('\\', "/");

    // T-03-04: Verify path does not escape project root after alias substitution.
    // If the resolved path is absolute and not under project_root, discard it.
    if Path::new(&result).is_absolute() {
        if !result.starts_with(&project_root.to_string_lossy().as_ref()) {
            eprintln!("warn: resolved path escapes project root: {}", raw_import);
            return raw_import.to_string();
        }
    }

    result
}

/// Resolve all edges: apply path aliases, resolve barrel chains, remove ReExport edges.
///
/// This is the main entry point for the resolution pass, called between extraction and
/// edge insertion in the Indexer::index() flow.
///
/// Three passes:
/// - Pass A: Resolve raw paths in edges to canonical file paths
/// - Pass B: Build ReExport hop map, resolve barrel chains with cycle guard (T-03-05)
/// - Pass C: Remove ReExport edges (folded into Import edges)
pub fn resolve_edges(
    edges: &mut Vec<SymbolEdge>,
    graph: &mut CodeGraph,
    project_root: &Path,
    aliases: &TsConfigAliases,
) {
    // --- Pass A: Resolve raw paths to canonical file paths ---
    for edge in edges.iter_mut() {
        // Skip unresolved:: prefixed targets (Call and TypeRef edges)
        if edge.target_id.starts_with("unresolved::") {
            continue;
        }

        // Extract file path and symbol from source_id and target_id
        // Format: "file_path::symbol_name" or "raw_path::symbol_name"
        let (target_raw_path, target_symbol) = match split_id(&edge.target_id) {
            Some(parts) => parts,
            None => continue,
        };

        // Determine the source file for relative path resolution
        let source_file_path = match split_id(&edge.source_id) {
            Some((path, _)) => path.to_string(),
            None => continue,
        };

        // Only resolve paths that look like relative imports or alias paths
        // Absolute paths (already in the graph as file paths) don't need resolution
        if target_raw_path.starts_with('.')
            || aliases.aliases.keys().any(|prefix| target_raw_path.starts_with(prefix.as_str()))
        {
            let resolved = resolve_file_path(
                Path::new(&source_file_path),
                target_raw_path,
                project_root,
                aliases,
            );

            // Try extension resolution: if no known extension, try .ts, .tsx, /index.ts, /index.tsx
            let final_path = resolve_extension(&resolved, target_symbol, graph);

            edge.target_id = format!("{}::{}", final_path, target_symbol);
        }
    }

    // --- Pass B: Build ReExport hop map and resolve barrel chains ---

    // Collect all ReExport edges into a hop map: source_id -> target_id
    let mut hop_map: HashMap<String, String> = HashMap::new();

    for edge in edges.iter() {
        if edge.kind == EdgeKind::ReExport {
            // For star re-exports (source "file::*", target "file::*"),
            // expand to individual symbol entries
            if edge.source_id.ends_with("::*") && edge.target_id.ends_with("::*") {
                let barrel_file = edge.source_id.trim_end_matches("::*");
                let target_file = edge.target_id.trim_end_matches("::*");

                // Find all exported symbols in the target file
                for idx in graph.graph.node_indices() {
                    let node = &graph.graph[idx];
                    if node.is_exported && node.file_path == target_file {
                        let from_key = format!("{}::{}", barrel_file, node.name);
                        let to_key = format!("{}::{}", target_file, node.name);
                        hop_map.insert(from_key, to_key);
                    }
                }
            } else {
                hop_map.insert(edge.source_id.clone(), edge.target_id.clone());
            }

            // Mark barrel files
            if let Some((file_path, _)) = split_id(&edge.source_id) {
                graph.mark_barrel_file(file_path.to_string());
            }
        }
    }

    // Resolve barrel chains for Import edges: follow hops iteratively with cycle guard (T-03-05)
    for edge in edges.iter_mut() {
        if edge.kind != EdgeKind::Import {
            continue;
        }

        let mut current_target = edge.target_id.clone();
        let mut visited: HashSet<String> = HashSet::new();
        let mut hops = 0;
        const MAX_HOPS: usize = 20;

        while let Some(next) = hop_map.get(&current_target) {
            if visited.contains(&current_target) || hops >= MAX_HOPS {
                // Cycle detected or max hops reached -- stop following
                break;
            }
            visited.insert(current_target.clone());
            current_target = next.clone();
            hops += 1;
        }

        edge.target_id = current_target;
    }

    // --- Pass C: Remove ReExport edges ---
    edges.retain(|edge| edge.kind != EdgeKind::ReExport);
}

/// Try to resolve a file path with TypeScript extension resolution.
///
/// If the path already has a known extension, return it.
/// Otherwise try appending .ts, .tsx, /index.ts, /index.tsx and check
/// if a node exists in the graph with that file_path.
fn resolve_extension(resolved_path: &str, symbol: &str, graph: &CodeGraph) -> String {
    // If path already has a known extension, use it
    if resolved_path.ends_with(".ts")
        || resolved_path.ends_with(".tsx")
        || resolved_path.ends_with(".js")
        || resolved_path.ends_with(".jsx")
    {
        return resolved_path.to_string();
    }

    // Try extension candidates
    let candidates = [
        format!("{}.ts", resolved_path),
        format!("{}.tsx", resolved_path),
        format!("{}/index.ts", resolved_path),
        format!("{}/index.tsx", resolved_path),
    ];

    for candidate in &candidates {
        // Check if any node in the graph has this file_path
        let test_id = format!("{}::{}", candidate, symbol);
        if graph.get_index(&test_id).is_some() {
            return candidate.clone();
        }

        // Also check if any node has this file_path (without requiring exact symbol match)
        for idx in graph.graph.node_indices() {
            let node = &graph.graph[idx];
            if node.file_path == *candidate {
                return candidate.clone();
            }
        }
    }

    // No match found -- return the original path
    resolved_path.to_string()
}

/// Split a "file_path::symbol_name" ID into (file_path, symbol_name).
///
/// Returns None if the ID doesn't contain "::".
fn split_id(id: &str) -> Option<(&str, &str)> {
    id.rsplit_once("::")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::CodeGraph;
    use cgraph_core::{SymbolNode, SymbolKind, Language, SymbolEdge, EdgeKind};

    /// Helper to create a SymbolNode for testing.
    fn make_node(id: &str, file_path: &str, name: &str, exported: bool) -> SymbolNode {
        SymbolNode {
            id: id.to_string(),
            name: name.to_string(),
            kind: SymbolKind::Function,
            file_path: file_path.to_string(),
            language: Language::TypeScript,
            line_start: 1,
            line_end: 10,
            is_exported: exported,
        }
    }

    #[test]
    fn test_tsconfig_alias_resolve() {
        let aliases = TsConfigAliases {
            aliases: HashMap::from([("@/".to_string(), vec!["src/".to_string()])]),
            base_url: None,
        };
        assert_eq!(aliases.resolve("@/components/Button"), "src/components/Button");
    }

    #[test]
    fn test_tsconfig_no_match() {
        let aliases = TsConfigAliases {
            aliases: HashMap::from([("@/".to_string(), vec!["src/".to_string()])]),
            base_url: None,
        };
        assert_eq!(aliases.resolve("./local"), "./local");
    }

    #[test]
    fn test_tsconfig_load_missing_file() {
        let aliases = TsConfigAliases::load(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(aliases.aliases.is_empty());
        assert!(aliases.base_url.is_none());
    }

    #[test]
    fn test_normalize_relative_path() {
        let source_file = Path::new("/project/src/hooks/useToggle.ts");
        let project_root = Path::new("/project");
        let result = normalize_import_path(source_file, "../utils/format", project_root);
        assert_eq!(result, PathBuf::from("/project/src/utils/format"));
    }

    #[test]
    fn test_normalize_parent_dir_segments() {
        let source_file = Path::new("/project/src/deep/nested/file.ts");
        let project_root = Path::new("/project");
        let result = normalize_import_path(source_file, "../../utils/helper", project_root);
        assert_eq!(result, PathBuf::from("/project/src/utils/helper"));
    }

    #[test]
    fn test_barrel_chain_single_hop() {
        // Simulate: consumer.ts imports from index.ts, which re-exports from hooks.ts
        let mut graph = CodeGraph::new();

        // Add nodes for all three files
        graph.add_symbol(make_node("hooks.ts::useToggle", "hooks.ts", "useToggle", true));
        graph.add_symbol(make_node("index.ts::useToggle", "index.ts", "useToggle", true));
        graph.add_symbol(make_node("consumer.ts::<import>", "consumer.ts", "<import>", false));

        // Edges before resolution:
        // - consumer imports useToggle from index.ts
        // - index.ts re-exports useToggle from hooks.ts
        let mut edges = vec![
            SymbolEdge {
                source_id: "consumer.ts::<import>".to_string(),
                target_id: "index.ts::useToggle".to_string(),
                kind: EdgeKind::Import,
                source_location: 1,
            },
            SymbolEdge {
                source_id: "index.ts::useToggle".to_string(),
                target_id: "hooks.ts::useToggle".to_string(),
                kind: EdgeKind::ReExport,
                source_location: 1,
            },
        ];

        resolve_edges(&mut edges, &mut graph, Path::new("/project"), &TsConfigAliases {
            aliases: HashMap::new(),
            base_url: None,
        });

        // After resolution:
        // - Import edge should point from consumer to hooks (true source)
        // - ReExport edge should be removed
        assert_eq!(edges.len(), 1, "only Import edge should remain");
        assert_eq!(edges[0].kind, EdgeKind::Import);
        assert_eq!(edges[0].source_id, "consumer.ts::<import>");
        assert_eq!(edges[0].target_id, "hooks.ts::useToggle");
    }

    #[test]
    fn test_barrel_chain_star_expansion() {
        // Simulate: barrel.ts has `export * from './hooks'`
        // hooks.ts exports useToggle and useCurrentUser
        let mut graph = CodeGraph::new();

        graph.add_symbol(make_node("hooks.ts::useToggle", "hooks.ts", "useToggle", true));
        graph.add_symbol(make_node("hooks.ts::useCurrentUser", "hooks.ts", "useCurrentUser", true));
        graph.add_symbol(make_node("consumer.ts::<import>", "consumer.ts", "<import>", false));

        // Star re-export from barrel to hooks
        // Consumer imports useToggle from barrel
        let mut edges = vec![
            SymbolEdge {
                source_id: "consumer.ts::<import>".to_string(),
                target_id: "barrel.ts::useToggle".to_string(),
                kind: EdgeKind::Import,
                source_location: 1,
            },
            SymbolEdge {
                source_id: "barrel.ts::*".to_string(),
                target_id: "hooks.ts::*".to_string(),
                kind: EdgeKind::ReExport,
                source_location: 1,
            },
        ];

        resolve_edges(&mut edges, &mut graph, Path::new("/project"), &TsConfigAliases {
            aliases: HashMap::new(),
            base_url: None,
        });

        // After resolution: import should resolve through the star expansion
        // barrel.ts::useToggle -> hooks.ts::useToggle
        assert_eq!(edges.len(), 1, "only Import edge should remain");
        assert_eq!(edges[0].target_id, "hooks.ts::useToggle");
    }

    #[test]
    fn test_resolve_unresolved_call_single() {
        // A Call edge with target "unresolved::format" is rewritten to "src/utils.ts::format"
        // when a node with name="format", is_exported=true exists in the graph.
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_node("src/utils.ts::format", "src/utils.ts", "format", true));
        graph.add_symbol(make_node("src/app.ts::App", "src/app.ts", "App", true));

        let mut edges = vec![
            SymbolEdge {
                source_id: "src/app.ts::<call>".to_string(),
                target_id: "unresolved::format".to_string(),
                kind: EdgeKind::Call,
                source_location: 5,
            },
        ];

        super::resolve_unresolved_edges(&mut edges, &graph);

        assert_eq!(edges[0].target_id, "src/utils.ts::format");
        assert_eq!(edges[0].kind, EdgeKind::Call);
    }

    #[test]
    fn test_resolve_unresolved_typeref() {
        // A TypeRef edge with target "unresolved::UserProfile" is rewritten to
        // "src/types.ts::UserProfile" when a matching exported type exists.
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_node("src/types.ts::UserProfile", "src/types.ts", "UserProfile", true));
        graph.add_symbol(make_node("src/app.ts::App", "src/app.ts", "App", true));

        let mut edges = vec![
            SymbolEdge {
                source_id: "src/app.ts::App".to_string(),
                target_id: "unresolved::UserProfile".to_string(),
                kind: EdgeKind::TypeRef,
                source_location: 3,
            },
        ];

        super::resolve_unresolved_edges(&mut edges, &graph);

        assert_eq!(edges[0].target_id, "src/types.ts::UserProfile");
        assert_eq!(edges[0].kind, EdgeKind::TypeRef);
    }

    #[test]
    fn test_resolve_unresolved_ambiguous_prefers_import_file() {
        // When two files export "format", and the source file has an Import edge targeting
        // one of those files, the resolution picks the file that was imported.
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_node("src/helpers.ts::format", "src/helpers.ts", "format", true));
        graph.add_symbol(make_node("src/utils.ts::format", "src/utils.ts", "format", true));
        graph.add_symbol(make_node("src/app.ts::App", "src/app.ts", "App", true));

        let mut edges = vec![
            // Import from utils (provides disambiguation context)
            SymbolEdge {
                source_id: "src/app.ts::<import>".to_string(),
                target_id: "src/utils.ts::format".to_string(),
                kind: EdgeKind::Import,
                source_location: 1,
            },
            // Unresolved call to format
            SymbolEdge {
                source_id: "src/app.ts::<call>".to_string(),
                target_id: "unresolved::format".to_string(),
                kind: EdgeKind::Call,
                source_location: 5,
            },
        ];

        super::resolve_unresolved_edges(&mut edges, &graph);

        // Should resolve to utils (since app.ts imports from utils)
        assert_eq!(edges[1].target_id, "src/utils.ts::format");
    }

    #[test]
    fn test_resolve_unresolved_no_match() {
        // A Call edge with target "unresolved::thirdPartyFn" stays unchanged when
        // no exported symbol matches.
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_node("src/utils.ts::format", "src/utils.ts", "format", true));

        let mut edges = vec![
            SymbolEdge {
                source_id: "src/app.ts::<call>".to_string(),
                target_id: "unresolved::thirdPartyFn".to_string(),
                kind: EdgeKind::Call,
                source_location: 10,
            },
        ];

        super::resolve_unresolved_edges(&mut edges, &graph);

        assert_eq!(edges[0].target_id, "unresolved::thirdPartyFn");
    }

    #[test]
    fn test_resolve_extension_js_to_ts() {
        // An import path ending in ".js" resolves to the ".ts" file when a node
        // with file_path ending in ".ts" exists in the graph.
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_node("src/utils.ts::format", "src/utils.ts", "format", true));

        // Test .js -> .ts
        let result = super::resolve_extension("src/utils.js", "format", &graph);
        assert_eq!(result, "src/utils.ts");

        // Test .jsx -> .tsx
        graph.add_symbol(make_node("src/Button.tsx::Button", "src/Button.tsx", "Button", true));
        let result = super::resolve_extension("src/Button.jsx", "Button", &graph);
        assert_eq!(result, "src/Button.tsx");

        // Test .mjs -> .mts
        graph.add_symbol(make_node("src/config.mts::config", "src/config.mts", "config", true));
        let result = super::resolve_extension("src/config.mjs", "config", &graph);
        assert_eq!(result, "src/config.mts");

        // Test .cjs -> .cts
        graph.add_symbol(make_node("src/loader.cts::loader", "src/loader.cts", "loader", true));
        let result = super::resolve_extension("src/loader.cjs", "loader", &graph);
        assert_eq!(result, "src/loader.cts");
    }

    #[test]
    fn test_resolve_extension_index_js() {
        // An import path like "./utils/index.js" resolves to "./utils/index.ts".
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_node("src/utils/index.ts::format", "src/utils/index.ts", "format", true));

        let result = super::resolve_extension("src/utils/index.js", "format", &graph);
        assert_eq!(result, "src/utils/index.ts");
    }

    #[test]
    fn test_barrel_chain_cycle_guard() {
        // Simulate circular barrel re-exports: A re-exports from B, B re-exports from A
        let mut graph = CodeGraph::new();
        graph.add_symbol(make_node("a.ts::foo", "a.ts", "foo", true));
        graph.add_symbol(make_node("b.ts::foo", "b.ts", "foo", true));
        graph.add_symbol(make_node("consumer.ts::<import>", "consumer.ts", "<import>", false));

        let mut edges = vec![
            SymbolEdge {
                source_id: "consumer.ts::<import>".to_string(),
                target_id: "a.ts::foo".to_string(),
                kind: EdgeKind::Import,
                source_location: 1,
            },
            // Circular re-exports
            SymbolEdge {
                source_id: "a.ts::foo".to_string(),
                target_id: "b.ts::foo".to_string(),
                kind: EdgeKind::ReExport,
                source_location: 1,
            },
            SymbolEdge {
                source_id: "b.ts::foo".to_string(),
                target_id: "a.ts::foo".to_string(),
                kind: EdgeKind::ReExport,
                source_location: 1,
            },
        ];

        // This should NOT panic or infinite loop
        resolve_edges(&mut edges, &mut graph, Path::new("/project"), &TsConfigAliases {
            aliases: HashMap::new(),
            base_url: None,
        });

        // Import edge should remain; re-export edges should be removed
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::Import);
        // The target should be either a.ts::foo or b.ts::foo (cycle terminates)
        assert!(
            edges[0].target_id == "b.ts::foo" || edges[0].target_id == "a.ts::foo",
            "cycle should terminate at one of the nodes, got: {}",
            edges[0].target_id
        );
    }
}
