# Phase 5: Graph Interaction - Research

**Researched:** 2026-05-03
**Domain:** D3.js v7 interactive graph, Rust/Axum API extension, vanilla JS state management
**Confidence:** HIGH

---

## Summary

Phase 5 transforms the static D3 force graph from Phase 4 into a fully interactive explorer.
All ten components (CommandPalette, SearchBar, NavHistory, NodeExpander, DeadCodeOverlay,
BlastRadiusOverlay, FilterPanel, QuickFilterPills, FitToScreenBtn, AnalysisPanel) are
pure vanilla JS + D3 — no build step, no framework. The design contract in 05-UI-SPEC.md
is fully locked. The hardest sub-problem is node expand/collapse: three modes (orbital ring,
force-integrated, stacked list) require dynamically adding/removing nodes from a live D3
simulation while keeping file nodes stable in position.

The Rust side has two concrete tasks: (1) extend `FileGraphResponse` to include symbol nodes,
symbol-level edges with `edge_type` tags, and dead code flags; (2) pass a reference to the
live `CodeGraph` (or its pre-computed dead code result) into `AppState` so the handler can
build the richer response. All analysis algorithms already exist in `analysis.rs`
(`dead_code()`, `blast_radius()`). Client-side blast radius traversal uses the adjacency data
bundled in the API response — no server endpoint needed (D-82).

Navigation history is a simple JS array (`historyStack`, `historyIndex`) with 50-entry cap.
Filtering combines AND logic across three independent axes; the existing `adjacency` Map
extends naturally to cover symbol nodes when files are expanded.

