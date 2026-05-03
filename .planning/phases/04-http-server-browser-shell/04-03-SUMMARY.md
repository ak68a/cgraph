---
phase: 04-http-server-browser-shell
plan: 03
subsystem: cli
tags: [tokio, webbrowser, async-main, server-startup, browser-open, signal-handling]
dependency_graph:
  requires: [cgraph-server (plan 04-01), cgraph-indexer, cgraph-core]
  provides: [cg binary with HTTP server startup, auto-open browser, --no-open flag]
  affects: [end-user CLI behavior: cg <path> now starts server and opens browser]
tech_stack:
  added: [tokio 1 (full features), webbrowser 1, cgraph-server dependency in CLI crate]
  patterns: [async tokio main, cgraph_server::serve() wrapper, graceful Ctrl-C shutdown]
key_files:
  modified:
    - crates/cli/Cargo.toml
    - crates/cli/src/main.rs
    - crates/cli/tests/cli_smoke.rs
decisions:
  - "CLI calls cgraph_server::serve() wrapper -- no direct axum dependency in CLI crate"
  - "Smoke tests updated with --no-open flag + 10s timeout since server now blocks on Ctrl-C"
  - "Added scan_prints_server_url and help_shows_no_open_flag tests for new functionality"
metrics:
  duration: "~12 minutes"
  completed: "2026-05-03"
  tasks_completed: 2
  files_created: 0
  files_modified: 3
---

# Phase 4 Plan 03: CLI Server Wiring Summary

Async main with tokio, server startup via cgraph_server::serve(), auto-open browser via webbrowser crate, and Ctrl-C graceful shutdown after INFR-02.

## What Was Built

The CLI crate (`crates/cli`) was updated to complete the Phase 4 integration:

1. **Dependency additions** (Task 1): Added `cgraph-server`, `tokio` (full), and `webbrowser` to `crates/cli/Cargo.toml`. No direct `axum` dependency — the server crate encapsulates that.

2. **Async main with server startup** (Task 2): Converted `fn main() -> Result<()>` to `#[tokio::main] async fn main() -> Result<()>`. The existing synchronous analysis flow (indexer, dead code, cycles reporting) works unchanged inside the async context. After analysis, the CLI:
   - Derives project name from the scanned path's last directory component
   - Builds `ScanStats` from the completed indexer run
   - Calls `file_level_projection()` to pre-compute the file-level graph
   - Calls `find_available_port(3000)` to find a free port (D-60)
   - Spawns `cgraph_server::serve(listener, state)` in a background task
   - Prints server URL and optionally opens browser via `webbrowser::open()` (D-62)
   - Blocks on `tokio::signal::ctrl_c()` for graceful shutdown

3. **`--no-open` flag** (D-62): Added to `Cli` struct. When set, suppresses browser open and prints the URL with instructions to open manually. Intended for headless/CI use.

## Task Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Add server and browser-open dependencies to CLI crate | 3c0619f |
| 2 | Convert CLI to async main with server startup and browser open | f7ff84d |

## Verification Results

- `cargo build -p cg` — compiles without errors
- `cargo test -p cg` — all 10 tests pass (including 3 new tests)
- `cargo test -p cgraph-server` — all 4 server unit tests pass
- `cargo run -p cg -- --help` — shows `--no-open` flag in help output

### Test Results

```
running 10 tests
test file_path_errors ... ok
test help_shows_no_open_flag ... ok
test nonexistent_path_errors ... ok
test help_flag ... ok
test version_flag ... ok
test cycles_flag ... ok
test scan_prints_analysis_summary ... ok
test dead_code_flag ... ok
test scan_fixture_directory ... ok
test scan_prints_server_url ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.43s
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CLI smoke tests would hang indefinitely without --no-open and timeout**
- **Found during:** Task 2 verification
- **Issue:** Existing smoke tests run `cg <fixtures>` without `--no-open`. Since the CLI now blocks on `tokio::signal::ctrl_c()`, all tests that start a scan would hang forever. The original smoke tests called `.assert().success()` which requires the process to exit.
- **Fix:** Updated `cli_smoke.rs` to:
  - Add `--no-open` arg to all tests that run a scan (suppresses browser open)
  - Add `.timeout(Duration::from_secs(10))` so the test process is killed after the output is written
  - Remove `.success()` assertions from blocking tests (process is killed, not Ctrl-C'd)
  - Added two new tests: `help_shows_no_open_flag` and `scan_prints_server_url`
- **Files modified:** crates/cli/tests/cli_smoke.rs
- **Commit:** f7ff84d

## Security

Threat T-04-02 (server bind address to 127.0.0.1) already mitigated in Plan 01 via `find_available_port()` always binding to `127.0.0.1`. The CLI consumes this function and inherits the mitigation without additional changes.

Threat T-04-05 (webbrowser::open elevation) accepted: opens user's default browser with a localhost URL — standard developer tool behavior.

## Known Stubs

None. The CLI correctly wires all components: indexer output → file_level_projection → AppState → cgraph_server::serve().

## Self-Check

### Modified files exist:
- crates/cli/Cargo.toml: FOUND
- crates/cli/src/main.rs: FOUND
- crates/cli/tests/cli_smoke.rs: FOUND

### Commits exist:
- 3c0619f: Task 1 (chore: add server and browser-open dependencies to CLI crate)
- f7ff84d: Task 2 (feat: convert CLI to async main with server startup and browser open)

## Self-Check: PASSED
