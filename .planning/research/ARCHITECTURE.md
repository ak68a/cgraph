# Architecture Patterns: Multi-Language Code Graph Tool

**Domain:** Static analysis + interactive graph visualization CLI
**Researched:** 2026-05-02
**Overall confidence:** HIGH (Sourcetrail internals, tree-sitter docs, dependency-cruiser patterns all verified)

---

## Recommended Architecture

Five components with clear, unidirectional data flow:

```
[CLI Entry]
     |
     v
[Indexer]
  |- [File Crawler]
  |- [Extractor Registry]
       |- [TS/TSX Extractor]
       |- [Swift Extractor]
       |- [Go Extractor]
       |- [Python Extractor]
  |- [Symbol Resolver]
  |- [Graph Store] (in-memory)
     |
     v
[HTTP Server + WebSocket]
     |
     v
[Browser Client]
  |- [D3 Force Graph]
  |- [Filter/Search Panel]
  |- [Analysis Overlays] (dead code, blast radius)
```

---

## Component Boundaries

### 1. CLI Entry (`src/cli/`)

**Responsibility:** Argument parsing, project root resolution, mode dispatch (full scan vs watch).

**Communicates with:** Indexer (kicks off scan), HTTP Server (starts it after indexing completes).

**Does NOT contain:** Any parsing or graph logic. Thin shell only.

**Implementation note:** Use `commander` for arg parsing. Detect `--watch` flag and pass it as a boolean to the Indexer. After full scan completes, start HTTP Server, then open browser with `open` package.

---

### 2. Indexer (`src/indexer/`)

**Responsibility:** Orchestrates file discovery, dispatches files to extractors, accumulates the Graph Store, manages watch mode.

**Sub-components:**

- **File Crawler**: Walks the project directory, filters by extension to language mapping (`*.ts` → typescript, `*.swift` → swift, etc.), respects `.gitignore` via `fast-glob` with ignore patterns.
- **Extractor Registry**: Maps file extensions to extractor instances. New languages added by registering a new extractor — nothing else changes.
- **Symbol Resolver**: Post-extraction pass that resolves import strings to concrete symbol IDs across file boundaries. Uses a two-pass strategy: first pass builds a definition index (all exports keyed by file + name), second pass resolves all import references against that index.
- **Watch Coordinator**: Wraps chokidar with debounce (300-500ms). On file change: re-runs the affected file's extractor, removes the file's old nodes/edges from the Graph Store, inserts the new ones, pushes a diff to the HTTP Server via WebSocket.

**Communicates with:** Extractor Registry (parallel), Symbol Resolver (sequential, after all files parsed), HTTP Server (pushes graph updates in watch mode).

**Key invariant:** The Indexer knows nothing about graph rendering. It produces a pure data structure (the Graph Store) and notifies the server when it changes.

---

### 3. Extractor Registry and Per-Language Extractors (`src/extractors/`)

**Responsibility:** Each extractor is a self-contained module that takes a file path + source text and returns an array of `SymbolNode` and `SymbolEdge` objects. The Registry maps extensions to extractor instances.

**Interface contract** (every extractor must satisfy this):

```typescript
interface Extractor {
  supportedExtensions: string[];
  extract(filePath: string, source: string): ExtractionResult;
}

interface ExtractionResult {
  nodes: SymbolNode[];
  edges: SymbolEdge[];
}
```

**How extractors work internally:**

1. Create a tree-sitter `Parser` instance, set the language grammar (e.g., `require('tree-sitter-typescript').typescript`).
2. Call `parser.parse(source)` to get the concrete syntax tree.
3. Run pre-written tree-sitter queries (S-expression pattern files, e.g., `queries/typescript/tags.scm`) to capture named nodes: function declarations, class declarations, export statements, import declarations, call expressions.
4. Map captures to `SymbolNode` / `SymbolEdge` shapes.
5. Return.

**Why queries-in-files, not inline code:** Each language's query is isolated, testable, and replaceable without touching extractor logic.

**Grammar packages needed:**

| Language | npm package |
|----------|-------------|
| TypeScript/TSX | `tree-sitter-typescript` |
| Swift | `tree-sitter-swift` |
| Go | `tree-sitter-go` |
| Python | `tree-sitter-python` |
| Core bindings | `tree-sitter` (node-tree-sitter) |

**Confidence:** HIGH — tree-sitter Node bindings are well-documented; grammar packages exist for all four target languages.

---

### 4. Graph Store (`src/graph/store.ts`)

**Responsibility:** In-memory graph state. The single source of truth the server reads from and the Indexer writes to.

