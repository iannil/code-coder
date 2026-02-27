import z from "zod"
import { Tool } from "../tool"
import { checkChannel } from "./doctor"
import { ReachConfigManager } from "./config"
import { MCP } from "../../mcp"
import { Log } from "@/util/log"

/**
 * Agent Reach - Douyin Tool
 *
 * Read and search 抖音 videos via MCP server
 */

const log = Log.create({ service: "reach.douyin" })

const DESCRIPTION = `Read and search 抖音 (TikTok China) videos.

Actions:
- read: Read a specific video by ID or URL
- search: Search videos by keyword
- user: Get user profile and videos

Examples:
- Read video: { "videoId": "abc123", "action": "read" }
- Search: { "keyword": "搞笑视频", "action": "search", "limit": 10 }
- User: { "userId": "user123", "action": "user" }

Requires douyin MCP server to be configured and running.`

export const DouyinTool = Tool.define("reach_douyin", {
  description: DESCRIPTION,
  parameters: z.object({
    action: z.enum(["read", "search", "user"]).default("read").describe("Action to perform"),
    videoId: z.string().optional().describe("Video ID or URL (required for read action)"),
    keyword: z.string().optional().describe("Search keyword (required for search action)"),
    userId: z.string().optional().describe("User ID (required for user action)"),
    limit: z.number().optional().default(10).describe("Maximum number of results"),
  }),
  async execute(params, ctx) {
    // Check if MCP server is available
    const channelStatus = await checkChannel("douyin")
    if (channelStatus.status === "off" || channelStatus.status === "error") {
      return {
        title: "抖音 - Unavailable",
        metadata: { error: true },
        output: `抖音 tool unavailable: ${channelStatus.message}`,
      }
    }

    const pattern = params.videoId ?? params.keyword ?? params.userId ?? "douyin"
    await ctx.ask({
      permission: "reach_douyin",
      patterns: [pattern],
      always: ["*"],
      metadata: { action: params.action },
    })

    const mcpName = await ReachConfigManager.getMcpName("douyin")
    if (!mcpName) {
      return {
        title: "抖音 - Not Configured",
        metadata: { error: true },
        output: "MCP server not configured for douyin. Add to ~/.codecoder/reach.json",
      }
    }

    try {
      const clients = await MCP.clients()
      const client = clients[mcpName]

      if (!client) {
        return {
          title: "抖音 - MCP Not Connected",
          metadata: { error: true },
          output: `MCP server '${mcpName}' is not connected. Check MCP status.`,
        }
      }

      const tools = await MCP.tools()
      const toolName = findMcpTool(tools, mcpName, params.action)

      if (!toolName) {
        return {
          title: "抖音 - Tool Not Found",
          metadata: { error: true, availableTools: Object.keys(tools).filter((k) => k.includes(mcpName)) },
          output: `No matching MCP tool found for action '${params.action}'`,
        }
      }

      return await callMcpTool(tools[toolName], params)
    } catch (error) {
      log.error("douyin mcp call failed", { action: params.action, error })
      return {
        title: "抖音 - Error",
        metadata: { error: true },
        output: `Failed to call MCP: ${error instanceof Error ? error.message : "Unknown error"}`,
      }
    }
  },
})

function findMcpTool(
  tools: Awaited<ReturnType<typeof MCP.tools>>,
  mcpName: string,
  action: string,
): string | undefined {
  const candidates = [
    `${mcpName}_${action}`,
    `${mcpName}_video_${action}`,
    `${mcpName}_dy_${action}`,
    `douyin_${action}`,
  ]

  for (const name of candidates) {
    if (tools[name]) return name
  }

  return undefined
}

async function callMcpTool(
  tool: Awaited<ReturnType<typeof MCP.tools>>[string],
  params: { action: string; videoId?: string; keyword?: string; userId?: string; limit?: number },
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const args: Record<string, unknown> = {}

  if (params.videoId) args.videoId = params.videoId
  if (params.keyword) args.keyword = params.keyword
  if (params.userId) args.userId = params.userId
  if (params.limit) args.limit = params.limit

  // AI SDK tools require toolCallId, messages, and abortSignal
  if (!tool.execute) {
    throw new Error("Tool does not have an execute function")
  }
  const result = await tool.execute(args, {
    toolCallId: `reach_douyin_${Date.now()}`,
    messages: [],
    abortSignal: new AbortController().signal,
  })

  const content = typeof result === "object" && result !== null && "content" in result
    ? formatMcpContent(result.content)
    : String(result)

  return {
    title: `抖音 - ${params.action}`,
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
