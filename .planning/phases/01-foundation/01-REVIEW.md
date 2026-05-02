---
phase: 01-foundation
reviewed: 2026-05-02T00:00:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - Cargo.toml
  - crates/cli/Cargo.toml
  - crates/cli/src/main.rs
  - crates/cli/tests/cli_smoke.rs
  - crates/core/Cargo.toml
  - crates/core/src/detect.rs
  - crates/core/src/extractor.rs
  - crates/core/src/lib.rs
  - crates/core/src/model.rs
  - crates/core/tests/grammar_test.rs
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-05-02
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Reviewed the Phase 01 foundation: workspace layout, CLI entrypoint, language detection, model types, the Extractor trait, and all tests. The skeleton is structurally sound — the crate split, error types, and `WalkDir` usage are all reasonable. One logic bug in the output layer produces a wrong file count that will mislead users. Three warnings address silent error suppression, a test-isolation hazard, and a non-ergonomic API on `DetectionResult`. Two info items flag code style issues worth cleaning up before the codebase grows.

---

## Critical Issues

### CR-01: "Total files found" double-counts Unknown-extension files

**File:** `crates/cli/src/main.rs:52`

**Issue:** `scan_directory` pushes every file that has a recognised extension into `result.detected`, then additionally pushes Unknown-extension files into `result.skipped`. The two collections are not disjoint — every skipped file already appears in `detected`. The `total` calculation adds both lengths:

```rust
let total = result.detected.len() + result.skipped.len();
```

For a directory containing 3 `.ts` files and 2 `.json` files this prints "Total files found: 7" when there are only 5 files with extensions. Files with no extension (Makefiles, etc.) are silently invisible in this count, compounding the confusion.

The root cause is that `DetectionResult.detected` is semantically "all files that have any extension," whereas the caller treats it as "all files scanned." The simplest fix is to compute `total` as `parseable.len() + skipped.len()` (which equals `detected.len()` and avoids the double-count), or alternatively to stop populating `detected` at all and derive the total from the two disjoint buckets:

```rust
// Fix: total is the union of the two disjoint buckets
let total = result.parseable.len() + result.skipped.len();
println!("Total files found: {}", total);
```

If `detected` is still needed for callers who want the raw `Language` enum for every file (parseable or not), that is fine, but the `total` line must not add `skipped.len()` on top of it.

---

## Warnings

### WR-01: WalkDir errors are silently swallowed in `scan_directory`

**File:** `crates/core/src/detect.rs:42`

**Issue:** The iterator arm `Some(Err(_)) => continue` discards all WalkDir errors without recording them anywhere. A permission-denied error on a directory causes that subtree to be silently skipped. For a scan tool this means the user may receive an incomplete result with no indication that files were missed.

```rust
Some(Err(_)) => continue,   // permission errors, I/O errors — silently dropped
```

**Fix:** Collect walk errors into a dedicated field on `DetectionResult` (or surface them as warnings on stderr) so callers can decide whether to abort or warn:

```rust
#[derive(Debug, Default)]
pub struct DetectionResult {
    pub detected: Vec<(PathBuf, Language)>,
    pub parseable: Vec<(PathBuf, Language)>,
    pub skipped: Vec<(PathBuf, String)>,
    pub walk_errors: Vec<walkdir::Error>,  // add this field
}

// in the loop:
Some(Err(e)) => {
    result.walk_errors.push(e);
    continue;
}
```

The CLI can then print a warning for each walk error when `--verbose` is set, or always print them to stderr.

### WR-02: Integration tests use fixed temp-directory names, causing races and stale-state failures

**File:** `crates/core/src/detect.rs:145,174,192`

**Issue:** The three `scan_directory` tests each hard-code a temp path:

```rust
let tmp = std::env::temp_dir().join("cgraph_test_scan_directory");
let tmp = std::env::temp_dir().join("cgraph_test_scan_hidden");
let tmp = std::env::temp_dir().join("cgraph_test_scan_node_modules");
```

