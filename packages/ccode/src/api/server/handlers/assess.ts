/**
 * Technical Feasibility Assessment API Handler
 *
 * Provides endpoints for analyzing code feasibility of new features.
 * This enables PM/non-developer workflows described in goals.md:
 * - IM channel asks "Is adding WeChat login complex?"
 * - CodeCoder scans codebase and returns structured assessment
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { SemanticGraph } from "../../../memory/knowledge/graph"
import z from "zod"

// ============================================================================
// Request/Response Types
// ============================================================================

const FeasibilityRequest = z.object({
  /** Natural language query describing the feature */
  query: z.string().min(1),
  /** Optional project path (defaults to current project) */
  project_path: z.string().optional(),
  /** Analysis options */
  options: z
    .object({
      /** Analysis depth: quick (fast), standard (balanced), deep (thorough) */
      depth: z.enum(["quick", "standard", "deep"]).default("standard"),
      /** Include code references in response */
      include_code_refs: z.boolean().default(true),
      /** Response language */
      language: z.enum(["zh-CN", "en-US"]).default("zh-CN"),
    })
    .optional(),
})
type FeasibilityRequest = z.infer<typeof FeasibilityRequest>

const ExistingCapability = z.object({
  name: z.string(),
  path: z.string(),
  relevance: z.string(),
})

const RequiredChange = z.object({
  file: z.string(),
  action: z.enum(["create", "modify", "delete"]),
  description: z.string(),
})

const Dependency = z.object({
  name: z.string(),
  type: z.enum(["npm", "pip", "cargo", "other"]),
  reason: z.string(),
})

const FeasibilityAnalysis = z.object({
  complexity: z.enum(["low", "medium", "high", "critical"]),
  summary: z.string(),
  existing_capabilities: z.array(ExistingCapability),
  required_changes: z.array(RequiredChange),
  dependencies: z.array(Dependency),
  risks: z.array(z.string()),
  confidence: z.number().min(0).max(1),
})
type FeasibilityAnalysis = z.infer<typeof FeasibilityAnalysis>

export interface FeasibilityResponse {
  success: boolean
  data?: {
    summary: string
    complexity: "low" | "medium" | "high" | "critical"
    analysis: FeasibilityAnalysis
    confidence: number
    tokens_used?: number
  }
  error?: string
}

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

/**
 * Build analysis prompt with semantic graph context
 */
function buildAnalysisPrompt(query: string, graph: SemanticGraph.Graph, depth: string): string {
  const graphSummary = summarizeGraph(graph)

  return `## 代码库语义图

${graphSummary}

## 用户需求

${query}

## 分析深度

${depth === "quick" ? "快速分析：仅识别主要模块和关键变更" : depth === "deep" ? "深度分析：详细评估所有相关代码、依赖和风险" : "标准分析：平衡速度和深度"}

请基于以上信息，输出技术可行性评估的 JSON 结果。`
}

/**
 * Summarize semantic graph for prompt context
 */
function summarizeGraph(graph: SemanticGraph.Graph): string {
  const sections: string[] = []

  // Group nodes by type
  const nodesByType: Record<string, SemanticGraph.Node[]> = {}
  for (const node of graph.nodes) {
    if (!nodesByType[node.node_type]) nodesByType[node.node_type] = []
    nodesByType[node.node_type].push(node)
  }

  // Summarize each type
  for (const [type, nodes] of Object.entries(nodesByType)) {
    const names = nodes.slice(0, 20).map((n) => `- ${n.name} (${n.file})`)
    const truncated = nodes.length > 20 ? `\n  ... 及其他 ${nodes.length - 20} 个` : ""
    sections.push(`### ${type.toUpperCase()} (${nodes.length}个)\n${names.join("\n")}${truncated}`)
  }

  // Add edge statistics
  const edgesByType: Record<string, number> = {}
  for (const edge of graph.edges) {
    edgesByType[edge.type] = (edgesByType[edge.type] || 0) + 1
  }
  const edgeStats = Object.entries(edgesByType)
    .map(([type, count]) => `- ${type}: ${count}`)
    .join("\n")
  sections.push(`### 关系统计\n${edgeStats}`)

  return sections.join("\n\n")
}

/**
 * Parse LLM response into structured analysis
 */
function parseAnalysisResult(response: string): FeasibilityAnalysis {
  // Try to extract JSON from response
  const jsonMatch = response.match(/```json\s*([\s\S]*?)\s*```/) || response.match(/\{[\s\S]*\}/)

  if (!jsonMatch) {
    throw new Error("Failed to parse analysis result: no JSON found in response")
  }

  const jsonStr = jsonMatch[1] || jsonMatch[0]
  const parsed = JSON.parse(jsonStr)

  // Validate with schema
  return FeasibilityAnalysis.parse(parsed)
}

/**
 * Format analysis for IM channels (human-readable)
 */
