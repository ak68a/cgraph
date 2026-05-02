# Phase 2: TypeScript Extractor - Research

**Researched:** 2026-05-02
**Domain:** Tree-sitter TypeScript/TSX AST traversal, symbol extraction, edge detection
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Barrel Re-Export Resolution**
- D-25: Extractor emits ReExport edges only -- does NOT follow re-export chains across files. The indexer (Phase 3) resolves multi-hop barrel chains to find the true source. This preserves D-18 (extractors are pure transformation, no file I/O).
- D-26: Supported re-export patterns: named (`export { foo, bar } from './module'`) and star (`export * from './module'`). Star re-exports emit a single edge with a wildcard marker for the indexer to resolve.
- D-27: Renamed re-exports (`export { foo as bar }`) and default re-exports (`export { default as Foo }`) are deferred.

**Path Alias Resolution**
- D-28: Extractor emits raw import paths as-is (e.g., `@/components/Button`). No tsconfig.json reading, no alias resolution.
- D-29: Phase 2's extractor crate has zero tsconfig awareness. PARS-10 split: Phase 2 captures the raw import, Phase 3 resolves it.

**Call Edge Detection**
- D-30: Direct named calls only -- `foo()`, `Component()`, `useHook()`. Top-level identifiers in call expressions. Skip method calls (`obj.method()`), dynamic dispatch, callbacks, and IIFE.
- D-31: Captures the most valuable signal for blast radius and dead code with lowest false-positive rate.

**Symbol Extraction**
- D-32: Extract all exported symbols: functions, arrow functions, components (JSX return + PascalCase), hooks (use* prefix), types, interfaces, classes, enums.
- D-33: `is_exported` flag set based on `export` keyword presence. Default exports are captured.
- D-34: Symbol IDs follow D-01: `file_path::symbol_name` format.

**Crate Structure**
- D-35: New crate `crates/ts-extractor` with dependency on `cgraph-core`. Implements `Extractor` trait for TypeScript and TypeScriptReact languages.
- D-36: Uses `tree-sitter-typescript` crate. LANGUAGE_TYPESCRIPT for .ts, LANGUAGE_TSX for .tsx.

**Test Strategy**
- D-37: OversizeConnect-inspired fixtures -- synthetic files modeled after real patterns.
- D-38: Assertions verify key patterns, not exact counts.
- D-39: Test fixture directory: `crates/ts-extractor/tests/fixtures/`.

### Claude's Discretion
None specified -- all Phase 2 decisions are locked.

### Deferred Ideas (OUT OF SCOPE)
- Member call edges (`obj.method()`) -- post-v1 enhancement
- Renamed re-exports (`export { foo as bar }`) -- gap closure if needed
- Default re-exports (`export { default as Foo }`) -- gap closure if needed
- Import type detection (`import type { Foo }`) -- distinguish type-only imports
- Decorator extraction (`@Injectable()`, `@Component()`) -- Angular/NestJS support
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PARS-01 | Tool parses TypeScript/TSX files and extracts all exported symbols (functions, components, hooks, types, classes, interfaces) | Tree-sitter Query API with multi-pattern matching extracts all symbol types from `export_statement` with `declaration:` field; TSX handled with LANGUAGE_TSX dialect |
| PARS-05 | Tool extracts import relationships between modules | `import_statement` AST nodes provide `import_clause` (named/default/namespace) and `source` string field for the module path |
| PARS-06 | Tool extracts function/method call relationships | `call_expression` with `function: (identifier)` field captures direct named calls; member_expression calls filtered out per D-30 |
| PARS-07 | Tool extracts type reference relationships (extends, implements, uses-type) | `extends_clause`, `implements_clause`, `extends_type_clause`, and `type_annotation > type_identifier` capture all type references |
| PARS-08 | Tool extracts re-export relationships (barrel files) | `export_statement` with `source:` field and `export_clause` (named) or `*` (star) emits ReExport edges |
| PARS-09 | Tool resolves multi-hop barrel re-export chains to find the true source | Phase 2 emits ReExport edges with raw paths; Phase 3 indexer resolves multi-hop chains (per D-25) |
| PARS-10 | Tool resolves TypeScript path aliases (tsconfig paths) | Phase 2 emits raw import paths; Phase 3 reads tsconfig.json and resolves aliases (per D-28/D-29) |
</phase_requirements>

---

## Summary

Phase 2 implements a tree-sitter-based TypeScript/TSX extractor as a new Rust crate (`crates/ts-extractor`) that fulfills the `Extractor` trait defined in Phase 1. The extractor takes source text and produces `SymbolNode` and `SymbolEdge` vectors covering exported symbols, import edges, call edges, type reference edges, and re-export edges.

The implementation strategy uses tree-sitter's **Query API** with multi-pattern S-expression queries rather than manual AST traversal. This approach was verified on this machine: a single `Query` object with 6+ patterns efficiently matches all export variants in one pass, returning `pattern_index` to distinguish symbol kinds. The `StreamingIterator` trait from tree-sitter 0.26.8 provides safe iteration over query matches.

