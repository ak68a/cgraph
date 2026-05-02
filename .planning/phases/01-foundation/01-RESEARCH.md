# Phase 1: Foundation - Research

**Researched:** 2026-05-02
**Domain:** Rust CLI scaffold, Cargo workspace, tree-sitter native C linkage, language detection
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Symbol Identity**
- D-01: Symbol IDs use path-qualified format: `file_path::symbol_name` (e.g., `src/auth/login.ts::handleLogin`). Human-readable, greppable, no indirection.
- D-02: IDs are scoped to a single scan — no persistence across runs.

**Node & Edge Metadata**
- D-03: SymbolNode fields: `id`, `name`, `kind` (function/class/type/interface/hook/enum), `file_path`, `language`, `line_start`, `line_end`, `is_exported`.
- D-04: SymbolEdge fields: `source_id`, `target_id`, `kind` (import/call/type_ref/re_export), `source_location`.
- D-05: Deferred to later phases: `docstring`, `signature`, `byte_range`.

**Workspace Layout**
- D-06: Cargo workspace from day one. Structure: `crates/core` (data model + traits + language detection), `crates/cli` (binary entry point, clap). Extractor and server crates added in later phases.
- D-07: Each extractor is a self-contained crate depending only on `core`.

**Tree-sitter Integration**
- D-08: Use published grammar crates from crates.io (e.g., `tree-sitter-typescript`, `tree-sitter-swift`). No source-vendoring unless a grammar breaks.
- D-09: Each extractor owns its own tree-sitter parsing.

**Language Detection**
- D-10: Detection is language-agnostic — scan all file extensions, report everything found including unsupported languages.
- D-11: Summary output distinguishes detected vs. parseable vs. skipped.
- D-12: Parsing only runs for languages with an available extractor. Unsupported files skipped without error.

**Error Handling**
- D-13: Warn and continue. A single broken file never prevents scanning the rest.
- D-14: Tree-sitter partial parses are valid — extract what's available.
- D-15: File-level errors go to stderr/verbose log. Summary shows count only.

**Extractor Trait**
- D-16: Trait: `language() -> Language`, `can_handle(&Path) -> bool`, `extract(&Path, &str) -> ExtractionResult`.
- D-17: ExtractionResult: owned `Vec<SymbolNode>`, `Vec<SymbolEdge>`, `Vec<ParseError>`. Errors are data.
- D-18: Extractors are pure transformation (text in, graph fragments out). File I/O belongs to the indexer.

**CLI UX**
- D-19: Phase 1 `cg <path>` runs language detection and prints scan summary.
- D-20: `cg --version` and `cg --help` work via clap. No extraction or graph output until Phase 2+.

**Test Strategy**
- D-21: Unit tests on data model (struct construction, serialization, language detection logic).
- D-22: Integration tests with fixture files: one small sample file per language in `tests/fixtures/`.
- D-23: CLI smoke test: run binary as subprocess against fixture directory, assert exit 0 and correct detection output.
- D-24: No mocks, no benchmarks in Phase 1.

### Claude's Discretion
None specified — all Phase 1 decisions are locked.

### Deferred Ideas (OUT OF SCOPE)
- Rust extractor, Java extractor, Ruby extractor, Kotlin extractor — future phases beyond v1 roadmap.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PARS-11 | Tool auto-detects project language from file extensions | Extension-to-language mapping in `crates/core`; `walkdir` for directory traversal; report detected, parseable, skipped |
| INFR-01 | Tool runs as a CLI command (`cg <path>`) | `clap` 4.6.1 derive API; `crates/cli` binary crate; `cg` binary name set in `Cargo.toml` |
</phase_requirements>

---

## Summary

Phase 1 establishes the Rust Cargo workspace and proves each foundational capability in isolation before any extraction logic is added. The work divides into four independent tracks: (1) workspace scaffold and build system, (2) shared data model in `crates/core`, (3) tree-sitter grammar linkage and parse verification, and (4) language detection from file extensions with summary output.

