# Phase 4: HTTP Server & Browser Shell - Pattern Map

**Mapped:** 2026-05-02
**Files analyzed:** 8 new/modified files
**Analogs found:** 5 / 8

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/server/Cargo.toml` | config | — | `crates/cli/Cargo.toml` | exact (same crate-manifest pattern) |
| `crates/server/src/lib.rs` | service/entry | request-response | `crates/indexer/src/lib.rs` | role-match (module re-export gateway) |
| `crates/server/src/graph_api.rs` | service | request-response | `crates/indexer/src/analysis.rs` | role-match (graph transform + structs) |
| `crates/server/src/static_assets.rs` | middleware | request-response | none | no analog |
| `crates/server/tests/server_test.rs` | test | request-response | `crates/cli/tests/cli_smoke.rs` | role-match (integration test structure) |
| `crates/cli/src/main.rs` | controller | request-response | self (extension) | exact (modify existing file) |
| `client/index.html` | component | request-response | none | no analog |
| `client/graph.js` | component | event-driven | none | no analog |

---

## Pattern Assignments

### `crates/server/Cargo.toml` (config)

**Analog:** `crates/cli/Cargo.toml`

**Crate manifest pattern** (full file):
```toml
[package]
name = "cg"
version.workspace = true
edition.workspace = true

[dependencies]
cgraph-core = { path = "../core" }
cgraph-indexer = { path = "../indexer" }
cgraph-ts-extractor = { path = "../ts-extractor" }
clap = { version = "4.6.1", features = ["derive", "cargo"] }
anyhow = "1.0"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
```

**Apply this template:**
```toml
[package]
name = "cgraph-server"
version.workspace = true
edition.workspace = true

[dependencies]
cgraph-indexer = { path = "../indexer" }
cgraph-core = { path = "../core" }
axum = "0.8.9"
tokio = { version = "1.52.1", features = ["full"] }
rust-embed = "8.11.0"
mime_guess = "2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[dev-dependencies]
# no extra test deps — cargo built-in #[test] suffices
```

**Workspace members update** — `Cargo.toml` (root), add `"crates/server"` to the members list:
```toml
[workspace]
members = [
    "crates/core",
    "crates/cli",
    "crates/ts-extractor",
    "crates/indexer",
    "crates/server",        # <-- add this
]
```

---

### `crates/server/src/lib.rs` (service, request-response)

**Analog:** `crates/indexer/src/lib.rs` (lines 1–8)

**Module re-export gateway pattern** — the lib.rs is a thin public surface:
```rust
// crates/indexer/src/lib.rs — full file
pub mod graph;
pub mod crawl;
pub mod resolve;
pub mod analysis;

pub use graph::CodeGraph;
pub use crawl::{Indexer, IndexerError};
pub use analysis::{DeadCodeResult, DeadCodeEntry, Confidence, CycleResult, blast_radius, transitive_deps, detect_cycles, dead_code};
```

**Apply this pattern:**
```rust
pub mod graph_api;
pub mod static_assets;

pub use graph_api::{FileGraphResponse, file_level_projection};
// create_router is the primary public API consumed by crates/cli/src/main.rs
pub use graph_api::create_router;
pub use graph_api::AppState;
```

---

### `crates/server/src/graph_api.rs` (service, request-response)

**Analog:** `crates/indexer/src/analysis.rs`

**Struct definition pattern** (lines 21–43 of analysis.rs):
```rust
/// A single dead code entry with location and confidence information.
#[derive(Debug, Clone)]
pub struct DeadCodeEntry {
    pub symbol_id: String,
    pub file_path: String,
    pub symbol_name: String,
    pub kind: SymbolKind,
    pub line_start: u32,
    pub line_end: u32,
    pub confidence: Confidence,
}

