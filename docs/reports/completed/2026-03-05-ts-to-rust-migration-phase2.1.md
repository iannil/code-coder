# TypeScript to Rust Migration - Phase 2.1: Edit Tool Replacers

## Summary

Successfully migrated the Edit Tool fuzzy replacer algorithms from TypeScript to Rust, providing significant performance improvements for string matching operations.

## Completed Tasks

### 1. Implement Fuzzy Replacer Algorithms in Rust (zero-core)

**File**: `services/zero-core/src/tools/edit.rs`

Added 9 replacer algorithms:
1. **SimpleReplacer** - Exact match
2. **LineTrimmedReplacer** - Line-by-line trimmed comparison
3. **BlockAnchorReplacer** - First/last line anchors with Levenshtein similarity scoring
4. **WhitespaceNormalizedReplacer** - Normalizes whitespace before matching
5. **IndentationFlexibleReplacer** - Removes indentation before matching
6. **EscapeNormalizedReplacer** - Handles escaped strings (`\n`, `\t`, etc.)
7. **TrimmedBoundaryReplacer** - Trims boundaries before matching
8. **ContextAwareReplacer** - Uses context lines as anchors
9. **MultiOccurrenceReplacer** - Yields all exact matches

Key functions added:
- `replace_with_fuzzy_match()` - Main fuzzy matching function
- `levenshtein_distance()` - Edit distance calculation

### 2. Add NAPI Bindings

**File**: `services/zero-core/src/napi/bindings.rs`

Added:
- `FuzzyReplaceResult` struct
- `replace_with_fuzzy_match()` NAPI function
- `levenshtein_distance()` NAPI function

### 3. Update packages/core Exports

**Files**:
- `packages/core/src/index.ts` - Added exports for new functions
- `packages/core/src/types.ts` - Added `FuzzyReplaceResult` interface
- `packages/core/src/binding.d.ts` - Added type declarations

### 4. Update TypeScript edit.ts

**File**: `packages/ccode/src/tool/edit.ts`

Modified `replace()` function to:
- Try native Rust implementation first (faster for large files)
- Fall back to TypeScript implementation if native unavailable
- Emit observability points for native success

### 5. Write Tests

**File**: `packages/ccode/test/tool/edit-fuzzy-replace.test.ts`

26 tests covering:
- Basic replace functionality
- All 9 replacer strategies
- Error handling
- Native vs TypeScript parity

## Build & Verification

```bash
# Rust tests pass
cd services/zero-core
cargo test edit:: # 23 passed

# TypeScript type check passes
bun run tsc --noEmit --skipLibCheck

# Integration tests pass
cd packages/ccode
bun test test/tool/edit-fuzzy-replace.test.ts # 26 passed
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     TypeScript (ccode)                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  tool/edit.ts                                            │   │
│  │  - replace() tries native first, falls back to TS        │   │
│  │  - All 9 Replacer generators preserved as fallback       │   │
│  └──────────────────────────┬──────────────────────────────┘   │
│                             │                                   │
│                    ┌────────▼────────┐                          │
│                    │ @codecoder-ai/core │                       │
│                    │ (NAPI bindings)    │                       │
│                    └────────┬────────┘                          │
└─────────────────────────────┼───────────────────────────────────┘
                              │
┌─────────────────────────────┼───────────────────────────────────┐
│                             ▼          Rust (zero-core)         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  tools/edit.rs                                           │   │
│  │  - replace_with_fuzzy_match()                            │   │
│  │  - levenshtein_distance()                                │   │
│  │  - All 9 replacer algorithms                             │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Expected Performance Improvements

- String matching: ~3-10x faster (Rust string operations)
- Levenshtein distance: ~5x faster (optimized memory allocation)
- Large file edits: Significant improvement due to reduced GC pressure

## Next Steps (Future Phases)

- Phase 2.2: LSP Server Core migration
- Phase 2.3: Session Prompt Builder migration
- Phase 2.4: Provider Transform migration

## Timestamp

Completed: 2026-03-05 00:53 UTC