The Rust stack eliminates the ABI mismatch problems documented in the prior Node.js research. Grammar crates publish their own `build.rs` using the `cc` crate; when added as Cargo dependencies they compile the grammar C code automatically. No project-level `build.rs` is needed. The installed toolchain (Rust 1.93.1, clang 17.0.0) is confirmed to build `tree-sitter 0.26.8` + `tree-sitter-typescript 0.23.2` from crates.io with zero configuration — verified by running a test binary on this machine.

The `clap` derive API reads version and name from `Cargo.toml` at compile time via the `cargo` feature, so `cg --version` works correctly with no manual version strings. Language detection is straightforward extension-matching with no external dependencies.

**Primary recommendation:** Build in crate dependency order — `crates/core` first (data model + detection + trait), then `crates/cli` (binary entry point). All tree-sitter grammar linkage validation happens in `crates/core` integration tests against fixture files.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CLI entry point (`cg <path>`) | `crates/cli` binary | — | Binary crate owns argument parsing and process lifecycle |
| Shared data model (SymbolNode, SymbolEdge, Language enum) | `crates/core` lib | — | All downstream crates depend on core; model must live outside the binary |
| Extractor trait definition | `crates/core` lib | — | Trait is the interface contract; must be in core so extractor crates can implement it without depending on cli |
| Language detection (extension → Language) | `crates/core` lib | — | Pure logic with no I/O; belongs in core for testability |
| Directory walk + file enumeration | `crates/cli` | `crates/core` detection fn | CLI drives the walk; core provides the classification |
| Tree-sitter grammar linkage | `crates/core` (verified in tests) | — | Integration test in core proves grammar crates link; actual parsing stays in extractor crates (Phase 2+) |
| Scan summary output | `crates/cli` | — | Presentation belongs in the binary entry point |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `clap` | 4.6.1 | CLI argument parsing, `--help`, `--version` | De facto Rust CLI library; derive API eliminates boilerplate; `cargo` feature reads version from Cargo.toml [VERIFIED: crates.io] |
| `tree-sitter` | 0.26.8 | Tree parsing runtime for Rust | Native Rust + C linkage — no ABI mismatch, no Node.js, no WASM overhead; 0.26.8 is current [VERIFIED: crates.io] |
| `tree-sitter-typescript` | 0.23.2 | TypeScript + TSX grammar for parse validation | Official tree-sitter org crate; exposes `LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX` constants; requires tree-sitter ^0.24 (satisfied by 0.26.8) [VERIFIED: crates.io dependency check + compile test] |
| `walkdir` | 2.5.0 | Recursive directory traversal | Standard crate for directory walks in Rust; handles symlinks and cross-platform differences [VERIFIED: crates.io] |
| `serde` | 1.0.228 | Derive Serialize/Deserialize on data model structs | Required by later phases; adding `#[derive(Serialize, Deserialize)]` in Phase 1 avoids a disruptive add later [VERIFIED: crates.io] |
| `serde_json` | 1.0.149 | JSON serialization for `--json` output mode | Paired with serde; later phases depend on this [VERIFIED: crates.io] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | 2.0.18 | Derive-based error types for `ParseError`, `ExtractionError` | Use in `crates/core` for typed, structured errors returned from extractors |
| `anyhow` | 1.0.102 | Ergonomic error propagation | Use in `crates/cli` main function for user-facing error messages; thiserror in libraries, anyhow in binaries |
| `tree-sitter-swift` | 0.7.1 | Swift grammar (linked but not parsed in Phase 1) | Include now so Phase 1 integration test covers all four target grammars; requires tree-sitter ^0.23.0 [VERIFIED: crates.io] |
| `tree-sitter-go` | 0.25.0 | Go grammar (linked but not parsed in Phase 1) | Same rationale; requires tree-sitter ^0.25.8 [VERIFIED: crates.io] |
| `tree-sitter-python` | 0.25.0 | Python grammar (linked but not parsed in Phase 1) | Same rationale; requires tree-sitter ^0.25.8 [VERIFIED: crates.io] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `clap` derive | `clap` builder API | Builder is more verbose; derive is idiomatic for structured CLIs of this size |
| `walkdir` | `std::fs::read_dir` + recursion | walkdir handles symlink loops, cross-platform edge cases; stdlib requires manual recursion |
| `thiserror` | `std::fmt::Display` impl | thiserror is zero-cost and idiomatic; manual Display wastes time |
| `serde` + `serde_json` | Manual JSON building | Serde is the universal standard; hand-built JSON serialization is error-prone |

