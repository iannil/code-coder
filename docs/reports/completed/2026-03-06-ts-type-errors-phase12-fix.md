# TypeScript Type Errors Fix - Phase 12 PTY Integration

**Date:** 2026-03-06
**Status:** Completed
**Errors Fixed:** 21 → 0

## Summary

Fixed 21 TypeScript type errors discovered after Phase 12 PTY integration. These errors resulted from API signature mismatches between TypeScript code and native Rust NAPI bindings.

## Changes Made

### 1. Audit Log (`src/audit/audit-log.ts`)
- **Error:** `AuditLogFallback` not exported from `@codecoder-ai/core`
- **Fix:** Removed non-existent `AuditLogFallback` from re-exports

### 2. Grep/Glob Tools (`src/file/ripgrep.ts`, `src/tool/grep.ts`, `bench/tool.bench.ts`)
- **Error:** API signature mismatch - native uses `grep(pattern, path, options?)` vs fallback uses `grep(options)`
- **Fix:** Cast functions through `unknown` to use correct native API signature
- **Pattern:**
  ```typescript
  const grepFn = nativeGrep as unknown as (pattern: string, path: string, options?: any) => Promise<any[]>
  const result = await grepFn(pattern, path, options)
  ```

### 3. Edit Tool (`src/tool/edit.ts`)
- **Error:** `replaceWithFuzzyMatch` takes 3 args, code passed 4
- **Fix:** Implemented `replaceAll` loop manually since native only supports single replacement

### 4. Provider Transform (`src/provider/transform.ts`)
- **Error:** Wrong parameter types for `transformMessagesNative`
- **Fix:** Simplified call to use native signature: `(provider: string, messages: any[]): any[]`

### 5. Memory Chunker (`src/memory/chunker.ts`)
- **Error:** Missing types and wrong function signatures
- **Fix:**
  - Removed non-existent type imports (`Chunk`, `ChunkerConfig`)
  - Updated `chunkText` call to single-arg signature
  - Added fallback for `estimateTokens`

### 6. Tech Fingerprints (`src/util/tech-fingerprints.ts`)
- **Error:** `detectWebTechnologies` expects `string`, not object
- **Fix:** Pass content string directly instead of input object

### 7. Java Fingerprints (`src/util/java-fingerprints.ts`)
- **Error:** `FingerprintEngineHandle.create` doesn't exist
- **Fix:** Created wrapper using function-based `detectJavaTechnologies` API

### 8. JAR Analyzer (`src/util/jar-analyzer.ts`)
- **Error:** `analyzeJar` takes 1 arg, not 2
- **Fix:** Removed `maxClasses` parameter (not supported by native API)

## Technical Notes

### Union Type Issue
The `grep` and `glob` functions export both native (positional args) and fallback (options object) signatures. TypeScript creates an intersection type (`string & GrepOptions`) that can't be satisfied.

**Solution:** Cast through `unknown` at call sites:
```typescript
const grepFn = nativeGrep as unknown as (pattern: string, path: string, options?: any) => Promise<any[]>
```

### Pre-existing Test Issues
The `edit-fuzzy-replace.test.ts` imports TypeScript replacer functions (`SimpleReplacer`, `LineTrimmedReplacer`, etc.) that were removed during migration. This is unrelated to the type errors fixed here.

## Verification

```bash
$ bun turbo typecheck --filter=ccode
# 0 errors, passes successfully
```

## Files Modified

| File | Changes |
|------|---------|
| `src/audit/audit-log.ts` | Removed invalid export |
| `src/file/ripgrep.ts` | Updated glob/grep API calls |
| `src/tool/grep.ts` | Updated grep API call |
| `src/tool/edit.ts` | Fixed replaceWithFuzzyMatch call |
| `src/provider/transform.ts` | Fixed transformMessages call |
| `src/memory/chunker.ts` | Fixed imports and chunkText call |
| `src/util/tech-fingerprints.ts` | Fixed detectWebTechnologies call |
| `src/util/java-fingerprints.ts` | Fixed FingerprintEngineHandle usage |
| `src/util/jar-analyzer.ts` | Fixed analyzeJar call |
| `bench/tool.bench.ts` | Updated grep benchmark call |
