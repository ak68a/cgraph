# Phase 3: Indexer & Analysis Pipeline - Context

**Gathered:** 2026-05-02
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers the indexer and analysis pipeline — a new `crates/indexer` crate that crawls a project directory, dispatches files to the appropriate extractor via a dynamic registry, assembles the full graph in memory (petgraph), resolves barrel re-export chains and tsconfig path aliases, and runs analysis algorithms (dead code detection with confidence scoring, blast radius, transitive dependencies, circular dependency detection). The CLI is updated to print scan statistics and analysis summaries, with `--dead-code` and `--cycles` flags for detailed reports. All headless — no server, no browser.

Requirements: PARS-09, PARS-10, ANLS-01, ANLS-02, ANLS-03, ANLS-04, ANLS-05, INFR-03

</domain>

<decisions>
## Implementation Decisions

### Dead Code Detection
- **D-40:** Entry points are identified by convention — zero config. Auto-detected entry point files: main.ts/index.ts at project root, files in test/ or __tests__/, App.tsx/App.ts (React entry), setup.*/config.* patterns, files with side-effect-only imports. Exported symbols in entry point files are never flagged as dead code.
- **D-41:** Two-tier confidence model: **confirmed dead** (exported, zero incoming edges, not an entry point, not re-exported by any barrel) vs **suspicious** (zero direct edges but demoted by heuristics: symbol name appears as a string literal elsewhere in the project, file has a namespace import `import * as X` that could access the symbol, or symbol is a type used only in generic constraints).

### Circular Dependency Detection
- **D-42:** File-level cycle detection only. Detect cycles between files via import edges (A.ts imports B.ts imports A.ts). Symbol-level call cycles (mutual recursion) are intentional patterns and are not flagged. Cycles are reported as ordered chains showing the import path.

### CLI Output
- **D-43:** Default `cg <path>` prints scan statistics (INFR-03: files scanned, symbols found, edges found, elapsed time) plus analysis summary (dead code count by confidence tier, circular dependency count). Detail flags `--dead-code` and `--cycles` print full reports. Blast radius and transitive dependency queries are deferred to Phase 11 (query CLI).
- **D-44:** Detailed output (`--dead-code`, `--cycles`) uses human-readable text format grouped by file, with symbol kind and line ranges. Machine-readable JSON output deferred to Phase 11 (`--json` flag, AGNT-01).

### Graph Storage
- **D-45:** Use petgraph's `DiGraph<SymbolNode, EdgeKind>` as the in-memory graph. Built-in Tarjan's SCC for cycle detection, DFS for blast radius / transitive deps. Avoids reimplementing graph algorithms.
- **D-46:** Single symbol-level graph with file-level views derived on demand (project symbol edges to file pairs, deduplicate, run cycle detection on the projection). No separate file-level graph — single source of truth, derived views computed as needed.

### Crate Organization
- **D-47:** New `crates/indexer` crate with modules: `lib.rs` (public API), `graph.rs` (CodeGraph struct wrapping petgraph), `resolve.rs` (barrel chain + tsconfig alias resolution), `analysis.rs` (dead code, blast radius, cycles), `crawl.rs` (file crawl + extractor dispatch). Depends on `cgraph-core` and `petgraph`. CLI depends on indexer.
- **D-48:** Dynamic extractor registry — `Indexer::new(extractors: Vec<Box<dyn Extractor>>)`. The CLI builds the registry and passes it in. The indexer crate has no direct dependency on any extractor crate, staying language-agnostic. Adding Swift/Go/Python extractors (Phases 7-9) only requires changes to the CLI registration, not the indexer.

### Barrel & Path Resolution (from Phase 2 context)
- **D-25 (carried):** Extractor emits ReExport edges only — indexer resolves multi-hop barrel chains to find the true source.
- **D-26 (carried):** Star re-exports emit a wildcard marker for the indexer to expand.
- **D-28 (carried):** Extractor emits raw import paths. Indexer reads tsconfig.json once and resolves all alias paths during graph assembly.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Context
- `.planning/ROADMAP.md` — Phase 3 goal, success criteria, dependency chain
- `.planning/REQUIREMENTS.md` — PARS-09, PARS-10, ANLS-01 through ANLS-05, INFR-03
- `.planning/PROJECT.md` — Tech stack constraints, extractor design philosophy

