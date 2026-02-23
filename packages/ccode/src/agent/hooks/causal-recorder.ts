/**
 * Causal Recorder Hook
 *
 * Records agent decisions and tool executions to the CausalGraph.
 * Enables decision traceability and pattern-based suggestions.
 *
 * Part of Phase 18: Agent 因果链集成
 */

import { Log } from "@/util/log"
import { CausalGraph } from "@/memory/knowledge/causal-graph"
import { CausalAnalysis } from "@/memory/knowledge/causal-analysis"
import type { ActionType, CausalChain } from "@/memory/knowledge/causal-types"

const log = Log.create({ service: "agent.hooks.causal-recorder" })

export namespace CausalRecorder {
  // Session to decision mapping - tracks active decision for each session
  const sessionDecisions = new Map<string, string>()

  // Action to decision mapping - for linking outcomes back to decisions
  const actionDecisions = new Map<string, string>()

  // Tool name to ActionType mapping
  const TOOL_ACTION_TYPE_MAP: Record<string, ActionType> = {
    write: "file_operation",
    edit: "file_operation",
    read: "file_operation",
    glob: "search",
    grep: "search",
    bash: "tool_execution",
    webfetch: "api_call",
    websearch: "search",
    mcp__browser__browser_navigate: "tool_execution",
    mcp__browser__browser_click: "tool_execution",
    mcp__browser__browser_type: "tool_execution",
  }

  /**
   * Map a tool name to an ActionType
   */
  function mapToolToActionType(toolName: string): ActionType {
    const mapped = TOOL_ACTION_TYPE_MAP[toolName.toLowerCase()]
    if (mapped) return mapped

    // Handle MCP tools
    if (toolName.startsWith("mcp__")) return "tool_execution"

    // Handle code-related tools
    if (toolName.includes("code") || toolName.includes("lint") || toolName.includes("format")) {
      return "code_change"
    }

    // Default
    return "other"
  }

  /**
   * Record a decision made by @decision agent or other agents
   */
  export async function recordAgentDecision(ctx: {
    sessionId: string
    agentId: string
    prompt: string
    reasoning: string
    confidence: number
    context?: {
      files?: string[]
      tools?: string[]
    }
  }): Promise<string> {
    const decision = await CausalGraph.recordDecision({
      sessionId: ctx.sessionId,
      agentId: ctx.agentId,
      prompt: ctx.prompt,
      reasoning: ctx.reasoning,
      confidence: ctx.confidence,
      context: ctx.context,
    })

    // Track for linking future actions
    sessionDecisions.set(ctx.sessionId, decision.id)

    log.info("Recorded agent decision", {
      decisionId: decision.id,
      agentId: ctx.agentId,
      sessionId: ctx.sessionId,
    })

    return decision.id
  }

  /**
   * Record a tool action (called from PostToolUse hook)
   */
  export async function recordToolAction(ctx: {
    sessionId: string
    toolName: string
    toolInput: Record<string, unknown>
    toolOutput?: string
    duration?: number
  }): Promise<string | null> {
    const decisionId = sessionDecisions.get(ctx.sessionId)
    if (!decisionId) {
      // No decision recorded for this session, skip
      log.debug("No active decision for session, skipping action recording", {
        sessionId: ctx.sessionId,
        toolName: ctx.toolName,
      })
      return null
    }

    const actionType = mapToolToActionType(ctx.toolName)

    // Build description from tool name and key input
    const inputSummary = summarizeInput(ctx.toolInput)
    const description = `${ctx.toolName}: ${inputSummary}`

    const action = await CausalGraph.recordAction({
      decisionId,
      actionType,
      description,
      input: ctx.toolInput,
      output: ctx.toolOutput ? { result: ctx.toolOutput.slice(0, 1000) } : undefined,
      duration: ctx.duration,
    })

    // Track for outcome recording
    actionDecisions.set(action.id, decisionId)

    log.debug("Recorded tool action", {
      actionId: action.id,
      decisionId,
      toolName: ctx.toolName,
      actionType,
    })

    return action.id
  }

