---
phase: 02-typescript-extractor
verified: 2026-05-02T18:00:00Z
status: gaps_found
score: 4/6 roadmap success criteria verified
overrides_applied: 0
gaps:
  - truth: "A symbol re-exported through one or more barrel files traces back to its defining file, not the barrel (ROADMAP SC5, PARS-09)"
    status: failed
    reason: "Phase 2 emits single-hop raw ReExport edges only. Multi-hop chain resolution to the true defining file is not implemented anywhere. Phase 3 roadmap does not include PARS-09 in its requirements list, leaving this requirement unowned."
    artifacts:
      - path: "crates/ts-extractor/src/edges.rs"
        issue: "extract_reexports emits raw single-hop edges as intended, but no resolver exists or is planned"
      - path: ".planning/ROADMAP.md"
        issue: "Phase 3 requirements list (ANLS-01 to ANLS-05, INFR-03) does not include PARS-09"
    missing:
      - "Either implement chain resolution in Phase 2, or add PARS-09 to Phase 3 requirements and update ROADMAP.md, or create an override accepting the raw-edge design"
  - truth: "tsconfig paths aliases in import statements resolve to the correct file path (ROADMAP SC6, PARS-10)"
    status: failed
    reason: "Phase 2 emits import paths raw (no alias resolution). No tsconfig paths resolver exists anywhere in the codebase. Phase 3 roadmap does not include PARS-10 in its requirements list, leaving this requirement unowned."
    artifacts:
      - path: "crates/ts-extractor/src/edges.rs"
        issue: "extract_imports emits path strings as-is from source text — @/foo aliases are not resolved"
      - path: ".planning/ROADMAP.md"
        issue: "Phase 3 requirements list does not include PARS-10"
    missing:
      - "Either implement tsconfig path alias resolution in Phase 2, or add PARS-10 to Phase 3 requirements and update ROADMAP.md, or create an override accepting raw alias emission"
---

# Phase 2: TypeScript Extractor Verification Report

**Phase Goal:** Implement TypeScript/TSX extractor using tree-sitter — exported symbols (functions, classes, types, interfaces, enums, hooks) and edges (imports, calls, type references, re-exports).
**ROADMAP Goal (exact):** Users can point cgraph at a TypeScript/React Native project and get a complete, accurate graph of all symbols and their relationships, including barrel re-exports resolved to their true source and tsconfig path aliases resolved to real file paths.
**Verified:** 2026-05-02T18:00:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | Running against a TypeScript project extracts all exported functions, components, hooks, types, classes, and interfaces from .ts/.tsx files | VERIFIED | 8 symbol extraction tests pass: exported_functions_extracted, exported_types_extracted, hook_detection, tsx_components_extracted, exported_enums_extracted, exported_classes_extracted, non_exported_functions_captured, symbol_id_format |
| SC2 | Import edges between files are present and correctly directed (importer → importee) | VERIFIED | import_named, import_named_relative, import_default, import_raw_alias_path all pass; target_id format is "path::symbol_name" |
| SC3 | Function call edges are present where functions call other named functions | VERIFIED | call_direct and call_no_member pass; member expressions (obj.method) correctly excluded |
| SC4 | Type reference edges (extends, implements, uses-type) are present | VERIFIED | type_ref_extends, type_ref_implements, type_ref_iface_extends all pass |
| SC5 | A symbol re-exported through one or more barrel files traces back to its defining file, not the barrel | FAILED | Phase 2 emits single-hop raw ReExport edges only. PARS-09 resolution is not implemented and is not assigned to Phase 3 in ROADMAP.md |
| SC6 | tsconfig paths aliases in import statements resolve to the correct file path | FAILED | Phase 2 emits alias paths raw (e.g., @/components/Button is preserved as-is). PARS-10 resolver is not implemented and is not assigned to Phase 3 in ROADMAP.md |

