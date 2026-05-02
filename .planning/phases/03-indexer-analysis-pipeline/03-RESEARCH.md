# Phase 3: Indexer & Analysis Pipeline - Research

**Researched:** 2026-05-02
**Domain:** Rust graph assembly, petgraph algorithms, tsconfig path resolution, dead code analysis
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Dead Code Detection**
- D-40: Entry points are identified by convention — zero config. Auto-detected entry point files: main.ts/index.ts at project root, files in test/ or __tests__/, App.tsx/App.ts (React entry), setup.*/config.* patterns, files with side-effect-only imports. Exported symbols in entry point files are never flagged as dead code.
- D-41: Two-tier confidence model: **confirmed dead** (exported, zero incoming edges, not an entry point, not re-exported by any barrel) vs **suspicious** (zero direct edges but demoted by heuristics: symbol name appears as a string literal elsewhere in the project, file has a namespace import `import * as X` that could access the symbol, or symbol is a type used only in generic constraints).

**Circular Dependency Detection**
- D-42: File-level cycle detection only. Detect cycles between files via import edges (A.ts imports B.ts imports A.ts). Symbol-level call cycles (mutual recursion) are intentional patterns and are not flagged. Cycles are reported as ordered chains showing the import path.

**CLI Output**
- D-43: Default `cg <path>` prints scan statistics (INFR-03: files scanned, symbols found, edges found, elapsed time) plus analysis summary (dead code count by confidence tier, circular dependency count). Detail flags `--dead-code` and `--cycles` print full reports. Blast radius and transitive dependency queries are deferred to Phase 11 (query CLI).
- D-44: Detailed output (`--dead-code`, `--cycles`) uses human-readable text format grouped by file, with symbol kind and line ranges. Machine-readable JSON output deferred to Phase 11 (`--json` flag, AGNT-01).

**Graph Storage**
- D-45: Use petgraph's `DiGraph<SymbolNode, EdgeKind>` as the in-memory graph. Built-in Tarjan's SCC for cycle detection, DFS for blast radius / transitive deps. Avoids reimplementing graph algorithms.
- D-46: Single symbol-level graph with file-level views derived on demand (project symbol edges to file pairs, deduplicate, run cycle detection on the projection). No separate file-level graph — single source of truth, derived views computed as needed.

**Crate Organization**
- D-47: New `crates/indexer` crate with modules: `lib.rs` (public API), `graph.rs` (CodeGraph struct wrapping petgraph), `resolve.rs` (barrel chain + tsconfig alias resolution), `analysis.rs` (dead code, blast radius, cycles), `crawl.rs` (file crawl + extractor dispatch). Depends on `cgraph-core` and `petgraph`. CLI depends on indexer.
- D-48: Dynamic extractor registry — `Indexer::new(extractors: Vec<Box<dyn Extractor>>)`. The CLI builds the registry and passes it in. The indexer crate has no direct dependency on any extractor crate, staying language-agnostic. Adding Swift/Go/Python extractors (Phases 7-9) only requires changes to the CLI registration, not the indexer.

**Barrel & Path Resolution (from Phase 2 context)**
- D-25 (carried): Extractor emits ReExport edges only — indexer resolves multi-hop barrel chains to find the true source.
- D-26 (carried): Star re-exports emit a wildcard marker for the indexer to expand.
- D-28 (carried): Extractor emits raw import paths. Indexer reads tsconfig.json once and resolves all alias paths during graph assembly.

### Claude's Discretion
None specified — all Phase 3 decisions are locked.

### Deferred Ideas (OUT OF SCOPE)
- Config file for entry points (`.cgraph.toml`) — override conventions for non-standard project layouts.
- Symbol-level cycle detection — mutual recursion detection.
- Directory/module-level cycle detection — architectural cycle view for large monorepos.
- Blast radius CLI query (`cg blast-radius <symbol-id>`) — deferred to Phase 11 (Agent Interface, AGNT-02).
- JSON output format (`--json` flag) — deferred to Phase 11 (AGNT-01).
- Graph caching / incremental rebuild — if performance becomes an issue on large codebases.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PARS-09 | Tool resolves multi-hop barrel re-export chains to find the true source | Barrel chain resolution algorithm in `resolve.rs`; iterative hop following using ReExport edges; star re-export wildcard expansion |
| PARS-10 | Tool resolves TypeScript path aliases (tsconfig paths, babel moduleNameMapper) | tsconfig.json JSON parsing with serde_json; prefix-match alias resolution; fallback to raw path when no alias matches |
| ANLS-01 | Tool identifies dead code (exported symbols with zero incoming edges) | petgraph `neighbors_directed(n, Incoming)` to count incoming edges; entry point file exclusion by filename convention |
| ANLS-02 | Dead code detection uses confidence scoring (suspicious vs confirmed dead) | Two-tier model: confirmed (zero edges + not entry + not barrel-reachable) vs suspicious (demoted by string-literal scan, namespace import presence, type-only usage heuristics) |
| ANLS-03 | Tool detects circular dependencies between modules | File-level graph projection derived from symbol graph; `tarjan_scc` from petgraph on file projection; SCCs with size > 1 are cycles |
| ANLS-04 | Tool computes transitive dependents for any symbol (blast radius) | DFS on `Reversed<&DiGraph>` starting from target symbol; collects all reachable node IDs |
| ANLS-05 | Tool computes transitive dependencies for any symbol (what it uses) | DFS on normal `&DiGraph` starting from symbol; collects all reachable node IDs |
| INFR-03 | Tool displays scan statistics after parsing (files, symbols, edges, time) | `std::time::Instant::now()` before crawl; format: `cgraph scan: {files} files, {symbols} symbols, {edges} edges ({time}ms)` |
</phase_requirements>

