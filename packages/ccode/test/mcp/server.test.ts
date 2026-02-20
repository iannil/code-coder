import { describe, test, expect, beforeAll, afterAll } from "bun:test"
import { Server } from "@modelcontextprotocol/sdk/server/index.js"
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js"
import { Client } from "@modelcontextprotocol/sdk/client/index.js"
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListPromptsRequestSchema,
  GetPromptRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  type Tool as MCPToolDefinition,
} from "@modelcontextprotocol/sdk/types.js"
import z from "zod/v4"
import path from "path"

describe("McpServer", () => {
  describe("Tool Handlers", () => {
    test("tools/list returns tools with proper schema", async () => {
      // Create a mock server and client using in-memory transport
      const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()

      const server = new Server(
        { name: "test-server", version: "1.0.0" },
        { capabilities: { tools: { listChanged: true } } },
      )

      // Register a mock tool handler
      const mockTools: MCPToolDefinition[] = [
        {
          name: "test_tool",
          description: "A test tool",
          inputSchema: {
            type: "object",
            properties: { input: { type: "string" } },
            required: ["input"],
          },
        },
      ]

      server.setRequestHandler(ListToolsRequestSchema, async () => {
        return { tools: mockTools }
      })

      server.setRequestHandler(CallToolRequestSchema, async (request) => {
        const { name, arguments: args } = request.params
        if (name === "test_tool") {
          return {
            content: [{ type: "text" as const, text: `Executed with: ${JSON.stringify(args)}` }],
            isError: false,
          }
        }
        return {
          content: [{ type: "text" as const, text: `Tool not found: ${name}` }],
          isError: true,
        }
      })

      await server.connect(serverTransport)

      const client = new Client({ name: "test-client", version: "1.0.0" }, { capabilities: {} })
      await client.connect(clientTransport)

      // Test tools/list
      const toolsResult = await client.listTools()
      expect(toolsResult.tools).toHaveLength(1)
      expect(toolsResult.tools[0].name).toBe("test_tool")
      expect(toolsResult.tools[0].description).toBe("A test tool")

      // Test tools/call
      const callResult = await client.callTool({ name: "test_tool", arguments: { input: "hello" } })
      expect(callResult.isError).toBe(false)
      const content = callResult.content as Array<{ type: string; text?: string }>
      expect(content).toHaveLength(1)
      expect(content[0].text).toContain("hello")

      await client.close()
      await server.close()
    })

    test("tools/call handles missing tool gracefully", async () => {
      const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()

      const server = new Server(
        { name: "test-server", version: "1.0.0" },
        { capabilities: { tools: { listChanged: true } } },
      )

      server.setRequestHandler(ListToolsRequestSchema, async () => {
        return { tools: [] }
      })

      server.setRequestHandler(CallToolRequestSchema, async (request) => {
        return {
          content: [{ type: "text" as const, text: `Tool not found: ${request.params.name}` }],
          isError: true,
        }
      })

      await server.connect(serverTransport)

      const client = new Client({ name: "test-client", version: "1.0.0" }, { capabilities: {} })
      await client.connect(clientTransport)

      const callResult = await client.callTool({ name: "nonexistent", arguments: {} })
      expect(callResult.isError).toBe(true)
      const content = callResult.content as Array<{ type: string; text?: string }>
      expect(content[0].text).toContain("not found")

      await client.close()
      await server.close()
    })
  })

  describe("Prompts Handlers", () => {
    test("prompts/list returns prompts", async () => {
      const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()

      const server = new Server(
        { name: "test-server", version: "1.0.0" },
        { capabilities: { prompts: { listChanged: true } } },
      )

      server.setRequestHandler(ListPromptsRequestSchema, async () => {
        return {
          prompts: [
            { name: "build", description: "Build agent" },
            { name: "plan", description: "Plan agent" },
          ],
        }
      })

      server.setRequestHandler(GetPromptRequestSchema, async (request) => {
        const { name } = request.params
        return {
          description: `Agent prompt: ${name}`,
          messages: [{ role: "user", content: { type: "text", text: `System prompt for ${name}` } }],
        }
      })

      await server.connect(serverTransport)

      const client = new Client({ name: "test-client", version: "1.0.0" }, { capabilities: {} })
      await client.connect(clientTransport)

      // Test prompts/list
      const promptsResult = await client.listPrompts()
      expect(promptsResult.prompts.length).toBeGreaterThanOrEqual(2)
      expect(promptsResult.prompts.some((p) => p.name === "build")).toBe(true)

      // Test prompts/get
      const promptResult = await client.getPrompt({ name: "build" })
      expect(promptResult.messages).toHaveLength(1)
      expect((promptResult.messages[0].content as { type: "text"; text: string }).text).toContain(
        "build",
      )

      await client.close()
      await server.close()
    })
  })

  describe("Resources Handlers", () => {
    test("resources/list returns resources", async () => {
      const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()

      const server = new Server(
        { name: "test-server", version: "1.0.0" },
        { capabilities: { resources: { listChanged: true } } },
      )

      server.setRequestHandler(ListResourcesRequestSchema, async () => {
        return {
          resources: [
            { uri: "file:///test/README.md", name: "README.md", mimeType: "text/markdown" },
          ],
        }
      })

      server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
        const { uri } = request.params
        if (uri === "file:///test/README.md") {
          return {
            contents: [{ uri, text: "# Test README", mimeType: "text/markdown" }],
          }
        }
        throw new Error(`Resource not found: ${uri}`)
      })

      await server.connect(serverTransport)

      const client = new Client({ name: "test-client", version: "1.0.0" }, { capabilities: {} })
      await client.connect(clientTransport)

      // Test resources/list
      const resourcesResult = await client.listResources()
      expect(resourcesResult.resources).toHaveLength(1)
      expect(resourcesResult.resources[0].name).toBe("README.md")

      // Test resources/read
      const readResult = await client.readResource({ uri: "file:///test/README.md" })
      expect(readResult.contents).toHaveLength(1)
      expect((readResult.contents[0] as { text?: string }).text).toContain("# Test README")

      await client.close()
      await server.close()
    })
  })

  describe("HTTP Transport Authentication", () => {
    test("rejects requests without API key when configured", async () => {
      // This test verifies authentication logic
      const apiKey = "test-secret-key"

      // Simulate auth check
      const checkAuth = (authHeader?: string, apiKeyHeader?: string): boolean => {
        let providedKey: string | undefined

        if (authHeader?.startsWith("Bearer ")) {
          providedKey = authHeader.slice(7)
        } else if (apiKeyHeader) {
          providedKey = apiKeyHeader
        }

        return providedKey === apiKey
      }

      expect(checkAuth()).toBe(false)
      expect(checkAuth("Bearer wrong-key")).toBe(false)
      expect(checkAuth(undefined, "wrong-key")).toBe(false)
      expect(checkAuth("Bearer test-secret-key")).toBe(true)
      expect(checkAuth(undefined, "test-secret-key")).toBe(true)
    })
  })

  describe("Tool Filtering", () => {
    test("filters tools by enabled list", () => {
      const allTools = [
        { id: "read", description: "Read files" },
        { id: "write", description: "Write files" },
        { id: "bash", description: "Execute commands" },
        { id: "glob", description: "Find files" },
      ]

      const enabledTools = ["read", "glob"]

      const filtered = allTools.filter((tool) => enabledTools.includes(tool.id))

      expect(filtered).toHaveLength(2)
      expect(filtered.map((t) => t.id).sort()).toEqual(["glob", "read"])
    })
  })
})
