import { dynamicTool, type Tool, jsonSchema, type JSONSchema7 } from "ai"
import { McpClientManager, type McpClientConfig, type McpConnectionStatus } from "@codecoder-ai/core"
import { Config } from "../config/config"
import { Log } from "@/util/log"
import { NamedError } from "@codecoder-ai/core/util/error"
import z from "zod/v4"
import { Instance } from "../project/instance"
import { McpOAuthCallback } from "./oauth-callback"
import { McpAuth } from "./auth"
import { BusEvent } from "../bus/bus-event"
import { Bus } from "@/bus"
import { TuiEvent } from "@/cli/cmd/tui/event"
import open from "open"

export namespace MCP {
  const log = Log.create({ service: "mcp" })
  const OAUTH_CALLBACK_PORT = 19876
  const OAUTH_CALLBACK_PATH = "/mcp/oauth/callback"

  export const Resource = z
    .object({
      name: z.string(),
      uri: z.string(),
      description: z.string().optional(),
      mimeType: z.string().optional(),
      client: z.string(),
    })
    .meta({ ref: "McpResource" })
  export type Resource = z.infer<typeof Resource>

  export const ToolsChanged = BusEvent.define(
    "mcp.tools.changed",
    z.object({ server: z.string() }),
  )

  export const BrowserOpenFailed = BusEvent.define(
    "mcp.browser.open.failed",
    z.object({ mcpName: z.string(), url: z.string() }),
  )

  export const Failed = NamedError.create(
    "MCPFailed",
    z.object({ name: z.string() }),
  )

  export const Status = z
    .discriminatedUnion("status", [
      z.object({ status: z.literal("connected") }).meta({ ref: "MCPStatusConnected" }),
      z.object({ status: z.literal("disabled") }).meta({ ref: "MCPStatusDisabled" }),
      z.object({ status: z.literal("failed"), error: z.string() }).meta({ ref: "MCPStatusFailed" }),
      z.object({ status: z.literal("needs_auth") }).meta({ ref: "MCPStatusNeedsAuth" }),
      z.object({ status: z.literal("needs_client_registration"), error: z.string() }).meta({ ref: "MCPStatusNeedsClientRegistration" }),
    ])
    .meta({ ref: "MCPStatus" })
  export type Status = z.infer<typeof Status>

  type McpClientEntry = Config.Mcp | Config.McpDisabled

  function isMcpConfigured(entry: McpClientEntry): entry is Config.Mcp {
    return typeof entry === "object" && entry !== null && "type" in entry
  }

  function getMcpClientEntries(mcp: NonNullable<Config.Info["mcp"]>): Record<string, McpClientEntry> {
    const result: Record<string, McpClientEntry> = {}
    for (const [key, value] of Object.entries(mcp)) {
      if (key === "server") continue
      result[key] = value as McpClientEntry
    }
    return result
  }

  function toClientConfig(name: string, mcp: Config.Mcp): McpClientConfig {
    if (mcp.type === "remote") {
      const oauthConfig = typeof mcp.oauth === "object" ? mcp.oauth : undefined
      return {
        name,
        transport: "http",
        url: mcp.url,
        timeoutMs: mcp.timeout,
        headers: mcp.headers,
        cwd: Instance.directory,
        oauth: oauthConfig ? {
          clientId: oauthConfig.clientId,
          clientSecret: oauthConfig.clientSecret,
          scope: oauthConfig.scope,
        } : undefined,
        oauthDisabled: mcp.oauth === false,
      }
    }
    // mcp.type === "local"
    return {
      name,
      transport: "stdio",
      command: mcp.command,
      environment: mcp.environment,
      timeoutMs: mcp.timeout,
      cwd: Instance.directory,
    }
  }

  function fromCoreStatus(status: McpConnectionStatus): Status {
    if (status.status === "needs_client_registration") {
      return { status: "needs_client_registration", error: status.error ?? "Client registration required" }
    }
    if (status.status === "failed") {
      return { status: "failed", error: status.error ?? "Unknown error" }
    }
    return { status: status.status }
  }

