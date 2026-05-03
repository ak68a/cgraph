# Phase 5: Graph Interaction - Pattern Map

**Mapped:** 2026-05-03
**Files analyzed:** 4 (2 modified heavily, 2 modified lightly)
**Analogs found:** 4 / 4

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `client/graph.js` | component (JS module) | event-driven | `client/graph.js` itself (Phase 4) | exact — Phase 5 extends in place |
| `client/index.html` | config/view | request-response | `client/index.html` itself (Phase 4) | exact — Phase 5 extends in place |
| `crates/server/src/graph_api.rs` | service + handler | request-response | `crates/server/src/graph_api.rs` itself (Phase 4) | exact — Phase 5 extends in place |
| `crates/cli/src/main.rs` | config/wiring | batch | `crates/cli/src/main.rs` itself (Phase 4) | exact — Phase 5 extends AppState wiring |

---

## Pattern Assignments

### `client/graph.js` (component, event-driven)

**Analog:** `client/graph.js` (existing file, lines 1–346 — fully read above)

This file is extended in-place. All new interaction modules (NodeExpander, SearchBar,
CommandPalette, NavHistory, BlastRadiusOverlay, DeadCodeOverlay, FilterPanel,
QuickFilterPills, FitToScreenBtn) are added as named functions inside the same
`loadAndRender()` closure scope. No build step, no modules — per D-61.

---

#### Panel initialization pattern (lines 6–22)

Existing collapsible section wiring in `initPanel()`. New Analysis section and
extended Filters section follow the same `.section-header` click → toggle `.chevron.open`
+ `.section-body.open` pattern:

```javascript
document.querySelectorAll('.section-header').forEach(function(hdr) {
    hdr.addEventListener('click', function(e) {
        if (e.target.closest('.section-actions')) return;
        var arrow = hdr.querySelector('.chevron');
        var body = hdr.nextElementSibling;
        if (arrow) arrow.classList.toggle('open');
        body.classList.toggle('open');
    });
});
```

No changes needed to `initPanel()` — new sections declared in HTML are picked up
automatically by `querySelectorAll('.section-header')`.

---

#### API fetch and error handling pattern (lines 24–43)

All Phase 5 data (symbol nodes, dead code flags, typed edges) arrives in the same
`/api/graph` fetch. No new fetch calls needed. The `data` object simply gains new
fields (`data.symbols`, `data.edges[n].edge_type`, dead code flags on symbol DTOs).

```javascript
async function loadAndRender() {
    var data;
    try {
        var response = await fetch('/api/graph');
        if (!response.ok) throw new Error('HTTP ' + response.status);
        data = await response.json();
    } catch (err) {
        document.getElementById('error-message').textContent =
            'Failed to load graph data. Check that the cgraph server is running.';
        document.getElementById('error-state').style.display = 'block';
        return;
    }
    // ... data.nodes, data.edges, data.symbols, data.stats, data.project_name
}
```

---

#### Simulation + D3 data join pattern (lines 81–123)

**Copy for NodeExpander.** When expanding a file node, push symbol nodes into `nodes`
and parent-child edges into `edges`, call `simulation.nodes(nodes)` +
`simulation.force('link').links(edges)`, then rejoin DOM using key function `d => d.id`.
The existing join in graph.js (lines 115–117) uses `.join('circle')` shorthand — Phase 5
must upgrade to the three-callback form to apply drag to entering nodes:

```javascript
// Current (lines 114-117) — must be upgraded to three-callback join for expand:
var nodeGroup = g.append('g').attr('class', 'nodes');
var node = nodeGroup.selectAll('circle').data(nodes).join('circle')
    .attr('r', function(d) { return d.radius; })
    .attr('fill', '#555').attr('stroke', 'none').style('cursor', 'grab');
```

Upgrade template for NodeExpander rejoin (copy this pattern):

```javascript
node = nodeGroup.selectAll('circle')
    .data(nodes, function(d) { return d.id; })  // stable key — never index-based
    .join(
        function(enter) {
            return enter.append('circle')
                .attr('r', 0)
                .attr('fill', function(d) { return nodeColor(d); })
                .attr('stroke', 'none')
                .style('cursor', 'grab')
                .call(drag)  // drag MUST be applied to entering nodes
                .transition().duration(300).attr('r', function(d) { return d.radius; });
        },
        function(update) { return update; },
        function(exit) {
            return exit.transition().duration(200).attr('r', 0).remove();
        }
    );
simulation.alpha(0.3).restart();
```

