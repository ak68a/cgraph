---
phase: 02
slug: typescript-extractor
status: verified
threats_open: 0
asvs_level: 1
created: 2026-05-02
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Source text input | Arbitrary file content passed to tree-sitter parser | Untrusted TS/TSX source code |
| File path input | Path string used for symbol ID construction | Local filesystem paths |
| Import path strings | Arbitrary string values from import statements | Raw path strings in edge target_id |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-02-01 | DoS | tree-sitter parser | accept | tree-sitter has bounded recursion; indexer enforces file size limits | closed |
| T-02-02 | Tampering | file_path parameter | accept | Path used only for string construction, no file I/O | closed |
| T-02-03 | DoS | Query compilation panic | mitigate | All 5 queries use `.expect()` at construction (lib.rs:31-39) — fails fast at startup, not runtime | closed |
| T-02-04 | DoS | extract_symbols pathological nesting | accept | tree-sitter query engine bounded; each match O(1) | closed |
| T-02-05 | Info Disclosure | symbol names in IDs | accept | Symbol names are the tool's purpose; file paths are local only | closed |
| T-02-06 | DoS | extract_edges thousands of imports | accept | Linear time: one edge per import/call/ref, no amplification | closed |
| T-02-07 | Tampering | Malicious import paths in target_id | accept | Import paths treated as opaque strings; no file I/O in extractor (D-18). Indexer validates in Phase 3 | closed |
| T-02-08 | DoS | Partial parse error recovery | mitigate | Errors recorded as PartialParse (lib.rs:87,100), extraction continues; bounded by tree-sitter | closed |
| T-02-04-01 | Info Disclosure | .planning/ docs | accept | Planning docs in-repo, no secrets, no PII | closed |
| T-02-05-01 | Tampering | edges.rs namespace detection | mitigate | namespace_export child validated before identifier extraction (edges.rs:268-273); falls through to no-op if missing | closed |
| T-02-05-02 | DoS | symbols.rs HashSet dedup | accept | O(n) allocation bounded by file size; tree-sitter already handles large files | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01 | T-02-01 | tree-sitter's C grammar has bounded recursion; caller enforces file size | gsd-security | 2026-05-02 |
| AR-02 | T-02-02 | Paths are string data only, no traversal or I/O | gsd-security | 2026-05-02 |
| AR-03 | T-02-04 | Bounded query engine, O(1) per match | gsd-security | 2026-05-02 |
| AR-04 | T-02-05 | Tool's purpose is to expose symbol names | gsd-security | 2026-05-02 |
| AR-05 | T-02-06 | Linear complexity, no amplification vector | gsd-security | 2026-05-02 |
| AR-06 | T-02-07 | Import paths are opaque strings; validation deferred to Phase 3 indexer | gsd-security | 2026-05-02 |
| AR-07 | T-02-04-01 | Planning docs contain no secrets or PII | gsd-security | 2026-05-02 |
| AR-08 | T-02-05-02 | O(n) bounded by file size | gsd-security | 2026-05-02 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-05-02 | 11 | 11 | 0 | gsd-security (orchestrator verify) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-05-02