Key architectural insight: the extractor performs **two passes** over the AST. Pass 1 (symbol extraction) uses queries against `export_statement` nodes to find all declared symbols. Pass 2 (edge extraction) uses queries for `import_statement`, `call_expression`, type references, and re-exports. Both passes are read-only over the same parsed tree -- no file I/O, no cross-file resolution (per D-18, D-25, D-28).

**Primary recommendation:** Use tree-sitter Query API with compiled multi-pattern queries for extraction. Structure as a two-pass algorithm (symbols first, edges second) with shared state holding the parsed tree. Separate query sets for .ts and .tsx (TSX needs JSX element detection for component classification).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| TypeScript/TSX parsing | `crates/ts-extractor` | -- | Extractor owns its parser instance (D-09) |
| Symbol extraction (exports) | `crates/ts-extractor` | -- | Pure transformation from AST to SymbolNode vec |
| Import edge extraction | `crates/ts-extractor` | -- | Reads import_statement nodes, emits edges with raw paths |
| Call edge extraction | `crates/ts-extractor` | -- | Reads call_expression nodes, emits edges to named identifiers |
| Type reference extraction | `crates/ts-extractor` | -- | Reads extends/implements/type_annotation nodes |
| Re-export edge extraction | `crates/ts-extractor` | -- | Reads export_statement with source field |
| Import path resolution | Phase 3 indexer | -- | D-28: extractor emits raw paths, indexer resolves |
| Barrel chain resolution | Phase 3 indexer | -- | D-25: extractor emits one-hop edges, indexer resolves chains |
| Data model (SymbolNode, SymbolEdge) | `crates/core` | -- | Shared types consumed by extractor |
| Extractor trait interface | `crates/core` | -- | Trait definition lives in core |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tree-sitter` | 0.26.8 | Parse runtime, Query API, QueryCursor | Already in workspace Cargo.lock; provides Query for efficient pattern matching [VERIFIED: Cargo.lock] |
| `tree-sitter-typescript` | 0.23.2 | LANGUAGE_TYPESCRIPT and LANGUAGE_TSX grammars | Already in workspace; proven working in Phase 1 grammar tests [VERIFIED: Cargo.lock + grammar_test.rs] |
| `cgraph-core` | path dep | SymbolNode, SymbolEdge, Extractor trait, Language enum | Phase 1 deliverable; all types needed by extractor [VERIFIED: crates/core/src/] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde` | 1.0 (workspace) | Derive Serialize on ExtractionResult for test assertions | Already in workspace via core; test helper serialization |
| `serde_json` | 1.0 (workspace) | JSON snapshot tests if needed | Already in workspace; useful for debugging test output |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tree-sitter Query API | Manual TreeCursor walk | Query is declarative, less code, pattern-index dispatch; manual walk is more flexible but 3-5x more code |
| Multi-pattern single Query | Separate Query per symbol kind | Single query is one pass over the tree; separate queries are multiple passes |
| Struct-based extractor | Function-based extraction | Struct holds compiled queries (amortized cost); functions would recompile queries per call |

**Installation -- `crates/ts-extractor/Cargo.toml`:**
```toml
[package]
name = "cgraph-ts-extractor"
version.workspace = true
edition.workspace = true

[dependencies]
cgraph-core = { path = "../core" }
tree-sitter = "0.26.8"
tree-sitter-typescript = "0.23.2"

[dev-dependencies]
serde_json = "1.0"
```

**Workspace root `Cargo.toml` update:**
```toml
[workspace]
members = [
    "crates/core",
    "crates/cli",
    "crates/ts-extractor",
]
resolver = "2"
```

**Version verification:** All versions already locked in workspace Cargo.lock. No new external dependencies needed. [VERIFIED: Cargo.lock]

---

## Architecture Patterns

### System Architecture Diagram

```
Input: (&Path, &str) — file path + source text
        |
        v
[TsExtractor::extract()]
  1. Select grammar: .ts → LANGUAGE_TYPESCRIPT, .tsx → LANGUAGE_TSX
  2. Parse source → Tree
  3. Check root_node.has_error() → record ParseError if true
        |
        v
[Pass 1: Symbol Extraction]
  Run compiled symbol_query against root node
  For each match:
    - pattern_index → SymbolKind mapping
    - Capture @symbol_name → node text, line range
    - Parent export_statement → is_exported = true
    - Build SymbolNode { id: "path::name", ... }
        |
        v
[Pass 2: Edge Extraction]
  Run compiled edge queries:
    a) import_query → EdgeKind::Import edges
    b) call_query → EdgeKind::Call edges  
    c) type_ref_query → EdgeKind::TypeRef edges
    d) reexport_query → EdgeKind::ReExport edges
  For each match:
    - Build SymbolEdge { source_id, target_id, kind, source_location }
        |
        v
Output: ExtractionResult { nodes, edges, errors }
```

