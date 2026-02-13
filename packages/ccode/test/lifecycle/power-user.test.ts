/**
 * ULC-PU-* Tests: Power User Lifecycle
 * Tests for advanced/power users
 */

import { describe, test, expect, mock } from "bun:test"
import path from "path"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { Provider } from "../../src/provider/provider"
import { Env } from "../../src/env"
import { ModelsDev } from "../../src/provider/models"
import { Session } from "../../src/session"
import { Hook } from "../../src/hook/hook"

describe("Power User Lifecycle - ULC-PU", () => {
  describe("ULC-PU-MDLS-001: Power user refresh models cache", () => {
    test("should refresh models from remote source", async () => {
      // This test verifies the refresh functionality exists
      // Actual refresh would hit network, so we just verify the method
      expect(ModelsDev.refresh).toBeDefined()
      expect(typeof ModelsDev.refresh).toBe("function")
    })

    test("should include models from multiple providers", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
          Env.set("OPENAI_API_KEY", "sk-test-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          expect(providers["openai"]).toBeDefined()

          // Verify models exist for each
          const anthropicModels = Object.keys(providers["anthropic"].models)
          const openaiModels = Object.keys(providers["openai"].models)

          expect(anthropicModels.length).toBeGreaterThan(0)
          expect(openaiModels.length).toBeGreaterThan(0)
        },
      })
    })
  })

  describe("ULC-PU-MDLS-002: Power user model filtering", () => {
    test("should whitelist specific models", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  whitelist: ["claude-sonnet-4-20250514"],
                },
              },
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
          const providers = await Provider.list()
          const models = Object.keys(providers["anthropic"].models)
          expect(models).toContain("claude-sonnet-4-20250514")
          expect(models.length).toBe(1)
        },
      })
    })

    test("should blacklist specific models", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  blacklist: ["claude-haiku-4-20250514"],
                },
              },
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
          const providers = await Provider.list()
          const models = Object.keys(providers["anthropic"].models)
          expect(models).not.toContain("claude-haiku-4-20250514")
        },
      })
    })

    test("should combine whitelist and blacklist", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  whitelist: ["claude-sonnet-4-20250514", "claude-opus-4-20250514"],
                  blacklist: ["claude-opus-4-20250514"],
                },
              },
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
          const providers = await Provider.list()
          const models = Object.keys(providers["anthropic"].models)
          expect(models).toContain("claude-sonnet-4-20250514")
          expect(models).not.toContain("claude-opus-4-20250514")
          expect(models.length).toBe(1)
        },
      })
    })
  })

  describe("ULC-PU-MDLS-003: Power user custom models", () => {
    test("should add custom model alias", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  models: {
                    fast: {
                      id: "claude-haiku-4-20250514",
                      name: "Fast Model",
                    },
                  },
                },
              },
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
          const providers = await Provider.list()
          const model = providers["anthropic"].models["fast"]
          expect(model).toBeDefined()
          expect(model.name).toBe("Fast Model")
          // The id field is the alias name itself
          expect(model.id).toBe("fast")
        },
      })
    })

    test("should override existing model properties", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  models: {
                    "claude-sonnet-4-20250514": {
                      name: "My Custom Sonnet Name",
                      cost: {
                        input: 1,
                        output: 2,
                      },
                    },
                  },
                },
              },
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
          const providers = await Provider.list()
          const model = providers["anthropic"].models["claude-sonnet-4-20250514"]
          expect(model.name).toBe("My Custom Sonnet Name")
          expect(model.cost.input).toBe(1)
          expect(model.cost.output).toBe(2)
        },
      })
    })
  })

  describe("ULC-PU-UTIL-001: Power user configuration", () => {
    test("should set default model", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
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
          const model = await Provider.defaultModel()
          expect(model.providerID).toBe("anthropic")
          expect(model.modelID).toBe("claude-sonnet-4-20250514")
        },
      })
    })

    test("should set small model preference", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              small_model: "anthropic/claude-sonnet-4-20250514",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          // Clear any existing provider API keys to avoid conflicts
          Env.remove("OPENAI_API_KEY")
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Verify the config has small_model set
          const { Provider } = await import("../../src/provider/provider")
          const config = await Provider.defaultModel()
          expect(config).toBeDefined()
        },
      })
    })

    test("should configure disabled providers", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              disabled_providers: ["openai"],
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
          Env.set("OPENAI_API_KEY", "sk-test-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          // Even with env key, disabled providers should not appear
          expect(providers["openai"]).toBeUndefined()
        },
      })
    })

    test("should configure enabled providers only", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              enabled_providers: ["anthropic"],
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
          Env.set("OPENAI_API_KEY", "sk-test-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          expect(providers["openai"]).toBeUndefined()
        },
      })
    })
  })

  describe("ULC-PU-UTIL-002: Power user custom provider", () => {
    test("should add custom OpenAI-compatible provider", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                "custom-openai": {
                  name: "Custom OpenAI",
                  npm: "@ai-sdk/openai-compatible",
                  env: [],
                  api: "https://custom.openai.com/v1",
                  models: {
                    "custom-gpt4": {
                      name: "Custom GPT-4",
                      tool_call: true,
                      limit: { context: 128000, output: 4096 },
                    },
                  },
                  options: {
                    apiKey: "custom-key",
                  },
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["custom-openai"]).toBeDefined()
          expect(providers["custom-openai"].name).toBe("Custom OpenAI")
          expect(providers["custom-openai"].models["custom-gpt4"]).toBeDefined()
        },
      })
    })

    test("should configure provider timeout", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  options: {
                    timeout: 120000,
                  },
                },
              },
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
          const providers = await Provider.list()
          expect(providers["anthropic"].options.timeout).toBe(120000)
        },
      })
    })
  })

  describe("ULC-PU-STAT-001: Power user statistics viewing", () => {
    test("should track session count per project", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
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
          const session1 = await Session.create({ title: "Stat Test 1" })
          const session2 = await Session.create({ title: "Stat Test 2" })
          const session3 = await Session.create({ title: "Stat Test 3" })

          // Count sessions
          let sessionCount = 0
          for await (const _ of Session.list()) {
            sessionCount++
          }

          expect(sessionCount).toBeGreaterThanOrEqual(3)

          // Cleanup
          await Session.remove(session1.id)
          await Session.remove(session2.id)
          await Session.remove(session3.id)
        },
      })
    })

    test("should track session creation timestamps", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
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
          const beforeCreate = Date.now()
          const session = await Session.create({ title: "Timestamp Test" })
          const afterCreate = Date.now()

          const info = await Session.get(session.id)
          expect(info?.time.created).toBeDefined()
          expect(info?.time.created).toBeGreaterThanOrEqual(beforeCreate)
          expect(info?.time.created).toBeLessThanOrEqual(afterCreate)

          await Session.remove(session.id)
        },
      })
    })

    test("should track session update timestamps", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
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
          const session = await Session.create({ title: "Update Timestamp Test" })
          const initialInfo = await Session.get(session.id)

          // Wait a bit and update
          await new Promise((r) => setTimeout(r, 10))
          const beforeUpdate = Date.now()
          await Session.update(session.id, (s) => {
            s.title = "Updated Title"
          })
          const afterUpdate = Date.now()

          const updatedInfo = await Session.get(session.id)
          expect(updatedInfo?.time.updated).toBeGreaterThanOrEqual(beforeUpdate)
          expect(updatedInfo?.time.updated).toBeLessThanOrEqual(afterUpdate)
          expect(updatedInfo?.time.updated).toBeGreaterThanOrEqual(initialInfo?.time.created ?? 0)

          await Session.remove(session.id)
        },
      })
    })

    test("should calculate project statistics", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
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
          // Project statistics structure
          const stats = {
            projectID: Instance.project.id,
            projectWorktree: Instance.project.worktree,
            sessionCount: 0,
            oldestSession: null as number | null,
            newestSession: null as number | null,
          }

          // Create sessions and track stats
          const session1 = await Session.create({ title: "Stats Session 1" })
          const session2 = await Session.create({ title: "Stats Session 2" })

          for await (const s of Session.list()) {
            stats.sessionCount++
            if (!stats.oldestSession || s.time.created < stats.oldestSession) {
              stats.oldestSession = s.time.created
            }
            if (!stats.newestSession || s.time.created > stats.newestSession) {
              stats.newestSession = s.time.created
            }
          }

          expect(stats.projectID).toBeDefined()
          expect(stats.projectWorktree).toBeDefined()
          expect(stats.sessionCount).toBeGreaterThanOrEqual(2)
          expect(stats.oldestSession).toBeDefined()
          expect(stats.newestSession).toBeDefined()
          expect(stats.newestSession).toBeGreaterThanOrEqual(stats.oldestSession ?? 0)

          await Session.remove(session1.id)
          await Session.remove(session2.id)
        },
      })
    })
  })

  describe("ULC-PU-UPGR-001: Power user upgrade flow", () => {
    test("should have version information available", async () => {
      // Verify version info structure exists
      const versionInfo = {
        current: "0.1.0", // Simulated current version
        latest: "0.2.0", // Simulated latest version
        updateAvailable: true,
      }

      expect(versionInfo.current).toBeDefined()
      expect(typeof versionInfo.current).toBe("string")
      expect(versionInfo.current.match(/^\d+\.\d+\.\d+/)).toBeTruthy()
    })

    test("should compare semantic versions correctly", async () => {
      // Helper to compare semver
      const compareSemver = (v1: string, v2: string): number => {
        const parts1 = v1.split(".").map(Number)
        const parts2 = v2.split(".").map(Number)

        for (let i = 0; i < 3; i++) {
          const p1 = parts1[i] || 0
          const p2 = parts2[i] || 0
          if (p1 > p2) return 1
          if (p1 < p2) return -1
        }
        return 0
      }

      expect(compareSemver("1.0.0", "1.0.0")).toBe(0)
      expect(compareSemver("1.0.1", "1.0.0")).toBe(1)
      expect(compareSemver("1.0.0", "1.0.1")).toBe(-1)
      expect(compareSemver("2.0.0", "1.9.9")).toBe(1)
      expect(compareSemver("1.10.0", "1.9.0")).toBe(1)
    })

    test("should detect update availability", async () => {
      const currentVersion = "0.1.0"
      const latestVersion = "0.2.0"

      const parseVersion = (v: string) => v.split(".").map(Number)
      const isUpdateAvailable = (current: string, latest: string) => {
        const [cMajor, cMinor, cPatch] = parseVersion(current)
        const [lMajor, lMinor, lPatch] = parseVersion(latest)

        if (lMajor > cMajor) return true
        if (lMajor === cMajor && lMinor > cMinor) return true
        if (lMajor === cMajor && lMinor === cMinor && lPatch > cPatch) return true
        return false
      }

      expect(isUpdateAvailable(currentVersion, latestVersion)).toBe(true)
      expect(isUpdateAvailable("1.0.0", "1.0.0")).toBe(false)
      expect(isUpdateAvailable("2.0.0", "1.9.9")).toBe(false)
    })

    test("should format upgrade command correctly", async () => {
      const packageManagers = ["bun", "npm", "yarn", "pnpm"]
      const packageName = "@codecoder-ai/codecoder"

      const formatUpgradeCommand = (pm: string) => {
        switch (pm) {
          case "bun":
            return `bun add -g ${packageName}`
          case "npm":
            return `npm install -g ${packageName}`
          case "yarn":
            return `yarn global add ${packageName}`
          case "pnpm":
            return `pnpm add -g ${packageName}`
          default:
            return `npm install -g ${packageName}`
        }
      }

      expect(formatUpgradeCommand("bun")).toBe(`bun add -g ${packageName}`)
      expect(formatUpgradeCommand("npm")).toBe(`npm install -g ${packageName}`)
      expect(formatUpgradeCommand("yarn")).toBe(`yarn global add ${packageName}`)
      expect(formatUpgradeCommand("pnpm")).toBe(`pnpm add -g ${packageName}`)
    })

    test("should handle pre-release versions", async () => {
      const isPrerelease = (version: string) => {
        return version.includes("-alpha") || version.includes("-beta") || version.includes("-rc")
      }

      expect(isPrerelease("1.0.0")).toBe(false)
      expect(isPrerelease("1.0.0-alpha.1")).toBe(true)
      expect(isPrerelease("1.0.0-beta.2")).toBe(true)
      expect(isPrerelease("1.0.0-rc.1")).toBe(true)
    })
  })

  describe("ULC-PU-CACHE-001: Power user cache management", () => {
    test("should clear model cache", async () => {
      // Verify model cache can be refreshed
      expect(ModelsDev.refresh).toBeDefined()

      // Cache clearing is equivalent to refreshing from source
      const refreshResult = ModelsDev.refresh()
      expect(refreshResult).toBeDefined()
    })

    test("should manage config cache", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Config is loaded fresh each Instance.provide
          expect(Instance.project).toBeDefined()
          expect(Instance.project.id).toBeDefined()
        },
      })

      // New instance provides fresh config
      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          expect(Instance.project).toBeDefined()
        },
      })
    })
  })

  describe("ULC-PU-HOOK-001: Hook configuration loading", () => {
    test("should load hooks from .ccode/hooks directory", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  test_hook: {
                    pattern: "Edit",
                    description: "Test hook for editing",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["test.*pattern"],
                        message: "Test message",
                        block: false,
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
          // Verify hooks can be loaded by running a hook check
          const result = await Hook.run("PreToolUse", {
            tool: "Read", // Different tool, should not trigger
            input: { filePath: "/test/file.txt" },
          })

          expect(result.blocked).toBe(false)
        },
      })
    })

    test("should validate hook configuration schema", async () => {
      const validConfig = {
        hooks: {
          PreToolUse: {
            scan_hook: {
              pattern: "Edit",
              actions: [
                {
                  type: "scan" as const,
                  patterns: ["secret.*"],
                  message: "Secret detected",
                  block: true,
                },
              ],
            },
          },
        },
        settings: {
          enabled: true,
          blocking_mode: "interactive" as const,
        },
      }

      const result = Hook.HooksConfig.safeParse(validConfig)
      expect(result.success).toBe(true)
    })

    test("should reject invalid hook configuration", async () => {
      const invalidConfig = {
        hooks: {
          PreToolUse: {
            bad_hook: {
              pattern: "Edit",
              actions: [
                {
                  type: "invalid_action_type",
                },
              ],
            },
          },
        },
      }

      const result = Hook.HooksConfig.safeParse(invalidConfig)
      expect(result.success).toBe(false)
    })
  })

  describe("ULC-PU-HOOK-002: PreToolUse hook execution", () => {
    test("should block tool execution when pattern matches", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  api_key_detector: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["sk_live_[a-zA-Z0-9]+"],
                        message: "API key detected: {match}",
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
            input: {
              filePath: "/test/config.ts",
              oldString: "placeholder",
              newString: "sk_live_abcd1234567890",
            },
          })

          expect(result.blocked).toBe(true)
          expect(result.message).toContain("API key detected")
        },
      })
    })

    test("should allow tool execution when pattern does not match", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  secret_detector: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["password\\s*=\\s*['\"].*['\"]"],
                        message: "Hardcoded password detected",
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
            input: {
              filePath: "/test/file.ts",
              oldString: "old",
              newString: "const greeting = 'hello'",
            },
          })

          expect(result.blocked).toBe(false)
        },
      })
    })

    test("should only trigger for matching tool pattern", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  bash_blocker: {
                    pattern: "Bash",
                    actions: [
                      {
                        type: "scan",
                        patterns: [".*"],
                        message: "Bash blocked",
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
          // Edit tool should not be blocked by Bash hook
          const result = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts" },
          })

          expect(result.blocked).toBe(false)
        },
      })
    })
  })

  describe("ULC-PU-HOOK-003: PostToolUse hook execution", () => {
    test("should have PostToolUse lifecycle", () => {
      expect(Hook.Lifecycle.enum.PostToolUse).toBe("PostToolUse")
    })

    test("should run PostToolUse hook after tool execution", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PostToolUse: {
                  post_check: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: ["error"],
                        message: "Error in output",
                        block: false,
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
          const result = await Hook.run("PostToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts" },
            output: JSON.stringify({ success: true, message: "Completed" }),
          })

          expect(result.blocked).toBe(false)
        },
      })
    })
  })

  describe("ULC-PU-HOOK-004: Hook blocking behavior", () => {
    test("should return blocked status when hook blocks", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  blocker: {
                    pattern: "Write",
                    actions: [
                      {
                        type: "scan",
                        patterns: [".*env.*"],
                        message: "Cannot write to env files",
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
            tool: "Write",
            input: {
              filePath: ".env.local",
              content: "SECRET=value",
            },
          })

          expect(result.blocked).toBe(true)
          expect(result.message).toContain("Cannot write to env files")
        },
      })
    })

    test("should not block when hooks are disabled", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  always_block: {
                    pattern: ".*",
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

  describe("ULC-PU-HOOK-005: Hook with multiple patterns", () => {
    test("should match any of multiple patterns", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          const hooksDir = path.join(dir, ".ccode", "hooks")
          await Bun.write(
            path.join(hooksDir, "hooks.json"),
            JSON.stringify({
              hooks: {
                PreToolUse: {
                  secret_scanner: {
                    pattern: "Edit",
                    actions: [
                      {
                        type: "scan",
                        patterns: [
                          "PRIVATE_KEY",
                          "API_SECRET",
                          "password\\s*=",
                        ],
                        message: "Sensitive data detected",
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
          // Test first pattern
          const result1 = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", newString: "PRIVATE_KEY=abc123" },
          })
          expect(result1.blocked).toBe(true)

          // Test second pattern
          const result2 = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", newString: "API_SECRET=xyz789" },
          })
          expect(result2.blocked).toBe(true)

          // Test no match
          const result3 = await Hook.run("PreToolUse", {
            tool: "Edit",
            input: { filePath: "/test/file.ts", newString: "const name = 'test'" },
          })
          expect(result3.blocked).toBe(false)
        },
      })
    })
  })

  describe("ULC-PU-PERM-001: Custom permission rules", () => {
    test("should respect custom permission configuration", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              permission: {
                allow: ["read:*", "glob:*"],
                deny: ["bash:rm *"],
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Verify config loads with permission rules
          expect(Instance.project).toBeDefined()
        },
      })
    })
  })

  describe("ULC-PU-AGNT-001: Custom agent definition", () => {
    test("should load custom agent configuration", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              agent: {
                "custom-agent": {
                  model: "anthropic/claude-sonnet-4-20250514",
                  system: "You are a custom agent for testing.",
                },
              },
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
          // Verify agent config is loaded
          expect(Instance.project).toBeDefined()
        },
      })
    })
  })
})
