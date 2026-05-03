---
phase: 03-indexer-analysis-pipeline
verified: 2026-05-02T23:55:00Z
status: passed
score: 8/8
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 8/8
  gaps_closed: []
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Run CLI against a real TypeScript project with inter-file imports, barrel re-exports, and known dead exports"
    expected: "Edge count is non-zero and proportional to project size; dead code report identifies known dead exports; barrel-re-exported symbols are NOT falsely flagged"
    why_human: "Unit tests verify each algorithm independently with manually-constructed graphs. Integration tests use small fixture dirs (7 files). Real-world validation on a production codebase confirms resolution quality at scale."
  - test: "Verify --dead-code accuracy on a project with known dead and alive exports"
    expected: "Confirmed dead code entries match known dead exports. No false positives for barrel-re-exported, entry-point, or test file symbols."
    why_human: "Requires domain knowledge of which exports are actually dead in the target project. Cannot be automated without ground truth annotations."
---

# Phase 3: Indexer & Analysis Pipeline Verification Report

**Phase Goal:** The indexer crawls a project directory, feeds all files through the extractor registry, assembles the full graph in memory, and runs analysis algorithms so dead code, blast radius, and circular dependencies are available as queryable data -- all without any browser or server.
**Verified:** 2026-05-02T21:30:00Z
**Status:** passed
**Re-verification:** Yes -- re-verification after gap closure (Plans 03-05, 03-06). Human UAT passed: tested against 4 real codebases (nighthawk, agentcommercekit, signum-api, OversizeConnect). Edge ratios healthy (75-287%). Dead code spot-checks confirmed accurate.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | After scanning a project, the CLI prints a summary line: files scanned, symbols found, edges found, and elapsed time | VERIFIED | `cargo run -- crates/ts-extractor/tests/fixtures/` outputs `cgraph scan: 7 files, 21 symbols, 8 edges (10ms)`. Format matches INFR-03 spec. CLI smoke test `scan_fixture_directory` passes. |
| 2 | Exported symbols with zero incoming edges are flagged as dead code; symbols re-exported through barrels are not falsely flagged | VERIFIED | `dead_code()` in analysis.rs (line 123) checks `is_exported`, `neighbors_directed(Incoming)`, barrel exclusion via `is_barrel_file()`. Tests: `test_dead_code_confirmed`, `test_dead_code_barrel_exclusion` -- both pass (49 indexer tests total, 0 failures). |
| 3 | Dead code results include a confidence level (confirmed vs. suspicious) rather than a binary flag | VERIFIED | `Confidence` enum (line 12) with `Confirmed` and `Suspicious(String)`. `DeadCodeResult` (line 32) has separate `confirmed` and `suspicious` Vecs. Test `test_dead_code_suspicious_unresolved_call` verifies demotion heuristic. |
| 4 | Given any symbol ID, the indexer returns the complete set of its transitive dependents (blast radius) | VERIFIED | `blast_radius()` in analysis.rs (line 259) uses `Reversed(&graph.graph)` with `Dfs`. Tests: `test_blast_radius_simple` (linear chain), `test_blast_radius_diamond` (diamond pattern), `test_blast_radius_unknown_symbol` (empty Vec for missing IDs). All pass. |
| 5 | Given any symbol ID, the indexer returns the complete set of things it transitively depends on | VERIFIED | `transitive_deps()` in analysis.rs (line 279) uses forward `Dfs`. Tests: `test_transitive_deps_simple`, `test_transitive_deps_unknown_symbol`. Both pass. |
| 6 | Circular dependency chains between modules are detected and enumerable | VERIFIED | `detect_cycles()` in analysis.rs (line 298) builds file-level projection with `update_edge` dedup, runs `tarjan_scc`, filters SCCs > 1. Tests: `test_cycle_detection_simple`, `test_cycle_detection_triangle`, `test_no_false_cycles`, `test_cycle_ignores_self_file_edges`. All pass. |
| 7 | Re-export chains through barrel files are resolved to the true defining file -- the graph contains no intermediate barrel-only edges (PARS-09) | VERIFIED | `resolve_edges()` in resolve.rs (line 412) implements 3-pass resolution: path resolution, barrel hop-following with 20-hop limit and HashSet cycle guard, ReExport edge removal. Integration test `test_barrel_chain_integration` verifies end-to-end: consumer->index->hooks resolves to consumer->hooks, index.ts marked as barrel, no ReExport edges remain. |
| 8 | Import paths using tsconfig path aliases are resolved to the actual file path relative to the project root (PARS-10) | VERIFIED | `TsConfigAliases::load()` in resolve.rs (line 23) parses tsconfig.json with JSONC comment stripping, follows `extends` chains with cycle guard (line 35: `load_tsconfig_from_path`), extracts `compilerOptions.paths` and `baseUrl`. `resolve_candidates()` (line 125) handles multi-target and baseUrl resolution. Integration tests `test_tsconfig_alias_integration` and `test_tsconfig_extends_baseurl_integration` verify end-to-end. |

