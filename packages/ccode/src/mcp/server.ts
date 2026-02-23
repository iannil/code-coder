/**
 * MCP Server - Exposes CodeCoder's tools, prompts, and resources via Model Context Protocol
 *
 * This server allows external clients (like ZeroBot) to call CodeCoder's
 * 20+ built-in tools through the standardized MCP protocol.
 *
 * Architecture:
 * ┌─────────────────────────────────────────────────────────────┐
 * │                      CodeCoder                               │
 * │  ┌─────────────────────────────────────────────────────┐    │
 * │  │              MCP Server                               │    │
 * │  │  Exposes 20+ tools, prompts, resources via MCP       │    │
 * │  └─────────────────────────────────────────────────────┘    │
 * │                          ▲                                   │
 * └──────────────────────────│───────────────────────────────────┘
 *                            │ MCP Protocol (stdio/HTTP)
 *                            │
 * ┌──────────────────────────│───────────────────────────────────┐
 * │                      ZeroBot                                 │
 * │  ┌─────────────────────────────────────────────────────┐    │
 * │  │              MCP Client                               │    │
 * │  │  Connects to CodeCoder MCP Server                     │    │
 * │  └─────────────────────────────────────────────────────┘    │
 * └─────────────────────────────────────────────────────────────┘
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js"
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js"
import { WebStandardStreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/webStandardStreamableHttp.js"
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListPromptsRequestSchema,
  GetPromptRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  ListResourceTemplatesRequestSchema,
  SetLevelRequestSchema,
  type CallToolResult,
  type Tool as MCPToolDefinition,
  type Prompt as MCPPrompt,
  type Resource as MCPResource,
  type ResourceTemplate as MCPResourceTemplate,
  type GetPromptResult,
  type PromptMessage,
} from "@modelcontextprotocol/sdk/types.js"
import z from "zod/v4"
import { ToolRegistry } from "../tool/registry"
import { Tool } from "../tool/tool"
import { Agent } from "../agent/agent"
import { VERSION } from "../version"
import { Log } from "@/util/log"
import { ulid } from "ulid"
import { Hono } from "hono"
import { Instance } from "../project/instance"
import { Config } from "../config/config"
import path from "path"

export namespace McpServer {
  const log = Log.create({ service: "mcp.server" })

  /** Default HTTP port for MCP server */
  export const DEFAULT_PORT = 4420

  /** Options for starting the MCP server */
  export interface ServeOptions {
    /** Transport mode: stdio (default) or http */
    transport: "stdio" | "http"
    /** Port for HTTP transport (default: 4420) */
    port?: number
    /** API key for authentication (HTTP only) */
    apiKey?: string
    /** Session ID for context */
    sessionID?: string
    /** Model info for tool filtering */
    model?: {
      providerID: string
      modelID: string
    }
    /** Filter tools by specific agent */
    agentFilter?: string
    /** Only enable specific tools */
    enabledTools?: string[]
    /** Glob patterns for additional resources to expose */
    resourcePatterns?: string[]
  }

  /** Context for tool execution */
  interface ToolContext {
    sessionID: string
    messageID: string
    agent: string
    abort: AbortSignal
    metadata: (input: { title?: string; metadata?: Record<string, unknown> }) => void
    ask: () => Promise<void>
  }

  /** Tool info with initialized execute function */
  interface InitializedTool {
    id: string
    description: string
    parameters: z.ZodType
    execute: (
      args: unknown,
      ctx: Tool.Context,
    ) => Promise<{
      title: string
      metadata: Record<string, unknown>
      output: string
    }>
  }

  /** Convert Zod schema to JSON Schema format for MCP */
  function zodToJsonSchema(schema: z.ZodType): Record<string, unknown> {
    try {
      return z.toJSONSchema(schema) as Record<string, unknown>
    } catch {
      return { type: "object", properties: {} }
    }
  }

  /** Convert CodeCoder Tool to MCP Tool definition */
  function toMcpToolDefinition(tool: InitializedTool): MCPToolDefinition {
    const jsonSchema = zodToJsonSchema(tool.parameters)
    return {
      name: tool.id,
      description: tool.description,
      inputSchema: {
        type: "object",
        properties: (jsonSchema.properties ?? {}) as Record<string, object>,
        required: (jsonSchema.required ?? []) as string[],
      },
    }
  }

  /** Create a tool execution context */
  function createToolContext(sessionID: string, messageID: string): ToolContext {
    const controller = new AbortController()
    return {
      sessionID,
      messageID,
      agent: "mcp-server",
      abort: controller.signal,
      metadata: () => {},
      ask: async () => {},
    }
  }

  /** Convert Agent.Info to MCP Prompt */
  function toMcpPrompt(agent: Agent.Info): MCPPrompt {
    return {
      name: agent.name,
      description: agent.description ?? `Agent: ${agent.name}`,
      arguments: [],
    }
  }

  /** Create MCP server with capabilities */
  function createServer(toolCount: number, hasPrompts: boolean, hasResources: boolean): Server {
    return new Server(
      {
        name: "codecoder",
        version: VERSION,
      },
      {
        capabilities: {
          tools: {
            listChanged: true,
          },
          ...(hasPrompts && {
            prompts: {
              listChanged: true,
            },
          }),
          ...(hasResources && {
            resources: {
              listChanged: true,
            },
          }),
          logging: {},
        },
        instructions: `CodeCoder MCP Server - Provides access to ${toolCount} development tools including:
- File operations: read, write, edit, glob, grep
- Shell execution: bash
- Web operations: webfetch, websearch
- Code search and analysis
- Task management and more

Use these tools to assist with software engineering tasks.`,
      },
    )
  }

  /** Register logging handlers on server */
  function registerLoggingHandlers(server: Server) {
    server.setRequestHandler(SetLevelRequestSchema, async (request) => {
      const { level } = request.params
      log.info("mcp log level changed", { level })
      return {}
    })
  }

  /** Register tool handlers on server */
  function registerToolHandlers(
    server: Server,
    tools: Map<string, InitializedTool>,
    sessionID: string,
  ) {
    server.setRequestHandler(ListToolsRequestSchema, async () => {
      const mcpTools: MCPToolDefinition[] = []
      for (const tool of tools.values()) {
        mcpTools.push(toMcpToolDefinition(tool))
      }
      log.debug("listing tools", { count: mcpTools.length })
      return { tools: mcpTools }
    })

    server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params
      const messageID = ulid()

      log.info("executing tool", { name, messageID })

      const tool = tools.get(name)
      if (!tool) {
        log.warn("tool not found", { name })
        return {
          content: [{ type: "text" as const, text: `Error: Tool "${name}" not found` }],
          isError: true,
        } satisfies CallToolResult
      }

      try {
        const ctx = createToolContext(sessionID, messageID)
        const result = await tool.execute(args ?? {}, ctx)
        log.info("tool executed", { name, messageID, title: result.title })

        return {
          content: [{ type: "text" as const, text: result.output }],
          isError: false,
        } satisfies CallToolResult
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error)
        log.error("tool execution failed", { name, messageID, error: errorMessage })

        return {
          content: [{ type: "text" as const, text: `Error executing ${name}: ${errorMessage}` }],
          isError: true,
        } satisfies CallToolResult
      }
    })
  }

  /** Register prompts handlers on server */
  function registerPromptsHandlers(server: Server, agents: Map<string, Agent.Info>) {
    server.setRequestHandler(ListPromptsRequestSchema, async () => {
      const prompts: MCPPrompt[] = []
      for (const agent of agents.values()) {
        if (!agent.hidden) {
          prompts.push(toMcpPrompt(agent))
        }
      }
      log.debug("listing prompts", { count: prompts.length })
      return { prompts }
    })

    server.setRequestHandler(GetPromptRequestSchema, async (request) => {
      const { name, arguments: args } = request.params
      log.info("getting prompt", { name })

      const agent = agents.get(name)
      if (!agent) {
        throw new Error(`Prompt "${name}" not found`)
      }

      const messages: PromptMessage[] = []

      if (agent.prompt) {
        let content = agent.prompt

        // Replace argument placeholders if provided
        if (args) {
          for (const [key, value] of Object.entries(args)) {
            content = content.replace(new RegExp(`\\{${key}\\}`, "g"), String(value))
          }
        }

        messages.push({
          role: "user",
          content: { type: "text", text: content },
        })
      }

      return {
        description: agent.description ?? `Agent prompt: ${agent.name}`,
        messages,
      } satisfies GetPromptResult
    })
  }

  /** Register resources handlers on server */
  function registerResourcesHandlers(
    server: Server,
    workdir: string,
    resourcePatterns?: string[],
  ) {
    // Register resource templates handler (for glob patterns)
    server.setRequestHandler(ListResourceTemplatesRequestSchema, async () => {
      const templates: MCPResourceTemplate[] = []

      if (resourcePatterns?.length) {
        for (const pattern of resourcePatterns) {
          // Convert glob pattern to URI template format
          const uriTemplate = `file://${workdir}/${pattern.replace(/\*\*\/?\*/g, "{+path}")}`
          templates.push({
            uriTemplate,
            name: pattern,
            description: `Files matching ${pattern}`,
          })
        }
      }

      log.debug("listing resource templates", { count: templates.length })
      return { resourceTemplates: templates }
    })

    server.setRequestHandler(ListResourcesRequestSchema, async () => {
      const resources: MCPResource[] = []
      const seenPaths = new Set<string>()

      // Add CLAUDE.md as a primary resource if it exists
      const claudeMdPath = path.join(workdir, "CLAUDE.md")
      if (await Bun.file(claudeMdPath).exists()) {
        resources.push({
          uri: `file://${claudeMdPath}`,
          name: "CLAUDE.md",
          description: "Project instructions for AI assistants",
          mimeType: "text/markdown",
        })
        seenPaths.add(claudeMdPath)
      }

      // Add README.md if it exists
      const readmePath = path.join(workdir, "README.md")
      if (await Bun.file(readmePath).exists()) {
        resources.push({
          uri: `file://${readmePath}`,
          name: "README.md",
          description: "Project documentation",
          mimeType: "text/markdown",
        })
        seenPaths.add(readmePath)
      }

      // Add package.json if it exists
      const packagePath = path.join(workdir, "package.json")
      if (await Bun.file(packagePath).exists()) {
        resources.push({
          uri: `file://${packagePath}`,
          name: "package.json",
          description: "Project package configuration",
          mimeType: "application/json",
        })
        seenPaths.add(packagePath)
      }

      // Scan files matching configured glob patterns
      if (resourcePatterns?.length) {
        for (const pattern of resourcePatterns) {
          const glob = new Bun.Glob(pattern)
          for await (const file of glob.scan({ cwd: workdir, absolute: true })) {
            // Skip already-added files
            if (seenPaths.has(file)) continue
            seenPaths.add(file)

            resources.push({
              uri: `file://${file}`,
              name: path.relative(workdir, file),
              mimeType: getMimeType(file),
            })
          }
        }
      }

      log.debug("listing resources", { count: resources.length })
      return { resources }
    })

    server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params
      log.info("reading resource", { uri })

      if (!uri.startsWith("file://")) {
        throw new Error(`Unsupported URI scheme: ${uri}`)
      }

      const filePath = uri.slice(7) // Remove "file://"

      // Security check: ensure file is within workdir
      const resolvedPath = path.resolve(filePath)
      const resolvedWorkdir = path.resolve(workdir)
      if (!resolvedPath.startsWith(resolvedWorkdir)) {
        throw new Error(`Access denied: ${uri}`)
      }

      const file = Bun.file(filePath)
      if (!(await file.exists())) {
        throw new Error(`Resource not found: ${uri}`)
      }

      const content = await file.text()
      const mimeType = getMimeType(filePath)

      return {
        contents: [
          {
            uri,
            text: content,
            mimeType,
          },
        ],
      }
    })
  }

  /** Get MIME type from file extension */
  function getMimeType(filePath: string): string {
    const ext = path.extname(filePath).toLowerCase()
    const mimeTypes: Record<string, string> = {
      ".md": "text/markdown",
      ".txt": "text/plain",
      ".json": "application/json",
      ".js": "text/javascript",
      ".ts": "text/typescript",
      ".tsx": "text/typescript",
      ".jsx": "text/javascript",
      ".html": "text/html",
      ".css": "text/css",
      ".yaml": "text/yaml",
      ".yml": "text/yaml",
      ".toml": "text/toml",
      ".xml": "text/xml",
    }
    return mimeTypes[ext] ?? "text/plain"
  }

  /** Initialize tools based on options */
  async function initializeTools(options: ServeOptions): Promise<Map<string, InitializedTool>> {
    const modelInfo = options.model ?? { providerID: "ccode", modelID: "default" }
    const agent = options.agentFilter ? await Agent.get(options.agentFilter) : undefined
    const toolDefs = await ToolRegistry.tools(modelInfo, agent)
    const tools = new Map<string, InitializedTool>()

    for (const tool of toolDefs) {
      // Apply tool filtering if specified
      if (options.enabledTools && !options.enabledTools.includes(tool.id)) {
        continue
      }
      tools.set(tool.id, tool)
    }

    return tools
  }

  /** Initialize agents for prompts */
  async function initializeAgents(): Promise<Map<string, Agent.Info>> {
    const agentList = await Agent.list()
    const agents = new Map<string, Agent.Info>()

    for (const agent of agentList) {
      agents.set(agent.name, agent)
    }

    return agents
  }

  /** Start the MCP server with stdio transport */
  async function serveStdio(
    server: Server,
    sessionID: string,
  ): Promise<void> {
    const stdioTransport = new StdioServerTransport()
    await server.connect(stdioTransport)
    log.info("mcp server started", { transport: "stdio", sessionID })

    // Keep process alive
    await new Promise<void>((resolve) => {
      process.on("SIGINT", () => {
        log.info("received SIGINT, shutting down")
        server.close().then(resolve).catch(resolve)
      })
      process.on("SIGTERM", () => {
        log.info("received SIGTERM, shutting down")
        server.close().then(resolve).catch(resolve)
      })
    })
  }

  /** Start the MCP server with HTTP transport */
  async function serveHttp(
    tools: Map<string, InitializedTool>,
    agents: Map<string, Agent.Info>,
    workdir: string,
    options: ServeOptions,
    sessionID: string,
  ): Promise<void> {
    const port = options.port ?? DEFAULT_PORT

    // Track active sessions (each session has its own server and transport)
    const sessions = new Map<
      string,
      { server: Server; transport: WebStandardStreamableHTTPServerTransport }
    >()

    const app = new Hono()

    // API Key authentication middleware for /mcp routes
    if (options.apiKey) {
      app.use("/mcp/*", async (c, next) => {
        const authHeader = c.req.header("Authorization")
        const apiKeyHeader = c.req.header("X-API-Key")

        let providedKey: string | undefined

        if (authHeader?.startsWith("Bearer ")) {
          providedKey = authHeader.slice(7)
        } else if (apiKeyHeader) {
          providedKey = apiKeyHeader
        }

        if (providedKey !== options.apiKey) {
          log.warn("authentication failed", { path: c.req.path })
          return c.json({ error: "Unauthorized" }, 401)
        }

        await next()
      })
    }

    // Handle all MCP requests
    app.all("/mcp", async (c) => {
      // Get or create session
      const sessionHeader = c.req.header("Mcp-Session-Id")
      let session = sessionHeader ? sessions.get(sessionHeader) : undefined

      if (!session) {
        // Create new transport and server for this session
        const transport = new WebStandardStreamableHTTPServerTransport({
          sessionIdGenerator: () => ulid(),
          onsessioninitialized: (sid) => {
            sessions.set(sid, session!)
            log.info("session initialized", { sessionId: sid })
          },
          onsessionclosed: (sid) => {
            sessions.delete(sid)
            log.info("session closed", { sessionId: sid })
          },
        })

        // Create new server for this session
        const server = createServer(tools.size, agents.size > 0, true)
        registerToolHandlers(server, tools, sessionID)
        registerPromptsHandlers(server, agents)
        registerResourcesHandlers(server, workdir, options.resourcePatterns)
        registerLoggingHandlers(server)

        server.onclose = () => {
          log.debug("session server closed", { sessionId: transport.sessionId })
        }

        server.onerror = (error) => {
          log.error("session server error", { error: error.message })
        }

        // Connect server to transport
        await server.connect(transport)

        session = { server, transport }
      }

      // Handle the request
      const response = await session.transport.handleRequest(c.req.raw)
      return response
    })

    // Health check endpoint
    app.get("/health", (c) => c.json({ status: "ok", version: VERSION }))

    // Start the server
    const bunServer = Bun.serve({
      port,
      fetch: app.fetch,
    })

    log.info("mcp server started", {
      transport: "http",
      port,
      sessionID,
      authenticated: !!options.apiKey,
    })

    // Graceful shutdown
    await new Promise<void>((resolve) => {
      const shutdown = async () => {
        log.info("shutting down http server")

        // Close all sessions
        for (const { server, transport } of sessions.values()) {
          await transport.close().catch(() => {})
          await server.close().catch(() => {})
        }
        sessions.clear()

        bunServer.stop()
        resolve()
      }

      process.on("SIGINT", shutdown)
      process.on("SIGTERM", shutdown)
    })
  }

  /** Start the MCP server */
  export async function serve(cliOptions: ServeOptions): Promise<void> {
    // Load config and merge: CLI params > config file > defaults
    const cfg = await Config.get()
    const serverConfig = cfg.mcp?.server

    const options: ServeOptions = {
      transport: cliOptions.transport ?? serverConfig?.defaultTransport ?? "stdio",
      port: cliOptions.port ?? serverConfig?.port ?? DEFAULT_PORT,
      apiKey: cliOptions.apiKey ?? serverConfig?.apiKey,
      sessionID: cliOptions.sessionID,
      model: cliOptions.model,
      agentFilter: cliOptions.agentFilter,
      enabledTools: cliOptions.enabledTools,
      resourcePatterns: cliOptions.resourcePatterns ?? serverConfig?.resources,
    }

    const { transport, sessionID = ulid() } = options

    log.info("starting mcp server", {
      transport,
      sessionID,
      port: options.port,
      hasApiKey: !!options.apiKey,
      configSource: serverConfig ? "config+cli" : "cli",
    })

    // Initialize tools
    const tools = await initializeTools(options)
    log.info("loaded tools", { count: tools.size, tools: Array.from(tools.keys()) })

    // Initialize agents for prompts
    const agents = await initializeAgents()
    log.info("loaded agents", { count: agents.size })

    // Get workdir for resources
    const workdir = Instance.directory

    // Start with appropriate transport
    if (transport === "stdio") {
      // Create server with all capabilities
      const server = createServer(tools.size, agents.size > 0, true)

      // Register all handlers
      registerToolHandlers(server, tools, sessionID)
      registerPromptsHandlers(server, agents)
      registerResourcesHandlers(server, workdir, options.resourcePatterns)
      registerLoggingHandlers(server)

      // Handle server events
      server.onclose = () => {
        log.info("server closed")
      }

      server.onerror = (error) => {
        log.error("server error", { error: error.message })
      }

      await serveStdio(server, sessionID)
    } else {
      // HTTP mode creates server instances per session
      await serveHttp(tools, agents, workdir, options, sessionID)
    }
  }

  /** Get list of available tool names */
  export async function listToolNames(): Promise<string[]> {
    return ToolRegistry.ids()
  }
}