---

## Summary

Phase 3 adds a new `crates/indexer` crate that assembles the full in-memory graph from extractor output and runs analysis algorithms. The work splits into four distinct concerns: (1) file crawl and extractor dispatch (`crawl.rs`), (2) graph construction with barrel chain resolution and tsconfig alias resolution (`resolve.rs`), (3) analysis algorithms (`analysis.rs`), and (4) the public API surface consumed by the CLI and future HTTP server (`graph.rs` / `lib.rs`).

The graph storage decision (D-45/D-46) is already made: `petgraph::DiGraph<SymbolNode, EdgeKind>` as the single source of truth. All required algorithms are built into petgraph 0.8.3 — `tarjan_scc` for cycle detection, `Dfs` for blast radius and transitive dependency traversal, `neighbors_directed` with `Incoming` for dead code in-degree checks. The indexer crate has no knowledge of language-specific extractors (D-48), accepting only `Vec<Box<dyn Extractor>>` at construction time.

The most algorithmically tricky parts are barrel chain resolution (iterating ReExport edges to find the true defining node, handling `::*` wildcards) and the two-tier dead code confidence model (D-41 requires a secondary string-literal scan and namespace import detection pass). The tsconfig path alias resolution is straightforward JSON parsing with prefix-match substitution.

**Primary recommendation:** Build in module order — crawl, resolve, graph, analysis, CLI wiring. Test each layer independently with fixture files before integrating.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| File discovery | indexer (`crawl.rs`) | core (`scan_directory`) | `scan_directory` already exists; crawl.rs calls it to get file paths then dispatches to extractors |
| Extractor dispatch | indexer (`crawl.rs`) | — | Indexer owns all file I/O (D-18); extractors receive source text |
| Graph construction | indexer (`graph.rs`) | — | Assembles SymbolNode/Edge fragments into petgraph DiGraph; node dedup via HashMap |
| Barrel chain resolution | indexer (`resolve.rs`) | — | Multi-file concern; can't live in extractor (D-25); resolves ReExport hops to true source |
| tsconfig alias resolution | indexer (`resolve.rs`) | — | Project-level concern; reads tsconfig.json once; substitutes aliases in all edges |
| Dead code detection | indexer (`analysis.rs`) | — | Needs full graph (incoming edge counts across all files); pure graph query |
| Circular dependency detection | indexer (`analysis.rs`) | — | File-level SCC on derived projection from symbol graph |
| Blast radius / transitive deps | indexer (`analysis.rs`) | — | DFS traversal; ANLS-04/05 data lives in graph.rs public API |
| CLI output formatting | CLI (`crates/cli`) | indexer (analysis structs) | CLI formats results; indexer returns structured data types |
| Scan statistics | CLI (`crates/cli`) | — | `std::time::Instant` in CLI before calling indexer |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| petgraph | 0.8.3 | Directed graph storage, Tarjan's SCC, DFS traversal | Only mature Rust graph library with built-in algorithms; already chosen in D-45 |
| serde_json | 1.0.149 | Parse tsconfig.json for path alias maps | Already a workspace dependency in core; zero new weight |
| cgraph-core | workspace | SymbolNode, SymbolEdge, EdgeKind, Extractor trait | This project's own trait definitions |
| cgraph-ts-extractor | workspace | TsExtractor registered in CLI | First extractor in the registry |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| walkdir | 2.5.0 | Directory traversal (already used in core) | Called via `scan_directory` from core — no direct dep in indexer |
| anyhow | 1.0.102 | Error handling in CLI | Already in CLI crate; indexer uses thiserror for library errors |
| thiserror | 2.0.18 | Typed error enum for indexer | Library crates expose typed errors |
| std::time::Instant | stdlib | Elapsed time for INFR-03 | No external crate needed; `Instant::now()` before crawl |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| petgraph DiGraph | Custom HashMap adjacency | petgraph provides Tarjan's SCC and DFS for free; hand-rolling is high risk (D-45) |
| serde_json for tsconfig | jsonc-parser | tsconfig.json is valid JSON5/JSONC (has comments); however, the `paths` section used for alias resolution does not contain comments in practice, and serde_json with lenient parsing handles the required subset |
| Iterative barrel resolution | Recursive resolution | Iterative avoids stack overflow on deep barrel chains; use a work queue |