**Data model — Node:**

```typescript
interface SymbolNode {
  id: string;              // stable: "${filePath}::${symbolName}" — same format for definition and reference sites
  label: string;           // display name ("fetchPilotDetails")
  kind: SymbolKind;        // "function" | "class" | "interface" | "variable" | "type" | "file"
  filePath: string;        // absolute path
  line: number;            // definition line
  language: Language;      // "typescript" | "swift" | "go" | "python"
  exportStatus: "exported" | "internal";
  usageCount: number;      // computed by Symbol Resolver
}
```

**Data model — Edge:**

```typescript
interface SymbolEdge {
  id: string;              // "${sourceId}->${targetId}::${kind}"
  source: string;          // SymbolNode.id
  target: string;          // SymbolNode.id
  kind: EdgeKind;          // "calls" | "imports" | "extends" | "implements" | "uses-type"
}
```

**Why this model:**

- Stable IDs derived from filepath + name: the same ID is generated at the definition site and at every reference site. This is the standard approach validated by Sourcetrail (SQLite keyed by stable symbol IDs) and SCIP/Lore-style tools.
- `usageCount` is computable in a single graph traversal after resolution: count incoming "imports" edges to each node.
- Edge `kind` enables the filtering requirements (filter by edge type).

**Serialization:** `JSON.stringify({ nodes: [...], edges: [...] })` — the HTTP server sends this as the initial graph payload. No graph database needed; a flat JSON array with ~10K entries is cheap in-memory.

**File-level invalidation for watch mode:** The store maintains a secondary index `nodesByFile: Map<string, Set<string>>`. On file change, look up all node IDs for that file, delete those nodes and any edges that reference them, re-insert the new extraction result. O(file-affected nodes), not O(entire graph).

---

### 5. HTTP Server + WebSocket (`src/server/`)

**Responsibility:** Serves the browser bundle, provides the initial graph payload via REST, and pushes incremental graph updates via WebSocket.

**API surface:**

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/` | GET | Serves `index.html` (embedded browser bundle) |
| `/api/graph` | GET | Returns full graph JSON (Graph Store snapshot) |
| `/ws` | WebSocket | Pushes `{ type: "patch", added: [...], removed: [...] }` on file change |

**Implementation:** Express for HTTP + `ws` package for WebSocket. No socket.io — overkill for a single-connection localhost tool.

**Bundle strategy:** The browser client is a pre-built Vite bundle embedded in the npm package. The server serves it as static files from `dist/client/`. This means the client is built at publish time, not at run time.

**Communicates with:** Graph Store (reads), Indexer Watch Coordinator (receives patch notifications), Browser Client (pushes updates).

---

### 6. Browser Client (`client/`)

**Responsibility:** Renders the D3 force graph, handles user interaction (click, filter, search), applies analysis overlays (dead code highlight, blast radius expand).

**Sub-components:**

- **Graph Renderer:** D3 force simulation on SVG. Nodes as circles, edges as lines. Zoom/pan via `d3.zoom`.
- **Control Panel:** Filter dropdowns (file, language, edge type, usage count threshold), search input with live highlight.
- **Analysis Overlays:** Dead code mode (highlight nodes where `usageCount === 0` and `exportStatus === "exported"`). Blast radius mode (on node click, run BFS over incoming edges to collect all transitive dependents, highlight that subgraph).
- **WebSocket Client:** Listens for patch messages, applies added/removed node/edge diffs to the simulation incrementally — does not reload the full graph.

**Rendering performance note:** SVG handles up to ~1,000 nodes at interactive frame rates. For the typical single-project codebase, this is sufficient. If the graph exceeds ~2,000 nodes, Canvas rendering (replace D3 SVG node rendering with a Canvas draw loop while keeping D3 force simulation for layout) is the natural upgrade path. WebGL (PIXI.js) is the option beyond that but is not needed for v1.

**Build:** Vite + TypeScript. `npm run build:client` outputs to `dist/client/`.

---

## Data Flow

### Initial Scan

```
CLI runs
  -> Indexer.scan(projectRoot)
     -> FileCrawler.discover() -> [file paths]
     -> For each file in parallel:
          ExtractorRegistry.extract(filePath, source) -> { nodes, edges }
     -> GraphStore.bulkInsert(allNodes, allEdges)
     -> SymbolResolver.resolveReferences(GraphStore)
          -> Updates usageCount on all nodes
     -> Indexer emits "ready"
  -> HTTPServer.start(port)
  -> open("http://localhost:{port}")
