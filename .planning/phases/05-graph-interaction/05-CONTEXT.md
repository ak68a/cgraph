# Phase 5: Graph Interaction - Context

**Gathered:** 2026-05-03
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase transforms the static Phase 4 visualization into a fully interactive graph explorer. Users can expand file nodes to see exported symbols, search and navigate by name, click to focus on a node's neighborhood, activate blast radius and dead code overlays, and filter by file/type/edge. Exploration builds a navigable session history. All interaction happens client-side using data bundled in the initial API response.

Requirements: VIZN-03, VIZN-04, INTR-01, INTR-02, INTR-03, INTR-04, INTR-05, INTR-06, INTR-07, INTR-08

</domain>

<decisions>
## Implementation Decisions

### Node Expand/Collapse
- **D-70:** Three expand modes, selectable via a dropdown in the settings panel: (1) Orbital ring — symbols fan out in a circle around the parent file node at fixed radius, parent stays in place. (2) Force-integrated — symbols join the simulation as regular nodes with short parent-child links, positions settle via physics. (3) Stacked list — symbols appear as a vertical list below the file node, UML-style. User can switch between modes live to find their preference.
- **D-71:** Clicking a file node expands it; clicking again collapses it. Only one expand mode is active at a time (global setting, not per-node).
- **D-72:** Expanded symbols are color-coded by kind using the Phase 4 palette (D-51): functions=#2dd4bf (teal), classes=#f87171 (coral), types/interfaces=#fbbf24 (gold), hooks=#a78bfa (purple), enums=#4ade80 (green). File nodes remain gray (#555).

### Search & Focus
- **D-73:** Two search mechanisms: (1) Header bar search — always-visible input next to project name. Typing highlights matching nodes in real-time (purple glow on matches, non-matches fade). Enter or click flies camera to the node and activates focus mode. (2) Command palette — Cmd+K / Ctrl+K opens a centered overlay (VS Code-style) with a dropdown result list. Selecting a result focuses on that node.
- **D-74:** Click-to-focus uses the same visual treatment as the existing hover highlight (already implemented in Phase 4 graph.js) but persists on click instead of disappearing on mouseleave. Clicked node + direct neighbors stay full opacity, everything else fades to ~10-12%. Exit focus via Escape key or clicking the background.

### Navigation History
- **D-75:** Back/forward navigation with both arrow buttons (in the header bar) and a breadcrumb trail. Clicking through focused nodes builds a history stack. Back returns to previous focused node, forward to the next. Breadcrumb shows the path of focused nodes (e.g., auth.ts > login > UserService) — clicking any breadcrumb jumps to that node.

### Overlays & Analysis Views
- **D-76:** Add an "Analysis" section to the existing settings panel with toggles: "Dead code" overlay and "Blast radius" mode. Both are panel toggles in the Obsidian-style settings panel, consistent with the existing Filters/Display/Forces sections.
- **D-77:** Dead code visualization uses color intensity for confidence tiers (from Phase 3 D-41): confirmed dead = solid red/orange node border or fill; suspicious = same color but lower opacity or dashed border. Both get a small badge/icon indicator.
- **D-78:** Blast radius is per-node: activate blast radius mode in the panel, then click a node to highlight all its transitive dependents in purple. Computation done client-side via graph traversal of the in-memory adjacency data.

### Filtering System
- **D-79:** Full filter controls in the settings panel (extend existing Filters section): directory tree or dropdown for file/directory filtering, checkboxes for symbol types (function, class, type, hook, enum), checkboxes for edge types (import, call, type-ref). All filters combine with AND logic.
- **D-80:** Quick-filter pills in the header bar for the most common filters (e.g., "Functions only", "Imports only"). These are shortcuts to the panel filters — toggling a pill updates the corresponding panel checkbox and vice versa.

### API & Data Strategy
- **D-81:** Bundle everything in the initial `/api/graph` response: file nodes, symbol nodes, all edges (with type info), and dead code flags. No lazy endpoints — all data available in the browser from first load. For single-project use (OversizeConnect scale: 419 files, ~1000 symbols), the payload is ~200-400KB which loads instantly on localhost.
- **D-82:** Blast radius computed client-side via JS graph traversal of the in-memory adjacency data (no server endpoint needed). The full graph is already in memory.
- **D-83:** Response shape is structured (symbols nested or grouped by file, edge types tagged) so a future multi-repo phase could serve the same shape per-repo with lazy endpoints if needed.