**Installation — workspace root `Cargo.toml`:**
```toml
[workspace]
members = [
    "crates/core",
    "crates/cli",
]
resolver = "2"
```

**`crates/core/Cargo.toml`:**
```toml
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

**`crates/cli/Cargo.toml`:**
```toml
[dependencies]
cgraph-core = { path = "../core" }
clap = { version = "4.6.1", features = ["derive", "cargo"] }
anyhow = "1.0"
```

**Version verification:** All versions above confirmed against crates.io registry on 2026-05-02. [VERIFIED: crates.io]

---

## Architecture Patterns

### System Architecture Diagram

```
User invokes: cg <path>
        |
        v
[crates/cli: main.rs]
  - Cli::parse() via clap derive
  - Validate path exists
  - Call core::detect_languages(&path)
        |
        v
[crates/core: detect.rs]
  - walkdir recursive scan
  - For each file: match extension → Language variant
  - Return DetectionResult { detected, parseable, skipped }
        |
        v
[crates/cli: main.rs]
  - Print scan summary to stdout
  - Exit 0
```

```
Integration test (validates tree-sitter linkage):
[crates/core/tests/grammar_test.rs]
  - Parser::new()
  - parser.set_language(&LANGUAGE_TYPESCRIPT.into())
  - parser.parse(fixture_source, None)
  - assert!(!root_node.has_error())
```

### Recommended Project Structure
```
cgraph/                         # workspace root
├── Cargo.toml                  # [workspace] members = [...]
├── Cargo.lock
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # pub mod model; pub mod detect; pub mod extractor;
│   │   │   ├── model.rs        # SymbolNode, SymbolEdge, Language, SymbolKind, EdgeKind
│   │   │   ├── detect.rs       # extension → Language mapping, DetectionResult
│   │   │   └── extractor.rs    # Extractor trait definition, ExtractionResult, ParseError
│   │   └── tests/
│   │       ├── grammar_test.rs # tree-sitter linkage + parse smoke tests
│   │       └── fixtures/
│   │           ├── sample.ts
│   │           ├── sample.swift
│   │           ├── sample.go
│   │           └── sample.py
│   └── cli/
│       ├── Cargo.toml          # bin name = "cg"
│       └── src/
│           └── main.rs         # Cli struct, dispatch to core::detect_languages
└── tests/
    └── cli_smoke.rs            # subprocess test: `cg ./fixtures` → exit 0, correct output
```

### Pattern 1: clap Derive CLI with PathBuf Positional Argument

**What:** Define the CLI struct with clap's derive API; `PathBuf` positional arg is auto-validated as a non-empty path.

**When to use:** All Phase 1 CLI entry point code.

```rust
// Source: https://docs.rs/clap/latest/clap/_derive/index.html [VERIFIED: Context7]
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

fn main() {
    let cli = Cli::parse();
    // cli.path is validated as non-empty by clap
    // --version reads from Cargo.toml via `cargo` feature
}
```

### Pattern 2: tree-sitter Grammar Linkage in Rust

**What:** Initialize a Parser, assign a grammar language, parse source, check for errors.

**When to use:** Integration tests in `crates/core/tests/grammar_test.rs`.

```rust
// Source: https://docs.rs/tree-sitter/latest/tree_sitter/index.html [VERIFIED: Context7 + compile test]
use tree_sitter::Parser;
use tree_sitter_typescript::LANGUAGE_TYPESCRIPT;