**Primary recommendation:** Tackle the Rust API extension first (wave 1), then the header UI
shell (search + nav + pills), then the expand/collapse engine (the most complex JS piece),
then overlays/filters. This ordering de-risks the data contract before building UI on top.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-70:** Three expand modes: Orbital ring, Force-integrated, Stacked list. Selectable via dropdown in Display section. User can switch live.
- **D-71:** Click file node = expand; click again = collapse. One global expand mode at a time.
- **D-72:** Expanded symbols color-coded by kind using Phase 4 palette: functions=#2dd4bf, classes=#f87171, types/interfaces=#fbbf24, hooks=#a78bfa, enums=#4ade80. File nodes stay #555.
- **D-73:** Two search mechanisms: (1) Header bar search — always-visible 200px input, live highlight on type, Enter flies to node + activates focus. (2) Command palette Cmd+K / Ctrl+K, VS Code-style centered overlay, dropdown result list.
- **D-74:** Click-to-focus persists the hover highlight treatment. Clicked node + direct neighbors at 100% opacity; everything else at 10-12%. Exit via Escape or click background.
- **D-75:** Back/forward navigation: header arrow buttons + breadcrumb trail. History stack. Breadcrumb shows focused-node path; clicking any crumb jumps to that node.
- **D-76:** "Analysis" panel section with Dead code and Blast radius toggles. Placed after Filters, before Display.
- **D-77:** Dead code uses color intensity for confidence: confirmed = solid #f87171 3px border + x badge; suspicious = dashed #f87171 50% opacity + ? badge.
- **D-78:** Blast radius per-node: toggle ON in panel, click a node, transitive dependents highlighted in purple (#a882ff). Client-side traversal via adjacency data.
- **D-79:** Full filters in panel: directory text input with autocomplete, symbol type checkboxes (Function/Class/Type/Hook/Enum), edge type checkboxes (Import/Call/Type Reference). AND logic.
- **D-80:** 4 quick-filter pills in header center: "Functions" / "Classes" / "Imports" / "Calls". Synced bidirectionally with panel checkboxes.
- **D-81:** All data bundled in initial `/api/graph` response. No lazy endpoints.
- **D-82:** Blast radius computed client-side via JS graph traversal. No server endpoint.
- **D-83:** Response shape supports future multi-repo extension (symbols nested/grouped, edges tagged).

### Claude's Discretion

- Fit-to-screen button placement and behavior (VIZN-04)
- Keyboard shortcuts beyond Cmd+K (e.g., Escape to exit focus)
- Animation timing for expand/collapse transitions
- Quick-filter pill selection (which filters get promoted to the header)

### Deferred Ideas (OUT OF SCOPE)

- Edge type visual distinction (solid/dashed/colored by Import/Call/TypeRef)
- Keyboard shortcuts beyond Cmd+K
- Canvas rendering for large graphs (>2000 nodes) — ADVZ-02, v2
- Lazy API endpoints for multi-repo scale — Phase 12
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| VIZN-03 | User can expand a file node to see its exported symbols | NodeExpander module with three modes; symbol data in API response |
| VIZN-04 | User can zoom, pan, and fit-to-screen | D3 zoom behavior already wired; fit-to-screen needs programmatic zoomTransform via node position bounds |
| INTR-01 | User can search for a symbol by name with live highlighting | Header SearchBar + CommandPalette; filter node/label opacity via D3 transitions |
| INTR-02 | User can click a node to focus and see its immediate neighbors | Reuse existing hover highlight; persist on click; exit via Escape/background |
| INTR-03 | User can activate blast radius view to see all transitive dependents | Client-side BFS/DFS over adjacency Map; visual treatment via D3 transitions |
| INTR-04 | User can activate dead code overlay highlighting unused exports | Dead code flags in API response; overlay applied via DeadCodeOverlay module |
| INTR-05 | User can filter the graph by file or directory | Directory text input with autocomplete in panel; show/hide nodes |
| INTR-06 | User can filter the graph by symbol type | Symbol type checkboxes + quick-filter pills; show/hide nodes |
| INTR-07 | User can filter the graph by edge type | Edge type checkboxes; show/hide edges and isolated nodes |
| INTR-08 | User can navigate back/forward through focused nodes | JS history stack (Array + index pointer); back/forward buttons + breadcrumb |
</phase_requirements>

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Node expand/collapse | Browser (D3 SVG) | — | Pure client-side DOM + simulation mutation |
| Symbol data for expanded nodes | API (Rust) | Browser | Server builds symbol list from CodeGraph; browser renders |
| Dead code flags | API (Rust) | Browser | `analysis::dead_code()` runs server-side; flags bundled in response |
| Blast radius computation | Browser (JS) | — | Client-side BFS over in-memory adjacency (D-82) |
| Search highlighting | Browser (D3) | — | Live filter via D3 opacity transitions |
| Focus mode (click-to-focus) | Browser (D3) | — | Reuse adjacency Map; D3 transition on opacity |
| Navigation history | Browser (JS) | — | Simple stack/index; no server state needed |
| Filter logic | Browser (JS) | — | AND across visibility flags; re-render via D3 |
| Fit-to-screen | Browser (D3 zoom) | — | Programmatic `zoom.transform` to computed bounds |
| API response shape | API (Rust) | — | `FileGraphResponse` extended; serde serialization |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| D3.js | 7.9.0 (vendored) | Force simulation, zoom, transitions, data-join | Already bundled in `client/d3.v7.min.js` — do not upgrade |
| axum | (workspace) | HTTP handler extensions | Already in use for `/api/graph` |
| serde / serde_json | (workspace) | JSON serialization of extended response | Already derives on all response types |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| petgraph | (workspace) | `CodeGraph` graph structure | Server-side: iterate SymbolNodes for response construction |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Vanilla JS history stack | URL hash state / History API | URL state enables shareable links (deferred XPRT-02); not needed for v1 |
| Client-side BFS | Server blast_radius endpoint | Server endpoint adds latency and round-trip; full graph already in browser |
| Inline SVG badges (dead code) | HTML overlay positioned over SVG | SVG badges co-locate with nodes; HTML overlay requires coordinate mapping |

**Installation:** No new packages. All capabilities come from existing D3, axum, and petgraph.

---

## Architecture Patterns

### System Architecture Diagram

```
Browser (client-side)
┌──────────────────────────────────────────────────────────────┐
│  index.html                                                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Header bar                                          │   │
│  │  [SearchBar] [QuickFilterPills] [NavHistory arrows]  │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  #breadcrumb (absolute below header)                 │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  SVG graph canvas (D3 force simulation)              │   │
│  │  ┌────────────┐  ┌──────────────┐  ┌─────────────┐  │   │
│  │  │ file nodes │  │ symbol nodes │  │    edges     │  │   │
│  │  │ (expanded) │  │ (per expand  │  │ (typed,      │  │   │
│  │  │            │  │  mode)       │  │  filterable) │  │   │
│  │  └────────────┘  └──────────────┘  └─────────────┘  │   │
│  │  Overlays: DeadCodeOverlay, BlastRadiusOverlay       │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌────────────────────┐  ┌─────────────────────────────┐   │
│  │  Settings panel    │  │  CommandPalette (Cmd+K)      │   │
│  │  Analysis section  │  │  Centered overlay, z:1001   │   │
│  │  Filters section   │  └─────────────────────────────┘   │
│  │  Display section   │                                     │
│  │  Forces section    │  ┌────────────────────────────┐    │
│  └────────────────────┘  │  FitToScreenBtn (bottom-L) │    │
│                           └────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
         │ fetch /api/graph (initial load only)
         ▼
Server (Rust / axum)
┌──────────────────────────────────────────────────────────────┐
│  graph_api.rs                                                │
│  ┌──────────────┐   ┌──────────────────────────────────┐    │
│  │  AppState    │   │  graph_handler()                 │    │
│  │  file_graph  │──▶│  builds EnrichedGraphResponse    │    │
│  │  code_graph  │   │  with symbol nodes + dead code   │    │
│  │  dead_result │   │  flags bundled                   │    │
│  └──────────────┘   └──────────────────────────────────┘    │
│                                                              │
│  CodeGraph (petgraph DiGraph<SymbolNode, EdgeKind>)          │
│  dead_code() → DeadCodeResult { confirmed, suspicious }      │
└──────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure
```
client/
├── index.html      — Extended header bar, breadcrumb, Analysis panel section, filter controls
├── graph.js        — All new interaction modules inline (no build step per D-61)
└── d3.v7.min.js    — Unchanged vendored D3

crates/server/src/
└── graph_api.rs    — Extended FileGraphResponse → EnrichedGraphResponse
                      AppState gains Arc<CodeGraph> or pre-computed DeadCodeResult
```

### Pattern 1: Dynamic Simulation Update (Node Expand/Collapse)
**What:** Add/remove symbol nodes and parent-child edges to the live D3 simulation when a file node is clicked.
**When to use:** NodeExpander module — all three expand modes share this core pattern.

```javascript
// Source: D3 docs https://github.com/d3/d3/blob/main/docs/d3-force/simulation.md
// Step 1: push symbol nodes into the shared nodes array
nodes.push(...symbolNodesForFile);
edges.push(...parentChildEdges);

// Step 2: update simulation node list
simulation.nodes(nodes);
simulation.force('link').links(edges);

// Step 3: rejoin DOM elements using key function (file path = stable key)
node = nodeGroup.selectAll('circle')
  .data(nodes, d => d.id)
  .join(
    enter => enter.append('circle')
      .attr('r', d => d.radius)
      .attr('fill', d => nodeColor(d))
      .call(drag),
    update => update,
    exit => exit.transition().duration(200).attr('r', 0).remove()
  );

// Step 4: reheat simulation briefly
simulation.alpha(0.3).restart();
```

### Pattern 2: Fit-to-Screen (Programmatic Zoom)
**What:** Compute bounding box of all visible nodes, then apply a zoom transform that fits them within the viewport with padding.
**When to use:** FitToScreenBtn click handler.

```javascript
// Source: D3 docs https://github.com/d3/d3/blob/main/docs/d3-zoom.md
function fitToScreen() {
  var visibleNodes = nodes.filter(d => isVisible(d));
  if (visibleNodes.length === 0) return;

  var minX = d3.min(visibleNodes, d => d.x - d.radius);
  var maxX = d3.max(visibleNodes, d => d.x + d.radius);
  var minY = d3.min(visibleNodes, d => d.y - d.radius);
  var maxY = d3.max(visibleNodes, d => d.y + d.radius);

  var padding = 48;
  var svgW = +svg.attr('width');
  var svgH = +svg.attr('height');
  var scale = Math.min(
    (svgW - padding * 2) / (maxX - minX),
    (svgH - padding * 2) / (maxY - minY),
    1  // don't zoom in beyond 1:1
  );
  var tx = svgW / 2 - scale * (minX + maxX) / 2;
  var ty = svgH / 2 - scale * (minY + maxY) / 2;

  svg.transition().duration(500).ease(d3.easeCubicInOut)
    .call(zoomBehavior.transform, d3.zoomIdentity.translate(tx, ty).scale(scale));
}
```

### Pattern 3: Navigation History Stack
**What:** A capped array + index pointer for back/forward navigation through focused nodes.
**When to use:** NavHistory module.

```javascript
// Source: [ASSUMED] standard history pattern, no library needed
var history = [];
var historyIndex = -1;
var MAX_HISTORY = 50;

function pushHistory(nodeId) {
  // Truncate forward history on new navigation
  history = history.slice(0, historyIndex + 1);
  history.push(nodeId);
  if (history.length > MAX_HISTORY) history.shift();
  historyIndex = history.length - 1;
  updateNavButtons();
  updateBreadcrumb();
}

function navigateBack() {
  if (historyIndex > 0) {
    historyIndex--;
    focusNode(history[historyIndex]);
    updateNavButtons();
    updateBreadcrumb();
  }
}
```

### Pattern 4: Client-Side Blast Radius BFS
**What:** Traverse the adjacency map backwards from a seed node to find all transitive dependents.
**When to use:** BlastRadiusOverlay when user clicks a node.

```javascript
// Source: [ASSUMED] standard BFS pattern; mirrors server-side blast_radius() in analysis.rs
function computeBlastRadius(nodeId) {
  // adjacency Map stores BOTH directions (A->B means adjacency[A] contains B AND adjacency[B] contains A)
  // Need directed adjacency: incoming edges only (who depends on nodeId?)
  // The full edge list (with source/target) must be traversed for directed BFS.
  var dependents = new Set();
  var queue = [nodeId];
  while (queue.length > 0) {
    var current = queue.shift();
    edges.forEach(function(e) {
      var tgt = typeof e.target === 'object' ? e.target.id : e.target;
      var src = typeof e.source === 'object' ? e.source.id : e.source;
      if (tgt === current && !dependents.has(src)) {
        dependents.add(src);
        queue.push(src);
      }
    });
  }
  return dependents;
}
```

**Note:** The existing `adjacency` Map in graph.js stores undirected neighbors (both A→B and B→A).
For blast radius (directed: who depends on X?), traverse the raw `edges` array checking `e.target === nodeId`. This needs the edges array to stay in scope — it already does as a closure variable.

### Pattern 5: API Response Extension (Rust)
**What:** Add symbol nodes, typed edges, and dead code flags to the graph handler response.
**When to use:** `graph_api.rs` — extend `FileGraphResponse` or introduce `EnrichedGraphResponse`.

```rust
// Source: [ASSUMED] follows existing Serialize derive pattern in graph_api.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNodeDto {
    pub id: String,
    pub name: String,
    pub kind: String,       // "function" | "class" | "type" | "hook" | "enum" | "interface"
    pub file_path: String,
    pub is_dead_code: bool,
    pub dead_code_confidence: Option<String>, // "confirmed" | "suspicious"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String,  // "import" | "call" | "type_ref" | "re_export"
}

