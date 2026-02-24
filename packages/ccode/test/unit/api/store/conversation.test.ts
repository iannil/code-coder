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
