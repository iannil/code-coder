/**
 * MCP Server Integration Tests
 *
 * End-to-end tests using real HTTP transport to verify MCP server functionality.
 * These tests start an actual server and connect via HTTP.
 *
 * NOTE: These tests require spawning a server subprocess and may be skipped
 * in certain CI environments where subprocess spawning is restricted.
 */

import { describe, test, expect, beforeAll, afterAll, setDefaultTimeout } from "bun:test"
import { Client } from "@modelcontextprotocol/sdk/client/index.js"
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js"
import type { Subprocess } from "bun"

const TEST_PORT = 14405
const BASE_URL = `http://localhost:${TEST_PORT}/mcp`
const TEST_API_KEY = "test-integration-key"

// Skip integration tests in CI or when server can't start
const SKIP_INTEGRATION = process.env.CI === "true" || process.env.SKIP_MCP_INTEGRATION === "true"

// MCP server startup can take 15+ seconds
setDefaultTimeout(30000)

describe.skipIf(SKIP_INTEGRATION)("MCP Server Integration", () => {
  let serverProcess: Subprocess<"ignore", "pipe", "pipe"> | undefined
  let serverStarted = false

  async function startServer(apiKey?: string): Promise<Subprocess<"ignore", "pipe", "pipe">> {
    const args = [
      "run",
      "src/index.ts",
      "mcp",
      "serve",
      "--transport",
      "http",
      "--port",
      String(TEST_PORT),
    ]
    if (apiKey) {
      args.push("--api-key", apiKey)
    }

    const proc = Bun.spawn(["bun", ...args], {
      cwd: process.cwd(),
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    })

    // Wait for server to start with retry logic
    const maxRetries = 30
    const retryDelay = 500

    for (let i = 0; i < maxRetries; i++) {
      await Bun.sleep(retryDelay)
      try {
        const response = await fetch(`http://localhost:${TEST_PORT}/health`, {
          signal: AbortSignal.timeout(2000),
        })
        if (response.ok) {
          serverStarted = true
          break
        }
      } catch {
        // Server not ready yet, continue retrying
      }
    }

    if (!serverStarted) {
      console.error("MCP server failed to start after 15 seconds")
    }

    return proc
  }

  async function stopServer(proc?: Subprocess) {
    if (proc) {
      proc.kill()
      await Bun.sleep(500)
    }
  }

  describe("HTTP Transport - No Auth", () => {
    beforeAll(async () => {
      serverProcess = await startServer()
    })

    afterAll(async () => {
      await stopServer(serverProcess)
    })

    test("health check returns ok", async () => {
      const response = await fetch(`http://localhost:${TEST_PORT}/health`)
      expect(response.status).toBe(200)
      const data = await response.json()
      expect(data.status).toBe("ok")
    })

    test("list tools returns available tools", async () => {
      const transport = new StreamableHTTPClientTransport(new URL(BASE_URL))
      const client = new Client({ name: "test", version: "1.0" }, {})
      await client.connect(transport)

      const result = await client.listTools()
      expect(result.tools.length).toBeGreaterThan(5)

      // Verify some expected tools exist
      const toolNames = result.tools.map((t) => t.name)
      expect(toolNames).toContain("read")
      expect(toolNames).toContain("bash")
      expect(toolNames).toContain("glob")

      await client.close()
    })

    test("list prompts returns agent prompts", async () => {
      const transport = new StreamableHTTPClientTransport(new URL(BASE_URL))
      const client = new Client({ name: "test", version: "1.0" }, {})
      await client.connect(transport)

      const result = await client.listPrompts()
      expect(result.prompts.length).toBeGreaterThan(0)

      await client.close()
    })

    test("list resources returns project files", async () => {
      const transport = new StreamableHTTPClientTransport(new URL(BASE_URL))
      const client = new Client({ name: "test", version: "1.0" }, {})
      await client.connect(transport)

      const result = await client.listResources()
      // Should have at least one resource if CLAUDE.md or README.md exists
      expect(Array.isArray(result.resources)).toBe(true)

      await client.close()
    })
  })

  describe("HTTP Transport - With Auth", () => {
    beforeAll(async () => {
      await stopServer(serverProcess)
      serverProcess = await startServer(TEST_API_KEY)
    })

    afterAll(async () => {
      await stopServer(serverProcess)
    })

    test("rejects unauthenticated requests", async () => {
      const response = await fetch(BASE_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          method: "initialize",
          params: {
            protocolVersion: "2024-11-05",
            capabilities: {},
            clientInfo: { name: "test", version: "1.0" },
          },
          id: 1,
        }),
      })
      expect(response.status).toBe(401)
    })

    test("accepts authenticated requests with Bearer token", async () => {
      const response = await fetch(BASE_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json, text/event-stream",
          Authorization: `Bearer ${TEST_API_KEY}`,
        },
        body: JSON.stringify({
          jsonrpc: "2.0",
          method: "initialize",
          params: {
            protocolVersion: "2024-11-05",
            capabilities: {},
            clientInfo: { name: "test", version: "1.0" },
          },
          id: 1,
        }),
      })
      expect(response.status).toBe(200)
    })

    test("accepts authenticated requests with X-API-Key header", async () => {
      const response = await fetch(BASE_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json, text/event-stream",
          "X-API-Key": TEST_API_KEY,
        },
        body: JSON.stringify({
          jsonrpc: "2.0",
          method: "initialize",
          params: {
            protocolVersion: "2024-11-05",
            capabilities: {},
            clientInfo: { name: "test", version: "1.0" },
          },
          id: 1,
        }),
      })
      expect(response.status).toBe(200)
    })
  })

  describe("Session Management", () => {
    beforeAll(async () => {
      await stopServer(serverProcess)
      serverProcess = await startServer()
    })

    afterAll(async () => {
      await stopServer(serverProcess)
    })

    test("multiple sessions work independently", async () => {
      // Create first client
      const transport1 = new StreamableHTTPClientTransport(new URL(BASE_URL))
      const client1 = new Client({ name: "test-1", version: "1.0" }, {})
      await client1.connect(transport1)

      // Create second client
      const transport2 = new StreamableHTTPClientTransport(new URL(BASE_URL))
      const client2 = new Client({ name: "test-2", version: "1.0" }, {})
      await client2.connect(transport2)

      // Both should be able to list tools
      const [result1, result2] = await Promise.all([client1.listTools(), client2.listTools()])

      expect(result1.tools.length).toBeGreaterThan(0)
      expect(result2.tools.length).toBeGreaterThan(0)

      await Promise.all([client1.close(), client2.close()])
    })
  })

  describe("Logging Capability", () => {
    beforeAll(async () => {
      await stopServer(serverProcess)
      serverProcess = await startServer()
    })

    afterAll(async () => {
      await stopServer(serverProcess)
    })

    test("set logging level works", async () => {
      const transport = new StreamableHTTPClientTransport(new URL(BASE_URL))
      const client = new Client({ name: "test", version: "1.0" }, {})
      await client.connect(transport)

      // Set log level - should not throw
      await client.setLoggingLevel("debug")
      await client.setLoggingLevel("info")
      await client.setLoggingLevel("warning")
      await client.setLoggingLevel("error")

      await client.close()
    })
  })

  describe("Resource Templates", () => {
    beforeAll(async () => {
      await stopServer(serverProcess)
      serverProcess = await startServer()
    })

    afterAll(async () => {
      await stopServer(serverProcess)
    })

    test("list resource templates returns templates", async () => {
      const transport = new StreamableHTTPClientTransport(new URL(BASE_URL))
      const client = new Client({ name: "test", version: "1.0" }, {})
      await client.connect(transport)

      const result = await client.listResourceTemplates()
      // Templates may be empty if no glob patterns configured
      expect(Array.isArray(result.resourceTemplates)).toBe(true)

      await client.close()
    })
  })
})
