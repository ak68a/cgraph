# Phase 1: Foundation - Pattern Map

**Mapped:** 2026-05-02
**Files analyzed:** 14 (all greenfield — no existing source code)
**Analogs found:** 0 / 14 (greenfield project; all patterns sourced from RESEARCH.md verified examples)

---

## File Classification

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `Cargo.toml` (workspace root) | config | — | None (greenfield) | no-analog |
| `crates/core/Cargo.toml` | config | — | None (greenfield) | no-analog |
| `crates/cli/Cargo.toml` | config | — | None (greenfield) | no-analog |
| `crates/core/src/lib.rs` | library root | — | None (greenfield) | no-analog |
| `crates/core/src/model.rs` | model | — | None (greenfield) | no-analog |
| `crates/core/src/detect.rs` | utility | transform | None (greenfield) | no-analog |
| `crates/core/src/extractor.rs` | trait definition | transform | None (greenfield) | no-analog |
| `crates/cli/src/main.rs` | CLI entry point | request-response | None (greenfield) | no-analog |
| `crates/core/tests/grammar_test.rs` | test | — | None (greenfield) | no-analog |
| `crates/core/tests/fixtures/sample.ts` | test fixture | — | None (greenfield) | no-analog |
| `crates/core/tests/fixtures/sample.tsx` | test fixture | — | None (greenfield) | no-analog |
| `crates/core/tests/fixtures/sample.swift` | test fixture | — | None (greenfield) | no-analog |
| `crates/core/tests/fixtures/sample.go` | test fixture | — | None (greenfield) | no-analog |
| `crates/core/tests/fixtures/sample.py` | test fixture | — | None (greenfield) | no-analog |

> Note: A workspace-level CLI smoke test (`tests/cli_smoke.rs` or `crates/cli/tests/cli_smoke.rs`) is also needed per D-23. Location left to planner discretion — same subprocess pattern as grammar_test applies.

---

## Pattern Assignments

All patterns below are sourced from RESEARCH.md verified examples. Line references are into RESEARCH.md at `/Users/vayu/Dev/Projects/cgraph/.planning/phases/01-foundation/01-RESEARCH.md`.

---

### `Cargo.toml` (workspace root) — config

**Source pattern:** RESEARCH.md lines 328–343 (Pattern 4: Cargo Workspace Root)

**Workspace manifest pattern:**
```toml
[workspace]
members = [
    "crates/core",
    "crates/cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["cgraph contributors"]
```

**Key rules:**
- `resolver = "2"` is mandatory with edition 2024 (prevents feature unification conflicts in workspace).
- Use `[workspace.package]` so member crates can inherit `version.workspace = true` and `edition.workspace = true`.
- Do NOT use `resolver = "1"` or omit the resolver key — this is a breaking anti-pattern documented in RESEARCH.md.

---

### `crates/core/Cargo.toml` — config

**Source pattern:** RESEARCH.md lines 489–506 (Code Examples: Cargo workspace member with inherited version)

**Core crate manifest pattern:**
```toml
[package]
name = "cgraph-core"
version.workspace = true
edition.workspace = true

[dependencies]
tree-sitter = "0.26.8"
tree-sitter-typescript = "0.23.2"
tree-sitter-swift = "0.7.1"
tree-sitter-go = "0.25.0"
tree-sitter-python = "0.25.0"
walkdir = "2.5.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
```

**Critical version pins (RESEARCH.md lines 377–386 — Pitfall 1):**
- All four grammar crate versions must be pinned exactly as listed above. These were verified against crates.io for ABI compatibility with `tree-sitter = "0.26.8"`.
- `tree-sitter-typescript = "0.23.2"` — requires tree-sitter ^0.24, satisfied by 0.26.8.
- `tree-sitter-swift = "0.7.1"` — requires tree-sitter ^0.23, satisfied.
- `tree-sitter-go = "0.25.0"` and `tree-sitter-python = "0.25.0"` — require tree-sitter ^0.25.8, satisfied.
- Do NOT use `*` or wide semver ranges for grammar crates.

---

### `crates/cli/Cargo.toml` — config

**Source pattern:** RESEARCH.md lines 155–161 (Standard Stack) + lines 419–433 (Pitfall 4: binary name)

**CLI crate manifest pattern:**
```toml
[package]
name = "cg"
version.workspace = true
edition.workspace = true

[dependencies]
cgraph-core = { path = "../core" }
clap = { version = "4.6.1", features = ["derive", "cargo"] }
anyhow = "1.0"
```

