---
phase: 04-http-server-browser-shell
verified: 2026-05-03T04:45:21Z
status: human_needed
score: 6/6
overrides_applied: 1
overrides:
  - must_have: "Nodes are visually distinguished by symbol type via color coding (function, class, type, interface, hook, file each distinct)"
    reason: "At file-level view all nodes represent files, not individual symbol types. Uniform gray (#555) with purple hover highlight matches Obsidian theme. Per-type coloring applies when symbol-level nodes are introduced in Phase 5."
    accepted_by: "alex@trio.dev"
    accepted_at: "2026-05-03T00:00:00Z"
human_verification:
  - test: "Run `cargo run -p cg -- <typescript-project-path>` and verify the browser opens with a visible D3 force graph"
    expected: "Graph appears instantly settled -- no jitter or animation on initial load. Nodes are gray circles of varying sizes on a dark charcoal background."
    why_human: "Visual rendering, animation behavior, and layout quality cannot be verified programmatically"
  - test: "Hover over a node and check the tooltip"
    expected: "Tooltip shows full file path, export breakdown (e.g., '3 functions, 2 types'), and incoming/outgoing edge counts"
    why_human: "Tooltip positioning, content rendering, and visual quality require visual confirmation"
  - test: "Verify arrowheads on edges are visible and point toward dependency targets"
    expected: "Arrowheads visible at the end of edge lines, not hidden behind nodes. Arrow direction indicates dependency direction."
    why_human: "Arrowhead visibility at various zoom levels and node sizes is a visual property"
  - test: "Test zoom/pan via scroll and drag"
    expected: "Scroll zooms in/out smoothly. Click-drag pans the graph. Labels fade at low zoom levels."
    why_human: "Interaction smoothness and zoom behavior require manual testing"
  - test: "Toggle settings panel sections and directory halos"
    expected: "Panel sections collapse/expand. Directory halos toggle draws dashed boundary groups around files in the same directory."
    why_human: "UI interaction behavior and visual correctness require manual testing"
---

# Phase 4: HTTP Server & Browser Shell Verification Report

