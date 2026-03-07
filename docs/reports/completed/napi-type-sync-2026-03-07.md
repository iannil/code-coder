# NAPI Type Sync Implementation Report

**Date**: 2026-03-07
**Status**: Completed

---

## Summary

Implemented automated NAPI type synchronization to reduce manual maintenance burden for `packages/core/src/binding.d.ts`.

## Implementation Details

### Files Created/Modified

| File | Action | Description |
|------|--------|-------------|
| `scripts/sync-napi-types.ts` | Created | Smart merge script for NAPI types |
| `packages/core/package.json` | Modified | Added sync-types scripts |
| `ops.sh` | Modified | Added sync_napi_types function |

### Sync Strategy: Smart Merge

The implementation uses a **merge strategy** rather than simple replacement because:

1. **NAPI types** (from `services/zero-core/index.d.ts`): Reflect actual Rust implementations
2. **Manual types** (in `packages/core/src/binding.d.ts`): Forward declarations for planned features

The sync script:
1. Parses both files to extract type declarations
2. Uses NAPI-generated types as authoritative (shared types)
3. Preserves manual types that don't exist in NAPI output
4. Fixes TypeScript reserved keywords (`extends`, `interface`) in parameter names
5. Outputs merged result with clear header

### Usage

```bash
# Manual sync
cd packages/core
bun run sync-types

# Preview changes without writing
bun run sync-types:dry-run

# Automatic sync after Rust build
./ops.sh build rust
```

### Analysis Results

After initial sync:
- **NAPI types**: 255 (actual Rust implementations)
- **Manual types preserved**: 139 (forward declarations)
- **New types added**: 98 (from NAPI, not in old binding.d.ts)
- **Total exports**: 325 (up from 176)

## Pre-existing Issues Discovered

The typecheck was already failing before this implementation due to `index.ts` referencing types that don't exist in either file:

- `IgnoreEngineHandle`, `ContextLoaderHandle`, `EmbeddingIndexHandle`
- Various `Napi*` interfaces not yet implemented in Rust
- These are forward references to planned implementations

**Recommendation**: Create a tracking issue for implementing missing Rust bindings.

## Verification

```bash
# Sync is idempotent
bun scripts/sync-napi-types.ts
bun scripts/sync-napi-types.ts --dry-run
# Second run shows: "New in NAPI: 0"

# Reserved keyword fixes applied
grep -c "extendsFrom:" packages/core/src/binding.d.ts  # Should find matches
grep -c "interfaceName:" packages/core/src/binding.d.ts  # Should find matches
```

## Future Work

1. **Implement missing Rust bindings**: Address the pre-existing type errors
2. **CI integration**: Add GitHub Actions check for type sync
3. **Remove manual types**: As Rust implementations are added, manual types become redundant

---

## Technical Notes

### Reserved Keyword Handling

NAPI-RS uses Rust parameter names directly in generated TypeScript. Rust allows identifiers like `extends` and `interface`, but TypeScript reserves them. The sync script renames:

- `extends:` → `extendsFrom:`
- `interface:` → `interfaceName:`

### Multi-line Function Support

The parser handles multi-line function declarations by tracking parentheses depth until a return type is found.