**Score:** 8/8 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/indexer/Cargo.toml` | Crate manifest with petgraph, serde_json, thiserror, cgraph-core deps | VERIFIED | Contains `name = "cgraph-indexer"`, `petgraph = "0.8.3"`, `serde_json = "1.0"`, `thiserror = "2.0"`, `cgraph-core = { path = "../core" }`. 14 lines. |
| `crates/indexer/src/lib.rs` | Public module declarations and re-exports | VERIFIED | Re-exports `CodeGraph`, `Indexer`, `IndexerError`, `DeadCodeResult`, `DeadCodeEntry`, `Confidence`, `CycleResult`, `blast_radius`, `transitive_deps`, `detect_cycles`, `dead_code`. 8 lines. |
| `crates/indexer/src/graph.rs` | CodeGraph struct wrapping DiGraph with HashMap index | VERIFIED | `pub struct CodeGraph` with `DiGraph<SymbolNode, EdgeKind>`, `HashMap<String, NodeIndex>`, `HashSet<String>` barrel tracking. 156 lines, 5 unit tests. |
| `crates/indexer/src/crawl.rs` | Indexer struct with dynamic extractor registry and index() method | VERIFIED | `pub struct Indexer` with `extractors: Vec<Box<dyn Extractor>>`. `index()` method: scan_directory, extract, resolve_edges, resolve_unresolved_edges, remap pseudo-nodes, add_edge. 509 lines, 7 tests (3 unit + 4 integration). |
| `crates/indexer/src/resolve.rs` | TsConfigAliases loader, barrel chain resolution, path normalization, unresolved edge resolution | VERIFIED | `TsConfigAliases` with extends chain loading + JSONC stripping + baseUrl + multi-target. `resolve_edges()` with 3-pass resolution. `resolve_unresolved_edges()` with name-based symbol matching + import context disambiguation. 1057 lines, 20+ unit tests. |
| `crates/indexer/src/analysis.rs` | Dead code detection, blast radius, transitive deps, cycle detection | VERIFIED | All 4 public functions + result types (`Confidence`, `DeadCodeEntry`, `DeadCodeResult`, `CycleResult`). 615 lines, 17 unit tests. |
| `crates/cli/src/main.rs` | CLI with indexer integration, scan stats, --dead-code and --cycles flags | VERIFIED | `Cli` struct with `dead_code: bool` and `cycles: bool`. Indexer construction, timing, dead_code(), detect_cycles() calls. Print functions for reports. 158 lines. |
| `crates/cli/Cargo.toml` | CLI crate with cgraph-indexer and cgraph-ts-extractor dependencies | VERIFIED | Contains `cgraph-indexer = { path = "../indexer" }` and `cgraph-ts-extractor = { path = "../ts-extractor" }`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crawl.rs` | `cgraph_core::scan_directory` | Function call for file discovery | WIRED | Line 51: `let detection = scan_directory(project_root)?;` |
| `crawl.rs` | `cgraph_core::Extractor` | Trait object dispatch | WIRED | Line 33: `extractors: Vec<Box<dyn Extractor>>`, Line 67: `ext.can_handle(path)`, Line 73: `extractor.extract(path, &source)` |
| `graph.rs` | `petgraph::graph::DiGraph` | Wrapped ownership | WIRED | Line 9: `pub graph: DiGraph<SymbolNode, EdgeKind>` |
| `resolve.rs` | `graph.rs` | CodeGraph mutation | WIRED | Line 412: `resolve_edges(edges: &mut Vec<SymbolEdge>, graph: &mut CodeGraph, ...)` mutates barrel_files |
| `crawl.rs` | `resolve.rs` | Resolution passes called after extraction | WIRED | Lines 106-111: `TsConfigAliases::load(project_root)` + `resolve_edges(...)` + `resolve_unresolved_edges(...)` |
| `analysis.rs` | `graph.rs` | Immutable borrow of CodeGraph | WIRED | All functions take `&CodeGraph`: `dead_code(graph: &CodeGraph, ...)`, `blast_radius(graph: &CodeGraph, ...)`, etc. |
| `analysis.rs` | `petgraph::algo::tarjan_scc` | SCC algorithm | WIRED | Line 326: `let cycles = tarjan_scc(&file_graph)` |
| `analysis.rs` | `petgraph::visit::Dfs` | DFS traversal | WIRED | Line 264: `let mut dfs = Dfs::new(reversed, start);` and Line 283: `let mut dfs = Dfs::new(&graph.graph, start);` |
| `main.rs` | `cgraph_indexer::Indexer` | Constructs and calls index() | WIRED | Line 46: `let indexer = Indexer::new(extractors);`, Line 48: `let code_graph = indexer.index(&cli.path)?;` |
| `main.rs` | `cgraph_indexer::analysis` | Calls dead_code(), detect_cycles() | WIRED | Line 61: `let dead_result = dead_code(&code_graph, &cli.path);`, Line 62: `let cycle_result = detect_cycles(&code_graph);` |
| `main.rs` | `cgraph_ts_extractor::TsExtractor` | Registered in extractor vec | WIRED | Line 42: `Box::new(TsExtractor::new())` in extractor registry Vec |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| CLI prints scan stats with non-zero edges | `cargo run -- crates/ts-extractor/tests/fixtures/` | `cgraph scan: 7 files, 21 symbols, 8 edges (10ms)` | PASS |
| CLI prints analysis summary | (same command) | `analysis:\n  dead code: 0 confirmed, 0 suspicious\n  circular dependencies: 0` | PASS |
| --dead-code flag works | `cargo run -- crates/ts-extractor/tests/fixtures/ --dead-code` | `dead code: none found` | PASS |
| --cycles flag works | `cargo run -- crates/ts-extractor/tests/fixtures/ --cycles` | `circular dependencies: none found` | PASS |
| All workspace tests pass | `cargo test --workspace` | 109 tests pass, 0 failures | PASS |
| Indexer-specific tests pass | `cargo test -p cgraph-indexer` | 49 tests pass, 0 failures | PASS |
| CLI smoke tests pass | `cargo test -p cg` | 8 tests pass, 0 failures | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| PARS-09 | 03-02, 03-05 | Tool resolves multi-hop barrel re-export chains to find the true source | SATISFIED | `resolve_edges()` in resolve.rs implements barrel chain resolution with hop-following, star expansion, cycle guard. Integration test `test_barrel_chain_integration` passes. |
| PARS-10 | 03-02, 03-06 | Tool resolves TypeScript path aliases (tsconfig paths) | SATISFIED | `TsConfigAliases::load()` + `resolve()`/`resolve_candidates()` in resolve.rs. JSONC stripping. extends chains. baseUrl. Multi-target. Integration tests pass. |
| ANLS-01 | 03-03 | Tool identifies dead code (exported symbols with zero incoming edges) | SATISFIED | `dead_code()` in analysis.rs checks `is_exported` + `neighbors_directed(Incoming)`. Entry point, barrel, non-exported exclusions. 8 unit tests pass. |
| ANLS-02 | 03-03 | Dead code detection uses confidence scoring (suspicious vs confirmed dead) | SATISFIED | `Confidence` enum with `Confirmed`/`Suspicious(String)`. `DeadCodeResult` with separate vectors. Demotion heuristics. |
| ANLS-03 | 03-03 | Tool detects circular dependencies between modules | SATISFIED | `detect_cycles()` projects symbol graph to file-level, runs `tarjan_scc`. 4 tests pass. |
| ANLS-04 | 03-03 | Tool computes transitive dependents for any symbol (blast radius) | SATISFIED | `blast_radius()` uses `Reversed` DFS. 3 tests pass. |
| ANLS-05 | 03-03 | Tool computes transitive dependencies for any symbol (what it uses) | SATISFIED | `transitive_deps()` uses forward DFS. 2 tests pass. |
| INFR-03 | 03-04 | Tool displays scan statistics after parsing (files, symbols, edges, time) | SATISFIED | CLI prints `cgraph scan: {N} files, {N} symbols, {N} edges ({N}ms)` with `Instant`-based timing. CLI smoke test `scan_fixture_directory` confirms format. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | - | - | - | No TODO, FIXME, placeholder, stub, or debug output found in any Phase 3 source files. |

