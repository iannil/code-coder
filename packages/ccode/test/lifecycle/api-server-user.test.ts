/**
 * ULC-SU-* Tests: API/Server User Lifecycle
 * Tests for users accessing CodeCoder via HTTP API
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { Env } from "../../src/env"
import { LocalSession, LocalConfig, LocalPermission } from "../../src/api"
import { Session } from "../../src/session"
import { Permission } from "../../src/permission"
import { Identifier } from "../../src/id/id"

describe("API/Server User Lifecycle - ULC-SU", () => {
  describe("ULC-SU-API-001: API session management", () => {
    test("should list sessions via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create some sessions
          const s1 = await Session.create({ title: "API Session 1" })
          const s2 = await Session.create({ title: "API Session 2" })

          // List via API
          const sessions = await LocalSession.list({})
          expect(Array.isArray(sessions)).toBe(true)
          expect(sessions.length).toBeGreaterThanOrEqual(2)

          // Cleanup
          await Session.remove(s1.id)
          await Session.remove(s2.id)
        },
      })
    })

    test("should create session via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await LocalSession.create({ title: "Created via API" })
          expect(session).toBeDefined()
          expect(session.id).toBeDefined()
          expect(session.title).toBe("Created via API")

          await Session.remove(session.id)
        },
      })
    })

    test("should get session by ID via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const created = await LocalSession.create({ title: "Get Test" })
          const retrieved = await LocalSession.get(created.id)

          expect(retrieved).toBeDefined()
          expect(retrieved.id).toBe(created.id)
          expect(retrieved.title).toBe("Get Test")

          await Session.remove(created.id)
        },
      })
    })

    test("should remove session via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await LocalSession.create({ title: "To Be Removed" })
          const sessionId = session.id

          await LocalSession.remove(sessionId)

          // Should throw when trying to get removed session
          await expect(LocalSession.get(sessionId)).rejects.toThrow()
        },
      })
    })

    test("should list sessions with directory filter", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await LocalSession.create({ title: "Directory Filter Test" })

          // List with current directory filter
          const sessions = await LocalSession.list({ directory: tmp.path })
          expect(sessions.some((s) => s.id === session.id)).toBe(true)

          await Session.remove(session.id)
        },
      })
    })

    test("should list sessions with search filter", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await LocalSession.create({ title: "UniqueSearchTerm123" })

          // Search for the session
          const sessions = await LocalSession.list({ search: "UniqueSearchTerm" })
          expect(sessions.some((s) => s.id === session.id)).toBe(true)

          // Search for non-existent term
          const noResults = await LocalSession.list({ search: "NonExistentTerm999" })
          expect(noResults.some((s) => s.id === session.id)).toBe(false)

          await Session.remove(session.id)
        },
      })
    })

    test("should list sessions with limit", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create multiple sessions
          const sessions = await Promise.all([
            LocalSession.create({ title: "Limit Test 1" }),
            LocalSession.create({ title: "Limit Test 2" }),
            LocalSession.create({ title: "Limit Test 3" }),
          ])

          // List with limit
          const limited = await LocalSession.list({ limit: 2 })
          expect(limited.length).toBeLessThanOrEqual(2)

          // Cleanup
          await Promise.all(sessions.map((s) => Session.remove(s.id)))
        },
      })
    })
  })

  describe("ULC-SU-API-002: API session operations", () => {
    test("should get session messages via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await LocalSession.create({ title: "Messages Test" })

          const messages = await LocalSession.messages({ sessionID: session.id })
          expect(Array.isArray(messages)).toBe(true)

          await Session.remove(session.id)
        },
      })
    })

    test("should get session status via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const status = await LocalSession.status()
          expect(status).toBeDefined()
        },
      })
    })

    test("should fork session via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const original = await LocalSession.create({ title: "Original Session" })
          const forked = await LocalSession.fork({ sessionID: original.id })

          // Fork creates a new session with copied messages (but no parentID relationship)
          expect(forked).toBeDefined()
          expect(forked.id).not.toBe(original.id)
          expect(forked.title).toBeDefined()

          await Session.remove(forked.id)
          await Session.remove(original.id)
        },
      })
    })

    test("should get session children via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const parent = await LocalSession.create({ title: "Parent Session" })
          // Use create with parentID to establish parent-child relationship
          const child = await LocalSession.create({ title: "Child Session", parentID: parent.id })

          const children = await LocalSession.children(parent.id)
          expect(children.some((c) => c.id === child.id)).toBe(true)

          await Session.remove(child.id)
          await Session.remove(parent.id)
        },
      })
    })
  })

  describe("ULC-SU-API-003: API config operations", () => {
    test("should get config via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              theme: "dark",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await LocalConfig.get()
          expect(config).toBeDefined()
        },
      })
    })
  })

  describe("ULC-SU-API-004: API permission operations", () => {
    test("should list permissions via API", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const permissions = await LocalPermission.list()
          expect(permissions).toBeDefined()
        },
      })
    })
  })

  describe("ULC-SU-ERR-001: API error handling", () => {
    test("should handle non-existent session", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          await expect(LocalSession.get("ses_nonexistent123")).rejects.toThrow()
        },
      })
    })

    test("should handle invalid session ID format", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Invalid ID format (doesn't start with "ses_") throws ZodError during validation
          let threw = false
          try {
            await LocalSession.get("invalid-id")
          } catch (e) {
            threw = true
            expect(e).toBeDefined()
          }
          expect(threw).toBe(true)
        },
      })
    })

    test("should handle messages for non-existent session", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Messages for non-existent session returns empty array (doesn't throw)
          const messages = await LocalSession.messages({ sessionID: "ses_nonexistent123" })
          expect(Array.isArray(messages)).toBe(true)
          expect(messages.length).toBe(0)
        },
      })
    })

    test("should handle fork for non-existent session", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Fork from non-existent session creates a new empty session (doesn't throw)
          const forked = await LocalSession.fork({ sessionID: "ses_nonexistent123" })
          expect(forked).toBeDefined()
          expect(forked.id).toBeDefined()

          await Session.remove(forked.id)
        },
      })
    })
  })

  describe("ULC-SU-BATCH-001: API batch operations", () => {
    test("should handle concurrent session creates", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create multiple sessions concurrently
          const results = await Promise.all([
            LocalSession.create({ title: "Concurrent 1" }),
            LocalSession.create({ title: "Concurrent 2" }),
            LocalSession.create({ title: "Concurrent 3" }),
          ])

          expect(results.length).toBe(3)
          results.forEach((session) => {
            expect(session.id).toBeDefined()
          })

          // Cleanup
          await Promise.all(results.map((s) => Session.remove(s.id)))
        },
      })
    })

    test("should handle concurrent session operations", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await LocalSession.create({ title: "Concurrent Ops Test" })

          // Run multiple operations concurrently
          const [messages, status, retrieved] = await Promise.all([
            LocalSession.messages({ sessionID: session.id }),
            LocalSession.status(),
            LocalSession.get(session.id),
          ])

          expect(Array.isArray(messages)).toBe(true)
          expect(status).toBeDefined()
          expect(retrieved.id).toBe(session.id)

          await Session.remove(session.id)
        },
      })
    })
  })
})
