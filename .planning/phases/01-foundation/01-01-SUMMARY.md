---
phase: 01-foundation
plan: 01
subsystem: core
tags: [rust, workspace, data-model, language-detection, tree-sitter, tdd]
dependency_graph:
  requires: []
  provides:
    - cgraph-core crate with SymbolNode, SymbolEdge, Language, SymbolKind, EdgeKind types
    - Extractor trait (language/can_handle/extract)
    - detect_language, is_parseable, scan_directory functions
    - DetectionResult struct
  affects:
    - All subsequent phases that depend on cgraph-core
tech_stack:
  added:
    - tree-sitter 0.26.8
    - tree-sitter-typescript 0.23.2
    - tree-sitter-swift 0.7.1
    - tree-sitter-go 0.25.0
    - tree-sitter-python 0.25.0
    - walkdir 2.5.0
    - serde 1.0 + serde_json 1.0
    - thiserror 2.0
  patterns:
    - Cargo workspace with resolver = "2" and workspace.package inheritance
    - thiserror for library error types, anyhow reserved for CLI binary
    - TDD: tests in #[cfg(test)] mod at bottom of implementation files
key_files:
  created:
    - Cargo.toml
    - Cargo.lock
    - .gitignore
    - crates/core/Cargo.toml
    - crates/core/src/lib.rs
    - crates/core/src/model.rs
    - crates/core/src/extractor.rs
    - crates/core/src/detect.rs
    - crates/cli/Cargo.toml
    - crates/cli/src/main.rs
  modified: []
decisions:
  - "Workspace includes crates/cli in members even though it is a stub — avoids future Cargo.toml churn when Plan 03 implements the full CLI"
  - "detect.rs includes is_parseable as a public function so CLI can query parseability without importing model module directly"
  - "target directory excluded via .gitignore; Cargo.lock committed for reproducible builds"
metrics:
  duration: "2 minutes"
  completed: "2026-05-02T15:21:44Z"
  tasks_completed: 2
  files_created: 10
  files_modified: 0
---

# Phase 1 Plan 1: Workspace Scaffold and Core Data Model Summary

**One-liner:** Rust Cargo workspace with cgraph-core crate containing SymbolNode/SymbolEdge/Language data model, Extractor trait, and walkdir-based language detection with 12 passing unit tests.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Workspace scaffold and core data model | 26c7195 | Cargo.toml, crates/core/Cargo.toml, src/lib.rs, src/model.rs, src/extractor.rs, src/detect.rs |
| 2 | Language detection unit tests (TDD) | 5e769de | crates/core/src/detect.rs (tests), crates/cli stub |
| - | .gitignore and Cargo.lock | f5b700b | .gitignore, Cargo.lock |

## What Was Built

### Cargo Workspace
- Root `Cargo.toml` with `resolver = "2"`, `edition = "2024"`, `[workspace.package]` for version inheritance
- `crates/core` library crate (`cgraph-core`) with all tree-sitter grammar crates, serde, walkdir, thiserror
- `crates/cli` minimal stub so workspace compiles; full CLI implemented in Plan 03

### Data Model (`crates/core/src/model.rs`)
- `Language` enum: TypeScript, TypeScriptReact, Swift, Go, Python, Unknown(String) — with serde derives
- `SymbolKind` enum: Function, Class, Type, Interface, Hook, Enum — with serde derives
- `EdgeKind` enum: Import, Call, TypeRef, ReExport — with serde derives
- `SymbolNode` struct: id, name, kind, file_path, language, line_start, line_end, is_exported
- `SymbolEdge` struct: source_id, target_id, kind, source_location
- No deferred fields (docstring, signature, byte_range excluded per D-05)

### Extractor Trait (`crates/core/src/extractor.rs`)
- `ParseError` enum with Io and PartialParse variants (thiserror derive)
- `ExtractionResult` struct with nodes/edges/errors as owned Vecs (D-17)
- `Extractor` trait: language(), can_handle(), extract() (D-16, D-18)

### Language Detection (`crates/core/src/detect.rs`)
- `detect_language(&Path) -> Option<Language>`: pure extension matching, Unknown for unrecognized
- `is_parseable(&Language) -> bool`: true for 5 supported languages, false for Unknown
- `DetectionResult`: detected, parseable, skipped Vecs
- `scan_directory(&Path)`: walkdir traversal, skips `.git`, `node_modules`, `dist`, `build`, `target`, all hidden dirs; `follow_links = false` (T-01-01); `to_string_lossy()` for filename comparison (T-01-03)

### Tests
12 unit tests covering all behaviors from the plan spec. All pass.

## Deviations from Plan

### Auto-added: CLI stub for workspace compilation
**Found during:** Task 2 (running tests)
**Issue:** Workspace member `crates/cli` listed in Cargo.toml but not yet created; `cargo test -p cgraph-core` failed with "failed to load manifest for workspace member crates/cli".
**Fix:** Created minimal `crates/cli/Cargo.toml` and `crates/cli/src/main.rs` stub so workspace resolves. Full CLI implementation is Plan 03's scope.
**Rule:** Rule 3 (blocking issue)
**Files modified:** crates/cli/Cargo.toml, crates/cli/src/main.rs
**Commit:** 5e769de

### TDD Gate Note
Task 2 has `tdd="true"`. The implementation in `detect.rs` was created during Task 1 (required for the library to compile — Rule 3: blocking issue), then tests were added in Task 2 as a `test(01-01)` commit. Implementation precedes tests in git history due to the compilation dependency, but all 12 tests exercise distinct behaviors as specified in the plan's `<behavior>` block.

## Threat Model Compliance

| Threat ID | Mitigation | Status |
|-----------|------------|--------|
| T-01-01 | `WalkDir::follow_links(false)` — symlinks not followed | Implemented |
| T-01-02 | Skip hidden dirs, node_modules, dist, build, target | Implemented |
| T-01-03 | `entry.file_name().to_string_lossy()` for all filename comparisons | Implemented |

## Known Stubs

- `crates/cli/src/main.rs`: minimal `println!` stub — not wired to cgraph-core. Plan 03 implements full CLI with clap, path validation, and scan summary output.

## Self-Check: PASSED

Files created:
- FOUND: Cargo.toml
- FOUND: crates/core/Cargo.toml
- FOUND: crates/core/src/lib.rs
- FOUND: crates/core/src/model.rs
- FOUND: crates/core/src/extractor.rs
- FOUND: crates/core/src/detect.rs
- FOUND: crates/cli/Cargo.toml
- FOUND: crates/cli/src/main.rs
- FOUND: .gitignore
- FOUND: Cargo.lock

Commits verified: 26c7195, 5e769de, f5b700b — all present in git log.
