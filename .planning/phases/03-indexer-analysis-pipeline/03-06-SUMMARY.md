---
phase: 03-indexer-analysis-pipeline
plan: 06
subsystem: indexer-resolve
tags: [tsconfig, baseUrl, extends-chain, multi-target, path-resolution, gap-closure]
dependency_graph:
  requires: [03-05]
  provides: [tsconfig-extends-chain, baseUrl-resolution, multi-target-path-resolution]
  affects: [crawl-pipeline, dead-code-accuracy, edge-resolution]
tech_stack:
  added: []
  patterns: [extends-chain-with-cycle-guard, multi-candidate-resolution, bare-specifier-baseUrl]
key_files:
  created: []
  modified:
    - crates/indexer/src/resolve.rs
    - crates/indexer/src/crawl.rs
decisions:
  - "Child tsconfig paths fully override parent paths (no merge) matching TypeScript compiler behavior"
  - "resolve_candidates returns all path targets; resolve_file_path tries each against graph"
  - "baseUrl only applies to bare specifiers (not starting with . or /)"
  - "Cycle guard uses HashSet<PathBuf> with canonicalize for symlink safety"
metrics:
  duration: "4m 31s"
  completed: "2026-05-02T23:55:43Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 7
  tests_total_passing: 49
---

# Phase 03 Plan 06: TsConfig Enhanced Resolution Summary

Enhanced TsConfigAliases with extends chain following, baseUrl bare-specifier resolution, and multi-target path candidate evaluation against the code graph.

## One-liner

TsConfig extends chain following with cycle guard, baseUrl bare-specifier resolution, and multi-target path resolution trying all candidates against the graph.

## Tasks Completed

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Enhance TsConfigAliases with baseUrl, extends, multi-target | abe01e3 (RED), 2582bf3 (GREEN) | load_tsconfig_from_path with cycle guard, resolve_candidates, baseUrl for bare specifiers, updated resolve_file_path signature |
| 2 | Integration test for extends + baseUrl pipeline | 04afa7c | test_tsconfig_extends_baseurl_integration in crawl.rs |

## Implementation Details

### Extends Chain Following (T-03-10, T-03-11)

New internal `load_tsconfig_from_path(tsconfig_path, visited)` helper:
- `visited: HashSet<PathBuf>` with `canonicalize()` prevents infinite recursion on circular extends
- Resolves extends path relative to current tsconfig's directory
- Adds `.json` extension if not present on extends target
- Parent config loaded first, then child values override (child paths replace parent paths entirely)

### baseUrl Resolution

Integrated into `resolve()` via `resolve_candidates()`:
- Alias prefix matching takes priority (existing behavior)
- If no alias matches and path is a bare specifier (no `.` or `/` prefix), prepend baseUrl
- Relative paths (`./foo`, `../bar`) and absolute paths (`/foo`) are never modified by baseUrl

### Multi-Target Path Resolution

New `resolve_candidates(&self, raw_path: &str) -> Vec<String>`:
- Returns ALL possible resolutions for a raw import path
- One candidate per path target in the alias mapping
- `resolve_file_path()` tries each candidate against the graph, returning first match
- Falls back to first candidate if none match a graph node

### resolve_file_path Signature Change

Updated to accept `graph: &CodeGraph` and `symbol: &str` parameters for multi-target resolution. The `resolve_edges()` function was updated to pass these through. The `needs_resolution` check was also expanded to trigger on bare specifiers when baseUrl is set.

## TDD Gate Compliance

- RED commit: abe01e3 (test(03-06): add failing tests)
- GREEN commit: 2582bf3 (feat(03-06): implement resolution)
- Gate sequence: RED -> GREEN verified in git log

## Verification Results

- `cargo test -p cgraph-indexer -- resolve::tests`: 20 tests pass
- `cargo test -p cgraph-indexer`: 49 tests pass (all)
- `cargo test --workspace`: 109 tests pass, 0 failures
- Integration test proves extends + baseUrl works end-to-end through Indexer::index()

## Deviations from Plan

None - plan executed exactly as written.

## Self-Check: PASSED

- All source files exist (resolve.rs, crawl.rs)
- All commits exist (abe01e3, 2582bf3, 04afa7c)
- load_tsconfig_from_path function present (1 occurrence)
- resolve_candidates present (6 occurrences: definition + usage + tests)
- test_tsconfig_extends_baseurl_integration present in crawl.rs
