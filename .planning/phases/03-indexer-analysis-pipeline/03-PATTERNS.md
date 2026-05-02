# Phase 3: Indexer & Analysis Pipeline - Pattern Map

**Mapped:** 2026-05-02
**Files analyzed:** 9 new/modified files
**Analogs found:** 8 / 9

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/indexer/Cargo.toml` | config | — | `crates/ts-extractor/Cargo.toml` | exact |
| `crates/indexer/src/lib.rs` | library-root | request-response | `crates/core/src/lib.rs` | exact |
| `crates/indexer/src/crawl.rs` | service | batch | `crates/core/src/detect.rs` | role-match |
| `crates/indexer/src/graph.rs` | service | CRUD | `crates/core/src/extractor.rs` (ExtractionResult) | partial |
| `crates/indexer/src/resolve.rs` | utility | transform | `crates/core/src/detect.rs` (scan_directory) | partial |
| `crates/indexer/src/analysis.rs` | utility | transform | `crates/core/src/detect.rs` (pure functions) | partial |
| `crates/indexer/tests/` (integration tests) | test | batch | `crates/ts-extractor/tests/extraction_test.rs` | exact |
| `crates/cli/src/main.rs` (modified) | controller | request-response | `crates/cli/src/main.rs` itself | exact |
| `Cargo.toml` (workspace, modified) | config | — | `Cargo.toml` itself | exact |

---

## Pattern Assignments

### `crates/indexer/Cargo.toml` (config)

**Analog:** `crates/ts-extractor/Cargo.toml`

**Full pattern** (copy and adapt):
```toml
[package]
name = "cgraph-indexer"
version.workspace = true
edition.workspace = true

[dependencies]
cgraph-core = { path = "../core" }
petgraph = "0.8.3"
serde_json = "1.0"
thiserror = "2.0"

[dev-dependencies]
cgraph-ts-extractor = { path = "../ts-extractor" }
```

Key notes from analog (`crates/ts-extractor/Cargo.toml`):
- `version.workspace = true` and `edition.workspace = true` — always inherit from workspace, never hardcode
- `serde_json` is already in `cgraph-core`'s dependencies; indexer needs it directly for tsconfig parsing in `resolve.rs`
- `thiserror` is already used in `cgraph-core` (same version `"2.0"`) — use the same version
- No `serde` derive needed in indexer (it just reads serde types from core; it doesn't define new serializable structs)
- Dev dependency on `cgraph-ts-extractor` allows integration tests to build a real extractor registry

---

### `crates/indexer/src/lib.rs` (library-root, request-response)

**Analog:** `crates/core/src/lib.rs` (lines 1–8)

**Full pattern** (4 lines in analog):
```rust
pub mod model;
pub mod detect;
pub mod extractor;

// Re-export top-level types for ergonomic imports
pub use model::{Language, SymbolKind, EdgeKind, SymbolNode, SymbolEdge};
pub use extractor::{Extractor, ExtractionResult, ParseError};
pub use detect::{detect_language, scan_directory, DetectionResult};
```

**Adapt as:**
```rust
pub mod crawl;
pub mod graph;
pub mod resolve;
pub mod analysis;

// Re-export top-level types for ergonomic imports (CLI and Phase 4 server consume these)
pub use graph::{CodeGraph};
pub use crawl::{Indexer};
pub use analysis::{DeadCodeResult, Confidence, CycleResult};
```

Notes:
- The `pub mod` block lists all submodules — matches the established "declare then re-export" two-step
- Only pub-re-export what the downstream crate (CLI, Phase 4 server) needs — keep internal impl types crate-private
- `Indexer` is the main entry point struct (constructed in CLI, calls `index(path)`)

---

### `crates/indexer/src/crawl.rs` (service, batch)

**Analog:** `crates/core/src/detect.rs` (lines 1–78)

**Imports pattern** (from analog, lines 1–4):
```rust
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::model::Language;
```

**Adapt imports as:**
```rust
use std::path::Path;
use std::fs;
use cgraph_core::{Extractor, ExtractionResult, scan_directory};
use thiserror::Error;
```

**Core struct pattern** — Indexer struct holds the extractor registry (D-48):
```rust
pub struct Indexer {
    extractors: Vec<Box<dyn Extractor>>,
}

