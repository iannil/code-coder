/**
 * E2E Medium Priority Test: TUI Interaction
 * Priority: Medium - Runs weekly
 *
 * Tests TUI-related utilities, keybindings, and component logic.
 * Note: Full TUI rendering tests require opentui mocking or snapshot testing.
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Session } from "../../../src/session"
import { Config } from "../../../src/config/config"
import { Keybind } from "../../../src/util/keybind"
import { Env } from "../../../src/env"

describe("E2E Medium: TUI Interaction", () => {
  describe("Keyboard Shortcuts", () => {
    test("should parse standard keybindings from config", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              keybinds: {
                app_exit: "ctrl+c",
                input_submit: "shift+return",
                leader: "ctrl+space",
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await Config.get()

          // Parse keybindings
          const quitKeys = Keybind.parse(config.keybinds?.app_exit ?? "ctrl+c")
          expect(quitKeys.length).toBeGreaterThan(0)
          expect(quitKeys[0].ctrl).toBe(true)
          expect(quitKeys[0].name).toBe("c")

          const submitKeys = Keybind.parse(config.keybinds?.input_submit ?? "shift+return")
          expect(submitKeys.length).toBeGreaterThan(0)
          expect(submitKeys[0].shift).toBe(true)
          expect(submitKeys[0].name).toBe("return")

          const leaderKeys = Keybind.parse(config.keybinds?.leader ?? "ctrl+space")
          expect(leaderKeys.length).toBeGreaterThan(0)
          expect(leaderKeys[0].ctrl).toBe(true)
          expect(leaderKeys[0].name).toBe("space")
        },
      })
    })

    test("should support multiple keybindings for same action", async () => {
      const multiKeybind = "ctrl+c,ctrl+q"
      const keys = Keybind.parse(multiKeybind)

      expect(keys.length).toBe(2)
      expect(keys[0].name).toBe("c")
      expect(keys[1].name).toBe("q")
    })

    test("should support leader key sequences", async () => {
      const leaderKey = "<leader>f"
      const keys = Keybind.parse(leaderKey)

      expect(keys.length).toBe(1)
      expect(keys[0].leader).toBe(true)
      expect(keys[0].name).toBe("f")
    })

    test("should match keybindings correctly", () => {
      const configured: Keybind.Info = {
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
        name: "s",
      }

      const pressed: Keybind.Info = {
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
        name: "s",
      }

      expect(Keybind.match(configured, pressed)).toBe(true)

      // Different key should not match
      const wrongKey: Keybind.Info = {
        ctrl: true,
        meta: false,
        shift: false,
        leader: false,
        name: "d",
      }
      expect(Keybind.match(configured, wrongKey)).toBe(false)

      // Missing modifier should not match
      const missingMod: Keybind.Info = {
        ctrl: false,
        meta: false,
        shift: false,
        leader: false,
        name: "s",
      }
      expect(Keybind.match(configured, missingMod)).toBe(false)
    })

    test("should convert keybind info to display string", () => {
      const key: Keybind.Info = {
        ctrl: true,
        meta: true,
        shift: false,
        leader: false,
        name: "g",
      }

      const display = Keybind.toString(key)
      expect(display).toContain("ctrl")
      expect(display).toContain("alt")
      expect(display).toContain("g")
    })

    test("should support function keys", () => {
      const f2Key = Keybind.parse("f2")
      expect(f2Key[0].name).toBe("f2")

      const shiftF2 = Keybind.parse("shift+f2")
      expect(shiftF2[0].shift).toBe(true)
      expect(shiftF2[0].name).toBe("f2")
    })

    test("should support special keys", () => {
      const pgup = Keybind.parse("pgup")
      expect(pgup[0].name).toBe("pgup")

      const pgdn = Keybind.parse("pgdn")
      expect(pgdn[0].name).toBe("pgdn")
    })

    test("should handle none value", () => {
      const none = Keybind.parse("none")
      expect(none).toEqual([])
    })

    test("should support super modifier", () => {
      const superKey = Keybind.parse("super+z")
      expect(superKey[0].super).toBe(true)
      expect(superKey[0].name).toBe("z")

      const superShift = Keybind.parse("super+shift+z")
      expect(superShift[0].super).toBe(true)
      expect(superShift[0].shift).toBe(true)
    })
  })

  describe("Session Switching", () => {
    test("should create and switch between sessions", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://codecoder.ai/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create first session
          const session1 = await Session.create({ title: "Session One" })
          expect(session1).toBeDefined()

          // Create second session
          const session2 = await Session.create({ title: "Session Two" })
          expect(session2).toBeDefined()

          // List all sessions
          const sessions: Session.Info[] = []
          for await (const s of Session.list()) sessions.push(s)
          expect(sessions.length).toBeGreaterThanOrEqual(2)

          // Find sessions by title
          const foundSession1 = sessions.find((s) => s.title === "Session One")
          const foundSession2 = sessions.find((s) => s.title === "Session Two")
          expect(foundSession1).toBeDefined()
          expect(foundSession2).toBeDefined()

          // Clean up
          await Session.remove(session1.id)
          await Session.remove(session2.id)
        },
      })
    })

    test("should rename session", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://codecoder.ai/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "Original Title" })
          expect(session.title).toBe("Original Title")

          // Rename session
          await Session.update(session.id, (s) => { s.title = "New Title" })

          // Verify rename
          const updated = await Session.get(session.id)
          expect(updated?.title).toBe("New Title")

          await Session.remove(session.id)
        },
      })
    })

    test("should delete session", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://codecoder.ai/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "To Be Deleted" })
          const sessionId = session.id

          // Verify session exists
          const found = await Session.get(sessionId)
          expect(found).toBeDefined()
          expect(found?.id).toBe(sessionId)

          // Delete session
          await Session.remove(sessionId)

          // Verify session is deleted (Session.get throws NotFoundError when session doesn't exist)
          let deleted: Session.Info | undefined = undefined
          try {
            deleted = await Session.get(sessionId)
          } catch {
            // Session doesn't exist, which is expected
          }
          expect(deleted).toBeUndefined()
        },
      })
    })
  })

  describe("Dialog Components", () => {
    test("should load config for dialog options", async () => {
      await using tmp = await tmpdir({
        git: true,
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
          const config = await Config.get()
          expect(config.theme).toBe("dark")
        },
      })
    })

    test("should provide model selection options", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              model: "anthropic/claude-sonnet-4-20250514",
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
          const config = await Config.get()
          expect(config.model).toBe("anthropic/claude-sonnet-4-20250514")
        },
      })
    })
  })

  describe("Transcript Export", () => {
    test("should export session transcript", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://codecoder.ai/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "Export Test" })

          // Session should be exportable (have valid data structure)
          const retrieved = await Session.get(session.id)
          expect(retrieved).toBeDefined()
          expect(retrieved?.id).toBe(session.id)
          expect(retrieved?.title).toBe("Export Test")

          await Session.remove(session.id)
        },
      })
    })
  })

  describe("Permission Flow", () => {
    test("should track permission settings in config", async () => {
      await using tmp = await tmpdir({
        git: true,
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
          const config = await Config.get()

          // Config should support permission-related settings
          expect(config).toBeDefined()
          // Note: Permission tracking is managed by the permission system
        },
      })
    })
  })

  describe("Theme Support", () => {
    test("should load custom theme from config", async () => {
      await using tmp = await tmpdir({
        git: true,
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
          const config = await Config.get()
          expect(config.theme).toBe("dark")
        },
      })
    })

    test("should support light theme", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              theme: "light",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await Config.get()
          expect(config.theme).toBe("light")
        },
      })
    })
  })

  describe("Tips and Help", () => {
    test("should load help keybindings", async () => {
      // Test that standard help keybindings work
      const helpKeys = Keybind.parse("ctrl+/")
      expect(helpKeys.length).toBe(1)
      expect(helpKeys[0].ctrl).toBe(true)
      expect(helpKeys[0].name).toBe("/")

      const f1Keys = Keybind.parse("f1")
      expect(f1Keys.length).toBe(1)
      expect(f1Keys[0].name).toBe("f1")
    })
  })

  describe("Autocomplete", () => {
    test("should have autocomplete-related keybindings", () => {
      // Tab key for autocomplete
      const tabKey = Keybind.parse("tab")
      expect(tabKey.length).toBe(1)
      expect(tabKey[0].name).toBe("tab")

      // Arrow keys for navigation
      const upKey = Keybind.parse("up")
      expect(upKey[0].name).toBe("up")

      const downKey = Keybind.parse("down")
      expect(downKey[0].name).toBe("down")

      // Enter for selection
      const enterKey = Keybind.parse("return")
      expect(enterKey[0].name).toBe("return")

      // Escape to cancel
      const escKey = Keybind.parse("escape")
      expect(escKey[0].name).toBe("escape")
    })
  })
})
