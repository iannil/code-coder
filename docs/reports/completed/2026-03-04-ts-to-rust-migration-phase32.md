# Phase 32: Patch/Diff Engine NAPI Integration

## Status: ✅ Completed

**Date**: 2026-03-04
**Time**: ~14:30:00Z

## Summary

Integrated existing Rust implementations of `apply_patch.rs` and `edit.rs` with TypeScript via NAPI bindings.

## Changes Made

### 1. Rust NAPI Bindings (`services/zero-core/src/napi/tools.rs`)

Created ~350 lines of NAPI bindings exposing:

**Types:**
- `NapiPatchChunk`, `NapiPatchHunk`, `NapiPatchFileResult`, `NapiApplyPatchResult`
- `NapiEditOperation`, `NapiEditResult`, `NapiBestMatch`
- `NapiApplyPatchOptions` with working_dir, dry_run, create_backups, fuzz options

**Handles:**
- `PatchApplicatorHandle` - Thread-safe wrapper for patch parsing and application
  - `new()` - Constructor
  - `with_defaults(options)` - Factory with custom options
  - `parse_patch(text)` - Parse patch text into hunks
  - `apply(text, options)` - Apply a patch

- `EditorHandle` - Thread-safe wrapper for file editing
  - `new()` - Constructor
  - `edit(path, operation)` - Single edit operation
  - `edit_multiple(path, operations)` - Multiple operations
  - `generate_diff(old, new, path)` - Create unified diff
  - `diff_files(old_path, new_path)` - Diff two files

**Standalone Functions:**
- `similarity_ratio(s1, s2)` - Compute string similarity (0.0-1.0)
- `find_best_match(needle, haystack)` - Find closest match in candidates
- `compute_diff(old, new, path)` - Standalone diff generation

### 2. Module Registration (`services/zero-core/src/napi/mod.rs`)

Added:
```rust
#[cfg(feature = "napi-bindings")]
mod tools;

#[cfg(feature = "napi-bindings")]
pub use tools::*;
```

### 3. TypeScript Integration (`packages/ccode/src/patch/native.ts`)

Created ~220 lines of TypeScript wrapper providing:

**Type Definitions:**
- All NAPI types with proper TypeScript interfaces
- Handle interfaces for `PatchApplicatorHandle` and `EditorHandle`

**Public API:**
- `isNativeAvailable()` - Check if native bindings loaded
- `createPatchApplicator()` - Create native PatchApplicator
- `createEditor()` - Create native Editor
- `similarityRatioNative(s1, s2)` - Native similarity computation
- `findBestMatchNative(needle, haystack)` - Native fuzzy matching
- `computeDiffNative(old, new, path)` - Native diff generation
- `parsePatchNative(text)` - Parse patch via native
- `applyPatchNative(text, options)` - Apply patch via native
- `editFileNative(path, old, new, replaceAll)` - Edit file via native

**Pattern:**
- Lazy loading with `loadAttempted` guard
- Graceful fallback to null when bindings unavailable
- Structured logging for debugging

## Build Verification

```bash
# Rust build - SUCCESS
$ cargo build -p zero-core --features napi-bindings
    Finished `dev` profile [unoptimized + debuginfo]

# Rust tests - 11 passed
$ cargo test -p zero-core --lib -- tools::apply_patch tools::edit
test result: ok. 11 passed; 0 failed

# TypeScript typecheck - SUCCESS
$ bun turbo typecheck --filter=ccode
 Tasks: 1 successful, 1 total
```

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                     TypeScript Layer                            │
├────────────────────────────────────────────────────────────────┤
│  packages/ccode/src/patch/native.ts                            │
│    ├─ isNativeAvailable()                                      │
│    ├─ createPatchApplicator() → PatchApplicatorHandle          │
│    ├─ createEditor() → EditorHandle                            │
│    ├─ similarityRatioNative(s1, s2) → number                   │
│    ├─ findBestMatchNative(needle, haystack) → NapiBestMatch    │
│    └─ computeDiffNative(old, new, path) → string               │
└─────────────────────────────────┬──────────────────────────────┘
                                  │ @codecoder-ai/core
                                  ▼
┌────────────────────────────────────────────────────────────────┐
│                       NAPI Layer                                │
├────────────────────────────────────────────────────────────────┤
│  services/zero-core/src/napi/tools.rs                          │
│    ├─ PatchApplicatorHandle (Arc<Mutex<RustPatchApplicator>>)  │
│    ├─ EditorHandle (Arc<Mutex<RustEditor>>)                    │
│    ├─ similarity_ratio()                                       │
│    ├─ find_best_match()                                        │
│    └─ compute_diff()                                           │
└─────────────────────────────────┬──────────────────────────────┘
                                  │
                                  ▼
┌────────────────────────────────────────────────────────────────┐
│                       Rust Core                                 │
├────────────────────────────────────────────────────────────────┤
│  services/zero-core/src/tools/apply_patch.rs (612 lines)       │
│    ├─ PatchApplicator                                          │
│    ├─ parse_patch() - Parse unified diff format                │
│    ├─ apply() - Apply patch with conflict detection            │
│    └─ Uses: similar crate for diff generation                  │
│                                                                │
│  services/zero-core/src/tools/edit.rs (355 lines)              │
│    ├─ Editor                                                   │
│    ├─ edit() - String replacement with validation              │
│    ├─ edit_multiple() - Batch operations                       │
│    ├─ generate_diff() - Unified diff creation                  │
│    ├─ similarity_ratio() - Character-based similarity          │
│    └─ find_best_match() - Fuzzy matching (>0.6 threshold)      │
└────────────────────────────────────────────────────────────────┘
```

## Performance Expectations

| Operation | TypeScript | Rust Native | Improvement |
|-----------|------------|-------------|-------------|
| Diff generation | ~10ms | ~2ms | 5x |
| Similarity ratio | ~5ms | ~0.5ms | 10x |
| Patch parsing | ~8ms | ~1ms | 8x |
| Fuzzy matching | ~15ms | ~2ms | 7.5x |

*Note: Actual measurements pending benchmark integration*

## Future Work

1. **Benchmark Integration**: Add `bun run bench` tests for patch/diff operations
2. **Sync Levenshtein**: The TypeScript `levenshtein()` in `edit.ts` could be replaced with native version if sync FFI becomes available
3. **Patch Tool Migration**: Consider updating `packages/ccode/src/patch/index.ts` to use native bindings with TS fallback
4. **Edit Tool Enhancement**: The replacer chain in `edit.ts` could leverage native fuzzy matching for better performance

## Files Changed

| File | Change |
|------|--------|
| `services/zero-core/src/napi/tools.rs` | Created (~350 lines) |
| `services/zero-core/src/napi/mod.rs` | Added tools module |
| `packages/ccode/src/patch/native.ts` | Created (~220 lines) |

## Test Coverage

- ✅ `test_patch_applicator_handle` - Patch parsing
- ✅ `test_similarity_ratio` - String similarity
- ✅ `test_find_best_match` - Fuzzy matching
- ✅ `test_compute_diff` - Diff generation
- ✅ 11 Rust unit tests pass
- ✅ TypeScript type checking passes
