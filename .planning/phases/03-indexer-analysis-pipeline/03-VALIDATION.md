---
phase: 03
slug: indexer-analysis-pipeline
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-02
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml workspace |
| **Quick run command** | `cargo test -p cgraph-indexer` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p cgraph-indexer`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | PARS-09 | — | N/A | integration | `cargo test barrel_resolution` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | PARS-10 | — | N/A | integration | `cargo test path_alias` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ANLS-01 | — | N/A | unit | `cargo test dead_code` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ANLS-02 | — | N/A | unit | `cargo test dead_code_confidence` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ANLS-03 | — | N/A | unit | `cargo test circular_deps` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ANLS-04 | — | N/A | unit | `cargo test blast_radius` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ANLS-05 | — | N/A | unit | `cargo test transitive_deps` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | INFR-03 | — | N/A | integration | `cargo test scan_summary` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/cgraph-indexer/tests/` — test stubs for all 8 requirements
- [ ] Integration test fixtures — multi-file TS projects with barrels, aliases, circular deps

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