**Installation:**
```bash
# In crates/indexer/Cargo.toml:
# petgraph = "0.8.3"
# serde_json = "1.0"
# thiserror = "2.0"
# cgraph-core = { path = "../core" }
```

**Version verification:** [VERIFIED: cargo search petgraph, serde_json, walkdir, anyhow, thiserror, clap — all versions confirmed against crates.io registry on 2026-05-02]

---

## Architecture Patterns

### System Architecture Diagram

```
CLI entry point (crates/cli/src/main.rs)
  │
  ├── builds extractor registry: Vec<Box<dyn Extractor>>
  │     └── [TsExtractor]  (Phase 3: TS only)
  │
  └── Indexer::new(extractors) → index(path) → CodeGraph
        │
        ├── crawl.rs: scan_directory(path) → parseable file list
        │     └── for each file: fs::read_to_string → extractor.extract()
        │                        → ExtractionResult { nodes, edges, errors }
        │
        ├── resolve.rs: path alias resolution pass
        │     ├── read tsconfig.json → HashMap<alias_prefix, real_path>
        │     └── for each edge with raw path → substitute alias if matched
        │
        ├── resolve.rs: barrel chain resolution pass
        │     ├── collect all ReExport edges
        │     ├── iterative hop-following: follow ReExport chain until no more hops
        │     ├── star wildcard expansion: file::* → resolve all exported symbols
        │     └── rewrite edges to point at true defining nodes
        │
        └── graph.rs: CodeGraph assembly
              ├── HashMap<symbol_id, NodeIndex> for dedup
              ├── DiGraph<SymbolNode, EdgeKind> node/edge insertion
              └── analysis.rs: dead code, blast radius, cycles
                    ├── dead_code(): in-degree 0 + confidence scoring
                    ├── blast_radius(id): Dfs on Reversed<&graph>
                    ├── transitive_deps(id): Dfs on &graph
                    └── cycles(): tarjan_scc on file-level projection
```

### Recommended Project Structure
```
crates/
├── indexer/
│   ├── Cargo.toml                   # deps: cgraph-core, petgraph, serde_json, thiserror
│   └── src/
│       ├── lib.rs                   # pub use Indexer, CodeGraph, AnalysisResult
│       ├── crawl.rs                 # file walk + extractor dispatch
│       ├── graph.rs                 # CodeGraph struct wrapping DiGraph
│       ├── resolve.rs               # barrel chain + tsconfig alias resolution
│       └── analysis.rs             # dead code, blast radius, cycles
```

### Pattern 1: CodeGraph Struct Wrapping petgraph DiGraph

**What:** A newtype struct that owns the `DiGraph<SymbolNode, EdgeKind>` and a `HashMap<String, NodeIndex>` for O(1) lookup by symbol ID.
**When to use:** All graph operations — this is the single source of truth (D-46).

```rust
// Source: petgraph 0.8.3 docs (https://docs.rs/petgraph/latest/petgraph/graph/type.DiGraph.html)
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use cgraph_core::{SymbolNode, EdgeKind};

pub struct CodeGraph {
    pub graph: DiGraph<SymbolNode, EdgeKind>,
    node_index: HashMap<String, NodeIndex>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
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
}
```

### Pattern 2: Tarjan's SCC for File-Level Cycle Detection (ANLS-03)

**What:** Project symbol edges onto file pairs, deduplicate, run `tarjan_scc` on the derived file graph. SCCs with size > 1 are cycles.
**When to use:** `--cycles` flag or analysis summary in default output.

