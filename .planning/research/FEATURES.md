# Feature Landscape: Code Graph Visualization Tools

**Domain:** Static analysis + interactive code graph visualization (CLI-delivered, browser-rendered)
**Researched:** 2026-05-02
**Reference tools:** Sourcegraph, Dependency Cruiser, Madge/Skott, Code Maat, SciTools Understand, VS Code Call Hierarchy, AppMap, Sourcetrail (archived)

---

## Table Stakes

Features users expect from any code graph tool. Missing = product feels incomplete or broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Go to definition / symbol jump** | VS Code has trained every developer to expect click-to-navigate on any symbol | Low | Click a node, open file at that symbol's declaration |
| **Find all references (usages)** | Second most basic navigation act after go-to-def; VS Code / IntelliJ normalize this | Low | Show all call sites / import sites for a selected symbol |
| **Circular dependency detection** | Madge, dependency-cruiser, Skott all surface this; developers ask for it constantly | Low | Highlight cycle edges in red; call them out in a summary list |
| **Dead code / orphan detection** | Knip, ts-prune, Madge orphan flag, Skott — the #1 reason developers reach for static graph tools | Medium | Highlight nodes with zero incoming edges (unexported or unused exports) |
| **Symbol search / fuzzy find** | Sourcetrail's fuzzy search was universally praised; without it a graph of >50 nodes is unusable | Low | Type to filter; highlight matching nodes; clear on Escape |
| **Zoom and pan** | Force graphs are unusable without spatial navigation; every graph tool ships this | Low | Mouse wheel zoom, drag-to-pan, fit-to-screen button |
| **Node click = focus** | Sourcetrail's core pattern: click a symbol, graph recenters around it and its direct neighbors | Low | Replaces the overwhelming "show everything" default view |
| **Edge directionality** | Users need to know "imports" vs "is imported by" — arrows are mandatory | Low | Directed edges with arrowheads; distinguish import direction clearly |
| **File-level aggregation** | Symbol-level graphs hairball fast; file-level is the readable default for medium codebases | Low | Show files as nodes, collapse internal symbols; toggle to symbol level |
| **Filter by file / directory** | Dependency-cruiser, Skott, Nx graph all ship regex/path filters | Low | Include/exclude by path pattern; persist across sessions |
| **Incremental / watch mode** | Skott ships watch mode natively; Sourcetrail's re-index was its biggest complaint when slow | Medium | Re-analyze only changed files; update graph in-place without full reload |
| **Performance on medium codebases** | Skott is 7x faster than Madge; developers abandon tools that take >5s on a 1k-file project | Medium | Target: initial scan <3s for 500-file TS project; incremental <500ms per file |

---

## Differentiators

Features that separate the good tools from the forgotten ones. Not universally expected, but high signal for quality.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Blast radius view** | Click a symbol → highlight all transitive dependents (callers, importers); answers "what breaks if I change this?" | Medium | Topological traversal upward; color-code by hop count; the core pain from OversizeConnect's `_seconds/_nanoseconds` problem |
| **Dependency depth view** | Show how deep in the import chain a symbol lives; surface unnaturally deep chains | Medium | Compute longest path from root; color nodes by depth level |
| **Symbol type visual encoding** | Sourcetrail used distinct icons/colors per symbol type (function, class, interface, type alias, file) | Low | Color = symbol type; shape = optional distinction; helps scanning dense graphs |
| **Edge type visual encoding** | Distinguish "calls", "imports", "extends", "implements", "re-exports" edges visually | Medium | Different line styles or colors per edge type; hover tooltip shows relationship |
| **Multi-language single graph** | Madge is JS-only; Sourcetrail supported C/C++/Java/Python but not Swift/Go; no tool spans TS+Swift+Go+Python | High | Tree-sitter extractors per language feeding one shared graph = unique differentiator |
| **Usage count / hotspot coloring** | Show how often a symbol is referenced; dark = heavily used, light = rarely used; inspired by Code Maat hotspots | Low | Node color gradient based on in-degree (reference count) |
| **Pinned focus + neighbor expansion** | Sourcetrail's killer pattern: pin a node, expand neighbors one hop at a time | Low | "Show more connections" button per node; prevents graph explosion |
| **Keyboard-first navigation** | Sourcetrail supported WASD panning; developers keep hands on keyboard | Low | Arrow keys for pan, +/- for zoom, / for search, Escape to clear focus |
| **Persistent graph state via URL hash** | Encode focused symbol + filter state in URL so developers can share exact views with teammates | Low | Serialize graph viewport + active filters to URL params; decode on load |
| **Session history (back/forward)** | Sourcetrail shipped browser-style back/forward for graph navigation; crucial for exploration workflows | Low | Track navigation events; back/forward buttons + keyboard shortcuts |
| **Metrics overlay (optional)** | SciTools Understand's treemap showed complexity, line count, churn; overlay this on graph nodes | High | Optional toggle; do not force it on by default — complexity bloat risk |
| **Export snapshot** | Dependency-cruiser exports PNG/SVG/DOT; useful for docs and team discussions | Low | Render current canvas to PNG or SVG; download button |

---

## Anti-Features