### Recommended Project Structure
```
crates/ts-extractor/
├── Cargo.toml
├── src/
│   ├── lib.rs              # pub mod, TsExtractor struct, Extractor impl
│   ├── queries.rs          # Compiled Query constants (lazy_static or OnceLock)
│   ├── symbols.rs          # Pass 1: symbol extraction logic
│   ├── edges.rs            # Pass 2: edge extraction (imports, calls, type_refs, re-exports)
│   └── classify.rs         # Hook/component detection heuristics (use* prefix, PascalCase + JSX)
└── tests/
    ├── extraction_test.rs  # Integration tests using fixture files
    └── fixtures/
        ├── barrel.ts       # Re-export patterns (named + star)
        ├── hooks.ts        # Custom hooks with useState/useEffect calls
        ├── components.tsx  # React components with JSX, imports, type refs
        ├── schemas.ts      # Zod schemas, type aliases, interfaces
        ├── services.ts     # Class with extends/implements, method calls
        ├── enums.ts        # Enum declarations
        └── index.ts        # Barrel file re-exporting from other fixtures
```

### Pattern 1: TsExtractor Struct with Compiled Queries

**What:** A struct that pre-compiles tree-sitter queries on construction so they can be reused across multiple `extract()` calls without recompilation overhead.

**When to use:** The extractor struct definition and initialization.

```rust
// Source: Verified tree-sitter Query API behavior via compile test [VERIFIED: /tmp/ast_inspect]
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::{Language, Parser, Query};
use tree_sitter_typescript::{LANGUAGE_TYPESCRIPT, LANGUAGE_TSX};
use cgraph_core::{Extractor, ExtractionResult, ParseError};
use cgraph_core::model::{Language as CgLanguage, SymbolNode, SymbolEdge};

pub struct TsExtractor {
    // Queries are compiled once and reused
    symbol_query: Query,
    import_query: Query,
    call_query: Query,
    type_ref_query: Query,
    reexport_query: Query,
}

impl TsExtractor {
    pub fn new() -> Self {
        let lang: Language = LANGUAGE_TYPESCRIPT.into();
        Self {
            symbol_query: Query::new(&lang, SYMBOL_QUERY_SRC).expect("symbol query"),
            import_query: Query::new(&lang, IMPORT_QUERY_SRC).expect("import query"),
            call_query: Query::new(&lang, CALL_QUERY_SRC).expect("call query"),
            type_ref_query: Query::new(&lang, TYPE_REF_QUERY_SRC).expect("type ref query"),
            reexport_query: Query::new(&lang, REEXPORT_QUERY_SRC).expect("reexport query"),
        }
    }
}
```

### Pattern 2: Multi-Pattern Symbol Query

**What:** A single tree-sitter query with multiple patterns to extract all exported symbol kinds in one pass.

**When to use:** Symbol extraction (Pass 1).

```rust
// Source: Verified via /tmp/ast_inspect compile test [VERIFIED: multi-pattern query works]
const SYMBOL_QUERY_SRC: &str = r#"
; Pattern 0: exported function declaration
(export_statement
  declaration: (function_declaration
    name: (identifier) @symbol_name)) @export_stmt

; Pattern 1: exported arrow function (const)
(export_statement
  declaration: (lexical_declaration
    (variable_declarator
      name: (identifier) @symbol_name
      value: (arrow_function)))) @export_stmt

; Pattern 2: exported interface
(export_statement
  declaration: (interface_declaration
    name: (type_identifier) @symbol_name)) @export_stmt

; Pattern 3: exported type alias
(export_statement
  declaration: (type_alias_declaration
    name: (type_identifier) @symbol_name)) @export_stmt

; Pattern 4: exported class
(export_statement
  declaration: (class_declaration
    name: (type_identifier) @symbol_name)) @export_stmt

; Pattern 5: exported enum
(export_statement
  declaration: (enum_declaration
    name: (identifier) @symbol_name)) @export_stmt
"#;

// Map pattern_index to SymbolKind
fn pattern_to_kind(pattern_index: usize) -> SymbolKind {
    match pattern_index {
        0 => SymbolKind::Function,  // further classified as Hook if use* prefix
        1 => SymbolKind::Function,  // arrow fn; classified as Hook/Component by name
        2 => SymbolKind::Interface,
        3 => SymbolKind::Type,
        4 => SymbolKind::Class,
        5 => SymbolKind::Enum,
        _ => SymbolKind::Function,
    }
}
```

### Pattern 3: Call Edge Detection (Direct Named Calls Only)

**What:** Query for `call_expression` where the function is a bare `identifier` (not a member_expression).

**When to use:** Edge extraction (Pass 2) for D-30.

```rust
// Source: Verified via /tmp/ast_inspect [VERIFIED: correctly skips obj.method() calls]
const CALL_QUERY_SRC: &str = r#"
(call_expression
  function: (identifier) @call_target)
"#;
// This matches: foo(), useState(), Component()
// This does NOT match: obj.method(), super.call(), this.foo()
// The query naturally filters by requiring function: to be (identifier) not (member_expression)
```

### Pattern 4: Re-Export Edge Detection

**What:** Detect named and star re-exports by looking for `export_statement` nodes that have a `source:` field (indicating they re-export from another module).

**When to use:** Edge extraction for PARS-08.

