# Phase 2: TypeScript Extractor - Pattern Map

**Mapped:** 2026-05-02
**Files analyzed:** 12 (new/modified files)
**Analogs found:** 5 / 12

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/ts-extractor/Cargo.toml` | config | N/A | `crates/core/Cargo.toml` | exact |
| `crates/ts-extractor/src/lib.rs` | module-root | N/A | `crates/core/src/lib.rs` | role-match |
| `crates/ts-extractor/src/queries.rs` | utility | transform | None | no-analog |
| `crates/ts-extractor/src/symbols.rs` | service | transform | None | no-analog |
| `crates/ts-extractor/src/edges.rs` | service | transform | None | no-analog |
| `crates/ts-extractor/src/classify.rs` | utility | transform | `crates/core/src/detect.rs` | partial |
| `crates/ts-extractor/tests/extraction_test.rs` | test | request-response | `crates/core/tests/grammar_test.rs` | role-match |
| `crates/ts-extractor/tests/fixtures/barrel.ts` | fixture | N/A | `crates/core/tests/fixtures/sample.ts` | role-match |
| `crates/ts-extractor/tests/fixtures/hooks.ts` | fixture | N/A | `crates/core/tests/fixtures/sample.ts` | role-match |
| `crates/ts-extractor/tests/fixtures/components.tsx` | fixture | N/A | `crates/core/tests/fixtures/sample.tsx` | role-match |
| `crates/ts-extractor/tests/fixtures/schemas.ts` | fixture | N/A | `crates/core/tests/fixtures/sample.ts` | role-match |
| `crates/ts-extractor/tests/fixtures/services.ts` | fixture | N/A | `crates/core/tests/fixtures/sample.ts` | role-match |
| `crates/ts-extractor/tests/fixtures/enums.ts` | fixture | N/A | None | no-analog |
| `crates/ts-extractor/tests/fixtures/index.ts` | fixture | N/A | None | no-analog |
| `Cargo.toml` (workspace root - modification) | config | N/A | Self (current state) | exact |

## Pattern Assignments

### `crates/ts-extractor/Cargo.toml` (config)

**Analog:** `crates/core/Cargo.toml`

**Package declaration pattern** (lines 1-3):
```toml
[package]
name = "cgraph-core"
version.workspace = true
edition.workspace = true
```

**Dependency style** (lines 5-14):
```toml
[dependencies]
tree-sitter = "0.26.8"
tree-sitter-typescript = "0.23.2"
serde = { version = "1.0", features = ["derive"] }
```

**Key conventions:**
- Package name uses `cgraph-` prefix (the new crate should be `cgraph-ts-extractor`)
- Version and edition use `workspace = true` (inherit from workspace root)
- Internal dependencies use `path = "../core"` style (see `crates/cli/Cargo.toml` line 5)

---

### `crates/ts-extractor/src/lib.rs` (module-root)

**Analog:** `crates/core/src/lib.rs`

**Module declaration and re-export pattern** (lines 1-8):
```rust
pub mod model;
pub mod detect;
pub mod extractor;

// Re-export top-level types for ergonomic imports
pub use model::{Language, SymbolKind, EdgeKind, SymbolNode, SymbolEdge};
pub use extractor::{Extractor, ExtractionResult, ParseError};
pub use detect::{detect_language, scan_directory, DetectionResult};
```

**Key conventions:**
- `pub mod` declarations at top
- Re-export primary public types at crate root for ergonomic `use cgraph_ts_extractor::TsExtractor`
- Comment explaining purpose of re-exports

---

### `crates/ts-extractor/src/classify.rs` (utility, transform)

**Analog:** `crates/core/src/detect.rs`

**Pattern-matching utility function** (lines 5-15):
```rust
pub fn detect_language(path: &Path) -> Option<Language> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ts") => Some(Language::TypeScript),
        Some("tsx") => Some(Language::TypeScriptReact),
        Some("swift") => Some(Language::Swift),
        Some("go") => Some(Language::Go),
        Some("py") => Some(Language::Python),
        Some(ext) => Some(Language::Unknown(ext.to_string())),
        None => None,
    }
}
```

**Key conventions:**
- Pure functions that classify based on input patterns
- Uses `match` expressions with exhaustive pattern matching
- Return types are enums from `crate::model`
- No side effects, no I/O

---

### `crates/ts-extractor/tests/extraction_test.rs` (test, request-response)

**Analog:** `crates/core/tests/grammar_test.rs`

**Test structure and tree-sitter parser initialization** (lines 1-18):
```rust
use tree_sitter::Parser;
use tree_sitter_typescript::{LANGUAGE_TYPESCRIPT, LANGUAGE_TSX};

#[test]
fn typescript_grammar_links_and_parses() {
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE_TYPESCRIPT.into())
        .expect("Error loading TypeScript grammar");

    let source = std::fs::read_to_string("tests/fixtures/sample.ts")
        .expect("sample.ts fixture missing");
    let tree = parser.parse(&source, None).expect("parse returned None");
    let root = tree.root_node();

    assert_eq!(root.kind(), "program");
    assert!(!root.has_error(), "ERROR nodes in sample.ts");
}
```

**Key conventions:**
- `#[test]` annotation per function (no test framework beyond std)
- Fixtures loaded via `std::fs::read_to_string("tests/fixtures/...")` relative to crate root
- `.expect()` with descriptive message for setup steps
- Assertions use `assert!`, `assert_eq!` with failure messages
- Grammar loaded via `&LANGUAGE_TYPESCRIPT.into()` (the `&` and `.into()` are both required)
- TSX tests use separate `LANGUAGE_TSX`

---

### `Cargo.toml` (workspace root - modification)

