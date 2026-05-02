# Phase 1: Foundation - Context

**Gathered:** 2026-05-02
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers the project scaffold: Cargo workspace structure, shared graph data model (SymbolNode, SymbolEdge, traits), tree-sitter linked natively via published grammar crates, language auto-detection from file extensions, and a CLI skeleton that scans a directory and reports detected languages. No extraction logic — just the foundation everything else builds on.

</domain>

<decisions>
## Implementation Decisions

### Symbol Identity
- **D-01:** Symbol IDs use path-qualified format: `file_path::symbol_name` (e.g., `src/auth/login.ts::handleLogin`). Human-readable, greppable, no indirection required for agent queries.
- **D-02:** IDs are scoped to a single scan — no persistence across runs. A rescan produces fresh IDs from current file state.

### Node & Edge Metadata
- **D-03:** SymbolNode fields: `id`, `name`, `kind` (function/class/type/interface/hook/enum), `file_path`, `language`, `line_start`, `line_end`, `is_exported`.
- **D-04:** SymbolEdge fields: `source_id`, `target_id`, `kind` (import/call/type_ref/re_export), `source_location` (line where the reference occurs).
- **D-05:** Deferred to later phases: `docstring`, `signature`, `byte_range`. Add them when the phase that needs them arrives.

### Workspace Layout
- **D-06:** Cargo workspace from day one. Structure: `crates/core` (data model + traits + language detection), `crates/cli` (binary entry point, clap). Extractor crates (`crates/ts-extractor`, etc.) and server crate added in their respective phases.
- **D-07:** Each extractor is a self-contained crate with one dependency: `core`. Enforces the pluggable extractor boundary at the compiler level.

### Tree-sitter Integration
- **D-08:** Use published grammar crates from crates.io (e.g., `tree-sitter-typescript`, `tree-sitter-swift`). No source-vendoring unless a specific grammar breaks.
- **D-09:** Each extractor owns its own tree-sitter parsing. The indexer provides source text; extractors create their own parser and tree.

### Language Detection
- **D-10:** Detection is language-agnostic — scan all file extensions, report everything found including unsupported languages.
- **D-11:** Summary output distinguishes detected vs. parseable vs. skipped (unsupported).
- **D-12:** Parsing only runs for languages with an available extractor. Unsupported files are skipped without error.

### Error Handling
- **D-13:** Warn and continue, always. A single broken file never prevents scanning the rest of the project.
- **D-14:** Tree-sitter partial parses are valid — extract what's available, report errors as data.
- **D-15:** File-level errors go to stderr/verbose log, not the main scan summary. Summary shows count only (e.g., "4 with partial errors").

### Extractor Trait
- **D-16:** Trait interface: `language() -> Language`, `can_handle(&Path) -> bool`, `extract(&Path, &str) -> ExtractionResult`.
- **D-17:** ExtractionResult contains owned `Vec<SymbolNode>`, `Vec<SymbolEdge>`, `Vec<ParseError>`. Errors are data, not panics.
- **D-18:** Extractors are pure transformation (text in, graph fragments out). File I/O, import resolution, and graph assembly belong to the indexer.

### CLI UX
- **D-19:** In Phase 1, `cg <path>` runs language detection and prints a scan summary (detected languages, file counts, supported vs. unsupported).
- **D-20:** `cg --version` and `cg --help` work via clap. No extraction or graph output until Phase 2+.

### Test Strategy
- **D-21:** Unit tests on data model (struct construction, serialization, language detection logic).
- **D-22:** Integration tests with fixture files: one small sample file per language in `tests/fixtures/`. Validates tree-sitter grammar crates link and parse without error.
- **D-23:** CLI smoke test: run binary as subprocess against fixture directory, assert exit 0 and correct detection output.
- **D-24:** No mocks, no benchmarks. Real files through real code. Benchmarks arrive in Phase 3.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Context
- `.planning/ROADMAP.md` — Phase 1 goal, success criteria, and dependency chain
- `.planning/REQUIREMENTS.md` — PARS-11 (language auto-detection) and INFR-01 (CLI command)
- `.planning/PROJECT.md` — Tech stack constraints, key decisions, extractor design philosophy

### Research
- `.planning/research/ARCHITECTURE.md` — Component architecture, data flow, anti-patterns
- `.planning/research/PITFALLS.md` — Tree-sitter ABI issues, watch mode event storms (relevant for error handling philosophy)
- `.planning/research/STACK.md` — Technology choices and rationale

No external specs — requirements fully captured in decisions above.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- None — greenfield project, no existing source code.

### Established Patterns
- None yet — Phase 1 establishes the patterns all subsequent phases follow.

### Integration Points
- Phase 2 (TypeScript Extractor) will be the first consumer of the extractor trait and data model defined here.
- Phase 3 (Indexer) will be the first consumer of the CLI scanning and language detection logic.

</code_context>

<specifics>
## Specific Ideas

- OversizeConnect (React Native + Swift) is the first test case — fixture files should reflect the kinds of files found there
- The scan summary output should clearly communicate what cgraph will do vs. what it can't do yet (versioned capability messaging)

</specifics>

<deferred>
## Deferred Ideas

- **Rust extractor** — future phase beyond v1 roadmap
- **Java extractor** — future phase beyond v1 roadmap
- **Ruby extractor** — future phase beyond v1 roadmap
- **Kotlin extractor** — future phase beyond v1 roadmap

</deferred>

---

*Phase: 1-Foundation*
*Context gathered: 2026-05-02*
