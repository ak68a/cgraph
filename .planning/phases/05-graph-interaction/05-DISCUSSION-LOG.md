# Phase 5: Graph Interaction - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-03
**Phase:** 05-graph-interaction
**Areas discussed:** Node expand/collapse, Search & focus behavior, Overlays & analysis views, Filtering system

---

## Node Expand/Collapse

### Expansion Layout

| Option | Description | Selected |
|--------|-------------|----------|
| Orbital ring | Symbols orbit parent in a fixed-radius ring. Predictable, tidy. | ✓ (all 3) |
| Force-integrated | Symbols join simulation as regular nodes. Organic but unpredictable. | ✓ (all 3) |
| Stacked list | Vertical list below file node, UML-style. Compact but less graph-like. | ✓ (all 3) |

**User's choice:** All three, switchable via settings panel dropdown. User wants to try each during visual verification.
**Notes:** User explicitly requested building all three so they can play with them and see which feels best.

### Symbol Visual Style

| Option | Description | Selected |
|--------|-------------|----------|
| Color by kind | Each symbol type gets a distinct color per D-51 palette | ✓ |
| Shape by kind | Different shapes per type, uniform color | |
| Color + shape | Both distinct colors and shapes | |

**User's choice:** Color by kind.

---

## Search & Focus Behavior

### Search Location

| Option | Description | Selected |
|--------|-------------|----------|
| Header bar search | Always-visible input in header, real-time highlight | ✓ |
| Panel search | Extend existing Filters section search | |
| Command palette | Cmd+K overlay, VS Code-style | ✓ |

**User's choice:** Both header bar search AND command palette. Initially selected options 1 and 2, then clarified they meant options 1 and 3.

### Focus Behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Fade non-neighbors | Same as hover but persistent on click | ✓ |
| Hide non-neighbors | Fully hide non-neighbor nodes | |
| Fade + pull closer | Fade and strengthen forces to cluster | |

**User's choice:** Fade non-neighbors. User noted this already exists as hover behavior — focus just makes it persist on click.

### Navigation History

| Option | Description | Selected |
|--------|-------------|----------|
| Browser-style buttons | Back/forward arrows in header | |
| Breadcrumb trail | Horizontal trail showing path of focused nodes | |
| Both | Arrows plus breadcrumb trail | ✓ |

**User's choice:** Both.

---

## Overlays & Analysis Views

### Overlay Activation

| Option | Description | Selected |
|--------|-------------|----------|
| Panel toggles | Analysis section in settings panel with toggle switches | ✓ |
| Toolbar buttons | Separate floating toolbar with icon buttons | |
| Context menu only | Right-click per-node options only | |

**User's choice:** Panel toggles.

### Dead Code Confidence Display

| Option | Description | Selected |
|--------|-------------|----------|
| Color intensity | Confirmed=solid red/orange, suspicious=lower opacity/dashed | ✓ |
| Badge only | Warning icon badges, node color unchanged | |
| Separate colors | Red for confirmed, yellow/amber for suspicious | |

**User's choice:** Color intensity.

---

## Filtering System

### Filter UI

| Option | Description | Selected |
|--------|-------------|----------|
| Extend settings panel | Add filter controls to existing panel sections | ✓ |
| Separate filter bar | Horizontal bar below header with pill toggles | |
| Both panel + quick filters | Full panel filters plus quick-filter pills in header | ✓ |

**User's choice:** Initially selected "Extend settings panel", then requested quick-filter pills as well. Final: both panel filters and header quick-filter pills.

---

## API Design

| Option | Description | Selected |
|--------|-------------|----------|
| Lazy endpoints | Separate API calls on demand per interaction | |
| Bundle everything upfront | All data in initial /api/graph response | ✓ |
| Hybrid | Dead code bundled, rest lazy | |

**User's choice:** Bundle everything upfront. Rationale: this is a local tool (localhost), so payload size is not a concern. Simplifies client code. Response shape designed for future multi-repo lazy-loading split (D-83).
**Notes:** User raised multi-repo concern (Signum: 5 interrelated repos). Concluded that multi-repo (Phase 12) may need a different strategy, but for Phase 5 single-project scope, bundling is correct.

---

## Claude's Discretion

- Fit-to-screen button placement and behavior
- Keyboard shortcuts beyond Cmd+K
- Animation timing for expand/collapse transitions
- Quick-filter pill selection (which filters get promoted)
- Edge type visual distinction styling

## Deferred Ideas

- Edge type visual distinction (solid/dashed/colored) — could accompany edge type filter
- Keyboard shortcuts — future polish pass
- Canvas rendering for large graphs — v2 (ADVZ-02)
- Lazy API endpoints for multi-repo — Phase 12
