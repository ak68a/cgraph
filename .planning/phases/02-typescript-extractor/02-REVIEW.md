---
phase: 02-typescript-extractor
reviewed: 2026-05-02T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - Cargo.toml
  - crates/ts-extractor/Cargo.toml
  - crates/ts-extractor/src/classify.rs
  - crates/ts-extractor/src/edges.rs
  - crates/ts-extractor/src/lib.rs
  - crates/ts-extractor/src/queries.rs
  - crates/ts-extractor/src/symbols.rs
  - crates/ts-extractor/tests/extraction_test.rs
findings:
  critical: 1
  warning: 4
  info: 3
  total: 8
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-05-02
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

The TypeScript extractor is structurally sound and all 27 tests pass. The two-pass extraction
algorithm, tree-sitter query design, hook detection, and the star-re-export disambiguation guard
(checking for absence of `export_clause`) are all well-reasoned. However, the guard has one gap
that produces a wrong edge for the `export * as ns from` pattern. Additionally, there is no
deduplication of symbol nodes, which means TypeScript function overloads produce multiple
`SymbolNode` entries with identical `id` fields. Four compiler warnings indicate dead code that
should be cleaned up.

---

## Critical Issues

### CR-01: `export * as ns from './module'` Misclassified as Star Re-Export

**File:** `crates/ts-extractor/src/edges.rs:251-265`

**Issue:** The guard that prevents named re-exports from being emitted as star edges checks only
for the absence of an `export_clause` child on the `export_statement` node. However,
`export * as ns from './module'` (a namespace re-export) also has no `export_clause` child — it
has a `namespace_export` child instead. The current guard therefore lets namespace re-exports
through, emitting an incorrect `source_id = "file::*", target_id = "raw_path::*"` edge for
what is actually a named namespace binding, not a wildcard.

The `is_true_star` predicate at line 252 only excludes named exports (those with `export_clause`);
it does not exclude `namespace_export`:

```rust
// Current (broken):
let is_true_star = export_stmt_node.map_or(false, |stmt| {
    let mut cursor2 = stmt.walk();
    !stmt.children(&mut cursor2).any(|child| child.kind() == "export_clause")
});
```

**Fix:** Also require the absence of a `namespace_export` child:

```rust
let is_true_star = export_stmt_node.map_or(false, |stmt| {
    let mut cursor2 = stmt.walk();
    !stmt.children(&mut cursor2).any(|child| {
        child.kind() == "export_clause" || child.kind() == "namespace_export"
    })
});
```

Add a test covering this case:

```rust
#[test]
fn reexport_namespace_not_treated_as_star() {
    let extractor = TsExtractor::new();
    let source = r#"export * as utils from './utils';"#;
    let result = extractor.extract(Path::new("index.ts"), source);

    let star_edges: Vec<_> = result.edges.iter()
        .filter(|e| e.kind == cgraph_core::EdgeKind::ReExport && e.target_id.ends_with("::*"))
        .collect();
    assert!(
        star_edges.is_empty(),
        "export * as ns should not produce a star edge. Got: {:?}", star_edges
    );
}
```

---

## Warnings

### WR-01: Duplicate `SymbolNode` IDs for TypeScript Function Overloads

**File:** `crates/ts-extractor/src/symbols.rs:24-75`

**Issue:** The symbol query (Pattern 0) matches every `export_statement` wrapping a
`function_declaration`. TypeScript function overloads each appear as a separate top-level
`export_statement` node in the AST, so a file with:

```typescript
export function process(a: string): void;
export function process(a: number): void;
export function process(a: string | number): void { ... }
```

produces three `SymbolNode` entries all with `id = "file::process"`. No deduplication is
performed in `extract_symbols`. Downstream consumers that index by `id` would silently
overwrite or corrupt the graph for any overloaded function.

The non-exported path in `extract_non_exported_functions` has an `already_exists` guard at
lines 103 and 129, but this guard is absent from the main `extract_symbols` loop (the exported
path).

**Fix:** Deduplicate after collecting, keeping the last (implementation) entry, or add an
`already_exists` guard inside the exported match loop:

```rust
// After collecting all nodes in extract_symbols, before returning:
nodes.dedup_by(|a, b| a.id == b.id);
// Or use a seen-set keyed on id during the loop.
```

Alternatively, skip overload signatures explicitly: tree-sitter marks overload signatures with
a body of `None` on the `function_declaration` node — check `node.child_by_field_name("body")`
and skip matches where the body is absent.

---

### WR-02: `ts_lang` Field is Dead Code — Compiler Warning

**File:** `crates/ts-extractor/src/lib.rs:15`