  const state = Instance.state(
    async () => {
      const manager = new McpClientManager()
      if (!manager.isNative) {
        log.warn("MCP native bindings not available, falling back to stub")
        return { manager, status: {} as Record<string, Status> }
      }

      await manager.loadOAuth()
      const cfg = await Config.get()
      const mcpClientEntries = getMcpClientEntries(cfg.mcp ?? {})
      const statusMap: Record<string, Status> = {}

      await Promise.all(
        Object.entries(mcpClientEntries).map(async ([key, mcp]) => {
          if (!isMcpConfigured(mcp)) return
          if (mcp.enabled === false) {
            statusMap[key] = { status: "disabled" }
            return
          }

          const config = toClientConfig(key, mcp)
          const result = await manager.add(key, config).catch((e) => {
            log.error("failed to add MCP client", { key, error: e.message })
            return { status: "failed" as const, error: e.message }
          })

          statusMap[key] = fromCoreStatus(result)

          // Show toast for auth-required statuses
          if (result.status === "needs_auth") {
            Bus.publish(TuiEvent.ToastShow, {
              title: "MCP Authentication Required",
              message: `Server "${key}" requires authentication. Run: codecoder mcp auth ${key}`,
              variant: "warning",
              duration: 8000,
            }).catch((e) => log.debug("failed to show toast", { error: e }))
          }
        }),
      )

      return { manager, status: statusMap }
    },
    async (s) => {
      await s.manager.closeAll().catch((e) => log.error("failed to close MCP clients", { error: e }))
    },
  )

  export async function add(name: string, mcp: Config.Mcp) {
    const s = await state()
    const config = toClientConfig(name, mcp)
    const result = await s.manager.add(name, config).catch((e) => {
      log.error("failed to add MCP client", { name, error: e.message })
      return { status: "failed" as const, error: e.message }
    })
    s.status[name] = fromCoreStatus(result)
    return { status: s.status }
  }

  export async function status() {
    const s = await state()
    const cfg = await Config.get()
    const mcpClientEntries = getMcpClientEntries(cfg.mcp ?? {})
    const result: Record<string, Status> = {}

    for (const [key, mcp] of Object.entries(mcpClientEntries)) {
      if (!isMcpConfigured(mcp)) continue
      result[key] = s.status[key] ?? { status: "disabled" }
    }
    return result
  }

  export async function connect(name: string) {
    const cfg = await Config.get()
    const mcp = cfg.mcp?.[name]
    if (!mcp || !isMcpConfigured(mcp)) return
    await add(name, { ...mcp, enabled: true })
  }

  export async function disconnect(name: string) {
    const s = await state()
    await s.manager.remove(name).catch((e) => log.error("failed to remove MCP client", { name, error: e }))
    s.status[name] = { status: "disabled" }
  }

  /**
   * Returns a map of connected client names to a truthy value.
   * Used by reach tools to check if a client is connected.
   */
  export async function clients(): Promise<Record<string, object>> {
    const s = await state()
    const result: Record<string, object> = {}
    for (const [name, clientStatus] of Object.entries(s.status)) {
      if (clientStatus.status === "connected") {
        result[name] = { connected: true }
      }
    }
    return result
  }

