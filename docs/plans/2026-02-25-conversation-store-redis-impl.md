# Conversation Store Redis Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace in-memory `conversationToSession` Map with Redis-backed persistent storage for multi-instance deployment support.

**Architecture:** Create a `ConversationStore` module using ioredis that manages conversation-to-session mappings in Redis. The module exposes get/set/delete operations and handles connection lifecycle. chat.ts handlers are modified to use async ConversationStore calls instead of synchronous Map operations.

**Tech Stack:** TypeScript, ioredis, Bun, Zod (for config schema)

---

## Task 1: Add ioredis Dependency

**Files:**
- Modify: `packages/ccode/package.json`

**Step 1: Add ioredis to dependencies**

```bash
cd packages/ccode && bun add ioredis
```

**Step 2: Add @types/ioredis (if needed)**

Note: ioredis includes TypeScript types, so this may not be needed. Verify with:

```bash
cd packages/ccode && bun run typecheck
```

Expected: No type errors related to ioredis

**Step 3: Commit**

```bash
git add packages/ccode/package.json bun.lockb
git commit -m "chore: add ioredis dependency for conversation store"
```

---

## Task 2: Add Redis Config Schema

**Files:**
- Modify: `packages/ccode/src/config/config.ts`

**Step 1: Write failing type test**

Create a temporary test to verify the schema will work:

```typescript
// In config.ts, after existing schemas, add:
export const RedisConfig = z
  .object({
    url: z.string().default("redis://localhost:6379").describe("Redis connection URL"),
    password: z.string().optional().describe("Redis password"),
    db: z.number().int().min(0).max(15).default(0).describe("Redis database number"),
    keyPrefix: z.string().default("codecoder:").describe("Key prefix for all Redis keys"),
    connectTimeout: z.number().int().positive().default(5000).describe("Connection timeout in ms"),
    commandTimeout: z.number().int().positive().default(3000).describe("Command timeout in ms"),
    maxRetriesPerRequest: z.number().int().min(0).default(3).describe("Max retries per request"),
  })
  .strict()
  .optional()
  .describe("Redis configuration for conversation store")
export type RedisConfig = z.infer<typeof RedisConfig>
```

**Step 2: Add redis to Info schema**

Find the `Info` schema (around line 400+) and add:

```typescript
redis: RedisConfig,
```

**Step 3: Verify typecheck passes**

```bash
cd packages/ccode && bun run typecheck
```

Expected: PASS

**Step 4: Commit**

```bash
git add packages/ccode/src/config/config.ts
git commit -m "feat(config): add Redis configuration schema"
```

---

## Task 3: Create ConversationStore Module (Interface Only)

**Files:**
- Create: `packages/ccode/src/api/server/store/conversation.ts`

**Step 1: Create the store directory**

```bash
mkdir -p packages/ccode/src/api/server/store
```

**Step 2: Write the module with stub implementations**

```typescript
/**
 * Conversation Store
 *
 * Redis-backed storage for conversation_id to session_id mappings.
 * Enables multi-instance deployment by sharing conversation state.
 */

import Redis from "ioredis"
import { Config } from "@/config/config"
import { Log } from "@/util/log"

const log = Log.create({ service: "conversation-store" })

export namespace ConversationStore {
  const KEY_PREFIX = "conv:"
  let client: Redis | null = null
  let keyPrefix = "codecoder:"

  /**
   * Initialize Redis connection.
   * Must be called before using other methods.
   */
  export async function init(redisUrl?: string): Promise<void> {
    if (client) {
      log.warn("ConversationStore already initialized")
      return
    }

    const config = await Config.get()
    const redisConfig = config.redis ?? {}

    const url = redisUrl
      ?? process.env.REDIS_URL
      ?? redisConfig.url
      ?? "redis://localhost:6379"

    keyPrefix = redisConfig.keyPrefix ?? "codecoder:"

    client = new Redis(url, {
      password: process.env.REDIS_PASSWORD ?? redisConfig.password,
      db: redisConfig.db ?? 0,
      connectTimeout: redisConfig.connectTimeout ?? 5000,
      commandTimeout: redisConfig.commandTimeout ?? 3000,
      maxRetriesPerRequest: redisConfig.maxRetriesPerRequest ?? 3,
      lazyConnect: true,
    })

    // Connect and verify
    await client.connect()
    await client.ping()

    log.info("ConversationStore initialized", { url: url.replace(/\/\/.*@/, "//***@") })
  }

  /**
   * Get session_id for a conversation.
   * Returns null if not found.
   */
  export async function get(conversationId: string): Promise<string | null> {
    if (!client) throw new Error("ConversationStore not initialized")
    return client.get(`${keyPrefix}${KEY_PREFIX}${conversationId}`)
  }

  /**
   * Set mapping from conversation_id to session_id.
   */
  export async function set(conversationId: string, sessionId: string): Promise<void> {
    if (!client) throw new Error("ConversationStore not initialized")
    await client.set(`${keyPrefix}${KEY_PREFIX}${conversationId}`, sessionId)
  }

  /**
   * Delete a conversation mapping.
   * Returns true if key existed, false otherwise.
   */
  export async function delete_(conversationId: string): Promise<boolean> {
    if (!client) throw new Error("ConversationStore not initialized")
    const result = await client.del(`${keyPrefix}${KEY_PREFIX}${conversationId}`)
    return result > 0
  }

  /**
   * Check if a conversation mapping exists.
   */
  export async function exists(conversationId: string): Promise<boolean> {
    if (!client) throw new Error("ConversationStore not initialized")
    const result = await client.exists(`${keyPrefix}${KEY_PREFIX}${conversationId}`)
    return result > 0
  }

  /**
   * Health check - verify Redis connection is alive.
   */
  export async function healthCheck(): Promise<boolean> {
    if (!client) return false
    try {
      const result = await client.ping()
      return result === "PONG"
    } catch {
      return false
    }
  }

  /**
   * Close Redis connection.
   * Call during graceful shutdown.
   */
  export async function close(): Promise<void> {
    if (client) {
      await client.quit()
      client = null
      log.info("ConversationStore closed")
    }
  }

  /**
   * Check if store is initialized.
   */
  export function isInitialized(): boolean {
    return client !== null
  }
}
```

