---
phase: 02-typescript-extractor
plan: 02
subsystem: ts-extractor
tags: [rust, tree-sitter, symbol-extraction, tdd, pars-01]
dependency_graph:
  requires: [crates/ts-extractor (Plan 01 scaffold), crates/core/model.rs]
  provides: [PARS-01 symbol extraction, classify_function hook detection]
  affects: [crates/ts-extractor/src/symbols.rs, crates/ts-extractor/src/lib.rs]
tech_stack:
  added: []
  patterns: [StreamingIterator query matching, tree-sitter capture index lookup, AST tree walk for non-exported symbols]
key_files:
  created: []
  modified:
    - crates/ts-extractor/src/symbols.rs
    - crates/ts-extractor/src/lib.rs
    - crates/ts-extractor/tests/extraction_test.rs
decisions:
  - "Always use tsx_lang grammar for parsing both .ts and .tsx files — tree-sitter query engine requires node language to match query language; TSX is a strict superset of TS so all .ts syntax parses correctly under TSX grammar"
  - "Non-exported functions captured via top-level tree walk (not query) — query only matches export_statement patterns; separate tree walk handles bare function_declaration and lexical_declaration"
metrics:
  duration_minutes: 6
  completed_date: "2026-05-02"
  tasks_completed: 2
  files_created: 0
  files_modified: 3
---

# Phase 2 Plan 02: Symbol Extraction (Pass 1) Summary

**One-liner:** Pass 1 symbol extraction implemented via tree-sitter query matching (exported functions/interfaces/types/classes/enums/hooks) plus AST tree walk for non-exported functions, with hook classification via use* naming convention.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 (RED) | Add failing PARS-01 tests | 8fd37bf | crates/ts-extractor/tests/extraction_test.rs |
| 1 (GREEN) | Implement symbol extraction + fix grammar bug | 7c6625e | crates/ts-extractor/src/symbols.rs, lib.rs |
| 2 | Integration tests all passing (included in RED/GREEN) | 7c6625e | — |

## What Was Built

### `crates/ts-extractor/src/symbols.rs`

`extract_symbols()` implements Pass 1 of the two-pass extraction algorithm:

1. **Query-based exported symbol extraction**: Uses the pre-compiled `symbol_query` (from `queries.rs`) via `QueryCursor::matches()` with `StreamingIterator`. Iterates matches, maps pattern index to `SymbolKind` (0=Function, 1=Function/Arrow, 2=Interface, 3=Type, 4=Class, 5=Enum), finds the `@symbol_name` capture by index, builds `SymbolNode` with `is_exported=true`.

2. **Hook reclassification**: Functions are passed through `classify_function()` from `classify.rs` — if they match `use` prefix + uppercase 4th character (D-32), they become `SymbolKind::Hook`.

3. **Non-exported function capture**: A separate top-level tree walk via `goto_first_child()`/`goto_next_sibling()` captures `function_declaration` and `lexical_declaration` (const arrow functions) not wrapped in `export_statement`. These get `is_exported=false` for intra-file call edge resolution in Pass 2.

### `crates/ts-extractor/src/lib.rs` (bug fix)

Fixed grammar/query language mismatch: always use `tsx_lang` for parsing, not `ts_lang` for `.ts` files. See Deviations.

### `crates/ts-extractor/tests/extraction_test.rs`

8 new PARS-01 integration tests added (TDD RED committed first, then GREEN):
- `exported_functions_extracted` — fetchUser found as Function, is_exported=true, correct ID
- `exported_types_extracted` — UserType=Interface, UserRole=Type, Permission=Enum, ValidationError=Class
- `hook_detection` — useCurrentUser=Hook, useToggle=Hook
- `tsx_components_extracted` — ProfileCard=Function, exported (TSX file)
- `exported_enums_extracted` — Direction=Enum, Status=Enum
- `non_exported_functions_captured` — fetchFromDb found, is_exported=false
- `symbol_id_format` — all IDs contain `::` with correct file path prefix
- `exported_classes_extracted` — UserRepository=Class, UserService=Class, is_exported=true

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed tree-sitter query/grammar language mismatch in lib.rs**

- **Found during:** Task 1 (GREEN phase — tests still failing after implementing symbols.rs)
- **Issue:** `lib.rs` compiled all queries against `LANGUAGE_TSX` (per Plan 01 decision "TSX is a superset") but then parsed `.ts` files with `LANGUAGE_TYPESCRIPT`. Tree-sitter's query engine performs an internal language identity check — a query compiled against `tsx_lang` returns zero matches when run against nodes parsed with `ts_lang`, even though the node types are identical. The Plan 01 decision was architecturally correct in intent but incorrect in implementation.
- **Evidence:** Debug binary confirmed: hooks.ts parsed with TS grammar → 0 symbols found; same file with TSX grammar → 2 symbols found (useCurrentUser, useToggle). The `tsx_components_extracted` test passed because `components.tsx` correctly uses tsx_lang; other `.ts` fixture tests failed because they used ts_lang.
- **Fix:** Changed `lib.rs` line 78 to always use `&self.tsx_lang` for parsing. `is_tsx` flag retained for the `cg_language` enum assignment (TypeScript vs TypeScriptReact). TSX grammar is a strict superset of TypeScript; all `.ts` files parse correctly under TSX without errors.
- **Files modified:** `crates/ts-extractor/src/lib.rs`
- **Commit:** `7c6625e`

## TDD Gate Compliance

RED commit: `8fd37bf` (test(02-02): add failing tests for PARS-01 symbol extraction)
GREEN commit: `7c6625e` (feat(02-02): implement Pass 1 symbol extraction (PARS-01))

Gate sequence: RED → GREEN. REFACTOR not needed (code is clean on first pass).

## Known Stubs

None in Plan 02 scope. The `edges.rs` stub (`extract_edges` returns `Vec::new()`) is intentional and tracked from Plan 01 — it is Plan 03's responsibility.

## Threat Surface

No new security-relevant surface. Analysis:
- `extract_symbols` processes source text from the caller — same trust boundary as Plan 01 (T-02-04, T-02-05 accepted)
- `extract_non_exported_functions` walks AST tree — bounded by tree-sitter's grammar; no recursion beyond top-level children
- Symbol names appear in IDs — by design (T-02-05 accepted in threat model)

## Self-Check: PASSED

Files modified:
- crates/ts-extractor/src/symbols.rs: FOUND
- crates/ts-extractor/src/lib.rs: FOUND
- crates/ts-extractor/tests/extraction_test.rs: FOUND

Commits:
- 8fd37bf: test(02-02) RED phase
- 7c6625e: feat(02-02) GREEN phase
- 5725fa0: chore(02-02) Cargo.lock

Test results: 15 passed, 0 failed (cargo test -p cgraph-ts-extractor)