Browser loads
  -> GET /api/graph -> full graph JSON
  -> D3 simulation starts
```

### Watch Mode (file change)

```
chokidar emits change(filePath)
  -> Debounce 300ms
  -> ExtractorRegistry.extract(filePath, source) -> { nodes, edges }
  -> GraphStore.invalidateFile(filePath)    // remove old
  -> GraphStore.insert(newNodes, newEdges)  // add new
  -> SymbolResolver.resolveFile(filePath)   // re-resolve affected edges
  -> WebSocket.broadcast({ type: "patch", added, removed })
Browser WebSocket handler
  -> D3 simulation: add/remove nodes and links
  -> Re-apply current filter state
```

### Dead Code Detection

```
After SymbolResolver.resolveReferences():
  For each node where exportStatus === "exported":
    if node.usageCount === 0 -> mark as deadCode: true
Browser:
  Dead code overlay toggle -> filter nodes where deadCode === true -> highlight
```

### Blast Radius

```
User clicks node N
  -> BFS outward on incoming edges (callers of N, callers of callers...)
  -> Collect all ancestor node IDs
  -> Set highlight flag on collected nodes
  -> D3 updates visual styling
(All client-side — no server round-trip needed)
```

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Compiler-Grade Cross-File Resolution

**What:** Running a full TypeScript Language Server or using ts-morph for type-aware symbol resolution.
**Why bad:** Defeats the multi-language goal; ts-morph requires a tsconfig, doesn't work for Swift/Go/Python; adds 5-10 seconds startup time; huge dependency.
**Instead:** Name-matching heuristics in the Symbol Resolver. Import path `../auth/login` + exported name `loginUser` → resolve to the node with `filePath` containing `auth/login` and `label === "loginUser"`. This covers ~95% of cases in a well-structured codebase. Flag unresolved references as "unknown" rather than crashing.

### Anti-Pattern 2: Graph Database for Storage

**What:** Using Neo4j, SQLite, or any persistent graph database as the primary store.
**Why bad:** Adds operational complexity (Neo4j requires a running server; SQLite requires schema migrations); the tool is ephemeral by design — you run it, explore, close it. For a single project with ~5K–50K symbols, in-memory JSON is faster and simpler.
**Instead:** In-memory Graph Store with file-keyed invalidation. If persistence across sessions is needed later, serialize to a `.codegraph` JSON file on exit.

### Anti-Pattern 3: Per-Language Special Cases in the Indexer

**What:** `if (language === 'typescript') { ... } else if (language === 'swift') { ... }` in the Indexer core.
**Why bad:** Every new language requires modifying core logic. Dead code flags spread across files.
**Instead:** The Extractor interface contract. All language-specific logic lives in the extractor module. The Indexer only calls `extractor.extract()` and handles the result uniformly.

### Anti-Pattern 4: Full Graph Re-Render on Each Watch Event

**What:** Browser receives a "graph changed" WebSocket event, clears the DOM, and re-renders from scratch.
**Why bad:** D3 force simulation loses its current layout (node positions snap back to random), causing jarring visual resets on every file save.
**Instead:** Patch protocol. Server sends `{ added: [], removed: [] }`. Browser updates simulation data incrementally: remove stale nodes/edges, add new ones. D3 preserves existing node positions (they retain `x`, `y` from the simulation).

### Anti-Pattern 5: Serving the Browser Bundle as a CDN URL

**What:** `index.html` references D3 from `https://cdn.jsdelivr.net/...`.
**Why bad:** Tool must work offline (e.g., on a plane, in a corporate network with CDN blocks).
**Instead:** Bundle everything into `dist/client/bundle.js` at publish time via Vite. The Express server serves it as a local static file.

---

## Incremental Parsing — Tree-sitter API Detail

Tree-sitter's incremental parsing is designed for editors (character-level edits), but the watch mode use case (whole-file re-read) is simpler: just call `parser.parse(newSource, oldTree)`. Tree-sitter internally reuses unchanged subtrees. The performance gain is secondary here — the real reason to pass `oldTree` is correctness in error-recovery cases.

**For watch mode, the practical approach is simpler than full incremental:**

1. On file change, read the new source from disk.
2. Call `parser.parse(newSource)` — a fresh parse is fast enough for a single file (single-file parses are typically 1-5ms even for large files).
3. No need to maintain and edit `TSTree` instances between file saves.

Full incremental tree reuse (passing `oldTree`) is worth implementing if profiling shows parsing as a bottleneck, but is premature for v1.

---

## Scalability Considerations