```rust
// Source: petgraph 0.8.3 docs (https://docs.rs/petgraph/latest/petgraph/algo/fn.tarjan_scc.html)
use petgraph::algo::tarjan_scc;
use petgraph::graph::DiGraph;
use std::collections::HashMap;

pub fn detect_file_cycles(code_graph: &CodeGraph) -> Vec<Vec<String>> {
    // Build file-level graph: nodes are file paths, edges are import relationships
    let mut file_graph: DiGraph<String, ()> = DiGraph::new();
    let mut file_index: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

    for node_idx in code_graph.graph.node_indices() {
        let node = &code_graph.graph[node_idx];
        file_index
            .entry(node.file_path.clone())
            .or_insert_with(|| file_graph.add_node(node.file_path.clone()));
    }

    // Add file-level edges (deduplicated via HashSet or update_edge)
    for edge in code_graph.graph.edge_indices() {
        let (src_idx, tgt_idx) = code_graph.graph.edge_endpoints(edge).unwrap();
        let src_file = &code_graph.graph[src_idx].file_path;
        let tgt_file = &code_graph.graph[tgt_idx].file_path;
        if src_file != tgt_file {
            let s = file_index[src_file];
            let t = file_index[tgt_file];
            file_graph.update_edge(s, t, ());  // update_edge prevents duplicate edges
        }
    }

    // tarjan_scc returns SCCs in reverse topological order; filter to size > 1
    tarjan_scc(&file_graph)
        .into_iter()
        .filter(|scc| scc.len() > 1)
        .map(|scc| scc.iter().map(|&n| file_graph[n].clone()).collect())
        .collect()
}
```

### Pattern 3: Blast Radius via Reversed DFS (ANLS-04)

**What:** To find all symbols that transitively depend on a given symbol, run DFS on the graph with edge directions reversed (i.e., follow edges backwards). `Reversed<&G>` is petgraph's built-in adaptor for this.
**When to use:** `blast_radius(symbol_id)` method on CodeGraph.

```rust
// Source: petgraph 0.8.3 docs (https://docs.rs/petgraph/latest/petgraph/visit/struct.Reversed.html)
use petgraph::visit::{Dfs, Reversed};

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

### Pattern 4: tsconfig Path Alias Resolution (PARS-10)

**What:** Read `tsconfig.json` once, extract `compilerOptions.paths`, build a prefix map, then substitute matching prefixes in all raw import paths.
**When to use:** During the resolve pass, before graph assembly. Applied to all edge `target_id` values that start with an alias prefix.

```rust
// Source: serde_json 1.0 docs; tsconfig paths format from TypeScript docs
// [CITED: https://www.typescriptlang.org/tsconfig/#paths]
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub struct TsConfigAliases {
    /// Maps alias prefix (e.g., "@/") to one or more real path prefixes (e.g., "src/")
    pub aliases: HashMap<String, Vec<String>>,
    pub base_url: Option<String>,
}

impl TsConfigAliases {
    pub fn load(project_root: &Path) -> Self {
        let tsconfig_path = project_root.join("tsconfig.json");
        let Ok(content) = std::fs::read_to_string(&tsconfig_path) else {
            return Self { aliases: HashMap::new(), base_url: None };
        };
        let Ok(json): Result<Value, _> = serde_json::from_str(&content) else {
            return Self { aliases: HashMap::new(), base_url: None };
        };

        let compiler_options = &json["compilerOptions"];
        let base_url = compiler_options["baseUrl"].as_str().map(String::from);
        let mut aliases = HashMap::new();

        if let Some(paths) = compiler_options["paths"].as_object() {
            for (alias, targets) in paths {
                // alias like "@/*" → strip trailing "/*" → "@/"
                let prefix = alias.trim_end_matches('*').to_string();
                let resolved: Vec<String> = targets
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim_end_matches('*').to_string())
                    .collect();
                aliases.insert(prefix, resolved);
            }
        }

        Self { aliases, base_url }
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
        raw_path.to_string()
    }
}
```

### Pattern 5: Barrel Chain Resolution (PARS-09)

**What:** Follow ReExport edge chains iteratively until no more ReExport hops exist. Replace the original edge with a direct edge from the importer to the true source. Handle `::*` wildcards by expanding to all exported symbols in the target file.
**When to use:** After all files are extracted, before finalizing the graph. All ReExport intermediate edges are removed from the final graph.

```rust
// [ASSUMED] — Algorithm design based on D-25/D-26 decisions and edge format from
// crates/ts-extractor/src/edges.rs. Not a library pattern.
// Edge format from extractor:
//   Named ReExport: source_id = "barrel_file::symbol_name", target_id = "raw_path::symbol_name"
//   Star ReExport:  source_id = "barrel_file::*",           target_id = "raw_path::*"

