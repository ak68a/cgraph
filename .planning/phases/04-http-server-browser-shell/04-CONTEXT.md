# Phase 4: HTTP Server & Browser Shell - Context

**Gathered:** 2026-05-02
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers the browser visualization shell — a new `crates/server` crate (axum) that serves embedded static assets (HTML/JS/CSS with D3.js) and a JSON API endpoint exposing the CodeGraph as a file-level force-directed graph. Running `cg <path>` starts a localhost HTTP server, auto-opens the browser, and renders the graph with pre-settled force simulation, arrowhead edges, and color-coded file nodes. No interaction beyond hover tooltips — expand/collapse, search, filtering, and overlays belong to Phase 5.

Requirements: VIZN-01, VIZN-02, VIZN-05, VIZN-06, VIZN-07, VIZN-08, INFR-02

</domain>

<decisions>
## Implementation Decisions

### Visual Style
- **D-50:** Dark dev-tool aesthetic. Background #1a1a2e (dark navy). Nodes are bright against dark. Edges subtle gray with opacity. Text white/light gray. Evokes GitHub dependency graph / Figma dark canvas.
- **D-51:** Semantic color palette for node types. Phase 4 shows only file nodes (steel blue #4a9eff). Full palette for Phase 5 expand: functions=#2dd4bf (teal), classes=#f87171 (coral), types/interfaces=#fbbf24 (gold), hooks=#a78bfa (purple), enums=#4ade80 (green).
- **D-52:** Edges uniform in Phase 4 — all edges same color (#555 gray, 0.4 opacity) with arrowheads. Edge type distinction (solid/dashed/colored) deferred to Phase 5 when symbols are visible and edge types become meaningful.

### Node Presentation
- **D-53:** File nodes are filled circles with label below. Size scales with exported symbol count: 8px (1 export) to 24px (20+ exports). Larger nodes = more important files at a glance.
- **D-54:** Labels show filename only (e.g., "auth.ts"), truncated at 20 chars. If duplicate filenames exist in different directories, disambiguate with parent dir prefix ("utils/index.ts" vs "hooks/index.ts"). Full relative path shown on hover.
- **D-55:** Hover tooltip shows: full relative path, export summary by kind ("3 functions, 2 types"), edge counts ("8 incoming • 4 outgoing"). Quick overview without expanding.

### Layout & Grouping
- **D-56:** Two layout modes, togglable: (1) Natural force-based clustering (default) — files that share many edges cluster organically via force physics, showing actual dependency structure. (2) Directory halos — subtle dashed boundary and very-low-opacity background behind files from the same directory, showing filesystem context. Default: halos OFF (pure dependency structure). Toggle in floating panel.
- **D-57:** Medium density — balanced spacing where nodes don't overlap but the whole project structure is visible without excessive zooming. Tuned for typical sizes (nighthawk=92 files, OversizeConnect=419 files).

### Page Chrome
- **D-58:** Minimal header + floating legend panel. Thin top bar with project name and compact scan stats ("92 files • 277 symbols • 630 edges • 147ms"). Graph fills remaining viewport. No sidebar.
- **D-59:** Floating legend/control panel in bottom-right corner, collapsible. Contains: color key, directory halos toggle, and any future Phase 5 toggles. Unobtrusive — graph is the star.

### Server & Architecture
- **D-60:** axum for HTTP server (mentioned in CLAUDE.md tech stack). Serves embedded static assets via rust-embed or include_str. Single `/api/graph` JSON endpoint returns the file-level graph projection. Server starts on first available port (default 3000, increment if taken).
- **D-61:** Browser client is vanilla HTML + JS + D3.js. No build step, no framework, no TypeScript compilation. Single index.html with inline or co-located JS. Embedded as static assets in the Rust binary. Keeps distribution as a single binary with zero runtime deps.
- **D-62:** Auto-open browser via platform `open` command (macOS: `open`, Linux: `xdg-open`). Graceful fallback: if open fails, print URL to stdout. Support `--no-open` flag for headless/CI environments.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Context
- `.planning/ROADMAP.md` — Phase 4 goal, success criteria, dependency chain
- `.planning/REQUIREMENTS.md` — VIZN-01, VIZN-02, VIZN-05, VIZN-06, VIZN-07, VIZN-08, INFR-02
- `.planning/PROJECT.md` — Tech stack constraints, browser client philosophy

### Prior Phase Context
- `.planning/phases/01-foundation/01-CONTEXT.md` — D-01 through D-24 (symbol ID format, workspace layout, error handling)
- `.planning/phases/03-indexer-analysis-pipeline/03-CONTEXT.md` — D-45 through D-48 (petgraph DiGraph, CodeGraph API, dynamic extractor registry, file-level views derived on demand)

### Existing Code
- `crates/indexer/src/graph.rs` — CodeGraph struct (the data source for the browser graph)
- `crates/indexer/src/analysis.rs` — dead_code(), detect_cycles() (analysis data available for future overlays)
- `crates/cli/src/main.rs` — Current CLI entry point (needs extension for server mode)
- `crates/core/src/model.rs` — SymbolNode, SymbolEdge (already Serialize/Deserialize)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `CodeGraph` in `crates/indexer/src/graph.rs` — wraps `DiGraph<SymbolNode, EdgeKind>` with HashMap index. Has `node_count()`, `edge_count()`, `file_count()` methods. All model types derive `Serialize` — can serialize directly to JSON for the API endpoint.
- `scan_directory()` file count and `Indexer::index()` timing already computed in CLI — reuse for the stats header.

### Established Patterns
- Crate per concern: new `crates/server` for HTTP + static assets. Depends on `cgraph-indexer` for CodeGraph.
- CLI owns orchestration: the `main.rs` will gain a `--serve` or default-serve mode that builds the graph then starts the server.
- Error handling D-13: warn and continue. If server fails to bind, try next port. If browser open fails, print URL.

### Integration Points
- `crates/cli/src/main.rs` — needs new path: after `indexer.index()`, start server with `CodeGraph` and open browser
- `CodeGraph` → JSON serialization — need a file-level projection endpoint that groups symbols by file and aggregates edges between files
- Phase 5 will add WebSocket for interaction events; Phase 6 adds WebSocket for watch-mode graph patches. Server architecture should anticipate WebSocket upgrade path.

</code_context>

<specifics>
## Specific Ideas

- The graph should feel immediate — pre-settled simulation means the user sees the final layout instantly, no jittery animation on load
- OversizeConnect (419 files, 880 edges) and nighthawk (92 files, 630 edges) are the reference test cases for layout tuning
- The floating legend panel should be small enough to not obstruct the graph but discoverable enough that users find the halos toggle

</specifics>

<deferred>
## Deferred Ideas

- **Edge type visual distinction** (solid/dashed/colored by Import/Call/TypeRef) — belongs in Phase 5 when symbols are expanded and edge types become meaningful at that granularity
- **Dark/light mode toggle** — start with dark only, add system-adaptive in a future polish pass
- **Canvas rendering for large graphs** (>2000 nodes) — ADVZ-02 is a v2 requirement, not needed for file-level view at typical project sizes
- **Keyboard shortcuts** — Phase 5 interaction concern

</deferred>

---

*Phase: 4-HTTP Server & Browser Shell*
*Context gathered: 2026-05-02*
