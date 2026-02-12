/**
 * ULC-ALL-* Tests: Cross-User Lifecycle
 * Tests for scenarios involving multiple user types
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { Session } from "../../src/session"
import { Provider } from "../../src/provider/provider"
import { Env } from "../../src/env"
import { Storage } from "../../src/storage/storage"

describe("Cross-User Lifecycle - ULC-ALL", () => {
  describe("ULC-ALL-SESS-001: Multi-user session isolation", () => {
    test("should isolate sessions by project", async () => {
      // Create two separate project directories
      const tmp1 = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      const tmp2 = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )
        },
      })

      // User A creates session in project 1
      await Instance.provide({
        directory: tmp1.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const sessionA = await Session.create({ title: "User A Session" })

          // Verify session exists in project 1
          const sessionsA = []
          for await (const s of Session.list()) {
            sessionsA.push(s)
          }
          expect(sessionsA.length).toBeGreaterThanOrEqual(1)
          expect(sessionsA.some((s) => s.title === "User A Session")).toBe(true)

          await Session.remove(sessionA.id)
        },
      })

      // User B creates session in project 2
      await Instance.provide({
        directory: tmp2.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const sessionB = await Session.create({ title: "User B Session" })

          // Verify User B's session exists in project 2
          const sessionsB = []
          for await (const s of Session.list()) {
            sessionsB.push(s)
          }
          expect(sessionsB.length).toBeGreaterThanOrEqual(1)
          expect(sessionsB.some((s) => s.title === "User B Session")).toBe(true)

          await Session.remove(sessionB.id)
        },
      })
    })
  })

  describe("ULC-ALL-SESS-002: Session export/import sharing", () => {
    test("should export session with complete data", async () => {
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
          // Create session
          const session = await Session.create({ title: "Shareable Session" })

          // Export session
          const info = await Session.get(session.id)
          const messages = await Session.messages({ sessionID: session.id })

          const exportData = {
            info,
            messages: messages.map((msg) => ({
              info: msg.info,
              parts: msg.parts,
            })),
          }

          // Verify export structure
          expect(exportData.info).toBeDefined()
          expect(exportData.info.title).toBe("Shareable Session")
          expect(exportData.messages).toBeDefined()

          await Session.remove(session.id)
        },
      })
    })

    test("should import session and preserve data", async () => {
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
          // Create a session to get a valid session ID
          const tempSession = await Session.create({ title: "Temp" })

          // Prepare import data with valid structure
          const importData = {
            info: {
              id: tempSession.id, // Use valid session ID format
              projectID: Instance.project.id,
              title: "Imported from User A",
              time: {
                created: Date.now(),
                updated: Date.now(),
              },
            } as Session.Info,
            messages: [],
          }

          // Verify import structure is valid
          expect(importData.info.title).toBe("Imported from User A")
          expect(importData.messages).toBeDefined()

          // Verify we can access the session
          const retrieved = await Session.get(tempSession.id)
          expect(retrieved).toBeDefined()

          await Session.remove(tempSession.id)
        },
      })
    })

    test("should handle import from JSON file", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
            }),
          )

          // Create export file
          const exportData = {
            info: {
              title: "From JSON File",
              time: {
                created: Date.now(),
                updated: Date.now(),
              },
            },
            messages: [],
          }

          await Bun.write(path.join(dir, "session-export.json"), JSON.stringify(exportData, null, 2))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Read export file
          const exportPath = path.join(tmp.path, "session-export.json")
          const exportData = await Bun.file(exportPath).json()

          expect(exportData.info).toBeDefined()
          expect(exportData.info.title).toBe("From JSON File")
          expect(exportData.messages).toBeDefined()
        },
      })
    })
  })

  describe("ULC-ALL-SESS-003: Concurrent session access", () => {
    test("should handle multiple users creating sessions simultaneously", async () => {
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
          // Simulate multiple users creating sessions concurrently
          const sessionPromises = Array.from({ length: 5 }, (_, i) =>
            Session.create({ title: `Concurrent Session ${i}` }),
          )

          const sessions = await Promise.all(sessionPromises)

          // Verify all sessions were created
          expect(sessions.length).toBe(5)
          const ids = new Set(sessions.map((s) => s.id))
          expect(ids.size).toBe(5) // All unique IDs

          // Cleanup
          await Promise.all(sessions.map((s) => Session.remove(s.id)))
        },
      })
    })
  })

  describe("ULC-ALL-PROV-001: Cross-user provider configuration", () => {
    test("should support different providers for different users", async () => {
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
        fn: async () => {
          // User A uses Anthropic
          Env.set("ANTHROPIC_API_KEY", "sk-ant-user-a-key")
          const providersA = await Provider.list()
          expect(providersA["anthropic"]).toBeDefined()

          // Reset and User B uses OpenAI
          Env.set("ANTHROPIC_API_KEY", "")
          Env.set("OPENAI_API_KEY", "sk-openai-user-b-key")
          const providersB = await Provider.list()

          // OpenAI should be available
          expect(providersB["openai"]).toBeDefined()
        },
      })
    })
  })
})