---

#### Hover highlight / focus mode pattern (lines 163–232)

**Copy for FocusMode.** Focus mode is identical to hover — same D3 transitions — except
it persists on click rather than clearing on `mouseleave`. Gate hover handlers with a
`focusActive` boolean:

```javascript
var hoverActive = false;
var focusActive = false;       // Phase 5 addition
var FADE_IN = 250, FADE_OUT = 400;

node.on('mouseenter', function(event, d) {
    if (focusActive) return;   // gate: skip hover when focus is locked
    hoverActive = true;
    var connected = adjacency.get(d.id) || new Set();

    node.transition().duration(FADE_IN).ease(d3.easeCubicOut)
        .attr('fill', function(n) {
            if (n.id === d.id) return '#7f6df2';
            if (connected.has(n.id)) return '#a882ff';
            return '#555';
        })
        .style('opacity', function(n) {
            return (n.id === d.id || connected.has(n.id)) ? 1 : 0.12;
        });
    // ... labels and link transitions (lines 181-202) unchanged
});

node.on('mouseleave', function() {
    if (focusActive) return;   // gate: skip hover clear when focus is locked
    hoverActive = false;
    node.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
        .attr('fill', '#555').style('opacity', 1);
    // ...
});
```

Click-to-focus entry point (new, inside `loadAndRender()`):

```javascript
node.on('click', function(event, d) {
    event.stopPropagation();
    if (focusActive && focusedNodeId === d.id) {
        clearFocus();
        return;
    }
    activateFocus(d);
});

svg.on('click', function() { if (focusActive) clearFocus(); });

document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape' && focusActive) clearFocus();
});
```

---

#### Panel filter pattern (lines 237–261)

**Copy for FilterPanel.** All Phase 5 filter controls follow the same `addEventListener`
+ direct D3 node/label/link style mutation pattern. Source of truth is a `filterState`
object (not DOM state) to avoid bidirectional sync ping-pong:

```javascript
// Existing pattern (lines 245-249) — file search filter:
document.getElementById('search-files').addEventListener('input', function(e) {
    var q = e.target.value.toLowerCase();
    node.style('opacity', function(d) {
        return !q || d.path.toLowerCase().includes(q) ? 1 : 0.1;
    });
    labels.style('opacity', function(d) {
        return !q || d.path.toLowerCase().includes(q) ? 1 : 0.05;
    });
});
```

Phase 5 filter pattern (AND logic via `filterState`):

```javascript
var filterState = {
    dirQuery: '',
    symbolTypes: { Function: true, Class: true, Type: true, Hook: true, Enum: true },
    edgeTypes: { import: true, call: true, type_ref: true }
};

function applyFilters() {
    // Update node visibility (opacity + pointer-events, not display:none)
    node.style('opacity', function(d) {
        return isNodeVisible(d) ? 1 : 0.08;
    }).style('pointer-events', function(d) {
        return isNodeVisible(d) ? null : 'none';
    });
    link.style('opacity', function(e) {
        return isEdgeVisible(e) ? 0.25 : 0;
    }).style('pointer-events', function(e) {
        return isEdgeVisible(e) ? null : 'none';
    });
}

// Both pill and checkbox handlers write to filterState then call applyFilters():
document.getElementById('filter-functions').addEventListener('change', function() {
    filterState.symbolTypes.Function = this.checked;
    document.getElementById('pill-functions').classList.toggle('active', this.checked);
    applyFilters();
});
```

---

#### Settings panel simulation control pattern (lines 286–312)

**Copy for NodeExpander expand mode dropdown.** Live mode switching follows the same
`addEventListener` + `simulation.alpha(0.5).restart()` pattern:

```javascript
// Existing pattern (lines 286-292):
document.getElementById('slider-center').addEventListener('input', function() {
    var v = parseFloat(this.value);
    simulation.force('center', d3.forceCenter(width / 2, height / 2).strength(v));
    simulation.force('x', d3.forceX(width / 2).strength(v / 2));
    simulation.force('y', d3.forceY(height / 2).strength(v / 2));
    simulation.alpha(0.5).restart();
});

// Expand mode dropdown follows same structure:
document.getElementById('expand-mode').addEventListener('change', function() {
    expandMode = this.value; // 'orbital' | 'force' | 'stacked'
    // Re-expand any currently expanded nodes in new mode
    rebuildExpanded();
});
```

