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
    /// Follows `extends` chains with cycle guard (T-03-10, T-03-11).
    /// On file read error or JSON parse error, returns empty aliases (D-13).
    /// T-03-07: Do not expose file contents in error messages.
    pub fn load(project_root: &Path) -> Self {
        let tsconfig_path = project_root.join("tsconfig.json");
        let mut visited = HashSet::new();
        let (aliases, base_url) = Self::load_tsconfig_from_path(&tsconfig_path, &mut visited);
        Self { aliases, base_url }
    }

    /// Internal helper: load a tsconfig from a specific path, following `extends` chains.
    ///
    /// Uses a `visited` set to prevent infinite loops on circular extends (T-03-10).
    /// Each file is visited at most once. Resolve extends paths relative to the
    /// current tsconfig's directory (T-03-11: no absolute path injection).
    fn load_tsconfig_from_path(
        tsconfig_path: &Path,
        visited: &mut HashSet<PathBuf>,
    ) -> (HashMap<String, Vec<String>>, Option<String>) {
        // Canonicalize for cycle detection (resolve symlinks, normalize)
        let canonical = tsconfig_path.canonicalize().unwrap_or_else(|_| tsconfig_path.to_path_buf());

        // Cycle guard: if already visited, return empty
        if visited.contains(&canonical) {
            return (HashMap::new(), None);
        }
        visited.insert(canonical.clone());

        // Read file content
        let Ok(content) = std::fs::read_to_string(tsconfig_path) else {
            return (HashMap::new(), None);
        };

        // Strip comments (JSONC support)
        let stripped = strip_json_comments(&content);

        let Ok(json): Result<Value, _> = serde_json::from_str(&stripped) else {
            eprintln!("warn: could not parse tsconfig.json");
            return (HashMap::new(), None);
        };

        // Step 1: If extends field exists, recursively load parent config first
        let (mut aliases, mut base_url) = if let Some(extends_value) = json.get("extends") {
            if let Some(extends_str) = extends_value.as_str() {
                // Resolve extends path relative to current tsconfig's directory
                let tsconfig_dir = tsconfig_path.parent().unwrap_or(Path::new("."));
                let parent_path = tsconfig_dir.join(extends_str);
                // Add .json extension if not present
                let parent_path = if parent_path.extension().is_none() {
                    parent_path.with_extension("json")
                } else {
                    parent_path
                };
                Self::load_tsconfig_from_path(&parent_path, visited)
            } else {
                (HashMap::new(), None)
            }
        } else {
            (HashMap::new(), None)
        };

        // Step 2: Override with child's values (child wins on conflict)
        let compiler_options = &json["compilerOptions"];

        // Child's baseUrl overrides parent's
        if let Some(bu) = compiler_options["baseUrl"].as_str() {
            base_url = Some(bu.to_string());
        }

        // Child's paths override parent's (full override, not merge)
        if let Some(paths) = compiler_options["paths"].as_object() {
            let mut child_aliases = HashMap::new();
            for (alias, targets) in paths {
                let prefix = alias.trim_end_matches('*').to_string();
                let resolved: Vec<String> = targets
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim_end_matches('*').to_string())
                    .collect();
                child_aliases.insert(prefix, resolved);
            }
            // Child paths fully replace parent paths
            aliases = child_aliases;
        }

        (aliases, base_url)
    }

    /// Resolve a raw import path by substituting alias prefixes, then baseUrl.
    ///
    /// Priority:
    /// 1. Alias prefix matching (paths) — if any alias matches, return that result
    /// 2. baseUrl for bare specifiers (not starting with "." or "/")
    /// 3. Return raw_path unchanged
    pub fn resolve(&self, raw_path: &str) -> String {
        // First candidate from resolve_candidates
        self.resolve_candidates(raw_path).into_iter().next().unwrap_or_else(|| raw_path.to_string())
    }

    /// Return ALL possible resolutions for a raw import path.
    ///
    /// Returns one candidate per path target (alias match), plus baseUrl variant.
    /// If nothing matches, returns vec![raw_path].
    pub fn resolve_candidates(&self, raw_path: &str) -> Vec<String> {
        // Try alias prefix matching first
        for (prefix, targets) in &self.aliases {
            if raw_path.starts_with(prefix.as_str()) {
                let suffix = &raw_path[prefix.len()..];
                let candidates: Vec<String> = targets
                    .iter()
                    .map(|target| format!("{}{}", target, suffix))
                    .collect();
                if !candidates.is_empty() {
                    return candidates;
                }
            }
        }

        // If no alias matched AND path is a bare specifier (not starting with "." or "/"),
        // AND base_url is set, prepend baseUrl
        if let Some(ref base_url) = self.base_url {
            if !raw_path.starts_with('.') && !raw_path.starts_with('/') {
                return vec![format!("{}/{}", base_url, raw_path)];
            }
        }

        // No transformation
        vec![raw_path.to_string()]
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
/// Tries all candidates from alias resolution (multi-target support) and returns the
/// first one that matches a node in the graph. Falls back to the first candidate if
/// none match.
///
/// T-03-04 mitigation: After alias substitution, verify resolved path does not escape
/// project root. If it escapes, return the raw path (edge will be silently dropped).
pub fn resolve_file_path(
    source_file: &Path,
    raw_import: &str,
    project_root: &Path,
    aliases: &TsConfigAliases,
    graph: &CodeGraph,
    symbol: &str,
) -> String {
    let candidates = aliases.resolve_candidates(raw_import);

    let mut first_valid: Option<String> = None;

    for aliased in &candidates {
        // Normalize the path
        let normalized = normalize_import_path(source_file, aliased, project_root);

        // Convert to string with forward slashes (Windows compat)
        let result = normalized.to_string_lossy().replace('\\', "/");

        // T-03-04: Verify path does not escape project root after alias substitution.
        if Path::new(&result).is_absolute() {
            if !result.starts_with(&project_root.to_string_lossy().as_ref()) {
                continue;
            }
        }

        // Try extension resolution for this candidate
        let resolved = resolve_extension(&result, symbol, graph);

        // Check if this resolved path exists in the graph
        let test_id = format!("{}::{}", resolved, symbol);
        if graph.get_index(&test_id).is_some() {
            return resolved;
        }

        // Also check if any node has this file_path
        let found_in_graph = graph.graph.node_indices().any(|idx| {
            graph.graph[idx].file_path == resolved
        });
        if found_in_graph {
            return resolved;
        }

        // Track first valid candidate as fallback
        if first_valid.is_none() {
            first_valid = Some(resolved);
        }
    }

    // Fall back to first candidate if none matched graph
    first_valid.unwrap_or_else(|| {
        // Legacy fallback: single resolve
        let aliased = aliases.resolve(raw_import);
        let normalized = normalize_import_path(source_file, &aliased, project_root);
        let result = normalized.to_string_lossy().replace('\\', "/");

        if Path::new(&result).is_absolute() {
            if !result.starts_with(&project_root.to_string_lossy().as_ref()) {
                eprintln!("warn: resolved path escapes project root: {}", raw_import);
                return raw_import.to_string();
            }
        }

        result
    })
}

/// Resolve unresolved:: Call and TypeRef edge targets by matching called/referenced names
/// to exported symbols in the graph.
///
/// This function runs after resolve_edges() has resolved path-based imports and barrel chains,
/// so the graph already contains all symbol nodes. It matches unresolved targets by name
/// against exported symbols, using import context for disambiguation when multiple matches exist.
///
/// Gap Closure: This resolves the ~75% false-positive dead code issue caused by unresolved edges
/// being silently dropped by add_edge() (root cause of 9% edge ratio on OversizeConnect).
pub fn resolve_unresolved_edges(
    edges: &mut Vec<SymbolEdge>,
    graph: &CodeGraph,
) {
    // Step 1: Build name -> list of symbol IDs for all exported symbols
    let mut exported_by_name: HashMap<String, Vec<String>> = HashMap::new();
    for idx in graph.graph.node_indices() {
        let node = &graph.graph[idx];
        if node.is_exported {
            exported_by_name
                .entry(node.name.clone())
                .or_default()
                .push(node.id.clone());
        }
    }

    // Step 2: Build import context map: source file -> set of imported file paths.
    // Used for disambiguation when multiple symbols share the same name.
    let mut import_context: HashMap<String, HashSet<String>> = HashMap::new();
    for edge in edges.iter() {
        if edge.kind == EdgeKind::Import {
            // Extract source file path from source_id (e.g., "src/app.ts::<import>" -> "src/app.ts")
            let source_file = if let Some((file_part, _)) = edge.source_id.rsplit_once("::") {
                file_part.to_string()
            } else {
                edge.source_id.clone()
            };
            // Extract target file path from target_id (e.g., "src/utils.ts::format" -> "src/utils.ts")
            if let Some((target_file, _)) = edge.target_id.rsplit_once("::") {
                import_context
                    .entry(source_file)
                    .or_default()
                    .insert(target_file.to_string());
            }
        }
    }

    // Step 3: Resolve each unresolved:: edge
    for edge in edges.iter_mut() {
        if !edge.target_id.starts_with("unresolved::") {
            continue;
        }

        let symbol_name = match edge.target_id.strip_prefix("unresolved::") {
            Some(name) => name.to_string(),
            None => continue,
        };

        let candidates = match exported_by_name.get(&symbol_name) {
            Some(ids) => ids,
            None => continue, // No match: leave unchanged (will be dropped by add_edge)
        };

        if candidates.len() == 1 {
            // Exactly one match: rewrite directly
            edge.target_id = candidates[0].clone();
        } else {
            // Multiple matches: disambiguate using import context
            let source_file = if let Some((file_part, _)) = edge.source_id.rsplit_once("::") {
                file_part.to_string()
            } else {
                edge.source_id.clone()
            };

            let imported_files = import_context.get(&source_file);

            // Find candidate whose file_path appears in the source file's import set
            let mut matched: Option<&String> = None;
            if let Some(imports) = imported_files {
                let mut sorted_candidates: Vec<&String> = candidates.iter().collect();
                sorted_candidates.sort();
                for candidate_id in &sorted_candidates {
                    if let Some((candidate_file, _)) = candidate_id.rsplit_once("::") {
                        if imports.contains(candidate_file) {
                            matched = Some(candidate_id);
                            break;
                        }
                    }
                }
            }

            if let Some(id) = matched {
                edge.target_id = id.clone();
            } else {
                // No import disambiguates: pick first sorted alphabetically for determinism
                let mut sorted: Vec<&String> = candidates.iter().collect();
                sorted.sort();
                edge.target_id = sorted[0].clone();
            }
        }
    }
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

        // Resolve paths that look like relative imports, alias paths, or bare specifiers
        // when baseUrl is set. Absolute paths (already in the graph as file paths) don't need resolution.
        let needs_resolution = target_raw_path.starts_with('.')
            || aliases.aliases.keys().any(|prefix| target_raw_path.starts_with(prefix.as_str()))
            || (aliases.base_url.is_some() && !target_raw_path.starts_with('/') && !target_raw_path.starts_with('.'));

        if needs_resolution {
            let resolved = resolve_file_path(
                Path::new(&source_file_path),
                target_raw_path,
                project_root,
                aliases,
                graph,
                target_symbol,
            );

            // resolve_file_path now handles extension resolution internally for multi-target
            // but for single-candidate paths (relative imports), we still need extension resolution
            let final_path = if resolved == target_raw_path
                || resolved.ends_with(".ts")
                || resolved.ends_with(".tsx")
                || resolved.ends_with(".mts")
                || resolved.ends_with(".cts")
            {
                resolved
            } else {
                resolve_extension(&resolved, target_symbol, graph)
            };

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
/// Handles three cases:
/// 1. JS-to-TS mapping: .js->.ts, .jsx->.tsx, .mjs->.mts, .cjs->.cts
/// 2. Already has .ts/.tsx extension: return as-is
/// 3. No extension: try appending .ts, .tsx, /index.ts, /index.tsx
fn resolve_extension(resolved_path: &str, symbol: &str, graph: &CodeGraph) -> String {
    // JS-to-TS mapping: try the TypeScript equivalent of JavaScript extensions BEFORE
    // the early return for known extensions. This handles the common pattern where
    // TypeScript projects use .js extensions in imports but the actual files are .ts.
    let js_to_ts_mappings: &[(&str, &str)] = &[
        (".js", ".ts"),
        (".jsx", ".tsx"),
        (".mjs", ".mts"),
        (".cjs", ".cts"),
    ];

    for (js_ext, ts_ext) in js_to_ts_mappings {
        if resolved_path.ends_with(js_ext) {
            let ts_candidate = format!("{}{}", &resolved_path[..resolved_path.len() - js_ext.len()], ts_ext);

            // Check if the TS version exists in the graph
            let test_id = format!("{}::{}", ts_candidate, symbol);
            if graph.get_index(&test_id).is_some() {
                return ts_candidate;
            }

            // Also check by file_path
            for idx in graph.graph.node_indices() {
                let node = &graph.graph[idx];
                if node.file_path == ts_candidate {
                    return ts_candidate;
                }
            }

            // No TS version found: return the original JS path
            return resolved_path.to_string();
        }
    }

    // If path already has a known TS extension, use it
    if resolved_path.ends_with(".ts") || resolved_path.ends_with(".tsx") {
        return resolved_path.to_string();
    }

    // Extensionless: try extension candidates
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
    fn test_tsconfig_base_url_resolve() {
        // TsConfigAliases with base_url=Some("src") and empty aliases resolves
        // bare specifiers like "lib/build-url" to "src/lib/build-url".
        // Relative paths ("./foo") are NOT modified by baseUrl.
        let aliases = TsConfigAliases {
            aliases: HashMap::new(),
            base_url: Some("src".to_string()),
        };
        // Bare specifier should be resolved via baseUrl
        assert_eq!(aliases.resolve("lib/build-url"), "src/lib/build-url");
        // Relative paths should NOT be modified
        assert_eq!(aliases.resolve("./local"), "./local");
        assert_eq!(aliases.resolve("../utils/format"), "../utils/format");
        // Absolute paths should NOT be modified
        assert_eq!(aliases.resolve("/absolute/path"), "/absolute/path");
    }

    #[test]
    fn test_tsconfig_base_url_with_paths() {
        // When both baseUrl="src" and paths={"@/*": ["src/*"]} exist, paths take priority.
        // "@/utils" resolves to "src/utils" via paths.
        // "lib/format" resolves to "src/lib/format" via baseUrl (no path match).
        let aliases = TsConfigAliases {
            aliases: HashMap::from([("@/".to_string(), vec!["src/".to_string()])]),
            base_url: Some("src".to_string()),
        };
        // Path alias takes priority
        assert_eq!(aliases.resolve("@/utils"), "src/utils");
        // Bare specifier falls through to baseUrl
        assert_eq!(aliases.resolve("lib/format"), "src/lib/format");
    }

    #[test]
    fn test_tsconfig_extends_loads_parent() {
        // Create temp files: tsconfig.json with extends and tsconfig.base.json with paths.
        // TsConfigAliases::load should resolve the extends and inherit the paths.
        let tmp = std::env::temp_dir().join("cgraph_test_extends_loads_parent");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(
            tmp.join("tsconfig.json"),
            r#"{"extends": "./tsconfig.base.json"}"#,
        ).unwrap();

        std::fs::write(
            tmp.join("tsconfig.base.json"),
            r#"{"compilerOptions": {"paths": {"@/*": ["src/*"]}}}"#,
        ).unwrap();

        let aliases = TsConfigAliases::load(&tmp);
        assert_eq!(aliases.resolve("@/utils"), "src/utils");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_tsconfig_extends_child_overrides() {
        // When child tsconfig has paths and parent has different paths,
        // child paths take priority (override, not merge).
        let tmp = std::env::temp_dir().join("cgraph_test_extends_child_overrides");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(
            tmp.join("tsconfig.json"),
            r#"{"extends": "./tsconfig.base.json", "compilerOptions": {"paths": {"@/*": ["app/*"]}}}"#,
        ).unwrap();

        std::fs::write(
            tmp.join("tsconfig.base.json"),
            r#"{"compilerOptions": {"paths": {"@/*": ["src/*"]}}}"#,
        ).unwrap();

        let aliases = TsConfigAliases::load(&tmp);
        // Child's "@/*" -> "app/*" should take priority over parent's "@/*" -> "src/*"
        assert_eq!(aliases.resolve("@/utils"), "app/utils");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_tsconfig_multi_target() {
        // TsConfigAliases with paths={"@/*": ["src/*", "lib/*"]}.
        // resolve_candidates should return all possible candidates.
        let aliases = TsConfigAliases {
            aliases: HashMap::from([("@/".to_string(), vec!["src/".to_string(), "lib/".to_string()])]),
            base_url: None,
        };
        let candidates = aliases.resolve_candidates("@/utils");
        assert_eq!(candidates, vec!["src/utils".to_string(), "lib/utils".to_string()]);
    }

    #[test]
    fn test_tsconfig_extends_circular_guard() {
        // Create temp files where tsconfig.json extends tsconfig.base.json which extends tsconfig.json.
        // Load should not infinite loop.
        let tmp = std::env::temp_dir().join("cgraph_test_extends_circular");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(
            tmp.join("tsconfig.json"),
            r#"{"extends": "./tsconfig.base.json", "compilerOptions": {"paths": {"@/*": ["src/*"]}}}"#,
        ).unwrap();

        std::fs::write(
            tmp.join("tsconfig.base.json"),
            r#"{"extends": "./tsconfig.json", "compilerOptions": {"baseUrl": "src"}}"#,
        ).unwrap();

        // This should NOT infinite loop — returns whatever was loaded before the cycle
        let aliases = TsConfigAliases::load(&tmp);
        // The child's paths should be present
        assert_eq!(aliases.resolve("@/utils"), "src/utils");
        // The parent's baseUrl should be inherited
        assert!(aliases.base_url.is_some());

        std::fs::remove_dir_all(&tmp).ok();
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
