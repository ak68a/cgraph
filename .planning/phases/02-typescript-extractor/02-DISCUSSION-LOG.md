# Phase 2: TypeScript Extractor - Discussion Log

**Date:** 2026-05-02
**Areas discussed:** 4

## Area 1: Barrel Re-Export Resolution

**Question:** Should the extractor resolve re-export chains or emit edges for the indexer?
**Options:** Extractor emits edges only | Extractor resolves within barrel | Hybrid single-hop
**Selected:** Extractor emits edges only
**Notes:** Preserves D-18 (pure transformation). Indexer handles chain resolution in Phase 3.

**Question:** Which re-export patterns to support?
**Options:** Named | Star | Renamed | Default (multi-select)
**Selected:** Named only (initially), then confirmed Named + Star after follow-up
**Notes:** User initially selected named only. Clarified that star re-exports are very common in OversizeConnect barrels. User agreed to include both.

## Area 2: tsconfig Path Alias Handling

**Question:** Where should alias resolution happen?
**Options:** Indexer resolves | Extractor resolves | Two-pass in extractor crate
**Selected:** Indexer resolves
**Notes:** User asked for clarification on what this means. Explained that extractor emits raw import paths (@/components/Button as-is), indexer reads tsconfig.json once in Phase 3 and resolves globally. User confirmed after explanation.

## Area 3: Call Edge Detection Scope

**Question:** How aggressively should the extractor detect function calls?
**Options:** Direct named calls only | Named + member calls | All call expressions
**Selected:** Direct named calls only
**Notes:** User asked for detailed explanation of the differences. Provided concrete examples showing direct calls (foo(), useHook()) vs member calls (Alert.alert, console.log) vs dynamic calls. User understood the tradeoff and chose direct named calls. Explicitly requested that member call detection with exclusion filtering be captured as a deferred item for post-v1.

## Area 4: Test Strategy & Fixtures

**Question:** What should test fixtures look like?
**Options:** OversizeConnect-inspired | Minimal synthetic | Mix of both
**Selected:** OversizeConnect-inspired

**Question:** How should assertions work?
**Options:** Key patterns | Exact counts | Snapshot-based
**Selected:** Key patterns
**Notes:** Assert specific symbols/edges exist by name and kind, not exact counts. Resilient to fixture changes.

## Deferred Ideas

- Member call edges (obj.method()) with exclusion list filtering — post-v1 enhancement
- Renamed re-exports (export { foo as bar })
- Default re-exports (export { default as Foo })

---

*Discussion completed: 2026-05-02*