// Extend FileGraphResponse (or use a new EnrichedGraphResponse):
pub struct EnrichedGraphResponse {
    pub nodes: Vec<FileNode>,
    pub edges: Vec<TypedEdge>,      // upgrade: was FileEdge (untyped)
    pub symbols: Vec<SymbolNodeDto>, // new: symbol-level nodes
    pub stats: ScanStats,
    pub project_name: String,
}
```

**AppState extension:** The CLI calls `dead_code(&code_graph, &cli.path)` after indexing.
Store the result in `AppState` so the handler doesn't re-run analysis on every request.

```rust
#[derive(Clone)]
pub struct AppState {
    pub file_graph: Arc<EnrichedGraphResponse>,
    // code_graph: Arc<CodeGraph> NOT needed — all symbol data pre-computed into file_graph
}
```

### Anti-Patterns to Avoid
- **Re-running analysis on every request:** Pre-compute `dead_code()` in `main.rs` at startup (one-shot), store result in `AppState`. Do not call analysis functions inside request handlers.
- **Mutating nodes array while simulation is running:** Always call `simulation.stop()` or use `.nodes(newNodes)` + `.restart()` atomically — never splice nodes mid-tick.
- **Losing zoom state on expand:** The zoom behavior is on the SVG, not the `g` group — expand/collapse must not re-attach the zoom behavior. Only `simulation.nodes()` and `force('link').links()` need updating.
- **Using undirected adjacency for blast radius:** The current `adjacency` Map is undirected (both directions stored). Blast radius requires directed traversal — use the raw `edges` array.
- **Keying D3 data joins by array index:** Always use `d => d.id` as the key function when nodes are added/removed dynamically to prevent wrong exit/update selections.
- **Filtering by toggling CSS `display: none` on nodes inside the simulation:** Nodes hidden via `display: none` still participate in the force simulation. Use `visibility: hidden` or set `node.fx/fy` and exclude from simulation — or rebuild the simulation node set. The simplest correct approach is: keep all nodes in the simulation but use `opacity: 0` + `pointer-events: none` for hidden nodes; the `collide` force still runs but at negligible cost for OversizeConnect scale (~1000 nodes).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Zoom transitions to computed bounds | Custom pan/scale animation loop | `d3.zoom.transform` with `d3.zoomIdentity` and transition | D3 uses `interpolateZoom` internally for smooth transitions; handles edge cases |
| Adding nodes to live simulation | Direct array mutation + manual position update | `simulation.nodes(newNodes)` + `simulation.force('link').links(newEdges)` + `simulation.alpha(0.3).restart()` | D3 reinitializes node physics properties; manual mutation breaks velocity accounting |
| Node color lookup | Inline if/else chains | Object literal lookup table `{function: '#2dd4bf', class: '#f87171', ...}` | Maintainable, matches D-72 palette exactly |
| Dead code traversal on server | Server endpoint for blast radius | Client-side BFS (D-82) | Full graph in browser, zero latency, no extra endpoint |

---

## Common Pitfalls

### Pitfall 1: expand/collapse breaks drag behavior
**What goes wrong:** After adding symbol nodes, the `drag` call is only wired to the original node selection. New symbol nodes are not draggable.
**Why it happens:** D3 `drag` is applied to a selection at call time. `.join()` enter function must re-apply drag to new nodes.
**How to avoid:** In the `join` enter callback, call `.call(drag)` on the entering nodes selection.
**Warning signs:** Hovering newly-added symbol nodes shows `cursor: default` instead of `cursor: grab`.

### Pitfall 2: SVG rendering order breaks overlay badges
**What goes wrong:** Dead code badges (small circles with x/?) appear behind node circles because SVG renders in DOM order.
**Why it happens:** Badges appended after circles in the same `<g>` will still be Z-sorted within that group.
**How to avoid:** Append badges in a separate `<g class="badges">` group that follows the `<g class="nodes">` group in DOM order.
**Warning signs:** Badge shapes visible only on nodes at graph edges; disappears on overlap with other nodes.

### Pitfall 3: focus mode and hover mode conflict
**What goes wrong:** `mouseenter`/`mouseleave` events fire while in focus mode, undoing the persistent focus highlight.
**Why it happens:** The existing hover handlers unconditionally set node colors and opacity.
**How to avoid:** Gate hover handlers with a `focusActive` boolean flag. When `focusActive === true`, skip the hover transition or make it a no-op.
**Warning signs:** Clicking a node to focus, then hovering elsewhere, causes the focus to visually reset.

### Pitfall 4: stale adjacency on expanded graph
**What goes wrong:** After expanding a file node, the `adjacency` Map only reflects file-level connections; symbol-level neighbors are not present. Focus mode and blast radius give wrong results.
**Why it happens:** `adjacency` is built once in `loadAndRender()` from the initial edges list. Symbol edges arrive in the response but are not loaded into the Map until expand.
**How to avoid:** Build a separate `symbolAdjacency` Map when loading the API response (precomputed from `symbols` list and their edges). On expand, merge symbol adjacency entries into the main `adjacency` map. On collapse, remove them.
**Warning signs:** Click-to-focus on an expanded symbol shows no neighbors even when edges exist.

### Pitfall 5: Filter AND logic state management
**What goes wrong:** Toggling a quick-filter pill that syncs with a panel checkbox causes infinite loop when the checkbox `change` event re-triggers the pill update.
**Why it happens:** Bidirectional sync without a guard flag causes event ping-pong.
**How to avoid:** Use a single `filterState` object as source of truth. Both pill and checkbox handlers update `filterState` then call `applyFilters()` which updates the DOM. Neither handler reads the other's DOM state directly.
**Warning signs:** Clicking a pill causes rapid flicker on the corresponding checkbox.

### Pitfall 6: serde rename for snake_case vs camelCase
**What goes wrong:** Rust fields serialize as `edge_type` but JS expects `edgeType` (or vice versa).
**Why it happens:** serde defaults to snake_case; D3/JS convention is camelCase.
**How to avoid:** Either (a) use `#[serde(rename_all = "camelCase")]` on new DTOs to match JS expectations, or (b) consistently use snake_case everywhere (the existing `file_path`, `export_counts`, `elapsed_ms` fields in the current response are already snake_case and graph.js reads them fine). Match the existing convention: **use snake_case** everywhere. JS already accesses `d.file_path`, `d.export_counts.functions`, etc.
**Warning signs:** `edge.edge_type` is `undefined` in JS; `console.log(edge)` shows the field is missing.

