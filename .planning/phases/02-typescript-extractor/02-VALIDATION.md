---
phase: 2
slug: typescript-extractor
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-02
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | `crates/ts-extractor/Cargo.toml` |
| **Quick run command** | `cargo test -p ts-extractor` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p ts-extractor`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | PARS-01 | — | N/A | unit | `cargo test -p ts-extractor` | ❌ W0 | ⬜ pending |
| 02-01-02 | 01 | 1 | PARS-05 | — | N/A | unit | `cargo test -p ts-extractor` | ❌ W0 | ⬜ pending |
| 02-01-03 | 01 | 1 | PARS-06 | — | N/A | unit | `cargo test -p ts-extractor` | ❌ W0 | ⬜ pending |
| 02-01-04 | 01 | 1 | PARS-07 | — | N/A | unit | `cargo test -p ts-extractor` | ❌ W0 | ⬜ pending |
| 02-01-05 | 01 | 1 | PARS-08 | — | N/A | unit | `cargo test -p ts-extractor` | ❌ W0 | ⬜ pending |
| 02-01-06 | 01 | 1 | PARS-09 | — | N/A | unit | `cargo test -p ts-extractor` | ❌ W0 | ⬜ pending |
| 02-01-07 | 01 | 1 | PARS-10 | — | N/A | unit | `cargo test -p ts-extractor` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/ts-extractor/tests/` — test module stubs for extraction
- [ ] `crates/ts-extractor/tests/fixtures/` — TypeScript fixture files (barrel re-exports, hooks, components, type refs)

*Existing infrastructure covers framework needs (cargo test built-in).*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