**Step 3: Verify typecheck passes**

```bash
cd packages/ccode && bun run typecheck
```

Expected: PASS

**Step 4: Commit**

```bash
git add packages/ccode/src/api/server/store/conversation.ts
git commit -m "feat: add ConversationStore module for Redis-backed conversation mappings"
```

---

## Task 4: Write Unit Tests for ConversationStore

**Files:**
- Create: `packages/ccode/test/unit/api/store/conversation.test.ts`

**Step 1: Create the test directory**

```bash
mkdir -p packages/ccode/test/unit/api/store
```

**Step 2: Write the test file**

```typescript
import { describe, test, expect, beforeAll, afterAll, beforeEach } from "bun:test"
import { ConversationStore } from "@/api/server/store/conversation"

describe("ConversationStore", () => {
  const TEST_REDIS_URL = process.env.TEST_REDIS_URL ?? "redis://localhost:6379"

  beforeAll(async () => {
    await ConversationStore.init(TEST_REDIS_URL)
  })

  afterAll(async () => {
    await ConversationStore.close()
  })

  beforeEach(async () => {
    // Clean up test keys before each test
    await ConversationStore.delete_("test:telegram:123")
    await ConversationStore.delete_("test:slack:456")
    await ConversationStore.delete_("test:discord:789")
  })

  test("isInitialized returns true after init", () => {
    expect(ConversationStore.isInitialized()).toBe(true)
  })

  test("healthCheck returns true", async () => {
    const result = await ConversationStore.healthCheck()
    expect(result).toBe(true)
  })

  test("get returns null for non-existent key", async () => {
    const result = await ConversationStore.get("test:nonexistent:key")
    expect(result).toBeNull()
  })

  test("set and get work correctly", async () => {
    await ConversationStore.set("test:telegram:123", "sess_abc123")
    const result = await ConversationStore.get("test:telegram:123")
    expect(result).toBe("sess_abc123")
  })

  test("set overwrites existing value", async () => {
    await ConversationStore.set("test:telegram:123", "sess_old")
    await ConversationStore.set("test:telegram:123", "sess_new")
    const result = await ConversationStore.get("test:telegram:123")
    expect(result).toBe("sess_new")
  })

  test("delete_ returns true for existing key", async () => {
    await ConversationStore.set("test:slack:456", "sess_xyz")
    const result = await ConversationStore.delete_("test:slack:456")
    expect(result).toBe(true)
  })

  test("delete_ returns false for non-existent key", async () => {
    const result = await ConversationStore.delete_("test:nonexistent:key")
    expect(result).toBe(false)
  })

  test("get returns null after delete", async () => {
    await ConversationStore.set("test:discord:789", "sess_qwe")
    await ConversationStore.delete_("test:discord:789")
    const result = await ConversationStore.get("test:discord:789")
    expect(result).toBeNull()
  })

  test("exists returns true for existing key", async () => {
    await ConversationStore.set("test:telegram:123", "sess_exists")
    const result = await ConversationStore.exists("test:telegram:123")
    expect(result).toBe(true)
  })

  test("exists returns false for non-existent key", async () => {
    const result = await ConversationStore.exists("test:nonexistent:key")
    expect(result).toBe(false)
  })
})
```

**Step 3: Run tests (requires Redis running)**

```bash
cd packages/ccode && bun test test/unit/api/store/conversation.test.ts
```

