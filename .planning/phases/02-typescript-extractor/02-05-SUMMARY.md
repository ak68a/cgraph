---
phase: 02-typescript-extractor
plan: "05"
subsystem: ts-extractor
tags:
  - anti-pattern-fix
  - dead-code
  - namespace-reexport
  - overload-dedup
  - zero-warnings
dependency_graph:
  requires:
    - 02-01 (TsExtractor struct foundation)
    - 02-02 (symbol extraction)
    - 02-03 (edge extraction)
    - 02-04 (test fixtures and integration tests)
  provides:
    - Clean zero-warning build for cgraph-ts-extractor
    - Correct namespace re-export edge emission
    - Deduplicated overloaded function symbols
  affects:
    - Phase 3 indexer (consumes extractor output — incorrect edges and duplicate nodes now fixed)
tech_stack:
  added: []
  patterns:
    - HashSet dedup for idempotent symbol collection
    - Tree-sitter child node kind inspection for AST shape discrimination
key_files:
  created: []
  modified:
    - crates/ts-extractor/src/lib.rs
    - crates/ts-extractor/src/edges.rs
    - crates/ts-extractor/src/symbols.rs
    - crates/ts-extractor/tests/extraction_test.rs
decisions:
  - "Namespace re-export uses existing EdgeKind::ReExport with namespace name in source_id (not a new variant)"
  - "Overload dedup retains first occurrence (broadest line span for overloads)"
metrics:
  duration: "~15 minutes"
  completed_date: "2026-05-02T17:22:08Z"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 4
---

# Phase 2 Plan 05: Anti-Pattern Fixes Summary

**One-liner:** Four anti-patterns fixed — dead struct field removed, namespace re-export correctly emits named edge not star edge, TypeScript function overloads deduplicated to single SymbolNode, 30 tests pass with zero warnings.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Fix lib.rs dead code and unused imports | 97fa298 | crates/ts-extractor/src/lib.rs |
| 2 | Fix namespace re-export misclassification and add overload dedup | 910c5cf | crates/ts-extractor/src/edges.rs, symbols.rs, tests/extraction_test.rs |

## What Was Built

### Task 1: lib.rs Dead Code Removal

Removed three dead/unused items from `lib.rs`:
- `ts_lang: TsLanguage` struct field — never read (TSX grammar used for all files)
- `LANGUAGE_TYPESCRIPT` import from tree-sitter-typescript — no longer needed
- `SymbolNode` and `SymbolEdge` from cgraph_core imports — used only in submodules

Result: `cargo build -p cgraph-ts-extractor` produces zero compiler warnings.

### Task 2: Namespace Re-Export Fix (CR-01)

The `extract_reexports` function previously used a single `is_true_star` check (absence of `export_clause`) to gate star edge emission. However, `export * as ns from './module'` has a `namespace_export` child (not an `export_clause`), so it passed the star check and emitted an incorrect wildcard `::*` source edge.

**Fix:** Added `has_namespace_export` detection alongside `has_export_clause`. Three-case logic:
1. `has_namespace_export` → emit ReExport edge with namespace identifier as source_id (`file_path::ns`)
2. `!has_export_clause && !has_namespace_export` → true star export, emit `file_path::*` edge
3. `has_export_clause` → named re-export handled by Pattern 0, skip

### Task 2: Overload Deduplication (WR-01)

TypeScript function overload signatures produce multiple `function_declaration` AST nodes with the same name. The extractor was creating a `SymbolNode` for each, producing duplicates with identical `id` fields.

**Fix:** Added a HashSet dedup step after the query match loop in `extract_symbols`, before calling `extract_non_exported_functions`. Retains the first occurrence (which has the broadest line span encompassing all overload signatures).

### Task 2: New Tests

Three new tests added (27 existing + 3 new = 30 total):
- `namespace_reexport_not_star` — verifies `export * as Utils from './utils'` produces `source_id = "index.ts::Utils"`, not `"index.ts::*"`
- `star_reexport_still_works` — regression guard ensuring plain `export * from './helpers'` still produces the correct star edge after the namespace fix
- `overload_dedup` — verifies three overload signatures for `greet` produce exactly one SymbolNode

## Verification Results

- `cargo build -p cgraph-ts-extractor` — zero warnings
- `cargo test -p cgraph-ts-extractor` — 30 passed, 0 failed
- All three new tests pass individually
- All 27 existing tests continue to pass

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced.

## Threat Model Compliance

| Threat ID | Disposition | Status |
|-----------|-------------|--------|
| T-02-05-01 | mitigate | Implemented — `has_namespace_export` guard validates child node exists before extracting identifier; falls through to no-op if missing |
| T-02-05-02 | accept | Accepted — HashSet allocation is O(n) for n symbols, bounded by file size |

## Self-Check: PASSED

Files exist:
- crates/ts-extractor/src/lib.rs — FOUND
- crates/ts-extractor/src/edges.rs — FOUND
- crates/ts-extractor/src/symbols.rs — FOUND
- crates/ts-extractor/tests/extraction_test.rs — FOUND

Commits exist:
- 97fa298 — FOUND
- 910c5cf — FOUND
