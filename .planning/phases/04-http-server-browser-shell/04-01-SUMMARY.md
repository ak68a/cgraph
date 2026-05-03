---
phase: 04-http-server-browser-shell
plan: 01
subsystem: server
tags: [axum, rust-embed, http, graph-projection, static-assets]
dependency_graph:
  requires: [cgraph-indexer, cgraph-core]
  provides: [cgraph-server, FileGraphResponse, serve(), file_level_projection()]
  affects: [crates/cli (plan 03 will import cgraph-server)]
tech_stack:
  added: [axum 0.8, tokio 1 (full), rust-embed 8, mime_guess 2]
  patterns: [file-level graph projection, rust-embed static serving, axum state injection]
key_files:
  created:
    - crates/server/Cargo.toml
    - crates/server/src/lib.rs
    - crates/server/src/graph_api.rs
    - crates/server/src/static_assets.rs
    - client/index.html
  modified:
    - Cargo.toml (workspace members)
    - Cargo.lock
decisions:
  - "serve() wrapper in lib.rs encapsulates axum dep so CLI crate does not need it directly"
  - "find_available_port() binds to 127.0.0.1 only (T-04-02: no network exposure)"
  - "static_assets.rs created in Task 1 alongside graph_api.rs since lib.rs mod declaration required it for compilation"
metrics:
  duration: "~10 minutes"
  completed: "2026-05-02"
  tasks_completed: 2
  files_created: 5
  files_modified: 2
---

# Phase 4 Plan 01: Server Crate Scaffold Summary

axum HTTP server with file-level CodeGraph projection, rust-embed static serving, and path traversal protection.

## What Was Built

The `crates/server` crate is the complete Rust backend for Phase 4. It:

1. Takes a `CodeGraph` from the indexer and pre-computes a file-level view via `file_level_projection()`.
2. Serves that view as JSON at `GET /api/graph` using axum.
3. Embeds the `client/` directory at compile time via rust-embed and serves files at `GET /*path`.
4. Rejects path traversal attempts (paths with `..` or absolute `/` prefix) with 400 Bad Request.
5. Exposes a `serve()` wrapper in `lib.rs` so the CLI crate (plan 03) can start the server without a direct axum dependency.

## Task Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Server crate scaffold + file-level projection | b9b8b5c |
| 2 | Static asset handler with path traversal protection | 8a4f607 |

## File-Level Projection Details

The `file_level_projection()` function:
- Groups all `SymbolNode`s by `file_path` to produce one `FileNode` per file.
- Counts exported symbols by kind (`Function`, `Class`, `Type`, `Interface`, `Hook`, `Enum`) — `Module` is skipped.
- Detects duplicate basenames (e.g., two `index.ts` files) and disambiguates with parent dir prefix per D-54.
- Truncates display filenames at 20 characters.
- Computes visual radius: `8.0 + (total_exports / 20.0 * 16.0).min(16.0)` → range 8–24px per D-53.
- Lifts symbol-level edges to file-level, deduplicates via `HashSet`, excludes self-loops (D-42).
- Computes per-file `incoming`/`outgoing` edge counts for tooltip data (D-55).

## Security

Threat T-04-01 (path traversal) mitigated: `static_handler` rejects paths containing `..` or starting with `/` before calling `ClientAssets::get()`.

Threat T-04-02 (network exposure) mitigated: `find_available_port()` always binds to `127.0.0.1`, never `0.0.0.0`.

## Test Results

```
running 4 tests
test graph_api::tests::test_file_level_projection_basic ... ok
test graph_api::tests::test_duplicate_basename_disambiguation ... ok
test graph_api::tests::test_radius_capping ... ok
test graph_api::tests::test_self_edges_excluded ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] static_assets.rs created in Task 1 alongside graph_api.rs**
- **Found during:** Task 1 implementation
- **Issue:** `lib.rs` declares `pub mod static_assets`, which means the module file must exist for the crate to compile. The plan assigns `static_assets.rs` to Task 2, but Task 1's verification (`cargo test -p cgraph-server`) would fail without it.
- **Fix:** Created the complete `static_assets.rs` implementation (identical to Task 2 spec) during Task 1. Task 2 verified its correctness and committed it separately to maintain per-task commit granularity.
- **Files modified:** crates/server/src/static_assets.rs
- **Commit:** 8a4f607

No other deviations.

## Known Stubs

- `client/index.html` — stub HTML (`loading...`) required for rust-embed compilation. Will be replaced with full D3 browser shell in plan 04-02.

## Self-Check

### Created files exist:
- crates/server/Cargo.toml: FOUND
- crates/server/src/lib.rs: FOUND
- crates/server/src/graph_api.rs: FOUND
- crates/server/src/static_assets.rs: FOUND
- client/index.html: FOUND

### Commits exist:
- b9b8b5c: FOUND
- 8a4f607: FOUND

## Self-Check: PASSED
