/**
 * E2E High Priority Test: MCP Integration
 * Priority: High - Runs daily
 *
 * Tests MCP server configuration and management
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "../../../src/project/instance"
import { Config } from "../../../src/config/config"
import { MCP } from "../../../src/mcp/index"
import { Env } from "../../../src/env"

describe("E2E High: MCP Integration", () => {
  describe("MCP Server Configuration", () => {
    test("should load MCP server from config", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              mcp: {
                "test-server": {
                  type: "local",
                  command: ["node", "server.js"],
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
            expect(testServer.command).toEqual(["node", "server.js"])
          }
        },
      })
    })

    test("should support remote MCP server", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              mcp: {
                "remote-server": {
                  type: "remote",
                  url: "https://mcp.example.com",
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
          const remoteServer = config.mcp?.["remote-server"]
          expect(remoteServer).toBeDefined()
          if (remoteServer && "type" in remoteServer && remoteServer.type === "remote") {
            expect(remoteServer.url).toBe("https://mcp.example.com")
          }
        },
      })
    })

    test("should support MCP server with environment variables", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              mcp: {
                "env-server": {
                  type: "local",
                  command: ["npx", "-y", "@mcp/server"],
                  environment: {
                    API_KEY: "test-key",
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
          const envServer = config.mcp?.["env-server"]
          expect(envServer).toBeDefined()
          if (envServer && "type" in envServer && envServer.type === "local") {
            expect(envServer.environment?.API_KEY).toBe("test-key")
            expect(envServer.environment?.DEBUG).toBe("true")
          }
        },
      })
    })
  })

  describe("MCP Status", () => {
    test("should report MCP status", async () => {
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
          const status = await MCP.status()
          expect(status).toBeDefined()
          expect(typeof status).toBe("object")
        },
      })
    })

    test("should list MCP tools", async () => {
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
        fn: async () => {
          const tools = await MCP.tools()
          expect(tools).toBeDefined()
          expect(typeof tools).toBe("object")
        },
      })
    })
  })

  describe("Multiple MCP Servers", () => {
    test("should support multiple MCP servers", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              mcp: {
                "server-1": {
                  type: "local",
                  command: ["node", "server1.js"],
                },
                "server-2": {
                  type: "local",
                  command: ["node", "server2.js"],
                },
                "server-3": {
                  type: "remote",
                  url: "https://mcp3.example.com",
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
          expect(Object.keys(config.mcp ?? {}).length).toBe(3)
          expect(config.mcp?.["server-1"]).toBeDefined()
          expect(config.mcp?.["server-2"]).toBeDefined()
          expect(config.mcp?.["server-3"]).toBeDefined()
        },
      })
    })
  })
})
