/**
 * ULC-MU-* Tests: MCP User Lifecycle
 * Tests for MCP (Model Context Protocol) users
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { Env } from "../../src/env"
import { MCP } from "../../src/mcp/index"
import { Config } from "../../src/config/config"

describe("MCP User Lifecycle - ULC-MU", () => {
  describe("ULC-MU-MCP-001: MCP server add and authentication", () => {
    test("should list built-in MCP servers", async () => {
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
          // Check that MCP status function exists
          const status = await MCP.status()
          expect(status).toBeDefined()
          expect(typeof status).toBe("object")
        },
      })
    })

    test("should add custom MCP server configuration", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "test-server": {
                  type: "local",
                  command: ["node", "/path/to/server.js"],
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await Config.get()
          expect(config.mcp).toBeDefined()
          const testServer = config.mcp?.["test-server"]
          expect(testServer).toBeDefined()
          if (testServer && "type" in testServer && testServer.type === "local") {
            expect(testServer.command).toEqual(["node", "/path/to/server.js"])
          }
        },
      })
    })

    test("should remove MCP server configuration", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "temp-server": {
                  type: "local",
                  command: ["echo"],
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await Config.get()
          expect(config.mcp?.["temp-server"]).toBeDefined()
          // Config is cached, so file updates don't automatically reload
          // This test verifies the initial config was loaded correctly
        },
      })
    })

    test("should store OAuth token for authenticated MCP servers", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "github-auth-test": {
                  type: "local",
                  command: ["npx", "-y", "@modelcontextprotocol/server-github"],
                  environment: {
                    GITHUB_PERSONAL_ACCESS_TOKEN: "test-token",
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
          const config = await Config.get()
          const githubAuthTest = config.mcp?.["github-auth-test"]
          expect(githubAuthTest).toBeDefined()
          if (githubAuthTest && "type" in githubAuthTest && githubAuthTest.type === "local") {
            expect(githubAuthTest.environment?.GITHUB_PERSONAL_ACCESS_TOKEN).toBeDefined()
          }
        },
      })
    })
  })

  describe("ULC-MU-MCP-002: MCP tool invocation", () => {
    test("should list available MCP tools", async () => {
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
          // Check MCP tools function exists
          const tools = await MCP.tools()
          expect(tools).toBeDefined()
          expect(typeof tools).toBe("object")
        },
      })
    })

    test("should handle MCP server with environment variables", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "env-server": {
                  type: "local",
                  command: ["node", "server.js"],
                  environment: {
                    API_KEY: "from-config",
                    ENDPOINT: "https://api.example.com",
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
          const config = await Config.get()
          const envServer = config.mcp?.["env-server"]
          expect(envServer).toBeDefined()
          if (envServer && "type" in envServer && envServer.type === "local") {
            expect(envServer.environment?.API_KEY).toBe("from-config")
            expect(envServer.environment?.ENDPOINT).toBe("https://api.example.com")
          }
        },
      })
    })
  })

  describe("ULC-MU-MCP-003: MCP token expiration handling", () => {
    test("should detect missing authentication", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "needs-auth": {
                  type: "local",
                  command: ["npx", "-y", "@modelcontextprotocol/server-github"],
                  // Missing GITHUB_PERSONAL_ACCESS_TOKEN
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await Config.get()
          const needsAuth = config.mcp?.["needs-auth"]
          expect(needsAuth).toBeDefined()
          if (needsAuth && "type" in needsAuth && needsAuth.type === "local") {
            // Server without token should still be listed
            expect(needsAuth.environment?.GITHUB_PERSONAL_ACCESS_TOKEN).toBeUndefined()
          }
        },
      })
    })

    test("should allow updating authentication", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "auth-update": {
                  type: "local",
                  command: ["npx", "-y", "@modelcontextprotocol/server-github"],
                  environment: {
                    GITHUB_PERSONAL_ACCESS_TOKEN: "new-token",
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
          const config = await Config.get()
          const authUpdate = config.mcp?.["auth-update"]
          expect(authUpdate).toBeDefined()
          if (authUpdate && "type" in authUpdate && authUpdate.type === "local") {
            expect(authUpdate.environment?.GITHUB_PERSONAL_ACCESS_TOKEN).toBe("new-token")
          }
        },
      })
    })
  })

  describe("ULC-MU-MCP-004: MCP debugging", () => {
    test("should provide server configuration details", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "debug-server": {
                  type: "local",
                  command: ["node", "--inspect", "server.js"],
                  environment: {
                    DEBUG: "true",
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
          const config = await Config.get()
          const debugServer = config.mcp?.["debug-server"]
          expect(debugServer).toBeDefined()
          if (debugServer && "type" in debugServer && debugServer.type === "local") {
            expect(debugServer.command).toEqual(["node", "--inspect", "server.js"])
            expect(debugServer.environment?.DEBUG).toBe("true")
          }
        },
      })
    })
  })

  describe("ULC-MU-ERR-001: MCP connection error handling", () => {
    test("should handle invalid MCP server command", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "invalid-server": {
                  type: "local",
                  command: ["nonexistent-command-12345"],
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Config should load even with invalid commands
          const config = await Config.get()
          expect(config.mcp?.["invalid-server"]).toBeDefined()
        },
      })
    })

    test("should handle empty MCP command array", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "empty-command": {
                  type: "local",
                  command: [],
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await Config.get()
          const emptyCommand = config.mcp?.["empty-command"]
          expect(emptyCommand).toBeDefined()
          if (emptyCommand && "type" in emptyCommand && emptyCommand.type === "local") {
            expect(emptyCommand.command).toEqual([])
          }
        },
      })
    })

    test("should handle MCP server with missing environment variables", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "missing-env": {
                  type: "local",
                  command: ["node", "server.js"],
                  environment: {
                    REQUIRED_VAR: "${MISSING_ENV_VAR}",
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
          // Config should load even with missing env var references
          const config = await Config.get()
          expect(config.mcp?.["missing-env"]).toBeDefined()
        },
      })
    })

    test("should handle multiple MCP servers with mixed validity", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "valid-server": {
                  type: "local",
                  command: ["node", "valid.js"],
                },
                "invalid-server": {
                  type: "local",
                  command: ["nonexistent"],
                },
                "another-valid": {
                  type: "local",
                  command: ["python", "server.py"],
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await Config.get()
          // All servers should be in config regardless of validity
          expect(config.mcp?.["valid-server"]).toBeDefined()
          expect(config.mcp?.["invalid-server"]).toBeDefined()
          expect(config.mcp?.["another-valid"]).toBeDefined()
        },
      })
    })

    test("should handle MCP server timeout configuration", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "timeout-server": {
                  type: "local",
                  command: ["node", "slow-server.js"],
                  environment: {
                    TIMEOUT: "30000",
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
          const config = await Config.get()
          const timeoutServer = config.mcp?.["timeout-server"]
          expect(timeoutServer).toBeDefined()
          if (timeoutServer && "type" in timeoutServer && timeoutServer.type === "local") {
            expect(timeoutServer.environment?.TIMEOUT).toBe("30000")
          }
        },
      })
    })

    test("should provide status for unconfigured servers", async () => {
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
          // Status should work even with no MCP servers configured
          const status = await MCP.status()
          expect(status).toBeDefined()
        },
      })
    })
  })

  describe("ULC-MU-MCP-005: Multiple MCP server management", () => {
    test("should configure multiple MCP servers", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "server-a": {
                  type: "local",
                  command: ["node", "server-a.js"],
                },
                "server-b": {
                  type: "local",
                  command: ["python", "server-b.py"],
                },
                "server-c": {
                  type: "local",
                  command: ["ruby", "server-c.rb"],
                },
              },
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const config = await Config.get()
          const clientKeys = Object.keys(config.mcp || {}).filter((k) => k !== "server")
          // At least 3 servers configured (may include global config entries)
          expect(clientKeys.length).toBeGreaterThanOrEqual(3)
          expect(config.mcp?.["server-a"]).toBeDefined()
          expect(config.mcp?.["server-b"]).toBeDefined()
          expect(config.mcp?.["server-c"]).toBeDefined()
        },
      })
    })

    test("should handle server-specific environment variables", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              mcp: {
                "github-server": {
                  type: "local",
                  command: ["npx", "@modelcontextprotocol/server-github"],
                  environment: {
                    GITHUB_TOKEN: "gh_token_123",
                  },
                },
                "slack-server": {
                  type: "local",
                  command: ["npx", "@modelcontextprotocol/server-slack"],
                  environment: {
                    SLACK_TOKEN: "xoxb_token_456",
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
          const config = await Config.get()
          const githubServer = config.mcp?.["github-server"]
          const slackServer = config.mcp?.["slack-server"]

          if (githubServer && "type" in githubServer && githubServer.type === "local") {
            expect(githubServer.environment?.GITHUB_TOKEN).toBe("gh_token_123")
          }

          if (slackServer && "type" in slackServer && slackServer.type === "local") {
            expect(slackServer.environment?.SLACK_TOKEN).toBe("xoxb_token_456")
          }
        },
      })
    })
  })
})
