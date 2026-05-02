# Phase 2: TypeScript Extractor - Context

**Gathered:** 2026-05-02
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers the TypeScript/TSX extractor — a new crate (`crates/ts-extractor`) that implements the `Extractor` trait from Phase 1. Given a `.ts` or `.tsx` file's source text, it produces `SymbolNode` and `SymbolEdge` vectors covering: exported symbols (functions, components, hooks, types, classes, interfaces, enums), import edges, direct function call edges, type reference edges (extends, implements, uses-type), and re-export edges (named and star). No import path resolution, no barrel chain resolution, no tsconfig awareness — those belong to the indexer in Phase 3.

Requirements: PARS-01, PARS-05, PARS-06, PARS-07, PARS-08, PARS-09, PARS-10

</domain>

<decisions>
## Implementation Decisions

### Barrel Re-Export Resolution
- **D-25:** Extractor emits ReExport edges only — does NOT follow re-export chains across files. The indexer (Phase 3) resolves multi-hop barrel chains to find the true source. This preserves D-18 (extractors are pure transformation, no file I/O).
- **D-26:** Supported re-export patterns: named (`export { foo, bar } from './module'`) and star (`export * from './module'`). Star re-exports emit a single edge with a wildcard marker for the indexer to resolve.
- **D-27:** Renamed re-exports (`export { foo as bar }`) and default re-exports (`export { default as Foo }`) are deferred — can be added in gap closure if needed.

### Path Alias Resolution
- **D-28:** Extractor emits raw import paths as-is (e.g., `@/components/Button`). No tsconfig.json reading, no alias resolution. The indexer (Phase 3) reads tsconfig.json once and resolves all alias paths during graph assembly.
- **D-29:** This means Phase 2's extractor crate has zero tsconfig awareness. PARS-10 (path alias resolution) is split: Phase 2 captures the raw import, Phase 3 resolves it.

### Call Edge Detection
- **D-30:** Direct named calls only — `foo()`, `Component()`, `useHook()`. Top-level identifiers in call expressions. Skip method calls (`obj.method()`), dynamic dispatch, callbacks, and IIFE.
- **D-31:** This captures the most valuable signal for blast radius and dead code with the lowest false-positive rate. Member call detection (obj.method()) with exclusion filtering is deferred as a post-v1 enhancement.

### Symbol Extraction
- **D-32:** Extract all exported symbols: functions, arrow functions, components (detected via JSX return + PascalCase), hooks (use* prefix convention), types, interfaces, classes, enums.
- **D-33:** `is_exported` flag set based on `export` keyword presence. Default exports are captured.
- **D-34:** Symbol IDs follow D-01: `file_path::symbol_name` format.

### Crate Structure
- **D-35:** New crate `crates/ts-extractor` with dependency on `cgraph-core`. Implements `Extractor` trait for TypeScript and TypeScriptReact languages.
- **D-36:** Uses `tree-sitter-typescript` crate (already validated in Phase 1 grammar tests). LANGUAGE_TYPESCRIPT for .ts, LANGUAGE_TSX for .tsx (Pitfall 3 from research).

### Test Strategy
- **D-37:** OversizeConnect-inspired fixtures — synthetic files modeled after real patterns: barrel re-exports, hook files, typed schemas, component files with JSX, import chains.
- **D-38:** Assertions verify key patterns, not exact counts. Assert specific symbols exist by name/kind, specific edges exist by source/target/kind, and no ERROR nodes in parse. Resilient to fixture tweaks.
- **D-39:** Test fixture directory: `crates/ts-extractor/tests/fixtures/` with realistic multi-file structure.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Context
- `.planning/ROADMAP.md` — Phase 2 goal, success criteria, dependency chain
- `.planning/REQUIREMENTS.md` — PARS-01, PARS-05 through PARS-10
- `.planning/PROJECT.md` — Tech stack constraints, extractor design philosophy

### Prior Phase Context
- `.planning/phases/01-foundation/01-CONTEXT.md` — D-01 through D-24 (symbol ID format, node/edge fields, extractor trait interface, error handling philosophy)
- `.planning/phases/01-foundation/01-RESEARCH.md` — Tree-sitter ABI validation, grammar crate versions, pitfalls

### Existing Code (from Phase 1)
- `crates/core/src/model.rs` — SymbolNode, SymbolEdge, Language, SymbolKind, EdgeKind definitions
- `crates/core/src/extractor.rs` — Extractor trait, ExtractionResult, ParseError
- `crates/core/src/lib.rs` — Public API re-exports
- `crates/core/tests/grammar_test.rs` — Proven grammar linkage patterns for TypeScript/TSX

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Extractor` trait in `crates/core/src/extractor.rs` — the interface this phase implements
- `tree-sitter-typescript` crate with `LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX` — proven working in Phase 1 grammar tests
- Grammar test patterns in `crates/core/tests/grammar_test.rs` — shows correct parser setup (Parser::new, set_language, parse, root_node)
- Fixture files in `crates/core/tests/fixtures/sample.ts` and `sample.tsx` — minimal but valid parse targets

### Established Patterns
- Crate per concern: `crates/core` for shared types, `crates/cli` for binary, `crates/ts-extractor` for this phase
- Tree-sitter grammar usage: `LANGUAGE_TYPESCRIPT.into()` for set_language (not bare `LANGUAGE`)
- Test fixtures as real files parsed by real tree-sitter (D-24: no mocks)

### Integration Points
- Phase 3 (Indexer) will be the first consumer of ExtractionResult from this extractor
- Phase 3 will handle: barrel chain resolution (D-25), tsconfig alias resolution (D-28), and graph assembly
- CLI (`crates/cli`) will need to register the TS extractor — either in Phase 2 or Phase 3

</code_context>

<specifics>
## Specific Ideas

- OversizeConnect is the proving ground — fixtures should reflect its patterns (barrel re-exports through index.ts files, Zod schemas with type references, Firebase service files, React Navigation types)
- Hook detection via `use*` naming convention is React-specific but reliable for this codebase
- JSX component detection via PascalCase + JSX return is a heuristic — document edge cases

</specifics>

<deferred>
## Deferred Ideas

- **Member call edges (obj.method())** — valuable for capturing SDK and API client calls. Add with exclusion list filtering (console.*, Array.*, etc.) post-v1 or as enhancement phase.
- **Renamed re-exports** (`export { foo as bar }`) — add in gap closure if barrel resolution needs it
- **Default re-exports** (`export { default as Foo }`) — add in gap closure if needed
- **Import type detection** (`import type { Foo }`) — distinguish type-only imports from value imports for more precise edges
- **Decorator extraction** — `@Injectable()`, `@Component()` patterns if Angular/NestJS support is desired later

</deferred>

---

*Phase: 2-TypeScript Extractor*
*Context gathered: 2026-05-02*
