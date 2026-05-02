---
phase: 03-indexer-analysis-pipeline
plan: 01
subsystem: indexer
tags: [graph, petgraph, crawl, extractor-dispatch, scaffold]
dependency_graph:
  requires: [cgraph-core, cgraph-ts-extractor]
  provides: [cgraph-indexer, CodeGraph, Indexer, IndexerError]
  affects: [crates/indexer/, Cargo.toml]
tech_stack:
  added: [petgraph 0.8.3]
  patterns: [DiGraph wrapper with HashMap index, dynamic extractor registry, two-phase graph assembly]
key_files:
  created:
    - crates/indexer/Cargo.toml
    - crates/indexer/src/lib.rs
    - crates/indexer/src/graph.rs
    - crates/indexer/src/crawl.rs
    - crates/indexer/src/resolve.rs
    - crates/indexer/src/analysis.rs
  modified:
    - Cargo.toml
    - Cargo.lock
decisions:
  - "CodeGraph wraps DiGraph<SymbolNode, EdgeKind> with HashMap<String, NodeIndex> for O(1) lookup (D-45)"
  - "Barrel file tracking uses HashSet<String> side-channel in CodeGraph rather than SymbolNode field (RESEARCH.md A2)"
  - "Two-phase graph assembly: all symbols inserted first, then edges added to maximize resolution"
  - "Dynamic extractor registry via Vec<Box<dyn Extractor>> keeps indexer language-agnostic (D-48)"
metrics:
  duration: "198s"
  completed: "2026-05-02T19:30:47Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 8
  tests_total_workspace: 33
---

# Phase 3 Plan 1: Indexer Crate Scaffold Summary

Indexer crate with petgraph DiGraph wrapper (CodeGraph) and file crawl + extractor dispatch loop (Indexer), enabling directory-to-graph assembly with dynamic language support.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Create indexer crate scaffold with CodeGraph struct | ef00a2f | Cargo.toml, crates/indexer/Cargo.toml, graph.rs, lib.rs |
| 2 | Implement Indexer with file crawl and extractor dispatch | a030802 | crawl.rs, lib.rs |

## What Was Built

### CodeGraph (graph.rs)
- `DiGraph<SymbolNode, EdgeKind>` wrapped with `HashMap<String, NodeIndex>` for O(1) symbol lookup
- `add_symbol()` inserts node and updates index map
- `add_edge()` validates both source and target via HashMap before calling petgraph (prevents panics on unknown NodeIndex, T-03-03 mitigation)
- `barrel_files: HashSet<String>` side-channel for barrel file tracking
- `file_count()` derives unique file count from node weights

### Indexer (crawl.rs)
- Dynamic extractor registry: `Vec<Box<dyn Extractor>>` passed at construction
- `index(project_root)` method: scan_directory -> read files -> dispatch to first matching extractor -> assemble CodeGraph
- Two-phase assembly: Phase 1 extracts all symbols, Phase 2 adds edges (maximizes target resolution)
- Graceful error handling: file read failures and parse errors warn to stderr and continue (D-13, D-14)
- `IndexerError` type with thiserror for typed error propagation

### Crate Structure
- `lib.rs` re-exports CodeGraph, Indexer, IndexerError
- `resolve.rs` placeholder for Plan 02 (barrel chain + tsconfig alias resolution)
- `analysis.rs` placeholder for Plan 03 (dead code, blast radius, cycles)

## Test Results

8 new tests added, all passing:
- `test_add_symbol_and_lookup` -- symbol insertion and HashMap lookup
- `test_add_edge_known_nodes` -- edge between two known symbols
- `test_add_edge_unknown_target` -- edge to unknown target silently skipped (no panic)
- `test_file_count` -- unique file counting across symbol nodes
- `test_barrel_file_tracking` -- barrel file HashSet operations
- `test_index_single_file` -- end-to-end: single .ts file -> CodeGraph with nodes
- `test_index_empty_dir` -- empty directory produces empty graph
- `test_index_syntax_errors_continue` -- partial parse files still contribute symbols

Full workspace: 33 tests passing (25 existing + 8 new), zero regressions.

## Deviations from Plan

None -- plan executed exactly as written.

## Known Stubs

| File | Line | Description | Resolved By |
|------|------|-------------|-------------|
| crates/indexer/src/resolve.rs | 1 | Placeholder module | Plan 03-02 |
| crates/indexer/src/analysis.rs | 1 | Placeholder module | Plan 03-03 |

These are intentional scaffolding stubs that do not prevent this plan's goals. Both modules are declared in lib.rs and compile as empty modules.

## Verification

- `cargo test -p cgraph-indexer` -- 8 tests pass
- `cargo test --workspace` -- 33 tests pass
- `cargo build --workspace` -- compiles with zero errors

## Self-Check: PASSED

All files exist, both commits verified, all acceptance criteria met (22/22 checks pass).