/// Result of dead code analysis: confirmed dead and suspicious entries.
#[derive(Debug, Default)]
pub struct DeadCodeResult {
    pub confirmed: Vec<DeadCodeEntry>,
    pub suspicious: Vec<DeadCodeEntry>,
}
```

**Serializable response structs — apply this pattern with `serde::Serialize`:**
```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileNode {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub export_counts: ExportCounts,
    pub radius: f32,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct ExportCounts {
    pub functions: u32,
    pub classes: u32,
    pub types: u32,
    pub interfaces: u32,
    pub hooks: u32,
    pub enums: u32,
    pub total: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileEdge {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanStats {
    pub files: usize,
    pub symbols: usize,
    pub edges: usize,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileGraphResponse {
    pub nodes: Vec<FileNode>,
    pub edges: Vec<FileEdge>,
    pub stats: ScanStats,
}
```

**Graph iteration pattern** (lines 182–202 of analysis.rs) — iterating graph nodes:
```rust
for node_idx in graph.graph.node_indices() {
    let node = &graph.graph[node_idx];
    if !node.is_exported { continue; }
    // ... process node
}
```

**Edge iteration pattern** (lines 127–133 of analysis.rs):
```rust
for edge_idx in graph.graph.edge_indices() {
    if let Some((src, tgt)) = graph.graph.edge_endpoints(edge_idx) {
        let src_file = &graph.graph[src].file_path;
        let tgt_file = &graph.graph[tgt].file_path;
        if src_file != tgt_file {
            // deduplicated file-level edge
        }
    }
}
```

**File-level deduplication pattern** (lines 299–323 of analysis.rs) — `detect_cycles` already does the exact same file-level projection. Copy this algorithm:
```rust
// Build file_index: HashMap<String, NodeIndex>
// Use file_graph.update_edge(s, t, ()) to deduplicate file->file edges
let mut file_index: HashMap<String, NodeIndex> = HashMap::new();
for node_idx in graph.graph.node_indices() {
    let node = &graph.graph[node_idx];
    file_index
        .entry(node.file_path.clone())
        .or_insert_with(|| ...);
}
for edge_idx in graph.graph.edge_indices() {
    if let Some((src_idx, tgt_idx)) = graph.graph.edge_endpoints(edge_idx) {
        let src_file = &graph.graph[src_idx].file_path;
        let tgt_file = &graph.graph[tgt_idx].file_path;
        if src_file != tgt_file {
            // record file->file edge (deduplicated via HashSet)
        }
    }
}
```

**SymbolKind matching pattern** (lines 7, 338 of analysis.rs):
```rust
use cgraph_core::SymbolKind;
// Match on kind for export counts:
match node.kind {
    SymbolKind::Function => counts.functions += 1,
    SymbolKind::Class => counts.classes += 1,
    SymbolKind::Type => counts.types += 1,
    SymbolKind::Interface => counts.interfaces += 1,
    SymbolKind::Hook => counts.hooks += 1,
    SymbolKind::Enum => counts.enums += 1,
    SymbolKind::Module => {} // skip — module nodes are not exported symbols
}
```

**Error handling pattern** (D-13 — warn and continue; no Result return from projection):
```rust
// analysis.rs: functions return plain structs, not Result<>
// The projection function follows the same convention:
pub fn file_level_projection(graph: &CodeGraph, stats: ScanStats) -> FileGraphResponse {
    // ... never fails; silently skips edge cases
}
```

**AppState + axum router pattern** — no existing analog, use RESEARCH.md Pattern 2:
```rust
use axum::{extract::State, routing::get, Json, Router};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub file_graph: Arc<FileGraphResponse>,
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

### `crates/server/src/static_assets.rs` (middleware, request-response)

**No analog in codebase.** Use RESEARCH.md Pattern 1 directly:

```rust
use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../client/"]   // relative to crates/server/ manifest directory
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

// Security: reject path traversal (ASVS V5, RESEARCH.md security section)
pub async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    if path.contains("..") || path.starts_with('/') {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }
    StaticFile(path).into_response()
}

pub async fn index_handler() -> impl IntoResponse {
    StaticFile("index.html".to_string())
}
```

---

### `crates/server/tests/server_test.rs` (test, request-response)

**Analog:** `crates/cli/tests/cli_smoke.rs` (full file)

**Integration test structure** (lines 1–16):
```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

/// Return the workspace root (parent of crates/cli).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("crates/cli has no parent")
        .parent()
        .expect("crates has no parent")
        .to_path_buf()
}
```

**Unit test pattern** (lines 19–27 of cli_smoke.rs — `#[test]` with assert):
```rust
#[test]
fn test_name() {
    // arrange: create minimal CodeGraph
    // act: call function under test
    // assert: check output shape
    assert_eq!(nodes.len(), expected);
}
```

**Server test pattern** — use axum's `axum::test` or manual HTTP client. No external test crate needed:
```rust
// Test VIZN-02: /api/graph returns file-level nodes only (not symbol nodes)
#[test]
fn test_graph_api_returns_file_nodes() {
    use cgraph_indexer::CodeGraph;
    use cgraph_core::{SymbolNode, SymbolKind, Language};
    // Build a minimal CodeGraph with 2 files, add symbols, call file_level_projection
    let mut graph = CodeGraph::new();
    // ... add symbols ...
    let stats = ScanStats { files: 2, symbols: 3, edges: 1, elapsed_ms: 0 };
    let response = file_level_projection(&graph, stats);
    assert_eq!(response.nodes.len(), 2);  // one node per file
    assert_eq!(response.edges.len(), 1);
}

// Test VIZN-08: file-level view only (no symbol node IDs leak through)
#[test]
fn test_file_only_projection() {
    // node IDs in response.nodes must be file paths, not "file::symbol" IDs
    // ...
}
```

---

### `crates/cli/src/main.rs` (controller — modification)

**Analog:** Self (existing file, lines 1–89)

**CLI struct extension pattern** (lines 10–27) — add `--no-open` and `--serve` args:
```rust
#[derive(Parser, Debug)]
#[command(version, about = "Code graph visualization — cgraph")]
pub struct Cli {
    /// Path to the project directory to scan
    pub path: PathBuf,

    /// Print verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Print detailed dead code report grouped by file
    #[arg(long)]
    pub dead_code: bool,

    /// Print detailed circular dependency report
    #[arg(long)]
    pub cycles: bool,

    // NEW: add these two
    /// Do not auto-open browser (for headless/CI)
    #[arg(long)]
    pub no_open: bool,
}
```

**Error handling pattern** (lines 32–37) — bail! for fatal errors, eprintln! for soft errors:
```rust
if !cli.path.exists() {
    bail!("Path does not exist: {}", cli.path.display());
}
```

**Timing pattern** (lines 46–49):
```rust
let start = Instant::now();
let code_graph = indexer.index(&cli.path)?;
let elapsed = start.elapsed();
```

**Stats extraction pattern** (lines 51–58):
```rust
println!(
    "cgraph scan: {} files, {} symbols, {} edges ({:.0}ms)",
    code_graph.file_count(),
    code_graph.node_count(),
    code_graph.edge_count(),
    elapsed.as_secs_f64() * 1000.0
);
```

**Server start pattern** — add after line 58 (analysis can remain or be moved to `--analyze` mode):
```rust
// Build ScanStats for the API response
let stats = cgraph_server::graph_api::ScanStats {
    files: code_graph.file_count(),
    symbols: code_graph.node_count(),
    edges: code_graph.edge_count(),
    elapsed_ms: elapsed.as_millis() as u64,
};

// Pre-compute file-level projection
let file_graph = cgraph_server::file_level_projection(&code_graph, stats);
let state = cgraph_server::AppState {
    file_graph: std::sync::Arc::new(file_graph),
};

// Find available port and start server (D-60)
let (port, listener) = cgraph_server::find_available_port(3000).await;
let url = format!("http://localhost:{}", port);
println!("cgraph listening on {} — opening browser...", url);

let router = cgraph_server::create_router(state);
tokio::spawn(axum::serve(listener, router));

// Open browser (D-62) — after bind, before blocking
if !cli.no_open {
    if let Err(e) = webbrowser::open(&url) {
        eprintln!("Could not open browser: {}. Open manually: {}", e, url);
    }
}

// Block until Ctrl-C
tokio::signal::ctrl_c().await?;
```

**async main pattern** — existing `fn main() -> Result<()>` must become:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ... same body, now async
}
```

---

### `client/index.html` (component, request-response)

**No analog in codebase.** Minimal shell per D-61 (vanilla HTML, no framework):

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>cgraph</title>
  <style>
    /* Dark dev-tool aesthetic (D-50) */
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { background: #1a1a2e; color: #e0e0e0; font-family: monospace; }
    #header { height: 40px; /* ... */ }
    #graph { width: 100vw; height: calc(100vh - 40px); }
  </style>
</head>
<body>
  <div id="header">
    <span id="project-name"></span>
    <span id="stats"></span>
  </div>
  <div id="graph"></div>
  <div id="legend-panel"><!-- floating legend --></div>
  <script src="d3.v7.min.js"></script>
  <script src="graph.js"></script>
</body>
</html>
```

---

### `client/graph.js` (component, event-driven)

**No analog in codebase.** Use RESEARCH.md Patterns 4, 5, and pitfall mitigations directly.

**Fetch + pre-settle pattern** (RESEARCH.md Pattern 4, Code Examples section):
```javascript
async function loadAndRender() {
    const data = await fetch('/api/graph').then(r => r.json());
    const { nodes, edges, stats } = data;

    document.getElementById('stats').textContent =
        `${stats.files} files • ${stats.symbols} symbols • ${stats.edges} edges • ${stats.elapsed_ms}ms`;

    const width = window.innerWidth;
    const height = window.innerHeight - 40;

    const simulation = d3.forceSimulation(nodes)
        .force("link", d3.forceLink(edges).id(d => d.id).distance(80))
        .force("charge", d3.forceManyBody().strength(-120))
        .force("center", d3.forceCenter(width / 2, height / 2))
        .force("collide", d3.forceCollide().radius(d => d.radius + 20))
        .stop();

    simulation.tick(300);  // VIZN-07: pre-settle before any render
    renderGraph(svg, nodes, edges);
}
```

**Arrowhead marker + adjusted endpoint** (RESEARCH.md Pattern 5 + Pitfall 1):
```javascript
svg.append("defs").append("marker")
    .attr("id", "arrowhead")
    .attr("viewBox", "-0 -5 10 10")
    .attr("refX", 13)
    .attr("refY", 0)
    .attr("orient", "auto")
    .attr("markerWidth", 8)
    .attr("markerHeight", 8)
  .append("svg:path")
    .attr("d", "M 0,-5 L 10,0 L 0,5")
    .attr("fill", "#555555");

// Adjust edge endpoint to circumference (Pitfall 1 fix)
function adjustedEndpoint(source, target, targetRadius) {
    const dx = target.x - source.x;
    const dy = target.y - source.y;
    const dist = Math.sqrt(dx * dx + dy * dy);
    if (dist === 0) return { x: target.x, y: target.y };
    return {
        x: target.x - (dx / dist) * (targetRadius + 8),
        y: target.y - (dy / dist) * (targetRadius + 8)
    };
}
```

**D3 v7 event parameter note** (RESEARCH.md State of the Art):
```javascript
// D3 v7: event is first parameter in callbacks (NOT d3.event global)
svg.call(d3.zoom().on("zoom", function(event) {
    g.attr("transform", event.transform);
}));
```

---

## Shared Patterns

### Error Handling (D-13: warn and continue)
**Source:** `crates/indexer/src/crawl.rs` lines 56–70; `crates/indexer/src/resolve.rs` lines 49–57
**Apply to:** `graph_api.rs` (projection), `static_assets.rs` (asset lookup), `main.rs` (port loop, browser open)
```rust
// Soft error: eprintln! and continue
match fs::read_to_string(path) {
    Ok(s) => s,
    Err(e) => {
        eprintln!("warn: could not read {}: {}", path.display(), e);
        continue;
    }
};

// Fatal error: bail! (anyhow)
if !cli.path.exists() {
    bail!("Path does not exist: {}", cli.path.display());
}
```

### Serde Derive on Structs
**Source:** `crates/core/src/model.rs` lines 1–50 (all public types derive Serialize/Deserialize)
**Apply to:** All new response structs in `graph_api.rs`
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MyType { /* fields */ }
```

### Module-level `use` import ordering
**Source:** `crates/indexer/src/analysis.rs` lines 1–8; `crates/indexer/src/crawl.rs` lines 1–6
**Apply to:** All new Rust files
```rust
// Order: std, external crates, workspace crates, local modules
use std::collections::{HashMap, HashSet};
use petgraph::...;
use cgraph_core::{SymbolKind};
use crate::graph::CodeGraph;
```

### `#[cfg(test)] mod tests` placement
**Source:** `crates/indexer/src/graph.rs` lines 87–155; `crates/indexer/src/analysis.rs` lines 335–end
**Apply to:** `graph_api.rs` (unit tests for projection logic inline)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use cgraph_core::{Language, SymbolKind, EdgeKind};

    fn make_node(id: &str, file_path: &str, ...) -> SymbolNode { ... }

    #[test]
    fn test_something() {
        let mut graph = CodeGraph::new();
        // arrange, act, assert
    }
}
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/server/src/static_assets.rs` | middleware | request-response | No HTTP server exists yet; no asset-serving pattern in codebase |
| `client/index.html` | component | request-response | No browser client exists yet |
| `client/graph.js` | component | event-driven | No D3/JS code exists in project; use RESEARCH.md Patterns 4–5 |

---

## Metadata

**Analog search scope:** `crates/cli/`, `crates/indexer/`, `crates/core/`, `crates/ts-extractor/`
**Files scanned:** 18 Rust source files
**Key insights:**
- `crates/indexer/src/analysis.rs` `detect_cycles()` already performs file-level graph projection (petgraph edge iteration, file deduplication via `update_edge`) — `file_level_projection()` in `graph_api.rs` should copy this exact algorithm structure
- All model types in `cgraph_core` derive `Serialize` — response types in `graph_api.rs` follow the same derive pattern
- The `main.rs` `fn main() -> Result<()>` must become `async fn main()` with `#[tokio::main]` — this is the only structural change to existing code; all new logic appends after the existing analysis block
- Workspace `Cargo.toml` uses `version.workspace = true` and `edition.workspace = true` — new crate must follow the same pattern
**Pattern extraction date:** 2026-05-02
