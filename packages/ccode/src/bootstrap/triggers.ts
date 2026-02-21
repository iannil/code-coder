import { Log } from "@/util/log"
import { Session } from "@/session"
import { BootstrapTypes } from "./types"
import { SkillGeneration } from "./generation"
import { ExecutionLoop } from "./verification"
import { CandidateStore } from "./candidate-store"
import { ConfidenceSystem } from "./confidence"

const log = Log.create({ service: "bootstrap.triggers" })

/**
 * Triggers module manages the various ways skill extraction can be triggered.
 */
export namespace Triggers {
  /**
   * Session context for tracking tool calls
   */
  interface SessionContext {
    sessionId: string
    toolCalls: BootstrapTypes.ToolCallRecord[]
    startTime: number
  }

  // Track active sessions
  const sessionContexts = new Map<string, SessionContext>()

  /**
   * Novelty detection result
   */
  interface NoveltyResult {
    isNovel: boolean
    reason?: string
    confidence: number
  }

  /**
   * Start tracking a session
   */
  export function startSession(sessionId: string): void {
    sessionContexts.set(sessionId, {
      sessionId,
      toolCalls: [],
      startTime: Date.now(),
    })
    log.info("started tracking session", { sessionId })
  }

  /**
   * Record a tool call in the session context
   */
  export function recordToolCall(
    sessionId: string,
    toolCall: BootstrapTypes.ToolCallRecord,
  ): void {
    const ctx = sessionContexts.get(sessionId)
    if (!ctx) {
      startSession(sessionId)
      sessionContexts.get(sessionId)!.toolCalls.push(toolCall)
      return
    }
    ctx.toolCalls.push(toolCall)
  }

  /**
   * Get session context
   */
  export function getSessionContext(sessionId: string): SessionContext | undefined {
    return sessionContexts.get(sessionId)
  }

  /**
   * End session tracking
   */
  export function endSession(sessionId: string): SessionContext | undefined {
    const ctx = sessionContexts.get(sessionId)
    sessionContexts.delete(sessionId)
    return ctx
  }

  /**
   * Detect if a solution is novel (worth crystallizing)
   */
  export async function detectNovelty(
    toolCalls: BootstrapTypes.ToolCallRecord[],
    problem: string,
    solution: string,
  ): Promise<NoveltyResult> {
    // Skip if too few tool calls
    if (toolCalls.length < 2) {
      return { isNovel: false, reason: "too few tool calls", confidence: 0 }
    }

    // Check for unique tool combinations
    const uniqueTools = [...new Set(toolCalls.map((tc) => tc.tool))]
    if (uniqueTools.length >= 3) {
      return {
        isNovel: true,
        reason: "multi-tool workflow",
        confidence: 0.7,
      }
    }

    // Check for repeated patterns (suggesting a workflow)
    const toolSequence = toolCalls.map((tc) => tc.tool).join(",")
    const hasRepetition = /(.+?,)\1/.test(toolSequence)
    if (hasRepetition) {
      return {
        isNovel: true,
        reason: "repeated pattern detected",
        confidence: 0.6,
      }
    }

    // Check for bash commands that could be scripted
    const bashCalls = toolCalls.filter((tc) => tc.tool === "bash")
    if (bashCalls.length >= 3) {
      return {
        isNovel: true,
        reason: "multi-step bash workflow",
        confidence: 0.65,
      }
    }

    // Check for solution length (longer solutions often valuable)
    if (solution.length > 500) {
      return {
        isNovel: true,
        reason: "substantial solution",
        confidence: 0.5,
      }
    }

    return { isNovel: false, confidence: 0.3 }
  }

  /**
   * Auto-trigger: Called after each successful tool use
   * Checks if the recent tool calls form a novel solution worth extracting
   */
  export async function onPostToolUse(input: {
    sessionId: string
    toolCall: BootstrapTypes.ToolCallRecord
  }): Promise<void> {
    recordToolCall(input.sessionId, input.toolCall)

    const ctx = sessionContexts.get(input.sessionId)
    if (!ctx || ctx.toolCalls.length < 3) {
      return // Not enough context yet
    }

    // Get recent tool calls (last 10)
    const recentCalls = ctx.toolCalls.slice(-10)

    // Simple heuristic: check if we should evaluate
    const shouldEvaluate =
      recentCalls.length >= 5 ||
      recentCalls.filter((tc) => tc.tool === "bash").length >= 3 ||
      new Set(recentCalls.map((tc) => tc.tool)).size >= 3

    if (!shouldEvaluate) {
      return
    }

    // Detect novelty (non-blocking)
    setImmediate(async () => {
      try {
        const problem = "Auto-detected pattern from tool calls"
        const solution = recentCalls
          .map((tc) => `${tc.tool}: ${JSON.stringify(tc.input).slice(0, 100)}`)
          .join("\n")

        const novelty = await detectNovelty(recentCalls, problem, solution)

        if (novelty.isNovel && novelty.confidence >= 0.5) {
          log.info("detected novel solution", {
            sessionId: input.sessionId,
            reason: novelty.reason,
            confidence: novelty.confidence,
          })

          // Queue for extraction (don't block)
          await SkillGeneration.extractAndStore({
            sessionId: input.sessionId,
            toolCalls: recentCalls,
            problem,
            solution,
            triggerType: "auto",
          })
        }
      } catch (error) {
        log.warn("auto-trigger failed", { error })
      }
    })
  }