// Algorithm outline (iterative, not recursive — avoids stack overflow):
// 1. Collect all ReExport edges into a work queue
// 2. For each edge in queue:
//    a. If target_id resolves to a node (true source): rewrite edge kind to Import, done
//    b. If target_id is another barrel (raw_path::symbol exists as ReExport source):
//       follow the hop, re-add to queue
//    c. If target_id ends in ::*: expand to all exported symbols in raw_path, emit edges
// 3. Remove all intermediate ReExport edges from graph once resolution completes
// Cycle guard: track visited (source, target) pairs to break infinite loops
```

### Anti-Patterns to Avoid

- **Calling `graph.add_edge(a, b, ...)` with unknown NodeIndex:** petgraph panics if the index is out of bounds. Always look up via `node_index` HashMap first and silently skip unknown targets (D-13).
- **Recursive barrel chain resolution:** Deep barrel chains (index.ts re-exporting from another index.ts re-exporting...) will stackoverflow. Use an iterative work queue.
- **Running Tarjan's SCC on the full symbol graph for file cycles:** This works but is wasteful — project first to a file-level graph, then run SCC. Symbol-level SCCs capture mutual recursion which D-42 says to ignore.
- **Using `graph.add_edge` for file projection:** Use `graph.update_edge` instead — it prevents duplicate file→file edges, which would inflate SCC component detection.
- **Assuming tsconfig.json is always present:** Many projects (plain JS, non-TS) have no tsconfig. Load gracefully with empty aliases as fallback (D-13).
- **Using `NodeIndex` as a stable identifier across graph mutations:** `NodeIndex` values can be invalidated by node removal. Use the `symbol_id` String as the stable key; only convert to `NodeIndex` for algorithm calls.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Strongly connected components | Custom DFS cycle detection | `petgraph::algo::tarjan_scc` | Tarjan's SCC is O(V+E); hand-roll introduces subtle bugs in SCC ordering and back-edge detection |
| Graph DFS traversal | Manual visited-set + stack | `petgraph::visit::Dfs` | Handles the visited set, start node, and next() iterator correctly; Reversed<> adaptor for incoming |
| Incoming edge counting | Manual iterator over all edges | `graph.neighbors_directed(n, Incoming)` | O(degree) per node; petgraph maintains adjacency lists for both directions |
| Duplicate file-level edges | Manual HashSet dedup | `DiGraph::update_edge` | Creates or updates an existing edge — safe deduplication with O(e) lookup |

**Key insight:** petgraph was specifically designed to avoid reimplementing graph algorithms. The only custom code needed is the barrel resolution logic and confidence scoring, which are domain-specific enough to be hand-written.

---

## Runtime State Inventory

Phase 3 is a new crate addition — no rename, no migration, no stored data. Skipped per instructions.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust / cargo | Build | ✓ | rustc 1.93.1 (2025 edition) | — |
| petgraph 0.8.3 | graph.rs, analysis.rs | ✓ (crates.io) | 0.8.3 | — |
| serde_json 1.0 | resolve.rs tsconfig parsing | ✓ (already in workspace) | 1.0.149 | — |
| cargo test | All tests | ✓ | cargo 1.93.1 | — |

**Missing dependencies with no fallback:** None.

---

## Common Pitfalls

### Pitfall 1: tsconfig.json Contains JSONC (Comments)
**What goes wrong:** `serde_json::from_str` fails with a parse error if tsconfig.json contains comments (`// ...` or `/* */`), which is common in TypeScript projects.
**Why it happens:** JSON5/JSONC comments are not valid JSON. `serde_json` is strict JSON only.
**How to avoid:** Use `serde_json::from_str` in a `Result` match and fall back to empty aliases on parse failure (D-13 — warn and continue). For a more robust solution, strip single-line comments before parsing using a simple pass that removes `// ...` to end-of-line while respecting string literals. The `paths` section that matters for alias resolution virtually never contains comments.
**Warning signs:** Test fails when pointing at a real TypeScript project — check stderr for JSON parse errors.

### Pitfall 2: Barrel Chain Resolution Infinite Loop
**What goes wrong:** Two barrel files re-exporting each other (`a/index.ts` re-exports from `b/index.ts`, which re-exports from `a/index.ts`) causes the iterative resolver to loop forever.
**Why it happens:** Circular barrel re-exports are pathological but syntactically valid TypeScript.
**How to avoid:** Track a `visited: HashSet<(String, String)>` of (source_id, target_id) pairs in the resolution work queue. If a pair is seen twice, drop it and emit a warning.
**Warning signs:** Hang/timeout during resolve pass; the OversizeConnect fixture tests will catch this if fixtures include cross-referencing barrels.

### Pitfall 3: Star Re-Export Expansion Ordering
**What goes wrong:** A `file::*` wildcard expands to all symbols exported from the target file — but those symbols may not yet be in the graph if the target file hasn't been processed.
**Why it happens:** `scan_directory` returns files in filesystem order, which is non-deterministic across platforms.
**How to avoid:** The barrel resolution pass runs after all files are extracted and all nodes are added. The full symbol set is available. Process star expansions in the resolve pass, not during extraction.
**Warning signs:** Intermittent test failures where `file::*` expansion finds zero symbols.

