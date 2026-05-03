---
phase: 05-graph-interaction
plan: 03
subsystem: ui
tags: [d3, vanilla-js, expand-collapse, focus-mode, fit-to-screen, node-expander]
dependency_graph:
  requires:
    - 05-01: EnrichedGraphResponse with data.symbols array
    - 05-02: DOM element IDs (#expand-mode, #btn-fit, #focus-hint)
  provides:
    - NodeExpander with orbital/force/stacked modes
    - FocusMode persistent highlight with Escape/background-click exit
    - FitToScreen zoom with F key shortcut
  affects:
    - client/graph.js
tech_stack:
  added: []
  patterns:
    - "D3 three-callback join (enter/update/exit) with stable key function for dynamic node add/remove"
    - "Symbol adjacency precomputation from API symbolEdges at load time"
    - "wireNodeEvents() pattern: re-apply hover/click handlers after simulation rebuild"
    - "focusActive gate on hover handlers to prevent Pitfall 3 conflict"
key_files:
  created: []
  modified:
    - client/graph.js
decisions:
  - "Both Tasks 1 and 2 implemented in single file pass for internal consistency; committed as two logical commits"
  - "hoverActive reset to false and tooltip hidden on activateFocus (Rule 2: missing UX correctness)"
  - "Stacked mode preserves fx/fy on symbol nodes; drag end handler respects expandMode to keep stacked nodes fixed"
  - "symbolEdges prefiltered at load time (:: in id) to avoid re-scanning all edges on every expand"
metrics:
  duration: 4m
  completed: 2026-05-03
  tasks_completed: 2
  files_modified: 1
requirements:
  - VIZN-03
  - VIZN-04
  - INTR-02
---

# Phase 5 Plan 03: NodeExpander, FocusMode, FitToScreen Summary

**One-liner:** Node expand/collapse with orbital/force/stacked modes, persistent click-to-focus highlight with Escape exit, and fit-to-screen zoom via D3 zoomBehavior.transform in client/graph.js.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | NodeExpander with three expand modes, symbol data storage, rebuildSimulation | 5910cbe | client/graph.js |
| 2 | FocusMode persistent highlight and FitToScreen zoom | 5ca3a2b | client/graph.js |

## What Was Built

### NodeExpander (D-70, D-71, D-72)

**`expandFileNode(fileNode)`** — Pushes symbol child nodes into the shared `nodes` array, adds parent-child edges, merges symbol adjacency into the main adjacency map, then calls `rebuildSimulation()`. Symbols come from `symbolsByFile[fileId]` precomputed at load time.

**`collapseFileNode(fileId)`** — Filters symbol nodes and their edges from the arrays, cleans up adjacency entries, calls `rebuildSimulation()`.

**`positionSymbols(fileNode, symbolNodes)`** — Three mode implementations:
- `orbital`: evenly distributed on a ring of radius 40 around parent, no fixed positions
- `stacked`: column below parent, `fx/fy` pinned so simulation doesn't scatter them
- `force`: random jitter near parent, fully simulation-positioned

**`rebuildSimulation()`** — Reassigns `simulation.nodes(nodes)` and `simulation.force('link')`, then rejoins all three D3 selections (circles, text, lines) with stable key functions `d => d.id`. Entering nodes receive the drag handler via `.call(drag)`.

**Data precomputation at load time:**
- `symbolsByFile`: Map of `file_path -> SymbolNodeDto[]` from `data.symbols`
- `symbolAdjacency`: Map of `symbolId -> Set<neighborId>` from symbol-level edges (edges where source or target contains `::`)
- `edges` (initial): file-level only (no `::` in source or target)
- `symbolEdges`: symbol-level edges stored separately for merge on expand

**`NODE_COLORS`** palette (D-72): function=#2dd4bf, class=#f87171, type=#fbbf24, interface=#fbbf24, hook=#a78bfa, enum=#4ade80, file=#555.

### FocusMode (D-74)

**`wireNodeEvents(sel)`** — Extracted event wiring into a function so `rebuildSimulation` can re-apply it to newly entering nodes. Contains mouseenter/mousemove/mouseleave/click. Mouseenter and mouseleave are gated with `if (focusActive) return` to prevent hover state from overwriting focus state.

**`activateFocus(d)`** — Sets `focusActive = true`, fades non-neighbors to 0.1 opacity via D3 transitions, shows `#focus-hint`. Also hides tooltip and resets hoverActive (Rule 2 fix).

**`clearFocus()`** — Resets all nodes to 1 opacity, hides `#focus-hint`, sets `focusActive = false`.

**Exit mechanisms:** `svg.on('click')` calls `clearFocus()` on background click. `document.addEventListener('keydown')` calls `clearFocus()` on Escape.

**Click handler:** Clicking a file node toggles expand/collapse AND activates focus. Clicking a symbol node activates focus only.

### FitToScreen (VIZN-04)

**`fitToScreen()`** — Computes bounding box of all nodes with `d3.min`/`d3.max` over `x ± radius`, applies 48px padding, constrains scale to max 1.0, then calls `svg.transition().call(zoomBehavior.transform, d3.zoomIdentity.translate(tx, ty).scale(scale))`.

**Wired to:**
- `#btn-fit` click event
- `F`/`f` keydown event (guarded: skipped when activeElement is INPUT/TEXTAREA/SELECT)

### Zoom Behavior Reference

The `d3.zoom()` call is now stored as `var zoomBehavior` before `svg.call(zoomBehavior)`. The zoom callback references `focusActive` to gate `updateLabelVisibility`. Both `focusActive` and `zoomBehavior` are declared before the zoom setup.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing UX correctness] Tooltip and hover state not cleared on activateFocus**

- **Found during:** Task 2 implementation
- **Issue:** When a node is clicked, `activateFocus` was called while the tooltip from the preceding `mouseenter` remained visible. `hoverActive` also remained `true`, which could cause incorrect label visibility behavior.
- **Fix:** Added `document.getElementById('tooltip').style.display = 'none'` and `hoverActive = false` inside `activateFocus`.
- **Files modified:** client/graph.js
- **Commit:** 5ca3a2b

**2. [Implementation approach] Both tasks implemented in single file pass**

- **Found during:** Task 1 implementation
- **Situation:** Both tasks modify only `client/graph.js`. Writing the complete coherent file in one pass is more consistent than writing partial code, committing, then adding more. The Task 2 commit (5ca3a2b) represents the incremental addition of the tooltip/hoverActive fix.
- **Impact:** None — all acceptance criteria for both tasks are met.

## Known Stubs

None — all expand/collapse, focus, and fit-to-screen features are fully wired to real data.

## Threat Surface Scan

The plan's threat model (T-05-04) specifies that all text must be rendered via D3 `.text()` and `textContent` (no `.innerHTML`). Verified:

- Symbol name labels: `enter.append('text').text(function(d) { return d._isSymbol ? d.name : d.filename; })` — uses `.text()`.
- Tooltip fields: all use `.textContent =` — no innerHTML.
- No new network endpoints, auth paths, or trust boundaries introduced.

## Self-Check: PASSED

- FOUND: client/graph.js
- FOUND: .planning/phases/05-graph-interaction/05-03-SUMMARY.md
- FOUND: commit 5910cbe (Task 1 — NodeExpander)
- FOUND: commit 5ca3a2b (Task 2 — FocusMode + FitToScreen)