Expected: All tests PASS (if Redis is running on localhost:6379)

**Step 4: Commit**

```bash
git add packages/ccode/test/unit/api/store/conversation.test.ts
git commit -m "test: add unit tests for ConversationStore"
```

---

## Task 5: Initialize ConversationStore on Server Start

**Files:**
- Modify: `packages/ccode/src/api/server/index.ts`

**Step 1: Add import**

At the top of the file, add:

```typescript
import { ConversationStore } from "./store/conversation"
```

**Step 2: Initialize in start function**

In the `start` function, after `await registerRoutes()` (around line 226), add:

```typescript
  // Initialize ConversationStore (Redis)
  try {
    await ConversationStore.init()
  } catch (error) {
    console.error(
      JSON.stringify({
        timestamp: new Date().toISOString(),
        event: "redis_init_failed",
        error: error instanceof Error ? error.message : String(error),
      }),
    )
    // Continue without Redis - chat will create new sessions each time
    console.log(
      JSON.stringify({
        timestamp: new Date().toISOString(),
        event: "redis_degraded_mode",
        message: "Conversation persistence disabled",
      }),
    )
  }
```

**Step 3: Close in shutdown handler**

In the `shutdown` function (around line 50), before `process.exit(0)`, add:

```typescript
    await ConversationStore.close()
```

**Step 4: Verify typecheck passes**

```bash
cd packages/ccode && bun run typecheck
```

Expected: PASS

**Step 5: Commit**

```bash
git add packages/ccode/src/api/server/index.ts
git commit -m "feat(server): initialize ConversationStore on startup"
```

---

## Task 6: Modify chat.ts to Use ConversationStore

**Files:**
- Modify: `packages/ccode/src/api/server/handlers/chat.ts`

**Step 1: Add import**

At the top of the file, add:

```typescript
import { ConversationStore } from "../store/conversation"
```

**Step 2: Remove the in-memory Map**

Find and delete this line (around line 137):

```typescript
const conversationToSession = new Map<string, string>()
```

**Step 3: Rewrite getOrCreateSession function**

Replace the entire `getOrCreateSession` function with:

```typescript
async function getOrCreateSession(conversationId: string | undefined): Promise<string> {
  const { LocalSession } = await import("../../../api")

  // If we have a conversation_id, check Redis for existing session
  if (conversationId && ConversationStore.isInitialized()) {
    try {
      const existingSessionId = await ConversationStore.get(conversationId)
      if (existingSessionId) {
        // Verify session still exists
        try {
          await LocalSession.get(existingSessionId)
          return existingSessionId
        } catch {
          // Session doesn't exist anymore, delete stale mapping
          await ConversationStore.delete_(conversationId)
        }
      }
    } catch (redisError) {
      // Redis unavailable - log and continue to create new session
      console.error(
        JSON.stringify({
          timestamp: new Date().toISOString(),
          event: "redis_error",
          function: "getOrCreateSession",
          error: redisError instanceof Error ? redisError.message : String(redisError),
        }),
      )
    }
  }

  // Create a new session
  const session = await LocalSession.create({
    title: `Chat: ${new Date().toISOString()}`,
  })

  // Map conversation_id if provided and Redis is available
  if (conversationId && ConversationStore.isInitialized()) {
    try {
      await ConversationStore.set(conversationId, session.id)
    } catch (redisError) {
      console.error(
        JSON.stringify({
          timestamp: new Date().toISOString(),
          event: "redis_error",
          function: "getOrCreateSession.set",
          error: redisError instanceof Error ? redisError.message : String(redisError),
        }),
      )
    }
  }

  return session.id
}
```

**Step 4: Update clearConversation function**

Find the `clearConversation` function and replace the Map operations. Change:

```typescript
    const hadMapping = conversationToSession.has(input.conversation_id)
    conversationToSession.delete(input.conversation_id)
```

To:

```typescript
    let hadMapping = false
    if (ConversationStore.isInitialized()) {
      try {
        hadMapping = await ConversationStore.delete_(input.conversation_id)
      } catch (redisError) {
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "redis_error",
            function: "clearConversation",
            error: redisError instanceof Error ? redisError.message : String(redisError),
          }),
        )
      }
    }
```

**Step 5: Update compactConversation function**

Find the `compactConversation` function and replace Map operations. Change:

```typescript
    const sessionId = conversationToSession.get(input.conversation_id)
```

To:

```typescript
    let sessionId: string | null = null
    if (ConversationStore.isInitialized()) {
      try {
        sessionId = await ConversationStore.get(input.conversation_id)
      } catch (redisError) {
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "redis_error",
            function: "compactConversation.get",
            error: redisError instanceof Error ? redisError.message : String(redisError),
          }),
        )
      }
    }
```

And change:

```typescript
    conversationToSession.set(input.conversation_id, newSession.id)
```

To:

```typescript
    if (ConversationStore.isInitialized()) {
      try {
        await ConversationStore.set(input.conversation_id, newSession.id)
      } catch (redisError) {
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "redis_error",
            function: "compactConversation.set",
            error: redisError instanceof Error ? redisError.message : String(redisError),
          }),
        )
      }
    }
```

**Step 6: Verify typecheck passes**

```bash
cd packages/ccode && bun run typecheck
```

Expected: PASS

**Step 7: Commit**

```bash
git add packages/ccode/src/api/server/handlers/chat.ts
git commit -m "feat(chat): use Redis-backed ConversationStore for session mappings"
```

---

## Task 7: Write Integration Test

**Files:**
- Create: `packages/ccode/test/integration/chat-redis.test.ts`

**Step 1: Create the test file**

```typescript
import { describe, test, expect, beforeAll, afterAll } from "bun:test"
import { ConversationStore } from "@/api/server/store/conversation"

describe("Chat Redis Integration", () => {
  const TEST_REDIS_URL = process.env.TEST_REDIS_URL ?? "redis://localhost:6379"

  beforeAll(async () => {
    await ConversationStore.init(TEST_REDIS_URL)
  })

  afterAll(async () => {
    // Clean up test data
    await ConversationStore.delete_("integration:telegram:user1")
    await ConversationStore.delete_("integration:slack:user2")
    await ConversationStore.close()
  })

  test("conversation mapping persists across get/set", async () => {
    const conversationId = "integration:telegram:user1"
    const sessionId = "sess_integration_test_1"

    // Set mapping
    await ConversationStore.set(conversationId, sessionId)

    // Verify it persists
    const result = await ConversationStore.get(conversationId)
    expect(result).toBe(sessionId)
  })

  test("clear removes mapping", async () => {
    const conversationId = "integration:slack:user2"
    const sessionId = "sess_integration_test_2"

    // Set mapping
    await ConversationStore.set(conversationId, sessionId)

    // Clear it
    const deleted = await ConversationStore.delete_(conversationId)
    expect(deleted).toBe(true)

    // Verify it's gone
    const result = await ConversationStore.get(conversationId)
    expect(result).toBeNull()
  })

  test("compact updates mapping to new session", async () => {
    const conversationId = "integration:telegram:user1"
    const oldSessionId = "sess_old"
    const newSessionId = "sess_new_compacted"

    // Set old mapping
    await ConversationStore.set(conversationId, oldSessionId)

    // Compact (update to new session)
    await ConversationStore.set(conversationId, newSessionId)

    // Verify new mapping
    const result = await ConversationStore.get(conversationId)
    expect(result).toBe(newSessionId)
  })
})
```

**Step 2: Run integration test**

```bash
cd packages/ccode && bun test test/integration/chat-redis.test.ts
```

Expected: All tests PASS (if Redis is running)

**Step 3: Commit**

```bash
git add packages/ccode/test/integration/chat-redis.test.ts
git commit -m "test: add integration tests for chat Redis functionality"
```

---

## Task 8: Final Verification and Documentation

**Step 1: Run full test suite**

```bash
cd packages/ccode && bun test
```

Expected: All tests PASS

**Step 2: Run typecheck**

```bash
cd packages/ccode && bun run typecheck
```

Expected: PASS

**Step 3: Test manually with Redis**

Start Redis (if not running):
```bash
docker run -d -p 6379:6379 redis:alpine
```

Start the server:
```bash
cd packages/ccode && bun dev serve
```

Test the chat endpoint:
```bash
curl -X POST http://localhost:4400/api/v1/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello", "user_id": "test", "channel": "test", "conversation_id": "test:manual:1"}'
```

Verify in Redis:
```bash
docker exec -it <redis-container> redis-cli
> GET codecoder:conv:test:manual:1
```

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete Redis-backed conversation store implementation

- Add ioredis dependency
- Add Redis config schema to config.ts
- Create ConversationStore module
- Update chat.ts handlers to use Redis
- Add unit and integration tests
- Support graceful degradation when Redis unavailable

Closes: conversation persistence for multi-instance deployment"
```

---

## Summary

| Task | Files | Estimated Steps |
|------|-------|-----------------|
| 1. Add ioredis | package.json | 3 |
| 2. Redis config schema | config.ts | 4 |
| 3. ConversationStore module | store/conversation.ts | 4 |
| 4. Unit tests | test/unit/api/store/conversation.test.ts | 4 |
| 5. Server init | index.ts | 5 |
| 6. chat.ts changes | handlers/chat.ts | 7 |
| 7. Integration test | test/integration/chat-redis.test.ts | 3 |
| 8. Final verification | - | 4 |

**Total: 8 tasks, 34 steps**