### Prior Phase Context
- `.planning/phases/01-foundation/01-CONTEXT.md` — D-01 through D-24 (symbol ID format, node/edge fields, extractor trait interface, error handling philosophy, workspace layout, test strategy)
- `.planning/phases/02-typescript-extractor/02-CONTEXT.md` — D-25 through D-39 (barrel re-export strategy, path alias deferral, call edge detection, crate structure, test fixtures)

### Existing Code
- `crates/core/src/model.rs` — SymbolNode, SymbolEdge, Language, SymbolKind, EdgeKind definitions
- `crates/core/src/extractor.rs` — Extractor trait, ExtractionResult, ParseError
- `crates/core/src/detect.rs` — scan_directory (file crawl with directory filtering), detect_language, DetectionResult
- `crates/core/src/lib.rs` — Public API re-exports
- `crates/ts-extractor/src/lib.rs` — TsExtractor implementing Extractor trait (the first extractor to register)
- `crates/cli/src/main.rs` — Current CLI (scan + summary only, needs extension for indexer + analysis output)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `scan_directory()` in `crates/core/src/detect.rs` — already walks project directory, returns file paths + languages, skips hidden dirs / node_modules / dist / build / target. The indexer's crawl module can use this directly for file discovery.
- `Extractor` trait in `crates/core/src/extractor.rs` — the interface the dynamic registry dispatches through. `can_handle(&Path) -> bool` determines which extractor handles each file.
- `TsExtractor` in `crates/ts-extractor/` — the first (and currently only) extractor to register. Emits SymbolNodes, SymbolEdges (including ReExport edges with raw paths), and ParseErrors.
- `DetectionResult` in `crates/core/src/detect.rs` — provides `parseable` list (files with known-language extensions) which is the input set for extraction.

### Established Patterns
- Crate per concern: `core` (types + traits), `ts-extractor` (language-specific), `cli` (binary entry point). Indexer follows this as the "assembly + analysis" concern.
- Extractors are pure transformation (D-18): text in, graph fragments out. The indexer owns file I/O — reads source text, passes it to extractors, collects results.
- Error handling: warn and continue (D-13). Partial tree-sitter parses are valid data (D-14). The indexer must propagate this — a broken file doesn't stop the scan.
- Symbol IDs: `file_path::symbol_name` (D-01). The indexer uses these as graph node identifiers and HashMap keys.

### Integration Points
- CLI (`crates/cli/src/main.rs`) currently calls `scan_directory()` and prints summary. Phase 3 extends this to: build extractor registry → create Indexer → scan → print stats + analysis summary → optionally print detail reports.
- Phase 4 (HTTP Server) will consume `CodeGraph` to serialize as JSON for D3 rendering.
- Phase 6 (Watch Mode) will need incremental graph updates — the `CodeGraph` API should support removing/re-adding nodes for a single file.
- Phase 11 (Agent Interface) will query `CodeGraph` for blast radius, dead code, and dependencies.

</code_context>

<specifics>
## Specific Ideas

- OversizeConnect is the first real test target — fixtures and tests should reflect its patterns: barrel re-exports through nested index.ts files, Zod schema type references, Firebase service imports, React Navigation types
- The scan summary line format should match the preview: `cgraph scan: {files} files, {symbols} symbols, {edges} edges ({time})`
- Dead code report groups by file path for scannability — developers think in files, not flat symbol lists
- The suspicious tier annotation should explain WHY (e.g., "namespace import in validators.ts") so the user can quickly verify

</specifics>

<deferred>
## Deferred Ideas

- **Config file for entry points** (`.cgraph.toml`) — override conventions for non-standard project layouts. Add when a user reports a false positive that conventions can't handle.
- **Symbol-level cycle detection** — mutual recursion detection. Intentional in most cases; revisit if users request it.
- **Directory/module-level cycle detection** — architectural cycle view for large monorepos. Add when cgraph targets codebases significantly larger than OversizeConnect.
- **Blast radius CLI query** (`cg blast-radius <symbol-id>`) — deferred to Phase 11 (Agent Interface, AGNT-02).
- **JSON output format** (`--json` flag) — deferred to Phase 11 (AGNT-01).
- **Graph caching / incremental rebuild** — if performance becomes an issue on large codebases, cache the file projection. Not needed for v1 scale.

</deferred>

---

*Phase: 3-Indexer & Analysis Pipeline*
*Context gathered: 2026-05-02*