impl Indexer {
    pub fn new(extractors: Vec<Box<dyn Extractor>>) -> Self {
        Self { extractors }
    }
}
```

**Core batch dispatch pattern** — loop structure from `scan_directory` analog (detect.rs lines 37–76):
```rust
// detect.rs pattern: iterate DetectionResult::parseable, match on language
for (path, _lang) in &detection.parseable {
    // File I/O owned by indexer (D-18) — extractor receives source text, not path
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            // D-13: warn and continue — broken file does not stop the scan
            eprintln!("warn: could not read {}: {}", path.display(), e);
            continue;
        }
    };

    // Extractor dispatch: find the first extractor that can_handle this path
    let extractor = self.extractors.iter().find(|e| e.can_handle(path));
    if let Some(ext) = extractor {
        let result = ext.extract(path, &source);
        // Collect nodes, edges, errors — D-13: log errors but continue
        for err in &result.errors {
            eprintln!("warn: {}", err);
        }
        all_nodes.extend(result.nodes);
        all_edges.extend(result.edges);
    }
}
```

**Error type pattern** — from `crates/core/src/extractor.rs` (lines 5–15):
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("I/O error scanning directory {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}
```

Key notes:
- `scan_directory` from `cgraph-core` already handles directory skip logic (hidden dirs, node_modules, dist, build, target) — call it directly; do not re-implement
- The `follow_links(false)` behavior is already baked into `scan_directory` (security: path traversal via symlink)
- Error handling: `detect.rs` uses `continue` on walk errors (line 43); indexer follows the same warn-and-continue pattern (D-13)

---

### `crates/indexer/src/graph.rs` (service, CRUD)

**No exact analog** — this struct has no precedent in the codebase. The closest structural analog is `ExtractionResult` in `crates/core/src/extractor.rs` (lines 17–22), which is a plain data holder. `CodeGraph` is a richer wrapper with mutation methods.

**Imports pattern** (from RESEARCH.md Pattern 1):
```rust
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use cgraph_core::{SymbolNode, EdgeKind};
```

**Core struct pattern** (from RESEARCH.md Pattern 1):
```rust
pub struct CodeGraph {
    pub graph: DiGraph<SymbolNode, EdgeKind>,
    node_index: HashMap<String, NodeIndex>,  // symbol_id -> NodeIndex (O(1) lookup)
    barrel_files: std::collections::HashSet<String>, // file paths of detected barrel files (A2 side-channel)
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
            barrel_files: std::collections::HashSet::new(),
        }
    }

    pub fn add_symbol(&mut self, node: SymbolNode) -> NodeIndex {
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.node_index.insert(id, idx);
        idx
    }

    pub fn add_edge(&mut self, source_id: &str, target_id: &str, kind: EdgeKind) {
        if let (Some(&src), Some(&tgt)) = (
            self.node_index.get(source_id),
            self.node_index.get(target_id),
        ) {
            self.graph.add_edge(src, tgt, kind);
        }
        // Silently skip edges to unknown nodes (D-13: warn and continue)
    }

    pub fn get_index(&self, symbol_id: &str) -> Option<NodeIndex> {
        self.node_index.get(symbol_id).copied()
    }

    pub fn is_barrel_file(&self, file_path: &str) -> bool {
        self.barrel_files.contains(file_path)
    }

    pub fn mark_barrel_file(&mut self, file_path: String) {
        self.barrel_files.insert(file_path);
    }

    pub fn file_count(&self) -> usize {
        // Count unique file_path values across all nodes
        self.graph.node_weights()
            .map(|n| &n.file_path)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }
}
```

Key notes:
- NEVER use `NodeIndex` as a stable identifier — it can become a "hole" on removal (RESEARCH.md Pitfall 4). Use `symbol_id` String as the stable key.
- NEVER call `graph.add_edge(a, b, ...)` with `NodeIndex` values not first verified via `node_index` HashMap (petgraph panics on out-of-bounds index)
- `update_edge` (not `add_edge`) for file-level projection edges to deduplicate (see analysis.rs)
- `barrel_files` is a `HashSet<String>` side-channel, not a field on `SymbolNode` (per RESEARCH.md A2 recommendation)

---

### `crates/indexer/src/resolve.rs` (utility, transform)