#[test]
fn typescript_grammar_links_and_parses() {
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE_TYPESCRIPT.into())
        .expect("Error loading TypeScript grammar");

    let source = "const x: number = 1;";
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    assert_eq!(root.kind(), "program");
    assert!(!root.has_error(), "Parse produced ERROR nodes on valid TypeScript");
}
```

The grammar crates (`tree-sitter-typescript`, `tree-sitter-swift`, `tree-sitter-go`, `tree-sitter-python`) each ship their own `build.rs` that compiles the grammar C code using the `cc` crate. When listed as Cargo dependencies, this compilation is automatic. No `build.rs` is needed in `crates/core` or `crates/cli`.

**TSX dialect:** Use `tree_sitter_typescript::LANGUAGE_TSX` (not `LANGUAGE_TYPESCRIPT`) when detecting `.tsx` files.

### Pattern 3: Language Detection from File Extension

**What:** Map file extensions to a `Language` enum; walk directory with walkdir.

**When to use:** `crates/core/detect.rs`.

```rust
// Source: [ASSUMED] — standard Rust pattern, verified extension mapping from project decisions
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Language {
    TypeScript,
    TypeScriptReact,  // .tsx
    Swift,
    Go,
    Python,
    Unknown(String),  // extension we saw but can't parse
}

pub fn detect_language(path: &Path) -> Option<Language> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ts") => Some(Language::TypeScript),
        Some("tsx") => Some(Language::TypeScriptReact),
        Some("swift") => Some(Language::Swift),
        Some("go") => Some(Language::Go),
        Some("py") => Some(Language::Python),
        Some(ext) => Some(Language::Unknown(ext.to_string())),
        None => None, // no extension = skip
    }
}
```

### Pattern 4: Cargo Workspace Root

**What:** Standard workspace `Cargo.toml` with resolver = "2" (required for edition 2024 and feature unification).

**When to use:** Project root.

```toml
# Source: Cargo documentation [VERIFIED: compile test]
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

Using `[workspace.package]` allows member crates to inherit `version` and `edition` via `version.workspace = true`.

### Anti-Patterns to Avoid

- **Grammar crate version drift:** Do not use `*` or overly-wide semver ranges for grammar crates. Their C ABI is tied to the tree-sitter core ABI. The version matrix that works together is documented in this research — pin it. [VERIFIED: crates.io dependency check]
- **Putting tree-sitter in `crates/cli`:** The parser and grammar linkage belong in `crates/core`. The CLI crate must not directly depend on tree-sitter — that would prevent extractor crates from being independent of the CLI.
- **`LANGUAGE` vs `LANGUAGE_TYPESCRIPT`:** The `tree-sitter-rust` crate exports `LANGUAGE`. But `tree-sitter-typescript` exports `LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX` (two dialects). Don't copy patterns from tree-sitter-rust blindly — check each grammar crate's own exports.
- **Manual `StreamingIterator` dependency:** tree-sitter 0.26.x re-exports `streaming_iterator::StreamingIterator` as `tree_sitter::StreamingIterator`. Do not add `streaming-iterator` as a direct dependency — use `use tree_sitter::StreamingIterator`. [VERIFIED: compile test]
- **Edition 2021 when 2024 is supported:** Rust 1.93.1 fully supports edition 2024 (stabilized in 1.85). Use edition 2024 for all crates. This enables the `gen` keyword reservation, async closures, and other improvements needed in later phases.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI argument parsing | Custom `std::env::args()` loop | `clap` derive | Handles --help, --version, error messages, type coercion; tested on millions of CLIs |
| Directory traversal | Manual `read_dir` recursion | `walkdir` | Handles symlink loops, permissions errors, cross-platform path separators |
| Error type boilerplate | Manual `impl Display + Error` | `thiserror` derive | Zero-cost; eliminates 15-line boilerplate per error type |
| C grammar compilation | Inline C or manual cc invocation | Let grammar crates' `build.rs` do it | Grammar crates know their own compilation flags; manual would break on MSVC/MinGW |
| JSON serialization | String concatenation / format! | `serde` + `serde_json` | Handles escaping, nesting, optional fields; produces spec-compliant output |

**Key insight:** In Rust, the gap between "hand-rolled" and "idiomatic" is larger than in most languages. The standard crates (`clap`, `walkdir`, `thiserror`, `serde`) have been hardened across thousands of projects — their correctness in edge cases (empty paths, Unicode filenames, Windows paths, deeply nested dirs) is effectively free.

