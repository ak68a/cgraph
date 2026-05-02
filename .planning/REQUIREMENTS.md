# Requirements: cgraph

**Defined:** 2026-05-02
**Core Value:** Instantly see what's connected to what — dead code, blast radius, dependency depth — without manual grep work.

## v1 Requirements

### Parsing

- [ ] **PARS-01**: Tool parses TypeScript/TSX files and extracts all exported symbols (functions, components, hooks, types, classes, interfaces)
- [ ] **PARS-02**: Tool parses Swift files and extracts all symbols (funcs, structs, classes, protocols, enums)
- [ ] **PARS-03**: Tool parses Go files and extracts all symbols (funcs, structs, interfaces, methods)
- [ ] **PARS-04**: Tool parses Python files and extracts all symbols (functions, classes, methods)
- [ ] **PARS-05**: Tool extracts import relationships between modules
- [ ] **PARS-06**: Tool extracts function/method call relationships
- [ ] **PARS-07**: Tool extracts type reference relationships (extends, implements, uses-type)
- [ ] **PARS-08**: Tool extracts re-export relationships (barrel files)
- [x] **PARS-09**: Tool resolves multi-hop barrel re-export chains to find the true source
- [x] **PARS-10**: Tool resolves TypeScript path aliases (tsconfig paths, babel moduleNameMapper)
- [ ] **PARS-11**: Tool auto-detects project language from file extensions

### Analysis

- [x] **ANLS-01**: Tool identifies dead code (exported symbols with zero incoming edges)
- [x] **ANLS-02**: Dead code detection uses confidence scoring (suspicious vs confirmed dead)
- [x] **ANLS-03**: Tool detects circular dependencies between modules
- [x] **ANLS-04**: Tool computes transitive dependents for any symbol (blast radius)
- [x] **ANLS-05**: Tool computes transitive dependencies for any symbol (what it uses)

### Visualization

- [ ] **VIZN-01**: Graph renders as D3 force-directed layout in the browser
- [ ] **VIZN-02**: Default view shows file-level nodes (not individual symbols)
- [ ] **VIZN-03**: User can expand a file node to see its exported symbols
- [ ] **VIZN-04**: User can zoom, pan, and fit-to-screen
- [ ] **VIZN-05**: Edges show directionality via arrowheads
- [ ] **VIZN-06**: Nodes are color-coded by symbol type (function, class, type, interface, hook, file)
- [ ] **VIZN-07**: Force simulation pre-settles before rendering (no freeze on load)
- [ ] **VIZN-08**: Graph uses progressive disclosure (files → exports → internals)

### Interaction

- [ ] **INTR-01**: User can search for a symbol by name with live highlighting
- [ ] **INTR-02**: User can click a node to focus and see its immediate neighbors
- [ ] **INTR-03**: User can activate blast radius view to see all transitive dependents of a symbol
- [ ] **INTR-04**: User can activate dead code overlay highlighting unused exports
- [ ] **INTR-05**: User can filter the graph by file or directory
- [ ] **INTR-06**: User can filter the graph by symbol type
- [ ] **INTR-07**: User can filter the graph by edge type (imports, calls, type refs)
- [ ] **INTR-08**: User can navigate back/forward through focused nodes (session history)

### Infrastructure

- [ ] **INFR-01**: Tool runs as a CLI command (`cg <path>`)
- [ ] **INFR-02**: Tool starts a localhost HTTP server and auto-opens the browser
- [x] **INFR-03**: Tool displays scan statistics after parsing (files, symbols, edges, time)
- [ ] **INFR-04**: Tool supports watch mode that re-parses changed files on save
- [ ] **INFR-05**: Watch mode pushes incremental graph updates via WebSocket (no full reload)
- [ ] **INFR-06**: Tool is distributed as a single binary (cargo install, Homebrew, and npm prebuilt binaries)
- [ ] **INFR-07**: Watch mode detects git branch switches (checkout/switch) and performs a full re-scan rather than incremental file-level updates

### Agent Interface

