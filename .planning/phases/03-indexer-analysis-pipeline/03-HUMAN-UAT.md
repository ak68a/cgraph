---
status: resolved
phase: 03-indexer-analysis-pipeline
source: [03-VERIFICATION.md]
started: 2026-05-02T21:40:00Z
updated: 2026-05-02T23:55:00Z
---

## Current Test

[all tests passed]

## Tests

### 1. Run CLI against a real TypeScript project with inter-file imports
expected: Edge count is non-zero; dead code report shows confirmed and suspicious entries; barrel re-exports resolve correctly
result: PASSED — Tested against 4 real projects: nighthawk (630 edges/277 symbols=227%), agentcommercekit (1917/667=287%), signum-api (2034/932=218%), OversizeConnect/mobile-app (880/1173=75%). OversizeConnect edge ratio improved from 9% (106 edges) to 75% (880 edges) after gap closure.

### 2. Verify --dead-code output on a real project with known dead exports
expected: Known dead exports appear as confirmed; barrel-re-exported symbols do NOT appear; entry point files excluded
result: PASSED — Spot-checked nighthawk dead code: _resetShellRunner, _setShellRunner, PMAdapter confirmed unreferenced via grep. No barrel-re-exported symbols falsely flagged. OversizeConnect cycle detection found real circular dependency (parsers.ts ↔ core.ts).

## Summary

total: 2
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