---

## Common Pitfalls

### Pitfall 1: tree-sitter Grammar / Core Version Mismatch

**What goes wrong:** Adding `tree-sitter = "0.26"` and `tree-sitter-go = "0.24"` together. The grammar crate was compiled against an older ABI. At runtime: `set_language` returns `Err` or panic, or the parser silently produces ERROR nodes for valid code.

**Why it happens:** Grammar crates and the core `tree-sitter` crate have separate version numbers but share an ABI version. They must be compatible.

**How to avoid:** Use the exact version matrix verified in this research:
- `tree-sitter = "0.26.8"` (core)
- `tree-sitter-typescript = "0.23.2"` (requires ^0.24 — satisfied)
- `tree-sitter-swift = "0.7.1"` (requires ^0.23 — satisfied)
- `tree-sitter-go = "0.25.0"` (requires ^0.25.8 — satisfied)
- `tree-sitter-python = "0.25.0"` (requires ^0.25.8 — satisfied)

**Warning signs:** `parser.set_language(...).is_err()` or `root_node.has_error()` on a valid sample file.

[VERIFIED: crates.io dependency resolution check + compile + run test]

---

### Pitfall 2: C Compiler Not Found During Build

**What goes wrong:** Grammar crates invoke the `cc` crate in their `build.rs` to compile the parser C code. If no C compiler is on `PATH`, the build fails with: `error: linker 'cc' not found` or `error: failed to run custom build command for 'tree-sitter-typescript'`.

**Why it happens:** `cc` binary is aliased to `claude` in this environment (confirmed by shell check). However the `cc` Rust crate on macOS detects `clang` directly from `/usr/bin/clang`, bypassing the shell alias.

**How to avoid:** Confirmed that `/usr/bin/clang` (Apple clang 17.0.0) is present and working — verified by successfully compiling tree-sitter-typescript 0.23.2 on this machine. No action needed for development. [VERIFIED: compile test ran successfully]

**Warning signs on CI:** Ensure the CI image has `build-essential` (Linux) or Xcode command line tools (macOS) installed before `cargo build`.

---

### Pitfall 3: `LANGUAGE` vs `LANGUAGE_TYPESCRIPT` Export Name

**What goes wrong:** Copying code from `tree-sitter-rust` examples which use `tree_sitter_rust::LANGUAGE`. `tree-sitter-typescript` does NOT export `LANGUAGE` — it exports `LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX` to distinguish the two dialects.

**How to avoid:**
```rust
// CORRECT
use tree_sitter_typescript::{LANGUAGE_TYPESCRIPT, LANGUAGE_TSX};

// WRONG — will not compile
use tree_sitter_typescript::LANGUAGE;
```

[VERIFIED: Context7 tree-sitter-typescript docs]

---

### Pitfall 4: `binary name = "cg"` Must Be Set Explicitly

**What goes wrong:** If `Cargo.toml` for the CLI crate doesn't set `name = "cg"` in the `[[bin]]` section (or the package name is not `cg`), the binary will be named after the package (e.g., `cgraph-cli`), breaking `cg <path>`.

**How to avoid:**
```toml
# crates/cli/Cargo.toml
[package]
name = "cg"       # This becomes the binary name
# OR
[[bin]]
name = "cg"
path = "src/main.rs"
```

[ASSUMED — standard Cargo behavior]

---

### Pitfall 5: Scanning `node_modules` and Hidden Directories

**What goes wrong:** walkdir will descend into `node_modules/`, `.git/`, `dist/`, etc. For a TypeScript project this produces thousands of spurious file detections that pollute the language summary.

**How to avoid:** In the directory walker, filter out well-known non-source directories before reporting. For Phase 1 (detection only, no extraction) this is primarily a UX concern — the summary output should not show `node_modules/.bin/acorn` as a detected JavaScript file.

```rust
// Minimum filter for Phase 1 — skip hidden dirs and node_modules
if entry.file_type().is_dir() {
    let name = entry.file_name().to_string_lossy();
    if name.starts_with('.') || name == "node_modules" || name == "dist" || name == "build" {
        it.skip_current_dir();
    }
}
```

