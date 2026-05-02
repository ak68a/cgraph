# Phase 1: Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-02
**Phase:** 1-Foundation
**Areas discussed:** Symbol identity scheme, Node/edge metadata scope, Workspace layout, Tree-sitter grammar strategy, Language detection behavior, Error handling philosophy, Extractor trait design, CLI UX for Phase 1, Test strategy

---

## Symbol Identity Scheme

| Option | Description | Selected |
|--------|-------------|----------|
| Path-qualified | file_path::symbol_name — human-readable, greppable, stable within a scan | ✓ |
| Content-addressed hash | SHA of path+name+kind — compact but opaque, requires lookup step | |
| Scoped with nesting | file_path::parent::symbol — handles nested symbols but IDs get long | |

**User's choice:** Path-qualified (after requesting education on the options and rationale)
**Notes:** User initially didn't understand the options. Claude presented the case for path-qualified with tradeoff analysis. User confirmed the recommendation held after understanding.

---

## Node/Edge Metadata Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Minimum only | id, name, kind, file_path, language | |
| Minimum + likely-useful | Add line numbers, visibility/is_exported, source_location on edges | ✓ |
| Full metadata | Also add docstrings, signatures, byte_range | |

**User's choice:** Minimum + likely-useful tier
**Notes:** User asked whether metadata would let them see all invocations of a function and navigate to source (like Xcode). Claude clarified this is edge traversal (inherent in graph structure), not node metadata — confirmed the distinction.

---

## Workspace Layout

| Option | Description | Selected |
|--------|-------------|----------|
| Single crate, modules | One Cargo.toml, src/ subdirectories | |
| Cargo workspace | Separate crates for core, cli, extractors, server | ✓ |
| Hybrid | Single crate now, split later | |

**User's choice:** Cargo workspace from day one
**Notes:** No additional discussion needed.

---

## Tree-sitter Grammar Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Published crates | tree-sitter-typescript etc. from crates.io | ✓ |
| Build from source | Vendor grammar C files, compile via build.rs | |

**User's choice:** Published crates
**Notes:** Claude presented recommendation with rationale. User agreed immediately.

---

## Language Detection Behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Detect all, report both | Scan all extensions, parse supported only, show both in summary | ✓ |
| Supported only | Silently ignore unknown file types | |

**User's choice:** Detect all, parse supported, report both
**Notes:** User agreed and additionally mentioned wanting to support Rust, Java, Ruby, Kotlin — captured as deferred ideas (scope creep beyond v1 roadmap).

---

## Error Handling Philosophy

| Option | Description | Selected |
|--------|-------------|----------|
| Warn and continue | Partial parses valid, never fail scan for one file | ✓ |
| Strict | Fail on parse errors | |
| Silent skip | Skip broken files with no output | |

**User's choice:** Warn and continue
**Notes:** No additional discussion needed.

---

## Extractor Trait Design

| Option | Description | Selected |
|--------|-------------|----------|
| Text-in, fragments-out | Extractor receives &str, returns owned nodes/edges/errors | ✓ |

**User's choice:** Agreed with recommended design
**Notes:** User asked what "adjusting the boundary" meant. Claude clarified: the question was whether extractors should handle more (e.g., import resolution) or less. User confirmed the proposed boundary was right.

---

## CLI UX for Phase 1

| Option | Description | Selected |
|--------|-------------|----------|
| Detection + summary | cg <path> runs detection, prints file counts by language | ✓ |
| Help only | Just print help/version, defer all scan behavior | |

**User's choice:** Detection + summary
**Notes:** No additional discussion needed.

---

## Test Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Three layers | Unit tests (model), integration (fixtures + tree-sitter), CLI smoke test | ✓ |

**User's choice:** Three layers, no mocks, no benchmarks
**Notes:** No additional discussion needed.

---

## Claude's Discretion

None — user made explicit choices on all areas.

## Deferred Ideas

- Rust language extractor — future phase beyond v1 roadmap
- Java language extractor — future phase beyond v1 roadmap
- Ruby language extractor — future phase beyond v1 roadmap
- Kotlin language extractor — future phase beyond v1 roadmap