```rust
// Source: Verified via /tmp/ast_inspect [VERIFIED: source field present on re-export statements]
const REEXPORT_QUERY_SRC: &str = r#"
; Named re-export: export { foo, bar } from './module'
(export_statement
  (export_clause
    (export_specifier
      name: (identifier) @specifier_name))
  source: (string
    (string_fragment) @source_path)) @named_reexport

; Star re-export: export * from './helpers'
(export_statement
  source: (string
    (string_fragment) @star_source)) @star_reexport
"#;
// Distinguish star vs named by checking:
// - pattern_index (star_reexport pattern) OR
// - absence of export_clause child on the export_statement node
```

### Pattern 5: StreamingIterator Usage

**What:** Correct iteration pattern for tree-sitter QueryMatches in Rust.

**When to use:** All query execution loops.

```rust
// Source: tree-sitter 0.26.8 API [VERIFIED: compile test]
use tree_sitter::{QueryCursor, StreamingIterator};

let mut cursor = QueryCursor::new();
let mut matches = cursor.matches(&self.symbol_query, root_node, source.as_bytes());

while let Some(m) = matches.next() {
    let pattern_idx = m.pattern_index;
    let name_capture_idx = self.symbol_query.capture_index_for_name("symbol_name").unwrap();
    
    for cap in m.captures {
        if cap.index == name_capture_idx as u32 {
            let name = &source[cap.node.byte_range()];
            let line_start = cap.node.start_position().row as u32 + 1;
            // ... build SymbolNode
        }
    }
}
```

### Pattern 6: Hook and Component Classification

**What:** Heuristic classification of functions as Hooks or Components based on naming conventions.

**When to use:** After symbol extraction, reclassify Function kind.

```rust
// Source: React naming conventions [ASSUMED - standard React convention]
fn classify_symbol(name: &str, kind: SymbolKind, has_jsx_return: bool) -> SymbolKind {
    match kind {
        SymbolKind::Function => {
            if name.starts_with("use") && name.chars().nth(3).is_some_and(|c| c.is_uppercase()) {
                SymbolKind::Hook
            } else if name.chars().next().is_some_and(|c| c.is_uppercase()) && has_jsx_return {
                // PascalCase + JSX return = component (treated as Function kind for now)
                SymbolKind::Function // No separate Component kind in model
            } else {
                SymbolKind::Function
            }
        }
        other => other,
    }
}
```

### Anti-Patterns to Avoid

- **Manual TreeCursor walk for extraction:** The Query API is purpose-built for this. Manual walking produces 3-5x more code, is harder to maintain, and more error-prone. Only use manual walk for cases queries cannot express (e.g., detecting JSX return inside a function body requires checking descendants).

- **Compiling queries per `extract()` call:** Query compilation is not free. Compile once on struct construction or use `OnceLock`/`LazyLock`. The extractor may be called hundreds of times per scan.

- **Matching `export_statement` without the `declaration:` field:** Using `(export_statement (function_declaration ...))` without specifying the field name `declaration:` could match nested structures. Always use field names when available.

- **Using `LANGUAGE_TYPESCRIPT` for `.tsx` files:** TSX has additional JSX grammar rules. Using the wrong language will produce ERROR nodes on JSX syntax. Must select grammar based on file extension (D-36).

- **Attempting cross-file resolution:** Per D-18 and D-25, the extractor receives text and returns graph fragments. Never open files, read tsconfig.json, or follow import paths within the extractor.

- **Building symbol IDs without the file path:** IDs must be `file_path::symbol_name` (D-01). The `path` parameter to `extract()` provides the file path component.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TypeScript parsing | Custom regex or string matching | tree-sitter Query API | Regex cannot handle nested TypeScript syntax; tree-sitter handles all edge cases including template literals, generics, arrow functions |
| AST pattern matching | Recursive node visitor with match arms | tree-sitter Query with S-expressions | Query patterns are declarative, composable, and the tree-sitter engine optimizes matching internally |
| Hook detection | Complex heuristic analyzing function body | `name.starts_with("use")` convention check | React's naming convention is the standard detection method; body analysis adds complexity without accuracy |
| Export detection | Regex for `export` keyword | Query for `export_statement` parent | Regex breaks on comments, strings containing "export", re-exports vs declarations |
| Call graph extraction | Walking every expression looking for calls | `(call_expression function: (identifier) @name)` query | Query naturally filters to direct calls, skips member expressions, and returns only the function name node |

**Key insight:** Tree-sitter's Query API eliminates the need for manual AST traversal patterns common in other parser ecosystems (visitor pattern, recursive descent). A well-written query set replaces hundreds of lines of matching logic with declarative patterns.

---

## Common Pitfalls

### Pitfall 1: TSX vs TypeScript Grammar Selection

**What goes wrong:** Using `LANGUAGE_TYPESCRIPT` grammar to parse a `.tsx` file. Tree-sitter produces ERROR nodes on JSX elements like `<div>`, `<Component />` because the TypeScript grammar doesn't include JSX productions.

**Why it happens:** Both `.ts` and `.tsx` look like TypeScript. The grammar difference is invisible until JSX appears.