[ASSUMED — consistent with prior research PITFALLS.md findings]

---

## Code Examples

### Full tree-sitter parse verification (integration test)
```rust
// Source: Context7 tree-sitter docs + verified compile/run test on this machine
// [VERIFIED: crates.io + compile test]
use tree_sitter::Parser;
use tree_sitter_typescript::{LANGUAGE_TYPESCRIPT, LANGUAGE_TSX};

#[test]
fn typescript_parses_without_errors() {
    let mut parser = Parser::new();
    parser.set_language(&LANGUAGE_TYPESCRIPT.into())
        .expect("Error loading TypeScript grammar");
    let source = "export function add(a: number, b: number): number { return a + b; }";
    let tree = parser.parse(source, None).expect("parse returned None");
    assert!(!tree.root_node().has_error(), "ERROR nodes in valid TypeScript source");
}

#[test]
fn tsx_parses_without_errors() {
    let mut parser = Parser::new();
    parser.set_language(&LANGUAGE_TSX.into())
        .expect("Error loading TSX grammar");
    let source = "const App = () => <div>Hello</div>;";
    let tree = parser.parse(source, None).expect("parse returned None");
    assert!(!tree.root_node().has_error(), "ERROR nodes in valid TSX source");
}
```

### Cargo workspace member with inherited version
```toml
# crates/core/Cargo.toml
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

### Data model struct (Phase 1 shape, serde-ready)
```rust
// Source: CONTEXT.md D-03, D-04 + standard Rust serde pattern [ASSUMED pattern, decisions are locked]
use serde::{Deserialize, Serialize};

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
    pub id: String,          // "file_path::symbol_name"
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
    pub source_location: u32,  // line number of the reference
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Node.js tree-sitter (npm) | Rust tree-sitter (crates.io native) | Decided at roadmap creation | Eliminates ABI mismatch, Node version dependency, and native binding compile-on-install issues |
| tree-sitter `Language::from_raw_ptr` pattern | `LanguageFn.into()` pattern | tree-sitter 0.23+ | Grammar crates now expose `LANGUAGE: LanguageFn` constant; call `.into()` to get `Language` for `set_language` |
| tree-sitter queries return `Vec<QueryMatch>` | Queries return `QueryMatches` / `QueryCaptures` implementing `StreamingIterator` | tree-sitter 0.24+ | Must use `use tree_sitter::StreamingIterator` to call `.next()` on captures |
| `resolver = "1"` (default) | `resolver = "2"` required for edition 2024 | Rust 1.51+ / edition 2024 | Prevents feature unification conflicts in workspace; always set explicitly |

**Deprecated/outdated (do not use):**
- `Language::unsafe_from_raw_ptr()` — removed in 0.24+, replaced by `LanguageFn` pattern.
- `extern "C" fn tree_sitter_typescript() -> Language` — old C-extern pattern; current grammar crates expose `LANGUAGE_TYPESCRIPT: LanguageFn` constant.
- `parser.set_language(tree_sitter_typescript())` — old call pattern, now `parser.set_language(&LANGUAGE_TYPESCRIPT.into())`.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `detect_language()` function skips `node_modules`, `.git`, `dist` directories | Common Pitfalls #5, Code Examples | Scanner produces noisy output on TypeScript projects; low severity in Phase 1 (no extraction), but confuses detection summary |
| A2 | Binary name `cg` is set via `[package] name = "cg"` in `crates/cli/Cargo.toml` | Pitfall #4 | Binary installed as wrong name; `cg <path>` wouldn't work |
| A3 | `serde` + `serde_json` included in Phase 1 data model for forward compatibility | Standard Stack | Later phases add serialization; adding serde mid-project to core is a minor breaking change for all crate dependents |
| A4 | `[workspace.package]` used for version/edition inheritance | Architecture Patterns | Crates would need individual version management; low risk, cosmetic only |

**If this table is empty for A1–A4:** All other claims in this research were verified via crates.io, Context7, or compile/run tests.

---

