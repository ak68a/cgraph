---
phase: 03-indexer-analysis-pipeline
reviewed: 2026-05-02T12:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - crates/indexer/src/graph.rs
  - crates/indexer/src/crawl.rs
  - crates/indexer/src/resolve.rs
  - crates/indexer/src/analysis.rs
  - crates/indexer/src/lib.rs
  - crates/indexer/Cargo.toml
  - crates/cli/src/main.rs
  - crates/cli/tests/cli_smoke.rs
  - crates/cli/Cargo.toml
findings:
  critical: 2
  warning: 7
  info: 3
  total: 12
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-05-02T12:00:00Z
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

The Phase 3 implementation delivers an indexer/crawl pipeline, import resolution (alias + barrel chains), and graph analysis (dead code, cycles, blast radius, transitive deps). The architecture is sound -- two-phase extraction followed by resolution is the correct approach, and the cycle guard on barrel chain resolution is well-considered. However, the review uncovered two critical correctness bugs in the resolution and analysis modules, plus several warnings around silent data loss, path handling robustness, and HashMap iteration non-determinism.

## Critical Issues

### CR-01: `split_id` uses first `::` but file paths on Windows contain `C::`

**File:** `crates/indexer/src/resolve.rs:344-347`
**Issue:** `split_id` finds the first `::` in an ID string to split `file_path::symbol_name`. On Windows, absolute paths such as `C:\project\src\file.ts` will be stored as `C:/project/src/file.ts` after the backslash replacement on line 169. However, if any path reaches `split_id` *before* backslash normalization (e.g., during the edge resolution pass when `source_id` already contains the original OS path), a Windows path like `C:\foo\bar.ts::func` would split at position 1 producing `("C", "\\foo\\bar.ts::func")`, corrupting both the file path and symbol name. More critically, even without Windows, the convention `file_path::symbol_name` is fragile: if any file path ever contains a literal `::` (rare but legal on Unix), every split will be wrong.

Additionally, `split_id` takes the *first* `::` occurrence, which means a file path containing `::` would cause a mis-split. The extractor builds IDs as `format!("{}::{}", file_path, name)`, so the last `::` is the separator, not the first.

**Fix:** Use `rsplit_once("::")` to split on the *last* `::` delimiter:
```rust
fn split_id(id: &str) -> Option<(&str, &str)> {
    id.rsplit_once("::")
}
```

### CR-02: `blast_radius` follows edges in the wrong direction -- computes dependents, not impact radius

**File:** `crates/indexer/src/analysis.rs:259-272`
**Issue:** The function documentation says "Returns all symbol IDs that transitively depend on the given symbol" and it uses `Reversed` graph with DFS from the target. However, `Reversed(&graph)` with `Dfs` from `start` traverses edges in the *reverse* direction -- that is, it follows incoming edges outward, which finds all nodes that *can reach* the start node via directed edges. In a graph where `A -> B -> C` (meaning A imports B, B imports C), calling `blast_radius("c.ts::C")` should find A and B (they depend on C). `Reversed` flips edge directions so the DFS follows what were originally incoming edges. This is actually correct for finding "who depends on me".

**However**, the `Dfs` type from petgraph traverses by following outgoing edges of the given graph reference. When you pass `Reversed(&graph)`, outgoing edges in the reversed graph are incoming edges in the original graph. So from C, it follows incoming edges to find B and A. This is correct.

*On re-examination, the logic is correct.* Downgrading -- removing this finding.

---

*CR-02 retracted after trace-through. Replacing with:*

### CR-02: Path traversal via tsconfig alias substitution not fully mitigated (T-03-04)

**File:** `crates/indexer/src/resolve.rs:156-181`
**Issue:** The T-03-04 path traversal check on lines 173-178 only guards against *absolute* resolved paths escaping the project root. A crafted tsconfig alias like `"@/": ["../../outside-project/"]` would produce a *relative* resolved path (e.g., `../../outside-project/secret.ts`) that passes the `is_absolute()` check because it's not absolute. The `normalize_import_path` function on line 119 handles relative paths by joining against the source file's parent directory, but the source file's parent is *inside* the project, so `../..` could escape. After normalization the path becomes absolute (joined against the source file's absolute parent), and only *then* is it checked -- but `strip_prefix(project_root)` on line 143 handles the failure case by returning the un-stripped absolute path, which is then checked on line 173. The issue is that `normalize_import_path` is called *before* `resolve_file_path` performs the escape check, and the normalization happens on the *aliased* path. If the aliased path is not relative (does not start with `./` or `../`), `normalize_import_path` returns it unchanged (line 148: `PathBuf::from(raw_import)`), so it stays as `../../outside-project/secret.ts`. This non-absolute, non-relative path bypasses both the relative-path normalization and the absolute-path escape check.