**How to avoid:** Check file extension in `can_handle()` and select grammar accordingly:
```rust
let lang = if path.extension().map_or(false, |e| e == "tsx") {
    LANGUAGE_TSX.into()
} else {
    LANGUAGE_TYPESCRIPT.into()
};
```

**Warning signs:** `root_node.has_error()` returns true on valid TSX files. ERROR nodes appear at JSX element positions.

[VERIFIED: Phase 1 grammar_test.rs uses separate LANGUAGE_TSX for sample.tsx]

### Pitfall 2: Query Pattern Compatibility Between TS and TSX Grammars

**What goes wrong:** Assuming queries compiled against `LANGUAGE_TYPESCRIPT` work identically against `LANGUAGE_TSX`. While the core TypeScript node types are the same in both grammars, the TSX grammar adds `jsx_element`, `jsx_self_closing_element`, etc. Queries that work for one should work for both for non-JSX constructs, but you need separate queries compiled against the correct language.

**Why it happens:** tree-sitter queries are validated against a specific language's node type set at compile time.

**How to avoid:** Either (a) compile two sets of queries (one per grammar), or (b) use a single grammar (TSX) for all TypeScript files since TSX is a superset of TS. Option (b) is simpler but means non-TSX code gets parsed with the slightly larger TSX grammar.

**Recommendation:** Use approach (b) -- parse everything with LANGUAGE_TSX. The TSX grammar is a strict superset of TypeScript grammar. Non-JSX TypeScript parses identically in both. This eliminates dual-query compilation.

[VERIFIED: compiled test confirms TypeScript-only code parses correctly under LANGUAGE_TSX]

### Pitfall 3: Default Exports Without Declaration Names

**What goes wrong:** `export default config;` (exporting an existing identifier) does not have a `declaration:` field. The node structure is `export_statement > identifier`. This pattern is NOT matched by the symbol query patterns that look for `declaration: (function_declaration ...)` etc.

**Why it happens:** The `declaration:` field only exists when the export declares something new. Exporting an existing binding uses a different AST shape.

**How to avoid:** Add a separate query pattern for default export of identifiers:
```
(export_statement
  (identifier) @default_name)
```
And for default export of anonymous functions/classes:
```
(export_statement
  declaration: (function_declaration) @default_func)
; This already works if function has a name
```

**Warning signs:** Missing symbols in extraction results for files that use `export default existingVariable`.

[VERIFIED: /tmp/ast_inspect confirmed `export default config` produces `export_statement > identifier` without declaration field]

### Pitfall 4: Star Re-Export Query Overlap with Named Re-Exports

**What goes wrong:** The star re-export query `(export_statement source: (string ...))` also matches named re-exports (they also have a `source:` field). Both patterns fire for `export { foo } from './module'`.

**Why it happens:** Both star and named re-exports have the `source` field. The star query is too broad.

**How to avoid:** Use `pattern_index` to disambiguate, or add a negative constraint. The named re-export pattern (which has `export_clause`) should be checked first; star re-exports are those that have a `source:` field but NO `export_clause` child. In practice, use the `pattern_index` from the multi-pattern query to know which matched.

Alternatively, for star re-exports only, check that the export_statement has no `export_clause` named child:
```rust
// After matching a "star_reexport" pattern, verify it's truly a star export
let export_node = /* the @star_reexport captured node */;
let has_export_clause = export_node.child_by_field_name("declaration").is_none()
    && export_node.children(&mut cursor).any(|c| c.kind() == "export_clause").not();
```

[VERIFIED: /tmp/ast_inspect output shows both named and star re-exports have source field]

### Pitfall 5: `source_id` and `target_id` for Edges Before Symbol ID Resolution

**What goes wrong:** Attempting to build fully-resolved symbol IDs (`file_path::symbol_name`) for edge targets that reference symbols in other files. The extractor only sees one file at a time.

**Why it happens:** Import edges reference symbols in other files. The extractor doesn't know the target file path (only the raw import path like `'./module'`).

**How to avoid:** Per D-18 and D-28, edge `target_id` for imports uses the **raw import path** as a placeholder (e.g., `"./module::useState"`). The indexer (Phase 3) resolves this to the actual file path. For edges within the same file (calls, type refs), use the file path from the `path` parameter.

Edge ID strategy:
- **Import edges:** `source_id = "this_file::importing_context"`, `target_id = "raw_path::symbol_name"`
- **Call edges:** `source_id = "this_file::calling_function"`, `target_id = "unresolved::called_name"` (indexer resolves)
- **Type ref edges:** `source_id = "this_file::source_symbol"`, `target_id = "unresolved::referenced_type"`
- **Re-export edges:** `source_id = "this_file::*"` or `"this_file::specifier_name"`, `target_id = "raw_path::specifier_name"`

[ASSUMED -- exact ID format for unresolved edges needs to be consistent with Phase 3 indexer expectations]

### Pitfall 6: StreamingIterator Import Path

