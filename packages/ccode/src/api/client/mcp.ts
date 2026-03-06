/**
 * MCP API Client - Model Context Protocol via the zero-api service
 */

import z from "zod"
import { getClient, type ZeroClient } from "./index"

// ============================================================================
// MCP Types (matching zero-api)
// ============================================================================

/** MCP tool info */
export const McpToolInfo = z.object({
  name: z.string(),
  description: z.string(),
  input_schema: z.record(z.string(), z.unknown()),
})

export type McpToolInfo = z.infer<typeof McpToolInfo>

/** MCP content block */
export const McpContent = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("text"),
    text: z.string(),
  }),
  z.object({
    type: z.literal("image"),
    data: z.string(),
    mime_type: z.string(),
  }),
  z.object({
    type: z.literal("resource"),
    uri: z.string(),
    text: z.string().optional(),
  }),
])

export type McpContent = z.infer<typeof McpContent>

/** MCP call request */
export const McpCallRequest = z.object({
  name: z.string(),
  arguments: z.record(z.string(), z.unknown()),
})

export type McpCallRequest = z.infer<typeof McpCallRequest>

/** MCP call response */
export const McpCallResponse = z.object({
  content: z.array(McpContent),
  is_error: z.boolean(),
})

export type McpCallResponse = z.infer<typeof McpCallResponse>

// ============================================================================
// MCP Client
// ============================================================================

/**
 * MCP API client
 */
export class McpClient {
  constructor(private client: ZeroClient = getClient()) {}

  /**
   * List available MCP tools
   */
  async listTools(): Promise<McpToolInfo[]> {
    return this.client.get<McpToolInfo[]>("/api/v1/mcp/tools")
  }

  /**
   * Call an MCP tool
   */
  async call(request: McpCallRequest): Promise<McpCallResponse> {
    return this.client.post<McpCallResponse>("/api/v1/mcp/call", request)
  }

  /**
   * Call an MCP tool with typed arguments
   */
  async callTool(name: string, args: Record<string, unknown>): Promise<McpCallResponse> {
    return this.call({ name, arguments: args })
  }

  /**
   * Call a tool and extract text content
   */
  async callForText(name: string, args: Record<string, unknown>): Promise<string> {
    const response = await this.callTool(name, args)
    if (response.is_error) {
      const errorText = response.content
        .filter((c): c is { type: "text"; text: string } => c.type === "text")
        .map((c) => c.text)
        .join("\n")
      throw new Error(`MCP tool error: ${errorText}`)
    }
    return response.content
      .filter((c): c is { type: "text"; text: string } => c.type === "text")
      .map((c) => c.text)
      .join("\n")
  }
}

/**
 * Singleton MCP client
 */
let mcpClient: McpClient | null = null

/**
 * Get the MCP client
 */
export function getMcpClient(): McpClient {
  if (!mcpClient) {
    mcpClient = new McpClient()
  }
  return mcpClient
}

/**
 * Namespace for MCP API
 */
export namespace ZeroMcp {
  export const listTools = () => getMcpClient().listTools()
  export const call = (request: McpCallRequest) => getMcpClient().call(request)
  export const callTool = (name: string, args: Record<string, unknown>) => getMcpClient().callTool(name, args)
  export const callForText = (name: string, args: Record<string, unknown>) => getMcpClient().callForText(name, args)
}
