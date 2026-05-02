# Phase 3: Indexer & Analysis Pipeline - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-02
**Phase:** 03-indexer-analysis-pipeline
**Areas discussed:** Dead code confidence model, CLI analysis output, Graph storage design, Crate organization

---

## Dead code confidence model

### Q1: Entry point identification

| Option | Description | Selected |
|--------|-------------|----------|
| Convention-based | Auto-detect entry points by convention: main/index files, test files, framework patterns. Zero config. | ✓ |
| Explicit config file | User provides .cgraph.toml listing entry point globs. Precise but requires setup. | |
| Both (convention + override) | Convention-based by default, with optional .cgraph.toml to add/remove. | |

**User's choice:** Convention-based
**Notes:** None — straightforward choice for zero-config first-run experience.

### Q2: Confidence scoring model

| Option | Description | Selected |
|--------|-------------|----------|
| Edge count + visibility | Confirmed dead vs suspicious. Suspicious demoted by: string literal matches, namespace imports, type-only generic usage. | ✓ |
| Simple binary + entry point | Just dead or alive. No heuristic scanning for dynamic usage. | |
| Three-tier with usage context | Confirmed, suspicious, AND "likely used". More granular heuristics. | |

**User's choice:** Edge count + visibility (two-tier)
**Notes:** None.

### Q3: Cycle detection level

| Option | Description | Selected |
|--------|-------------|----------|
| File level only | Detect cycles between files via import edges. Symbol-level cycles are valid recursion. | ✓ |
| Both levels | File-level AND symbol-level call cycles. | |
| Module level (directory) | Cycles between directories. Coarser grain. | |

**User's choice:** File level only
**Notes:** None.

---

## CLI analysis output

### Q4: Analysis result access

| Option | Description | Selected |
|--------|-------------|----------|
| Default summary + flags | `cg <path>` prints stats + analysis summary. `--dead-code` and `--cycles` for detail. | ✓ |
| Subcommands for everything | `cg scan`, `cg dead-code`, `cg cycles`, `cg blast-radius <symbol>`. | |
| Scan stats only | Phase 3 only prints scan stats. All analysis output deferred to later phases. | |

**User's choice:** Default summary + flags
**Notes:** None.

### Q5: Detail output format

| Option | Description | Selected |
|--------|-------------|----------|
| Human-readable text | Grouped by file, with symbol kind and line ranges. Annotated reasons for suspicious. | ✓ |
| One-per-line grep-friendly | Colon-separated fields. Easy to pipe but less readable. | |

**User's choice:** Human-readable text
**Notes:** None.

---

## Graph storage design

### Q6: In-memory graph structure

| Option | Description | Selected |
|--------|-------------|----------|
| petgraph crate | DiGraph with built-in Tarjan's SCC, DFS, topological sort. 11M downloads. | ✓ |
| Custom HashMap adjacency | Hand-roll adjacency lists. Full control, no dependency. ~200 lines of algorithms. | |
| Let Claude decide | Claude picks during planning. | |

**User's choice:** petgraph
**Notes:** None.

### Q7: Graph view strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Single graph + file grouping | One DiGraph with all symbols. File-level views derived on demand. Single source of truth. | ✓ |
| Two parallel graphs | Separate file-level and symbol-level DiGraphs kept in sync. | |

**User's choice:** Single graph + file grouping
**Notes:** None.

---

## Crate organization

### Q8: Workspace structure

| Option | Description | Selected |
|--------|-------------|----------|
| New crates/indexer crate | Dedicated crate: graph.rs, resolve.rs, analysis.rs, crawl.rs. Depends on core + petgraph. | ✓ |
| Extend core crate | Add graph/resolution/analysis to core. Fewer crates but core becomes monolith. | |
| Split: indexer + analysis | Two new crates. Maximum separation but over-engineered for v1. | |

**User's choice:** New crates/indexer crate
**Notes:** None.

### Q9: Extractor registration

| Option | Description | Selected |
|--------|-------------|----------|
| Dynamic registry | Indexer accepts Vec<Box<dyn Extractor>> from caller. CLI builds the registry. | ✓ |
| Hardcoded in indexer | Indexer directly instantiates extractors. Simpler but each new extractor modifies indexer. | |

**User's choice:** Dynamic registry
**Notes:** None.

---

## Claude's Discretion

None — all decisions made by user.

## Deferred Ideas

- Config file for entry points (`.cgraph.toml`) — future enhancement for non-standard layouts
- Symbol-level cycle detection — intentional patterns in most cases
- Directory/module-level cycle detection — for large monorepos beyond v1 scale
- Blast radius CLI query — Phase 11
- JSON output format — Phase 11
- Graph caching / incremental rebuild — optimization if needed
