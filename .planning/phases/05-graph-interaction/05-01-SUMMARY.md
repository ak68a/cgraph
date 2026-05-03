---
phase: 05-graph-interaction
plan: 01
subsystem: server-api
tags: [api, enriched-graph, dead-code, typed-edges, dto]
dependency_graph:
  requires: []
  provides: [EnrichedGraphResponse, SymbolNodeDto, TypedEdge, enriched_projection]
  affects: [crates/server/src/graph_api.rs, crates/cli/src/main.rs, crates/server/src/lib.rs]
tech_stack:
  added: []
  patterns: [DTO projection pattern, TDD red-green-refactor]
key_files:
  created: []
  modified:
    - crates/server/src/graph_api.rs
    - crates/server/src/lib.rs
    - crates/cli/src/main.rs
decisions:
  - Only exported symbols included in symbols array (VIZN-03: expand shows exports)
  - file_level_projection kept intact for backward compatibility; enriched_projection is primary path
  - Edge types use snake_case: import/call/type_ref/re_export (matches existing ExportCounts conventions)
  - Symbol kinds use lowercase: function/class/type/interface/hook/enum/module
  - Both symbol-level AND deduplicated file-level edges emitted in EnrichedGraphResponse.edges
metrics:
  duration: 15m
  completed: 2026-05-03
  tasks_completed: 2
  files_modified: 3
---

# Phase 5 Plan 01: Enriched Graph API Summary

**One-liner:** EnrichedGraphResponse with SymbolNodeDto, TypedEdge, and dead code flags via enriched_projection, replacing FileGraphResponse in the API layer.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 (RED) | Add failing tests for EnrichedGraphResponse | 39d3bef | crates/server/src/graph_api.rs |
| 1 (GREEN) | Add EnrichedGraphResponse types and enriched_projection | 1741423 | crates/server/src/graph_api.rs |
| 2 | Wire CLI to enriched_projection, update server exports | a7ba8fb | crates/cli/src/main.rs, crates/server/src/lib.rs |

## What Was Built

### New Types in `graph_api.rs`

- `SymbolNodeDto`: id, name, kind (lowercase string), file_path, is_dead_code, dead_code_confidence (Option<String>)
- `TypedEdge`: source, target, edge_type (snake_case string)
- `EnrichedGraphResponse`: nodes (Vec<FileNode>), edges (Vec<TypedEdge>), symbols (Vec<SymbolNodeDto>), stats, project_name

### New Function: `enriched_projection`

Signature: `pub fn enriched_projection(graph: &CodeGraph, dead_result: &DeadCodeResult, stats: ScanStats, project_name: String) -> EnrichedGraphResponse`

Builds:
1. File nodes (identical logic to file_level_projection)
2. Symbol nodes (exported only) with dead code flags from DeadCodeResult.confirmed/suspicious
3. Edges: both symbol-level typed edges (with actual symbol IDs and edge_type) AND deduplicated file-level edges (with file path IDs and edge_type "import")

### AppState Updated

`AppState.file_graph` type changed from `Arc<FileGraphResponse>` to `Arc<EnrichedGraphResponse>`. The `graph_handler` axum handler body is unchanged (it serializes whatever is behind the Arc).

### CLI Wiring

`main.rs` now calls `enriched_projection(&code_graph, &dead_result, stats, project_name)` instead of `file_level_projection`. The `dead_result` was already computed at line 69 before the projection call.

## TDD Gate Compliance

- RED gate: commit 39d3bef — `test(05-01): add failing tests for EnrichedGraphResponse`
- GREEN gate: commit 1741423 — `feat(05-01): add EnrichedGraphResponse types and enriched_projection function`
- Tests confirmed failing before implementation, passing after.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — all fields are wired to real data from CodeGraph and DeadCodeResult.

## Threat Flags

None — no new network endpoints or trust boundaries introduced. The response shape expands the existing /api/graph payload with additional fields. Server remains 127.0.0.1-only.

## Self-Check: PASSED

- FOUND: crates/server/src/graph_api.rs
- FOUND: crates/cli/src/main.rs
- FOUND: .planning/phases/05-graph-interaction/05-01-SUMMARY.md
- FOUND: commit 39d3bef (RED phase tests)
- FOUND: commit 1741423 (GREEN phase implementation)
- FOUND: commit a7ba8fb (Task 2 CLI wiring)
