---
phase: 03-indexer-analysis-pipeline
plan: 05
subsystem: indexer-resolve
tags: [edge-resolution, unresolved-edges, extension-mapping, gap-closure]
dependency_graph:
  requires: [03-02]
  provides: [resolve_unresolved_edges, js-to-ts-extension-mapping]
  affects: [crawl-pipeline, dead-code-accuracy]
tech_stack:
  added: []
  patterns: [name-based-symbol-resolution, import-context-disambiguation]
key_files:
  created: []
  modified:
    - crates/indexer/src/resolve.rs
    - crates/indexer/src/crawl.rs
decisions:
  - "Disambiguation uses import context (which file the source already imports) to resolve ambiguous name matches"
  - "When no import disambiguates, alphabetical sort provides deterministic fallback"
  - "JS-to-TS extension mapping runs BEFORE the early-return for known extensions"
metrics:
  duration: "4m 22s"
  completed: "2026-05-02T23:48:32Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 7
  tests_total_passing: 33
---

# Phase 03 Plan 05: Unresolved Edge Resolution Summary

Resolve unresolved:: Call and TypeRef edges by name-matching to exported symbols, with import-context disambiguation and JS-to-TS extension mapping.

## One-liner

Name-based resolution of unresolved:: edge targets with import-context disambiguation and .js/.jsx/.mjs/.cjs to .ts/.tsx/.mts/.cts extension mapping.

## Tasks Completed

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Implement unresolved edge resolution and expand extension candidates | db52a34, 79c47a5 | resolve_unresolved_edges() with name lookup, disambiguation, JS-to-TS extension mapping, 6 unit tests |
| 2 | Wire into index() pipeline and add integration test | 051939d | Call in crawl.rs after resolve_edges(), integration test with multi-file TS project |

## Implementation Details

### resolve_unresolved_edges()

New public function in `crates/indexer/src/resolve.rs`:

1. **Builds exported symbol name map**: HashMap<name, Vec<symbol_id>> from all nodes where `is_exported == true`
2. **Builds import context map**: HashMap<source_file, HashSet<imported_file>> from Import edges for disambiguation
3. **Resolves each unresolved:: edge**:
   - Single match: rewrite target_id directly
   - Multiple matches: prefer candidate whose file is in source's import set
   - No match: leave unchanged (dropped by add_edge for external/third-party calls)

### Extension Resolution Expansion

Modified `resolve_extension()` to handle JS-to-TS mapping:
- `.js` -> `.ts`, `.jsx` -> `.tsx`, `.mjs` -> `.mts`, `.cjs` -> `.cts`
- Runs BEFORE the "already has known extension" early return
- Falls back to original JS path if no TS version exists in graph

### Pipeline Integration

`resolve_unresolved_edges()` called in `Indexer::index()` after `resolve_edges()` (which resolves paths and barrel chains) and before pseudo-node remapping (which strips `<import>`/`<call>` suffixes).

## Verification Results

- `cargo test -p cgraph-indexer`: 42 tests pass
- `cargo test --workspace`: 33 tests pass, 0 failures
- Integration test confirms: edge_count >= 2, Call edges exist, no unresolved:: targets remain

## TDD Gate Compliance

- RED commit: db52a34 (test(03-05): add failing tests)
- GREEN commit: 79c47a5 (feat(03-05): implement resolution)
- Gate sequence: RED -> GREEN verified in git log

## Deviations from Plan

None - plan executed exactly as written.

## Self-Check: PASSED

- All source files exist (resolve.rs, crawl.rs)
- All commits exist (db52a34, 79c47a5, 051939d)
- resolve_unresolved_edges function present in resolve.rs
- resolve_unresolved_edges call wired in crawl.rs
