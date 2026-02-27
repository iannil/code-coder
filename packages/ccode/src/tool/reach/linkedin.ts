import z from "zod"
import { Tool } from "../tool"
import { checkChannel } from "./doctor"
import { ReachConfigManager } from "./config"
import { MCP } from "../../mcp"
import { Log } from "@/util/log"

/**
 * Agent Reach - LinkedIn Tool
 *
 * Read LinkedIn profiles and posts via MCP server
 */

const log = Log.create({ service: "reach.linkedin" })

const DESCRIPTION = `Read LinkedIn profiles, posts, and company information.

Actions:
- profile: Get user profile information
- posts: Get user's recent posts
- company: Get company page information
- search: Search for profiles or jobs

Examples:
- Profile: { "profileId": "johndoe", "action": "profile" }
- Posts: { "profileId": "johndoe", "action": "posts", "limit": 10 }
- Company: { "companyId": "microsoft", "action": "company" }
- Search: { "keyword": "software engineer", "action": "search" }

Requires linkedin MCP server to be configured and running.`

export const LinkedInTool = Tool.define("reach_linkedin", {
  description: DESCRIPTION,
  parameters: z.object({
    action: z.enum(["profile", "posts", "company", "search"]).default("profile").describe("Action to perform"),
    profileId: z.string().optional().describe("LinkedIn profile ID or URL"),
    companyId: z.string().optional().describe("Company ID or URL"),
    keyword: z.string().optional().describe("Search keyword"),
    limit: z.number().optional().default(10).describe("Maximum number of results"),
  }),
  async execute(params, ctx) {
    // Check if MCP server is available
    const channelStatus = await checkChannel("linkedin")
    if (channelStatus.status === "off" || channelStatus.status === "error") {
      return {
        title: "LinkedIn - Unavailable",
        metadata: { error: true },
        output: `LinkedIn tool unavailable: ${channelStatus.message}`,
      }
    }

    const pattern = params.profileId ?? params.companyId ?? params.keyword ?? "linkedin"
    await ctx.ask({
      permission: "reach_linkedin",
      patterns: [pattern],
      always: ["*"],
      metadata: { action: params.action },
    })

    const mcpName = await ReachConfigManager.getMcpName("linkedin")
    if (!mcpName) {
      return {
        title: "LinkedIn - Not Configured",
        metadata: { error: true },
        output: "MCP server not configured for linkedin. Add to ~/.codecoder/reach.json",
      }
    }

    try {
      const clients = await MCP.clients()
      const client = clients[mcpName]

      if (!client) {
        return {
          title: "LinkedIn - MCP Not Connected",
          metadata: { error: true },
          output: `MCP server '${mcpName}' is not connected. Check MCP status.`,
        }
      }

      const tools = await MCP.tools()
      const toolName = findMcpTool(tools, mcpName, params.action)

      if (!toolName) {
        return {
          title: "LinkedIn - Tool Not Found",
          metadata: { error: true, availableTools: Object.keys(tools).filter((k) => k.includes(mcpName)) },
          output: `No matching MCP tool found for action '${params.action}'`,
        }
      }

      return await callMcpTool(tools[toolName], params)
    } catch (error) {
      log.error("linkedin mcp call failed", { action: params.action, error })
      return {
        title: "LinkedIn - Error",
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
    `${mcpName}_linkedin_${action}`,
    `${mcpName}_get_${action}`,
    `linkedin_${action}`,
  ]

  for (const name of candidates) {
    if (tools[name]) return name
  }

  return undefined
}

async function callMcpTool(
  tool: Awaited<ReturnType<typeof MCP.tools>>[string],
  params: { action: string; profileId?: string; companyId?: string; keyword?: string; limit?: number },
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const args: Record<string, unknown> = {}

  if (params.profileId) args.profileId = params.profileId
  if (params.companyId) args.companyId = params.companyId
  if (params.keyword) args.keyword = params.keyword
  if (params.limit) args.limit = params.limit

  // AI SDK tools require toolCallId, messages, and abortSignal
  if (!tool.execute) {
    throw new Error("Tool does not have an execute function")
  }
  const result = await tool.execute(args, {
    toolCallId: `reach_linkedin_${Date.now()}`,
    messages: [],
    abortSignal: new AbortController().signal,
  })

  const content = typeof result === "object" && result !== null && "content" in result
    ? formatMcpContent(result.content)
    : String(result)

  return {
    title: `LinkedIn - ${params.action}`,
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