export function formatForChannel(analysis: FeasibilityAnalysis): string {
  const complexityEmoji: Record<string, string> = {
    low: "🟢",
    medium: "🟡",
    high: "🟠",
    critical: "🔴",
  }

  const complexityLabel: Record<string, string> = {
    low: "低",
    medium: "中等",
    high: "较高",
    critical: "关键",
  }

  const lines: string[] = [
    "📊 **技术可行性评估**",
    "",
    `**需求**: ${analysis.summary}`,
    `**复杂度**: ${complexityEmoji[analysis.complexity]} ${complexityLabel[analysis.complexity]}`,
    "",
  ]

  if (analysis.existing_capabilities.length > 0) {
    lines.push("✅ **现有能力**")
    for (const cap of analysis.existing_capabilities.slice(0, 5)) {
      lines.push(`• ${cap.name} (${cap.path})`)
    }
    lines.push("")
  }

  if (analysis.required_changes.length > 0) {
    lines.push("📝 **需要修改**")
    for (const change of analysis.required_changes.slice(0, 8)) {
      const actionLabel = change.action === "create" ? "[新建]" : change.action === "modify" ? "[修改]" : "[删除]"
      lines.push(`${actionLabel} ${change.file}`)
    }
    lines.push("")
  }

  if (analysis.dependencies.length > 0) {
    lines.push("📦 **新增依赖**")
    for (const dep of analysis.dependencies.slice(0, 5)) {
      lines.push(`• ${dep.name} (${dep.type})`)
    }
    lines.push("")
  }

  if (analysis.risks.length > 0) {
    lines.push("⚠️ **风险提示**")
    for (const risk of analysis.risks.slice(0, 3)) {
      lines.push(`• ${risk}`)
    }
    lines.push("")
  }

  lines.push(`置信度: ${Math.round(analysis.confidence * 100)}%`)

  return lines.join("\n")
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * POST /api/v1/assess/feasibility
 *
 * Assess technical feasibility of a feature request.
 *
 * Example request:
 * ```json
 * {
 *   "query": "我们要增加微信扫码登录功能，技术复杂度高吗？",
 *   "options": {
 *     "depth": "standard",
 *     "include_code_refs": true,
 *     "language": "zh-CN"
 *   }
 * }
 * ```
 */
export async function assessFeasibility(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = FeasibilityRequest.parse(JSON.parse(body))

    const depth = input.options?.depth ?? "standard"

    // Load semantic graph (builds if not exists)
    const graph = await SemanticGraph.getAsGraph()

    // Build the analysis prompt
    const analysisPrompt = buildAnalysisPrompt(input.query, graph, depth)

    // Import prompt template
    const FEASIBILITY_PROMPT = await import("../../../agent/prompt/feasibility-assess.txt").then((m) => m.default)

    // Use LocalSession to invoke the analysis
    const { LocalSession } = await import("../../../api")

    // Create a transient session for analysis
    const session = await LocalSession.create({
      title: `Feasibility: ${input.query.slice(0, 50)}...`,
    })

    // Send analysis request to general agent with specialized prompt
    const result = await LocalSession.prompt({
      sessionID: session.id,
      agent: "general",
      parts: [
        {
          type: "text",
          text: `${FEASIBILITY_PROMPT}\n\n${analysisPrompt}`,
        },
      ],
    })

    // Wait for the response (streaming)
    const messages = await LocalSession.messages({ sessionID: session.id })
    const lastMessage = messages[messages.length - 1]

    if (!lastMessage || lastMessage.info.role !== "assistant") {
      return errorResponse("Analysis failed: no response from agent", 500)
    }

    // Extract text content from message parts
    const textPart = lastMessage.parts.find((p: { type: string; text?: string }) => p.type === "text")
    if (!textPart || textPart.type !== "text" || !("text" in textPart)) {
      return errorResponse("Analysis failed: no text content in response", 500)
    }

    // Parse the structured analysis
    const analysis = parseAnalysisResult((textPart as { type: "text"; text: string }).text)

    // Extract token usage from assistant message
    const assistantInfo = lastMessage.info as { tokens?: { input: number; output: number; reasoning?: number; cache?: { read: number; write: number } } }
    const tokens = assistantInfo.tokens
    const tokensUsed = tokens
      ? tokens.input + tokens.output + (tokens.reasoning ?? 0) + (tokens.cache?.read ?? 0) + (tokens.cache?.write ?? 0)
      : undefined

    // Clean up transient session
    await LocalSession.remove(session.id)

    return jsonResponse(
      {
        success: true,
        data: {
          summary: analysis.summary,
          complexity: analysis.complexity,
          analysis,
          confidence: analysis.confidence,
          tokens_used: tokensUsed,
        },
      },
      200,
    )
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Invalid request: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/assess/health
 *
 * Health check for the assessment service.
 */
export async function assessHealth(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    // Check if SemanticGraph is accessible
    const graph = await SemanticGraph.getAsGraph()
    const hasGraph = graph.nodes.length > 0

    return jsonResponse({
      success: true,
      data: {
        status: "healthy",
        semantic_graph: hasGraph ? "available" : "needs_build",
        nodes_count: graph.nodes.length,
        edges_count: graph.edges.length,
      },
    })
  } catch (error) {
    return jsonResponse(
      {
        success: true,
        data: {
          status: "degraded",
          error: error instanceof Error ? error.message : String(error),
        },
      },
      200,
    )
  }
}
