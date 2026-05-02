---
phase: 02-typescript-extractor
plan: "03"
subsystem: ts-extractor
tags: [rust, tree-sitter, edge-extraction, tdd, imports, calls, type-refs, re-exports]
dependency_graph:
  requires: [02-02]
  provides: [complete-extraction-result, edge-extraction-pass2]
  affects: [cgraph-ts-extractor, extraction-tests]
tech_stack:
  added: []
  patterns:
    - "Two-pass extraction: Pass 1 = symbols (Plan 02), Pass 2 = edges (Plan 03)"
    - "tree-sitter query pattern_index discrimination for re-export star vs named"
    - "export_clause child check to guard against Pattern 1 false-positive star matches"
    - "Raw path emission for all import/re-export edges (D-28, D-25)"
key_files:
  created: []
  modified:
    - crates/ts-extractor/src/edges.rs
    - crates/ts-extractor/tests/extraction_test.rs
decisions:
  - "Use pattern_index + export_clause child guard to distinguish star vs named re-exports: Pattern 1 in REEXPORT_QUERY_SRC matches any export_statement with a source field (including named re-exports), so we check for absence of export_clause child node to confirm true star export"
  - "source_id for imports uses file_path::<import> sentinel (file-level context, not function scope): Phase 3 can refine to caller scope if needed"
  - "source_id for calls uses file_path::<call> sentinel: member calls (obj.method()) naturally excluded by CALL_QUERY requiring function:(identifier)"
metrics:
  duration_seconds: 255
  completed_date: "2026-05-02T16:54:15Z"
  tasks_completed: 2
  files_modified: 2
---

# Phase 2 Plan 03: Edge Extraction (Pass 2) Summary

Pass 2 edge extraction implemented, completing the full TsExtractor. The crate now produces complete `ExtractionResult` with nodes (Pass 1, Plan 02) and edges (Pass 2, Plan 03).

## What Was Built

**`crates/ts-extractor/src/edges.rs`** — Full implementation replacing the stub:
- `extract_edges`: top-level dispatcher calling four sub-functions
- `extract_imports`: named/default/namespace import edges; raw path preserved per D-28
- `extract_calls`: direct identifier calls only; member calls excluded by query structure per D-30
- `extract_type_refs`: class extends/implements and interface extends TypeRef edges
- `extract_reexports`: per-specifier named ReExport edges and single wildcard star ReExport edge

**`crates/ts-extractor/tests/extraction_test.rs`** — 15 new integration tests (27 total in file, 29 including unit tests):
- PARS-05 (imports): 4 tests covering named, relative, default, alias path preservation
- PARS-06 (calls): 2 tests verifying direct calls captured and member calls excluded
- PARS-07 (type refs): 3 tests for class extends, implements, and interface extends
- PARS-08 (re-exports): 3 tests for named per-specifier edges, star wildcard, raw path
- 2 full integration tests: nodes+edges together, partial parse without panic

## Test Results

```
test result: ok. 27 passed; 0 failed; 0 ignored; finished in 0.42s
```

Full workspace: 53 tests, all green.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Star re-export query Pattern 1 false-positive detection**
- **Found during:** Task 1 implementation analysis
- **Issue:** `REEXPORT_QUERY_SRC` Pattern 1 `(export_statement source: ...)` matches both `export * from './x'` AND `export { foo } from './x'` since both have a `source` field. Using pattern_index alone would emit spurious star edges for every named re-export statement.
- **Fix:** Added `export_clause` child guard: after matching Pattern 1, walk up to the `export_statement` node and check that it has no `export_clause` child. Named re-exports have `export_clause`; star exports do not.
- **Files modified:** `crates/ts-extractor/src/edges.rs`
- **Commit:** 09da61f

## Known Stubs

None. All edge types fully implemented and verified by tests.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. Threat model concerns T-02-06 (linear DoS), T-02-07 (malicious import paths as opaque strings), and T-02-08 (partial parse with error recovery) are all mitigated as specified. `partial_parse_still_extracts` test confirms D-14 behavior.

## Self-Check: PASSED

- [x] `crates/ts-extractor/src/edges.rs` exists and contains all five functions
- [x] `crates/ts-extractor/tests/extraction_test.rs` exists with 27 test functions
- [x] Task 1 commit exists: 09da61f
- [x] Task 2 commit exists: 508d96a
- [x] `cargo test -p cgraph-ts-extractor` passes all 27 tests
- [x] `cargo test` (full workspace) passes all 53 tests