### Pitfall 4: petgraph NodeIndex Invalidation on Removal
**What goes wrong:** If barrel resolution requires removing intermediate nodes or edges and re-adding them, previously cached `NodeIndex` values may be invalidated.
**Why it happens:** `DiGraph::remove_node` shifts indices in petgraph's stable-index graph only if using `StableGraph`; `DiGraph` (not stable) does NOT shift indices, but the slot becomes a "hole" with a bumped generation counter — accessing the old index returns `None` from `node_weight`.
**How to avoid:** Do not remove and re-add barrel nodes during resolution. Instead: (1) keep barrel nodes in the graph but mark them (e.g., `is_barrel: bool` field added to `SymbolNode`), (2) rewrite edges to skip barrels rather than deleting intermediate nodes. The graph consumer (Phase 4 server) can filter barrel-only nodes from the visualization.
**Warning signs:** `node_weight` returns `None` for a NodeIndex you just used; panics in `graph[node_idx]` indexing.

### Pitfall 5: Relative Import Path Normalization
**What goes wrong:** Import paths from the extractor are raw strings like `./hooks`, `../utils/format`, `@/components/Button`. The graph assembler needs to resolve these to canonical file paths relative to the project root to match them against node IDs (which use `file_path::symbol_name` format).
**Why it happens:** Symbol node IDs use the actual filesystem path (`crates/ts-extractor/tests/fixtures/hooks.ts::useToggle`), but import edges reference paths as written in source (`./hooks`).
**How to avoid:** In `resolve.rs`, before looking up an edge target, normalize the path: (1) apply tsconfig alias substitution, (2) resolve relative paths against the source file's directory using `Path::join` + `canonicalize`, (3) map the resulting absolute path to a project-root-relative path for ID matching, (4) try adding `.ts` and `.tsx` and `/index.ts` extensions if the path has no extension (TypeScript's module resolution).
**Warning signs:** Import edges exist in the graph with `target_id` that starts with `raw_path::` — these are unresolved edges that failed to find a node.

### Pitfall 6: Dead Code False Positives for Barrel Re-Exported Symbols
**What goes wrong:** A symbol exported from `hooks.ts` that is re-exported through `index.ts` has zero direct incoming Import edges in the raw graph — all imports come from `index.ts::symbol_name`, not `hooks.ts::symbol_name`. The dead code detector sees zero incoming edges and incorrectly marks it as dead.
**Why it happens:** After barrel resolution, edges point to the true source. But if barrel resolution is incomplete or the symbol is only accessed via barrel re-export (i.e., consumers import from the barrel, not directly), the true source may legitimately have zero direct incoming edges.
**How to avoid:** The barrel resolution pass must add direct edges from all consumers of barrel re-exports to the true defining symbol, replacing the barrel hop. After resolution, a symbol that is barrel-re-exported and consumed will have incoming edges. Test this specifically with the existing `index.ts` / `barrel.ts` / `hooks.ts` fixture chain.
**Warning signs:** Well-known exported hooks showing up as dead code in the OversizeConnect scan.

---

## Code Examples

Verified patterns from official sources:

### In-Degree Check for Dead Code
```rust
// Source: petgraph 0.8.3 docs (https://docs.rs/petgraph/latest/petgraph/stable_graph/type.StableUnGraph.html)
use petgraph::Direction::Incoming;

pub fn has_incoming_edges(graph: &DiGraph<SymbolNode, EdgeKind>, idx: NodeIndex) -> bool {
    graph.neighbors_directed(idx, Incoming).next().is_some()
}
```

### Transitive Dependencies DFS (ANLS-05)
```rust
// Source: petgraph 0.8.3 docs (https://docs.rs/petgraph/latest/petgraph/visit/struct.Dfs.html)
use petgraph::visit::Dfs;

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

### Scan Statistics Timing (INFR-03)
```rust
// Source: Rust stdlib std::time::Instant
// Format matches D-43 / context specifics: "cgraph scan: {files} files, {symbols} symbols, {edges} edges ({time}ms)"
use std::time::Instant;

let start = Instant::now();
let code_graph = indexer.index(&path)?;
let elapsed = start.elapsed();

println!(
    "cgraph scan: {} files, {} symbols, {} edges ({:.0}ms)",
    code_graph.file_count(),
    code_graph.graph.node_count(),
    code_graph.graph.edge_count(),
    elapsed.as_secs_f64() * 1000.0
);
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hand-roll Tarjan's SCC | `petgraph::algo::tarjan_scc` (built-in) | petgraph 0.5+ | No custom graph algorithm code needed |
| `petgraph 0.6` (stable, older API) | `petgraph 0.8.3` (current, 2024 edition APIs) | 2024 | Use `graph.update_edge` for dedup; `StreamingIterator` pattern in 0.8+ |
| Separate file-level and symbol-level graphs | Single symbol graph with derived file view | D-46 decision | Simpler; avoids synchronization bugs between two graphs |