**Current state** (lines 1-5):
```toml
[workspace]
members = [
    "crates/core",
    "crates/cli",
]
resolver = "2"
```

**Key conventions:**
- Members listed one per line in `members` array
- New crate `crates/ts-extractor` appended to members list
- `resolver = "2"` stays as-is

---

## Shared Patterns

### Crate Import Convention
**Source:** `crates/core/src/extractor.rs` lines 1-3, `crates/core/src/detect.rs` lines 1-2
**Apply to:** All source files in `crates/ts-extractor/src/`

```rust
// Internal crate imports use `crate::` prefix
use crate::model::{Language, SymbolNode, SymbolEdge};

// External dependencies at top
use std::path::Path;

// Core crate types via the `cgraph_core` dependency
use cgraph_core::{Extractor, ExtractionResult, ParseError, Language, SymbolNode, SymbolEdge, SymbolKind, EdgeKind};
```

### Tree-sitter Parser Initialization
**Source:** `crates/core/tests/grammar_test.rs` lines 5-9
**Apply to:** `crates/ts-extractor/src/lib.rs` (TsExtractor struct methods)

```rust
use tree_sitter::Parser;
use tree_sitter_typescript::{LANGUAGE_TYPESCRIPT, LANGUAGE_TSX};

let mut parser = Parser::new();
parser
    .set_language(&LANGUAGE_TYPESCRIPT.into())
    .expect("Error loading TypeScript grammar");
```

### Error as Data Pattern
**Source:** `crates/core/src/extractor.rs` lines 6-15
**Apply to:** All extraction logic that encounters parse errors

```rust
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Parse produced ERROR nodes in {path} at line {line}")]
    PartialParse { path: String, line: u32 },
}
```

Convention: Errors are returned as `Vec<ParseError>` inside `ExtractionResult`, not propagated via `Result`. Extraction continues even when partial parse errors occur (D-14).

### SymbolNode Construction
**Source:** `crates/core/src/model.rs` lines 31-41
**Apply to:** `crates/ts-extractor/src/symbols.rs`

```rust
pub struct SymbolNode {
    pub id: String,          // "file_path::symbol_name" (D-01)
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub language: Language,
    pub line_start: u32,
    pub line_end: u32,
    pub is_exported: bool,
}
```

Construction pattern:
```rust
SymbolNode {
    id: format!("{}::{}", file_path, name),
    name: name.to_string(),
    kind: symbol_kind,
    file_path: file_path.to_string(),
    language: Language::TypeScript, // or TypeScriptReact for .tsx
    line_start: node.start_position().row as u32 + 1,
    line_end: node.end_position().row as u32 + 1,
    is_exported: true, // only exported symbols via export_statement query
}
```

### SymbolEdge Construction
**Source:** `crates/core/src/model.rs` lines 43-49
**Apply to:** `crates/ts-extractor/src/edges.rs`

```rust
pub struct SymbolEdge {
    pub source_id: String,
    pub target_id: String,
    pub kind: EdgeKind,
    pub source_location: u32, // line number of the reference (D-04)
}
```

Construction pattern:
```rust
SymbolEdge {
    source_id: format!("{}::{}", file_path, source_symbol_name),
    target_id: format!("{}::{}", raw_import_path, target_symbol_name),
    kind: EdgeKind::Import, // or Call, TypeRef, ReExport
    source_location: node.start_position().row as u32 + 1,
}
```

### Fixture File Loading in Tests
**Source:** `crates/core/tests/grammar_test.rs` lines 11-12
**Apply to:** `crates/ts-extractor/tests/extraction_test.rs`

```rust
let source = std::fs::read_to_string("tests/fixtures/sample.ts")
    .expect("sample.ts fixture missing");
```

Convention: Test fixtures are loaded relative to the crate root. `cargo test -p cgraph-ts-extractor` runs with cwd set to `crates/ts-extractor/`.

### Extractor Trait Interface
**Source:** `crates/core/src/extractor.rs` lines 24-34
**Apply to:** `crates/ts-extractor/src/lib.rs` (impl block)

```rust
pub trait Extractor {
    /// The language this extractor handles.
    fn language(&self) -> Language;

    /// Returns true if this extractor can handle the given file path.
    fn can_handle(&self, path: &Path) -> bool;

    /// Extract graph fragments from the given file.
    /// `source` is the full text content -- file I/O is the caller's responsibility (D-18).
    fn extract(&self, path: &Path, source: &str) -> ExtractionResult;
}
```

---

## No Analog Found

Files with no close match in the codebase (planner should use RESEARCH.md patterns instead):

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/ts-extractor/src/queries.rs` | utility | transform | No tree-sitter Query compilation pattern exists in the codebase; use RESEARCH.md Pattern 1 and Pattern 2 |
| `crates/ts-extractor/src/symbols.rs` | service | transform | No symbol extraction logic exists yet; use RESEARCH.md Pattern 2 (multi-pattern symbol query) and Pattern 5 (StreamingIterator) |
| `crates/ts-extractor/src/edges.rs` | service | transform | No edge extraction logic exists yet; use RESEARCH.md Patterns 3, 4 (call/re-export queries) and Code Examples (import/type-ref queries) |
| `crates/ts-extractor/tests/fixtures/enums.ts` | fixture | N/A | First enum-focused fixture; model after existing `sample.ts` but with enum declarations |
| `crates/ts-extractor/tests/fixtures/index.ts` | fixture | N/A | Barrel file pattern; no re-export fixtures exist yet |

---

## Metadata

**Analog search scope:** `crates/core/`, `crates/cli/`, workspace root
**Files scanned:** 14 (all non-target Rust sources and configs)
**Pattern extraction date:** 2026-05-02
