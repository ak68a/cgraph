---
phase: 5
slug: graph-interaction
status: active
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-03
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) + browser manual (JS/D3) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p cgraph-server` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p cgraph-server`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-T1 | 05-01 | 1 | VIZN-03, INTR-04, INTR-03 | T-05-01, T-05-02 | N/A | unit (Rust) | `cargo test -p cgraph-server -- --nocapture` | Yes (created by task) | ⬜ pending |
| 01-T2 | 05-01 | 1 | VIZN-03 | — | N/A | build | `cargo build && cargo test --workspace` | Yes | ⬜ pending |
| 02-T1 | 05-02 | 1 | VIZN-04, INTR-01, INTR-05-08 | T-05-03 | N/A | grep | `grep -c 'id="header-search"' client/index.html` | Yes | ⬜ pending |
| 02-T2 | 05-02 | 1 | INTR-04, INTR-03, INTR-05-07 | T-05-03 | N/A | grep | `grep -c 'id="toggle-dead-code"' client/index.html` | Yes | ⬜ pending |
| 03-T1 | 05-03 | 2 | VIZN-03 | T-05-04 | N/A | grep | `grep -c 'expandFileNode' client/graph.js` | Yes | ⬜ pending |
| 03-T2 | 05-03 | 2 | INTR-02, VIZN-04 | T-05-05 | N/A | grep | `grep -c 'focusActive' client/graph.js && grep -c 'fitToScreen' client/graph.js` | Yes | ⬜ pending |
| 04-T1 | 05-04 | 3 | INTR-01 | T-05-06 | XSS: textContent only | grep | `grep -c 'openPalette' client/graph.js` | Yes | ⬜ pending |
| 04-T2 | 05-04 | 3 | INTR-08 | T-05-07 | XSS: textContent only | grep | `grep -c 'historyStack' client/graph.js` | Yes | ⬜ pending |
| 05-T1 | 05-05 | 4 | INTR-04, INTR-03 | T-05-08, T-05-09 | N/A | grep | `grep -c 'showDeadCodeOverlay' client/graph.js` | Yes | ⬜ pending |
| 05-T2 | 05-05 | 4 | INTR-05, INTR-06, INTR-07 | T-05-10 | filter-dir client-only | grep | `grep -c 'filterState' client/graph.js` | Yes | ⬜ pending |
| 06-T1 | 05-06 | 5 | ALL | — | N/A | manual | Visual checkpoint | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠ flaky*

---

## Wave 0 Requirements

Wave 0 test scaffolds are created inline by Plan 05-01 Task 1 (TDD tests for `enriched_projection`). No separate Wave 0 plan needed. Tests created:

- `test_enriched_response_includes_symbols` in `crates/server/src/graph_api.rs`
- `test_enriched_response_dead_code_flags` in `crates/server/src/graph_api.rs`
- `test_enriched_response_typed_edges` in `crates/server/src/graph_api.rs`

All browser-side verification is grep-based (element existence) or manual visual (Plan 05-06 checkpoint). This is justified because the JS client has no build step, no test runner, and no framework (D-61).

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Node expand/collapse visual | VIZN-03 | D3 visual behavior in browser | Click file node, verify symbols appear as child nodes |
| Zoom/pan/fit-to-screen | VIZN-04 | Mouse/trackpad interaction | Scroll to zoom, drag to pan, click fit button |
| Search highlights in real-time | INTR-01 | Visual highlight verification | Type in search box, verify purple glow on matches |
| Click-to-focus dims unrelated | INTR-02 | Visual opacity verification | Click node, verify neighbors stay full opacity |
| Blast radius highlight | INTR-03 | Visual overlay verification | Enable blast radius, click node, verify purple highlights |
| Dead code overlay | INTR-04 | Visual confidence coloring | Toggle dead code overlay, verify red/orange indicators |
| Filter controls | INTR-05/06/07 | UI panel interaction | Toggle filters, verify nodes/edges hide/show |
| Back/forward history | INTR-08 | Navigation state verification | Click through nodes, use back/forward buttons |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** ready