### Claude's Discretion
- Fit-to-screen button placement and behavior (VIZN-04)
- Keyboard shortcuts beyond Cmd+K (e.g., Escape to exit focus)
- Animation timing for expand/collapse transitions
- Quick-filter pill selection (which filters get promoted to the header)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Context
- `.planning/ROADMAP.md` — Phase 5 goal, success criteria, dependency chain
- `.planning/REQUIREMENTS.md` — VIZN-03, VIZN-04, INTR-01 through INTR-08
- `.planning/PROJECT.md` — Tech stack constraints, browser client philosophy

### Prior Phase Context
- `.planning/phases/04-http-server-browser-shell/04-CONTEXT.md` — D-50 through D-62 (visual style, color palette, server architecture, browser client philosophy)
- `.planning/phases/03-indexer-analysis-pipeline/03-CONTEXT.md` — D-40 through D-48 (dead code confidence model, cycle detection, graph storage, CodeGraph API)

### Existing Code (MUST READ before modifying)
- `client/graph.js` — Current D3 visualization (346 lines): hover highlight, drag, zoom, settings panel wiring, adjacency map. Phase 5 extends this heavily.
- `client/index.html` — Current HTML shell (246 lines): header bar, settings panel with Filters/Display/Forces sections, tooltip. Phase 5 adds new sections and header elements.
- `crates/server/src/graph_api.rs` — File-level projection, FileGraphResponse, AppState. Phase 5 must extend the response to include symbol-level data and dead code flags.
- `crates/indexer/src/analysis.rs` — `dead_code()`, `blast_radius()`, `detect_cycles()`, `transitive_deps()` APIs available server-side.
- `crates/indexer/src/graph.rs` — `CodeGraph` struct wrapping `DiGraph<SymbolNode, EdgeKind>`.
- `crates/core/src/model.rs` — `SymbolNode`, `SymbolEdge`, `EdgeKind`, `SymbolKind` (all derive Serialize).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Hover highlight logic in `graph.js:166-234` — D3 transitions for fade in/out with adjacency-based neighbor detection. Focus mode reuses this exact visual treatment, just makes it persistent on click.
- Settings panel in `index.html:157-233` — Obsidian-style collapsible sections with toggle switches and sliders. New Analysis/Filter sections follow the same pattern.
- `adjacency` Map in `graph.js:80` — already tracks node connections. Extend for symbol-level adjacency when files are expanded.
- `analysis.rs` exports `DeadCodeResult` with `confirmed` and `suspicious` vectors — map directly to D-77 visual treatment.

### Established Patterns
- Vanilla JS + D3.js (no framework, no build step) — per D-61. All new interaction code follows this pattern.
- Settings panel section pattern: `.panel-section` > `.section-header` (clickable with chevron) > `.section-body` (toggleable). New sections replicate this.
- Server API pattern: handler function in `graph_api.rs`, route in `create_router()`, state in `AppState`.

### Integration Points
- `graph_api.rs:FileGraphResponse` — must be extended with symbol nodes, edge types, and dead code flags (D-81)
- `graph_api.rs:AppState` — currently holds only `file_graph: Arc<FileGraphResponse>`. May need full `CodeGraph` reference for symbol data.
- `client/graph.js:loadAndRender()` — entry point that fetches `/api/graph` and builds the visualization. All Phase 5 features plug into this function or are called from it.
- Header bar (`#header` in index.html) — needs search input, back/forward buttons, breadcrumb trail, quick-filter pills.

</code_context>

<specifics>
## Specific Ideas

- The three expand modes should be switchable live so the user can try each and decide which feels best during visual verification
- Focus mode should feel like "the existing hover but it sticks" — minimal new visual treatment, reuse what's already working
- Command palette (Cmd+K) should feel like VS Code — centered overlay, type-ahead filtering, Enter to select
- Dead code overlay should be immediately obvious (red/orange) since it's the primary analysis insight
- Quick-filter pills should be the 3-4 most useful filters, not every possible filter

</specifics>

<deferred>
## Deferred Ideas

- **Edge type visual distinction** (solid/dashed/colored by Import/Call/TypeRef) — could be added alongside the edge type filter, but visual treatment is Claude's discretion
- **Keyboard shortcuts** beyond Cmd+K — future polish pass
- **Canvas rendering for large graphs** (>2000 nodes) — ADVZ-02, v2 requirement
- **Lazy API endpoints for multi-repo scale** — Phase 12 concern, response shape designed to support future split (D-83)

</deferred>

---

*Phase: 5-Graph Interaction*
*Context gathered: 2026-05-03*
