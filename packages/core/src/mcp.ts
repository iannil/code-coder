/**
 * MCP Client Manager - TypeScript wrapper for native Rust implementation
 *
 * This module provides a high-level API for managing MCP (Model Context Protocol)
 * clients with OAuth 2.0 PKCE authentication support.
 *
 * @example
 * ```typescript
 * import { McpClientManager } from '@codecoder-ai/core'
 *
 * const manager = new McpClientManager()
 * await manager.loadOAuth() // Load stored credentials
 *
 * // Connect to an MCP server
 * const status = await manager.add('my-server', {
 *   name: 'my-server',
 *   transport: 'http',
 *   url: 'https://api.example.com/mcp'
 * })
 *
 * if (status.status === 'needs_auth') {
 *   // Start OAuth flow
 *   const authUrl = await manager.startOAuth(
 *     'my-server',
 *     'https://api.example.com',
 *     'http://localhost:3000/callback'
 *   )
 *   // Open authUrl in browser...
 * }
 *
 * // List available tools
 * const tools = await manager.listTools()
 *
 * // Call a tool
 * const result = await manager.callTool('my-server', 'search', { query: 'hello' })
 * ```
 */

import type {
  AuthStatus,
  IMcpClientManager,
  McpClientConfig,
  McpConnectionStatus,
  McpContentItem,
  McpPrompt,
  McpPromptMessage,
  McpPromptResult,
  McpResource,
  McpTool,
  McpToolResult,
  OAuthConfig,
} from './protocol.js'

import type { McpClientManagerHandle } from './binding.d.ts'

interface NativeConfig {
  name: string
  transport: string
  command?: string[]
  url?: string
  environment?: Record<string, string>
  timeout_ms?: number
  headers?: Record<string, string>
  cwd?: string
  oauth?: NativeOAuthConfig
  oauth_disabled?: boolean
}

interface NativeOAuthConfig {
  client_id?: string
  client_secret?: string
  scope?: string
}

interface NativeStatus {
  status: string
  error?: string
}

interface NativeTool {
  name: string
  description: string
  inputSchema: unknown
}

interface NativeToolResult {
  content: unknown[]
  isError: boolean
}

// Try to load the native MCP client manager factory
let createNativeMcpClientManager: (() => McpClientManagerHandle) | null = null

try {
  const bindings = await import('./binding.js')
  createNativeMcpClientManager = bindings.createMcpClientManager
} catch {
  // Native bindings not available
}

/**
 * Convert TypeScript config to native config format
 */
function toNativeConfig(config: McpClientConfig): NativeConfig {
  return {
    name: config.name,
    transport: config.transport,
    command: config.command,
    url: config.url,
    environment: config.environment,
    timeout_ms: config.timeoutMs,
    headers: config.headers,
    cwd: config.cwd,
    oauth: config.oauth ? {
      client_id: config.oauth.clientId,
      client_secret: config.oauth.clientSecret,
      scope: config.oauth.scope,
    } : undefined,
    oauth_disabled: config.oauthDisabled,
  }
}

/**
 * Convert native OAuth config to native format
 */
function toNativeOAuthConfig(config?: OAuthConfig): NativeOAuthConfig | undefined {
  return config ? {
    client_id: config.clientId,
    client_secret: config.clientSecret,
    scope: config.scope,
  } : undefined
}

/**
 * Convert native status to TypeScript format
 */
function fromNativeStatus(status: NativeStatus): McpConnectionStatus {
  return {
    status: status.status as McpConnectionStatus['status'],
    error: status.error,
  }
}

/**
 * Convert native tool to TypeScript format
 */
function fromNativeTool(tool: NativeTool): McpTool {
  return {
    name: tool.name,
    description: tool.description,
    inputSchema: tool.inputSchema,
  }
}

/**
 * Convert native tool result to TypeScript format
 */
function fromNativeToolResult(result: NativeToolResult): McpToolResult {
  return {
    content: result.content.map(c => {
      const item = c as Record<string, unknown>
      const contentItem: McpContentItem = {
        type: (item.type as 'text' | 'image' | 'resource') || 'text',
        text: item.text as string | undefined,
        data: item.data as string | undefined,
        mimeType: item.mimeType as string | undefined,
        uri: item.uri as string | undefined,
      }
      return contentItem
    }),
    isError: result.isError,
  }
}

/**
 * Convert native auth status to TypeScript format
 */
function fromNativeAuthStatus(status: string): AuthStatus {
  switch (status) {
    case 'Authenticated':
      return 'authenticated'
    case 'Expired':
      return 'expired'
    default:
      return 'not_authenticated'
  }
}

/**
 * MCP Client Manager implementation using native Rust bindings
 */