### Pitfall 7: Orbital ring symbol positioning relative to transformed canvas
**What goes wrong:** Orbital ring symbol positions are computed in screen space but assigned as simulation `x`/`y` which are in canvas (pre-transform) space.
**Why it happens:** The SVG `g` group has a `transform` applied by the zoom behavior. Node positions are in the `g`'s local coordinate space, not screen space.
**How to avoid:** Compute orbital ring positions in canvas space relative to the parent file node's `d.x` / `d.y`. Do not subtract zoom translate or divide by zoom scale.
**Warning signs:** Expanded symbols appear far from their parent file node; position shifts erratically on zoom.

---

## Code Examples

### Programmatic Zoom: Reset to Identity
```javascript
// Source: D3 docs https://github.com/d3/d3/blob/main/docs/d3-zoom.md
svg.transition().duration(750).call(zoomBehavior.transform, d3.zoomIdentity);
```

### Focus Camera on Specific Node (fly-to)
```javascript
// Source: D3 docs - zoomIdentity.translate + scale
function flyToNode(d) {
  var svgW = +svg.attr('width');
  var svgH = +svg.attr('height');
  var scale = 1.5;
  var tx = svgW / 2 - scale * d.x;
  var ty = svgH / 2 - scale * d.y;
  svg.transition().duration(600).ease(d3.easeCubicInOut)
    .call(zoomBehavior.transform, d3.zoomIdentity.translate(tx, ty).scale(scale));
}
```