**Key rules:**
- Package `name = "cg"` makes Cargo emit the binary as `cg` (not `cgraph-cli`). See Pitfall 4 in RESEARCH.md.
- The `cargo` feature in clap enables `#[command(version)]` to read the version from `Cargo.toml` at compile time — no manual version string.
- `anyhow` is for `main()` error propagation only. Library crates (`cgraph-core`) use `thiserror` instead.
- Do NOT add `tree-sitter` as a direct dependency of the CLI crate. Grammar linkage belongs in `crates/core`.

---

### `crates/core/src/lib.rs` — library root

**Source pattern:** RESEARCH.md lines 211–213 (Recommended Project Structure, lib.rs comment)

**Library root pattern:**
```rust
pub mod model;
pub mod detect;
pub mod extractor;
```

**Key rules:**
- Re-export top-level types at crate root as needed so consumers can `use cgraph_core::SymbolNode` without deep paths.
- No logic in `lib.rs` — only `pub mod` declarations and optional `pub use` re-exports.

---

### `crates/core/src/model.rs` — model

**Source pattern:** RESEARCH.md lines 508–550 (Code Examples: Data model struct)

**Full data model pattern:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    TypeScript,
    TypeScriptReact,  // .tsx
    Swift,
    Go,
    Python,
    Unknown(String),  // extension seen but not parseable
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Class,
    Type,
    Interface,
    Hook,
    Enum,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    Import,
    Call,
    TypeRef,
    ReExport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEdge {
    pub source_id: String,
    pub target_id: String,
    pub kind: EdgeKind,
    pub source_location: u32,  // line number of the reference (D-04)
}
```

**Key rules:**
- All structs and enums get `#[derive(Debug, Clone, Serialize, Deserialize)]` — required for later phases (D-05 says deferred fields are added later, not that serde is deferred).
- Enums additionally derive `PartialEq, Eq, Hash` to allow use as HashMap keys.
- Do NOT add `docstring`, `signature`, or `byte_range` fields — deferred to the phase that needs them (D-05).
- `Language::Unknown(String)` carries the raw extension string for the scan summary output (D-10, D-11).

---

### `crates/core/src/detect.rs` — utility, transform

**Source pattern:** RESEARCH.md lines 289–320 (Pattern 3: Language Detection from File Extension) + lines 443–453 (Pitfall 5: node_modules filter)

**Language detection function pattern:**
```rust
use std::path::Path;
use crate::model::Language;

pub fn detect_language(path: &Path) -> Option<Language> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ts") => Some(Language::TypeScript),
        Some("tsx") => Some(Language::TypeScriptReact),
        Some("swift") => Some(Language::Swift),
        Some("go") => Some(Language::Go),
        Some("py") => Some(Language::Python),
        Some(ext) => Some(Language::Unknown(ext.to_string())),
        None => None,  // no extension = skip
    }
}
```

**Directory walker pattern (skip noise directories):**
```rust
// Minimum filter — skip hidden dirs, node_modules, dist, build
if entry.file_type().is_dir() {
    let name = entry.file_name().to_string_lossy();
    if name.starts_with('.') || name == "node_modules" || name == "dist" || name == "build" {
        it.skip_current_dir();
    }
}
```

**DetectionResult struct (summary output support per D-11):**
```rust
#[derive(Debug, Default)]
pub struct DetectionResult {
    pub detected: Vec<(PathBuf, Language)>,    // all files with a known extension
    pub parseable: Vec<(PathBuf, Language)>,   // files whose language has an extractor
    pub skipped: Vec<(PathBuf, String)>,       // files with Unknown extension
}
```

**Key rules:**
- Use `entry.file_name().to_string_lossy()` for directory name comparison — handles non-UTF-8 filenames without panic (RESEARCH.md security section, line 677).
- `walkdir` default `follow_links = false` — do not enable symlink following (security: prevents escaping scanned tree).
- `detect_language` is pure (no I/O) — it takes a `&Path` and returns `Option<Language>`. The directory walk lives in the CLI crate or a separate `scan` function in this module.

---

### `crates/core/src/extractor.rs` — trait definition, transform

**Source pattern:** RESEARCH.md lines 43–45 (D-16, D-17, D-18 decisions) + thiserror derive pattern from Standard Stack