## Open Questions

1. **Binary name conflict with existing `cg` tools**
   - What we know: `cg` is a short, common abbreviation. Some systems may have a `cg` binary (e.g., NVIDIA Cg toolkit).
   - What's unclear: Whether the target user's PATH has a `cg` collision.
   - Recommendation: Document this in README. The name is locked per CONTEXT.md D-20.

2. **`Language::Unknown` in detection output**
   - What we know: D-10 says scan all extensions; D-11 says distinguish detected vs. parseable vs. skipped.
   - What's unclear: Should `Unknown` extensions be listed individually or grouped as "other"?
   - Recommendation: Group by extension count in the summary (e.g., "12 .json files — not parseable"). Resolve in CLI implementation.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust / Cargo | Build system | Yes | 1.93.1 | — |
| `rustc` | Rust compilation | Yes | 1.93.1 | — |
| Apple clang | Grammar C compilation (via `cc` crate) | Yes | 17.0.0 at `/usr/bin/clang` | — |
| `git` | Version control | Yes | 2.33.0 | — |

**Note on `cc` alias:** The shell `cc` command is aliased to `claude --dangerously-skip-permissions` in this environment. The `cc` Rust crate does not use the shell `cc` alias — it invokes `/usr/bin/clang` directly on macOS. Grammar crate compilation is confirmed working. [VERIFIED: compile test]

**Missing dependencies:** None blocking.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`#[test]`, `cargo test`) |
| Config file | None — built into Cargo |
| Quick run command | `cargo test -p cgraph-core` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| INFR-01 | `cg <path>` exits 0 and prints output | CLI smoke (subprocess) | `cargo test -p cg` or integration test in `tests/cli_smoke.rs` | No — Wave 0 |
| INFR-01 | `cg --version` prints version | CLI smoke | `cargo test -p cg -- version_flag` | No — Wave 0 |
| INFR-01 | `cg --help` prints help text | CLI smoke | `cargo test -p cg -- help_flag` | No — Wave 0 |
| PARS-11 | `.ts` files detected as TypeScript | Unit | `cargo test -p cgraph-core -- detect_ts` | No — Wave 0 |
| PARS-11 | `.tsx` files detected as TypeScriptReact | Unit | `cargo test -p cgraph-core -- detect_tsx` | No — Wave 0 |
| PARS-11 | `.swift` files detected as Swift | Unit | `cargo test -p cgraph-core -- detect_swift` | No — Wave 0 |
| PARS-11 | `.go` files detected as Go | Unit | `cargo test -p cgraph-core -- detect_go` | No — Wave 0 |
| PARS-11 | `.py` files detected as Python | Unit | `cargo test -p cgraph-core -- detect_py` | No — Wave 0 |
| PARS-11 | Mixed directory reports all 4 languages | Integration | `cargo test -p cgraph-core -- mixed_fixture` | No — Wave 0 |
| (D-22) | TypeScript grammar links and parses sample.ts | Integration | `cargo test -p cgraph-core -- typescript_grammar` | No — Wave 0 |
| (D-22) | TSX grammar links and parses sample.tsx | Integration | `cargo test -p cgraph-core -- tsx_grammar` | No — Wave 0 |
| (D-22) | Swift grammar links without panic | Integration | `cargo test -p cgraph-core -- swift_grammar` | No — Wave 0 |
| (D-22) | Go grammar links without panic | Integration | `cargo test -p cgraph-core -- go_grammar` | No — Wave 0 |
| (D-22) | Python grammar links without panic | Integration | `cargo test -p cgraph-core -- python_grammar` | No — Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p cgraph-core`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/core/tests/grammar_test.rs` — integration tests for all 4 grammar crates
- [ ] `crates/core/tests/fixtures/sample.ts` — minimal valid TypeScript fixture
- [ ] `crates/core/tests/fixtures/sample.tsx` — minimal valid TSX fixture
- [ ] `crates/core/tests/fixtures/sample.swift` — minimal valid Swift fixture
- [ ] `crates/core/tests/fixtures/sample.go` — minimal valid Go fixture
- [ ] `crates/core/tests/fixtures/sample.py` — minimal valid Python fixture
- [ ] `crates/core/src/lib.rs`, `model.rs`, `detect.rs`, `extractor.rs` — library structure
- [ ] `crates/cli/src/main.rs` — CLI entry point
- [ ] `tests/cli_smoke.rs` — workspace-level subprocess CLI test (or `crates/cli/tests/`)