### D3 Data Join with Stable Keys (for dynamic node add/remove)
```javascript
// Source: D3 docs https://github.com/d3/d3/blob/main/docs/d3-selection/joining.md
node = nodeGroup.selectAll('circle')
  .data(nodes, d => d.id)  // key function prevents index-based matching
  .join(
    enter => enter.append('circle').attr('r', 0)
      .attr('fill', d => nodeColor(d))
      .call(drag)
      .transition().duration(300).attr('r', d => d.radius),
    update => update,
    exit => exit.transition().duration(200).attr('r', 0).remove()
  );
```

### Rust: Iterating CodeGraph Symbols for Response Construction
```rust
// Source: existing pattern in file_level_projection() in graph_api.rs
for node in graph.graph.node_weights() {
    // node: &SymbolNode — has .id, .name, .kind, .file_path, .is_exported
}

for edge_idx in graph.graph.edge_indices() {
    if let Some((src_idx, tgt_idx)) = graph.graph.edge_endpoints(edge_idx) {
        let kind: &EdgeKind = &graph.graph[edge_idx];
        let src: &SymbolNode = &graph.graph[src_idx];
        let tgt: &SymbolNode = &graph.graph[tgt_idx];
    }
}
```

### serde: Serializing EdgeKind as string
```rust
// Source: [ASSUMED] serde standard pattern; EdgeKind is already Serialize in model.rs
// EdgeKind derives Serialize — serializes as variant name: "Import", "Call", "TypeRef", "ReExport"
// To get lowercase: add #[serde(rename_all = "snake_case")] to EdgeKind enum, or map to String in DTO.
// Recommendation: map to String in SymbolEdgeDto for explicit control.
let edge_type = match kind {
    EdgeKind::Import   => "import",
    EdgeKind::Call     => "call",
    EdgeKind::TypeRef  => "type_ref",
    EdgeKind::ReExport => "re_export",
};
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `d3.event` global | `event` parameter in handler | D3 v6 (2020) | Event must come from function parameter, not global |
| `selection.enter().append()` chained | `selection.join()` | D3 v5 (2018) | join() handles enter/update/exit in one call |
| `zoom.on("zoom", function() { d3.event.transform })` | `zoom.on("zoom", function(event) { event.transform })` | D3 v6 | Already used correctly in current graph.js |

**Deprecated/outdated:**
- `d3.event`: Removed in D3 v6. Current graph.js correctly uses `event` parameter. All new code must use the same pattern.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | History stack pattern (push/slice/cap) | Pattern 3: Navigation History | Low — standard array manipulation, no D3 API dependency |
| A2 | Client-side blast radius BFS using raw edges array | Pattern 4 | Low — confirmed by D-82 decision; BFS is standard |
| A3 | `AppState` extended with pre-computed `DeadCodeResult` (not re-running per request) | Pattern 5 | Medium — implementation may differ; planner should verify CLI wiring in main.rs |
| A4 | snake_case field names for new API DTOs (matching existing convention) | Pitfall 6 | Medium — if existing fields mixed, JS may need camelCase; verify against current graph.js field access |
| A5 | Orbital ring mode positions computed in canvas space (not screen space) | Pitfall 7 | Medium — critical for correct expand mode positioning; testable visually |

---

## Open Questions (RESOLVED)

1. **AppState ownership of CodeGraph vs pre-computed DeadCodeResult**
   - What we know: CLI runs `dead_code()` in `main.rs` once at startup; result is not currently stored anywhere persistent.
   - What's unclear: Should `AppState` hold `Arc<CodeGraph>` (to support future server-side queries) or just `Arc<DeadCodeResult>` (simpler, smaller, already sufficient for D-81)?
   - Recommendation: Store `Arc<DeadCodeResult>` in `AppState` — it's immutable, cheap to clone, and sufficient. Adding full `Arc<CodeGraph>` to `AppState` is Phase 12 territory (multi-repo queries).

2. **Symbol-level adjacency: separate map vs merged map**
   - What we know: Current `adjacency` Map is file-level, undirected. Symbol nodes from expand need their own neighbor lookup for focus mode.
   - What's unclear: Whether to maintain one combined adjacency Map (file + symbol nodes) or two separate maps and query both.
   - Recommendation: Two maps (`fileAdjacency`, `symbolAdjacency`). Merge into a combined lookup at query time. Keeps expand/collapse logic simpler (add/remove from `symbolAdjacency` only).

3. **Stacked list mode: vertical positioning during force simulation**
   - What we know: Stacked list mode (D-70) places symbols in a vertical list below the file node.
   - What's unclear: Whether symbols should have `fx`/`fy` fixed positions (no physics) or be loose nodes with a strong y-force.
   - Recommendation: Use `fx`/`fy` fixed positions for stacked mode; remove `fx`/`fy` when switching to force-integrated mode. This prevents drift in stacked layout.

---

## Environment Availability

Step 2.6: SKIPPED — Phase 5 is purely client-side JS + Rust changes to existing crates. No new external tools, databases, or services are required. D3.js is vendored. Rust toolchain is already in use.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`) |
| Config file | Cargo.toml workspace (no separate config) |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VIZN-03 | Symbol nodes included in API response | unit (Rust) | `cargo test -p cgraph-server enriched_response` | ❌ Wave 0 |
| VIZN-03 | Expand/collapse DOM manipulation | manual visual | — | manual-only |
| VIZN-04 | Fit-to-screen button exists in DOM | smoke | manual visual | manual-only |
| INTR-01 | Search highlights matching nodes | manual visual | — | manual-only |
| INTR-02 | Click-to-focus persists neighbors | manual visual | — | manual-only |
| INTR-03 | Blast radius BFS returns correct set | unit (Rust, existing) | `cargo test -p cgraph-indexer test_blast_radius` | ✅ |
| INTR-03 | Client-side blast radius mirrors server | manual visual | — | manual-only |
| INTR-04 | Dead code flags in API response | unit (Rust) | `cargo test -p cgraph-server dead_code_flags` | ❌ Wave 0 |
| INTR-05 | File filter hides correct nodes | manual visual | — | manual-only |
| INTR-06 | Symbol type filter hides correct nodes | manual visual | — | manual-only |
| INTR-07 | Edge type filter hides correct edges | manual visual | — | manual-only |
| INTR-08 | History back/forward restores focus | manual visual | — | manual-only |