**What goes wrong:** `QueryMatches` implements `StreamingIterator`, not `Iterator`. Code like `for m in cursor.matches(...)` will not compile. Must use `while let Some(m) = matches.next()`.

**Why it happens:** tree-sitter 0.24+ changed from returning owned copies to borrowing from internal state. The `StreamingIterator` trait is re-exported from tree-sitter.

**How to avoid:**
```rust
use tree_sitter::StreamingIterator;  // MUST import this trait

let mut matches = cursor.matches(&query, node, source.as_bytes());
while let Some(m) = matches.next() {
    // m is a &QueryMatch -- borrowed, not owned
}
```

[VERIFIED: compile test confirms this pattern works with tree-sitter 0.26.8]

---

## Code Examples

### Complete Extractor Trait Implementation Pattern

```rust
// Source: crates/core/src/extractor.rs trait definition [VERIFIED: existing code]
// + tree-sitter Query API [VERIFIED: compile test]
use std::path::Path;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_typescript::{LANGUAGE_TYPESCRIPT, LANGUAGE_TSX};
use cgraph_core::{
    Extractor, ExtractionResult, ParseError,
    Language, SymbolNode, SymbolEdge, SymbolKind, EdgeKind,
};

pub struct TsExtractor {
    ts_lang: tree_sitter::Language,
    tsx_lang: tree_sitter::Language,
    symbol_query_ts: Query,
    symbol_query_tsx: Query,
    // ... other queries
}

impl Extractor for TsExtractor {
    fn language(&self) -> Language {
        Language::TypeScript // handles both TS and TSX
    }

    fn can_handle(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ts") | Some("tsx")
        )
    }

    fn extract(&self, path: &Path, source: &str) -> ExtractionResult {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut errors = Vec::new();

        // 1. Select grammar
        let is_tsx = path.extension().map_or(false, |e| e == "tsx");
        let lang = if is_tsx { &self.tsx_lang } else { &self.ts_lang };

        // 2. Parse
        let mut parser = Parser::new();
        parser.set_language(lang).expect("grammar load");
        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => {
                errors.push(ParseError::PartialParse {
                    path: path.display().to_string(),
                    line: 0,
                });
                return ExtractionResult { nodes, edges, errors };
            }
        };

        let root = tree.root_node();
        
        // 3. Check for parse errors (D-14: still extract what's available)
        if root.has_error() {
            errors.push(ParseError::PartialParse {
                path: path.display().to_string(),
                line: root.start_position().row as u32,
            });
        }

        let file_path = path.display().to_string();

        // 4. Pass 1: Extract symbols
        // ... (use self.symbol_query_ts or symbol_query_tsx based on is_tsx)

        // 5. Pass 2: Extract edges
        // ... (import, call, type_ref, re-export queries)

        ExtractionResult { nodes, edges, errors }
    }
}
```

### Import Edge Extraction

```rust
// Source: Verified AST structure via /tmp/ast_inspect [VERIFIED]
const IMPORT_QUERY_SRC: &str = r#"
; Named imports: import { foo, bar } from './module'
(import_statement
  (import_clause
    (named_imports
      (import_specifier
        name: (identifier) @import_name)))
  source: (string
    (string_fragment) @import_path))

; Default import: import axios from 'axios'
(import_statement
  (import_clause
    (identifier) @default_import_name)
  source: (string
    (string_fragment) @import_path))

; Namespace import: import * as utils from './utils'
(import_statement
  (import_clause
    (namespace_import
      (identifier) @namespace_name))
  source: (string
    (string_fragment) @import_path))
"#;
```

### Type Reference Edge Extraction

