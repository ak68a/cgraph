# Roadmap: cgraph

## Overview

cgraph is a Rust CLI tool with a browser-based D3 visualization client. Built in ten phases that follow the load-bearing build order: data model first, TypeScript extractor second (proves the interface), headless analysis pipeline third, then visualization and interaction layers, watch mode, the three additional language extractors, and finally distribution. Tree-sitter is used natively (C/Rust, no bindings) eliminating all ABI/install issues. The browser client (HTML/JS/D3) is embedded as static assets in the binary. The later extractor phases (Swift, Go, Python) are independent of each other and could run in parallel.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Foundation** - Project scaffold, shared data model, tree-sitter setup, language detection, CLI skeleton
- [ ] **Phase 2: TypeScript Extractor** - Full TS/TSX extraction including imports, calls, type refs, barrel re-exports, and path alias resolution
- [ ] **Phase 3: Indexer & Analysis Pipeline** - File crawl, symbol resolution, dead code detection, blast radius, circular dependency analysis
- [ ] **Phase 4: HTTP Server & Browser Shell** - Localhost server, static browser client, D3 force graph rendering, scan statistics output
- [ ] **Phase 5: Graph Interaction** - Node expand/collapse, zoom/pan, search, click-to-focus, blast radius view, dead code overlay, filters, session history
- [ ] **Phase 6: Watch Mode** - File watcher, incremental re-parse on save, WebSocket push of graph patches to browser
- [ ] **Phase 7: Swift Extractor** - tree-sitter-swift grammar, extraction of Swift symbols and relationships
- [ ] **Phase 8: Go Extractor** - tree-sitter-go grammar, extraction of Go symbols and relationships
- [ ] **Phase 9: Python Extractor** - tree-sitter-python grammar, extraction of Python symbols and relationships
- [ ] **Phase 10: Distribution** - cargo install, Homebrew tap, prebuilt binaries, README
- [ ] **Phase 11: Agent Interface** - MCP server mode, JSON output, query CLI for programmatic access by AI agents
- [ ] **Phase 12: Multi-Repo Analysis** - Cross-service edge detection, multi-directory scanning, API route matching, service-layer graph view

## Phase Details

### Phase 1: Foundation
**Goal**: The project has a working Rust skeleton — CLI entry point (clap), shared graph data model (structs/enums), tree-sitter linked natively, language auto-detection from file extensions — so every subsequent phase builds on a stable, agreed-upon shape.
**Depends on**: Nothing (first phase)
**Requirements**: PARS-11, INFR-01
**Success Criteria** (what must be TRUE):
  1. Running `cg <path>` prints a usage/version line and exits cleanly
  2. The shared `SymbolNode` and `SymbolEdge` structs are defined and used by all modules via the graph crate/module
  3. Tree-sitter parses a sample TypeScript file without errors (native C linkage, no bindings)
  4. Given a directory of mixed .ts, .swift, .go, and .py files, the tool correctly reports which language(s) it detected
**Plans:** 3 plans
Plans:
**Wave 1**
- [x] 01-01-PLAN.md — Workspace scaffold, core data model, language detection, extractor trait

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 01-02-PLAN.md — Tree-sitter grammar linkage validation with fixture files
- [x] 01-03-PLAN.md — CLI binary crate with scan summary and smoke tests

### Phase 2: TypeScript Extractor
**Goal**: Users can point cgraph at a TypeScript/React Native project and get a complete, accurate graph of all symbols and their relationships, including barrel re-exports resolved to their true source and tsconfig path aliases resolved to real file paths.
**Depends on**: Phase 1
**Requirements**: PARS-01, PARS-05, PARS-06, PARS-07, PARS-08, PARS-09, PARS-10
**Success Criteria** (what must be TRUE):
  1. Running against OversizeConnect extracts all exported functions, components, hooks, types, classes, and interfaces from .ts/.tsx files
  2. Import edges between files are present and correctly directed (importer → importee)
  3. Function call edges are present where functions call other named functions
  4. Type reference edges (extends, implements, uses-type) are present
  5. A symbol re-exported through one or more barrel files traces back to its defining file, not the barrel
  6. tsconfig `paths` aliases in import statements resolve to the correct file path
**Plans:** 5 plans
Plans:
**Wave 1**
- [x] 02-01-PLAN.md — Crate scaffold, workspace integration, test fixtures, TsExtractor struct with query compilation

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 02-02-PLAN.md — Symbol extraction (Pass 1): exported functions, types, classes, enums, hooks

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 02-03-PLAN.md — Edge extraction (Pass 2): imports, calls, type refs, re-exports

