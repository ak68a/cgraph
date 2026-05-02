---
phase: 03-indexer-analysis-pipeline
plan: 04
subsystem: cli-integration
tags: [cli, indexer-integration, scan-stats, dead-code-report, cycle-report]
dependency_graph:
  requires: [cgraph-indexer (Plan 01-03), cgraph-ts-extractor, cgraph-core]
  provides: [CLI with indexer pipeline, --dead-code flag, --cycles flag, scan statistics output]
  affects: [crates/cli/src/main.rs, crates/cli/Cargo.toml, crates/cli/tests/cli_smoke.rs]
tech_stack:
  added: []
  patterns: [dynamic extractor registry in CLI, Instant-based timing, HashMap grouping for file-based report output]
key_files:
  created: []
  modified:
    - crates/cli/Cargo.toml
    - crates/cli/src/main.rs
    - crates/cli/tests/cli_smoke.rs
    - crates/indexer/src/lib.rs
decisions:
  - "Removed CodeGraph from CLI import list (unused directly; accessed via indexer.index() return value)"
  - "Updated CLI smoke tests to match new output format rather than maintaining backward compatibility (old format fully replaced)"
metrics:
  duration: "221s"
  completed: "2026-05-02T19:49:25Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 2
  tests_total_workspace: 95
---

# Phase 3 Plan 4: CLI Integration Summary

CLI binary wired to indexer and analysis pipeline with scan statistics output, analysis summary, and --dead-code/--cycles detail flags -- everything built in Plans 01-03 now usable through `cg <path>`.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Update CLI Cargo.toml and add --dead-code and --cycles flags | bee2df7 | Cargo.toml, main.rs, lib.rs |
| 2 | End-to-end CLI integration test with fixture project | 8b94d10 | cli_smoke.rs |

## What Was Built

### CLI Rewrite (main.rs)

- Replaced old `scan_directory()` + `print_summary()` flow with full indexer pipeline
- Builds dynamic extractor registry with `TsExtractor` (D-48)
- Runs `Indexer::new(extractors).index(&cli.path)` with `Instant`-based timing (INFR-03)
- Prints scan statistics: `cgraph scan: {files} files, {symbols} symbols, {edges} edges ({time}ms)`
- Runs `dead_code()` and `detect_cycles()` analysis, prints summary always (D-43)
- `--dead-code` flag: detailed report grouped by file, showing symbol kind, name, line ranges (D-44)
- `--cycles` flag: detailed cycle report with ordered file chains and cycle-closing indicator (D-42, D-44)
- Suspicious dead code entries include the demotion reason string (D-41)
- Path validation retained unchanged (T-01-05)

### New CLI Flags

- `--dead-code`: Prints `print_dead_code_report()` with confirmed and suspicious tiers, grouped by file using sorted HashMap
- `--cycles`: Prints `print_cycles_report()` with numbered cycles and ordered file chains

### Updated Dependencies (Cargo.toml)

- Added `cgraph-indexer = { path = "../indexer" }`
- Added `cgraph-ts-extractor = { path = "../ts-extractor" }`

### Updated Tests (cli_smoke.rs)

- `scan_fixture_directory`: checks for "cgraph scan:" with files/symbols/edges
- `scan_prints_analysis_summary`: checks for analysis section with dead code and cycle counts (replaces old `scan_detects_typescript`)
- `dead_code_flag`: verifies --dead-code produces dead code output (replaces old `verbose_flag_shows_files`)
- `cycles_flag`: verifies --cycles produces circular dependencies output (new test)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing `dead_code` re-export to indexer lib.rs**
- **Found during:** Task 1
- **Issue:** `dead_code` function was not re-exported from `crates/indexer/src/lib.rs` despite the plan's interface doc listing it. Compilation failed with `no dead_code in the root`.
- **Fix:** Added `dead_code` to the `pub use analysis::{...}` line in lib.rs
- **Files modified:** crates/indexer/src/lib.rs
- **Commit:** bee2df7

## Test Results

95 total workspace tests passing (84 from prior plans + 8 CLI smoke tests + 2 net new + 1 renamed):
- CLI smoke tests: 8 passing (4 unchanged + 2 updated + 2 new)
- Indexer tests: 35 passing
- ts-extractor tests: 33 passing
- Core tests: 12 + 5 + 2 = 19 passing

Zero regressions.

## End-to-End Verification

```
$ cg crates/ts-extractor/tests/fixtures/
cgraph scan: 5 files, 14 symbols, 0 edges (6ms)

analysis:
  dead code: 0 confirmed, 0 suspicious
  circular dependencies: 0

$ cg crates/ts-extractor/tests/fixtures/ --dead-code
[...scan stats and summary...]
dead code: none found

$ cg crates/ts-extractor/tests/fixtures/ --cycles
[...scan stats and summary...]
circular dependencies: none found
```

## Verification

- `cargo build -p cg` compiles with zero errors and zero warnings
- `cargo run -- crates/ts-extractor/tests/fixtures/` produces scan stats + analysis summary
- `cargo run -- crates/ts-extractor/tests/fixtures/ --dead-code` produces dead code report
- `cargo run -- crates/ts-extractor/tests/fixtures/ --cycles` produces cycle report
- `cargo test --workspace` -- 95 tests pass, zero failures

## Self-Check: PASSED

All files exist (5/5), both commits verified (bee2df7, 8b94d10), all acceptance criteria met.