  /**
   * Session end trigger: Process all pending candidates at session end
   */
  export async function onSessionEnd(input: {
    sessionId: string
    problem?: string
    solution?: string
  }): Promise<void> {
    const ctx = endSession(input.sessionId)
    if (!ctx || ctx.toolCalls.length < 3) {
      return
    }

    log.info("processing session end", {
      sessionId: input.sessionId,
      toolCallCount: ctx.toolCalls.length,
    })

    // Extract candidate if we have enough context
    const problem = input.problem ?? "Session workflow"
    const solution =
      input.solution ??
      ctx.toolCalls
        .map((tc) => `${tc.tool}: ${JSON.stringify(tc.input).slice(0, 100)}`)
        .join("\n")

    const candidate = await SkillGeneration.extractAndStore({
      sessionId: input.sessionId,
      toolCalls: ctx.toolCalls,
      problem,
      solution,
      triggerType: "session_end",
    })

    if (candidate) {
      // Run verification in background
      setImmediate(async () => {
        try {
          await ExecutionLoop.runVerificationLoop(candidate)
        } catch (error) {
          log.warn("verification failed", { error })
        }
      })
    }
  }

  /**
   * Manual trigger: User explicitly requests crystallization
   */
  export async function onManualCrystallize(input: {
    sessionId: string
    name?: string
    type?: BootstrapTypes.SkillType
    problem: string
    solution: string
  }): Promise<BootstrapTypes.SkillCandidate | null> {
    const ctx = sessionContexts.get(input.sessionId)
    const toolCalls = ctx?.toolCalls ?? []

    log.info("manual crystallize triggered", {
      sessionId: input.sessionId,
      name: input.name,
      type: input.type,
    })

    const candidate = await SkillGeneration.extractAndStore({
      sessionId: input.sessionId,
      toolCalls,
      problem: input.problem,
      solution: input.solution,
      triggerType: "manual",
    })

    if (!candidate) {
      return null
    }

    // Update with user preferences
    if (input.name || input.type) {
      await CandidateStore.update(candidate.id, (c) => {
        if (input.name) c.name = input.name
        if (input.type) c.type = input.type
      })
    }

    // Run verification
    await ExecutionLoop.runVerificationLoop(candidate)

    // Return updated candidate
    return (await CandidateStore.get(candidate.id)) ?? candidate
  }

  /**
   * Scheduled trigger: Analyze recent sessions for patterns
   */
  export async function onScheduledAnalysis(): Promise<{
    analyzed: number
    extracted: number
  }> {
    log.info("running scheduled analysis")

    const candidates = await CandidateStore.list()
    let analyzed = 0
    let extracted = 0

    // Re-verify pending candidates
    const pending = candidates.filter((c) => c.verification.status === "pending")
    for (const candidate of pending) {
      analyzed++
      try {
        const result = await ExecutionLoop.runVerificationLoop(candidate)
        if (result.passed) {
          extracted++
        }
      } catch (error) {
        log.warn("scheduled verification failed", { id: candidate.id, error })
      }
    }

    // Clean up old/low-confidence candidates
    await CandidateStore.cleanup()

    // Promote mature candidates
    const readyForPromotion = await CandidateStore.listReadyForPromotion()
    for (const candidate of readyForPromotion) {
      try {
        await SkillGeneration.persist(candidate)
        await CandidateStore.update(candidate.id, (c) => {
          c.verification.lastResult = JSON.stringify({
            type: "promoted",
            timestamp: Date.now(),
          })
        })
        extracted++
        log.info("promoted candidate to skill", { name: candidate.name })
      } catch (error) {
        log.warn("failed to promote candidate", { id: candidate.id, error })
      }
    }

    log.info("scheduled analysis complete", { analyzed, extracted })

    return { analyzed, extracted }
  }
}