### Human Verification Required

### 1. End-to-end on a real TypeScript project

**Test:** Run `cg <path-to-real-ts-project>` against a TypeScript project with known inter-file imports, barrel re-exports, and dead exports (e.g., OversizeConnect or any medium-sized TS project with index.ts barrels).
**Expected:** Edge count is proportional to project complexity (not near-zero). Dead code report identifies known dead exports. Barrel-re-exported symbols are NOT falsely flagged. The edge ratio (edges/symbols) should be reasonable (>20% for a project with active imports).
**Why human:** Integration tests use small fixture directories (7 files, 8 edges). Real-world validation on a production codebase confirms the resolution pipeline (tsconfig aliases, barrel chains, unresolved edge matching) produces correct results at scale. This cannot be automated without a known-good reference project with ground truth annotations.

### 2. Dead code accuracy on real codebase

**Test:** Run `cg <path> --dead-code` on a project where you know which exports are dead and which are used.
**Expected:** Confirmed dead code entries match known dead exports. No false positives for barrel-re-exported, entry-point, or test file symbols.
**Why human:** Requires domain knowledge of which exports are actually dead in the target project to validate accuracy.

### Gaps Summary

No blocking gaps found. All 8 ROADMAP success criteria verified with code evidence and passing tests. All 8 requirement IDs (PARS-09, PARS-10, ANLS-01-05, INFR-03) are satisfied with implementation evidence.

**Improvement since previous verification:** The gap closure plans (03-05, 03-06) resolved the "0 edges on fixture directory" issue. The CLI now produces 8 edges when scanning the 7-file fixture directory (previously 0). This demonstrates that Call and TypeRef edges are being resolved to real graph nodes via `resolve_unresolved_edges()`. The tsconfig extends chains and baseUrl resolution also improve real-world alias handling.

The remaining human verification items confirm real-world accuracy at scale -- all programmatically-verifiable criteria are met.

---

_Verified: 2026-05-02T21:30:00Z_
_Verifier: Claude (gsd-verifier)_