**Extractor trait and result types pattern:**
```rust
use std::path::Path;
use thiserror::Error;
use crate::model::{Language, SymbolNode, SymbolEdge};

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

#[derive(Debug)]
pub struct ExtractionResult {
    pub nodes: Vec<SymbolNode>,
    pub edges: Vec<SymbolEdge>,
    pub errors: Vec<ParseError>,  // errors are data, not panics (D-17)
}

pub trait Extractor {
    /// The language this extractor handles.
    fn language(&self) -> Language;

    /// Returns true if this extractor can handle the given file path.
    fn can_handle(&self, path: &Path) -> bool;

    /// Extract graph fragments from the given file.
    /// `source` is the full text content — file I/O is the caller's responsibility (D-18).
    fn extract(&self, path: &Path, source: &str) -> ExtractionResult;
}
```

**Key rules:**
- `thiserror::Error` derive for `ParseError` — zero boilerplate, produces correct `std::error::Error` impl.
- `ExtractionResult` uses owned `Vec<_>` fields (D-17). No `Result` wrapper — errors are data inside the struct.
- `extract()` takes `&str` (already-read source), NOT a `Path` for I/O. File reading belongs to the indexer (D-18).
- Extractor crates will implement this trait with `impl Extractor for TypeScriptExtractor` — they only depend on `cgraph-core`.

---

### `crates/cli/src/main.rs` — CLI entry point, request-response

**Source pattern:** RESEARCH.md lines 229–256 (Pattern 1: clap Derive CLI) + lines 171–199 (Architecture Diagram)

**Full CLI entry point pattern:**
```rust
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "Code graph visualization — cgraph")]
pub struct Cli {
    /// Path to the project directory to scan
    pub path: PathBuf,

    /// Print verbose output including per-file language detection
    #[arg(short, long)]
    pub verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Validate path exists and is a directory
    if !cli.path.exists() {
        anyhow::bail!("Path does not exist: {}", cli.path.display());
    }
    if !cli.path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", cli.path.display());
    }

    // Call core detection
    let result = cgraph_core::detect::scan_directory(&cli.path)?;

    // Print scan summary to stdout (D-19)
    // Errors go to stderr (D-15)
    println!("Detected {} files", result.detected.len());

    Ok(())
}
```

**Key rules:**
- `#[command(version)]` with the `cargo` feature reads version from `Cargo.toml` at compile time — no manual version string needed (RESEARCH.md lines 106–107).
- `fn main() -> Result<()>` with `anyhow::Result` is the idiomatic binary error propagation pattern. Libraries use `thiserror`; binaries use `anyhow`.
- Validate path before walking (security: RESEARCH.md lines 672–680).
- Scan summary to `stdout`; per-file errors to `stderr` (D-15).
- Do NOT import `tree-sitter` in this crate. All grammar linkage is in `crates/core`.

---

### `crates/core/tests/grammar_test.rs` — integration test

**Source pattern:** RESEARCH.md lines 461–486 (Code Examples: Full tree-sitter parse verification) + lines 258–287 (Pattern 2: tree-sitter Grammar Linkage)

**Grammar linkage test pattern — copy this exactly:**
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

#[test]
fn tsx_grammar_links_and_parses() {
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE_TSX.into())
        .expect("Error loading TSX grammar");

    let source = std::fs::read_to_string("tests/fixtures/sample.tsx")
        .expect("sample.tsx fixture missing");
    let tree = parser.parse(&source, None).expect("parse returned None");
    assert!(!tree.root_node().has_error(), "ERROR nodes in sample.tsx");
}

#[test]
fn swift_grammar_links() {
    use tree_sitter_swift::LANGUAGE;
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE.into())
        .expect("Error loading Swift grammar");
    // Phase 1: just verify linkage, not parse correctness
}

#[test]
fn go_grammar_links() {
    use tree_sitter_go::LANGUAGE;
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE.into())
        .expect("Error loading Go grammar");
}

#[test]
fn python_grammar_links() {
    use tree_sitter_python::LANGUAGE;
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE.into())
        .expect("Error loading Python grammar");
}
```

**Critical anti-pattern to avoid (RESEARCH.md lines 402–414 — Pitfall 3):**
```rust
// WRONG — tree-sitter-typescript does NOT export `LANGUAGE`
use tree_sitter_typescript::LANGUAGE;

