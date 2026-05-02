---
phase: 03-indexer-analysis-pipeline
plan: 02
subsystem: indexer-resolve
tags: [barrel-resolution, tsconfig-aliases, path-normalization, cycle-guard]
dependency_graph:
  requires: [cgraph-indexer, CodeGraph, cgraph-core]
  provides: [TsConfigAliases, resolve_edges, normalize_import_path, resolve_file_path]
  affects: [crates/indexer/src/resolve.rs, crates/indexer/src/crawl.rs]
tech_stack:
  added: []
  patterns: [iterative barrel hop-following with visited set, JSONC comment stripping, component-based path normalization]
key_files:
  created: []
  modified:
    - crates/indexer/src/resolve.rs
    - crates/indexer/src/crawl.rs
decisions:
  - "JSONC comment stripping before serde_json parse handles tsconfig with comments (Pitfall 1)"
  - "Path normalization uses Path::components() iteration instead of canonicalize (RESEARCH.md A3)"
  - "Import edges with source_id file::<import> are not SymbolNodes so get silently dropped by add_edge; resolution still works on the edge vec before insertion"
  - "Star re-export expansion uses graph node iteration to find all exported symbols in target file"
metrics:
  duration: "367s"
  completed: "2026-05-02T19:41:16Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 10
  tests_total_workspace: 77
---

# Phase 3 Plan 2: Barrel Chain Resolution and tsconfig Alias Resolution Summary

Barrel re-export chain resolution with cycle guard, tsconfig path alias loading with JSONC support, and path normalization -- wired into the Indexer::index() pipeline between extraction and edge insertion.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Implement resolve.rs -- tsconfig alias loading, path normalization, barrel chain resolution | f195874 | crates/indexer/src/resolve.rs |
| 2 | Wire resolution pass into Indexer::index() flow | 7a30d73 | crates/indexer/src/crawl.rs |

## What Was Built

### TsConfigAliases (resolve.rs)
- `TsConfigAliases::load(project_root)` reads tsconfig.json, strips JSONC comments, extracts `compilerOptions.paths` aliases
- `TsConfigAliases::resolve(raw_path)` substitutes alias prefixes (e.g., `@/components/Button` -> `src/components/Button`)
- Graceful fallback: missing or unparseable tsconfig returns empty aliases (D-13, T-03-07)

### Path Normalization (resolve.rs)
- `normalize_import_path()` resolves relative paths (`./`, `../`) using `Path::components()` iteration -- no `canonicalize` (no disk access required)
- `resolve_file_path()` chains alias resolution -> path normalization -> project root verification (T-03-04: path escape detection)
- Extension resolution: tries `.ts`, `.tsx`, `/index.ts`, `/index.tsx` when no extension present

### Barrel Chain Resolution (resolve.rs)
- `resolve_edges()` performs three-pass resolution:
  - Pass A: Resolve raw import paths to canonical file paths (alias substitution + normalization)
  - Pass B: Build ReExport hop map, expand star wildcards to individual symbols, follow barrel chains iteratively with `HashSet` visited set and 20-hop safety bound (T-03-05)
  - Pass C: Remove all ReExport edges (folded into Import edges)
- Star re-export expansion finds all `is_exported` symbols in the target file
- Cycle guard prevents infinite loops on circular barrel re-exports

### Integration (crawl.rs)
- Resolution pass inserted between extraction (Phase 1) and edge insertion (Phase 2) in `Indexer::index()`
- Runs after all symbols are in the graph so star expansion can find all exported symbols (Pitfall 3)

## Test Results

10 new tests added, all passing:

Unit tests (resolve.rs):
- `test_tsconfig_alias_resolve` -- alias prefix substitution
- `test_tsconfig_no_match` -- unmatched paths returned unchanged
- `test_tsconfig_load_missing_file` -- missing tsconfig returns empty aliases gracefully
- `test_normalize_relative_path` -- `../utils/format` resolves correctly from nested source
- `test_normalize_parent_dir_segments` -- multiple `..` segments handled without canonicalize
- `test_barrel_chain_single_hop` -- consumer->index->hooks resolves to consumer->hooks
- `test_barrel_chain_star_expansion` -- `::*` wildcard expands to individual symbol entries
- `test_barrel_chain_cycle_guard` -- circular A->B->A terminates without panic

Integration tests (crawl.rs):
- `test_barrel_chain_integration` -- end-to-end: 3-file barrel chain resolves correctly, barrel marked, no ReExport edges remain
- `test_tsconfig_alias_integration` -- end-to-end: @/utils alias resolves to src/utils.ts

Full workspace: 77 tests passing (33 previous + 10 new resolve/integration + 34 existing), zero regressions.

## Deviations from Plan

None -- plan executed exactly as written.

## Verification

- `cargo test -p cgraph-indexer -- resolve::` -- 8 resolve unit tests pass
- `cargo test -p cgraph-indexer -- barrel_chain_integration` -- passes
- `cargo test -p cgraph-indexer -- tsconfig_alias_integration` -- passes
- `cargo test --workspace` -- 77 tests pass, zero regressions

## Self-Check: PASSED

All files verified, both commits confirmed in git log.
