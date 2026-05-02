---
phase: 02-typescript-extractor
plan: 01
subsystem: ts-extractor
tags: [rust, tree-sitter, extractor, scaffold, typescript]
dependency_graph:
  requires: [crates/core]
  provides: [crates/ts-extractor]
  affects: [workspace Cargo.toml]
tech_stack:
  added: [tree-sitter-typescript@0.23.2 (already in workspace), cgraph-ts-extractor crate]
  patterns: [Extractor trait impl, two-pass AST extraction, TSX-superset query compilation]
key_files:
  created:
    - crates/ts-extractor/Cargo.toml
    - crates/ts-extractor/src/lib.rs
    - crates/ts-extractor/src/queries.rs
    - crates/ts-extractor/src/symbols.rs
    - crates/ts-extractor/src/edges.rs
    - crates/ts-extractor/src/classify.rs
    - crates/ts-extractor/tests/extraction_test.rs
    - crates/ts-extractor/tests/fixtures/barrel.ts
    - crates/ts-extractor/tests/fixtures/hooks.ts
    - crates/ts-extractor/tests/fixtures/components.tsx
    - crates/ts-extractor/tests/fixtures/schemas.ts
    - crates/ts-extractor/tests/fixtures/services.ts
    - crates/ts-extractor/tests/fixtures/enums.ts
    - crates/ts-extractor/tests/fixtures/index.ts
  modified:
    - Cargo.toml
decisions:
  - "Compile all 5 tree-sitter queries against LANGUAGE_TSX (superset of TS per research Pitfall 2) so the same Query objects work on both .ts and .tsx parse trees"
  - "symbols.rs and edges.rs are intentional stubs returning empty Vecs — extraction logic deferred to Plan 02 and Plan 03"
metrics:
  duration_minutes: 2
  completed_date: "2026-05-02"
  tasks_completed: 2
  files_created: 15
---

# Phase 2 Plan 01: ts-extractor Scaffold Summary

**One-liner:** cgraph-ts-extractor crate scaffolded with TsExtractor struct implementing the Extractor trait, 5 compiled tree-sitter query string constants, classify_function() hook detection, and 7 realistic TypeScript/TSX test fixtures.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Crate scaffold with workspace integration and test fixtures | 45e4ee0 | Cargo.toml, crates/ts-extractor/Cargo.toml, 7 fixture files |
| 2 | TsExtractor struct, query constants, and module stubs | 1254fda | lib.rs, queries.rs, symbols.rs, edges.rs, classify.rs, extraction_test.rs |

## What Was Built

### Crate Structure

`crates/ts-extractor` is now a workspace member with:
- `src/lib.rs` — TsExtractor struct implementing the Extractor trait from cgraph-core. Selects LANGUAGE_TYPESCRIPT or LANGUAGE_TSX based on file extension (.ts vs .tsx). Parses source text, records partial parse errors (D-14), then delegates to symbols and edges modules.
- `src/queries.rs` — 5 tree-sitter S-expression query string constants compiled and validated: SYMBOL_QUERY_SRC (7 patterns covering functions, arrow fns, interfaces, type aliases, classes, enums, default exports), IMPORT_QUERY_SRC (named/default/namespace), CALL_QUERY_SRC (direct identifier calls only per D-30), TYPE_REF_QUERY_SRC (extends/implements/interface extends), REEXPORT_QUERY_SRC (named and star re-exports per D-26).
- `src/classify.rs` — `classify_function()` detects React hooks via `use` prefix + uppercase 4th character (per D-32).
- `src/symbols.rs` — Stub returning empty Vec; ready for Plan 02 symbol extraction implementation.
- `src/edges.rs` — Stub returning empty Vec; ready for Plan 03 edge extraction implementation.

### Test Fixtures

7 TypeScript/TSX fixture files modeling real OversizeConnect-style patterns:
- `barrel.ts` — Named and star re-exports
- `hooks.ts` — React hooks (useCurrentUser, useToggle) with useState/useEffect calls
- `components.tsx` — React component with JSX, ProfileCard with ProfileProps interface
- `schemas.ts` — Interface, type alias, enum, class with extends
- `services.ts` — Classes with extends/implements, function calls, unexported helpers
- `enums.ts` — Numeric and string enum declarations
- `index.ts` — Barrel re-exporting from all other fixtures

### Test Results

All 7 tests pass:
- `extractor_compiles_queries_without_panic` — TsExtractor::new() succeeds; all 5 queries compile against tree-sitter-typescript 0.23.2
- `can_handle_ts_files` — Correctly accepts .ts/.tsx, rejects .js/.rs
- `extract_returns_no_errors_on_valid_ts` — schemas.ts parses without ERROR nodes
- `extract_returns_no_errors_on_valid_tsx` — components.tsx parses without ERROR nodes (JSX grammar)
- `grammar_selection_ts_vs_tsx` — services.ts and components.tsx both parse cleanly under their respective grammars
- `hook_detection` / `non_hook_functions` — classify_function() unit tests

## Deviations from Plan

None — plan executed exactly as written.

The query strings in the plan matched actual tree-sitter-typescript 0.23.2 grammar node types without any adjustments needed. Queries compiled on first attempt.

## Known Stubs

| Stub | File | Line | Reason |
|------|------|------|--------|
| `extract_symbols` returns `Vec::new()` | crates/ts-extractor/src/symbols.rs | 14 | Intentional — Plan 02 implements symbol extraction |
| `extract_edges` returns `Vec::new()` | crates/ts-extractor/src/edges.rs | 16 | Intentional — Plan 03 implements edge extraction |

These stubs are intentional scaffolding. The extraction result nodes and edges will be empty until Plan 02 and Plan 03 fill them in.

## Threat Surface

No new security-relevant surface introduced. The extractor:
- Receives pre-read source text only (no file I/O per D-18)
- Uses `path` parameter only for string construction (symbol IDs), not file I/O (T-02-02 accepted)
- Queries compiled at struct construction with `.expect()` — fail-fast at startup (T-02-03 mitigated)
- tree-sitter has bounded recursion in compiled C grammar (T-02-01 accepted)

## Self-Check: PASSED

Files created:
- crates/ts-extractor/Cargo.toml: FOUND
- crates/ts-extractor/src/lib.rs: FOUND
- crates/ts-extractor/src/queries.rs: FOUND
- crates/ts-extractor/src/symbols.rs: FOUND
- crates/ts-extractor/src/edges.rs: FOUND
- crates/ts-extractor/src/classify.rs: FOUND
- crates/ts-extractor/tests/extraction_test.rs: FOUND
- 7 fixture files: FOUND

Commits:
- 45e4ee0: chore(02-01) scaffold (Task 1)
- 1254fda: feat(02-01) TsExtractor and queries (Task 2)