export class McpClientManager implements IMcpClientManager {
  private handle: McpClientManagerHandle | null = null

  constructor() {
    if (createNativeMcpClientManager) {
      this.handle = createNativeMcpClientManager()
    }
  }

  /**
   * Check if native implementation is available
   */
  get isNative(): boolean {
    return this.handle !== null
  }

  private ensureHandle(): McpClientManagerHandle {
    if (!this.handle) {
      throw new Error('MCP client manager native bindings not available')
    }
    return this.handle
  }

  async add(name: string, config: McpClientConfig): Promise<McpConnectionStatus> {
    const handle = this.ensureHandle()
    const nativeConfig = toNativeConfig(config)
    const status = await handle.add(name, nativeConfig)
    return fromNativeStatus(status)
  }

  async status(): Promise<Record<string, McpConnectionStatus>> {
    const handle = this.ensureHandle()
    const statuses = await handle.status()
    const result: Record<string, McpConnectionStatus> = {}
    for (const [name, status] of Object.entries(statuses)) {
      result[name] = fromNativeStatus(status)
    }
    return result
  }

  async listTools(): Promise<Record<string, McpTool>> {
    const handle = this.ensureHandle()
    const tools = await handle.listTools()
    const result: Record<string, McpTool> = {}
    for (const [name, tool] of Object.entries(tools)) {
      result[name] = fromNativeTool(tool)
    }
    return result
  }

  async callTool(clientName: string, toolName: string, args: unknown): Promise<McpToolResult> {
    const handle = this.ensureHandle()
    const result = await handle.callTool(clientName, toolName, args)
    return fromNativeToolResult(result)
  }

  async remove(name: string): Promise<void> {
    const handle = this.ensureHandle()
    await handle.remove(name)
  }

  async closeAll(): Promise<void> {
    const handle = this.ensureHandle()
    await handle.closeAll()
  }

  // Resource methods

  async listResources(clientName: string): Promise<McpResource[]> {
    const handle = this.ensureHandle()
    const resources = await handle.listResources(clientName)
    return resources.map((r: { uri: string; name: string; description?: string; mime_type?: string }) => ({
      uri: r.uri,
      name: r.name,
      description: r.description,
      mimeType: r.mime_type,
    }))
  }

  async readResource(clientName: string, uri: string): Promise<unknown> {
    const handle = this.ensureHandle()
    return handle.readResource(clientName, uri)
  }

  // Prompt methods

  async listPrompts(clientName: string): Promise<McpPrompt[]> {
    const handle = this.ensureHandle()
    const prompts = await handle.listPrompts(clientName)
    return prompts.map((p: { name: string; description?: string; arguments: Array<{ name: string; description?: string; required: boolean }> }) => ({
      name: p.name,
      description: p.description,
      arguments: p.arguments.map(a => ({
        name: a.name,
        description: a.description,
        required: a.required,
      })),
    }))
  }

  async getPrompt(clientName: string, promptName: string, args?: Record<string, unknown>): Promise<McpPromptResult> {
    const handle = this.ensureHandle()
    const result = await handle.getPrompt(clientName, promptName, args)
    return {
      description: result.description,
      messages: result.messages as McpPromptMessage[],
    }
  }

  // OAuth methods

  async loadOAuth(): Promise<void> {
    const handle = this.ensureHandle()
    await handle.loadOauth()
  }

  async startOAuth(
    serverName: string,
    serverUrl: string,
    redirectUri: string,
    config?: OAuthConfig
  ): Promise<string> {
    const handle = this.ensureHandle()
    return handle.startOauth(serverName, serverUrl, redirectUri, toNativeOAuthConfig(config))
  }

  async finishOAuth(serverName: string, authorizationCode: string, state: string): Promise<void> {
    const handle = this.ensureHandle()
    await handle.finishOauth(serverName, authorizationCode, state)
  }

  async removeOAuth(serverName: string): Promise<void> {
    const handle = this.ensureHandle()
    await handle.removeOauth(serverName)
  }

  async getOAuthStatus(serverName: string): Promise<AuthStatus> {
    const handle = this.ensureHandle()
    const status = await handle.getOauthStatus(serverName)
    return fromNativeAuthStatus(status)
  }

  async hasOAuthCredentials(serverName: string): Promise<boolean> {
    const handle = this.ensureHandle()
    return handle.hasOauthCredentials(serverName)
  }

  async cancelOAuth(serverName: string): Promise<void> {
    const handle = this.ensureHandle()
    await handle.cancelOauth(serverName)
  }
}

/**
 * Check if MCP native bindings are available
 */
export const isMcpNative = createNativeMcpClientManager !== null