---

#### Drag behavior pattern (lines 143–155)

**Copy for NodeExpander.** New symbol nodes must receive the same drag handler. The drag
object (`var drag = ...`) is already in scope inside `loadAndRender()`. The entering
nodes in the join callback call `.call(drag)` — no other changes needed.

```javascript
var drag = d3.drag()
    .on('start', function(event, d) {
        if (!event.active) simulation.alphaTarget(0.3).restart();
        d.fx = d.x; d.fy = d.y;
        d3.select(this).style('cursor', 'grabbing');
    })
    .on('drag', function(event, d) { d.fx = event.x; d.fy = event.y; })
    .on('end', function(event, d) {
        if (!event.active) simulation.alphaTarget(0);
        d.fx = null; d.fy = null;  // release unless stacked mode (stacked keeps fx/fy)
        d3.select(this).style('cursor', 'grab');
    });
```

---

### `client/index.html` (config/view, request-response)

**Analog:** `client/index.html` (existing file, lines 1–246 — fully read above)

HTML extended in-place. All new elements copy existing patterns exactly.

---

#### Header bar pattern (lines 143–146)

Current header is a flex row with `justify-content: space-between`. Phase 5 adds
search input, back/forward buttons, breadcrumb, and quick-filter pills as new flex
children. New children must fit within the 40px height.

```html
<!-- Current (lines 143-146): -->
<div id="header">
  <span id="project-name"></span>
  <span id="stats"></span>
</div>

<!-- Phase 5 extended structure: -->
<div id="header">
  <div id="header-left">
    <button id="btn-back" title="Back">&#8592;</button>
    <button id="btn-forward" title="Forward">&#8594;</button>
    <span id="project-name"></span>
  </div>
  <div id="header-center">
    <!-- breadcrumb trail here -->
    <div id="breadcrumb"></div>
    <!-- quick-filter pills here -->
    <div id="quick-filters">
      <button class="pill" data-filter="functions">Functions</button>
      <button class="pill" data-filter="classes">Classes</button>
      <button class="pill" data-filter="imports">Imports</button>
      <button class="pill" data-filter="calls">Calls</button>
    </div>
  </div>
  <div id="header-right">
    <input type="text" id="header-search" placeholder="Search..." />
    <span id="stats"></span>
  </div>
</div>
```

---

#### Panel section pattern (lines 158–233)

New Analysis section follows the existing `.panel-section` > `.section-header` >
`.section-body` structure. The chevron SVG and toggle-switch components are copied
verbatim. Analysis section goes between Filters and Display (D-76):

```html
<!-- Existing section pattern (lines 179-206 — Display section): -->
<div class="panel-section">
  <div class="section-header" data-section="display">
    <svg class="chevron" width="16" height="16" viewBox="0 0 24 24" fill="none"
         stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="9 18 15 12 9 6"/>
    </svg>
    <span class="section-title">Display</span>
  </div>
  <div class="section-body" id="sec-display">
    <div class="ctrl-row">
      <span class="ctrl-label">Arrows</span>
      <label class="toggle-switch">
        <input type="checkbox" id="toggle-arrows" checked>
        <span class="toggle-track"></span>
      </label>
    </div>
    <!-- more ctrl-rows... -->
  </div>
</div>

<!-- Phase 5 Analysis section — copy the same structure: -->
<div class="panel-section">
  <div class="section-header" data-section="analysis">
    <svg class="chevron" width="16" height="16" viewBox="0 0 24 24" fill="none"
         stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="9 18 15 12 9 6"/>
    </svg>
    <span class="section-title">Analysis</span>
  </div>
  <div class="section-body" id="sec-analysis">
    <div class="ctrl-row">
      <span class="ctrl-label">Dead code</span>
      <label class="toggle-switch">
        <input type="checkbox" id="toggle-dead-code">
        <span class="toggle-track"></span>
      </label>
    </div>
    <div class="ctrl-row">
      <span class="ctrl-label">Blast radius</span>
      <label class="toggle-switch">
        <input type="checkbox" id="toggle-blast-radius">
        <span class="toggle-track"></span>
      </label>
    </div>
  </div>
</div>
```