**Phase Goal:** Users run `cg <path>` and a browser tab opens showing a D3 force-directed graph of the scanned project -- file-level nodes by default, edges with arrowheads, nodes color-coded by type, simulation pre-settled so the graph is immediately usable.
**Verified:** 2026-05-03T04:45:21Z
**Status:** human_needed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cg <path>` starts a localhost HTTP server, opens the browser automatically, and the graph is visible within a few seconds | VERIFIED | CLI has `#[tokio::main]`, calls `find_available_port(3000)`, spawns server via `cgraph_server::serve()`, calls `webbrowser::open()`. `--no-open` flag present. Behavioral spot-check confirmed: server starts, API returns JSON, HTML/JS/D3 assets serve at 200. |
| 2 | The default view shows file-level nodes only (not individual symbols), preventing the hairball on first load | VERIFIED | `file_level_projection()` in graph_api.rs groups symbols by file_path, produces one `FileNode` per file, deduplicates edges. API response confirmed: 7 file nodes from 21 symbols (fixture dir). Node IDs are file paths, no `::` symbol separators. |
| 3 | Edges render with arrowheads that indicate direction of dependency | VERIFIED | graph.js defines SVG marker `#arrow` with arrow path `M 0,-5 L 10,0 L 0,5`. Links use `marker-end: url(#arrow)`. `adjustedEndpoint()` function offsets line endpoint by `targetRadius + 6` to prevent arrowhead occlusion behind nodes. Arrows toggle in settings panel. |
| 4 | Nodes are visually distinguished by symbol type via color coding | PASSED (override) | Override: At file-level view all nodes represent files, not individual symbol types. Uniform gray (#555) with purple hover highlight (#7f6df2) matches Obsidian theme. Per-type coloring will apply when symbol-level nodes are introduced in Phase 5. Accepted by alex@trio.dev on 2026-05-03. |
| 5 | The force simulation completes before the graph is painted -- the graph does not animate/jitter when the page loads | VERIFIED | graph.js line 90: `simulation.stop()` called immediately after creation, before any SVG rendering. Line 92: `simulation.tick(300)` runs 300 iterations synchronously. `updatePositions()` called once after tick to set final positions. No live `tick` event handler for initial render. |
| 6 | The graph uses progressive disclosure: files are the top level, with the ability to go deeper in subsequent phases | VERIFIED | File-level projection provides the top layer. Phase 5 SC-1 explicitly covers "clicking a file node expands it to show exported symbols." The data model supports this: `FileNode.export_counts` breaks down by kind, and the full `CodeGraph` with symbol-level nodes is available server-side. |

**Score:** 6/6 truths verified (1 via override)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/server/Cargo.toml` | Server crate manifest with axum, tokio, rust-embed, serde | VERIFIED | Contains `name = "cgraph-server"`, all expected dependencies present |
| `crates/server/src/lib.rs` | Public API: create_router, serve, AppState, file_level_projection, FileGraphResponse | VERIFIED | 14 lines. Exports all required symbols. `serve()` wrapper encapsulates axum. |
| `crates/server/src/graph_api.rs` | File-level projection, response types, axum handler | VERIFIED | 465 lines. Contains `pub fn file_level_projection`, `FileNode`, `FileEdge`, `ScanStats`, `FileGraphResponse`, `AppState`, `create_router`, `find_available_port`. 4 unit tests. |
| `crates/server/src/static_assets.rs` | rust-embed asset serving with path traversal protection | VERIFIED | 42 lines. `#[derive(Embed)]` with `#[folder = "../../client/"]`. Path traversal check: `path.contains("..") \|\| path.starts_with('/')` returns 400 Bad Request. |
| `client/index.html` | Obsidian-themed HTML shell with header, graph container, tooltip, settings panel | VERIFIED | 246 lines. Dark charcoal #202020 background. Header bar, graph div, tooltip, settings panel with filters/display/forces sections, empty/error states. Script tags for d3.v7.min.js and graph.js. |
| `client/graph.js` | D3 force graph visualization | VERIFIED | 348 lines. Contains forceSimulation, tick(300), zoom, arrowhead markers, adjustedEndpoint, tooltip, directory halos via polygonHull, drag interaction, settings panel controls. |
| `client/d3.v7.min.js` | D3 v7 library bundled | VERIFIED | 280KB, 2 lines. Contains `forceSimulation`. |
| `crates/cli/src/main.rs` | Async main with server startup, browser open, --no-open | VERIFIED | 215 lines. `#[tokio::main]`, `find_available_port(3000)`, `cgraph_server::serve()`, `webbrowser::open()`, `tokio::signal::ctrl_c()`. No direct axum dependency. |
| `crates/cli/Cargo.toml` | CLI crate with server, tokio, webbrowser deps | VERIFIED | Contains cgraph-server, tokio (features=["full"]), webbrowser. No direct axum. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/server/src/graph_api.rs` | `crates/indexer/src/graph.rs` | `use cgraph_indexer::CodeGraph` | WIRED | Line 12: `use cgraph_indexer::CodeGraph;` -- used as parameter to `file_level_projection()` |
| `crates/server/src/graph_api.rs` | `crates/core/src/model.rs` | `use cgraph_core::SymbolKind` | WIRED | Line 11: `use cgraph_core::SymbolKind;` -- used in match arms for export counting |
| `crates/server/src/static_assets.rs` | `client/` | `#[folder = "../../client/"]` | WIRED | Line 9: rust-embed folder directive. Client files embedded at compile time. |
| `client/graph.js` | `/api/graph` | `fetch('/api/graph')` | WIRED | Line 27: `var response = await fetch('/api/graph');` -- response parsed as JSON, used for nodes/edges/stats |
| `client/index.html` | `client/graph.js` | `<script src="graph.js">` | WIRED | Line 244: `<script src="graph.js"></script>` |
| `client/index.html` | `client/d3.v7.min.js` | `<script src="d3.v7.min.js">` | WIRED | Line 243: `<script src="d3.v7.min.js"></script>` |
| `crates/cli/src/main.rs` | `crates/server/src/lib.rs` | `use cgraph_server` | WIRED | Line 11: imports ScanStats, AppState, file_level_projection, find_available_port. Line 125: `cgraph_server::serve(listener, state)` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `client/graph.js` | `data` (nodes, edges, stats) | `fetch('/api/graph')` | Yes -- API confirmed returning 7 nodes, 5 edges, populated stats from fixture scan | FLOWING |
| `crates/server/src/graph_api.rs` | `FileGraphResponse` | `file_level_projection(&code_graph, stats, project_name)` | Yes -- processes real CodeGraph from indexer, not static data | FLOWING |
| `crates/cli/src/main.rs` | `code_graph` | `indexer.index(&cli.path)` | Yes -- indexer scans real filesystem, returns real graph | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Server starts and API returns graph JSON | `cargo run -p cg -- fixtures --no-open` then `curl /api/graph` | 7 nodes, 5 edges, stats populated, project_name="fixtures" | PASS |
| HTML shell served at root | `curl http://localhost:3000/` | HTTP 200, contains `<!DOCTYPE html>` | PASS |
| graph.js served | `curl http://localhost:3000/graph.js` | HTTP 200 | PASS |
| D3 library served | `curl http://localhost:3000/d3.v7.min.js` | HTTP 200 | PASS |
| Path traversal blocked | `curl http://localhost:3000/foo..bar` | HTTP 400 "Invalid path" | PASS |
| --no-open flag in help | `cargo run -p cg -- --help` | Shows `--no-open` flag | PASS |
| Build compiles clean | `cargo build -p cg` | Exit 0, no warnings | PASS |
| All tests pass | `cargo test` | 33 tests passed, 0 failed | PASS |
| Server unit tests pass | `cargo test -p cgraph-server` | 4 tests passed | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| VIZN-01 | 04-02, 04-04 | Graph renders as D3 force-directed layout in the browser | SATISFIED | graph.js creates `d3.forceSimulation`, renders SVG circles and lines |
| VIZN-02 | 04-01, 04-04 | Default view shows file-level nodes (not individual symbols) | SATISFIED | `file_level_projection()` produces one node per file; API confirmed 7 file nodes from 21 symbols |
| VIZN-05 | 04-02, 04-04 | Edges show directionality via arrowheads | SATISFIED | SVG marker `#arrow` with `marker-end` on all link elements; `adjustedEndpoint()` for occlusion prevention |
| VIZN-06 | 04-02, 04-04 | Nodes are color-coded by symbol type | SATISFIED (override) | See override: uniform gray with purple hover at file-level; per-type colors deferred to Phase 5 symbol view |
| VIZN-07 | 04-02, 04-04 | Force simulation pre-settles before rendering | SATISFIED | `simulation.stop()` then `tick(300)` before any SVG creation; no live tick handler for initial paint |
| VIZN-08 | 04-01, 04-04 | Graph uses progressive disclosure (files -> exports -> internals) | SATISFIED | File-level view is the entry point; Phase 5 adds expand/collapse to reveal symbol-level nodes |
| INFR-02 | 04-03, 04-04 | Tool starts a localhost HTTP server and auto-opens the browser | SATISFIED | `find_available_port(3000)`, `cgraph_server::serve()`, `webbrowser::open()` in main.rs; binds 127.0.0.1 only |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `client/index.html` | 154 | `style="display:none"` on `#panel-toggle` button | INFO | Gear toggle button is hidden; panel is always visible. Appears intentional but the hidden button with attached event listener is dead UI code. |

### Human Verification Required

### 1. Graph renders without jitter on page load

**Test:** Run `cargo run -p cg -- <typescript-project-path>` and check the browser
**Expected:** Graph appears instantly settled -- no jitter or animation on initial load. Nodes are gray circles of varying sizes on a dark charcoal background.
**Why human:** Visual rendering, animation behavior, and layout quality cannot be verified programmatically

### 2. Tooltip content and positioning

**Test:** Hover over a node in the rendered graph
**Expected:** Tooltip shows full file path, export breakdown (e.g., '3 functions, 2 types'), and incoming/outgoing edge counts
**Why human:** Tooltip positioning, content rendering, and visual quality require visual confirmation

### 3. Arrowheads visible and correctly oriented

**Test:** Examine edges between nodes at various zoom levels
**Expected:** Arrowheads visible at the end of edge lines, not hidden behind nodes. Arrow direction indicates dependency direction.
**Why human:** Arrowhead visibility at various zoom levels and node sizes is a visual property

### 4. Zoom and pan interaction

**Test:** Scroll to zoom, click-drag to pan
**Expected:** Smooth zoom in/out. Click-drag pans the graph. Labels fade at low zoom levels.
**Why human:** Interaction smoothness and zoom behavior require manual testing

### 5. Settings panel and directory halos

**Test:** Toggle settings panel sections and directory halos checkbox
**Expected:** Panel sections collapse/expand. Directory halos toggle draws dashed boundary groups around files in the same directory.
**Why human:** UI interaction behavior and visual correctness require manual testing

### Gaps Summary

No blocking gaps found. All 6 success criteria are met (1 via approved override for color coding at file-level view). All 7 requirements mapped to Phase 4 are satisfied. All 9 required artifacts exist, are substantive, and are wired. All 7 key links are verified. Data flows from filesystem through indexer through server API through browser rendering. All 33 tests pass. All 9 behavioral spot-checks pass.

The only items requiring attention are 5 visual/interaction behaviors that need human verification -- these are inherently non-automatable (animation behavior, tooltip rendering, arrowhead visibility, zoom smoothness, panel interaction).

---

_Verified: 2026-05-03T04:45:21Z_
_Verifier: Claude (gsd-verifier)_