// CORRECT — two dialects, two constants
use tree_sitter_typescript::{LANGUAGE_TYPESCRIPT, LANGUAGE_TSX};
```

**Note on `StreamingIterator` (RESEARCH.md lines 348–350):** When using tree-sitter queries (Phase 2+), use `use tree_sitter::StreamingIterator` — do NOT add `streaming-iterator` as a direct dependency. Not needed in Phase 1 tests.

---

### Test Fixtures — `crates/core/tests/fixtures/`

These are minimal valid source files used by `grammar_test.rs`. The content only needs to be syntactically valid for the grammar to parse without ERROR nodes.

**`sample.ts`** — minimal valid TypeScript:
```typescript
export function add(a: number, b: number): number {
    return a + b;
}

interface User {
    id: string;
    name: string;
}
```

**`sample.tsx`** — minimal valid TSX:
```typescript
import React from 'react';

const App = (): React.ReactElement => {
    return <div>Hello, cgraph</div>;
};

export default App;
```

**`sample.swift`** — minimal valid Swift:
```swift
import Foundation

func greet(name: String) -> String {
    return "Hello, \(name)"
}

struct Point {
    var x: Double
    var y: Double
}
```

**`sample.go`** — minimal valid Go:
```go
package main

import "fmt"

func main() {
    fmt.Println("cgraph")
}

type Node struct {
    ID   string
    Name string
}
```

**`sample.py`** — minimal valid Python:
```python
def add(a: int, b: int) -> int:
    return a + b

class Node:
    def __init__(self, id: str, name: str):
        self.id = id
        self.name = name
```

---

## Shared Patterns

### Error Handling: `thiserror` in libraries, `anyhow` in binary

**Apply to:** `crates/core/src/extractor.rs` (thiserror), `crates/cli/src/main.rs` (anyhow)

**Source:** RESEARCH.md lines 117–119 (Standard Stack — Supporting)

```rust
// In library crates (cgraph-core): typed errors with thiserror
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("description: {0}")]
    Variant(String),
}

// In binary crate (crates/cli): anyhow for ergonomic propagation
use anyhow::{Result, bail};

fn main() -> Result<()> {
    // bail! produces a user-facing error message
    bail!("something went wrong: {}", detail);
}
```

**Rule:** Never use `anyhow` in `crates/core`. Never use `thiserror` in `crates/cli/src/main.rs` for top-level error handling. The boundary is library vs. binary.

---

### Serde Derives on All Data Model Types

**Apply to:** All structs and enums in `crates/core/src/model.rs`

**Source:** RESEARCH.md lines 110–111 + lines 511–550

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SomeEnum { ... }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SomeStruct { ... }
```

**Rule:** Add serde derives in Phase 1 even though JSON output is not used until later phases. Adding serde mid-project to `core` requires a rebuild of all dependent crates and is a minor disruption (RESEARCH.md Assumption A3).

---

### Path Safety: `to_string_lossy()` for Filename Comparison

**Apply to:** `crates/core/src/detect.rs` (directory walker), `crates/cli/src/main.rs`

**Source:** RESEARCH.md lines 677–679 (Security Domain)

```rust
// CORRECT — handles non-UTF-8 filenames without panic
let name = entry.file_name().to_string_lossy();
if name.starts_with('.') || name == "node_modules" { ... }

// WRONG — panics on non-UTF-8 filenames
let name = entry.file_name().to_str().unwrap();
```

---

### Grammar Version Matrix (Pin These Exactly)

**Apply to:** `crates/core/Cargo.toml`

**Source:** RESEARCH.md lines 377–386 (Pitfall 1)

```toml
tree-sitter = "0.26.8"
tree-sitter-typescript = "0.23.2"
tree-sitter-swift = "0.7.1"
tree-sitter-go = "0.25.0"
tree-sitter-python = "0.25.0"
```

These exact versions are verified to compile together on this machine. Any change requires re-verifying ABI compatibility.

---

## No Analog Found

This is a greenfield Rust project. There is no existing source code in the repository. Every file in this phase is novel.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| All 14 files | various | various | Greenfield project — no prior Rust source exists in `/Users/vayu/Dev/Projects/cgraph/` |

All patterns are sourced from RESEARCH.md verified examples and the locked decisions in CONTEXT.md.

---

## Metadata

**Analog search scope:** Full repository (confirmed no `.rs` or `.toml` files outside `.planning/`)
**Files scanned:** 0 source files (greenfield)
**Pattern extraction date:** 2026-05-02
**Pattern source:** `/Users/vayu/Dev/Projects/cgraph/.planning/phases/01-foundation/01-RESEARCH.md`