Expand mode dropdown goes in the Display section (`id="sec-display"`), after the
existing slider controls, as a `ctrl-row` with a `<select>` instead of a toggle:

```html
<div class="ctrl-row">
  <span class="ctrl-label">Expand mode</span>
  <select id="expand-mode" style="background:#333;border:1px solid #444;
    color:#ddd;font-size:11px;border-radius:4px;padding:2px 4px;">
    <option value="orbital">Orbital</option>
    <option value="force">Force</option>
    <option value="stacked">Stacked</option>
  </select>
</div>
```

---

#### Tooltip pattern (lines 148–152)

Dead code badge tooltip and command palette overlay follow the same fixed-position,
`pointer-events: none`, `z-index: 1000` pattern as the existing tooltip:

```html
<!-- Existing tooltip (lines 148-152): -->
<div id="tooltip">
  <div class="tooltip-path"></div>
  <div class="tooltip-exports"></div>
  <div class="tooltip-edges"></div>
</div>

<!-- Command palette overlay — same CSS base, higher z-index: -->
<div id="command-palette" style="display:none; position:fixed; top:20%; left:50%;
     transform:translateX(-50%); width:480px; background:#2a2a2a;
     border:1px solid #3a3a3a; border-radius:12px; z-index:1001;
     box-shadow:0 8px 32px rgba(0,0,0,0.6); padding:0;">
  <input type="text" id="palette-input" placeholder="Go to symbol..."
         style="width:100%;padding:12px 16px;background:transparent;
         border:none;border-bottom:1px solid #3a3a3a;color:#ddd;
         font-size:14px;outline:none;">
  <div id="palette-results" style="max-height:320px;overflow-y:auto;"></div>
</div>
```

---

#### SVG badge group pattern

Dead code badges need a separate `<g class="badges">` group appended after
`<g class="nodes">` in the D3-built SVG to guarantee correct Z-order (see Pitfall 2).
This is DOM-built by JS, not HTML — no HTML change needed. Pattern from halosGroup
(lines 313–340 in graph.js):

```javascript
// Existing halo group pattern (lines 313-314) — same insert approach:
halosGroup = g.insert('g', '.edges').attr('class', 'halos');

// Phase 5 badges group — appended AFTER nodes group:
var badgeGroup = g.append('g').attr('class', 'badges');
```

---

### `crates/server/src/graph_api.rs` (service + handler, request-response)

**Analog:** `crates/server/src/graph_api.rs` (existing file — fully read above)

---

#### Derive + struct pattern (lines 17–74)

All new DTOs copy the same derive chain and doc comment style:

```rust
// Existing pattern (lines 32-47):
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    /// Equals file_path — used as node ID in the graph.
    pub id: String,
    pub path: String,
    pub filename: String,
    pub export_counts: ExportCounts,
    pub radius: f32,
    pub incoming: usize,
    pub outgoing: usize,
}

// Phase 5 new DTOs — copy the same derive chain:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNodeDto {
    pub id: String,
    pub name: String,
    pub kind: String,         // snake_case string: "function"|"class"|"type"|"hook"|"enum"
    pub file_path: String,
    pub is_dead_code: bool,
    pub dead_code_confidence: Option<String>, // "confirmed" | "suspicious"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String,    // "import" | "call" | "type_ref" | "re_export"
}
```

**Convention:** All field names use snake_case matching existing fields (`file_path`,
`export_counts`, `elapsed_ms`). JS in graph.js already accesses `d.file_path` etc.
Do NOT add `#[serde(rename_all = "camelCase")]` — match the existing convention.

---

#### Response envelope extension pattern (lines 68–82)

`FileGraphResponse` is upgraded to `EnrichedGraphResponse` by adding `symbols` and
upgrading `edges` from `Vec<FileEdge>` to `Vec<TypedEdge>`:

```rust
// Existing envelope (lines 68-74):
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGraphResponse {
    pub nodes: Vec<FileNode>,
    pub edges: Vec<FileEdge>,
    pub stats: ScanStats,
    pub project_name: String,
}

// Phase 5 replacement:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedGraphResponse {
    pub nodes: Vec<FileNode>,          // unchanged: file-level nodes
    pub edges: Vec<TypedEdge>,         // upgraded: now carries edge_type
    pub symbols: Vec<SymbolNodeDto>,   // new: symbol-level data for expand
    pub stats: ScanStats,              // unchanged
    pub project_name: String,          // unchanged
}
```

---

#### AppState pattern (lines 79–82)

```rust
// Existing (lines 79-82):
#[derive(Clone)]
pub struct AppState {
    pub file_graph: Arc<FileGraphResponse>,
}

// Phase 5 — field rename only, type changes:
#[derive(Clone)]
pub struct AppState {
    pub file_graph: Arc<EnrichedGraphResponse>,
}
// Note: Arc<DeadCodeResult> is NOT added to AppState — dead code flags are
// pre-computed into SymbolNodeDto.is_dead_code in main.rs and bundled into
// EnrichedGraphResponse at startup. No per-request analysis. (D-81, research A3)
```

---

#### Projection function pattern (lines 91–232)

`file_level_projection()` is the direct analog for the new `enriched_projection()`
function. Same signature shape: takes `&CodeGraph`, `ScanStats`, `String`,
optionally `&DeadCodeResult`; returns new response type.

Key loops to copy from `file_level_projection()`:

Node iteration (lines 99–135):
```rust
for node in graph.graph.node_weights() {
    // node: &SymbolNode — has .id, .name, .kind, .file_path, .is_exported
}
```

Edge iteration (lines 178–186):
```rust
for edge_idx in graph.graph.edge_indices() {
    if let Some((src_idx, tgt_idx)) = graph.graph.edge_endpoints(edge_idx) {
        let src_file = graph.graph[src_idx].file_path.clone();
        let tgt_file = graph.graph[tgt_idx].file_path.clone();
        // Phase 5: also capture edge kind
        let edge_kind = &graph.graph[edge_idx]; // &EdgeKind
    }
}
```

EdgeKind → string mapping (copy from research Pattern 5):
```rust
let edge_type = match edge_kind {
    EdgeKind::Import   => "import",
    EdgeKind::Call     => "call",
    EdgeKind::TypeRef  => "type_ref",
    EdgeKind::ReExport => "re_export",
};
```

---

#### Handler pattern (lines 268–270)

```rust
// Existing (lines 268-270):
pub async fn graph_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json((*state.file_graph).clone())
}
// Phase 5: no change to handler body — just the type behind the Arc changes.
```

---

#### Test helper pattern (lines 298–323)

New unit tests for `EnrichedGraphResponse` copy the `make_symbol` + `dummy_stats`
helper pattern from the existing test module:

```rust
// Existing helpers (lines 298-323):
fn make_symbol(id: &str, file_path: &str, kind: SymbolKind, is_exported: bool) -> SymbolNode {
    SymbolNode {
        id: id.to_string(),
        name: id.split("::").last().unwrap_or(id).to_string(),
        kind,
        file_path: file_path.to_string(),
        language: Language::TypeScript,
        line_start: 1,
        line_end: 10,
        is_exported,
    }
}

fn dummy_stats() -> ScanStats {
    ScanStats { files: 0, symbols: 0, edges: 0, elapsed_ms: 0 }
}
```

---

### `crates/cli/src/main.rs` (config/wiring, batch)

**Analog:** `crates/cli/src/main.rs` (existing file — fully read above)

---

#### AppState construction pattern (lines 105–117)

Phase 5 passes `&dead_result` into the projection function. The existing
`dead_result` variable is already computed at line 69 before `AppState` is built.
Only the `AppState` construction block changes:

```rust
// Existing (lines 106-117):
let stats = ScanStats {
    files: code_graph.file_count(),
    symbols: code_graph.node_count(),
    edges: code_graph.edge_count(),
    elapsed_ms: elapsed.as_millis() as u64,
};
let file_graph = file_level_projection(&code_graph, stats, project_name);
let state = AppState {
    file_graph: Arc::new(file_graph),
};

// Phase 5 replacement — call enriched_projection instead:
let stats = ScanStats {
    files: code_graph.file_count(),
    symbols: code_graph.node_count(),
    edges: code_graph.edge_count(),
    elapsed_ms: elapsed.as_millis() as u64,
};
let file_graph = enriched_projection(&code_graph, &dead_result, stats, project_name);
let state = AppState {
    file_graph: Arc::new(file_graph),
};
// dead_result is already in scope from line 69:
//   let dead_result = dead_code(&code_graph, &cli.path);
```