**Justification for manual-only tests:** All browser interaction (DOM transitions, hover/click, canvas animation) requires a real browser environment. No test runner for vanilla JS is in the stack, and adding one is out of scope (D-61: no build step).

### Sampling Rate
- **Per task commit:** `cargo test --workspace`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/server/src/graph_api.rs` — unit test: `EnrichedGraphResponse` includes symbol nodes for all file nodes
- [ ] `crates/server/src/graph_api.rs` — unit test: dead code flags correctly attached to symbol DTOs

*(All browser-side tests are manual visual verification — no infrastructure gaps, just plan for manual walkthrough during wave merge)*

---

## Security Domain

`security_enforcement: true` in config.json. ASVS level 1.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | No auth in this tool (local-only CLI) |
| V3 Session Management | No | No session; stateless client |
| V4 Access Control | No | Single-user local tool |
| V5 Input Validation | Yes | Search input (client-side filter only — no server impact); directory path input in filter panel |
| V6 Cryptography | No | No secrets or crypto in this phase |

### Known Threat Patterns for {stack}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| XSS via node labels displayed from API response | Tampering | D3 `.text()` assignment (not `.html()`) is safe; avoid `.innerHTML` in breadcrumb/tooltip |
| Path traversal via filter input (client-only) | Tampering | Client filter is local string match only; no server request, no file access |
| Prototype pollution via JSON response | Tampering | `Object.assign({}, d)` used in current graph.js is safe; do not use `eval()` on API data |

**Assessment:** ASVS level 1 has no blocking security concerns for this phase. The API is local-only (`127.0.0.1`), stateless, and read-only. The main input validation concern (V5) is XSS in the command palette and breadcrumb rendering — use D3 `.text()` not `.html()` for all user-data-derived text.

---

## Sources

### Primary (HIGH confidence)
- `/d3/d3` (Context7) — zoom behavior, `zoomIdentity`, `transition.call`, `selection.join`, `simulation.nodes`
- D3 v7.9.0 source embedded in `client/d3.v7.min.js` — confirmed version
- `crates/server/src/graph_api.rs` — verified FileGraphResponse structure, AppState, file_level_projection
- `crates/indexer/src/analysis.rs` — verified DeadCodeResult, blast_radius(), dead_code() APIs
- `crates/core/src/model.rs` — verified SymbolNode, EdgeKind (all Serialize)
- `client/graph.js` — verified adjacency Map, hover highlight code, simulation structure
- `client/index.html` — verified panel HTML pattern, section structure, color values
- `crates/cli/src/main.rs` — verified AppState construction, dead_code() call at startup

### Secondary (MEDIUM confidence)
- D3 docs context7 snippets for `simulation.nodes()` restart pattern — cross-verified with existing graph.js usage
- `d3.zoomIdentity.translate().scale()` pattern — cross-verified with context7 D3 zoom docs

### Tertiary (LOW confidence)
- None — all critical claims verified via codebase inspection or Context7

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries confirmed in codebase; no new dependencies
- Architecture: HIGH — all integration points verified by reading actual source files
- Pitfalls: MEDIUM — D3 pitfalls (keying, event model) verified via Context7; JS state management pitfalls from direct code analysis
- Rust API extension: HIGH — existing pattern in graph_api.rs; serde derive chain verified

**Research date:** 2026-05-03
**Valid until:** 2026-06-03 (stable: no external dependencies that can change)
