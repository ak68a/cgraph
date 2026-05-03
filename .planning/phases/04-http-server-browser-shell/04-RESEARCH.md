# Phase 4: HTTP Server & Browser Shell - Research

**Researched:** 2026-05-02
**Domain:** Rust HTTP server (axum), embedded static assets (rust-embed), D3.js force-directed graph
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-50:** Dark dev-tool aesthetic. Background #1a1a2e (dark navy). Nodes bright against dark. Edges subtle gray with opacity. Text white/light gray.
- **D-51:** Semantic color palette. Phase 4: file nodes only (#4a9eff steel blue). Full palette for Phase 5: functions=#2dd4bf, classes=#f87171, types/interfaces=#fbbf24, hooks=#a78bfa, enums=#4ade80.
- **D-52:** Edges uniform in Phase 4 — all #555 gray, 0.4 opacity, with arrowheads. Edge type distinction deferred to Phase 5.
- **D-53:** File nodes: filled circles, radius scales linearly 8px (1 export) to 24px (20+ exports). Formula: `r = 8 + (exports / 20) * 16`, capped at 24px.
- **D-54:** Labels show filename only, truncated at 20 chars. Duplicate filenames in different dirs disambiguated with parent dir prefix. Full relative path on hover.
- **D-55:** Hover tooltip: full relative path, export summary by kind, edge counts (incoming/outgoing). No interactive elements.
- **D-56:** Two layout modes togglable: (1) Natural force clustering (default), (2) Directory halos — dashed boundary, low-opacity background by directory. Default: halos OFF. Toggle in floating panel.
- **D-57:** Medium density — no overlap, whole project visible without excessive zoom. Tuned for nighthawk (92 files) and OversizeConnect (419 files).
- **D-58:** Minimal header: thin top bar, project name left, stats right ("{N} files • {N} symbols • {N} edges • {N}ms"). Graph fills remaining viewport.
- **D-59:** Floating legend/control panel, bottom-right corner, collapsible. Contains: color key, halos toggle.
- **D-60:** axum HTTP server. Embedded static assets via rust-embed or include_str. Single `/api/graph` JSON endpoint. Default port 3000, increment if taken.
- **D-61:** Vanilla HTML + JS + D3.js. No build step, no framework, no TypeScript. Single index.html with inline or co-located JS. Embedded in binary.
- **D-62:** Auto-open browser: macOS `open`, Linux `xdg-open`. Graceful fallback: print URL if open fails. Support `--no-open` flag.

### Claude's Discretion

None explicitly listed — the above decisions cover all Phase 4 scope.

### Deferred Ideas (OUT OF SCOPE)

- Edge type visual distinction (solid/dashed/colored) — Phase 5
- Dark/light mode toggle — post-v1 polish
- Canvas rendering for >2000 nodes — v2 (ADVZ-02)
- Keyboard shortcuts — Phase 5
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| VIZN-01 | Graph renders as D3 force-directed layout in the browser | D3 v7 CDN, forceSimulation API, SVG rendering patterns documented |
| VIZN-02 | Default view shows file-level nodes only (not individual symbols) | File-level projection from CodeGraph — group SymbolNodes by file_path, aggregate edges between files |
| VIZN-05 | Edges show directionality via arrowheads | SVG `<defs>/<marker>` pattern with `marker-end` attribute; variable refX per target node radius |
| VIZN-06 | Nodes are color-coded by symbol type | D-51 palette locked; Phase 4 is single type (file nodes = #4a9eff) |
| VIZN-07 | Force simulation pre-settles before rendering (no jitter on load) | `simulation.stop()` + `simulation.tick(300)` pattern confirmed by d3js.org docs |
| VIZN-08 | Graph uses progressive disclosure (files → exports → internals) | Phase 4 implements file layer only; architecture supports symbol expansion in Phase 5 |
| INFR-02 | Tool starts localhost HTTP server and auto-opens browser | axum + tokio::net::TcpListener port loop; webbrowser crate or std::process::Command |
</phase_requirements>

---

## Summary

Phase 4 adds two new concerns to the existing Rust workspace: an HTTP server crate (`crates/server`) and a browser client (vanilla HTML/JS/D3). The server serves the client as embedded static assets and exposes a single JSON endpoint that returns a file-level graph projection derived from the `CodeGraph` built in Phase 3. The CLI gains a default-serve mode — after indexing, it starts the server, opens a browser tab, and blocks until interrupted.

The heaviest implementation decisions are already locked by the CONTEXT.md. The remaining technical unknowns are: (1) the exact rust-embed + axum 0.8 integration pattern, (2) the D3 pre-settle tick approach, and (3) the arrowhead-to-variable-radius positioning problem for directed edges. All three have verified solutions documented below.

The file-level projection is a new serialization layer that lives in `crates/server` — it consumes the `CodeGraph` (symbol-level graph) and produces a JSON structure with one node per file and deduplicated edges between files. This is the only non-trivial algorithmic work: grouping symbol edges into file edges and counting export kinds for tooltip data.

**Primary recommendation:** Create `crates/server` with axum 0.8 + rust-embed 8, embed `client/` assets at compile time, expose `GET /api/graph` returning file-level JSON, and serve `index.html` as the catch-all route.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| File-level graph projection (data transform) | API / Backend (Rust) | — | CodeGraph lives in Rust; projection is a server-side reduction, not a client concern |
| HTTP routing and static asset serving | API / Backend (Rust) | — | axum in `crates/server` owns the transport layer |
| Force simulation (layout computation) | Browser / Client | — | D3's forceSimulation runs in JS; pre-settling with tick(300) keeps it client-side |
| SVG rendering (nodes, edges, arrowheads) | Browser / Client | — | D3 manipulates the DOM; pure client responsibility |
| Port discovery and browser auto-open | CLI | — | Orchestration logic belongs in `crates/cli/src/main.rs` |
| Hover tooltip | Browser / Client | — | DOM event listeners; no server round-trip needed |
| Directory halos (convex hull rendering) | Browser / Client | — | d3-polygon convex hull; purely presentational |
| Scan stats (files, symbols, edges, ms) | CLI → API | Browser / Client | Stats computed during indexing, passed to server state, returned in `/api/graph` response |

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.8.9 | HTTP server, routing, JSON responses | Locked by D-60 and CLAUDE.md; ergonomic, tower-based, production Rust standard |
| tokio | 1.52.1 | Async runtime | Required by axum; already used in workspace |
| rust-embed | 8.11.0 | Embed `client/` assets in binary at compile time | Locked by D-60; single binary distribution requires embedded assets |
| serde_json | 1.0 | JSON serialization of graph API response | Already in workspace; derives on model types already exist |
| D3.js | 7.9.0 (CDN) | Force-directed graph rendering in browser | Locked by D-61; vanilla JS, no npm build |

[VERIFIED: npm registry — D3 v7.9.0 is current stable]
[VERIFIED: cargo search — axum 0.8.9, tokio 1.52.1, rust-embed 8.11.0]

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| mime_guess | 2.x | MIME type detection for static asset responses | Required by rust-embed custom handler pattern |
| webbrowser | 1.2.1 | Cross-platform browser open (macOS/Linux/Windows) | Use instead of raw `std::process::Command` for reliability; handles all platforms including WSL edge cases |
| tower-http | (axum transitive) | CORS middleware if needed later | Phase 5 interaction may need CORS; already available transitively |

[VERIFIED: cargo search — webbrowser 1.2.1, mime_guess available in rust-embed example]

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rust-embed | `include_str!` per file | include_str requires one macro call per file; rust-embed handles entire directory, MIME, and dev/release modes |
| rust-embed | tower-http ServeDir | ServeDir reads from filesystem at runtime — breaks single-binary requirement (D-61) |
| webbrowser crate | `std::process::Command` manually | Manual approach requires per-platform detection (macOS/Linux/Windows/WSL); webbrowser handles this correctly |
| D3 v7 via CDN | Bundled in binary | CDN approach needs internet; for developer tools, embedding D3 in the binary is cleaner. Embedding ~550KB D3 UMD as part of the static assets (via rust-embed) is preferred |

**Installation (Cargo.toml additions):**

```toml
# crates/server/Cargo.toml
[dependencies]
axum = "0.8.9"
tokio = { version = "1.52.1", features = ["full"] }
rust-embed = "8.11.0"
mime_guess = "2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
cgraph-indexer = { path = "../indexer" }
cgraph-core = { path = "../core" }

# crates/cli/Cargo.toml additions
webbrowser = "1.2.1"
cgraph-server = { path = "../server" }
```

**Version verification:** Confirmed via `cargo search` on 2026-05-02. [VERIFIED: cargo search]

---

## Architecture Patterns

### System Architecture Diagram

```
cg <path>
    |
    v
[crates/cli/src/main.rs]
    |  1. Validate path
    |  2. Run Indexer::index() -> CodeGraph
    |  3. Extract stats (files, symbols, edges, elapsed_ms)
    |  4. Find available port (3000, 3001, ...)
    |  5. Spawn axum server with Arc<AppState>
    |  6. Print URL to stdout
    |  7. Open browser (webbrowser::open or --no-open)
    |  8. axum::serve().await (blocks until Ctrl-C)
    |
    v
[crates/server] — tokio + axum
    |
    +-- GET /           -> serve embedded index.html
    +-- GET /api/graph  -> file_graph_handler(State<AppState>) -> Json<FileGraphResponse>
    +-- GET /*path      -> static asset handler (JS, CSS, D3 bundle)
    |
    v
[AppState] (Arc, read-only after construction)
    |
    +-- file_graph: FileGraphResponse  (pre-computed from CodeGraph at startup)
    +-- scan_stats: ScanStats

[FileGraphResponse] (computed once, served repeatedly)
    |
    +-- nodes: Vec<FileNode>     { id, path, filename, export_counts, radius }
    +-- edges: Vec<FileEdge>     { source_file, target_file }  (deduplicated)
    +-- stats: ScanStats         { files, symbols, edges, elapsed_ms }

[browser: client/index.html + client/graph.js]
    |
    1. fetch('/api/graph') -> FileGraphResponse JSON
    2. d3.forceSimulation(nodes).stop() + tick(300) -> settled positions
    3. render SVG (circles, edge lines with arrowheads, labels)
    4. attach zoom, hover tooltip, legend toggle handlers
```

### Recommended Project Structure

```
crates/
  server/
    Cargo.toml
    src/
      lib.rs           — pub fn create_router(), pub struct AppState, pub fn file_level_projection()
      graph_api.rs     — GET /api/graph handler; FileGraphResponse, FileNode, FileEdge types
      static_assets.rs — rust-embed Assets struct, static file handler
  cli/
    src/
      main.rs          — extended: port loop, webbrowser::open, axum::serve

client/                — embedded at compile time by rust-embed
  index.html           — minimal shell: <div id="graph">, loads graph.js
  graph.js             — D3 force simulation, render, zoom, tooltip, legend
  d3.v7.min.js         — D3 bundled locally (no CDN dependency at runtime)
```

---

### Pattern 1: rust-embed + axum Static File Handler

**What:** Embed entire `client/` directory at compile time; serve files with correct MIME types via axum handler.
**When to use:** Single-binary distribution requirement (D-61).

```rust
// Source: https://docs.rs/crate/rust-embed/latest/source/examples/axum.rs
use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../client/"]   // relative to crates/server/
struct ClientAssets;

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();
        match ClientAssets::get(path.as_str()) {
            Some(content) => {
                let mime = mime_guess::from_path(&path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            }
            None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        }
    }
}

async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    StaticFile(path)
}

async fn index_handler() -> impl IntoResponse {
    StaticFile("index.html".to_string())
}
```

---

### Pattern 2: axum Router with AppState

**What:** Share pre-computed `FileGraphResponse` across requests via `Arc<AppState>`.
**When to use:** The graph is computed once at startup; all `/api/graph` requests serve the same cached response.

```rust
// Source: https://docs.rs/axum/0.8.8/axum/extract/struct.State.html
use axum::{extract::State, routing::get, Json, Router};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub file_graph: Arc<FileGraphResponse>,  // pre-computed, immutable
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/graph", get(graph_handler))
        .route("/*path", get(static_handler))
        .with_state(state)
}

async fn graph_handler(State(state): State<AppState>) -> Json<Arc<FileGraphResponse>> {
    Json(state.file_graph.clone())
}
```

---

### Pattern 3: Port Discovery Loop

**What:** Try ports 3000, 3001, 3002... until a TcpListener binds successfully.
**When to use:** D-60 requirement; avoids "port in use" errors.

```rust
// Source: docs.rust-lang.org/std/net/struct.TcpListener.html (pattern)
// [VERIFIED: WebSearch + Rust stdlib docs]
async fn find_available_port(start: u16) -> (u16, tokio::net::TcpListener) {
    for port in start.. {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            Ok(listener) => return (port, listener),
            Err(_) => {
                eprintln!("Port {} in use, trying {}...", port, port + 1);
            }
        }
    }
    panic!("No available port found");
}
```

---

### Pattern 4: D3 Pre-Settled Force Simulation

**What:** Run simulation.tick(300) synchronously before rendering any SVG. Zero live animation on load.
**When to use:** VIZN-07 — graph must not jitter when page loads.

```javascript
// Source: https://d3js.org/d3-force/simulation
// [VERIFIED: d3js.org official docs — "natural number of ticks is 300 by default"]

const simulation = d3.forceSimulation(nodes)
    .force("link", d3.forceLink(links).id(d => d.id).distance(80))
    .force("charge", d3.forceManyBody().strength(-120))
    .force("center", d3.forceCenter(width / 2, height / 2))
    .force("collide", d3.forceCollide().radius(d => d.radius + 20))
    .stop();  // do NOT start the automatic timer

// Synchronously advance to settled state
simulation.tick(300);

// Now render the static graph — positions are final
renderGraph(nodes, links);
```

**Key insight:** `simulation.stop()` before any ticks prevents the automatic async timer. `simulation.tick(300)` drives the physics to completion synchronously. Tick events are NOT dispatched in this path — only the `"tick"` event handler would be called by the internal timer (which is stopped). [VERIFIED: d3js.org]

---

### Pattern 5: SVG Arrowhead Marker

**What:** Define an SVG `<marker>` in `<defs>` and reference it via `marker-end` on edge `<line>` elements.
**When to use:** VIZN-05 — directionality of dependency edges.

```javascript
// Source: https://gist.github.com/fancellu/2c782394602a93921faff74e594d1bb1
// [VERIFIED: multiple WebSearch sources confirm this pattern]

// Define marker in SVG defs
svg.append("defs").append("marker")
    .attr("id", "arrowhead")
    .attr("viewBox", "-0 -5 10 10")
    .attr("refX", 13)      // offset from line endpoint — must account for node radius
    .attr("refY", 0)
    .attr("orient", "auto")
    .attr("markerWidth", 8)
    .attr("markerHeight", 8)
  .append("svg:path")
    .attr("d", "M 0,-5 L 10,0 L 0,5")
    .attr("fill", "#555555");

// Apply to edge lines
const link = svg.append("g")
    .selectAll("line")
    .data(links)
    .join("line")
    .attr("stroke", "#555555")
    .attr("stroke-opacity", 0.4)
    .attr("marker-end", "url(#arrowhead)");

// After pre-settle tick, set static positions:
link
    .attr("x1", d => d.source.x)
    .attr("y1", d => d.source.y)
    .attr("x2", d => d.target.x)
    .attr("y2", d => d.target.y);
```

---

### Pattern 6: File-Level Graph Projection

**What:** Reduce the symbol-level `CodeGraph` into a file-node/file-edge structure for the browser API.
**When to use:** VIZN-02 — default view shows file nodes only.

```rust
// [ASSUMED — derived from CodeGraph API in graph.rs; no external source]
use std::collections::{HashMap, HashSet};
use cgraph_core::{SymbolKind};
use cgraph_indexer::CodeGraph;

#[derive(serde::Serialize)]
pub struct FileNode {
    pub id: String,          // == file_path (used as D3 node id)
    pub path: String,        // relative path from project root
    pub filename: String,    // basename or "parent/basename" if duplicate
    pub export_counts: ExportCounts,
    pub radius: f32,         // pre-computed: 8 + (total_exports / 20.0 * 16.0).min(16.0)
}

#[derive(serde::Serialize, Default)]
pub struct ExportCounts {
    pub functions: u32,
    pub classes: u32,
    pub types: u32,
    pub interfaces: u32,
    pub hooks: u32,
    pub enums: u32,
    pub total: u32,
}

#[derive(serde::Serialize)]
pub struct FileEdge {
    pub source: String,  // file_path of source file
    pub target: String,  // file_path of target file
}

pub fn file_level_projection(graph: &CodeGraph) -> (Vec<FileNode>, Vec<FileEdge>) {
    // 1. Collect all file paths and their exported symbols
    // 2. Compute export counts per file per kind
    // 3. Compute radius per file (D-53 formula)
    // 4. Detect duplicate basenames and apply parent-dir prefix (D-54)
    // 5. Iterate graph edges, map source/target symbol -> file, deduplicate file->file edges
    // ...
}
```

---

### Anti-Patterns to Avoid

- **Serving assets from filesystem at runtime:** Breaks single-binary distribution (D-61). Never use `tower_http::ServeDir` with a real path in production.
- **Starting the D3 simulation without `.stop()` first:** Results in animated jitter on page load (violates VIZN-07). Always call `.stop()` before constructing the simulation, or call it immediately after.
- **Using `std::thread::sleep` to "wait for the server to start" before opening the browser:** Fragile. Instead, open the browser immediately after the `axum::serve` task is spawned (not awaited).
- **Mutating AppState after construction:** The graph is computed once and read-only. Use `Arc<FileGraphResponse>` directly — no `Mutex` needed.
- **Putting file-level projection logic in `crates/cli`:** It belongs in `crates/server` (consumed by the API handler) or a separate function in `crates/indexer`. The CLI should only orchestrate.
- **Hardcoding `localhost` in browser open URL:** Use `127.0.0.1` in the bind address but `localhost` in the browser URL for user friendliness. The listener should bind `127.0.0.1`, not `0.0.0.0`, to avoid firewall prompts.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-platform browser open | `if cfg!(target_os = "macos")` chain | `webbrowser` crate | Handles macOS, Linux, Windows, and WSL edge cases correctly |
| MIME type detection | Manual HashMap of extensions | `mime_guess` crate | Covers ~1000 MIME types; used in rust-embed official examples |
| Static file embedding | Manual `include_bytes!` per file | `rust-embed` with `#[derive(Embed)]` | Handles entire directories, debug/release toggle, MIME, and ETag |
| D3 force layout algorithm | Custom graph layout | D3 `forceSimulation` | Force-directed layout is a complex physics simulation; D3's is battle-tested at scale |
| Convex hull for directory halos | Polygon math | `d3.polygonHull()` | Built into D3; correct Andrew's monotone chain algorithm [VERIFIED: d3js.org docs] |

**Key insight:** Every listed problem has an existing, battle-tested solution used by hundreds of production projects. Custom implementations introduce bugs in exactly the edge cases (WSL browser detection, obscure MIME types, convex hull degenerate inputs) that users will encounter.

---

## Common Pitfalls

### Pitfall 1: Arrowhead Hidden Behind Target Node

**What goes wrong:** `marker-end` places the arrowhead at the line's mathematical endpoint (the center of the target node). For larger nodes (radius up to 24px), the arrowhead is drawn under the node circle and invisible.

**Why it happens:** `refX` on the marker is a fixed offset from the path endpoint. With variable node radii, one `refX` value cannot work for all node sizes.

**How to avoid:** One of two approaches:
1. **Shorten the line to the node circumference** — compute the angle between source and target, offset the endpoint by `target_radius` pixels in the opposite direction. This is the cleanest solution and the one to use. [VERIFIED: d3-js Google Groups discussion]
2. **Multiple markers** — create a marker per radius value (marker-r8, marker-r12, etc.) with different `refX`. More DOM overhead, less elegant.

**Recommended line endpoint adjustment:**
```javascript
// After tick(300), compute adjusted endpoint for each edge:
function adjustedEndpoint(source, target, targetRadius) {
    const dx = target.x - source.x;
    const dy = target.y - source.y;
    const dist = Math.sqrt(dx * dx + dy * dy);
    if (dist === 0) return { x: target.x, y: target.y };
    return {
        x: target.x - (dx / dist) * (targetRadius + 8),  // +8 for arrowhead length
        y: target.y - (dy / dist) * (targetRadius + 8)
    };
}
```

**Warning signs:** All arrowheads disappear on nodes larger than ~12px radius.

---

### Pitfall 2: Simulation Not Fully Settled at 300 Ticks for Dense Graphs

**What goes wrong:** With 419 nodes and ~880 edges (OversizeConnect), the default 300 ticks may leave residual alpha above `alphaMin` (0.001), resulting in nodes that shift slightly after page load if the simulation is ever restarted.

**Why it happens:** D3's default `alphaDecay = 1 - pow(0.001, 1/300)` = 0.0228 is calibrated for 300 ticks. High `chargeStrength` values or tight collision radii slow convergence.

**How to avoid:** Verify empirically with OversizeConnect. If needed, increase tick count to 500 or reduce `chargeStrength`. The UI-SPEC already specifies `alphaDecay` tuning parameters as starting values. Since we `.stop()` the simulation after ticking and never restart it in Phase 4, residual alpha doesn't cause visual jitter — this only becomes a concern if Phase 5/6 resumes the simulation.

**Warning signs:** Graph feels "loosely packed" or nodes too evenly spread after tick(300) — indicates alphaDecay may be too high.

---

### Pitfall 3: rust-embed Debug vs Release Build Difference

**What goes wrong:** rust-embed embeds files at compile time in **release** mode but reads them from the **filesystem** in **debug** mode. This means `cargo run` works differently than `cargo build --release && ./target/release/cg`.

**Why it happens:** rust-embed's default behavior enables live-reload in debug. The `client/` directory must exist at the path specified in `#[folder = "..."]` relative to the manifest.

**How to avoid:** The `#[folder]` path is relative to the crate's manifest directory (`crates/server/`). Use `../../client/` to reference the workspace-level `client/` directory. Test both `cargo run` (filesystem) and the release binary (embedded).

**Warning signs:** Binary works in dev but assets not found in the distributed binary — path is wrong.

---

### Pitfall 4: Port Already In Use at 3000

**What goes wrong:** If port 3000 is already bound (dev server, another cgraph instance), the server panics instead of retrying.

**Why it happens:** `TcpListener::bind` returns an `Err` which is typically `.unwrap()`-ed.

**How to avoid:** The port loop pattern in Pattern 3 above. Print `"Port 3000 in use, trying 3001..."` to stderr (D-62 behavior), not stdout (to avoid polluting piped output).

**Warning signs:** Panic message containing "address already in use" on startup.

---

### Pitfall 5: Opening Browser Before Server Is Ready

**What goes wrong:** `webbrowser::open("http://localhost:3000")` is called before `axum::serve()` has bound the port. The browser receives a "connection refused" error.

**Why it happens:** `axum::serve()` is an `async fn` that binds on first `.await`. Calling `webbrowser::open` before the first `.await` means the socket isn't listening yet.

**How to avoid:** Use `tokio::spawn` to run the server, then open the browser after the `TcpListener::bind()` succeeds (the bind happens before `axum::serve()` is called). The correct order:

```rust
let (port, listener) = find_available_port(3000).await;
println!("cgraph listening on http://localhost:{} — opening browser...", port);
let url = format!("http://localhost:{}", port);
// Spawn server task
tokio::spawn(axum::serve(listener, router));
// Now open browser — server is already bound
let _ = webbrowser::open(&url);
// Block main thread (or wait for signal)
tokio::signal::ctrl_c().await.unwrap();
```

**Warning signs:** Browser shows "ERR_CONNECTION_REFUSED" immediately on first open.

---

## Code Examples

Verified patterns from official sources:

### axum Complete Server Setup

```rust
// Source: https://docs.rs/axum/0.8.8/axum/fn.serve.html
// [VERIFIED: Context7 /websites/rs_axum_0_8_8_axum]
use axum::{extract::State, routing::get, Json, Router};
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    file_graph: Arc<FileGraphResponse>,
}

pub async fn start_server(
    state: AppState,
    start_port: u16,
) -> Result<(u16, tokio::task::JoinHandle<()>), anyhow::Error> {
    let (port, listener) = find_available_port(start_port).await;
    let router = create_router(state);
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    Ok((port, handle))
}
```

### D3 Force Simulation Full Pattern

```javascript
// Source: https://d3js.org/d3-force/simulation (official D3 docs)
// [VERIFIED: d3js.org]
async function loadAndRender() {
    const data = await fetch('/api/graph').then(r => r.json());
    const { nodes, edges, stats } = data;

    // Update header stats
    document.getElementById('stats').textContent =
        `${stats.files} files • ${stats.symbols} symbols • ${stats.edges} edges • ${stats.elapsed_ms}ms`;

    const width = window.innerWidth;
    const height = window.innerHeight - 40;  // minus header

    // Pre-settle simulation (VIZN-07)
    const simulation = d3.forceSimulation(nodes)
        .force("link", d3.forceLink(edges).id(d => d.id).distance(80))
        .force("charge", d3.forceManyBody().strength(-120))
        .force("center", d3.forceCenter(width / 2, height / 2))
        .force("collide", d3.forceCollide().radius(d => d.radius + 20))
        .stop();

    simulation.tick(300);

    // Render static graph
    renderGraph(svg, nodes, edges);
}
```

### webbrowser Open Pattern

```rust
// Source: https://docs.rs/webbrowser (webbrowser crate docs)
// [VERIFIED: cargo search webbrowser 1.2.1]
fn open_browser(url: &str, no_open: bool) {
    if no_open {
        return;
    }
    if let Err(e) = webbrowser::open(url) {
        eprintln!("Could not open browser: {}. Open manually: {}", e, url);
        println!("cgraph listening on {} — open in browser to view graph", url);
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| D3 v4/v5 `d3.event` global | D3 v7 event parameter in callbacks | D3 v6 (2020) | Zoom handlers use `function(event)` not global `d3.event` |
| axum `Server::bind()` | `axum::serve(listener, router)` | axum 0.6 → 0.7 | New API; old examples show wrong syntax |
| axum `body::Full` / `body::Empty` | `axum::body::Body::from(bytes)` or `Bytes` | axum 0.7 | Many old static file examples use removed types |
| `rust_embed::EmbeddedFile.data` as `Cow<[u8]>` | Still `Cow<'static, [u8]>` in 8.x | — | Unchanged — `content.data.into_owned()` or direct use |

**Deprecated/outdated:**
- D3 v4/v5 drag: `d3.event` inside handlers is removed in v6+. All v6+ examples pass `event` as first callback parameter.
- axum 0.5 / 0.6 `Router::new().route("*path")`: wildcard syntax changed to `"/*path"` (with leading slash) in 0.7+.
- axum-embed crate: Only supports axum ^0.7.x, NOT 0.8. Use the manual rust-embed handler pattern (Pattern 1 above) for axum 0.8.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | File-level projection logic (Pattern 6 code) — algorithm structure assumed | Code Examples | If CodeGraph API changes between Phase 3 and this implementation, the projection code needs adjustment — low risk since graph.rs is stable |
| A2 | D3 UMD bundle can be downloaded and embedded in `client/` at ~550KB | Standard Stack | If bundle size is much larger, may slow binary size; actual size should be verified when downloading |
| A3 | `tokio::spawn(axum::serve(...))` returns a `JoinHandle` the CLI can detach from | Code Examples | If axum::serve signature changes, the spawning pattern may need adjustment — low risk |

---

## Open Questions (RESOLVED)

1. **D3 bundle delivery: CDN vs embedded**
   - What we know: D-61 requires a single binary with zero runtime deps. A CDN link violates this for offline/VPN environments.
   - What's unclear: Should `d3.v7.min.js` be committed to the repo or downloaded as part of `cargo build` (build.rs script)?
   - RESOLVED: Commit `client/d3.v7.min.js` (~550KB minified) to the repo at project creation time. No build.rs complexity needed. This is standard practice for single-binary tools.

2. **`--no-open` flag placement: CLI arg or server arg?**
   - What we know: D-62 specifies `--no-open`. The current CLI only has `--path`, `--verbose`, `--dead-code`, `--cycles`.
   - What's unclear: Should `--no-open` be added to the existing `Cli` struct in `crates/cli`?
   - RESOLVED: Yes — add `--no-open` as a clap arg on the existing `Cli` struct. It belongs in the CLI orchestration layer, not the server.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo | Build | ✓ | 1.93.1 | — |
| node | D3 CDN check only | ✓ | v24.13.0 | Not needed at runtime |
| Rust edition 2024 | Workspace | ✓ | (already set) | — |

**Missing dependencies with no fallback:** None. All runtime dependencies (axum, tokio, rust-embed) are Rust crates pulled at build time.

**Missing dependencies with fallback:** None.

**Note:** `client/` directory does not exist yet — must be created as part of Wave 0. rust-embed will fail to compile if the `#[folder]` path does not exist, even if empty. A stub `client/index.html` must exist before `cargo build` succeeds.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `assert_cmd` for CLI integration tests |
| Config file | None — `cargo test` discovers tests automatically |
| Quick run command | `cargo test -p cg` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VIZN-01 | D3 graph renders without JS errors | manual (browser) | — | — |
| VIZN-02 | `/api/graph` returns file-level nodes (not symbol nodes) | integration | `cargo test -p cgraph-server -- test_graph_api` | ❌ Wave 0 |
| VIZN-05 | Arrowhead markers defined in SVG defs | manual (browser DOM inspection) | — | — |
| VIZN-06 | File nodes have correct color (#4a9eff) | manual (visual) | — | — |
| VIZN-07 | Simulation settled — no alpha-driven animation on load | manual (visual observation) | — | — |
| VIZN-08 | File-level view only (no symbol nodes in response) | integration | `cargo test -p cgraph-server -- test_file_only_projection` | ❌ Wave 0 |
| INFR-02 | Server starts, port is bound, URL printed to stdout | integration | `cargo test -p cg -- test_server_starts` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p cgraph-server` (server unit tests)
- **Per wave merge:** `cargo test` (full suite)
- **Phase gate:** Full suite green + manual browser verification before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/server/src/` — entire new crate (lib.rs, graph_api.rs, static_assets.rs)
- [ ] `crates/server/Cargo.toml` — new crate manifest
- [ ] `crates/server/tests/server_test.rs` — covers VIZN-02, VIZN-08, INFR-02
- [ ] `client/index.html` — stub required for rust-embed compilation
- [ ] `client/graph.js` — D3 visualization code
- [ ] `client/d3.v7.min.js` — D3 bundle (download once, commit)
- [ ] `Cargo.toml` workspace members update — add `"crates/server"`

---

## Security Domain

Security enforcement is enabled (config: `security_enforcement: true`, `security_asvs_level: 1`).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | localhost-only server; no user auth |
| V3 Session Management | No | Stateless API; no sessions |
| V4 Access Control | Partial | Bind to 127.0.0.1 only (not 0.0.0.0) — prevents network exposure |
| V5 Input Validation | Yes | Sanitize file path in static asset handler — prevent path traversal |
| V6 Cryptography | No | No secrets, tokens, or encryption in Phase 4 |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal in static asset handler | Tampering | Reject paths containing `..` or absolute paths before calling `Asset::get()` |
| Server bound on 0.0.0.0 (network-exposed) | Information Disclosure | Bind on `127.0.0.1` only — localhost tools should not be network-accessible |
| Large graph JSON DoS | Denial of Service | Low risk — graph is pre-computed at startup; no per-request computation |

**Path traversal mitigation:**

```rust
// In static_handler — reject traversal attempts
async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    if path.contains("..") || path.starts_with('/') {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }
    StaticFile(path).into_response()
}
```

[VERIFIED: OWASP path traversal guidance; standard Rust pattern]

---

## Sources

### Primary (HIGH confidence)

- Context7 `/websites/rs_axum_0_8_8_axum` — axum Router, State, Json, TcpListener, serve() API
- Context7 `/websites/rs_rust-embed` — Embed derive macro, Asset::get(), EmbeddedFile
- Context7 `/d3/d3` — forceSimulation, forceLink, forceManyBody, forceCenter, forceCollide, zoom, polygonHull
- https://d3js.org/d3-force/simulation — simulation.tick(), simulation.stop(), alphaDecay, alphaMin, 300-tick default
- https://docs.rs/crate/rust-embed/latest/source/examples/axum.rs — StaticFile IntoResponse + handler pattern

### Secondary (MEDIUM confidence)

- https://crates.io/crates/webbrowser — webbrowser 1.2.1 cross-platform browser open
- https://gist.github.com/fancellu/2c782394602a93921faff74e594d1bb1 — D3 arrowhead marker definition and edge path updates
- WebSearch: "Rust find available port TcpListener bind increment loop" — confirmed by stdlib docs

### Tertiary (LOW confidence)

- WebSearch: "D3 force graph SVG marker refX variable node sizes" — arrowhead hidden behind large nodes; solution (shorten line to circumference) cited from multiple Google Groups threads but not official D3 docs

---

## Project Constraints (from CLAUDE.md)

| Directive | Impact on Phase 4 |
|-----------|-------------------|
| Language: Rust | All server code in Rust; no Node.js server-side code |
| Browser: D3.js force graph (embedded static assets) | D3 loaded from embedded file; vanilla HTML/JS only |
| HTTP/WebSocket: axum or actix-web | axum confirmed (D-60); actix is alternative, not used |
| Distribution: single binary (cargo install) | rust-embed required; no runtime filesystem reads for assets |
| Commands: `cargo build`, `cargo run`, `cargo test` | No npm scripts, no Makefile steps for the browser client |

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crate versions verified via `cargo search` on 2026-05-02
- Architecture: HIGH — derived from existing codebase (graph.rs, main.rs) + verified axum patterns
- D3 patterns: HIGH for simulation API (verified d3js.org); MEDIUM for arrowhead variable-radius fix (WebSearch only)
- Pitfalls: MEDIUM — most from WebSearch cross-verified with official docs

**Research date:** 2026-05-02
**Valid until:** 2026-06-01 (axum/D3 are stable; webbrowser crate version may change)