```rust
// Source: Verified AST structure via /tmp/ast_inspect [VERIFIED]
const TYPE_REF_QUERY_SRC: &str = r#"
; Class extends: class Foo extends Bar
(class_declaration
  name: (type_identifier) @class_name
  (class_heritage
    (extends_clause
      (identifier) @extends_target)))

; Class implements: class Foo implements Bar, Baz
(class_declaration
  name: (type_identifier) @class_name
  (class_heritage
    (implements_clause
      (type_identifier) @implements_target)))

; Interface extends: interface Foo extends Bar
(interface_declaration
  name: (type_identifier) @iface_name
  (extends_type_clause
    (type_identifier) @extends_target))

; Type annotations referencing custom types
(type_annotation
  (type_identifier) @type_ref)
"#;
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual AST visitor pattern | tree-sitter Query API | tree-sitter 0.20+ | Declarative pattern matching replaces recursive visitors; 3-5x less code |
| `Language::unsafe_from_raw_ptr()` | `LANGUAGE_TYPESCRIPT.into()` via `LanguageFn` | tree-sitter 0.23+ | Safe, no unsafe code needed |
| `Iterator` for query results | `StreamingIterator` for `QueryMatches` | tree-sitter 0.24+ | Must import `StreamingIterator` trait; use `while let Some(m) = matches.next()` |
| Separate TS and TSX parser configurations | TSX grammar as superset | tree-sitter-typescript grammar design | TSX grammar handles all TS code correctly; can use single grammar for both |

**Deprecated/outdated (do not use):**
- `tree_sitter::Parser::set_language(unsafe_lang_ptr)` -- replaced by safe `LanguageFn` pattern
- `for m in cursor.matches(...)` -- `QueryMatches` is a `StreamingIterator`, not `Iterator`
- Custom tree visitor with recursive `walk()` calls -- Query API handles all extraction needs

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | TSX grammar is a strict superset of TypeScript grammar (TS-only code parses identically in both) | Pitfall 2 | Would need dual grammar compilation; verified empirically but not by official docs |
| A2 | Edge target_id for unresolved cross-file references uses format `"raw_path::symbol_name"` | Pitfall 5 | Phase 3 indexer would expect a different format; needs alignment during Phase 3 planning |
| A3 | Hook detection via `name.starts_with("use")` + 4th char uppercase is sufficient | Pattern 6 | False positives on names like `useful`, `userService`; the 4th-char check mitigates this |
| A4 | `type_annotation > type_identifier` captures meaningful type references without excessive noise | Code Examples | Could produce many edges to built-in types (string, number, boolean); need filtering for `predefined_type` vs `type_identifier` |
| A5 | Queries compiled against LANGUAGE_TYPESCRIPT work against LANGUAGE_TSX parse trees | Architecture | If not, need separate query compilation per grammar; could increase struct size |

---

## Open Questions

1. **Edge target_id format for unresolved references**
   - What we know: Import edges reference symbols in other files. The extractor cannot resolve paths.
   - What's unclear: The exact string format for `target_id` that Phase 3 indexer will consume.
   - Recommendation: Use `"raw_import_path::symbol_name"` format. Document the contract. Phase 3 research will confirm or adjust.

2. **Should non-exported symbols be extracted?**
   - What we know: D-32 says "extract all exported symbols." Internal functions used only within the file are not explicitly mentioned.
   - What's unclear: Are internal symbols needed for intra-file call edges?
   - Recommendation: Extract non-exported symbols with `is_exported = false` so call edges within the same file can reference them. The `is_exported` flag lets downstream phases filter.

3. **Component vs Function distinction in SymbolKind**
   - What we know: The `SymbolKind` enum from Phase 1 has `Function`, `Hook` but no `Component` variant.
   - What's unclear: Should components be classified as `Function` or should `SymbolKind` be extended?
   - Recommendation: Classify as `Function` for now. If visualization (Phase 4) needs component distinction, add it then. Current model is locked per D-03.

4. **Query compatibility across TS and TSX Language objects**
   - What we know: Queries are compiled against a specific `Language`. TSX and TS are separate `Language` values.
   - What's unclear: Whether a query compiled with `LANGUAGE_TYPESCRIPT` works on a tree parsed with `LANGUAGE_TSX`.
   - Recommendation: Compile queries against `LANGUAGE_TSX` (superset). If needed, maintain two query sets. This needs empirical verification during implementation.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust / Cargo | Build system | Yes | 1.93.1 | -- |
| tree-sitter | Parsing runtime | Yes | 0.26.8 (in Cargo.lock) | -- |
| tree-sitter-typescript | TS/TSX grammar | Yes | 0.23.2 (in Cargo.lock) | -- |
| Apple clang | Grammar C compilation | Yes | 17.0.0 | -- |

**Missing dependencies:** None blocking. All dependencies already in workspace.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`#[test]`, `cargo test`) |
| Config file | None -- built into Cargo |
| Quick run command | `cargo test -p cgraph-ts-extractor` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| PARS-01 | Exported functions extracted with correct name/kind | Integration | `cargo test -p cgraph-ts-extractor -- exported_functions` | No -- Wave 0 |
| PARS-01 | Exported interfaces/types/classes/enums extracted | Integration | `cargo test -p cgraph-ts-extractor -- exported_types` | No -- Wave 0 |
| PARS-01 | Hooks detected via use* naming | Integration | `cargo test -p cgraph-ts-extractor -- hook_detection` | No -- Wave 0 |
| PARS-01 | TSX components with JSX return extracted | Integration | `cargo test -p cgraph-ts-extractor -- tsx_components` | No -- Wave 0 |
| PARS-01 | Default exports captured | Integration | `cargo test -p cgraph-ts-extractor -- default_exports` | No -- Wave 0 |
| PARS-05 | Named imports produce Import edges | Integration | `cargo test -p cgraph-ts-extractor -- import_named` | No -- Wave 0 |
| PARS-05 | Default imports produce Import edges | Integration | `cargo test -p cgraph-ts-extractor -- import_default` | No -- Wave 0 |
| PARS-05 | Namespace imports produce Import edges | Integration | `cargo test -p cgraph-ts-extractor -- import_namespace` | No -- Wave 0 |
| PARS-06 | Direct function calls produce Call edges | Integration | `cargo test -p cgraph-ts-extractor -- call_direct` | No -- Wave 0 |
| PARS-06 | Method calls (obj.method()) are NOT captured | Integration | `cargo test -p cgraph-ts-extractor -- call_no_member` | No -- Wave 0 |
| PARS-07 | Class extends produces TypeRef edge | Integration | `cargo test -p cgraph-ts-extractor -- type_ref_extends` | No -- Wave 0 |
| PARS-07 | Class implements produces TypeRef edges | Integration | `cargo test -p cgraph-ts-extractor -- type_ref_implements` | No -- Wave 0 |
| PARS-07 | Interface extends produces TypeRef edge | Integration | `cargo test -p cgraph-ts-extractor -- type_ref_iface_extends` | No -- Wave 0 |
| PARS-08 | Named re-export produces ReExport edges | Integration | `cargo test -p cgraph-ts-extractor -- reexport_named` | No -- Wave 0 |
| PARS-08 | Star re-export produces single wildcard ReExport edge | Integration | `cargo test -p cgraph-ts-extractor -- reexport_star` | No -- Wave 0 |
| PARS-09 | Extractor emits raw ReExport edge (resolution is Phase 3) | Integration | `cargo test -p cgraph-ts-extractor -- reexport_raw_path` | No -- Wave 0 |
| PARS-10 | Extractor emits raw import path (alias resolution is Phase 3) | Integration | `cargo test -p cgraph-ts-extractor -- import_raw_alias_path` | No -- Wave 0 |
| (D-14) | Partial parse produces errors but still extracts available symbols | Integration | `cargo test -p cgraph-ts-extractor -- partial_parse` | No -- Wave 0 |
| (D-36) | .ts uses LANGUAGE_TYPESCRIPT, .tsx uses LANGUAGE_TSX | Unit | `cargo test -p cgraph-ts-extractor -- grammar_selection` | No -- Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p cgraph-ts-extractor`
- **Per wave merge:** `cargo test` (full workspace)
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/ts-extractor/Cargo.toml` -- package definition with dependencies
- [ ] `crates/ts-extractor/src/lib.rs` -- module structure and TsExtractor struct
- [ ] `crates/ts-extractor/tests/extraction_test.rs` -- integration test harness
- [ ] `crates/ts-extractor/tests/fixtures/` -- all fixture files (barrel.ts, hooks.ts, components.tsx, schemas.ts, services.ts, enums.ts, index.ts)
- [ ] Workspace `Cargo.toml` update to include `crates/ts-extractor`

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | No auth in extractor crate |
| V3 Session Management | No | Stateless, single-file processing |
| V4 Access Control | No | Receives pre-read text, no file system access |
| V5 Input Validation | Yes | Validate source text is valid UTF-8; handle zero-length input gracefully; tree-sitter handles malformed syntax via partial parse |
| V6 Cryptography | No | No secrets or crypto |
| V7 Error Handling | Yes | ParseError returned as data (D-17); no panics on malformed input |

