import z from "zod"
import { Tool } from "../tool"
import { checkChannel } from "./doctor"
import { ReachConfigManager } from "./config"
import { MCP } from "../../mcp"
import { Log } from "@/util/log"

/**
 * Agent Reach - Xiaohongshu Tool
 *
 * Read and search 小红书 notes via MCP server
 */

const log = Log.create({ service: "reach.xiaohongshu" })

const DESCRIPTION = `Read and search 小红书 (Little Red Book) notes.

Actions:
- read: Read a specific note by ID or URL
- search: Search notes by keyword
- feed: Get feed/recommended notes

Examples:
- Read note: { "noteId": "abc123", "action": "read" }
- Search: { "keyword": "旅行攻略", "action": "search", "limit": 10 }
- Feed: { "action": "feed", "limit": 20 }

Requires xiaohongshu MCP server to be configured and running.`

export const XiaohongshuTool = Tool.define("reach_xiaohongshu", {
  description: DESCRIPTION,
  parameters: z.object({
    action: z.enum(["read", "search", "feed"]).default("read").describe("Action to perform"),
    noteId: z.string().optional().describe("Note ID or URL (required for read action)"),
    keyword: z.string().optional().describe("Search keyword (required for search action)"),
    limit: z.number().optional().default(10).describe("Maximum number of results"),
  }),
  async execute(params, ctx) {
    // Check if MCP server is available
    const channelStatus = await checkChannel("xiaohongshu")
    if (channelStatus.status === "off" || channelStatus.status === "error") {
      return {
        title: "小红书 - Unavailable",
        metadata: { error: true },
        output: `小红书 tool unavailable: ${channelStatus.message}`,
      }
    }

    const pattern = params.noteId ?? params.keyword ?? "xiaohongshu"
    await ctx.ask({
      permission: "reach_xiaohongshu",
      patterns: [pattern],
      always: ["*"],
      metadata: { action: params.action },
    })

    const mcpName = await ReachConfigManager.getMcpName("xiaohongshu")
    if (!mcpName) {
      return {
        title: "小红书 - Not Configured",
        metadata: { error: true },
        output: "MCP server not configured for xiaohongshu. Add to ~/.codecoder/reach.json",
      }
    }

    try {
      const clients = await MCP.clients()
      const client = clients[mcpName]

      if (!client) {
        return {
          title: "小红书 - MCP Not Connected",
          metadata: { error: true },
          output: `MCP server '${mcpName}' is not connected. Check MCP status.`,
        }
      }

      // Call appropriate MCP tool based on action
      const toolName = `${mcpName}_${params.action}`
      const tools = await MCP.tools()

      if (!tools[toolName]) {
        // Try alternative naming patterns
        const altNames = [
          `${mcpName}_xhs_${params.action}`,
          `${mcpName}_note_${params.action}`,
          `xhs_${params.action}`,
        ]

        for (const alt of altNames) {
          if (tools[alt]) {
            return await callMcpTool(tools[alt], params)
          }
        }

        return {
          title: "小红书 - Tool Not Found",
          metadata: { error: true, availableTools: Object.keys(tools).filter((k) => k.includes(mcpName)) },
          output: `MCP tool '${toolName}' not found. Available tools: ${Object.keys(tools).filter((k) => k.includes(mcpName)).join(", ")}`,
        }
      }

      return await callMcpTool(tools[toolName], params)
    } catch (error) {
      log.error("xiaohongshu mcp call failed", { action: params.action, error })
      return {
        title: "小红书 - Error",
        metadata: { error: true },
        output: `Failed to call MCP: ${error instanceof Error ? error.message : "Unknown error"}`,
      }
    }
  },
})

async function callMcpTool(
  tool: Awaited<ReturnType<typeof MCP.tools>>[string],
  params: { action: string; noteId?: string; keyword?: string; limit?: number },
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const args: Record<string, unknown> = {}

  if (params.noteId) args.noteId = params.noteId
  if (params.keyword) args.keyword = params.keyword
  if (params.limit) args.limit = params.limit

  // AI SDK tools require toolCallId, messages, and abortSignal
  if (!tool.execute) {
    throw new Error("Tool does not have an execute function")
  }
  const result = await tool.execute(args, {
    toolCallId: `reach_xiaohongshu_${Date.now()}`,
    messages: [],
    abortSignal: new AbortController().signal,
  })

  // Format MCP response
  const content = typeof result === "object" && result !== null && "content" in result
    ? formatMcpContent(result.content)
    : String(result)

  return {
    title: `小红书 - ${params.action}`,
    metadata: { action: params.action },
    output: content,
  }
}

function formatMcpContent(content: unknown): string {
  if (!Array.isArray(content)) return String(content)
  return content
    .map((c: { type?: string; text?: string }) => (c.type === "text" ? c.text : JSON.stringify(c)))
    .join("\n")
}
