/**
 * Chat API Handler
 *
 * Provides a unified chat endpoint for IM channels via ZeroBot bridge.
 * Handles intent detection, agent routing, and message processing.
 *
 * POST /api/v1/chat - Send a message and receive a response
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { ConversationStore } from "../store/conversation"

// ============================================================================
// Types
// ============================================================================

interface ChatRequest {
  /** User message content */
  message: string
  /** Optional conversation ID for context continuity */
  conversation_id?: string
  /** Optional agent to use (auto-detected if not specified) */
  agent?: string
  /** User identifier */
  user_id: string
  /** Channel type (telegram, slack, discord, etc.) */
  channel: string
}

interface ChatResponse {
  /** Response message content */
  message: string
  /** Conversation ID for follow-up messages */
  conversation_id: string
  /** Agent used for this response */
  agent: string
  /** Token usage information */
  usage?: {
    input_tokens: number
    output_tokens: number
    total_tokens: number
  }
}

/** Tracing context extracted from HTTP headers */
interface TracingContext {
  traceId: string
  spanId: string
  parentSpanId?: string
  userId?: string
}

/** Lifecycle event for structured logging (ODD compliance) */
interface LifecycleEvent {
  timestamp: string
  trace_id: string
  span_id: string
  parent_span_id?: string
  event_type: "function_start" | "function_end" | "error" | "http_request" | "http_response"
  service: string
  payload: Record<string, unknown>
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
 * Extract text content from message parts.
 */
function extractTextFromParts(parts: Array<{ type: string; text?: string }>): string {
  return parts
    .filter((p) => p.type === "text" && p.text)
    .map((p) => p.text)
    .join("\n\n")
}

/**
 * Generate a unique span ID (8 character UUID prefix)
 */
function generateSpanId(): string {
  return crypto.randomUUID().slice(0, 8)
}

/**
 * Generate a unique trace ID (full UUID)
 */
function generateTraceId(): string {
  return crypto.randomUUID()
}

/**
 * Extract tracing context from HTTP headers
 */
function extractTracingContext(req: HttpRequest): TracingContext {
  const headers = req.headers
  const traceId = headers.get("X-Trace-Id") ?? generateTraceId()
  const parentSpanId = headers.get("X-Span-Id") ?? undefined
  const userId = headers.get("X-User-Id") ?? undefined

  return {
    traceId,
    spanId: generateSpanId(),
    parentSpanId,
    userId,
  }
}

/**
 * Log a lifecycle event in structured JSON format (ODD compliance)
 */
function logLifecycleEvent(ctx: TracingContext, eventType: LifecycleEvent["event_type"], payload: Record<string, unknown>) {
  const event: LifecycleEvent = {
    timestamp: new Date().toISOString(),
    trace_id: ctx.traceId,
    span_id: ctx.spanId,
    parent_span_id: ctx.parentSpanId,
    event_type: eventType,
    service: "codecoder-api",
    payload,
  }
  console.log(JSON.stringify(event))
}

// ============================================================================
// Session Management
// ============================================================================

async function getOrCreateSession(conversationId: string | undefined): Promise<string> {
  const { LocalSession } = await import("../../../api")

  // If we have a conversation_id, check Redis for existing session
  if (conversationId && ConversationStore.isInitialized()) {
    try {
      const existingSessionId = await ConversationStore.get(conversationId)
      if (existingSessionId) {
        // Verify session still exists
        try {
          await LocalSession.get(existingSessionId)
          return existingSessionId
        } catch {
          // Session doesn't exist anymore, delete stale mapping
          await ConversationStore.delete_(conversationId)
        }
      }
    } catch (redisError) {
      // Redis unavailable - log and continue to create new session
      console.error(
        JSON.stringify({
          timestamp: new Date().toISOString(),
          event: "redis_error",
          function: "getOrCreateSession",
          error: redisError instanceof Error ? redisError.message : String(redisError),
        }),
      )
    }
  }

  // Create a new session
  const session = await LocalSession.create({
    title: `Chat: ${new Date().toISOString()}`,
  })

  // Map conversation_id if provided and Redis is available
  if (conversationId && ConversationStore.isInitialized()) {
    try {
      await ConversationStore.set(conversationId, session.id)
    } catch (redisError) {
      console.error(
        JSON.stringify({
          timestamp: new Date().toISOString(),
          event: "redis_error",
          function: "getOrCreateSession.set",
          error: redisError instanceof Error ? redisError.message : String(redisError),
        }),
      )
    }
  }

  return session.id
}

// ============================================================================
// Autonomous Execution
// ============================================================================

import type { AutonomousSessionStore } from "../store/autonomous-session"

/**
 * Check if a message indicates an actionable task suitable for Evolution Loop
 */
function isActionableTask(message: string): boolean {
  const actionKeywords = [
    // Chinese action words
    "实现", "创建", "修复", "开发", "构建", "编写", "生成", "执行",
    "部署", "配置", "设置", "安装", "更新", "修改", "重构", "优化",
    "自动", "定时", "调度", "每天", "每周", "每小时",
    // English action words
    "implement", "create", "fix", "build", "write", "generate", "execute",
    "deploy", "configure", "setup", "install", "update", "modify", "refactor",
    "automate", "schedule", "cron", "daily", "weekly", "hourly",
  ]

  const lowerMessage = message.toLowerCase()
  return actionKeywords.some(keyword => lowerMessage.includes(keyword))
}

/**
 * Execute chat with autonomous mode enabled.
 * Uses CLOSE decision framework, Evolution Loop, and Auto-Builder.
 *
 * Flow:
 * 1. CLOSE evaluation to decide whether to proceed
 * 2. If actionable task → trigger Evolution Loop for autonomous problem-solving
 * 3. Evolution Loop includes: web search, tool discovery, code generation, reflection, gap detection
 * 4. If Evolution Loop fails → Gap Detection → Auto-Builder (if enabled)
 */
async function executeAutonomousChat(
  input: ChatRequest,
  autonomousState: AutonomousSessionStore.AutonomousState,
  ctx: TracingContext,
  startTime: number
): Promise<HttpResponse> {
  try {
    // Import autonomous components
    const { buildCriteria, DecisionTemplates } = await import("../../../autonomous/decision/criteria")
    const { DecisionEngine } = await import("../../../autonomous/decision/engine")
    const { createEvolutionLoop } = await import("../../../autonomous/execution/evolution-loop")
    type EvolutionResultType = Awaited<ReturnType<ReturnType<typeof createEvolutionLoop>["evolve"]>>
    const { SessionPrompt } = await import("../../../session/prompt")
    const { LocalSession } = await import("../../../api")
    const { MessageV2 } = await import("../../../session/message-v2")

    // Get or create session for this conversation
    const sessionId = await getOrCreateSession(input.conversation_id)

    // Create decision engine with autonomy level
    const autonomyLevel = autonomousState.autonomyLevel as "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"
    const decisionEngine = new DecisionEngine({ autonomyLevel })

    // Build CLOSE criteria for the request
    const isActionable = isActionableTask(input.message)
    const criteria = buildCriteria({
      type: isActionable ? "implementation" : "other",
      description: input.message,
      riskLevel: isActionable ? "low" : "low",
      convergence: isActionable ? 7 : 6,  // Actionable tasks have higher reversibility
      leverage: isActionable ? 8 : 7,      // Higher leverage for actionable tasks
      optionality: 8,
      surplus: 7,
      evolution: isActionable ? 8 : 6,     // Higher learning value for actionable tasks
    })

    const decisionContext = {
      sessionId,
      currentState: "executing",
      resourceUsage: {
        tokensUsed: 0,
        costUSD: 0,
        durationMinutes: 0,
      },
      errorCount: 0,
      recentDecisions: [],
    }

    // Evaluate using CLOSE framework
    const decision = await decisionEngine.evaluate(criteria, decisionContext)

    logLifecycleEvent(ctx, "http_request", {
      function: "executeAutonomousChat.closeEvaluation",
      close_score: decision.score.total,
      approved: decision.approved,
      action: decision.action,
      isActionable,
    })

    // If decision not approved, ask for confirmation
    if (!decision.approved) {
      const durationMs = Math.round(performance.now() - startTime)
      logLifecycleEvent(ctx, "function_end", {
        function: "executeAutonomousChat",
        duration_ms: durationMs,
        success: false,
        reason: "decision_blocked",
        score: decision.score.total,
      })

      return jsonResponse({
        success: true,
        data: {
          message: `⚠️ 自主决策暂停\n\n**CLOSE 评估分数**: ${decision.score.total.toFixed(1)}/10\n` +
            `- 收敛性 (C): ${decision.score.convergence.toFixed(1)}\n` +
            `- 杠杆性 (L): ${decision.score.leverage.toFixed(1)}\n` +
            `- 可选性 (O): ${decision.score.optionality.toFixed(1)}\n` +
            `- 可用余量 (S): ${decision.score.surplus.toFixed(1)}\n` +
            `- 进化性 (E): ${decision.score.evolution.toFixed(1)}\n\n` +
            `**原因**: ${decision.reasoning}\n\n请确认是否继续执行？`,
          conversation_id: input.conversation_id ?? sessionId,
          agent: "autonomous",
          autonomous_mode: true,
          decision_paused: true,
          close_score: decision.score,
        },
      })
    }

    // === Evolution Loop for Actionable Tasks ===
    let evolutionResult: EvolutionResultType | null = null

    if (isActionable) {
      logLifecycleEvent(ctx, "http_request", {
        function: "executeAutonomousChat.evolutionLoop",
        status: "starting",
        message_preview: input.message.slice(0, 100),
      })

      // Create and run Evolution Loop
      const evolutionLoop = createEvolutionLoop({
        maxRetries: 3,
        enableWebSearch: true,
        enableCodeExecution: true,
        enableToolDiscovery: true,
        enableToolLearning: true,
        enableSedimentation: true,
        enableGithubScout: true,
        enableAutoBuilder: true,
        enableAutoMetaBuilder: true, // Enable auto-building new concepts
        autoBuilderMinAttempts: 2,
        autoBuilderCloseThreshold: 5.5,
      })

      try {
        evolutionResult = await evolutionLoop.evolve({
          sessionId,
          description: input.message,
          errorMessage: undefined,
          technology: detectTechnology(input.message),
          workingDir: process.cwd(),
          maxRetries: 3,
          enableWebSearch: true,
          enableCodeExecution: true,
        })

        logLifecycleEvent(ctx, "http_request", {
          function: "executeAutonomousChat.evolutionLoop",
          status: "completed",
          solved: evolutionResult.solved,
          attempts: evolutionResult.attempts.length,
          gapDetected: !!evolutionResult.gapDetected,
          buildAttempted: evolutionResult.buildAttempted,
          durationMs: evolutionResult.durationMs,
        })
      } catch (evolutionError) {
        logLifecycleEvent(ctx, "error", {
          function: "executeAutonomousChat.evolutionLoop",
          error: evolutionError instanceof Error ? evolutionError.message : String(evolutionError),
        })
        // Continue with normal chat if evolution fails
      } finally {
        await evolutionLoop.cleanup()
      }
    }

    // === Generate Response ===
    let responseText = ""
    let usage = {
      input_tokens: 0,
      output_tokens: 0,
      total_tokens: 0,
    }

    // If Evolution Loop solved the problem, format the result
    if (evolutionResult?.solved) {
      responseText = formatEvolutionSuccess(evolutionResult, autonomousState.autonomyLevel)
    } else if (evolutionResult && !evolutionResult.solved) {
      // Evolution Loop failed - include gap detection info
      responseText = formatEvolutionFailure(evolutionResult, autonomousState.autonomyLevel)
    } else {
      // Fallback to regular agent for non-actionable tasks or Evolution Loop failure
      const { getRegistry } = await import("../../../agent/registry")
      const registry = await getRegistry()
      const recommended = registry.recommend(input.message)
      const agentName = recommended?.name ?? "general"

      const autonomousPrompt = `[自主模式 - ${autonomousState.autonomyLevel}]

${input.message}

---
请以自主模式执行此任务。你可以:
- 主动搜索和分析代码
- 生成并执行测试
- 做出合理的实现决策
- 在必要时使用 CLOSE 框架评估风险

CLOSE 评估结果:
- 总分: ${decision.score.total.toFixed(1)}/10
- 决策: ${decision.action}
- 理由: ${decision.reasoning}`

      const result = await SessionPrompt.prompt({
        sessionID: sessionId,
        agent: agentName,
        parts: [{ type: "text", text: autonomousPrompt }],
      })

      if (typeof result === "object" && "info" in result && "parts" in result) {
        const assistantMsg = result as { info: { role: string; tokens?: { input: number; output: number } }; parts: Array<{ type: string; text?: string }> }
        responseText = extractTextFromParts(assistantMsg.parts)

        if (assistantMsg.info.tokens) {
          usage.input_tokens = assistantMsg.info.tokens.input
          usage.output_tokens = assistantMsg.info.tokens.output
          usage.total_tokens = usage.input_tokens + usage.output_tokens
        }
      } else if (typeof result === "string") {
        const parts = await MessageV2.parts(result)
        responseText = extractTextFromParts(parts)
      }
    }

    if (!responseText.trim()) {
      responseText = "自主模式执行完成，但没有文本输出。"
    }

    const response: ChatResponse = {
      message: responseText,
      conversation_id: input.conversation_id ?? sessionId,
      agent: evolutionResult?.solved ? "autonomous-evolution" : "autonomous",
      usage,
    }

    const durationMs = Math.round(performance.now() - startTime)
    logLifecycleEvent(ctx, "function_end", {
      function: "executeAutonomousChat",
      duration_ms: durationMs,
      success: true,
      autonomous_mode: true,
      close_score: decision.score.total,
      evolution_solved: evolutionResult?.solved,
      gap_detected: !!evolutionResult?.gapDetected,
      build_attempted: evolutionResult?.buildAttempted,
    })

    return jsonResponse({
      success: true,
      data: {
        ...response,
        autonomous_mode: true,
        close_score: decision.score,
        evolution_result: evolutionResult ? {
          solved: evolutionResult.solved,
          attempts: evolutionResult.attempts.length,
          summary: evolutionResult.summary,
          gapDetected: evolutionResult.gapDetected?.type,
          buildAttempted: evolutionResult.buildAttempted,
          durationMs: evolutionResult.durationMs,
        } : undefined,
      },
    })
  } catch (error) {
    const durationMs = Math.round(performance.now() - startTime)
    logLifecycleEvent(ctx, "error", {
      function: "executeAutonomousChat",
      duration_ms: durationMs,
      error: error instanceof Error ? error.message : String(error),
    })

    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Detect technology/language from message content
 */
function detectTechnology(message: string): string | undefined {
  const lowerMessage = message.toLowerCase()

  if (lowerMessage.includes("python") || lowerMessage.includes("pip")) return "python"
  if (lowerMessage.includes("typescript") || lowerMessage.includes("ts")) return "typescript"
  if (lowerMessage.includes("javascript") || lowerMessage.includes("js") || lowerMessage.includes("node")) return "nodejs"
  if (lowerMessage.includes("rust") || lowerMessage.includes("cargo")) return "rust"
  if (lowerMessage.includes("bash") || lowerMessage.includes("shell") || lowerMessage.includes("脚本")) return "bash"

  return undefined
}

/**
 * Format successful Evolution Loop result
 */
function formatEvolutionSuccess(result: { solved: boolean; solution?: string; attempts: Array<{ attempt: number }>; summary: string; knowledgeId?: string; learnedToolId?: string; usedToolId?: string; githubScoutResult?: { summary: string }; durationMs: number }, autonomyLevel: string): string {
  const sections: string[] = []

  sections.push(`🤖 **[自主模式 - ${autonomyLevel}] 任务完成**\n`)
  sections.push(`✅ **状态**: 问题已解决`)
  sections.push(`⏱ **耗时**: ${(result.durationMs / 1000).toFixed(1)}s`)
  sections.push(`🔄 **尝试次数**: ${result.attempts.length}`)

  if (result.usedToolId) {
    sections.push(`🔧 **使用已有工具**: ${result.usedToolId}`)
  }

  if (result.learnedToolId) {
    sections.push(`📚 **学习到新工具**: ${result.learnedToolId}`)
  }

  if (result.knowledgeId) {
    sections.push(`💡 **知识沉淀**: ${result.knowledgeId}`)
  }

  if (result.githubScoutResult) {
    sections.push(`\n🔍 **GitHub Scout**: ${result.githubScoutResult.summary}`)
  }

  sections.push(`\n📝 **摘要**: ${result.summary}`)

  if (result.solution) {
    sections.push(`\n\`\`\`\n${result.solution.slice(0, 2000)}${result.solution.length > 2000 ? '\n... (truncated)' : ''}\n\`\`\``)
  }

  return sections.join("\n")
}

/**
 * Format failed Evolution Loop result with gap detection info
 */
function formatEvolutionFailure(result: { solved: boolean; attempts: Array<{ attempt: number; reflection?: { analysis?: string } }>; summary: string; gapDetected?: { id: string; type: string; description: string; confidence: number; closeScore: { total: number } }; buildAttempted?: boolean; buildResult?: { success: boolean; concept?: { type: string; identifier: string }; summary?: string }; durationMs: number }, autonomyLevel: string): string {
  const sections: string[] = []

  sections.push(`🤖 **[自主模式 - ${autonomyLevel}] 任务未完成**\n`)
  sections.push(`⚠️ **状态**: 未能自动解决`)
  sections.push(`⏱ **耗时**: ${(result.durationMs / 1000).toFixed(1)}s`)
  sections.push(`🔄 **尝试次数**: ${result.attempts.length}`)

  sections.push(`\n📝 **摘要**: ${result.summary}`)

  // Gap Detection Result
  if (result.gapDetected) {
    sections.push(`\n### 🔍 能力缺口检测`)
    sections.push(`- **类型**: ${result.gapDetected.type}`)
    sections.push(`- **描述**: ${result.gapDetected.description}`)
    sections.push(`- **置信度**: ${(result.gapDetected.confidence * 100).toFixed(0)}%`)
    sections.push(`- **CLOSE 分数**: ${result.gapDetected.closeScore.total.toFixed(1)}/10`)
  }

  // Auto-Builder Result
  if (result.buildAttempted) {
    sections.push(`\n### 🏗️ 自动构建`)
    if (result.buildResult?.success) {
      sections.push(`✅ **成功构建**: ${result.buildResult.concept?.type} - ${result.buildResult.concept?.identifier}`)
    } else {
      sections.push(`❌ **构建失败**: ${result.buildResult?.summary ?? '未知错误'}`)
    }
  }

  // Last attempt reflection
  const lastAttempt = result.attempts[result.attempts.length - 1]
  if (lastAttempt?.reflection?.analysis) {
    sections.push(`\n### 💭 最后尝试分析`)
    sections.push(lastAttempt.reflection.analysis.slice(0, 500))
  }

  sections.push(`\n---\n💡 **建议**: 您可能需要提供更多上下文或手动介入解决此问题。`)

  return sections.join("\n")
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * POST /api/v1/chat
 *
 * Process a chat message and return a response.
 * This endpoint:
 * 1. Accepts a message from an IM channel
 * 2. Detects intent and routes to the appropriate agent
 * 3. Waits for the full response
 * 4. Returns the response with usage statistics
 */
export async function chat(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const startTime = performance.now()

  // Extract tracing context from headers
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", {
    function: "chat",
    method: req.method,
    url: req.url,
  })

  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as ChatRequest

    // Validate required fields
    if (!input.message) {
      logLifecycleEvent(ctx, "error", { function: "chat", error: "message is required" })
      return errorResponse("message is required", 400)
    }
    if (!input.user_id) {
      logLifecycleEvent(ctx, "error", { function: "chat", error: "user_id is required" })
      return errorResponse("user_id is required", 400)
    }
    if (!input.channel) {
      logLifecycleEvent(ctx, "error", { function: "chat", error: "channel is required" })
      return errorResponse("channel is required", 400)
    }

    // Check if autonomous mode is enabled for this conversation
    if (input.conversation_id) {
      const { AutonomousSessionStore } = await import("../store/autonomous-session")

      if (AutonomousSessionStore.isInitialized()) {
        try {
          const autonomousState = await AutonomousSessionStore.getState(input.conversation_id)

          if (autonomousState?.enabled) {
            logLifecycleEvent(ctx, "http_request", {
              function: "chat",
              autonomous_mode: true,
              autonomy_level: autonomousState.autonomyLevel,
              conversation_id: input.conversation_id,
            })

            // Execute with autonomous mode
            return await executeAutonomousChat(input, autonomousState, ctx, startTime)
          }
        } catch (autonomousError) {
          // Log but continue with normal chat if autonomous check fails
          console.error(
            JSON.stringify({
              timestamp: new Date().toISOString(),
              event: "autonomous_check_error",
              function: "chat",
              error: autonomousError instanceof Error ? autonomousError.message : String(autonomousError),
            }),
          )
        }
      }
    }

    // Import dependencies
    const { getRegistry } = await import("../../../agent/registry")
    const { SessionPrompt } = await import("../../../session/prompt")
    const { MessageV2 } = await import("../../../session/message-v2")

    // Get or create session
    const sessionId = await getOrCreateSession(input.conversation_id)

    // Determine which agent to use
    let agentName = input.agent
    if (!agentName) {
      // Use agent registry to recommend an agent based on intent
      const registry = await getRegistry()
      const recommended = registry.recommend(input.message)
      agentName = recommended?.name ?? "general"
    }

    // Validate agent exists
    const { Agent } = await import("../../../agent/agent")
    const agents = await Agent.list()
    const agentExists = agents.some((a) => a.name === agentName)

    if (!agentExists) {
      // Fall back to general if agent not found
      agentName = "general"
    }

    logLifecycleEvent(ctx, "http_request", {
      function: "chat",
      user_id: input.user_id,
      channel: input.channel,
      agent: agentName,
      session_id: sessionId,
    })

    // Send the message and wait for response
    const result = await SessionPrompt.prompt({
      sessionID: sessionId,
      agent: agentName,
      parts: [{ type: "text", text: input.message }],
    })

    // Extract text content from assistant message
    let responseText = ""
    let usage = {
      input_tokens: 0,
      output_tokens: 0,
      total_tokens: 0,
    }

    if (typeof result === "object" && "info" in result && "parts" in result) {
      // Full response with parts
      const assistantMsg = result as { info: { role: string; tokens?: { input: number; output: number } }; parts: Array<{ type: string; text?: string }> }
      responseText = extractTextFromParts(assistantMsg.parts)

      // Extract token usage from assistant message
      if (assistantMsg.info.tokens) {
        usage.input_tokens = assistantMsg.info.tokens.input
        usage.output_tokens = assistantMsg.info.tokens.output
        usage.total_tokens = usage.input_tokens + usage.output_tokens
      }
    } else if (typeof result === "string") {
      // Just a message ID - need to fetch the message
      const parts = await MessageV2.parts(result)
      responseText = extractTextFromParts(parts)
    }

    // If no text was extracted, provide a default
    if (!responseText.trim()) {
      responseText = "I processed your request but have no text response to provide."
    }

    const response: ChatResponse = {
      message: responseText,
      conversation_id: input.conversation_id ?? sessionId,
      agent: agentName,
      usage,
    }

    const durationMs = Math.round(performance.now() - startTime)
    logLifecycleEvent(ctx, "function_end", {
      function: "chat",
      duration_ms: durationMs,
      success: true,
      agent: agentName,
      tokens: usage.total_tokens,
    })

    return jsonResponse({
      success: true,
      data: response,
    })
  } catch (error) {
    const durationMs = Math.round(performance.now() - startTime)
    logLifecycleEvent(ctx, "error", {
      function: "chat",
      duration_ms: durationMs,
      error: error instanceof Error ? error.message : String(error),
    })

    console.error("Chat API error:", error)
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/chat/health
 *
 * Health check endpoint for the chat service.
 */
export async function chatHealth(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", { function: "chatHealth" })

  const response = jsonResponse({
    success: true,
    data: {
      status: "healthy",
      timestamp: new Date().toISOString(),
    },
  })

  logLifecycleEvent(ctx, "function_end", { function: "chatHealth", success: true })

  return response
}

// ============================================================================
// Session Control Commands
// ============================================================================

interface ClearRequest {
  /** Conversation ID to clear */
  conversation_id: string
  /** User identifier */
  user_id: string
  /** Channel type */
  channel: string
}

interface CompactRequest {
  /** Conversation ID to compact */
  conversation_id: string
  /** User identifier */
  user_id: string
  /** Channel type */
  channel: string
}

/**
 * POST /api/v1/chat/clear
 *
 * Clear the conversation context (start fresh).
 * This removes the session mapping so next message creates a new session.
 */
export async function clearConversation(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", { function: "clearConversation" })

  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as ClearRequest

    if (!input.conversation_id) {
      return errorResponse("conversation_id is required", 400)
    }

    // Remove the session mapping
    let hadMapping = false
    let redisError: Error | null = null
    if (ConversationStore.isInitialized()) {
      try {
        hadMapping = await ConversationStore.delete_(input.conversation_id)
      } catch (err) {
        redisError = err instanceof Error ? err : new Error(String(err))
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "redis_error",
            function: "clearConversation",
            error: redisError.message,
          }),
        )
      }
    }