Cargo runs unit tests in the same process concurrently by default. If two test runs overlap (e.g. CI reruns without cleanup, or `cargo test` is called twice in quick succession), the directories from a previous run may contain leftover files that cause assertion failures. More importantly, the cleanup `std::fs::remove_dir_all(&tmp).ok()` only runs at end-of-test; if an assertion panics, cleanup is skipped and the stale directory persists.

**Fix:** Use a unique suffix per run, or use the `tempfile` crate for automatic cleanup on drop:

```rust
// Option A: unique suffix with thread ID / timestamp
let tmp = std::env::temp_dir()
    .join(format!("cgraph_test_scan_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos()));

// Option B (preferred): use tempfile crate for guaranteed cleanup
let tmp = tempfile::tempdir().expect("failed to create temp dir");
// tmp auto-deletes on Drop even if the test panics
```

### WR-03: `Language` enum is formatted with `{:?}` (Debug) in user-facing output

**File:** `crates/cli/src/main.rs:42,89`

**Issue:** Two places produce user-visible output using the `Debug` representation of `Language`:

```rust
let key = format!("{:?}", lang);          // line 42 — used as printed label
println!("  [parseable] {:?} — {}", lang, ...);  // line 89
```

`{:?}` on `Language::TypeScriptReact` prints `TypeScriptReact` (Rust enum variant name). This is an implementation detail, not a user-friendly label. It also means the display format is coupled to the Rust identifier name: renaming a variant would silently change user-visible output.

**Fix:** Implement `std::fmt::Display` for `Language` with explicit, stable labels:

```rust
impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Language::TypeScript     => "TypeScript",
            Language::TypeScriptReact => "TypeScript (TSX)",
            Language::Swift          => "Swift",
            Language::Go             => "Go",
            Language::Python         => "Python",
            Language::Unknown(ext)   => return write!(f, "Unknown (.{})", ext),
        };
        f.write_str(label)
    }
}
```

Then replace `{:?}` with `{}` at both call sites.

---

## Info

### IN-01: `DetectionResult.detected` field is a partial superset that invites future misuse

**File:** `crates/core/src/detect.rs:30-33`

**Issue:** `detected` contains all files that have any extension (both parseable and unknown-extension files). It does not contain files with no extension. The name implies "everything we looked at" but it is neither the complete set of walked files nor a fully distinct bucket. The three fields (`detected`, `parseable`, `skipped`) have an unclear and surprising relationship: `parseable` and `skipped` are disjoint, but both are subsets of `detected`, and files with no extension appear in none of them.

If the `detected` field has no use case distinct from `parseable.len() + skipped.len()`, removing it would eliminate the source of the CR-01 bug and simplify the struct. If it is needed (e.g. to preserve the `Language::Unknown` value for callers), a clarifying doc comment explaining the invariant is the minimum fix:

```rust
/// All files whose path has a file extension, regardless of whether they are
/// parseable. Equals `parseable ∪ { f | f ∈ skipped }`. Does NOT include
/// files with no extension (e.g. `Makefile`).
pub detected: Vec<(PathBuf, Language)>,
```

### IN-02: `SymbolNode.id` separator `::` is ambiguous for symbol names that contain `::`

**File:** `crates/core/src/model.rs:33`

**Issue:** The ID format is documented as `"file_path::symbol_name"`. If a symbol name itself contains `::` (common in Rust, possible in TypeScript namespaces), the ID becomes ambiguous and cannot be reliably split back into its components. This is a forward-looking concern for Phase 2 extractors, but the schema is frozen here.

**Fix:** Use a separator that cannot appear in file paths or symbol names across the target languages, or store `file_path` and `name` separately and build IDs only at serialisation time using a stable encoding. A safe separator choice would be a null byte or a fixed multi-character sequence unlikely to appear in paths (e.g. `\x00`). At minimum, add a code comment warning future extractor authors about the constraint:

```rust
pub id: String, // "{file_path}::{symbol_name}" — '::' MUST NOT appear in symbol_name
```

---

_Reviewed: 2026-05-02_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