**Deprecated/outdated:**
- `petgraph::algo::connected_components`: For undirected graphs only; don't use for cycle detection. Use `tarjan_scc` on directed graphs.
- `petgraph::visit::DfsPostOrder`: Useful for topological sort but not needed here; `Dfs` is sufficient for reachability.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | tsconfig.json `paths` section rarely contains comments in practice, making serde_json stripping sufficient | Pitfall 1 / Pattern 4 | Parse failures on real codebases; need to add JSONC comment-stripping or use a JSONC parser |
| A2 | Barrel chain resolution should keep barrel nodes in the graph (marked `is_barrel: bool`) rather than removing them | Pitfall 4 | Phase 4 server may need different treatment; adding `is_barrel` to SymbolNode in `cgraph-core` requires a model change |
| A3 | `std::path::Path::canonicalize` is the right approach for resolving relative import paths | Pitfall 5 | `canonicalize` follows symlinks and requires files to exist on disk; fixtures may not be real paths |

**Note on A2:** `SymbolNode` in `cgraph-core/src/model.rs` currently has no `is_barrel` field. This assumption requires a decision: either add it to the model (a core crate change), or track barrel file paths in a `HashSet<String>` inside `CodeGraph` as a side-channel. The side-channel approach avoids a model change and is sufficient for Phase 3.

**Note on A3:** For relative path resolution, consider using `parent().join(relative_path)` on the source file path rather than `canonicalize`, which fails for non-existent paths. Normalize the result with `Path::components()` to handle `..` segments without requiring disk access.

---

## Open Questions (RESOLVED)

1. **Is `SymbolNode` missing an `is_barrel` flag, or should barrel tracking be a side-channel in CodeGraph?**
   - What we know: Pitfall 4 shows that removing barrel nodes is risky. Phase 4 needs to know which nodes to hide in the default file view.
   - What's unclear: Whether `is_barrel` belongs in the core data model (persistent across phases) or is indexer-internal state.
   - Recommendation: Track as `HashSet<String>` (file paths of detected barrel files) in `CodeGraph` for Phase 3. Expose as `CodeGraph::is_barrel_file(path)`. If Phase 4 needs it, promote to `SymbolNode` then.

2. **How should unresolvable import edges be handled in the final graph?**
   - What we know: Edges to third-party library symbols (e.g., `react::useState`) will never resolve to a project node.
   - What's unclear: Should they be dropped, kept as dangling edges, or represented as synthetic "external" nodes?
   - Recommendation: Drop unresolvable edges silently. Dead code analysis only considers intra-project edges anyway. Document in code comments.

