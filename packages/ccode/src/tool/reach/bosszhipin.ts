import z from "zod"
import { Tool } from "../tool"
import { checkChannel } from "./doctor"
import { ReachConfigManager } from "./config"
import { MCP } from "../../mcp"
import { Log } from "@/util/log"

/**
 * Agent Reach - BossZhipin Tool
 *
 * Search job listings on Boss直聘 via MCP server
 */

const log = Log.create({ service: "reach.bosszhipin" })

const DESCRIPTION = `Search and read job listings on Boss直聘.

Actions:
- search: Search for jobs by keyword, location, or salary
- job: Get detailed job information
- company: Get company profile and job listings

Examples:
- Search: { "keyword": "后端开发", "city": "北京", "action": "search" }
- Job detail: { "jobId": "abc123", "action": "job" }
- Company: { "companyId": "comp123", "action": "company" }

Requires bosszhipin MCP server to be configured and running.`

export const BossZhipinTool = Tool.define("reach_bosszhipin", {
  description: DESCRIPTION,
  parameters: z.object({
    action: z.enum(["search", "job", "company"]).default("search").describe("Action to perform"),
    keyword: z.string().optional().describe("Job search keyword"),
    city: z.string().optional().describe("City name (e.g., 北京, 上海)"),
    jobId: z.string().optional().describe("Job ID for detail view"),
    companyId: z.string().optional().describe("Company ID"),
    salaryMin: z.number().optional().describe("Minimum salary in K"),
    salaryMax: z.number().optional().describe("Maximum salary in K"),
    limit: z.number().optional().default(20).describe("Maximum number of results"),
  }),
  async execute(params, ctx) {
    // Check if MCP server is available
    const channelStatus = await checkChannel("bosszhipin")
    if (channelStatus.status === "off" || channelStatus.status === "error") {
      return {
        title: "Boss直聘 - Unavailable",
        metadata: { error: true },
        output: `Boss直聘 tool unavailable: ${channelStatus.message}`,
      }
    }

    const pattern = params.keyword ?? params.jobId ?? params.companyId ?? "bosszhipin"
    await ctx.ask({
      permission: "reach_bosszhipin",
      patterns: [pattern],
      always: ["*"],
      metadata: { action: params.action },
    })

    const mcpName = await ReachConfigManager.getMcpName("bosszhipin")
    if (!mcpName) {
      return {
        title: "Boss直聘 - Not Configured",
        metadata: { error: true },
        output: "MCP server not configured for bosszhipin. Add to ~/.codecoder/reach.json",
      }
    }

    try {
      const clients = await MCP.clients()
      const client = clients[mcpName]

      if (!client) {
        return {
          title: "Boss直聘 - MCP Not Connected",
          metadata: { error: true },
          output: `MCP server '${mcpName}' is not connected. Check MCP status.`,
        }
      }

      const tools = await MCP.tools()
      const toolName = findMcpTool(tools, mcpName, params.action)

      if (!toolName) {
        return {
          title: "Boss直聘 - Tool Not Found",
          metadata: { error: true, availableTools: Object.keys(tools).filter((k) => k.includes(mcpName)) },
          output: `No matching MCP tool found for action '${params.action}'`,
        }
      }

      return await callMcpTool(tools[toolName], params)
    } catch (error) {
      log.error("bosszhipin mcp call failed", { action: params.action, error })
      return {
        title: "Boss直聘 - Error",
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
    `${mcpName}_boss_${action}`,
    `${mcpName}_job_${action}`,
    `bosszhipin_${action}`,
    `boss_${action}`,
  ]

  for (const name of candidates) {
    if (tools[name]) return name
  }

  return undefined
}

async function callMcpTool(
  tool: Awaited<ReturnType<typeof MCP.tools>>[string],
  params: {
    action: string
    keyword?: string
    city?: string
    jobId?: string
    companyId?: string
    salaryMin?: number
    salaryMax?: number
    limit?: number
  },
): Promise<{ title: string; metadata: Record<string, unknown>; output: string }> {
  const args: Record<string, unknown> = {}

  if (params.keyword) args.keyword = params.keyword
  if (params.city) args.city = params.city
  if (params.jobId) args.jobId = params.jobId
  if (params.companyId) args.companyId = params.companyId
  if (params.salaryMin) args.salaryMin = params.salaryMin
  if (params.salaryMax) args.salaryMax = params.salaryMax
  if (params.limit) args.limit = params.limit

  // AI SDK tools require toolCallId, messages, and abortSignal
  if (!tool.execute) {
    throw new Error("Tool does not have an execute function")
  }
  const result = await tool.execute(args, {
    toolCallId: `reach_bosszhipin_${Date.now()}`,
    messages: [],
    abortSignal: new AbortController().signal,
  })

  const content = typeof result === "object" && result !== null && "content" in result
    ? formatMcpContent(result.content)
    : String(result)

  return {
    title: `Boss直聘 - ${params.action}`,
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