    logLifecycleEvent(ctx, "function_end", {
      function: "clearConversation",
      success: true,
      had_mapping: hadMapping,
      conversation_id: input.conversation_id,
      redis_error: redisError?.message,
    })

    // Return different messages based on actual result
    const message = redisError
      ? "⚠️ 清空上下文时出现错误，请重试。"
      : hadMapping
        ? "✨ 上下文已清空，下一条消息将开始新对话。"
        : "✨ 已准备开始新对话。"

    return jsonResponse({
      success: true,
      data: {
        message,
        message_en: redisError
          ? "Error clearing context, please retry."
          : hadMapping
            ? "Context cleared. Next message will start a new conversation."
            : "Ready to start a new conversation.",
        conversation_id: input.conversation_id,
        cleared: hadMapping,
        redis_error: redisError ? true : undefined,
      },
    })
  } catch (error) {
    logLifecycleEvent(ctx, "error", {
      function: "clearConversation",
      error: error instanceof Error ? error.message : String(error),
    })

    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/chat/compact
 *
 * Compact the conversation context by summarizing the history.
 * This creates a new session with a summary of the previous conversation.
 */
export async function compactConversation(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", { function: "compactConversation" })

  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as CompactRequest

    if (!input.conversation_id) {
      return errorResponse("conversation_id is required", 400)
    }

    // Get the current session
    let sessionId: string | null = null
    if (ConversationStore.isInitialized()) {
      try {
        sessionId = await ConversationStore.get(input.conversation_id)
      } catch (redisError) {
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "redis_error",
            function: "compactConversation.get",
            error: redisError instanceof Error ? redisError.message : String(redisError),
          }),
        )
      }
    }
    if (!sessionId) {
      return jsonResponse({
        success: true,
        data: {
          message: "没有活跃的会话需要压缩。",
          message_en: "No active session to compact.",
          conversation_id: input.conversation_id,
          compacted: false,
        },
      })
    }

    // Import dependencies
    const { LocalSession } = await import("../../../api")
    const { SessionPrompt } = await import("../../../session/prompt")

    // Get the current session messages
    const messages = await LocalSession.messages({
      sessionID: sessionId,
    })

    if (!messages || messages.length < 3) {
      return jsonResponse({
        success: true,
        data: {
          message: "会话消息太少，无需压缩。",
          message_en: "Session has too few messages to compact.",
          conversation_id: input.conversation_id,
          compacted: false,
          message_count: messages?.length ?? 0,
        },
      })
    }

    // Create a summary prompt
    const summaryPrompt = `请用中文简洁地总结以下对话的关键信息和上下文，以便继续对话时保持连贯性。只输出总结，不要其他内容。

对话历史：
${messages.map((m: { info: { role: string }; parts: Array<{ type: string; text?: string }> }) =>
  `${m.info.role === "user" ? "用户" : "助手"}: ${extractTextFromParts(m.parts).slice(0, 500)}`
).join("\n\n")}`

    // Get summary using a quick model
    const summaryResult = await SessionPrompt.prompt({
      sessionID: sessionId,
      agent: "general",
      parts: [{ type: "text", text: summaryPrompt }],
    })

    // Extract summary text
    let summaryText = ""
    if (typeof summaryResult === "object" && "parts" in summaryResult) {
      summaryText = extractTextFromParts(summaryResult.parts as Array<{ type: string; text?: string }>)
    }

    // Create a new session with the summary as initial context
    const newSession = await LocalSession.create({
      title: `Compacted: ${new Date().toISOString()}`,
    })

    // Send the summary as the first message to establish context
    await SessionPrompt.prompt({
      sessionID: newSession.id,
      agent: "general",
      parts: [{ type: "text", text: `[上下文摘要]\n${summaryText}\n\n请基于以上上下文继续对话。` }],
    })

    // Update the mapping
    if (ConversationStore.isInitialized()) {
      try {
        await ConversationStore.set(input.conversation_id, newSession.id)
      } catch (redisError) {
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "redis_error",
            function: "compactConversation.set",
            error: redisError instanceof Error ? redisError.message : String(redisError),
          }),
        )
      }
    }

    const originalMessageCount = messages.length

    logLifecycleEvent(ctx, "function_end", {
      function: "compactConversation",
      success: true,
      conversation_id: input.conversation_id,
      original_messages: originalMessageCount,
      new_session_id: newSession.id,
    })

    return jsonResponse({
      success: true,
      data: {
        message: `上下文已压缩，从 ${originalMessageCount} 条消息精简为摘要。`,
        message_en: `Context compacted from ${originalMessageCount} messages to a summary.`,
        conversation_id: input.conversation_id,
        compacted: true,
        original_message_count: originalMessageCount,
        new_session_id: newSession.id,
      },
    })
  } catch (error) {
    logLifecycleEvent(ctx, "error", {
      function: "compactConversation",
      error: error instanceof Error ? error.message : String(error),
    })

    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// Autonomous Mode Toggle
// ============================================================================

interface AutonomousToggleRequest {
  /** Conversation ID */
  conversation_id: string
  /** User identifier */
  user_id: string
  /** Channel type */
  channel: string
  /** Enable or disable autonomous mode */
  enabled: boolean
  /** Optional autonomy level (wild, crazy, etc.) */
  autonomy_level?: string
}

/**
 * POST /api/v1/chat/autonomous
 *
 * Toggle autonomous mode for a conversation.
 * When enabled, the AI will use CLOSE decision framework to autonomously execute tasks.
 */
export async function toggleAutonomous(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", { function: "toggleAutonomous" })

  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as AutonomousToggleRequest

    if (!input.conversation_id) {
      return errorResponse("conversation_id is required", 400)
    }
    if (!input.user_id) {
      return errorResponse("user_id is required", 400)
    }
    if (typeof input.enabled !== "boolean") {
      return errorResponse("enabled must be a boolean", 400)
    }

    // Import autonomous session store
    const { AutonomousSessionStore } = await import("../store/autonomous-session")

    // Ensure store is initialized
    if (!AutonomousSessionStore.isInitialized()) {
      try {
        await AutonomousSessionStore.init()
      } catch (initError) {
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "autonomous_store_init_error",
            function: "toggleAutonomous",
            error: initError instanceof Error ? initError.message : String(initError),
          }),
        )
        // Continue without Redis - state won't persist but won't break
      }
    }

    // Set the autonomous state
    if (AutonomousSessionStore.isInitialized()) {
      await AutonomousSessionStore.setEnabled(
        input.conversation_id,
        input.enabled,
        input.user_id,
        input.autonomy_level
      )
    }

    logLifecycleEvent(ctx, "function_end", {
      function: "toggleAutonomous",
      success: true,
      conversation_id: input.conversation_id,
      enabled: input.enabled,
      autonomy_level: input.autonomy_level ?? "wild",
    })

    return jsonResponse({
      success: true,
      enabled: input.enabled,
      autonomyLevel: input.autonomy_level ?? "wild",
      message: input.enabled
        ? "Autonomous mode enabled"
        : "Autonomous mode disabled",
      message_zh: input.enabled
        ? "自主模式已启用"
        : "自主模式已关闭",
    })
  } catch (error) {
    logLifecycleEvent(ctx, "error", {
      function: "toggleAutonomous",
      error: error instanceof Error ? error.message : String(error),
    })

    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