3. **What happens when the same symbol is exported from two barrel files (diamond re-export)?**
   - What we know: The barrel chain resolver follows one hop at a time. A diamond pattern creates two resolution paths to the same true source.
   - What's unclear: Whether this creates duplicate edges in the final graph, and whether `petgraph` allows parallel edges on `DiGraph`.
   - Recommendation: `DiGraph` does allow parallel edges (see add_edge docs). Use `update_edge` for the resolved edges to deduplicate. Add a test fixture for this case.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`) |
| Config file | None — using workspace default |
| Quick run command | `cargo test -p cgraph-indexer` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PARS-09 | Barrel chain resolves to true source, no intermediate barrel edges | integration | `cargo test -p cgraph-indexer barrel_chain` | ❌ Wave 0 |
| PARS-09 | Star re-export wildcard expands to all source symbols | integration | `cargo test -p cgraph-indexer star_reexport` | ❌ Wave 0 |
| PARS-10 | tsconfig @/ alias resolves to src/ path | integration | `cargo test -p cgraph-indexer tsconfig_alias` | ❌ Wave 0 |
| PARS-10 | Missing tsconfig returns raw path unchanged (graceful fallback) | unit | `cargo test -p cgraph-indexer no_tsconfig` | ❌ Wave 0 |
| ANLS-01 | Zero-incoming-edge exported symbol is flagged as dead | unit | `cargo test -p cgraph-indexer dead_code_confirmed` | ❌ Wave 0 |
| ANLS-01 | Re-exported symbol not falsely flagged as dead | integration | `cargo test -p cgraph-indexer dead_code_barrel_reexport` | ❌ Wave 0 |
| ANLS-01 | Entry point file symbols not flagged | unit | `cargo test -p cgraph-indexer entry_point_exclusion` | ❌ Wave 0 |
| ANLS-02 | Confirmed tier: exported, zero edges, not barrel | unit | `cargo test -p cgraph-indexer confidence_confirmed` | ❌ Wave 0 |
| ANLS-02 | Suspicious tier: string literal mention demotes confidence | unit | `cargo test -p cgraph-indexer confidence_suspicious_string` | ❌ Wave 0 |
| ANLS-02 | Suspicious tier: namespace import in same project demotes confidence | unit | `cargo test -p cgraph-indexer confidence_suspicious_namespace` | ❌ Wave 0 |
| ANLS-03 | File-level cycle A→B→A detected and enumerated | unit | `cargo test -p cgraph-indexer cycle_detection` | ❌ Wave 0 |
| ANLS-03 | No false cycles (A→B→C is not a cycle) | unit | `cargo test -p cgraph-indexer no_false_cycles` | ❌ Wave 0 |
| ANLS-04 | blast_radius returns all transitive dependents | unit | `cargo test -p cgraph-indexer blast_radius` | ❌ Wave 0 |
| ANLS-05 | transitive_deps returns all transitive dependencies | unit | `cargo test -p cgraph-indexer transitive_deps` | ❌ Wave 0 |
| INFR-03 | Default output prints files/symbols/edges/elapsed | integration | `cargo test -p cg cli_scan_stats` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p cgraph-indexer`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/indexer/` crate — does not exist yet; Wave 0 creates the scaffold
- [ ] `crates/indexer/src/lib.rs` — public API
- [ ] `crates/indexer/src/crawl.rs` — file dispatch
- [ ] `crates/indexer/src/graph.rs` — CodeGraph struct
- [ ] `crates/indexer/src/resolve.rs` — barrel + alias resolution
- [ ] `crates/indexer/src/analysis.rs` — dead code, blast radius, cycles
- [ ] `crates/indexer/tests/` — integration tests with multi-file fixtures
- [ ] `crates/indexer/tests/fixtures/` — multi-file fixture set (barrel chain, diamond re-export, tsconfig.json)
- [ ] `Cargo.toml` workspace members — add `crates/indexer`
- [ ] `crates/cli/Cargo.toml` — add `cgraph-indexer` dependency

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (CLI tool, no auth) |
| V3 Session Management | no | — (stateless analysis) |
| V4 Access Control | yes (partial) | Path traversal prevention — scan_directory already skips hidden dirs/node_modules; indexer must not escape project root |
| V5 Input Validation | yes | File path validation already in CLI (T-01-05 pattern: check exists + is_dir before scanning) |
| V6 Cryptography | no | — (no crypto) |

### Known Threat Patterns for Rust CLI File Analysis

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via symlink | Elevation of Privilege | `scan_directory` uses `follow_links(false)` already — verified in detect.rs |
| tsconfig.json path alias pointing outside project root | Elevation of Privilege | After alias substitution, canonicalize and verify the resolved path starts with the project root prefix |
| Maliciously crafted source file causing OOM | Denial of Service | tree-sitter is bounded by file size; large files are parse errors, not panics; D-13 continues on error |
| Barrel resolution cycle causing infinite loop | Denial of Service | Cycle guard in resolution work queue (visited set) — see Pitfall 2 |

---

## Sources

### Primary (HIGH confidence)
- `/websites/rs_petgraph` (Context7) — DiGraph, add_node, add_edge, update_edge, Dfs, Reversed, tarjan_scc API signatures and examples
- `crates/core/src/model.rs` — SymbolNode, EdgeKind, exact field names
- `crates/core/src/extractor.rs` — Extractor trait, ExtractionResult
- `crates/core/src/detect.rs` — scan_directory implementation, skip patterns
- `crates/ts-extractor/src/edges.rs` — Edge ID format (`file::symbol`, `raw_path::name`, `file::*`, `unresolved::name`)
- `crates/cli/src/main.rs` — Current CLI structure, Cli struct, clap usage

### Secondary (MEDIUM confidence)
- TypeScript tsconfig `paths` format — [CITED: https://www.typescriptlang.org/tsconfig/#paths]
- `crates/indexer` module layout — derived from D-47 decision in CONTEXT.md

### Tertiary (LOW confidence)
- Assumption A1 (tsconfig comments rarity in paths section) — based on typical project conventions, not verified against OversizeConnect

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — petgraph 0.8.3 verified via cargo search; all algorithms confirmed in Context7 docs
- Architecture: HIGH — all design decisions locked in CONTEXT.md (D-40 through D-48); module layout is straight from D-47
- Pitfalls: HIGH — derived from reading actual extractor code (edge format) and petgraph API docs; A2/A3 flagged as assumptions
- Test map: HIGH — all requirement IDs from REQUIREMENTS.md mapped to specific test names

**Research date:** 2026-05-02
**Valid until:** 2026-06-02 (petgraph 0.8.x is stable; tsconfig format is stable)