  export async function tools() {
    const s = await state()
    const cfg = await Config.get()
    const defaultTimeout = cfg.experimental?.mcp_timeout
    const result: Record<string, Tool> = {}

    const coreTools = await s.manager.listTools().catch((e) => {
      log.error("failed to list tools", { error: e.message })
      return {}
    })

    for (const [toolKey, mcpTool] of Object.entries(coreTools)) {
      const [clientName] = toolKey.split("_")
      if (s.status[clientName]?.status !== "connected") continue

      const mcpConfig = cfg.mcp?.[clientName]
      const entry = mcpConfig && isMcpConfigured(mcpConfig) ? mcpConfig : undefined
      const timeout = entry?.timeout ?? defaultTimeout

      const inputSchema = mcpTool.inputSchema as Record<string, unknown>
      const schema: JSONSchema7 = {
        ...(inputSchema as JSONSchema7),
        type: "object",
        properties: (inputSchema.properties ?? {}) as JSONSchema7["properties"],
        additionalProperties: false,
      }

      result[toolKey] = dynamicTool({
        description: mcpTool.description ?? "",
        inputSchema: jsonSchema(schema),
        execute: async (args: unknown) => {
          return s.manager.callTool(clientName, mcpTool.name, args)
        },
      })
    }
    return result
  }

  export async function prompts() {
    const s = await state()
    const result: Record<string, { name: string; description?: string; arguments: Array<{ name: string; description?: string; required: boolean }>; client: string }> = {}

    for (const [clientName, clientStatus] of Object.entries(s.status)) {
      if (clientStatus.status !== "connected") continue

      const clientPrompts = await s.manager.listPrompts(clientName).catch((e) => {
        log.error("failed to list prompts", { clientName, error: e.message })
        return []
      })

      for (const prompt of clientPrompts) {
        const key = `${clientName}:${prompt.name}`.replace(/[^a-zA-Z0-9_:-]/g, "_")
        result[key] = { ...prompt, client: clientName }
      }
    }
    return result
  }

  export async function resources() {
    const s = await state()
    const result: Record<string, Resource> = {}

    for (const [clientName, clientStatus] of Object.entries(s.status)) {
      if (clientStatus.status !== "connected") continue

      const clientResources = await s.manager.listResources(clientName).catch((e) => {
        log.error("failed to list resources", { clientName, error: e.message })
        return []
      })

      for (const resource of clientResources) {
        const key = `${clientName}:${resource.name}`.replace(/[^a-zA-Z0-9_:-]/g, "_")
        result[key] = { ...resource, client: clientName }
      }
    }
    return result
  }

  export interface PromptMessageContent {
    type: "text" | "image" | "resource"
    text?: string
    data?: string
    mimeType?: string
    uri?: string
  }

  export interface PromptMessage {
    role: string
    content: PromptMessageContent
  }

  export interface GetPromptResult {
    description?: string
    messages: PromptMessage[]
  }

  export async function getPrompt(clientName: string, name: string, args?: Record<string, string>): Promise<GetPromptResult | undefined> {
    const s = await state()
    const result = await s.manager.getPrompt(clientName, name, args).catch((e) => {
      log.error("failed to get prompt", { clientName, name, error: e.message })
      return undefined
    })
    return result as GetPromptResult | undefined
  }

  export interface ResourceContent {
    uri: string
    mimeType?: string
    text?: string
    blob?: string
  }

  export interface ReadResourceResult {
    contents: ResourceContent[]
  }

  export async function readResource(clientName: string, resourceUri: string): Promise<ReadResourceResult | undefined> {
    const s = await state()
    const result = await s.manager.readResource(clientName, resourceUri).catch((e) => {
      log.error("failed to read resource", { clientName, resourceUri, error: e.message })
      return undefined
    })
    return result as ReadResourceResult | undefined
  }

  // OAuth Browser Flow (ccode-specific)

  function getRedirectUri() {
    return `http://127.0.0.1:${OAUTH_CALLBACK_PORT}${OAUTH_CALLBACK_PATH}`
  }

