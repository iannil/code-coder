/**
 * E2E Critical Test: Session Creation and Management
 * Priority: Critical - Runs on every commit
 *
 * Tests the core session creation and management flows
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Session } from "../../../src/session"
import { Env } from "../../../src/env"

describe("E2E Critical: Session Creation", () => {
  describe("Session Lifecycle", () => {
    test("should create a new session", async () => {
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
          const session = await Session.create({ title: "Test Session" })

          expect(session).toBeDefined()
          expect(session.id).toBeDefined()
          expect(session.title).toBe("Test Session")
          expect(session.projectID).toBeDefined()

          await Session.remove(session.id)
        },
      })
    })

    test("should persist session to storage", async () => {
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
          const session = await Session.create({ title: "Persistent Session" })

          // Retrieve session to verify persistence
          const retrieved = await Session.get(session.id)
          expect(retrieved).toBeDefined()
          expect(retrieved?.id).toBe(session.id)
          expect(retrieved?.title).toBe("Persistent Session")

          await Session.remove(session.id)
        },
      })
    })

    test("should list all sessions for project", async () => {
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
          const session1 = await Session.create({ title: "Session 1" })
          const session2 = await Session.create({ title: "Session 2" })
          const session3 = await Session.create({ title: "Session 3" })

          // List sessions
          const sessions = []
          for await (const s of Session.list()) {
            sessions.push(s)
          }

          expect(sessions.length).toBeGreaterThanOrEqual(3)
          expect(sessions.some((s) => s.title === "Session 1")).toBe(true)
          expect(sessions.some((s) => s.title === "Session 2")).toBe(true)
          expect(sessions.some((s) => s.title === "Session 3")).toBe(true)

          // Cleanup
          await Session.remove(session1.id)
          await Session.remove(session2.id)
          await Session.remove(session3.id)
        },
      })
    })

    test("should delete session", async () => {
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
          const session = await Session.create({ title: "To Be Deleted" })

          // Verify session exists
          const before = await Session.get(session.id)
          expect(before).toBeDefined()

          // Delete session
          await Session.remove(session.id)

          // Verify session is deleted (returns undefined, null, or throws)
          let after: Session.Info | undefined | null
          try {
            after = await Session.get(session.id)
          } catch {
            after = undefined
          }
          expect(after == null).toBe(true)
        },
      })
    })
  })

  describe("Session Update", () => {
    test("should update session title", async () => {
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
          const session = await Session.create({ title: "Original Title" })

          // Update title
          await Session.update(session.id, (s) => {
            s.title = "Updated Title"
          })

          // Verify update
          const updated = await Session.get(session.id)
          expect(updated?.title).toBe("Updated Title")

          await Session.remove(session.id)
        },
      })
    })

    test("should track session timestamps", async () => {
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
          const session = await Session.create({ title: "Timestamp Test" })

          const info = await Session.get(session.id)
          expect(info?.time.created).toBeDefined()
          expect(info?.time.updated).toBeDefined()
          expect(info?.time.created).toBeLessThanOrEqual(info!.time.updated)

          await Session.remove(session.id)
        },
      })
    })
  })

  describe("Session Messages", () => {
    test("should retrieve session messages", async () => {
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
          const session = await Session.create({ title: "Messages Test" })

          // Get messages (initially empty for new session)
          const messages = await Session.messages({ sessionID: session.id })
          expect(Array.isArray(messages)).toBe(true)

          await Session.remove(session.id)
        },
      })
    })

    test("should support message limit option", async () => {
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
          const session = await Session.create({ title: "Limit Test" })

          // Get messages with limit
          const messages = await Session.messages({
            sessionID: session.id,
            limit: 5,
          })
          expect(Array.isArray(messages)).toBe(true)
          expect(messages.length).toBeLessThanOrEqual(5)

          await Session.remove(session.id)
        },
      })
    })
  })

  describe("Session Isolation", () => {
    test("should isolate sessions between projects", async () => {
      // Create two separate project directories
      const tmp1 = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      const tmp2 = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      // Create session in project 1
      await Instance.provide({
        directory: tmp1.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "Project 1 Session" })
          expect(session.projectID).toBeDefined()
          await Session.remove(session.id)
        },
      })

      // Create session in project 2
      await Instance.provide({
        directory: tmp2.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "Project 2 Session" })
          expect(session.projectID).toBeDefined()

          // Sessions from project 1 should not be visible
          const sessions = []
          for await (const s of Session.list()) {
            sessions.push(s)
          }
          expect(sessions.every((s) => s.title !== "Project 1 Session")).toBe(true)

          await Session.remove(session.id)
        },
      })
    })

    test("should generate unique session IDs", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
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
          const promises = Array.from({ length: 10 }, (_, i) =>
            Session.create({ title: `Concurrent Session ${i}` }),
          )

          const sessions = await Promise.all(promises)
          const ids = new Set(sessions.map((s) => s.id))

          // All IDs should be unique
          expect(ids.size).toBe(10)

          // Cleanup
          await Promise.all(sessions.map((s) => Session.remove(s.id)))
        },
      })
    })
  })
})