- [ ] **AGNT-01**: Tool outputs full graph as structured JSON via `cg <path> --json`
- [ ] **AGNT-02**: Tool supports query mode (`cg query blast-radius <symbol-id>`) returning structured results
- [ ] **AGNT-03**: Tool runs as an MCP server (`cg mcp`) queryable by Claude Code and other AI agents

### Multi-Repo

- [ ] **MREP-01**: Tool scans multiple directories in a single invocation (`cg ./repo-a ./repo-b`)
- [ ] **MREP-02**: Tool detects API client calls and matches them to endpoint definitions across repos
- [ ] **MREP-03**: Graph provides a service-layer view showing only inter-service edges
- [ ] **MREP-04**: Blast radius queries cross repo boundaries (changing an endpoint shows affected consumers)

## v2 Requirements

### Advanced Visualization

- **ADVZ-01**: Metrics overlay (lines of code, complexity per node)
- **ADVZ-02**: Canvas rendering mode for large codebases (>2000 nodes)
- **ADVZ-03**: Cluster visualization by directory/module

### Export

- **XPRT-01**: User can export graph as PNG/SVG snapshot
- **XPRT-02**: Graph state is persisted in URL (shareable links)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Electron/desktop app | CLI + browser is sufficient; no packaging complexity |
| CI/report mode | Different tool, different trust model |
| Code modification/refactoring | Read-only analysis tool |
| 3D force graph | No navigational advantage; motion sickness risk |
| Configurable lint rules | Adoption killer per research (dependency-cruiser) |
| Persistent symbol tree sidebar | Duplicates graph; wastes screen space |
| LSP integration | Tree-sitter is sufficient for v1 extraction accuracy |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| PARS-01 | Phase 2 | Pending |
| PARS-02 | Phase 7 | Pending |
| PARS-03 | Phase 8 | Pending |
| PARS-04 | Phase 9 | Pending |
| PARS-05 | Phase 2 | Pending |
| PARS-06 | Phase 2 | Pending |
| PARS-07 | Phase 2 | Pending |
| PARS-08 | Phase 2 | Pending |
| PARS-09 | Phase 2 (raw edges), Phase 3 (resolution) | Complete |
| PARS-10 | Phase 2 (raw paths), Phase 3 (resolution) | Complete |
| PARS-11 | Phase 1 | Pending |
| ANLS-01 | Phase 3 | Complete |
| ANLS-02 | Phase 3 | Complete |
| ANLS-03 | Phase 3 | Complete |
| ANLS-04 | Phase 3 | Complete |
| ANLS-05 | Phase 3 | Complete |
| VIZN-01 | Phase 4 | Pending |
| VIZN-02 | Phase 4 | Pending |
| VIZN-03 | Phase 5 | Pending |
| VIZN-04 | Phase 5 | Pending |
| VIZN-05 | Phase 4 | Pending |
| VIZN-06 | Phase 4 | Pending |
| VIZN-07 | Phase 4 | Pending |
| VIZN-08 | Phase 4 | Pending |
| INTR-01 | Phase 5 | Pending |
| INTR-02 | Phase 5 | Pending |
| INTR-03 | Phase 5 | Pending |
| INTR-04 | Phase 5 | Pending |
| INTR-05 | Phase 5 | Pending |
| INTR-06 | Phase 5 | Pending |
| INTR-07 | Phase 5 | Pending |
| INTR-08 | Phase 5 | Pending |
| INFR-01 | Phase 1 | Pending |
| INFR-02 | Phase 4 | Pending |
| INFR-03 | Phase 3 | Complete |
| INFR-04 | Phase 6 | Pending |
| INFR-05 | Phase 6 | Pending |
| INFR-06 | Phase 10 | Pending |
| INFR-07 | Phase 6 | Pending |

**Coverage:**
- v1 requirements: 39 total
- Mapped to phases: 39
- Unmapped: 0 ✓

---
*Requirements defined: 2026-05-02*
*Last updated: 2026-05-02 after roadmap creation*
