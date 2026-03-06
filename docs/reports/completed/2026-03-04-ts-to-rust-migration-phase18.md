# Phase 18: File Search Engine with Ripgrep

**Date**: 2026-03-04
**Status**: Completed

## Summary

Updated `packages/ccode/src/file/ripgrep.ts` to use native Rust implementations from `@codecoder-ai/core` for file listing and content search, with fallback to the external `rg` binary.

## Changes Made

### 1. Added Native Imports
```typescript
import { grep as nativeGrep, glob as nativeGlob } from "@codecoder-ai/core"
```

### 2. Updated `files()` Function
- Tries native `glob()` first for SIMD-accelerated file traversal
- Falls back to rg binary if native fails
- Preserves generator interface for streaming results

### 3. Updated `search()` Function
- Tries native `grep()` first for parallel search
- Falls back to rg binary if native fails
- Transforms results to match existing API

## Architecture

```
Ripgrep.files()
    │
    ├─→ Try: nativeGlob() ─→ zero-core/src/tools/glob.rs
    │                              │
    │                              ├─→ WalkBuilder (ignore crate)
    │                              ├─→ Parallel file traversal
    │                              └─→ Respects .gitignore
    │
    └─→ Fallback: rg binary ─→ Bun.spawn(["rg", "--files", ...])

Ripgrep.search()
    │
    ├─→ Try: nativeGrep() ─→ zero-core/src/tools/grep.rs
    │                              │
    │                              ├─→ grep-regex + grep-searcher
    │                              ├─→ Parallel file search
    │                              └─→ Binary detection
    │
    └─→ Fallback: rg binary ─→ Bun.spawn(["rg", "--json", ...])
```

## Rust Implementation Details

The native implementation uses the same libraries as ripgrep:
- `grep-regex`: Regex pattern matching
- `grep-searcher`: Line-oriented searching with context
- `ignore`: Git-aware file walking with .gitignore support

Key features:
- Parallel search with `num_cpus::get().min(8)` threads
- Binary file detection (quits on NULL byte)
- File size limits (50MB default)
- Streaming results to avoid memory issues

## Files Modified

1. `packages/ccode/src/file/ripgrep.ts` - Added native implementations

## Expected Performance (with native bindings)

| Operation | rg binary | Native Rust | Improvement |
|-----------|-----------|-------------|-------------|
| files() | ~100ms | ~30ms | 3x |
| search() | ~200ms | ~80ms | 2.5x |

Performance improvements come from:
- No process spawn overhead
- Shared memory (no JSON serialization)
- Direct integration with Rust's parallel walker

## Benefits

1. **No external binary needed**: Falls back gracefully but prefers native
2. **Reduced latency**: No process spawn overhead
3. **Better error handling**: Direct Rust error propagation
4. **Memory efficiency**: Streaming without intermediate JSON parsing

## Verification

- ✅ TypeScript type check passes
- ✅ Maintains backward compatibility with existing API
- ✅ Graceful fallback to rg binary when native unavailable
