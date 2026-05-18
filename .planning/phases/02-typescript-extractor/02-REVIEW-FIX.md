---
phase: 02-typescript-extractor
fixed_at: 2026-05-02T17:30:00Z
review_path: .planning/phases/02-typescript-extractor/02-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 02: Code Review Fix Report

**Fixed at:** 2026-05-02T17:30:00Z
**Source review:** .planning/phases/02-typescript-extractor/02-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4
- Fixed: 4
- Skipped: 0

## Fixed Issues

### CR-01: Exported Symbol `line_start`/`line_end` Are Identical (Point to Identifier, Not Declaration Body)

**Files modified:** `crates/ts-extractor/src/symbols.rs`, `crates/ts-extractor/tests/extraction_test.rs`
**Commit:** 54c7a9c
**Applied fix:** Used the `@export_stmt` capture (which wraps the full `export_statement` node) instead of the `@symbol_name` capture (identifier token) for `line_start`/`line_end` computation. The queries already had `@export_stmt` defined on all patterns -- the code simply was not using it. Added a regression test `exported_symbol_line_span_covers_full_declaration` that verifies multi-line exported symbols (fetchUser, UserRepository) have `line_end > line_start`.

### WR-01: `PartialParse` Error Always Reports Line 0

**Files modified:** `crates/ts-extractor/src/lib.rs`, `crates/ts-extractor/tests/extraction_test.rs`
**Commit:** 7377ed3
**Applied fix:** Added `find_first_error_line()` function that recursively walks the tree to find the first ERROR or MISSING node and returns its 1-based line number. Replaced `root.start_position().row as u32` (always 0) with the result of this function. Added regression test `partial_parse_error_reports_correct_line` that verifies the error line is > 0 for syntax errors that occur after the first line.

### WR-02: Aliased Re-Exports Emit Wrong `source_id`

**Files modified:** `crates/ts-extractor/src/queries.rs`, `crates/ts-extractor/src/edges.rs`, `crates/ts-extractor/tests/extraction_test.rs`
**Commit:** 9ae9486
**Applied fix:** Added `alias: (identifier)? @alias_name` optional capture to the named re-export pattern in `REEXPORT_QUERY_SRC`. In `extract_reexports`, the code now reads the `@alias_name` capture and uses `alias.unwrap_or(name)` as the public name for `source_id`. The `target_id` continues to use the original name (the symbol the source module exports). Non-aliased re-exports are unaffected. Added regression test `reexport_aliased_uses_public_name` that verifies `export { foo as bar }` produces `source_id = "index.ts::bar"` and `target_id = "./module::foo"`.

### WR-03: Test Fixture `barrel.ts` References Non-Existent Symbol `UserSchema`

**Files modified:** `crates/ts-extractor/tests/fixtures/schemas.ts`
**Commit:** a6630a2
**Applied fix:** Added `export const UserSchema = { type: 'object', required: ['id', 'name', 'email', 'role'] };` to `schemas.ts` so that `barrel.ts` line 2 (`export { UserSchema, type UserType } from './schemas'`) references a symbol that actually exists. All existing tests continue to pass.

## Skipped Issues

None -- all findings were fixed.

---

_Fixed: 2026-05-02T17:30:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