**Issue:** The `ts_lang` field is stored in `TsExtractor` but never read after construction.
The implementation correctly uses `tsx_lang` for all parsing (since TSX is a superset of TS),
making `ts_lang` permanently dead. The compiler emits `warning: field 'ts_lang' is never read`
at build time.

```rust
pub struct TsExtractor {
    ts_lang: TsLanguage,   // <-- never read
    tsx_lang: TsLanguage,
    ...
}
```

**Fix:** Remove the `ts_lang` field and its initialization in `new()`:

```rust
// In new():
// Remove: let ts_lang: TsLanguage = LANGUAGE_TYPESCRIPT.into();
// Remove: ts_lang field from Self { ... }
```

---

### WR-03: Unused Imports in `lib.rs` — Compiler Warning

**File:** `crates/ts-extractor/src/lib.rs:11`

**Issue:** `SymbolNode` and `SymbolEdge` are imported but never referenced directly in `lib.rs`.
The compiler emits `warning: unused imports: 'SymbolEdge' and 'SymbolNode'`.

```rust
use cgraph_core::{
    Extractor, ExtractionResult, ParseError,
    Language, SymbolNode, SymbolEdge,   // SymbolNode and SymbolEdge unused
};
```

**Fix:** Remove the unused imports:

```rust
use cgraph_core::{
    Extractor, ExtractionResult, ParseError,
    Language,
};
```

---

### WR-04: `PartialParse` Error Always Reports Line 0

**File:** `crates/ts-extractor/src/lib.rs:101-106`

**Issue:** When `root.has_error()` is true, the error is recorded with
`line: root.start_position().row as u32` — but the root node always starts at row 0.
This means every partial-parse error is reported as occurring on line 0, regardless of where
the actual syntax error is in the file. The `ParseError::PartialParse.line` field becomes
useless for diagnostics.

```rust
if root.has_error() {
    errors.push(ParseError::PartialParse {
        path: path.display().to_string(),
        line: root.start_position().row as u32,  // always 0
    });
}
```

**Fix:** Walk the tree to find the first ERROR node and report its line. tree-sitter provides
`root.descendant_for_byte_range()` or a cursor walk to locate ERROR nodes:

```rust
if root.has_error() {
    // Find the first ERROR node in the tree
    let error_line = find_first_error_line(root);
    errors.push(ParseError::PartialParse {
        path: path.display().to_string(),
        line: error_line,
    });
}

fn find_first_error_line(node: Node) -> u32 {
    if node.is_error() {
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

## Info

### IN-01: `tests/fixtures/index.ts` Is Never Read by Any Test

**File:** `crates/ts-extractor/tests/fixtures/index.ts`

**Issue:** The fixture file `index.ts` (which contains barrel-style named and star re-exports)
exists but is not referenced by any test in `extraction_test.rs`. It is dead fixture data.

**Fix:** Either add tests that read this fixture to cover the multi-module barrel pattern, or
remove the file to avoid confusion about what is tested.

---

### IN-02: `barrel.ts` Fixture References Undefined Symbol `UserSchema`

**File:** `crates/ts-extractor/tests/fixtures/barrel.ts:2`

**Issue:** `barrel.ts` contains `export { UserSchema, type UserType } from './schemas'`, but
`schemas.ts` does not export any symbol named `UserSchema`. This makes the fixture semantically
invalid TypeScript (it would be a type error at compile time). While the extractor emits raw
edges without semantic validation and no test currently checks for `UserSchema` re-export edges,
the misleading fixture could cause confusion when tests are extended.

**Fix:** Either add `export const UserSchema = ...` to `schemas.ts`, or remove `UserSchema` from
the `barrel.ts` re-export to keep fixtures consistent with each other.

---

### IN-03: `export var` Arrow Functions Are Not Captured

**File:** `crates/ts-extractor/src/queries.rs:9-14`, `crates/ts-extractor/src/symbols.rs:118`

**Issue:** Pattern 1 of `SYMBOL_QUERY_SRC` matches `lexical_declaration` (which covers `const`
and `let`), but not `variable_declaration` (which covers `var`). Arrow functions declared with
`export var foo = () => {}` are silently ignored. The same gap exists in
`extract_non_exported_functions`. This is an undocumented limitation.

**Fix:** Either document this as an intentional scope decision (since `var` for arrow functions
is unusual in modern TypeScript), or add a second pattern to `SYMBOL_QUERY_SRC` that matches
`variable_declaration`:

```
; Pattern 1b: exported arrow function (var - uncommon but valid)
(export_statement
  declaration: (variable_declaration
    (variable_declarator
      name: (identifier) @symbol_name
      value: (arrow_function)))) @export_stmt
```

---

_Reviewed: 2026-05-02_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
