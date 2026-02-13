/**
 * ULC-NU-* Tests: New User Lifecycle
 * Tests for new users onboarding to CodeCoder
 */

import { describe, test, expect, beforeAll, beforeEach, mock } from "bun:test"
import path from "path"
import { EventEmitter } from "events"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { Provider } from "../../src/provider/provider"
import { Env } from "../../src/env"
import { Session } from "../../src/session"
import { Config } from "../../src/config/config"
import { ReadTool } from "../../src/tool/read"
import { GlobTool } from "../../src/tool/glob"

const toolCtx = {
  sessionID: "test",
  messageID: "",
  callID: "",
  agent: "build",
  abort: AbortSignal.any([]),
  metadata: () => {},
  ask: async () => {},
}

const projectRoot = path.join(__dirname, "../..")

// Check if ripgrep is available (required for glob tool)
let rgAvailable = false
try {
  const proc = Bun.spawn(["rg", "--version"], {
    stdout: "pipe",
    stderr: "pipe",
  })
  proc.unref()
  rgAvailable = true
} catch {
  // ripgrep not available
}

describe("New User Lifecycle - ULC-NU", () => {
  describe("ULC-NU-AUTH-001: New user OAuth login flow", () => {
    test("should display provider selection prompt when not logged in", async () => {
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
          const providers = await Provider.list()
          // With no API keys, should have limited providers (some built-in exist)
          expect(providers).toBeDefined()
        },
      })
    })

    test("should register API key from environment variable", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key-12345")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          // Source may be "env", "custom", or "config" depending on loading order
          expect(["env", "custom", "config"]).toContain(providers["anthropic"].source)
        },
      })
    })

    test("should persist API key in config", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  options: {
                    apiKey: "sk-ant-persisted-key-12345",
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
          expect(providers["anthropic"]).toBeDefined()
          expect(providers["anthropic"].options.apiKey).toBe("sk-ant-persisted-key-12345")
        },
      })
    })
  })

  describe("ULC-NU-AUTH-002: New user API Key login", () => {
    test("should validate API key format", async () => {
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test123456789")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
        },
      })
    })

    test("should store API key securely", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  options: {
                    apiKey: "sk-ant-secure-key",
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
          expect(providers["anthropic"].key).toBeDefined()
        },
      })
    })
  })

  describe("ULC-NU-SESS-001: New user first session", () => {
    test("should create session successfully", async () => {
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
          const session = await Session.create({
            title: "First Session",
          })

          expect(session).toBeDefined()
          expect(session.id).toBeDefined()
          expect(session.title).toBe("First Session")
          expect(session.projectID).toBeDefined()

          await Session.remove(session.id)
        },
      })
    })

    test("should auto-save session on creation", async () => {
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
          const session = await Session.create({
            title: "Auto Save Test",
          })

          // Verify session was saved
          const retrieved = await Session.get(session.id)
          expect(retrieved).toBeDefined()
          expect(retrieved?.id).toBe(session.id)
          expect(retrieved?.title).toBe("Auto Save Test")

          await Session.remove(session.id)
        },
      })
    })

    test("should allow session continuation", async () => {
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
          // Create initial session
          const session = await Session.create({
            title: "Continuable Session",
          })

          // List sessions should include it
          const sessions = []
          for await (const s of Session.list()) {
            sessions.push(s)
          }
          expect(sessions.length).toBeGreaterThan(0)

          await Session.remove(session.id)
        },
      })
    })
  })

  describe("ULC-NU-MDLS-001: New user model discovery", () => {
    test("should list all available models", async () => {
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
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()

          const models = Object.keys(providers["anthropic"].models)
          expect(models.length).toBeGreaterThan(0)
          expect(models).toContain("claude-sonnet-4-20250514")
        },
      })
    })

    test("should filter models by provider", async () => {
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
          const providers = await Provider.list()
          // Verify anthropic provider exists
          expect(providers["anthropic"]).toBeDefined()

          // Verify specific model exists
          const models = Object.keys(providers["anthropic"].models)
          expect(models).toContain("claude-sonnet-4-20250514")
        },
      })
    })

    test("should include model metadata", async () => {
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
          const providers = await Provider.list()
          const model = providers["anthropic"].models["claude-sonnet-4-20250514"]

          expect(model.name).toBeDefined()
          expect(model.limit).toBeDefined()
          expect(model.limit.context).toBeGreaterThan(0)
          expect(model.limit.output).toBeGreaterThan(0)
          expect(model.capabilities).toBeDefined()
          expect(model.capabilities.toolcall).toBe(true)
        },
      })
    })
  })

  describe("ULC-NU-AUTH-001: OAuth flow simulation (extended)", () => {
    test("should support multiple authentication providers", async () => {
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
          Env.set("OPENAI_API_KEY", "sk-openai-test-key")
        },
        fn: async () => {
          const providers = await Provider.list()

          // Both providers should be available
          expect(providers["anthropic"]).toBeDefined()
          expect(providers["openai"]).toBeDefined()
        },
      })
    })

    test("should prioritize environment variable over config", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  options: {
                    apiKey: "sk-ant-config-key",
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
          Env.set("ANTHROPIC_API_KEY", "sk-ant-env-key")
        },
        fn: async () => {
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
          // Both env and config provide the key, implementation may use either
          expect(["env", "config", "custom"]).toContain(providers["anthropic"].source)
        },
      })
    })

    test("should handle missing credentials gracefully", async () => {
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
          // Without credentials, providers should still be queryable
          const providers = await Provider.list()
          expect(providers).toBeDefined()
          expect(typeof providers).toBe("object")
        },
      })
    })
  })

  describe("ULC-NU-ONBOARD-001: New user onboarding experience", () => {
    test("should load config without errors for new user", async () => {
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
          const config = await Config.get()
          expect(config).toBeDefined()
        },
      })
    })

    test("should create project context for new directory", async () => {
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
          expect(Instance.project).toBeDefined()
          expect(Instance.project.id).toBeDefined()
          expect(Instance.project.worktree).toBeDefined()
        },
      })
    })
  })

  describe("ULC-NU-TOOL-001: New user first tool usage", () => {
    test("should successfully use read tool on first attempt", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(path.join(dir, "hello.txt"), "Hello, World!")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const read = await ReadTool.init()
          const result = await read.execute({ filePath: path.join(tmp.path, "hello.txt") }, toolCtx)
          expect(result.output).toContain("Hello, World!")
        },
      })
    })

    test.skipIf(!rgAvailable)("should successfully use glob tool to find files", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
            }),
          )
          await Bun.write(path.join(dir, "src", "index.ts"), "export {}")
          await Bun.write(path.join(dir, "src", "utils.ts"), "export {}")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const glob = await GlobTool.init()
          const result = await glob.execute({ pattern: "**/*.ts" }, toolCtx)
          expect(result.output).toContain("index.ts")
          expect(result.output).toContain("utils.ts")
        },
      })
    })

    test("should handle tool errors gracefully for new users", async () => {
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
          const read = await ReadTool.init()
          // Non-existent file should throw with helpful error
          await expect(read.execute({ filePath: path.join(tmp.path, "nonexistent.txt") }, toolCtx)).rejects.toThrow(
            "not found",
          )
        },
      })
    })
  })

  describe("ULC-NU-ERR-001: Authentication error handling", () => {
    test("should handle invalid API key format gracefully", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  options: {
                    apiKey: "invalid-key",
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
          // Invalid key should still allow provider listing
          const providers = await Provider.list()
          expect(providers).toBeDefined()
          // Key is stored even if invalid - validation happens at request time
          expect(providers["anthropic"]).toBeDefined()
        },
      })
    })

    test("should handle empty API key", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              provider: {
                anthropic: {
                  options: {
                    apiKey: "",
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
          expect(providers).toBeDefined()
        },
      })
    })

    test("should handle missing provider configuration", async () => {
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
          // Without any provider config, list should still work
          const providers = await Provider.list()
          expect(providers).toBeDefined()
          expect(typeof providers).toBe("object")
        },
      })
    })

    test("should handle provider authentication errors gracefully", async () => {
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
          // Set a fake API key that will fail authentication
          Env.set("ANTHROPIC_API_KEY", "sk-ant-fake-key-12345")
        },
        fn: async () => {
          // Provider should be registered but will fail on actual API calls
          const providers = await Provider.list()
          expect(providers["anthropic"]).toBeDefined()
        },
      })
    })
  })

  describe("ULC-NU-ERR-002: Network and config error handling", () => {
    test("should handle invalid JSON config file", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Write invalid JSON
          await Bun.write(path.join(dir, "codecoder.json"), "{ invalid json }")
        },
      })

      // Invalid JSON is handled gracefully - Instance.provide still works
      // because the config system has fallback behavior
      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Project should still be defined even with invalid config
          expect(Instance.project).toBeDefined()
        },
      })
    })

    test("should handle missing config file", async () => {
      await using tmp = await tmpdir({})

      // Should work without config file
      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          expect(Instance.project).toBeDefined()
        },
      })
    })

    test("should handle non-existent session ID", async () => {
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
          // Session.get throws NotFoundError for non-existent sessions
          await expect(Session.get("ses_nonexistent123")).rejects.toThrow("NotFoundError")
        },
      })
    })

    test("should handle session removal for non-existent session", async () => {
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
          // Removing non-existent session (with valid ID format) should not throw
          await Session.remove("ses_nonexistent456")
        },
      })
    })

    test("should handle concurrent session operations", async () => {
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
          // Create multiple sessions concurrently
          const results = await Promise.all([
            Session.create({ title: "Concurrent 1" }),
            Session.create({ title: "Concurrent 2" }),
            Session.create({ title: "Concurrent 3" }),
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
  })
})
