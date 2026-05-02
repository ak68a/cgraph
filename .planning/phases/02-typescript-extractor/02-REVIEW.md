---
phase: 02-typescript-extractor
reviewed: 2026-05-02T16:45:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - Cargo.toml
  - crates/ts-extractor/Cargo.toml
  - crates/ts-extractor/src/classify.rs
  - crates/ts-extractor/src/edges.rs
  - crates/ts-extractor/src/lib.rs
  - crates/ts-extractor/src/queries.rs
  - crates/ts-extractor/src/symbols.rs
  - crates/ts-extractor/tests/extraction_test.rs
  - crates/ts-extractor/tests/fixtures/barrel.ts
  - crates/ts-extractor/tests/fixtures/components.tsx
  - crates/ts-extractor/tests/fixtures/enums.ts
  - crates/ts-extractor/tests/fixtures/hooks.ts
  - crates/ts-extractor/tests/fixtures/index.ts
  - crates/ts-extractor/tests/fixtures/schemas.ts
  - crates/ts-extractor/tests/fixtures/services.ts
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-05-02T16:45:00Z
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

The TypeScript extractor is well-structured: the two-pass architecture (symbols then edges), the tree-sitter query design, hook classification, overload deduplication, and the star-vs-namespace re-export guard are all sound. All 30 tests pass with zero compiler warnings. Previous review findings (CR-01 namespace misclassification, WR-01 overload dedup, WR-02/WR-03 dead code) have been addressed.

However, a new critical bug exists in how exported symbol line ranges are computed -- `line_start` and `line_end` both point to the identifier token rather than the full declaration body, making the values useless for any consumer that needs the symbol's span (e.g., visualization, jump-to-definition). Three warnings cover a still-unfixed partial-parse error reporting bug, missing handling of aliased re-exports, and a semantically invalid test fixture.

---

## Critical Issues

### CR-01: Exported Symbol `line_start`/`line_end` Are Identical (Point to Identifier, Not Declaration Body)

**File:** `crates/ts-extractor/src/symbols.rs:52-69`

**Issue:** In `extract_symbols`, the `@symbol_name` capture is the `(identifier)` or `(type_identifier)` AST node -- a single-token node spanning one line. The code uses this node for both `line_start` and `line_end`:

```rust
let node = cap.node;  // This is the @symbol_name capture = identifier token
// ...
nodes.push(SymbolNode {
    // ...
    line_start: node.start_position().row as u32 + 1,  // identifier line
    line_end: node.end_position().row as u32 + 1,       // same line
    // ...
});
```

For a multi-line function like `fetchUser` in `services.ts` (lines 24-27), both `line_start` and `line_end` will be `24` -- the line where the identifier `fetchUser` appears. The actual function body ending at line 27 is not reflected. This affects every exported symbol (functions, classes, interfaces, enums, type aliases) extracted via the query path.

By contrast, `extract_non_exported_functions` (line 105-119) correctly uses the `function_declaration` node for line ranges, creating an inconsistency between exported and non-exported symbols from the same file.

The `SymbolNode.line_end` field (defined in `core/src/model.rs:39`) is intended to represent the end of the declaration. Downstream consumers (graph visualization, code navigation) that rely on this span will get incorrect results for every exported symbol.

**Fix:** Use the `@export_stmt` capture (which wraps the full `export_statement` node) or walk up from the `@symbol_name` capture to the declaration node to get the correct span:

```rust
// Option A: Look up the @export_stmt capture for the full span
let export_stmt_idx = symbol_query.capture_index_for_name("export_stmt");

// Inside the captures loop, after finding @symbol_name:
let (span_start, span_end) = if let Some(eidx) = export_stmt_idx {
    // Find the matching @export_stmt capture in this match
    m.captures.iter()
        .find(|c| c.index == eidx)
        .map(|c| (
            c.node.start_position().row as u32 + 1,
            c.node.end_position().row as u32 + 1,
        ))
        .unwrap_or((
            cap.node.start_position().row as u32 + 1,
            cap.node.end_position().row as u32 + 1,
        ))
} else {
    (cap.node.start_position().row as u32 + 1,
     cap.node.end_position().row as u32 + 1)
};

nodes.push(SymbolNode {
    // ...
    line_start: span_start,
    line_end: span_end,
    // ...
});
```

Option B (simpler): walk up from the identifier to the parent `function_declaration` / `class_declaration` / etc. node and use its span.

---

## Warnings

### WR-01: `PartialParse` Error Always Reports Line 0

**File:** `crates/ts-extractor/src/lib.rs:98-103`

**Issue:** When `root.has_error()` is true, the error is recorded with `line: root.start_position().row as u32`, which is always 0 because the root node starts at the beginning of the file. This makes the `line` field in `ParseError::PartialParse` useless for diagnostic purposes -- the caller cannot determine where in the file the syntax error occurred.

