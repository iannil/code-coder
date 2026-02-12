import { describe, test, expect } from "bun:test"
import { Hook } from "../../src/hook/hook"
import { Instance } from "../../src/project/instance"
import { tmpdir } from "../fixture/fixture"
import path from "path"

describe("Hook", () => {
  describe("Lifecycle", () => {
    test("should have expected lifecycle values", () => {
      expect(Hook.Lifecycle.enum.PreToolUse).toBe("PreToolUse")
      expect(Hook.Lifecycle.enum.PostToolUse).toBe("PostToolUse")
      expect(Hook.Lifecycle.enum.PreResponse).toBe("PreResponse")
      expect(Hook.Lifecycle.enum.Stop).toBe("Stop")
    })
  })

  describe("HooksConfig schema", () => {
    test("should validate a valid hooks config", () => {
      const config = {
        hooks: {
          PreToolUse: {
            test_hook: {
              pattern: "Edit",
              description: "Test hook",
              actions: [
                {
                  type: "scan" as const,
                  patterns: ["test.*pattern"],
                  message: "Test message",
                  block: false,
                },
              ],
            },
          },
        },
        settings: {
          enabled: true,
          blocking_mode: "interactive" as const,
          log_level: "info" as const,
        },
      }

      const result = Hook.HooksConfig.safeParse(config)
      expect(result.success).toBe(true)
    })

    test("should reject invalid action type", () => {
      const config = {
        hooks: {
          PreToolUse: {
            test_hook: {
              pattern: "Edit",
              actions: [
                {
                  type: "invalid_type",
                },
              ],
            },
          },
        },
      }

      const result = Hook.HooksConfig.safeParse(config)
      expect(result.success).toBe(false)
    })
  })

  describe("run", () => {
    test("should return not blocked when no hooks configured", async () => {
      await using tmp = await tmpdir({ git: true })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", oldString: "old", newString: "new" },
          })

          expect(result.blocked).toBe(false)
        },
      })
    })

    test("should load hooks from .ccode directory and block on pattern match", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  test_scan: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["sk_live_[a-zA-Z0-9]+"],
                        message: "Secret detected: {match}",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", oldString: "key", newString: "sk_live_abcdefghij1234567890" },
          })

          expect(result.blocked).toBe(true)
          expect(result.message).toContain("Secret detected")
        },
      })
    })

    test("should not block when content does not match pattern", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  test_scan: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["sk_live_[a-zA-Z0-9]+"],
                        message: "Secret detected",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", oldString: "old", newString: "safe_content" },
          })

          expect(result.blocked).toBe(false)
        },
      })
    })

    test("should skip hooks when tool pattern does not match", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  test_scan: {
                    pattern: "Bash",
                    actions: [
                      {
                        type: "scan",
                        patterns: [".*"],
                        message: "Always blocks",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: true },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts" },
          })

          expect(result.blocked).toBe(false)
        },
      })
    })

    test("should respect disabled settings", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  test_scan: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: [".*"],
                        message: "Always blocks",
                        block: true,
                      },
                    ],
                  },
                },
              },
              settings: { enabled: false },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", content: "anything" },
          })

          expect(result.blocked).toBe(false)
        },
      })
    })
  })
})