Specifically: alias resolution produces `../../outside-project/secret` from `@/secret`. This doesn't start with `./` or `../` -- wait, it does start with `../`. Let me retrace. The alias `@/` maps to `../../outside-project/`. So `@/secret` becomes `../../outside-project/secret`. This starts with `../`, so `normalize_import_path` treats it as relative, joining it with the source file's parent. If the source file is `/project/src/app.ts`, the joined path is `/project/src/../../outside-project/secret`, which normalizes to `/outside-project/secret`. Then `strip_prefix(/project)` fails, returning `/outside-project/secret`. Then `resolve_file_path` checks `is_absolute()` -- yes -- and `starts_with(project_root)` -- no -- so it correctly returns the raw import. **Actually, this is caught.**

But: if the alias target is an absolute path like `"/etc/"`, then `@/secret` becomes `/etc/secret`. `normalize_import_path` receives `/etc/secret` which does not start with `./` or `../`, so it returns `PathBuf::from("/etc/secret")`. Then `resolve_file_path` checks `is_absolute()` -- yes -- and it does not start with project root, so it returns the raw import. **This case is caught too.**

*On deeper trace-through, T-03-04 mitigation appears to cover the main attack vectors.* Downgrading this finding.

---

*CR-02 retracted after trace-through. Replacing with actual critical:*

### CR-02: `add_symbol` silently overwrites node_index on duplicate IDs, creating orphan graph nodes