Things that seem like good ideas but consistently hurt UX or add complexity without payoff. Explicitly do not build these in v1.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Show-all default view** | The #1 hairball problem. Every tool that renders the full graph by default gets abandoned. A 500-file TS project with symbol-level nodes produces 2000+ nodes — unusable. | Default to file-level graph; show only top-level entry points; user drills down |
| **Persistent sidebar with symbol tree** | IntelliJ, Understand, and Sourcetrail all shipped these; they duplicate information already in the graph and consume screen space that should be graph | Use the graph as the navigation surface; search to find, click to explore |
| **Fully configurable rules / lint mode** | Dependency-cruiser is incredibly powerful but the `.dependency-cruiserrc` learning curve kills adoption. Architecture enforcement is a separate tool job | Detect obvious violations (circular deps, dead code) without user configuration |
| **CI / report generation mode** | Dependency-cruiser, Code Maat are designed for CI pipelines. This requires different UX, different output formats, and different trust model | Focus on interactive exploration; CLI is for starting the server, not piping reports |
| **3D force graph** | dep-tree ships 3D; it looks impressive but offers no navigational advantage over 2D and has worse readability at scale | 2D force graph with good layout; third dimension adds motion sickness, not insight |
| **Auto-layout switching** | Offering hierarchical, radial, spring, and circular layouts confuses users. They try all of them, none is "right" for their case | One good force-directed layout with good defaults; file grouping via proximity |
| **Line-of-code metrics overlaid by default** | SciTools Understand's treemap overlays are powerful for architects but overwhelming for the "where is this thing connected" use case | Hide all metrics behind an optional panel; default view is graph structure only |
| **Inline code editing** | AppMap, Sourcegraph both tried to add light editing. It blurs the tool's identity and creates bugs (read-only = simpler mental model) | Read-only; clicking "go to source" opens the file in the user's existing editor |
| **Cross-repo analysis** | Sourcegraph's killer differentiator — but it requires a backend service, auth, indexing infrastructure, and transforms a CLI tool into a SaaS | Out of scope per PROJECT.md; single project directory only |
| **Sequence diagrams / flame graphs** | AppMap ships both; they answer different questions ("how did this execute?") vs this tool's question ("what is connected to what?") | Out of scope; static graph not runtime trace |

---

## Feature Dependencies

```
Symbol search → Node click = focus (search populates the focus subject)

File-level aggregation → Node click = focus (clicking a file node expands to symbol view)

Node click = focus → Blast radius view (blast radius is focus + directional traversal)
Node click = focus → Session history (each focus change is a history entry)
Node click = focus → Persistent URL state (serialize the current focus target)

Dead code detection → Usage count overlay (both require computing in-degree per node)
Dead code detection → Filter by file/directory (users want to scope dead code to one module)

Watch mode → Incremental parsing (watch mode is only useful if re-analysis is fast)

Edge type encoding → Go to definition (edge labels help user know what relationship they are navigating)

Blast radius view → Dependency depth view (depth is the numeric version of blast radius)

Circular dependency detection → Edge directionality (cycles only make sense with directed edges)
```

---

## MVP Recommendation

Prioritize in this order based on the OversizeConnect proving-ground use cases (dead code + blast radius):

1. **Symbol search + node click = focus** — without this the graph is not navigable
2. **File-level aggregation default** — without this the graph is not readable
3. **Dead code detection (zero in-degree nodes)** — validated need from OversizeConnect
4. **Blast radius view** — validated need from OversizeConnect (`_seconds/_nanoseconds` case)
5. **Circular dependency detection** — table stakes; low effort
6. **Incremental watch mode** — makes the tool "always on" not just on-demand
7. **Session history (back/forward)** — low complexity, dramatically improves exploration UX

Defer to post-MVP:
- **Metrics overlay** (usage count as color is OK in MVP; full metrics panel is post-v1)
- **Export snapshot** (nice-to-have; not blocking any core use case)
- **Persistent URL state** (useful for sharing; not blocking solo exploration)
- **Edge type visual encoding** (import edges are enough for v1; call edges, re-exports can come later)
- **Multi-language cross-linking** (each extractor works independently in v1; cross-language edges — e.g., Swift calling into JS bridge — is post-v1)

---

## Sources

- Sourcegraph code navigation docs: https://sourcegraph.com/docs/code-search/code-navigation/features
- dependency-cruiser GitHub: https://github.com/sverweij/dependency-cruiser
- Madge GitHub: https://github.com/pahen/madge
- Skott (Madge successor): https://github.com/antoine-coulon/skott
- Introducing Skott (feature comparison with Madge): https://dev.to/antoinecoulon/introducing-skott-the-new-madge-1bfl
- Sourcetrail (archived): https://github.com/CoatiSoftware/Sourcetrail
- Sourcetrail DeepWiki: https://deepwiki.com/CoatiSoftware/Sourcetrail
- AppMap visualization features: https://appmap.io/docs/reference/guides/using-appmap-diagrams.html
- SciTools Understand features: https://scitools.com/features
- Code Maat (logical coupling/hotspots): https://github.com/adamtornhill/code-maat
- Nx graph explorer: https://nx.dev/docs/features/explore-graph
- Cambridge Intelligence — hairball problem: https://cambridge-intelligence.com/how-to-fix-hairballs/
- Cambridge Intelligence — graph visualization UX: https://cambridge-intelligence.com/graph-visualization-ux-how-to-avoid-wrecking-your-graph-visualization/
- Knip (dead code): https://knip.dev/
- dep-tree (3D force graph): https://github.com/gabotechs/dep-tree
- Blast Radius (Terraform, D3): https://github.com/28mm/blast-radius