  export async function startAuth(mcpName: string): Promise<{ authorizationUrl: string }> {
    const cfg = await Config.get()
    const mcpConfig = cfg.mcp?.[mcpName]

    if (!mcpConfig || !isMcpConfigured(mcpConfig)) throw new Error(`MCP server not found: ${mcpName}`)
    if (mcpConfig.type !== "remote") throw new Error(`MCP server ${mcpName} is not a remote server`)
    if (mcpConfig.oauth === false) throw new Error(`MCP server ${mcpName} has OAuth explicitly disabled`)

    await McpOAuthCallback.ensureRunning()

    const oauthState = Array.from(crypto.getRandomValues(new Uint8Array(32)))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("")
    await McpAuth.updateOAuthState(mcpName, oauthState)

    const s = await state()
    const oauthConfig = typeof mcpConfig.oauth === "object" ? mcpConfig.oauth : undefined
    const authUrl = await s.manager.startOAuth(mcpName, mcpConfig.url, getRedirectUri(), oauthConfig)

    return { authorizationUrl: authUrl }
  }

  export async function authenticate(mcpName: string): Promise<Status> {
    const { authorizationUrl } = await startAuth(mcpName)
    if (!authorizationUrl) {
      const s = await state()
      return s.status[mcpName] ?? { status: "connected" }
    }

    const oauthState = await McpAuth.getOAuthState(mcpName)
    if (!oauthState) throw new Error("OAuth state not found")

    log.info("opening browser for oauth", { mcpName, url: authorizationUrl })
    const callbackPromise = McpOAuthCallback.waitForCallback(oauthState)

    try {
      const subprocess = await open(authorizationUrl)
      await new Promise<void>((resolve, reject) => {
        const timeout = setTimeout(() => resolve(), 500)
        subprocess.on("error", (error) => { clearTimeout(timeout); reject(error) })
        subprocess.on("exit", (code) => { if (code !== null && code !== 0) { clearTimeout(timeout); reject(new Error(`Browser open failed with exit code ${code}`)) } })
      })
    } catch {
      log.warn("failed to open browser, user must open URL manually", { mcpName })
      Bus.publish(BrowserOpenFailed, { mcpName, url: authorizationUrl })
    }

    const code = await callbackPromise

    const storedState = await McpAuth.getOAuthState(mcpName)
    if (storedState !== oauthState) {
      await McpAuth.clearOAuthState(mcpName)
      throw new Error("OAuth state mismatch - potential CSRF attack")
    }
    await McpAuth.clearOAuthState(mcpName)

    return finishAuth(mcpName, code, oauthState)
  }

  export async function finishAuth(mcpName: string, authorizationCode: string, oauthState: string): Promise<Status> {
    const s = await state()
    await s.manager.finishOAuth(mcpName, authorizationCode, oauthState)

    const cfg = await Config.get()
    const mcpConfig = cfg.mcp?.[mcpName]
    if (!mcpConfig || !isMcpConfigured(mcpConfig)) throw new Error(`MCP server not found: ${mcpName}`)

    const result = await add(mcpName, mcpConfig)
    const statusRecord = result.status as Record<string, Status>
    return statusRecord[mcpName] ?? { status: "failed", error: "Unknown error after auth" }
  }

  export async function removeAuth(mcpName: string): Promise<void> {
    const s = await state()
    await s.manager.removeOAuth(mcpName)
    McpOAuthCallback.cancelPending(mcpName)
    await McpAuth.clearOAuthState(mcpName)
    log.info("removed oauth credentials", { mcpName })
  }

  export async function supportsOAuth(mcpName: string): Promise<boolean> {
    const cfg = await Config.get()
    const mcpConfig = cfg.mcp?.[mcpName]
    if (!mcpConfig || !isMcpConfigured(mcpConfig)) return false
    return mcpConfig.type === "remote" && mcpConfig.oauth !== false
  }

  export async function hasStoredTokens(mcpName: string): Promise<boolean> {
    const s = await state()
    return s.manager.hasOAuthCredentials(mcpName)
  }

  export type AuthStatus = "authenticated" | "expired" | "not_authenticated"

  export async function getAuthStatus(mcpName: string): Promise<AuthStatus> {
    const s = await state()
    return s.manager.getOAuthStatus(mcpName)
  }
}

// Re-export McpServer for MCP server functionality
export { McpServer } from "./server"
