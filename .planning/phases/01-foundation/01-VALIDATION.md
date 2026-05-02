---
phase: 1
slug: foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-02
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`#[test]`, `cargo test`) |
| **Config file** | None — built into Cargo |
| **Quick run command** | `cargo test -p cgraph-core` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p cgraph-core`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | INFR-01 | — | N/A | CLI smoke | `cargo test -p cg` | No — W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | INFR-01 | — | N/A | CLI smoke | `cargo test -p cg -- version_flag` | No — W0 | ⬜ pending |
| 01-01-03 | 01 | 1 | INFR-01 | — | N/A | CLI smoke | `cargo test -p cg -- help_flag` | No — W0 | ⬜ pending |
| 01-02-01 | 02 | 1 | PARS-11 | — | N/A | unit | `cargo test -p cgraph-core -- detect_ts` | No — W0 | ⬜ pending |
| 01-02-02 | 02 | 1 | PARS-11 | — | N/A | unit | `cargo test -p cgraph-core -- detect_tsx` | No — W0 | ⬜ pending |
| 01-02-03 | 02 | 1 | PARS-11 | — | N/A | unit | `cargo test -p cgraph-core -- detect_swift` | No — W0 | ⬜ pending |
| 01-02-04 | 02 | 1 | PARS-11 | — | N/A | unit | `cargo test -p cgraph-core -- detect_go` | No — W0 | ⬜ pending |
| 01-02-05 | 02 | 1 | PARS-11 | — | N/A | unit | `cargo test -p cgraph-core -- detect_py` | No — W0 | ⬜ pending |
| 01-02-06 | 02 | 1 | PARS-11 | — | N/A | integration | `cargo test -p cgraph-core -- mixed_fixture` | No — W0 | ⬜ pending |
| 01-03-01 | 03 | 1 | D-22 | — | N/A | integration | `cargo test -p cgraph-core -- typescript_grammar` | No — W0 | ⬜ pending |
| 01-03-02 | 03 | 1 | D-22 | — | N/A | integration | `cargo test -p cgraph-core -- tsx_grammar` | No — W0 | ⬜ pending |
| 01-03-03 | 03 | 1 | D-22 | — | N/A | integration | `cargo test -p cgraph-core -- swift_grammar` | No — W0 | ⬜ pending |
| 01-03-04 | 03 | 1 | D-22 | — | N/A | integration | `cargo test -p cgraph-core -- go_grammar` | No — W0 | ⬜ pending |
| 01-03-05 | 03 | 1 | D-22 | — | N/A | integration | `cargo test -p cgraph-core -- python_grammar` | No — W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/core/tests/grammar_test.rs` — integration tests for all 4 grammar crates
- [ ] `crates/core/tests/fixtures/sample.ts` — minimal valid TypeScript fixture
- [ ] `crates/core/tests/fixtures/sample.tsx` — minimal valid TSX fixture
- [ ] `crates/core/tests/fixtures/sample.swift` — minimal valid Swift fixture
- [ ] `crates/core/tests/fixtures/sample.go` — minimal valid Go fixture
- [ ] `crates/core/tests/fixtures/sample.py` — minimal valid Python fixture
- [ ] `crates/core/src/lib.rs`, `model.rs`, `detect.rs`, `extractor.rs` — library structure
- [ ] `crates/cli/src/main.rs` — CLI entry point
- [ ] `tests/cli_smoke.rs` — workspace-level subprocess CLI test

*(Entire project is greenfield — all test infrastructure is Wave 0)*

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