  /**
   * Record the outcome of a tool action
   */
  export async function recordOutcome(ctx: {
    actionId: string
    success: boolean
    error?: string
    metrics?: {
      filesModified?: number
      testsPass?: number
      testsFail?: number
      linesAdded?: number
      linesRemoved?: number
    }
  }): Promise<void> {
    await CausalGraph.recordOutcome({
      actionId: ctx.actionId,
      status: ctx.success ? "success" : "failure",
      description: ctx.error ?? (ctx.success ? "Completed successfully" : "Failed"),
      metrics: ctx.metrics,
    })

    log.debug("Recorded outcome", {
      actionId: ctx.actionId,
      success: ctx.success,
    })
  }

  /**
   * Get suggestions based on current decision context
   */
  export async function getSuggestions(ctx: {
    agentId: string
    prompt: string
  }): Promise<Array<{ id: string; type: string; confidence: number; reasoning: string }>> {
    const suggestions = await CausalAnalysis.suggestFromHistory({
      agentId: ctx.agentId,
      prompt: ctx.prompt,
    })

    return suggestions.map((s) => ({
      id: s.id,
      type: s.type,
      confidence: s.confidence,
      reasoning: s.reasoning,
    }))
  }

  /**
   * Get causal history formatted for display
   */
  export async function getHistory(ctx: { agentId?: string; limit?: number }): Promise<string> {
    const chains = await CausalGraph.query({
      agentId: ctx.agentId,
      limit: ctx.limit ?? 10,
    })

    return formatChainsForDisplay(chains)
  }

  /**
   * Get active decision ID for a session
   */
  export function getActiveDecisionId(sessionId: string): string | undefined {
    return sessionDecisions.get(sessionId)
  }

  /**
   * Clear decision tracking for a session (call on session end)
   */
  export function clearSession(sessionId: string): void {
    sessionDecisions.delete(sessionId)
    log.debug("Cleared session decision tracking", { sessionId })
  }

  /**
   * Clear all tracking data (for testing)
   */
  export function clearAll(): void {
    sessionDecisions.clear()
    actionDecisions.clear()
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  /**
   * Summarize tool input for description
   */
  function summarizeInput(input: Record<string, unknown>): string {
    // Handle common tool input patterns
    if (typeof input.file_path === "string") {
      return input.file_path.split("/").slice(-2).join("/")
    }
    if (typeof input.path === "string") {
      return input.path.split("/").slice(-2).join("/")
    }
    if (typeof input.command === "string") {
      const cmd = input.command as string
      return cmd.length > 50 ? cmd.slice(0, 50) + "..." : cmd
    }
    if (typeof input.pattern === "string") {
      return `pattern: ${input.pattern}`
    }
    if (typeof input.url === "string") {
      const url = new URL(input.url as string)
      return url.hostname + url.pathname.slice(0, 30)
    }

    // Default: stringify and truncate
    const str = JSON.stringify(input)
    return str.length > 60 ? str.slice(0, 60) + "..." : str
  }

  /**
   * Format causal chains as readable Markdown
   */
  function formatChainsForDisplay(chains: CausalChain[]): string {
    if (chains.length === 0) {
      return "暂无决策历史记录。"
    }

    return chains
      .map((chain) => {
        const successCount = chain.outcomes.filter((o) => o.status === "success").length
        const failureCount = chain.outcomes.filter((o) => o.status === "failure").length
        const totalOutcomes = chain.outcomes.length

        const statusEmoji =
          totalOutcomes === 0
            ? "⏳"
            : successCount === totalOutcomes
              ? "✅"
              : failureCount === totalOutcomes
                ? "❌"
                : "⚠️"

        const promptPreview = chain.decision.prompt.length > 60
          ? chain.decision.prompt.slice(0, 60) + "..."
          : chain.decision.prompt

        const actionTypes = [...new Set(chain.actions.map((a) => a.actionType))].join(", ")

        return [
          `### ${statusEmoji} ${promptPreview}`,
          `- **Agent**: ${chain.decision.agentId}`,
          `- **置信度**: ${(chain.decision.confidence * 100).toFixed(0)}%`,
          `- **操作数**: ${chain.actions.length} (${actionTypes || "无"})`,
          `- **结果**: ${successCount} 成功 / ${failureCount} 失败`,
          `- **时间**: ${new Date(chain.decision.timestamp).toLocaleString("zh-CN")}`,
        ].join("\n")
      })
      .join("\n\n---\n\n")
  }
}
