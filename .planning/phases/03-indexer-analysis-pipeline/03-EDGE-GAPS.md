---
phase: 03-indexer-analysis-pipeline
type: known-gap
status: open
created: 2026-05-02
severity: medium
affects: [ANLS-01, ANLS-02, ANLS-03, ANLS-04, ANLS-05]
---

# Edge Resolution Gaps — Phase 3

## Problem

Running against OversizeConnect (real-world TypeScript/React Native project):

```
cgraph scan: 415 files, 1165 symbols, 106 edges (1719ms)
dead code: 409 confirmed, 0 suspicious
circular dependencies: 0
```

Only 106 edges resolved out of an estimated 1000+. The 409 "confirmed dead" count is mostly false positives — symbols appear dead because their incoming import edges were dropped during resolution.

## Root Causes

### 1. Call edges use `unresolved::calledName` targets
Call edges from the ts-extractor use `unresolved::functionName` as target_id. These never match graph nodes because the indexer doesn't perform intra-project symbol name resolution (matching `unresolved::format` to `src/utils.ts::format`).

### 2. TypeRef edges use `unresolved::typeName` targets
Same pattern — type reference edges like `unresolved::UserProfile` are never linked to their defining nodes.

### 3. tsconfig alias resolution incomplete
Some alias patterns may not match (multi-path aliases, baseUrl-only resolution without explicit paths, nested tsconfig extends).

### 4. Extension resolution misses
Import resolution tries `.ts`, `.tsx`, `/index.ts`, `/index.tsx` but may miss `.js`, `.jsx`, or projects using non-standard extensions.

## Impact

- Dead code detection: ~75% false positive rate on real projects
- Blast radius: returns incomplete results (misses call/type edges)
- Cycle detection: may miss cycles mediated by call edges (only import edges form file-level cycles currently)

## Fix Strategy

A gap closure phase (3.1) or improvement pass should:

1. **Resolve `unresolved::` Call edges** — after all symbols are indexed, scan for exported symbols matching the called name and create edges. This is an O(symbols × unresolved_calls) pass, optimizable with a HashMap lookup.
2. **Resolve `unresolved::` TypeRef edges** — same approach as Call edges.
3. **Improve tsconfig handling** — support `baseUrl` without `paths`, `extends` chains, and multiple path targets.
4. **Expand extension resolution** — add `.js`, `.jsx`, `.mjs`, `.cjs` to the resolution chain.

## Metric

Track: edges / symbols ratio. Current: 106 / 1165 = 9%. Target: >50% on OversizeConnect.