---

## Shared Patterns

### D3 Transition Pattern
**Source:** `client/graph.js` lines 166–232 (hover highlight)
**Apply to:** FocusMode, BlastRadiusOverlay, DeadCodeOverlay, SearchBar highlight

All overlay modes use the same D3 transition structure: `node.transition().duration(N).ease(d3.easeCubicOut)` setting `.attr('fill', ...)` and `.style('opacity', ...)`. Link transitions set `.attr('stroke', ...)` and `.attr('stroke-opacity', ...)`. Transition durations: `FADE_IN = 250`, `FADE_OUT = 400` (existing constants, reuse them).

The adjacency lookup pattern is:
```javascript
var connected = adjacency.get(d.id) || new Set();
// ... then test: n.id === d.id || connected.has(n.id)
```

### Serde DTO Pattern
**Source:** `crates/server/src/graph_api.rs` lines 17–74
**Apply to:** All new Rust DTOs (`SymbolNodeDto`, `TypedEdge`, `EnrichedGraphResponse`)

Always use `#[derive(Debug, Clone, Serialize, Deserialize)]`. Use snake_case field names
(not camelCase) to match the existing convention (`file_path`, `export_counts`,
`elapsed_ms`). JavaScript in graph.js already accesses snake_case fields.

### Simulation Restart Pattern
**Source:** `client/graph.js` lines 273–311 (panel control handlers)
**Apply to:** NodeExpander, expand mode switching

Any change that affects node positions or forces calls `simulation.alpha(0.3).restart()`
for minor changes (node add) or `simulation.alpha(0.5).restart()` for major changes
(force parameter change). Never mutate the nodes array while the simulation is running —
always call `simulation.nodes(newNodes)` first.

### Panel Control Event Handler Pattern
**Source:** `client/graph.js` lines 237–312 (Filters + Display + Forces sections)
**Apply to:** All new Analysis section handlers, FilterPanel handlers, QuickFilterPills

All handlers: `document.getElementById('id').addEventListener('change'/'input', function() { ... })`. No jQuery, no event delegation for panel controls. Write to a shared state object (`filterState`, `analysisState`) then call a render function — never read sibling DOM state directly.

### Rust Test Helper Pattern
**Source:** `crates/server/src/graph_api.rs` lines 298–323 and `crates/indexer/src/analysis.rs` lines 341–352
**Apply to:** New unit tests for `EnrichedGraphResponse` (Wave 0 gaps from RESEARCH.md)

Both `graph_api.rs` and `analysis.rs` use the same `make_symbol`/`make_node` helper
pattern with `Language::TypeScript` as the default. Copy this pattern for new test
functions; do not create a new helper if one already exists in the same test module.

---

## No Analog Found

All files in Phase 5 are extensions of existing files. No genuinely new files are
introduced — all interaction code is added inline to `graph.js` and `index.html`,
and all Rust changes are in-place modifications to `graph_api.rs` and `main.rs`.

The following Phase 5 capabilities have **no existing codebase analog** for their
specific interaction pattern (planner should use RESEARCH.md patterns directly):

| Capability | File | Reason |
|------------|------|--------|
| CommandPalette (Cmd+K overlay) | `client/graph.js` | No existing modal/overlay JS in codebase |
| Navigation history stack | `client/graph.js` | No existing history management in codebase |
| Orbital ring expand mode | `client/graph.js` | No existing dynamic node add/remove; research Pattern 1 is the reference |
| Fit-to-screen zoom | `client/graph.js` | Zoom behavior exists but no programmatic fit; research Pattern 2 is the reference |
| Client-side BFS blast radius | `client/graph.js` | No existing graph traversal in JS; research Pattern 4 is the reference |

---

## Metadata

**Analog search scope:** `client/`, `crates/server/src/`, `crates/indexer/src/`, `crates/core/src/`, `crates/cli/src/`
**Files read:** 6 source files (graph.js, index.html, graph_api.rs, analysis.rs, model.rs, main.rs)
**Pattern extraction date:** 2026-05-03
