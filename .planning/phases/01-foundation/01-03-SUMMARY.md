---
phase: 01-foundation
plan: "03"
subsystem: cli
tags: [cli, clap, integration-tests, assert_cmd]
dependency_graph:
  requires: [01-01]
  provides: [cg-binary, cli-smoke-tests]
  affects: []
tech_stack:
  added: [clap-4.6.1, anyhow-1.0, assert_cmd-2.0, predicates-3.0]
  patterns: [clap-derive-cli, anyhow-main-error-propagation, subprocess-integration-tests]
key_files:
  created:
    - crates/cli/tests/cli_smoke.rs
    - crates/core/tests/fixtures/sample.ts
  modified:
    - crates/cli/Cargo.toml
    - crates/cli/src/main.rs
    - Cargo.lock
decisions:
  - Smoke tests placed in crates/cli/tests/ instead of workspace root tests/ because virtual workspace manifests do not support [dev-dependencies]; dev-deps in crates/cli/Cargo.toml already satisfy assert_cmd and predicates
  - Used CARGO_MANIFEST_DIR + workspace_root() helper to construct absolute fixture paths, avoiding CWD-dependent relative path failures
  - Created minimal crates/core/tests/fixtures/sample.ts here rather than waiting for plan 02 to unblock the TypeScript detection smoke tests
metrics:
  duration: "3 minutes"
  completed: "2026-05-02T15:28:34Z"
  tasks_completed: 2
  files_changed: 5
---

# Phase 01 Plan 03: CLI Binary and Smoke Tests Summary

Delivered the `cg` binary with clap-based argument parsing, path validation, scan summary output, and 7 passing subprocess integration tests.

## What Was Built

**`cg` binary** (`crates/cli/src/main.rs`): Full clap CLI replacing the plan-01 stub. Accepts `<PATH>` as a required positional argument and `--verbose` / `-v` as an optional flag. Validates path existence and directory-ness before scanning (security mitigation T-01-05). Calls `cgraph_core::scan_directory()` and formats a human-readable summary distinguishing parseable files (by language) from skipped files (by extension). Verbose mode lists every file individually.

**CLI smoke tests** (`crates/cli/tests/cli_smoke.rs`): 7 subprocess integration tests using `assert_cmd` and `predicates` that invoke the compiled `cg` binary via `Command::cargo_bin("cg")`. Tests cover: `--version` prints `0.1.0`, `--help` shows about text and path argument, nonexistent path exits nonzero with error message, file-not-directory exits nonzero, valid directory scan prints summary, TypeScript detection in fixtures, verbose mode shows `[parseable]` file list.

**TypeScript fixture** (`crates/core/tests/fixtures/sample.ts`): Minimal valid TypeScript file (matching the content plan 02 will also create) so the TypeScript detection smoke tests have a real parseable file.

## Task Results

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | CLI binary crate with scan summary | 303a67a | crates/cli/Cargo.toml, crates/cli/src/main.rs |
| 2 | CLI smoke tests | 4c11ecf | crates/cli/tests/cli_smoke.rs, crates/core/tests/fixtures/sample.ts |

## Verification

```
cargo build -p cg      # exits 0, no warnings
cargo run -p cg -- --version   # prints "cg 0.1.0"
cargo run -p cg -- --help      # shows "Code graph visualization — cgraph"
cargo run -p cg -- ./crates    # prints scan summary (rs/toml as skipped)
cargo run -p cg -- /nonexistent  # exits 1 with "Path does not exist"
cargo test --test cli_smoke -p cg  # 7/7 tests pass
cargo test                    # 19 tests total, 0 failures
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Virtual workspace does not allow [dev-dependencies]**
- **Found during:** Task 2, while adding `assert_cmd`/`predicates` to workspace root `Cargo.toml`
- **Issue:** `cargo` rejected `[dev-dependencies]` in a virtual workspace manifest (`error: this virtual manifest specifies a 'dev-dependencies' section, which is not allowed`)
- **Fix:** Placed smoke test at `crates/cli/tests/cli_smoke.rs` instead of `tests/cli_smoke.rs`. Dev-dependencies `assert_cmd` and `predicates` declared in `crates/cli/Cargo.toml` are already in scope for the cli integration test target. This is idiomatic Cargo for binary crate integration tests.
- **Files modified:** `crates/cli/tests/cli_smoke.rs` (created), `crates/cli/Cargo.toml` (dev-deps added as part of Task 1 plan)

**2. [Rule 3 - Blocking] Relative fixture paths fail from crates/cli test CWD**
- **Found during:** Task 2, first smoke test run
- **Issue:** Tests using `"crates/core/tests/fixtures"` as a relative path failed because assert_cmd runs the binary with the test process CWD (`crates/cli/`), not the workspace root
- **Fix:** Added `workspace_root()` helper using `env!("CARGO_MANIFEST_DIR")` to construct absolute paths; updated `file_path_errors`, `scan_fixture_directory`, `scan_detects_typescript`, and `verbose_flag_shows_files` tests to use absolute paths
- **Files modified:** `crates/cli/tests/cli_smoke.rs`

**3. [Rule 1 - Bug] Double-reference `.clone()` warning in print_summary**
- **Found during:** Task 1 build verification
- **Issue:** `sorted.sort_by_key(|(k, _)| k.clone())` on `Vec<(&String, &usize)>` clones the reference rather than the string value (compiler warning `suspicious_double_ref_op`)
- **Fix:** Changed to `k.as_str()` which borrows correctly
- **Files modified:** `crates/cli/src/main.rs`

## Known Stubs

None. The CLI is fully wired to `cgraph_core::scan_directory()`.

## Threat Surface Scan

No new network endpoints, auth paths, or file access patterns beyond what the plan's threat model covers. Error messages use `path.display()` (safe Display impl) as required by T-01-06. Path validation before scan is implemented as required by T-01-05.

## Self-Check: PASSED

All created files verified present. Both task commits verified in git log. All 15 acceptance criteria checked and passed. `cargo test` 19/19 passing.
