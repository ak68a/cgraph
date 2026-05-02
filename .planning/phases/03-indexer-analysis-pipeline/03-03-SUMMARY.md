---
phase: 03-indexer-analysis-pipeline
plan: 03
subsystem: indexer-analysis
tags: [analysis, dead-code, blast-radius, transitive-deps, cycle-detection, petgraph]
dependency_graph:
  requires: [cgraph-indexer (Plan 01), CodeGraph, petgraph]
  provides: [dead_code, blast_radius, transitive_deps, detect_cycles, DeadCodeResult, DeadCodeEntry, Confidence, CycleResult]
  affects: [crates/indexer/src/analysis.rs, crates/indexer/src/lib.rs]
tech_stack:
  added: []
  patterns: [Reversed DFS for blast radius, Tarjan SCC for file-level cycles, file graph projection with update_edge dedup, two-tier confidence scoring]
key_files:
  created: []
  modified:
    - crates/indexer/src/analysis.rs
    - crates/indexer/src/lib.rs
decisions:
  - "Entry point detection uses filename convention (D-40): main.ts/index.ts at root, App.tsx, test dirs, setup/config stems"
  - "Dead code heuristic demotion checks unresolved call targets and namespace imports for suspicious tier (D-41)"
  - "File-level cycle detection projects symbol graph to file pairs using update_edge dedup, then runs tarjan_scc (D-42, D-46)"
  - "Same-file edges excluded from cycle detection file projection (intra-file calls are not circular deps)"
metrics:
  duration: "256s"
  completed: "2026-05-02T19:39:09Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 17
  tests_total_workspace: 84
---

# Phase 3 Plan 3: Analysis Algorithms Summary

Dead code detection with two-tier confidence scoring (confirmed/suspicious), blast radius via reversed DFS, transitive deps via forward DFS, and file-level cycle detection via Tarjan's SCC on projected graph -- all operating on immutable CodeGraph references.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Dead code detection with two-tier confidence (ANLS-01, ANLS-02) | 7c02e19 | analysis.rs, lib.rs |
| 2 | Blast radius, transitive deps, cycle detection (ANLS-03, ANLS-04, ANLS-05) | cf7e27e | analysis.rs, lib.rs |

## What Was Built

### Dead Code Detection (ANLS-01, ANLS-02)

- `dead_code(graph: &CodeGraph, project_root: &Path) -> DeadCodeResult`
- Two-tier confidence model:
  - **Confirmed**: exported symbol, zero incoming edges, not entry point, not barrel file
  - **Suspicious**: zero edges but demoted by heuristic (unresolved call target match, namespace import access)
- Entry point exclusion (D-40): main.ts/index.ts at project root, App.tsx/App.ts, test directories, setup/config files
- Barrel file exclusion: symbols in files marked via `graph.mark_barrel_file()` are never flagged
- Non-exported symbols never flagged (only exported symbols are candidates)
- Results sorted by file_path for deterministic output

### Blast Radius (ANLS-04)

- `blast_radius(graph: &CodeGraph, symbol_id: &str) -> Vec<String>`
- Uses `Reversed(&graph.graph)` with DFS to walk edges backwards
- Returns all symbol IDs that transitively depend on the given symbol
- Returns empty Vec for unknown symbol IDs (no panic)

### Transitive Dependencies (ANLS-05)

- `transitive_deps(graph: &CodeGraph, symbol_id: &str) -> Vec<String>`
- Uses forward DFS on `&graph.graph`
- Returns all symbol IDs that the given symbol transitively depends on
- Returns empty Vec for unknown symbol IDs (no panic)

### Cycle Detection (ANLS-03)

- `detect_cycles(graph: &CodeGraph) -> CycleResult`
- File-level only (D-42): projects symbol graph to file-level graph on demand (D-46)
- Uses `update_edge` for deduplication of file-level edges
- Excludes same-file edges (intra-file calls are not circular dependencies)
- Runs `tarjan_scc` on file projection, filters SCCs with size > 1

### Result Types

- `Confidence` enum: `Confirmed`, `Suspicious(String)` with reason
- `DeadCodeEntry`: symbol_id, file_path, symbol_name, kind, line_start, line_end, confidence
- `DeadCodeResult`: confirmed and suspicious Vec fields
- `CycleResult`: cycles as Vec of Vec of file path strings

### Re-exports

All analysis types and functions re-exported from `crates/indexer/src/lib.rs`: DeadCodeResult, DeadCodeEntry, Confidence, CycleResult, blast_radius, transitive_deps, detect_cycles.

## Test Results

17 new tests added, all passing:

**Dead code (8 tests):**
- `test_dead_code_confirmed` -- exported symbol with zero incoming edges flagged as confirmed dead
- `test_dead_code_not_flagged_with_incoming` -- symbol with incoming edge is NOT dead
- `test_dead_code_entry_point_exclusion` -- main.ts at root excluded
- `test_dead_code_app_tsx_exclusion` -- App.tsx excluded
- `test_dead_code_test_file_exclusion` -- __tests__ directory excluded
- `test_dead_code_barrel_exclusion` -- barrel file excluded
- `test_dead_code_non_exported_not_flagged` -- non-exported symbol excluded
- `test_dead_code_suspicious_unresolved_call` -- unresolved call target demotes to suspicious

**Blast radius (3 tests):**
- `test_blast_radius_simple` -- linear chain A->B->C, blast of C includes A and B
- `test_blast_radius_unknown_symbol` -- returns empty Vec
- `test_blast_radius_diamond` -- diamond pattern A->B->D, A->C->D, blast of D includes A, B, C

**Transitive deps (2 tests):**
- `test_transitive_deps_simple` -- linear chain A->B->C, deps of A includes B and C
- `test_transitive_deps_unknown_symbol` -- returns empty Vec

**Cycle detection (4 tests):**
- `test_cycle_detection_simple` -- A->B->A detected as cycle
- `test_no_false_cycles` -- linear chain A->B->C has no cycles
- `test_cycle_detection_triangle` -- A->B->C->A detected as 3-node cycle
- `test_cycle_ignores_self_file_edges` -- same-file edges produce no cycles

Full workspace: 84 tests passing (67 existing + 17 new), zero regressions.

## Deviations from Plan

None -- plan executed exactly as written.

## Verification

- `cargo test -p cgraph-indexer -- analysis` -- 17 tests pass
- `cargo test --workspace` -- 84 tests pass, zero regressions
- `cargo build --workspace` -- compiles with zero errors
- `analysis.rs` is 615 lines (well above min_lines: 100 requirement)

## Self-Check: PASSED

All files exist, both commits verified (7c02e19, cf7e27e), all 19 acceptance criteria met. All analysis types and functions correctly re-exported from lib.rs.
