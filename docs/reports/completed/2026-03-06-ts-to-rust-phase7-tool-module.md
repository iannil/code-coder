# TypeScript to Rust Migration - Phase 7: Tool Module

**Date**: 2026-03-06
**Status**: ✅ Completed
**Lines Removed**: ~50 (328 → 278)

## Overview

Phase 7 focused on removing the TypeScript fallback parser from `bash.ts`. After exploration, the actual scope was significantly reduced from the original estimate (~1,250 lines) because most target files were already using native-only mode.

## Changes Made

### bash.ts (packages/ccode/src/tool/bash.ts)

**Before** (328 lines):
- Imported both `parseShellCommand` and `isNative` from `@codecoder-ai/core`
- Included a ~51 line `fallbackParseCommand` function using regex-based parsing
- Used conditional logic to choose between native and fallback parsers
- Logged warnings when falling back to regex parser

**After** (278 lines):
- Imports only `parseShellCommand` from `@codecoder-ai/core`
- Module-level fail-fast validation ensures native bindings are available
- Direct native parser calls without conditional checks
- Simplified `parseCommand` function (3 lines instead of 7)

### Specific Changes

1. **Removed `isNative` import** - No longer needed with fail-fast pattern

2. **Deleted `fallbackParseCommand` function** (51 lines)
   - Regex-based shell command parsing
   - Quote handling logic
   - Command tokenization

3. **Added fail-fast validation**
   ```typescript
   if (!nativeParseShellCommand) {
     throw new Error(
       "Native shell parser bindings not available. Ensure @codecoder-ai/core is built with 'bun run build' in packages/core",
     )
   }
   ```

4. **Simplified `parseCommand` function**
   - From conditional check + fallback (7 lines)
   - To direct native call (3 lines)

5. **Updated log statement**
   - Changed `nativeParser: isNative` to `nativeParser: true`

### TypeScript Pattern for Module-Level Validation

Used import alias + local const pattern for TypeScript type narrowing:
```typescript
import { parseShellCommand as nativeParseShellCommand } from "@codecoder-ai/core"

if (!nativeParseShellCommand) {
  throw new Error(...)
}

// Store validated reference for TypeScript type narrowing
const parseShellCommand = nativeParseShellCommand
```

## Files Not Modified (Already Native-Only)

| File | Status | Reason |
|------|--------|--------|
| `tool/edit.ts` | ✅ Native-only | Uses `replaceWithFuzzyMatchNative` |
| `patch/index.ts` | ✅ Native-only | Uses `createPatchApplicator` |
| `context/fingerprint.ts` | ✅ Native-only | Already fail-fast mode |
| `util/tech-fingerprints.ts` | ✅ Native-only | Uses `nativeDetect` |
| `util/java-fingerprints.ts` | ✅ Native-only | Uses `FingerprintEngineHandle` |
| `tool/scheduler.ts` | N/A | HTTP API, no native bindings |
| `tool/project.ts` | N/A | Project management CRUD |

## Verification

### Type Check
```bash
cd packages/ccode && bun run tsgo --noEmit 2>&1 | grep bash.ts
# No bash.ts errors
```

### Unit Tests
```bash
bun test test/tool/bash.test.ts
# 12 pass, 0 fail
```

All tests pass including:
- Basic command execution
- Multi-command parsing (`echo foo && echo bar`)
- External directory permissions
- Workdir permissions
- Auto-approval patterns
- Output truncation

## Summary

| Metric | Value |
|--------|-------|
| Files Modified | 1 |
| Lines Removed | ~50 |
| Tests Passing | 12/12 |
| Type Errors | 0 (for bash.ts) |

## Next Steps

Phase 7 is complete. The plan indicates Phase 8 (Autonomous Execution Logic) as the next priority:
- 8.1 Git Operations (git-ops.ts) - ~650 lines
- 8.2 Sandbox Isolation - ~1,280 lines
- 8.3 Safety Module - ~2,050 lines
- 8.4 Metrics Module - ~970 lines

---

**Cumulative Migration Progress**:
| Phase | Lines Removed | Status |
|-------|--------------|--------|
| 1-5 | ~3,012 | ✅ |
| 6 | ~361 | ✅ |
| 7 | ~50 | ✅ |
| **Total** | **~3,423** | |