**Wave 4** *(gap closure)*
- [ ] 02-04-PLAN.md — Reassign PARS-09, PARS-10 to Phase 3 in ROADMAP and REQUIREMENTS docs
- [ ] 02-05-PLAN.md — Fix anti-patterns: dead code warnings, namespace re-export misclassification, overload dedup

### Phase 3: Indexer & Analysis Pipeline
**Goal**: The indexer crawls a project directory, feeds all files through the extractor registry, assembles the full graph in memory, and runs analysis algorithms so dead code, blast radius, and circular dependencies are available as queryable data — all without any browser or server.
**Depends on**: Phase 2
**Requirements**: ANLS-01, ANLS-02, ANLS-03, ANLS-04, ANLS-05, INFR-03
**Success Criteria** (what must be TRUE):
  1. After scanning a project, the CLI prints a summary line: files scanned, symbols found, edges found, and elapsed time
  2. Exported symbols with zero incoming edges are flagged as dead code; symbols re-exported through barrels are not falsely flagged
  3. Dead code results include a confidence level (confirmed vs. suspicious) rather than a binary flag
  4. Given any symbol ID, the indexer returns the complete set of its transitive dependents (blast radius)
  5. Given any symbol ID, the indexer returns the complete set of things it transitively depends on
  6. Circular dependency chains between modules are detected and enumerable
**Plans**: TBD

### Phase 4: HTTP Server & Browser Shell
**Goal**: Users run `cg <path>` and a browser tab opens showing a D3 force-directed graph of the scanned project — file-level nodes by default, edges with arrowheads, nodes color-coded by type, simulation pre-settled so the graph is immediately usable.
**Depends on**: Phase 3
**Requirements**: VIZN-01, VIZN-02, VIZN-05, VIZN-06, VIZN-07, VIZN-08, INFR-02
**Success Criteria** (what must be TRUE):
  1. `cg <path>` starts a localhost HTTP server, opens the browser automatically, and the graph is visible within a few seconds
  2. The default view shows file-level nodes only (not individual symbols), preventing the hairball on first load
  3. Edges render with arrowheads that indicate direction of dependency
  4. Nodes are visually distinguished by symbol type via color coding (function, class, type, interface, hook, file each distinct)
  5. The force simulation completes before the graph is painted — the graph does not animate/jitter when the page loads
  6. The graph uses progressive disclosure: files are the top level, with the ability to go deeper in subsequent phases
**Plans**: TBD
**UI hint**: yes

### Phase 5: Graph Interaction
**Goal**: Users can fully navigate and interrogate the graph — expanding file nodes to see exports, zooming and panning, searching by name, clicking to focus on a node's neighborhood, activating blast radius and dead code overlays, filtering by file/type/edge, and moving back and forward through their exploration history.
**Depends on**: Phase 4
**Requirements**: VIZN-03, VIZN-04, INTR-01, INTR-02, INTR-03, INTR-04, INTR-05, INTR-06, INTR-07, INTR-08
**Success Criteria** (what must be TRUE):
  1. Clicking a file node expands it to show its exported symbols as child nodes; clicking again collapses it
  2. The graph supports mouse/trackpad zoom, pan, and a fit-to-screen button that frames all visible nodes
  3. Typing in the search box highlights matching nodes in real time; pressing Enter or clicking a result focuses the graph on that node
  4. Clicking any node dims unrelated nodes and shows only its immediate neighbors (imports/importees/callers/callees)
  5. Activating blast radius mode on a selected node highlights all nodes that transitively depend on it
  6. Activating dead code overlay highlights all exported symbols with zero incoming edges using the confidence coloring from Phase 3
  7. The user can filter visible nodes/edges by file or directory, by symbol type, and by edge type independently
  8. Clicking through multiple focused nodes builds a back/forward history navigable with browser-style controls
**Plans**: TBD
**UI hint**: yes

### Phase 6: Watch Mode
**Goal**: Users running `cg <path> --watch` (or always-on dashboard mode) see the graph update automatically when they save a file — without a full page reload, using incremental WebSocket patches that only re-render the changed portions of the graph.
**Depends on**: Phase 5
**Requirements**: INFR-04, INFR-05
**Success Criteria** (what must be TRUE):
  1. `cg <path> --watch` starts the server and keeps it running; saving a file in the scanned project triggers a re-parse of that file within 1 second
  2. The browser graph updates to reflect the changed file without a full page reload — nodes and edges that unchanged remain in place
  3. Saving multiple files in rapid succession (e.g., a formatter touching 10 files) results in a single debounced graph update, not 10 separate re-renders
**Plans**: TBD

