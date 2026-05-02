# Research Summary — Code Graph

**Researched:** 2026-05-02
**Confidence:** HIGH

## Recommended Stack

| Package | Version | Role |
|---------|---------|------|
| `tree-sitter` | `0.25.1` (exact) | Core parsing runtime |
| `tree-sitter-typescript` | `0.23.2` (exact) | TS/TSX grammar |
| `tree-sitter-go` | `0.25.0` (exact) | Go grammar |
| `tree-sitter-python` | `0.25.0` (exact) | Python grammar |
| `tree-sitter-swift` | `0.7.1` (exact) | Swift grammar (community) |
| `commander` | `^14.0.0` | CLI args |
| `open` | `^9.1.0` | Browser launch (last CJS) |
| `ws` | `^8.20.0` | WebSocket |
| `chokidar` | `^4.0.0` | File watcher |
| `d3` | `^7.9.0` | Force graph (browser only) |
| Node.js `http` | built-in | HTTP server |

**Module system:** CommonJS (native addons require it).

## Table Stakes Features

- Symbol search + click-to-focus (graph unnavigable without this)
- File-level aggregation as default view (prevents hairball)
- Zoom, pan, fit-to-screen
- Edge directionality (arrowheads)
- Circular dependency detection
- Filter by file/directory

## Differentiators

- Dead code detection (zero in-degree exports) — validates `fetchPilotDetails` use case
- Blast radius view (transitive dependents) — validates `_seconds/_nanoseconds` use case
- Multi-language in one tool (no existing tool spans TS + Swift + Go + Python)
- Incremental watch mode with live graph updates

## Architecture

Five components, unidirectional data flow:

```
CLI Entry → Indexer → Graph Store → HTTP/WS Server → Browser Client
                ↑
         Extractor Registry (per-language)
```

**Key data model decisions:**
- `SymbolNode.id = "${filePath}::${symbolName}"` — stable across definition and reference sites
- `SymbolEdge.kind = "calls" | "imports" | "extends" | "implements" | "uses-type"`
- In-memory graph store with file-keyed invalidation index for watch mode

**Build order (load-bearing):**
1. Graph data model (everything depends on this shape)
2. TypeScript extractor (proves the interface)
3. Indexer core headless (test without browser)
4. HTTP server + browser static render
5. Browser interaction (search, filter, dead code, blast radius)
6. Watch mode (needs all layers in final form)
7. Swift/Go/Python extractors (parallel, independent)
8. Distribution packaging

## Critical Pitfalls

1. **Tree-sitter ABI mismatch** — Lock exact versions. Test on Node 18/20/22. Human-readable error if mismatch detected.
2. **D3 browser freeze** �� Pre-settle simulation before rendering. Default to file-level nodes. Canvas upgrade path if SVG chokes.
3. **Hairball first impression** — File-level default. Progressive disclosure. Never show all symbols at once.
4. **Dead code false positives** — Multi-hop barrel re-export resolution required. JSX element usage must count. Use confidence levels, not binary.
5. **Watch mode event storms** — 300-500ms debounce + `awaitWriteFinish`. Incremental patch architecture upfront.

## Open Questions

- SVG vs Canvas: measure actual OversizeConnect node count in Phase 2 before committing
- Swift grammar quality: 46 weekly npm downloads, needs validation on real Swift
- Tree-sitter WASM vs native: native is faster but harder to distribute; WASM is safer

## Suggested Phase Ordering

1. **Foundation** — Data model + tree-sitter version pinning + TypeScript extractor
2. **Indexer + Headless Pipeline** — File crawl, symbol resolution, barrel re-export tracing
3. **Browser Visualization** — D3 force graph with all interaction features
4. **Watch Mode** — Incremental patch over WebSocket
5. **Additional Extractors** — Swift, Go, Python (parallel)
6. **Distribution** — Prebuilt binaries, npm packaging, install UX

---
*Ready for roadmap: yes*
