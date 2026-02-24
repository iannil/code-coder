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