**No exact analog** — tsconfig/barrel resolution has no precedent. Closest structural analog is the pure-function module pattern from `crates/core/src/detect.rs` (free functions, no struct state except `TsConfigAliases` which owns its map).

**Imports pattern:**
```rust
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use serde_json::Value;
use cgraph_core::{SymbolEdge, EdgeKind};
use crate::graph::CodeGraph;
```

**tsconfig alias pattern** (from RESEARCH.md Pattern 4 — verified against TypeScript tsconfig docs):
```rust
pub struct TsConfigAliases {
    pub aliases: HashMap<String, Vec<String>>,
    pub base_url: Option<String>,
}

impl TsConfigAliases {
    pub fn load(project_root: &Path) -> Self {
        let tsconfig_path = project_root.join("tsconfig.json");
        let Ok(content) = std::fs::read_to_string(&tsconfig_path) else {
            return Self { aliases: HashMap::new(), base_url: None };  // D-13: graceful fallback
        };
        let Ok(json): Result<Value, _> = serde_json::from_str(&content) else {
            return Self { aliases: HashMap::new(), base_url: None };  // Pitfall 1: JSONC comments
        };
        // ... extract compilerOptions.paths ...
    }

    pub fn resolve(&self, raw_path: &str) -> String {
        for (prefix, targets) in &self.aliases {
            if raw_path.starts_with(prefix.as_str()) {
                if let Some(first_target) = targets.first() {
                    let suffix = &raw_path[prefix.len()..];
                    return format!("{}{}", first_target, suffix);
                }
            }
        }
        raw_path.to_string()  // no alias matched — return unchanged
    }
}
```

**Barrel chain resolution pattern** (iterative work queue — Pitfall 2: avoid recursive resolution):
```rust
// Algorithm: iterative hop-following with cycle guard
// Edge format from ts-extractor/src/edges.rs:
//   Named ReExport: source_id = "barrel_file::symbol_name", target_id = "raw_path::symbol_name"
//   Star ReExport:  source_id = "barrel_file::*",           target_id = "raw_path::*"

pub fn resolve_barrel_chains(code_graph: &mut CodeGraph) {
    let mut visited: HashSet<(String, String)> = HashSet::new();
    let mut work_queue: Vec<(String, String)> = collect_reexport_edges(code_graph);

    while let Some((source_id, target_id)) = work_queue.pop() {
        // Cycle guard (Pitfall 2)
        if !visited.insert((source_id.clone(), target_id.clone())) {
            eprintln!("warn: circular barrel re-export detected: {} -> {}", source_id, target_id);
            continue;
        }
        // Follow hop or finalize edge...
    }
}
```

**Relative path normalization** (Pitfall 5 — use `Path::join` + components(), NOT `canonicalize`):
```rust
// Per RESEARCH.md A3 note: use parent().join() not canonicalize() for fixtures/non-existent paths
fn normalize_import_path(source_file: &Path, raw_import: &str, project_root: &Path) -> PathBuf {
    let base = source_file.parent().unwrap_or(Path::new("."));
    let joined = base.join(raw_import);
    // Normalize .. segments without disk access
    let mut components = Vec::new();
    for c in joined.components() {
        match c {
            std::path::Component::ParentDir => { components.pop(); }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}
```

Key notes:
- tsconfig alias substitution runs BEFORE relative path normalization
- After substitution, verify resolved path is within project root (security: RESEARCH.md threat table)
- Star wildcard expansion (`::*`) runs AFTER all files are extracted — never during extraction (Pitfall 3)

---

### `crates/indexer/src/analysis.rs` (utility, transform)

**Analog:** Pure function pattern from `crates/core/src/detect.rs` (free functions operating on shared data). All analysis functions take `&CodeGraph` (immutable) and return result types.

**Imports pattern:**
```rust
use petgraph::algo::tarjan_scc;
use petgraph::visit::{Dfs, Reversed};
use petgraph::Direction::Incoming;
use std::collections::HashMap;
use cgraph_core::SymbolNode;
use crate::graph::CodeGraph;
```

**Dead code in-degree check** (from RESEARCH.md Code Examples — In-Degree Check):
```rust
// Source: petgraph 0.8.3 neighbors_directed API
fn has_incoming_edges(code_graph: &CodeGraph, idx: petgraph::graph::NodeIndex) -> bool {
    code_graph.graph.neighbors_directed(idx, Incoming).next().is_some()
}
```

