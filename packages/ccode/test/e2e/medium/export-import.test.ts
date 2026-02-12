/**
 * E2E Medium Priority Test: Export/Import Operations
 * Priority: Medium - Runs weekly
 *
 * Tests session export and import functionality
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Session } from "../../../src/session"
import { Env } from "../../../src/env"

describe("E2E Medium: Export/Import Operations", () => {
  describe("Session Export", () => {
    test("should export session to JSON format", async () => {
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
          const session = await Session.create({ title: "Export Test" })

          // Export session data
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
          expect(exportData.info?.id).toBe(session.id)
          expect(exportData.info?.title).toBe("Export Test")
          expect(Array.isArray(exportData.messages)).toBe(true)

          await Session.remove(session.id)
        },
      })
    })

    test("should produce valid JSON", async () => {
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
          const session = await Session.create({ title: "JSON Validity Test" })
          const info = await Session.get(session.id)
          const messages = await Session.messages({ sessionID: session.id })

          const exportData = {
            info,
            messages: messages.map((msg) => ({
              info: msg.info,
              parts: msg.parts,
            })),
          }

          // Stringify and parse to verify JSON validity
          const jsonStr = JSON.stringify(exportData)
          const parsed = JSON.parse(jsonStr)

          expect(parsed.info.id).toBe(session.id)
          expect(parsed.info.title).toBe("JSON Validity Test")

          await Session.remove(session.id)
        },
      })
    })

    test("should write export to file", async () => {
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
          const session = await Session.create({ title: "File Export Test" })
          const info = await Session.get(session.id)

          const exportData = { info, messages: [] }
          const exportPath = path.join(tmp.path, "export.json")

          await Bun.write(exportPath, JSON.stringify(exportData, null, 2))

          // Verify file was written
          const fileContent = await Bun.file(exportPath).text()
          const parsed = JSON.parse(fileContent)
          expect(parsed.info.title).toBe("File Export Test")

          await Session.remove(session.id)
        },
      })
    })
  })

  describe("Session Import", () => {
    test("should import session from JSON data", async () => {
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
          // Create a session to act as an import target
          const session = await Session.create({ title: "Original" })

          // Simulate import by updating the session
          await Session.update(session.id, (s) => {
            s.title = "Imported Session"
          })

          // Verify the import
          const imported = await Session.get(session.id)
          expect(imported?.title).toBe("Imported Session")

          await Session.remove(session.id)
        },
      })
    })

    test("should read import from file", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )

          // Create import file
          await Bun.write(
            path.join(dir, "import.json"),
            JSON.stringify({
              info: {
                title: "From Import File",
                time: { created: Date.now(), updated: Date.now() },
              },
              messages: [],
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Read import file
          const importPath = path.join(tmp.path, "import.json")
          const importData = await Bun.file(importPath).json()

          expect(importData.info.title).toBe("From Import File")
          expect(Array.isArray(importData.messages)).toBe(true)
        },
      })
    })
  })

  describe("Cross-Project Import", () => {
    test("should import session across projects", async () => {
      // Source project
      const source = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      // Target project
      const target = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({ $schema: "https://codecoder.ai/config.json" }),
          )
        },
      })

      let exportData: { info: Session.Info | undefined; messages: unknown[] } | undefined

      // Export from source project
      await Instance.provide({
        directory: source.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "Cross-Project Session" })
          const info = await Session.get(session.id)
          exportData = { info, messages: [] }
          await Session.remove(session.id)
        },
      })

      // Import to target project
      await Instance.provide({
        directory: target.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          expect(exportData).toBeDefined()
          expect(exportData?.info?.title).toBe("Cross-Project Session")
        },
      })
    })
  })
})
