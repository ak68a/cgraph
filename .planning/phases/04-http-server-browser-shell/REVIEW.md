---
phase: 04-http-server-browser-shell
reviewed: 2026-05-02T21:30:00Z
depth: deep
files_reviewed: 6
files_reviewed_list:
  - crates/server/src/graph_api.rs
  - crates/server/src/static_assets.rs
  - crates/server/src/lib.rs
  - crates/cli/src/main.rs
  - client/graph.js
  - client/index.html
findings:
  critical: 1
  warning: 4
  info: 3
  total: 8
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-05-02T21:30:00Z
**Depth:** deep
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Phase 4 adds an axum HTTP server with embedded static assets and a D3.js force graph browser client. The implementation is well-structured: file-level projection logic is clean, path traversal has defense-in-depth protection, and the TOCTOU-free port binding pattern (reusing the listener) is good. However, there is one critical arithmetic overflow bug in port discovery, and several warnings around silent server failure, an invisible UI toggle, and stale halo rendering.

## Critical Issues

### CR-01: Port discovery panics on u16 overflow (debug) or loops infinitely (release)

**File:** `crates/server/src/graph_api.rs:255`
**Issue:** `find_available_port` increments `port: u16` unconditionally via `port + 1`. When `port == 65535` (u16::MAX), this expression panics in debug builds (Rust's default overflow check) or wraps to 0 in release builds, creating an infinite loop through invalid/reserved ports. The loop has no upper-bound termination condition.
**Fix:**
```rust
pub async fn find_available_port(start: u16) -> Result<(u16, tokio::net::TcpListener), std::io::Error> {
    for port in start..=65535 {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            Ok(listener) => return Ok((port, listener)),
            Err(_) => {
                if port < 65535 {
                    eprintln!("Port {} in use, trying {}...", port, port + 1);
                }
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrNotAvailable,
        format!("No available port found in range {}..65535", start),
    ))
}
```
This also changes the return type to `Result` so the caller can handle the "no port available" case gracefully instead of looping forever. The call site in `main.rs:120` must be updated to handle the error.

## Warnings

### WR-01: Server failure is silently swallowed; user sees stale "listening" message

**File:** `crates/cli/src/main.rs:124-138`
**Issue:** The server runs inside `tokio::spawn`. If it fails immediately after spawning (e.g., the listener is somehow invalid, or a runtime error occurs), the error is only printed to stderr via `eprintln`. Meanwhile, the main thread unconditionally prints "cgraph listening on ..." and opens the browser. The user sees what appears to be a healthy running server, but HTTP requests fail silently in the browser (showing the error-state div).
**Fix:** Add a short delay or health-check after spawning the server to catch immediate failures before telling the user the server is ready. Alternatively, use a oneshot channel to signal readiness:
```rust
let (tx, rx) = tokio::sync::oneshot::channel();
tokio::spawn(async move {
    // Signal that the server has started accepting connections
    let _ = tx.send(());
    if let Err(e) = cgraph_server::serve(listener, state).await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
});
// Wait for the server to be ready before opening the browser
let _ = rx.await;
```

### WR-02: Panel toggle button is permanently hidden; users cannot collapse the settings panel

**File:** `client/index.html:154`
**Issue:** The panel toggle button has `style="display:none"` inline, and no JavaScript code ever removes this inline style or sets `display` to another value. The button is invisible and non-interactive. Meanwhile, the panel itself starts visible (no `collapsed` class). This means the settings panel is always visible and cannot be collapsed or toggled. The `initPanel()` function in `graph.js:6-22` sets up the toggle click handler, but the handler is unreachable because the button cannot be clicked.
**Fix:** Remove the inline `style="display:none"` from the button:
```html
<button id="panel-toggle" class="open" title="Toggle settings">
```

### WR-03: Directory halos become stale after node drag or simulation restart

**File:** `client/graph.js:316-343`
**Issue:** When the "Dir halos" toggle is enabled, convex hull paths are computed once from current node positions and appended as static SVG `<path>` elements. When users drag nodes, adjust force parameters (which restarts the simulation), or the layout otherwise changes, the halos do not update -- they remain frozen at the positions where they were originally computed. This creates a visually incorrect display where halos no longer enclose their directory's nodes.
**Fix:** Either re-render halos on each simulation tick (performance cost) or listen for `dragend` and force-change events to recompute. A pragmatic approach: add a `renderHalos()` function called from the simulation's `tick` handler when halos are active:
```javascript
function renderHalos() {
    if (!halosGroup) return;
    halosGroup.selectAll('path').remove();
    // ... (same hull computation logic, re-run with current positions)
}
// In the tick handler:
simulation.on('tick', function() { updatePositions(); renderHalos(); });
```

### WR-04: Orphan filter hides nodes but not their edges

**File:** `client/graph.js:254-264`
**Issue:** When the "Orphans" toggle is unchecked, orphan nodes and labels are hidden via `display: none`. However, edges connected to orphan nodes are not hidden. Since orphan nodes by definition have no edges in the adjacency map (which is built from file-level edges), this is correct in theory. But consider the edge case: if the adjacency map is empty for a node that DOES have edges (due to the adjacency map being built after simulation tick modifies source/target references), edges to that node would remain visible as dangling lines. The current implementation appears safe because adjacency is built after simulation, but the filter logic is fragile -- it relies on the adjacency map being consistent with the edge data rather than checking edges directly.
**Fix:** For robustness, also filter edge visibility when hiding orphans:
```javascript
link.style('display', function(e) {
    if (show) return null;
    var si = typeof e.source === 'object' ? e.source.id : e.source;
    var ti = typeof e.target === 'object' ? e.target.id : e.target;
    var sOrphan = (adjacency.get(si) || new Set()).size === 0;
    var tOrphan = (adjacency.get(ti) || new Set()).size === 0;
    return (sOrphan || tOrphan) ? 'none' : null;
});
```

## Info

### IN-01: Unused variables `allNodes` and `allEdges`

**File:** `client/graph.js:77-78`
**Issue:** `allNodes` and `allEdges` are assigned but never referenced anywhere else in the file. This is dead code that suggests an incomplete feature (possibly intended for filtering/reset functionality).
**Fix:** Remove the dead assignments:
```javascript
// Delete these lines:
var allNodes = nodes;
var allEdges = edges;
```

### IN-02: `truncate_str` silently truncates without visual indicator

**File:** `crates/server/src/graph_api.rs:240-244`
**Issue:** When a filename exceeds 20 characters, it is silently truncated with no ellipsis or other visual indicator. Two files whose names share the same first 20 characters would display identically despite being different nodes. While the D3 tooltip shows the full path on hover, the graph labels can be misleading.
**Fix:** Append an ellipsis when truncation occurs:
```rust
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 1).collect();
        format!("{}~", truncated)
    }
}
```

### IN-03: `compute_radius` returns 8.0 for 0-export files (same visual weight as 1-export)

**File:** `crates/server/src/graph_api.rs:235-237`
**Issue:** The spec states "8px (1 export) to 24px (20+ exports)". The formula `8.0 + (total / 20.0 * 16.0).min(16.0)` yields 8.0 for 0 exports and 8.8 for 1 export. Files with zero exports (internal-only modules) are visually indistinguishable from files with 1 export. This is a minor deviation from spec intent.
**Fix:** Consider clamping the minimum to 1:
```rust
fn compute_radius(total_exports: u32) -> f32 {
    let t = total_exports.max(1) as f32;
    8.0 + (t / 20.0 * 16.0).min(16.0)
}
```
Or, if 0-export files should appear smaller, use a distinct minimum like 6.0.

---

_Reviewed: 2026-05-02T21:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