| Concern | Small project (<1K files) | Medium project (1K–10K files) | Large project (>10K files) |
|---------|--------------------------|-------------------------------|---------------------------|
| Parsing speed | <1s full scan | 5-30s full scan | >30s, needs worker threads |
| Graph store size | <10K nodes — trivial | 10K–100K nodes — still fine in memory | >100K nodes — need pagination |
| D3 SVG rendering | Smooth | Laggy above ~2K visible nodes | Need Canvas or filtered views |
| Watch latency | Near-instant | Near-instant (single-file re-parse) | Near-instant (single-file re-parse) |

For the OversizeConnect codebase (TypeScript/React Native, ~1K–5K files), SVG rendering and in-memory store are the right choice. Canvas upgrade is a later optimization.

**Worker thread note:** Node.js `worker_threads` can parallelize file extraction across CPU cores. The Indexer should be structured so file extraction is a pure function (file path + source → nodes + edges) — this makes it trivially parallelizable later without an architectural change.

---

## Suggested Build Order

Components have these hard dependencies:

```
1. Graph Store data model (SymbolNode, SymbolEdge interfaces)
   — Everything else depends on this shape. Define it first.

2. One extractor (TypeScript)
   — Proves the extractor interface works. Unblocks the Indexer.

3. Indexer core (without watch mode)
   — File crawler + extractor dispatch + symbol resolver.
   — Can run headless, output JSON to stdout for testing.

4. HTTP Server (initial graph endpoint only, no WebSocket)
   — Proves the server can serialize the Graph Store.

5. Browser Client — static render (no interaction)
   — D3 force simulation consuming the /api/graph payload.
   — Proves the full pipeline end-to-end.

6. Browser Client — interaction layer
   — Filters, search, dead code overlay, blast radius.

7. Remaining extractors (Swift, Go, Python)
   — Each is independent; can be done in parallel.

8. Watch mode
   — Chokidar + WebSocket patch protocol.
   — Depends on: Indexer core, HTTP Server, Browser WebSocket client.
```

**Critical path:** Graph Store model → TypeScript extractor → Indexer core → HTTP Server → Browser static render.

Watch mode and additional language extractors are parallel work once step 5 is complete.

---

## How Existing Tools Compare

| Tool | Graph Model | Storage | Language Support | Watch Mode |
|------|-------------|---------|-----------------|------------|
| Sourcetrail | Nodes + edges in SQLite; stable symbol IDs; separate indexer process | SQLite (.srctrldb file) | Language-specific parsers (Clang, Java, Python) | No (manual re-index) |
| dependency-cruiser | Module-level only (files, not symbols); extract→validate→report pipeline; JSON schema output | In-memory, no persistence | JS/TS only (transpiler plugins) | No built-in watch |
| Madge | Module-level import graph; file nodes + import edges | In-memory | JS/TS only | No |
| **code-graph (this tool)** | Symbol-level nodes + typed edges; in-memory + WebSocket patches | In-memory, no persistence | TS/Swift/Go/Python via tree-sitter | Yes (chokidar + WebSocket) |

The key differentiator vs. dependency-cruiser and Madge: symbol-level granularity (function/class/interface, not just file) and multi-language support from a single parser infrastructure.

---

## Sources

- Sourcetrail architecture: [DeepWiki: CoatiSoftware/Sourcetrail](https://deepwiki.com/CoatiSoftware/Sourcetrail)
- Tree-sitter incremental parsing API: [Tree-sitter Advanced Parsing Docs](https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html)
- Tree-sitter Node bindings: [node-tree-sitter GitHub](https://github.com/tree-sitter/node-tree-sitter)
- Tree-sitter query syntax: [Tree-sitter Query Syntax](https://tree-sitter.github.io/tree-sitter/using-parsers/queries/1-syntax.html)
- Dependency-cruiser internals: [dependency-cruiser GitHub](https://github.com/sverweij/dependency-cruiser)
- Symbol ID stability: [Semantic Code Graph paper](https://arxiv.org/html/2310.02128v2)
- File-incremental indexing RFC: [sheeptechnologies SCIP RFC](https://github.com/orgs/sheeptechnologies/discussions/4)
- D3 force simulation: [d3/d3-force GitHub](https://github.com/d3/d3-force)
- D3 performance: SVG vs Canvas vs WebGL: [Graph viz efficiency PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC12061801/)
- Chokidar file watcher: [chokidar npm](https://www.npmjs.com/package/chokidar)
- Dead code detection patterns: [knip recommendation - Effective TypeScript](https://effectivetypescript.com/2023/07/29/knip/)