**Score:** 4/6 roadmap success criteria verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/ts-extractor/Cargo.toml` | Package definition with cgraph-core, tree-sitter, tree-sitter-typescript deps | VERIFIED | name=cgraph-ts-extractor, all three deps present at correct versions |
| `crates/ts-extractor/src/lib.rs` | TsExtractor struct with Extractor trait impl | VERIFIED | TsExtractor struct defined, impl Extractor block present, all 5 queries compiled against tsx_lang |
| `crates/ts-extractor/src/queries.rs` | All tree-sitter query string constants | VERIFIED | SYMBOL_QUERY_SRC, IMPORT_QUERY_SRC, CALL_QUERY_SRC, TYPE_REF_QUERY_SRC, REEXPORT_QUERY_SRC all present and non-empty |
| `crates/ts-extractor/src/symbols.rs` | Pass 1 symbol extraction (not a stub) | VERIFIED | extract_symbols fully implemented; constructs SymbolNode structs; no stub comment |
| `crates/ts-extractor/src/edges.rs` | Pass 2 edge extraction (not a stub) | VERIFIED | extract_edges dispatches to extract_imports, extract_calls, extract_type_refs, extract_reexports; all four functions fully implemented |
| `crates/ts-extractor/src/classify.rs` | Hook/function classification | VERIFIED | classify_function present with use* + uppercase-4th-char logic and unit tests |
| `crates/ts-extractor/tests/extraction_test.rs` | Integration tests for symbols and edges | VERIFIED | 27 integration tests present, all passing |
| `crates/ts-extractor/tests/fixtures/barrel.ts` | Re-export test fixture with export * from | VERIFIED | Contains export * from './hooks' and named re-exports |
| `Cargo.toml` (workspace) | crates/ts-extractor in workspace members | VERIFIED | "crates/ts-extractor" present in members array |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| Cargo.toml | crates/ts-extractor | workspace members array | WIRED | Line 5: "crates/ts-extractor" |
| crates/ts-extractor/src/lib.rs | cgraph-core | use cgraph_core | WIRED | Line 9-12: use cgraph_core::{Extractor, ExtractionResult, ParseError, Language, ...} |
| crates/ts-extractor/src/symbols.rs | crates/ts-extractor/src/queries.rs | symbol_query parameter | WIRED | symbol_query: &Query parameter passed from lib.rs which was compiled from SYMBOL_QUERY_SRC |
| crates/ts-extractor/src/symbols.rs | cgraph-core model | SymbolNode construction | WIRED | SymbolNode { ... } constructed at line 62 |
| crates/ts-extractor/src/symbols.rs | crates/ts-extractor/src/classify.rs | classify_function | WIRED | use crate::classify::classify_function at line 3, called at line 57 |
| crates/ts-extractor/src/lib.rs | crates/ts-extractor/src/edges.rs | edges::extract_edges call | WIRED | Line 121: edges::extract_edges(root, source, ...) |
| crates/ts-extractor/src/edges.rs | cgraph-core model | SymbolEdge construction | WIRED | SymbolEdge { ... } constructed throughout edges.rs |

### Data-Flow Trace (Level 4)

This is a library crate (no rendering). Data flows from source text input → tree-sitter parse → query matches → SymbolNode/SymbolEdge construction → returned ExtractionResult. The integration tests act as the data-flow oracle.

| Artifact | Data Source | Produces Real Data | Status |
|----------|-------------|-------------------|--------|
| symbols.rs extract_symbols | QueryCursor::matches + tree walk | Yes — 27 passing tests confirm real SymbolNode structs with correct names, kinds, is_exported | FLOWING |
| edges.rs extract_edges | QueryCursor::matches × 4 query types | Yes — 27 passing tests confirm real SymbolEdge structs with correct kinds and target_ids | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All tests pass | `cargo test -p cgraph-ts-extractor` | 29 passed (2 unit + 27 integration), 0 failed | PASS |
| Full workspace builds | `cargo build` | Finished dev profile, 0 errors (2 warnings) | PASS |
| Hook classification: useCurrentUser → Hook | test hook_detection | PASSED | PASS |
| Member call exclusion: obj.method() not captured | test call_no_member | PASSED | PASS |
| Star re-export wildcard edge | test reexport_star | PASSED | PASS |
| Raw alias path preserved | test import_raw_alias_path | PASSED | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| PARS-01 | 02-01, 02-02 | Tool parses TypeScript/TSX files and extracts all exported symbols | SATISFIED | 8 symbol tests pass; all 6 symbol kinds (Function, Hook, Interface, Type, Class, Enum) extracted correctly |
| PARS-05 | 02-03 | Tool extracts import relationships between modules | SATISFIED | import_named, import_named_relative, import_default tests pass; Import edges produced |
| PARS-06 | 02-03 | Tool extracts function/method call relationships | SATISFIED | call_direct, call_no_member tests pass; member calls excluded |
| PARS-07 | 02-03 | Tool extracts type reference relationships | SATISFIED | type_ref_extends, type_ref_implements, type_ref_iface_extends tests pass |
| PARS-08 | 02-03 | Tool extracts re-export relationships (barrel files) | SATISFIED | reexport_named, reexport_star, reexport_raw_path tests pass; per-specifier and wildcard edges produced |
| PARS-09 | 02-03 | Tool resolves multi-hop barrel re-export chains to find the true source | BLOCKED | Phase 2 emits single-hop raw edges only, citing Phase 3 deferral. However Phase 3 ROADMAP requirements list does not include PARS-09. Requirement is unowned across all phases. |
| PARS-10 | 02-03 | Tool resolves TypeScript path aliases (tsconfig paths) | BLOCKED | Phase 2 emits raw alias strings, citing Phase 3 deferral. However Phase 3 ROADMAP requirements list does not include PARS-10. Requirement is unowned across all phases. |

**Orphaned requirements:** PARS-09 and PARS-10 are mapped to Phase 2 in REQUIREMENTS.md traceability but Phase 3 ROADMAP does not list them. No later phase claims these requirements. They are at risk of falling through entirely.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| crates/ts-extractor/src/lib.rs | 15 | `ts_lang` field is never read (dead code) | Warning | Compiler warning; misleads readers into thinking ts_lang is used for TS files |
| crates/ts-extractor/src/lib.rs | 11 | Unused imports: SymbolNode, SymbolEdge | Warning | Compiler warning; minor clarity issue |
| crates/ts-extractor/src/edges.rs | 252-265 | `export * as ns from` misclassified as star re-export (review finding CR-01) | Warning | Incorrect edge emitted for namespace re-export pattern; no test currently covers this case |
| crates/ts-extractor/src/symbols.rs | 24-75 | No deduplication of SymbolNode for overloaded functions (review finding WR-01) | Warning | Multiple SymbolNode entries with identical `id` field for overloaded TypeScript functions |
| crates/ts-extractor/tests/fixtures/barrel.ts | 2 | `UserSchema` exported but not defined in schemas.ts (review finding IN-02) | Info | Semantically invalid fixture; could mislead future test authors |

Note: The `Vec::new()` occurrences in symbols.rs (line 18) and edges.rs (line 21) are local mutable accumulators that are populated by subsequent logic — they are NOT stubs. The old stubs (returning Vec::new() immediately) have been replaced with full implementations.

### Human Verification Required

None. All phase deliverables are verifiable programmatically.

### Gaps Summary

Two of the six ROADMAP success criteria are not achieved:

**Gap 1 — PARS-09 (multi-hop barrel resolution):** Phase 2 intentionally defers barrel re-export chain resolution to a later phase, but no phase in the ROADMAP currently owns PARS-09. The REQUIREMENTS.md traceability table maps PARS-09 to Phase 2, and Phase 3's requirements list does not include it. The requirement is orphaned — it will never be satisfied unless a phase explicitly claims it.

**Gap 2 — PARS-10 (tsconfig path alias resolution):** Same situation. Phase 2 emits alias paths raw, Phase 3 does not claim PARS-10, and the REQUIREMENTS.md traceability maps it to Phase 2. The requirement is orphaned.

These gaps share a root cause: the plans for Phase 2 reinterpreted PARS-09 and PARS-10 as "emit raw, resolve later" without updating the ROADMAP to assign the resolution work to a later phase.

**What's needed:** One of the following for each gap:
1. Implement the resolution in Phase 2 (add the work to the codebase now)
2. Update ROADMAP.md Phase 3 requirements to include PARS-09 and PARS-10, and add matching success criteria
3. Create a VERIFICATION.md override with a documented rationale and acceptance, acknowledging the raw-edge design is the intended architecture and resolution is explicitly assigned to a named later phase

The raw-edge approach is architecturally sound (extractors should be stateless, resolution belongs in the indexer layer). The issue is not the technical choice but the missing roadmap assignment.

---

_Verified: 2026-05-02T18:00:00Z_
_Verifier: Claude (gsd-verifier)_