```rust
if root.has_error() {
    errors.push(ParseError::PartialParse {
        path: path.display().to_string(),
        line: root.start_position().row as u32,  // always 0
    });
}
```

This was identified in the prior review (WR-04) but was not fixed.

**Fix:** Walk the tree to find the first ERROR or MISSING node and report its line:

```rust
if root.has_error() {
    let error_line = find_first_error_line(root);
    errors.push(ParseError::PartialParse {
        path: path.display().to_string(),
        line: error_line,
    });
}

fn find_first_error_line(node: Node) -> u32 {
    if node.is_error() || node.is_missing() {
        return node.start_position().row as u32 + 1;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let line = find_first_error_line(cursor.node());
            if line > 0 { return line; }
            if !cursor.goto_next_sibling() { break; }
        }
    }
    0
}
```

---

### WR-02: Aliased Re-Exports Emit Wrong `source_id`

**File:** `crates/ts-extractor/src/edges.rs:199-222`, `crates/ts-extractor/src/queries.rs:103-108`

**Issue:** The re-export query Pattern 0 captures `name: (identifier) @specifier_name` from the `export_specifier` node. For aliased re-exports like `export { foo as bar } from './module'`, tree-sitter's `export_specifier` has `name` = "foo" (the local/source name) and `alias` = "bar" (the publicly exported name). The code uses the `name` field for both `source_id` and `target_id`:

```rust
source_id: format!("{}::{}", file_path, name),  // "file::foo" -- should be "file::bar"
target_id: format!("{}::{}", path, name),        // "./module::foo" -- correct
```

The `source_id` should use the alias (the public name), since consuming files will import `bar`, not `foo`. A downstream resolver looking for `file::bar` will find no matching edge. The `target_id` correctly uses the original name to reference the source module.

While non-aliased re-exports (the common case) work correctly, any aliased re-export will produce an unreachable `source_id`.

**Fix:** Add an `@alias_name` capture to the re-export query and prefer it for `source_id`:

```
; In REEXPORT_QUERY_SRC Pattern 0:
(export_statement
  (export_clause
    (export_specifier
      name: (identifier) @specifier_name
      alias: (identifier)? @alias_name))
  source: (string
    (string_fragment) @source_path))
```

In `extract_reexports`:
```rust
let public_name = alias.unwrap_or(name);
edges.push(SymbolEdge {
    source_id: format!("{}::{}", file_path, public_name),
    target_id: format!("{}::{}", path, name),
    kind: EdgeKind::ReExport,
    source_location: line,
});
```

---

### WR-03: Test Fixture `barrel.ts` References Non-Existent Symbol `UserSchema`

**File:** `crates/ts-extractor/tests/fixtures/barrel.ts:2`

**Issue:** Line 2 of `barrel.ts` is `export { UserSchema, type UserType } from './schemas'`, but `schemas.ts` does not define or export any symbol named `UserSchema`. The extractor still produces a re-export edge for it (it operates on raw syntax, not semantic validity), but the fixture is semantically invalid TypeScript. If tests are later extended to validate cross-file consistency or if the fixture is used for integration testing, this discrepancy will cause false failures.

Additionally, no existing test verifies the re-export edges from the `./schemas` line in `barrel.ts`, so this fixture line is effectively untested dead data.

**Fix:** Either add an exported symbol `UserSchema` to `schemas.ts`:

```typescript
export const UserSchema = { /* ... */ };
```

Or correct `barrel.ts` to only reference symbols that `schemas.ts` actually exports:

```typescript
export { UserType, UserRole } from './schemas';
```

---

## Info

### IN-01: Test Fixture `index.ts` Is Never Used

**File:** `crates/ts-extractor/tests/fixtures/index.ts`

**Issue:** The fixture file `index.ts` contains 5 re-export statements but is not referenced by any test in `extraction_test.rs`. All tests that use an `index.ts` path construct inline source strings via `Path::new("index.ts")` with ad-hoc content. The fixture file is dead test data.

**Fix:** Either write tests that read this fixture to validate multi-statement barrel file extraction, or remove the file.

---

### IN-02: `export const` with Non-Arrow Values Silently Ignored

**File:** `crates/ts-extractor/src/queries.rs:9-14`

**Issue:** Pattern 1 of `SYMBOL_QUERY_SRC` only matches `export const x = () => {}` (arrow functions). Other common patterns are silently dropped:
- `export const Component = React.memo(...)` -- HOC-wrapped components
- `export const TIMEOUT = 5000` -- constant values
- `export const schema = z.object(...)` -- Zod schemas and similar builder patterns

This is likely an intentional scope decision (the tool focuses on function/class/type symbols), but it is undocumented and could surprise users who expect all exported `const` bindings to appear in the graph.

**Fix:** Document this as a known limitation in the crate-level docs or a comment in `queries.rs`, so future maintainers do not assume it is an oversight.

---

_Reviewed: 2026-05-02T16:45:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