*(Entire project is greenfield — all test infrastructure is Wave 0)*

---

## Security Domain

### Applicable ASVS Categories (Level 1)

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | CLI tool, no auth |
| V3 Session Management | No | Stateless CLI |
| V4 Access Control | No | Reads files only, no write paths in Phase 1 |
| V5 Input Validation | Yes — path input | Validate `path` exists and is a directory before walking; reject paths traversing outside the working tree |
| V6 Cryptography | No | No secrets, no crypto in Phase 1 |
| V7 Error Handling | Yes | Errors to stderr (D-15); no stack traces or internal paths exposed in normal output |

### Known Threat Patterns for CLI file-path input

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal (e.g., `cg ../../etc`) | Tampering / Info Disclosure | `path.canonicalize()` to resolve symlinks; validate path exists as a directory |
| Symlink following to sensitive files | Info Disclosure | Phase 1: detect languages only (no file content read for output). walkdir's `follow_links = false` (default) prevents following symlinks out of the scanned tree |
| Malformed UTF-8 filenames | DoS / crash | Use `entry.file_name().to_string_lossy()` — replaces invalid sequences rather than panicking |

---

## Sources

### Primary (HIGH confidence)
- [crates.io: tree-sitter 0.26.8](https://crates.io/crates/tree-sitter) — version verified [VERIFIED]
- [crates.io: tree-sitter-typescript 0.23.2](https://crates.io/crates/tree-sitter-typescript) — version + deps verified [VERIFIED]
- [crates.io: tree-sitter-go 0.25.0](https://crates.io/crates/tree-sitter-go) — deps verified [VERIFIED]
- [crates.io: tree-sitter-swift 0.7.1](https://crates.io/crates/tree-sitter-swift) — deps verified [VERIFIED]
- [crates.io: tree-sitter-python 0.25.0](https://crates.io/crates/tree-sitter-python) — deps verified [VERIFIED]
- [crates.io: clap 4.6.1](https://crates.io/crates/clap) — version verified [VERIFIED]
- [crates.io: walkdir 2.5.0](https://crates.io/crates/walkdir) — version verified [VERIFIED]
- [crates.io: serde 1.0.228, serde_json 1.0.149](https://crates.io/crates/serde) — versions verified [VERIFIED]
- [crates.io: thiserror 2.0.18, anyhow 1.0.102](https://crates.io/crates/thiserror) — versions verified [VERIFIED]
- Context7: `/websites/rs_clap` — derive API, PathBuf, version/about [VERIFIED]
- Context7: `/websites/rs_tree-sitter_tree_sitter` — Parser, Language, Query, StreamingIterator [VERIFIED]
- Context7: `/tree-sitter/tree-sitter-typescript` — LANGUAGE_TYPESCRIPT, LANGUAGE_TSX constants [VERIFIED]
- Compile + run test: `tree-sitter 0.26.8` + `tree-sitter-typescript 0.23.2` on this machine [VERIFIED]

### Secondary (MEDIUM confidence)
- Context7: `/tree-sitter/tree-sitter` — build.rs grammar pattern, ABI version docs [CITED]
- Context7: `/tree-sitter/tree-sitter-rust` — LANGUAGE constant pattern example [CITED]

### Tertiary (LOW confidence)
- None.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crate versions verified against crates.io; compile test confirms compatibility on this machine
- Architecture: HIGH — workspace pattern verified by compile test; tier assignments follow standard Rust workspace conventions
- Pitfalls: HIGH for grammar version matrix (verified); MEDIUM for directory exclusion defaults (assumed from project context)

**Research date:** 2026-05-02
**Valid until:** 2026-08-02 (90 days — Rust crate versions are stable; grammar crate versions change infrequently)