### Known Threat Patterns for tree-sitter parsing

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed input causing infinite loop in parser | DoS | tree-sitter has built-in timeout support via `parser.set_timeout_micros()`; also, grammar is compiled C code with bounded recursion |
| Extremely large files causing OOM | DoS | Caller (indexer) should enforce file size limits before passing to extractor; extractor trusts caller per D-18 |
| Unicode edge cases in identifiers | Tampering | tree-sitter handles Unicode; Rust strings are UTF-8; `byte_range()` returns correct byte offsets |

---

## Sources

### Primary (HIGH confidence)
- [Cargo.lock in workspace] -- tree-sitter 0.26.8, tree-sitter-typescript 0.23.2 locked versions [VERIFIED]
- [crates/core/src/extractor.rs] -- Extractor trait definition, ExtractionResult, ParseError [VERIFIED: file read]
- [crates/core/src/model.rs] -- SymbolNode, SymbolEdge, SymbolKind, EdgeKind, Language [VERIFIED: file read]
- [/tmp/ast_inspect compile tests] -- TypeScript AST node types, Query API patterns, multi-pattern queries, field names [VERIFIED: built and ran]
- [Context7: /tree-sitter/tree-sitter-typescript] -- AST node types for exports, imports, declarations [VERIFIED]
- [Context7: /websites/rs_tree-sitter_tree_sitter] -- Query, QueryCursor, StreamingIterator API [VERIFIED]
- [Context7: /websites/tree-sitter_github_io_tree-sitter] -- Query syntax (S-expressions, captures, alternation, field names) [VERIFIED]

### Secondary (MEDIUM confidence)
- [crates/core/tests/grammar_test.rs] -- Established parser initialization pattern [VERIFIED: file read]
- [Phase 1 CONTEXT.md] -- D-01 through D-24 decisions [CITED: .planning/phases/01-foundation/01-CONTEXT.md]

### Tertiary (LOW confidence)
- None.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in workspace, versions locked, verified working
- Architecture: HIGH -- Query API approach verified via compile tests; AST node types confirmed empirically
- Pitfalls: HIGH -- each pitfall verified via actual AST inspection; StreamingIterator pattern confirmed working
- Edge ID format: MEDIUM -- exact unresolved target_id format is an assumption pending Phase 3 alignment

**Research date:** 2026-05-02
**Valid until:** 2026-08-02 (90 days -- tree-sitter grammar and Rust crate ecosystem are stable)