**File:** `crates/indexer/src/graph.rs:28-33`
**Issue:** When `add_symbol` is called with a `SymbolNode` whose `id` already exists in `node_index`, the HashMap is overwritten to point to the new `NodeIndex`, but the old node remains in the petgraph `DiGraph` as an orphan -- unreachable via `get_index` but still counted by `node_count()` and iterated by `node_indices()`. This means:
1. Any edges added to the *old* node via its `NodeIndex` remain in the graph but the node is now shadow-indexed.
2. `dead_code` analysis iterates `node_indices()`, so it will see the orphan node and potentially flag it as dead code (false positive) since no edges will target it via `add_edge` (which looks up `node_index` and finds the *new* node).
3. The duplicate can occur in practice when two files define the same symbol ID (e.g., `file.ts::default` for default exports across files -- the extractor uses the file path, so this shouldn't happen, but barrel file re-export expansion in `resolve.rs:251-256` inserts hop_map entries for `barrel_file::name` which could collide with an actual node).

This is not purely theoretical: if the `TsExtractor` produces multiple nodes with the same `id` for a single file (e.g., function overloads, or duplicate declarations in partial-parse error recovery), the graph silently corrupts.

**Fix:** Either detect and reject duplicates, or remove the old node:
```rust
pub fn add_symbol(&mut self, node: SymbolNode) -> NodeIndex {
    let id = node.id.clone();
    if let Some(&existing_idx) = self.node_index.get(&id) {
        // Update existing node in-place rather than creating orphan
        *self.graph.node_weight_mut(existing_idx).unwrap() = node;
        return existing_idx;
    }
    let idx = self.graph.add_node(node);
    self.node_index.insert(id, idx);
    idx
}
```

## Warnings

### WR-01: HashMap iteration order in `TsConfigAliases::resolve` causes non-deterministic alias matching

**File:** `crates/indexer/src/resolve.rs:63-73`
**Issue:** When multiple alias prefixes could match a path (e.g., `"@/"` and `"@/components/"`), the `for (prefix, targets) in &self.aliases` loop iterates in HashMap's arbitrary order. The first matching prefix wins, which means the result is non-deterministic across runs (HashMap randomizes iteration order in Rust). A path like `@/components/Button` could match either `@/` or `@/components/` depending on iteration order, producing different resolved paths.

**Fix:** Sort alias prefixes by length descending (longest-prefix-first) at load time, or use a `BTreeMap` for deterministic ordering, or collect and sort at resolution time:
```rust
pub fn resolve(&self, raw_path: &str) -> String {
    let mut best_match: Option<(&str, &Vec<String>)> = None;
    for (prefix, targets) in &self.aliases {
        if raw_path.starts_with(prefix.as_str()) {
            if best_match.is_none() || prefix.len() > best_match.unwrap().0.len() {
                best_match = Some((prefix.as_str(), targets));
            }
        }
    }
    if let Some((prefix, targets)) = best_match {
        if let Some(first_target) = targets.first() {
            let suffix = &raw_path[prefix.len()..];
            return format!("{}{}", first_target, suffix);
        }
    }
    raw_path.to_string()
}
```

### WR-02: `strip_json_comments` does not handle block comments (`/* ... */`)

**File:** `crates/indexer/src/resolve.rs:78-110`
**Issue:** tsconfig.json files commonly use block comments (`/* ... */`) in addition to single-line comments. The comment stripping only handles `//` comments. A tsconfig with block comments will fail to parse, causing a silent fallback to empty aliases. This is a graceful degradation per D-13, but it means projects using block comments in tsconfig will silently lose all path alias resolution, which could cause all import edges to be unresolved.

**Fix:** Add block comment handling:
```rust
} else if chars[i] == '/' && i + 1 < len && chars[i + 1] == '*' {
    // Skip block comment
    i += 2;
    while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
        i += 1;
    }
    i += 2; // skip closing */
    continue;
}
```

### WR-03: `resolve_extension` scans all graph nodes for every extension candidate -- correctness issue with early match

**File:** `crates/indexer/src/resolve.rs:321-336`
**Issue:** The inner loop on lines 329-334 checks if *any* node in the graph has a matching `file_path` for the candidate, independent of the symbol name. This means if `foo.ts` exists in the graph with symbol `bar`, resolving `foo::baz` would match `foo.ts` even though `baz` is not defined there. The edge would then be created with target `foo.ts::baz`, which would fail to resolve in `add_edge` (silently dropped) -- a correct outcome but the resolution function gives a misleading "successful" resolution. More importantly, this creates a false positive: the path is "resolved" so no further fallback candidates are tried. If the correct file were `foo/index.ts::baz`, it would never be checked.

**Fix:** Remove the file-path-only fallback or move it to a separate, lower-priority pass:
```rust
for candidate in &candidates {
    let test_id = format!("{}::{}", candidate, symbol);
    if graph.get_index(&test_id).is_some() {
        return candidate.clone();
    }
}
// Only fall back to file-path match if no exact symbol match found
for candidate in &candidates {
    for idx in graph.graph.node_indices() {
        let node = &graph.graph[idx];
        if node.file_path == *candidate {
            return candidate.clone();
        }
    }
}
```

### WR-04: `dead_code` analysis iterates edges three times and graph nodes once redundantly

**File:** `crates/indexer/src/analysis.rs:127-180`
**Issue:** The dead code function iterates all edges three times in sequence (lines 128-138, 144-164, 167-180) to collect different sets. The first two loops (lines 128-138 and 144-164) both look for `unresolved::` targets and produce overlapping `unresolved_targets` and `unresolved_call_names` HashSets. The `unresolved_targets` set (line 128) is never actually used anywhere in the function -- it is populated but never read. This is dead code within the dead code detector.

**Fix:** Remove the first loop (lines 127-138) and the `unresolved_targets` variable entirely -- only `unresolved_call_names` is used in the demotion check on line 211.

### WR-05: Test temp directories not isolated -- parallel test runs can interfere

**File:** `crates/indexer/src/crawl.rs:114-129`
**Issue:** Tests use fixed temp directory names like `cgraph_test_index_single` under `std::env::temp_dir()`. When tests run in parallel (Rust's default), multiple test processes or `cargo test` invocations can race on the same directory. The `fs::remove_dir_all` at the start mitigates this partially, but there's a TOCTOU race between the `remove_dir_all` and `create_dir_all`. This applies to all test functions in `crawl.rs` (lines 113-257) and could cause flaky CI.

**Fix:** Use `tempfile::tempdir()` for unique, auto-cleaned temp directories:
```rust
let tmp = tempfile::tempdir().unwrap();
let tmp_path = tmp.path();
// ... use tmp_path instead of tmp ...
// directory auto-removed when `tmp` goes out of scope
```

### WR-06: `normalize_import_path` does not guard against `..` escaping past root component

**File:** `crates/indexer/src/resolve.rs:126-139`
**Issue:** The `ParentDir` handling on line 130 calls `components.pop()`. If the path has more `..` segments than there are components, `pop()` returns `None` and the loop continues -- the `..` is silently swallowed. Consider path `/project/src/../../../etc/passwd` resolved against project root `/project`. Components after joining: `[RootDir, "project", "src", ParentDir, ParentDir, ParentDir, "etc", "passwd"]`. Processing: `[RootDir, "project", "src"]` -> pop -> `[RootDir, "project"]` -> pop -> `[RootDir]` -> pop -> `[]` -> push "etc" -> push "passwd" -> result: `etc/passwd`. Then `strip_prefix(project_root)` fails, returning `etc/passwd` as a relative path. While `resolve_file_path` checks absolute paths, this relative `etc/passwd` passes through unchecked. In practice, the file won't exist so the edge is silently dropped, but the path traversal attempt itself should be caught and logged.

**Fix:** Guard the pop and abort if the path would escape root:
```rust
Component::ParentDir => {
    if components.pop().is_none() {
        eprintln!("warn: import path escapes filesystem root: {}", raw_import);
        return PathBuf::from(raw_import);
    }
}
```

### WR-07: `IndexerError::From<std::io::Error>` loses the file path context

**File:** `crates/indexer/src/crawl.rs:18-25`
**Issue:** The `From<std::io::Error>` impl creates an `IndexerError::Io` with `path: String::new()`. This means any `?` operator on an `io::Error` in `index()` (line 51: `scan_directory(project_root)?`) produces an error message like `"I/O error scanning : <os error>"` with an empty path, making the error undiagnosable.

**Fix:** Remove the blanket `From` impl and use explicit error construction with `.map_err()`:
```rust
let detection = scan_directory(project_root)
    .map_err(|e| IndexerError::Io {
        path: project_root.display().to_string(),
        source: e,
    })?;
```

## Info

### IN-01: Unused import in analysis.rs

**File:** `crates/indexer/src/analysis.rs:3`
**Issue:** `petgraph::visit::Dfs` is imported along with `Reversed`, which is used. However, `Dfs` is also used in `blast_radius` and `transitive_deps`, so this is actually fine. But `petgraph::graph::DiGraph` on line 2 is imported at module scope and only used inside `detect_cycles`. It's not wrong, but could be scoped to the function for clarity.

**Fix:** Move the import inside `detect_cycles` if desired, or leave as-is. Low priority.

### IN-02: `edge.kind.clone()` in crawl.rs is unnecessary for Copy-like enum

**File:** `crates/indexer/src/crawl.rs:99`
**Issue:** `EdgeKind` derives `Clone` but not `Copy`. Since it's a simple enum with no heap data, it could derive `Copy` to avoid the `.clone()` call. This is a minor ergonomic improvement.

**Fix:** Add `Copy` to `EdgeKind`'s derive list in `crates/core/src/model.rs:23`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind { ... }
```

### IN-03: `is_entry_point` treats *any* file in a test directory as an entry point

**File:** `crates/indexer/src/analysis.rs:66-69`
**Issue:** Any file with a path component matching `TEST_DIR_NAMES` is excluded from dead code detection. This means `__tests__/helpers/format.ts::formatDate` would be excluded even though test helpers can have dead exported functions. The current behavior is a reasonable conservative choice (avoid false positives in test files), but it may produce false negatives for dead test utilities.

**Fix:** Consider only excluding files that match test file naming patterns (e.g., `*.test.ts`, `*.spec.ts`) in addition to being in test directories, rather than blanket-excluding all files in test directories. Low priority.

---

_Reviewed: 2026-05-02T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
