# Rust Migration Phase 5: Session Message Compaction

## Status: Completed

**Date**: 2026-03-07

## Overview

Phase 5 migrates the prune logic from TypeScript to Rust while preserving the LLM-dependent summarization in TypeScript. This follows the project's core principle: high determinism → Rust, high uncertainty → TypeScript.

## Changes Summary

### Rust (services/zero-core)

#### session/compaction.rs

Added new types for prune computation:

- `PruneConfig` - Configuration for pruning (minimum, protect threshold, protected tools)
- `PartReference` - Reference to a message part to be pruned
- `PrunePlan` - Result of prune planning (parts to prune, should_execute flag)
- `ModelLimit` - Model token limits (context, output, input)
- `TokenUsage` - Current token usage (input, output, cache_read, cache_write)
- `ToolPartInfo` - Tool part info for prune computation
- `MessageInfo` - Message info with turn tracking

Added new methods to `Compactor`:

- `is_overflow()` - Check if token usage overflows model's context limit
- `compute_prune_plan()` - Compute which tool parts should be marked as compacted
- `compute_prune_plan_with_turns()` - Prune plan with message-level turn tracking

Added tests:

- `test_is_overflow` - Token overflow detection
- `test_is_overflow_zero_context` - Zero context limit handling
- `test_compute_prune_plan` - Basic prune plan computation
- `test_prune_protected_tools` - Protected tools are not pruned
- `test_prune_stops_at_compacted` - Stops at already compacted parts

#### session/mod.rs

Updated re-exports to include new types:
`MessageInfo`, `ModelLimit`, `PartReference`, `PruneConfig`, `PrunePlan`, `TokenUsage`, `ToolPartInfo`

#### napi/bindings.rs

Added NAPI types:

- `NapiPruneConfig`
- `NapiPartReference`
- `NapiPrunePlan`
- `NapiModelLimit`
- `NapiTokenUsage`
- `NapiToolPartInfo`
- `NapiMessageInfo`

Added NAPI methods to `CompactorHandle`:

- `isOverflow(tokens, limit)` - Check overflow
- `computePrunePlan(toolParts, config)` - Compute prune plan
- `computePrunePlanWithTurns(messages, config)` - Prune with turn tracking
- `estimateBatchTokens(texts)` - Batch token estimation

Added standalone NAPI functions:

- `isOverflow(tokens, limit)`
- `computePrunePlan(toolParts, config)`
- `computePrunePlanWithTurns(messages, config)`
- `createDefaultPruneConfig()`

### TypeScript (packages/core)

#### binding.d.ts

Added type declarations:

- `NapiPruneConfig`
- `NapiPartReference`
- `NapiPrunePlan`
- `NapiModelLimit`
- `NapiTokenUsage`
- `NapiToolPartInfo`
- `NapiMessageInfo`

Updated `CompactorHandle` with new methods.

Added function declarations:

- `isOverflow()`
- `computePrunePlan()`
- `computePrunePlanWithTurns()`
- `createDefaultPruneConfig()`

#### index.ts

Added exports:

- `isOverflow`
- `computePrunePlan`
- `computePrunePlanWithTurns`
- `createDefaultPruneConfig`
- Type exports for all prune types

### TypeScript (packages/ccode)

#### session/compaction.ts

Refactored `isOverflow()`:

- Uses native `isOverflow()` when available
- Falls back to TypeScript implementation

Refactored `prune()`:

- Converts messages to `NapiMessageInfo[]` format
- Uses native `computePrunePlanWithTurns()` when available
- Applies prune plan by updating parts
- Falls back to TypeScript implementation

Added helper function:

- `toNativeMessageInfo()` - Convert messages to native format

## Verification

```bash
# Rust compilation
cd services/zero-core && cargo check
# Result: Compiled successfully (8 warnings, 0 errors)

# TypeScript typecheck
bun turbo typecheck --filter="@codecoder-ai/*"
# Result: 4/4 packages passed

# ccode package
cd packages/ccode && bun run tsc --noEmit
# Result: No errors
```

## Architecture Decision

**Kept in TypeScript** (LLM-dependent):

- `process()` - Needs to call LLM API for summarization
- `create()` - Database operations, Bus events

**Moved to Rust** (deterministic computation):

- `isOverflow()` - Token usage calculation
- `prune()` planning - Iterate backwards, count tokens, mark parts

## Benefits

| Metric | Improvement |
|--------|-------------|
| Prune computation | O(N) Rust vs O(N) JS - faster iteration |
| Overflow check | Native calculation, no JS overhead |
| Token estimation | Batch processing in Rust |
| Code maintenance | Single source of truth in Rust |

## Files Changed

| File | Operation |
|------|-----------|
| `services/zero-core/src/session/compaction.rs` | Extended with prune types and methods |
| `services/zero-core/src/session/mod.rs` | Updated re-exports |
| `services/zero-core/src/napi/bindings.rs` | Added prune NAPI bindings |
| `packages/core/src/binding.d.ts` | Added type declarations |
| `packages/core/src/index.ts` | Added exports |
| `packages/ccode/src/session/compaction.ts` | Refactored to use native methods |

## Next Steps

Phase 6: 重复实现消除 (Eliminate duplicate implementations)

- Unify Fingerprint detection to Rust FingerprintEngineHandle
- JSON Schema validation migration to ConfigLoaderHandle.validateSchema()