### Phase 7: Swift Extractor
**Goal**: Users can point cgraph at a Swift project (or a mixed Swift/TypeScript project like OversizeConnect) and get a graph of Swift symbols and their relationships, using the same extractor interface proven in Phase 2.
**Depends on**: Phase 3
**Requirements**: PARS-02
**Success Criteria** (what must be TRUE):
  1. Running against a Swift project extracts functions, structs, classes, protocols, and enums as symbol nodes
  2. Import and type reference relationships between Swift files are present as edges
  3. Swift symbols appear in the graph alongside TypeScript symbols when both languages are present in a project directory
**Plans**: TBD

### Phase 8: Go Extractor
**Goal**: Users can point cgraph at a Go project and get a graph of Go symbols and their relationships, using the same extractor interface.
**Depends on**: Phase 3
**Requirements**: PARS-03
**Success Criteria** (what must be TRUE):
  1. Running against a Go project extracts functions, structs, interfaces, and methods as symbol nodes
  2. Import and type reference relationships between Go files are present as edges
  3. Go symbols appear correctly in a mixed-language graph when the project contains other supported languages
**Plans**: TBD

### Phase 9: Python Extractor
**Goal**: Users can point cgraph at a Python project and get a graph of Python symbols and their relationships, using the same extractor interface.
**Depends on**: Phase 3
**Requirements**: PARS-04
**Success Criteria** (what must be TRUE):
  1. Running against a Python project extracts functions, classes, and methods as symbol nodes
  2. Import relationships between Python modules are present as edges
  3. Python symbols appear correctly in a mixed-language graph when the project contains other supported languages
**Plans**: TBD

### Phase 10: Distribution
**Goal**: cgraph is published and installable via `cargo install cgraph`, Homebrew, and npm (prebuilt binaries) — a single static binary with no runtime dependencies, cross-compiled for macOS (arm64, x64) and Linux (x64), with documentation sufficient to use the tool.
**Depends on**: Phase 9
**Requirements**: INFR-06
**Success Criteria** (what must be TRUE):
  1. `cargo install cgraph` builds and installs a working binary
  2. `brew install cgraph` (via tap) installs a prebuilt binary on macOS without compilation
  3. Prebuilt binaries available for macOS arm64, macOS x64, and Linux x64
  4. README covers install methods, usage, and supported languages
**Plans**: TBD

### Phase 11: Agent Interface
**Goal**: cgraph is usable as a programmatic knowledge layer for AI agents (Claude Code, Cursor, etc.) — agents can query the graph for blast radius, dead code, dependencies, and symbol relationships without needing the visual UI.
**Depends on**: Phase 3 (needs indexer + analysis), Phase 10 (needs distribution for install)
**Requirements**: AGNT-01, AGNT-02, AGNT-03
**Success Criteria** (what must be TRUE):
  1. `cg ./path --json` outputs the full graph as structured JSON to stdout
  2. `cg query blast-radius <symbol-id>` returns a JSON list of transitive dependents
  3. `cg mcp` starts an MCP server that Claude Code can connect to and query the graph interactively
  4. An agent can determine "what breaks if I change X?" in under 2 seconds
**Plans**: TBD

### Phase 12: Multi-Repo Analysis
**Goal**: cgraph can analyze distributed systems spanning multiple repositories — detecting cross-service edges (API calls, shared contracts, message queues) and presenting a service-level graph view alongside the per-repo symbol graph.
**Depends on**: Phase 5 (needs full interaction layer), Phase 11 (agents need multi-repo queries)
**Requirements**: MREP-01, MREP-02, MREP-03, MREP-04
**Success Criteria** (what must be TRUE):
  1. `cg ./frontend ./backend ./auth-service` scans multiple directories and produces a unified graph
  2. API client calls (fetch/axios/HTTP) in one repo are matched to route/endpoint definitions in another repo
  3. The graph view offers a "service layer" toggle showing only inter-service edges
  4. Blast radius queries can cross repo boundaries ("changing this endpoint affects these consumers")
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order. Phases 7, 8, 9 all depend on Phase 3 (not each other) and can run in parallel if desired. Phase 11 depends on Phase 3+10. Phase 12 depends on Phase 5+11.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 3/3 | Complete | - |
| 2. TypeScript Extractor | 3/5 | Gap closure | - |
| 3. Indexer & Analysis Pipeline | 0/TBD | Not started | - |
| 4. HTTP Server & Browser Shell | 0/TBD | Not started | - |
| 5. Graph Interaction | 0/TBD | Not started | - |
| 6. Watch Mode | 0/TBD | Not started | - |
| 7. Swift Extractor | 0/TBD | Not started | - |
| 8. Go Extractor | 0/TBD | Not started | - |
| 9. Python Extractor | 0/TBD | Not started | - |
| 10. Distribution | 0/TBD | Not started | - |
| 11. Agent Interface | 0/TBD | Not started | - |
| 12. Multi-Repo Analysis | 0/TBD | Not started | - |
