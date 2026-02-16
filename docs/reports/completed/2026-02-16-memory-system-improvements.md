# Memory System Improvements

**Date**: 2026-02-16
**Status**: Completed

## Overview

Implemented three improvements to the CodeCoder/ZeroBot memory system to address:
1. Ambiguous write routing across memory layers
2. Repeated loading overhead
3. Concurrent write risks to ZeroBot database

## Changes Made

### 1. ZeroBotMemoryProvider Default Read-Only

**File**: `packages/ccode/src/memory-zerobot/provider.ts`

Changed default `readOnly` from `false` to `true` to prevent accidental writes to ZeroBot's SQLite database.

Added two new methods:
- `isWritable()`: Check if provider is in writable mode
- `tryStore()`: Safe write attempt that returns false instead of throwing on read-only mode

```typescript
// Before
this.readOnly = config.readOnly ?? false

// After
this.readOnly = config.readOnly ?? true
```

### 2. MemoryRouter - Unified Write Entry Point

**New file**: `packages/ccode/src/agent/memory-router.ts`

Created a unified routing layer that directs writes to the correct storage based on data type:

| Type | Destination | Use Case |
|------|-------------|----------|
| `preference` | MEMORY.md/用户偏好 | Long-term user preferences |
| `decision` | MEMORY.md/关键决策 | Key decisions for audit |
| `lesson` | MEMORY.md/经验教训 | Knowledge sediment |
| `context` | MEMORY.md/项目上下文 | Project-specific context |
| `daily` | daily/*.md | Chronological log entries |
| `pattern` | preferences/patterns | Code pattern learning |

Key exports:
- `routeMemoryWrite(request)`: Main routing function
- `batchMemoryWrite(requests)`: Batch operations
- Helper functions: `writePreference()`, `writeDecision()`, `writeLesson()`, `writeDailyNote()`, `learnPattern()`

### 3. Memory Context Caching

**File**: `packages/ccode/src/agent/memory-bridge.ts`

Added TTL-based caching (30 seconds) to `buildMemoryContext()` to reduce repeated loading overhead:

```typescript
const CACHE_TTL_MS = 30_000

interface CachedMemoryContext {
  context: BridgedMemoryContext
  timestamp: number
  optionsHash: string
}

let memoryCache: CachedMemoryContext | null = null
```

New features:
- Cache automatically invalidates after 30 seconds
- Cache invalidates when options hash changes
- `invalidateMemoryCache()` for manual invalidation
- `skipCache: true` option to force fresh load

Cache is automatically invalidated when `routeMemoryWrite()` successfully writes to markdown layers.

## Files Modified

| File | Changes |
|------|---------|
| `packages/ccode/src/memory-zerobot/provider.ts` | Default read-only, new methods |
| `packages/ccode/src/memory-zerobot/types.ts` | Updated documentation |
| `packages/ccode/src/agent/memory-bridge.ts` | Added caching layer |
| `packages/ccode/src/agent/memory-router.ts` | **New file** - Routing layer |

## Verification

- TypeScript type check passes for all modified files
- Pre-existing type errors in `provider.ts` (unrelated) remain

## Usage Examples

### Writing memory through router

```typescript
import { routeMemoryWrite, writePreference } from "@/agent/memory-router"

// Using main function
await routeMemoryWrite({
  type: "preference",
  key: "lang",
  content: "Prefers Rust over Python"
})

// Using helper
await writePreference("editor", "Uses Neovim")
```

### Using cached context

```typescript
import { buildMemoryContext, invalidateMemoryCache } from "@/agent/memory-bridge"

// First call loads fresh
const context1 = await buildMemoryContext()

// Second call within 30s uses cache
const context2 = await buildMemoryContext()

// Force fresh load
const context3 = await buildMemoryContext({ skipCache: true })

// Manual invalidation
invalidateMemoryCache()
```

### ZeroBot memory (read-only by default)

```typescript
import { createZeroBotMemory } from "@/memory-zerobot"

const memory = createZeroBotMemory()  // Read-only by default

// Check writability
if (memory.isWritable()) {
  memory.store("key", "content", "core")
}

// Safe write (returns false if read-only)
const success = memory.tryStore("key", "content", "core")

// Explicit writable mode
const writableMemory = createZeroBotMemory({ readOnly: false })
```