**Dead code result types** — follow `ExtractionResult` pattern from `crates/core/src/extractor.rs` (lines 17–22): plain data struct, no methods:
```rust
#[derive(Debug, Clone)]
pub enum Confidence {
    Confirmed,   // exported, zero incoming edges, not entry point, not barrel-reachable
    Suspicious(String), // zero direct edges but demoted by heuristic — String = reason
}

#[derive(Debug)]
pub struct DeadCodeEntry {
    pub symbol_id: String,
    pub file_path: String,
    pub kind: cgraph_core::SymbolKind,
    pub line_start: u32,
    pub line_end: u32,
    pub confidence: Confidence,
}

#[derive(Debug, Default)]
pub struct DeadCodeResult {
    pub confirmed: Vec<DeadCodeEntry>,
    pub suspicious: Vec<DeadCodeEntry>,
}
```

**Cycle detection pattern** (from RESEARCH.md Pattern 2 — Tarjan's SCC on file-level projection):
```rust
pub fn detect_file_cycles(code_graph: &CodeGraph) -> Vec<Vec<String>> {
    // Build file-level projected graph
    let mut file_graph: petgraph::graph::DiGraph<String, ()> = petgraph::graph::DiGraph::new();
    let mut file_index: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

    for node_idx in code_graph.graph.node_indices() {
        let node = &code_graph.graph[node_idx];
        file_index
            .entry(node.file_path.clone())
            .or_insert_with(|| file_graph.add_node(node.file_path.clone()));
    }

    for edge in code_graph.graph.edge_indices() {
        let (src_idx, tgt_idx) = code_graph.graph.edge_endpoints(edge).unwrap();
        let src_file = &code_graph.graph[src_idx].file_path;
        let tgt_file = &code_graph.graph[tgt_idx].file_path;
        if src_file != tgt_file {
            let s = file_index[src_file];
            let t = file_index[tgt_file];
            file_graph.update_edge(s, t, ());  // update_edge deduplicates (NOT add_edge)
        }
    }

    tarjan_scc(&file_graph)
        .into_iter()
        .filter(|scc| scc.len() > 1)
        .map(|scc| scc.iter().map(|&n| file_graph[n].clone()).collect())
        .collect()
}
```

**Blast radius pattern** (from RESEARCH.md Pattern 3 — Reversed DFS):
```rust
pub fn blast_radius(code_graph: &CodeGraph, symbol_id: &str) -> Vec<String> {
    let Some(start) = code_graph.get_index(symbol_id) else {
        return Vec::new();
    };
    let reversed = Reversed(&code_graph.graph);
    let mut dfs = Dfs::new(reversed, start);
    let mut result = Vec::new();
    while let Some(nx) = dfs.next(reversed) {
        if nx != start {
            result.push(code_graph.graph[nx].id.clone());
        }
    }
    result
}
```

**Transitive deps pattern** (from RESEARCH.md Code Examples — Transitive Dependencies DFS):
```rust
pub fn transitive_deps(code_graph: &CodeGraph, symbol_id: &str) -> Vec<String> {
    let Some(start) = code_graph.get_index(symbol_id) else {
        return Vec::new();
    };
    let mut dfs = Dfs::new(&code_graph.graph, start);
    let mut result = Vec::new();
    while let Some(nx) = dfs.next(&code_graph.graph) {
        if nx != start {
            result.push(code_graph.graph[nx].id.clone());
        }
    }
    result
}
```

Key notes:
- Entry point file detection (D-40) is a filename-convention check, not a graph query — pure `Path::file_name()` match
- `tarjan_scc` returns SCCs in reverse topological order; filter `scc.len() > 1` for actual cycles (NOT `connected_components` — that is for undirected graphs only)
- Two-tier confidence (D-41): "suspicious" requires a secondary string-literal scan pass over source text — this is a separate pass after graph assembly, not a graph query

---

### `crates/indexer/tests/` (integration tests, batch)

**Analog:** `crates/ts-extractor/tests/extraction_test.rs` (entire file)

**Test file structure pattern** (extraction_test.rs lines 1–8):
```rust
use std::path::Path;
use cgraph_core::Extractor;
use cgraph_ts_extractor::TsExtractor;
```

**Adapt imports as:**
```rust
use cgraph_indexer::{Indexer, CodeGraph};
use cgraph_ts_extractor::TsExtractor;
```

**Fixture-based test pattern** (extraction_test.rs lines 20–26):
```rust
#[test]
fn extract_returns_no_errors_on_valid_ts() {
    let extractor = TsExtractor::new();
    let source = std::fs::read_to_string("tests/fixtures/schemas.ts")
        .expect("schemas.ts fixture missing");
    let result = extractor.extract(Path::new("tests/fixtures/schemas.ts"), &source);
    assert!(result.errors.is_empty(), "Unexpected parse errors: {:?}", result.errors);
}
```

**Multi-file integration test pattern** — new pattern needed for barrel chain tests:
```rust
#[test]
fn barrel_chain_resolves_to_true_source() {
    // Write multi-file fixture set to a temp directory
    let tmp = std::env::temp_dir().join("cgraph_test_barrel_chain");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("hooks.ts"), "export function useToggle() {}").unwrap();
    std::fs::write(tmp.join("index.ts"), "export { useToggle } from './hooks';").unwrap();
    std::fs::write(tmp.join("consumer.ts"), "import { useToggle } from './index';").unwrap();

    let indexer = Indexer::new(vec![Box::new(TsExtractor::new())]);
    let graph = indexer.index(&tmp).expect("index failed");

    // After barrel resolution, consumer.ts should have a direct edge to hooks.ts::useToggle,
    // not to index.ts::useToggle
    // ...
    std::fs::remove_dir_all(&tmp).ok();
}
```

**Temp directory pattern** (from detect.rs tests, lines 145–169):
```rust
let tmp = std::env::temp_dir().join("cgraph_test_<test_name>");
std::fs::create_dir_all(&tmp).unwrap();
// ... write fixture files ...
// Cleanup:
std::fs::remove_dir_all(&tmp).ok();
```

---

### `crates/cli/src/main.rs` (modified — controller, request-response)

**Analog:** `crates/cli/src/main.rs` itself (current file, lines 1–95)

**Existing imports to extend** (lines 1–5):
```rust
use anyhow::{bail, Result};
use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;
use cgraph_core::{scan_directory, DetectionResult};
```

**Add imports:**
```rust
use std::time::Instant;
use cgraph_indexer::{Indexer, CodeGraph};
use cgraph_ts_extractor::TsExtractor;
```

**Existing Cli struct to extend** (lines 8–16):
```rust
#[derive(Parser, Debug)]
#[command(version, about = "Code graph visualization — cgraph")]
pub struct Cli {
    pub path: PathBuf,
    #[arg(short, long)]
    pub verbose: bool,
}
```

**Add new flags:**
```rust
/// Print detailed dead code report grouped by file
#[arg(long)]
pub dead_code: bool,

/// Print detailed circular dependency report
#[arg(long)]
pub cycles: bool,
```

**Scan statistics output pattern** (from RESEARCH.md Code Examples — Scan Statistics Timing, INFR-03):
```rust
// Replace the current scan_directory call + print_summary with:
let start = Instant::now();
let code_graph = indexer.index(&cli.path)?;
let elapsed = start.elapsed();

println!(
    "cgraph scan: {} files, {} symbols, {} edges ({:.0}ms)",
    code_graph.file_count(),
    code_graph.graph.node_count(),
    code_graph.graph.edge_count(),
    elapsed.as_secs_f64() * 1000.0
);
```

**Path validation pattern** (existing CLI lines 20–27 — keep unchanged):
```rust
// Validate path exists and is a directory (security: T-01-05)
if !cli.path.exists() {
    bail!("Path does not exist: {}", cli.path.display());
}
if !cli.path.is_dir() {
    bail!("Path is not a directory: {}", cli.path.display());
}
```

**Detail report output pattern** — follow existing `print_summary` style (lines 38–95): group by file, sort for deterministic output:
```rust
fn print_dead_code_report(result: &cgraph_indexer::DeadCodeResult) {
    // Group by file path for scannability (D-44: human-readable, grouped by file)
    let mut by_file: HashMap<&str, Vec<_>> = HashMap::new();
    for entry in &result.confirmed {
        by_file.entry(&entry.file_path).or_default().push(entry);
    }
    // Sort file paths for deterministic output (same pattern as print_summary)
    let mut sorted_files: Vec<_> = by_file.keys().copied().collect();
    sorted_files.sort();
    for file in sorted_files {
        println!("  {}", file);
        for entry in &by_file[file] {
            println!("    [{:?}] {} (lines {}-{})", entry.kind, entry.symbol_id, entry.line_start, entry.line_end);
        }
    }
}
```

---

### `Cargo.toml` (workspace, modified)

**Analog:** Existing `Cargo.toml` workspace file

**Current members block** (lines 2–6):
```toml
[workspace]
members = [
    "crates/core",
    "crates/cli",
    "crates/ts-extractor",
]
```

**Add `"crates/indexer"` to members list** — that is the only required change. Keep existing `resolver = "2"` and `[workspace.package]` block unchanged.

---

## Shared Patterns

### Error Handling (D-13: Warn and Continue)
**Source:** `crates/core/src/detect.rs` lines 42–44 and `crates/ts-extractor/src/lib.rs` lines 86–92
**Apply to:** `crawl.rs`, `resolve.rs` (all file I/O and parse paths)
```rust
// Pattern: match on Result, continue on error — never bail on a single file
Some(Err(_)) => continue,  // detect.rs line 43
// or:
let source = match fs::read_to_string(path) {
    Ok(s) => s,
    Err(e) => { eprintln!("warn: {}", e); continue; }
};
```

### thiserror Error Enum
**Source:** `crates/core/src/extractor.rs` lines 5–15
**Apply to:** `crawl.rs` (IndexerError), `resolve.rs` (ResolveError — if needed)
```rust
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Parse produced ERROR nodes in {path} at line {line}")]
    PartialParse { path: String, line: u32 },
}
```
Adapt by changing enum name and variant names; keep the `#[error("...")]` and `#[source]` attribute style.

### Public API Surface Pattern
**Source:** `crates/core/src/lib.rs` lines 1–8
**Apply to:** `crates/indexer/src/lib.rs`
```rust
pub mod <module>;
// ...
pub use <module>::{Type};
```
Declare modules with `pub mod`, then re-export only the public API surface with `pub use`. Keep internal impl details crate-private.

### Workspace Inheritance
**Source:** `crates/ts-extractor/Cargo.toml` lines 1–5
**Apply to:** `crates/indexer/Cargo.toml`
```toml
version.workspace = true
edition.workspace = true
```
Always inherit `version` and `edition` from workspace; never hardcode.

### Fixture-Based Integration Test Setup
**Source:** `crates/core/src/detect.rs` lines 144–169 and `crates/ts-extractor/tests/extraction_test.rs` lines 20–26
**Apply to:** `crates/indexer/tests/`
```rust
// Pattern for single-file test: read fixture file, assert on result fields
let source = std::fs::read_to_string("tests/fixtures/foo.ts").expect("fixture missing");
// Pattern for multi-file test: write temp dir, run indexer, assert, cleanup
let tmp = std::env::temp_dir().join("cgraph_test_<name>");
std::fs::create_dir_all(&tmp).unwrap();
// ... write fixture files ...
std::fs::remove_dir_all(&tmp).ok();
```

### CLI Deterministic Output Sort
**Source:** `crates/cli/src/main.rs` lines 63–66
**Apply to:** `crates/cli/src/main.rs` (dead code and cycles detail reports)
```rust
// Sort for deterministic output
let mut sorted: Vec<_> = map.iter().collect();
sorted.sort_by_key(|(k, _)| k.as_str());
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/indexer/src/graph.rs` | service | CRUD | No graph data structure exists in codebase yet — closest is ExtractionResult (plain data bag). Use RESEARCH.md Pattern 1 (CodeGraph wrapping petgraph DiGraph). |
| `crates/indexer/src/resolve.rs` | utility | transform | No path resolution or tsconfig parsing exists in codebase. Use RESEARCH.md Pattern 4 (TsConfigAliases) and Pattern 5 (barrel chain algorithm outline). |

---

## Metadata

**Analog search scope:** `crates/core/src/`, `crates/ts-extractor/src/`, `crates/ts-extractor/tests/`, `crates/cli/src/`, `crates/cli/tests/`
**Files scanned:** 13 source files
**Pattern extraction date:** 2026-05-02
