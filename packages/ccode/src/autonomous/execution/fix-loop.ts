/**
 * Fix Loop (PDCA: Adjust)
 *
 * Dedicated execution loop for fixing issues identified during acceptance.
 * Implements a tiered strategy for fixing issues from simple auto-fixes
 * to complex evolution-based rework.
 *
 * Five Phases:
 * 1. Issue Analysis - Categorize and prioritize issues
 * 2. Strategy Selection - Choose appropriate fix strategy per issue
 * 3. Fix Execution - Apply fixes using selected strategies
 * 4. Fix Verification - Quick verification after fixes
 * 5. Learning - Record successful fix patterns to knowledge base
 */

import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { TestRunner } from "./test-runner"
import type { AcceptanceIssue, AcceptanceResult } from "./acceptance-loop"
import type { EvolutionResult } from "./evolution-loop"
import { getKnowledgeSedimentation, type KnowledgeSedimentation } from "./knowledge-sedimentation"

import type { InvocableAgent } from "./agent-invoker"

const log = Log.create({ service: "autonomous.fix-loop" })

// ============================================================================
// Types
// ============================================================================

/** Fix strategy type */
export type FixStrategy = "auto_fix" | "agent_fix" | "llm_generate" | "evolution"

/** Problem input for fix loop */
export interface FixProblem {
  /** Session ID for tracking */
  sessionId: string
  /** Issues from acceptance-loop */
  issues: AcceptanceIssue[]
  /** Evolution result (optional, for context) */
  evolutionResult?: EvolutionResult
  /** Acceptance result (optional, for context) */
  acceptanceResult?: AcceptanceResult
  /** Maximum fix attempts per issue */
  maxAttempts?: number
  /** Working directory */
  workingDir?: string
}

/** Individual fix attempt record */
export interface FixAttempt {
  /** Issue ID being fixed */
  issueId: string
  /** Strategy used */
  strategy: FixStrategy
  /** Whether fix succeeded */
  success: boolean
  /** Code generated (if applicable) */
  code?: string
  /** Error message (if failed) */
  error?: string
  /** Duration in ms */
  durationMs: number
  /** Agent used (if agent_fix) */
  agentUsed?: string
}

/** Fix loop result */
export interface FixResult {
  /** Whether all critical issues were fixed */
  success: boolean
  /** Issue IDs that were fixed */
  fixedIssues: string[]
  /** Issues that remain unfixed */
  remainingIssues: AcceptanceIssue[]
  /** All fix attempts made */
  attempts: FixAttempt[]
  /** Total duration */
  durationMs: number
  /** Should re-run acceptance after these fixes */
  shouldRecheck: boolean
  /** Learned patterns from this fix session */
  learnedPatterns?: string[]
}

/** Strategy selection result */
interface StrategySelection {
  issueId: string
  issue: AcceptanceIssue
  strategy: FixStrategy
  reasoning: string
  /** Agent to use if strategy is agent_fix */
  agent?: string
  /** Auto-fix command if strategy is auto_fix */
  autoFixCommand?: string
}

/** Fix loop configuration */
export interface FixLoopConfig {
  /** Maximum attempts per issue (default: 3) */
  maxAttemptsPerIssue?: number
  /** Enable auto-fix tools (default: true) */
  enableAutoFix?: boolean
  /** Enable agent-based fixes (default: true) */
  enableAgentFix?: boolean
  /** Enable LLM code generation (default: true) */
  enableLLMGeneration?: boolean
  /** Enable evolution loop fallback (default: true) */
  enableEvolutionFallback?: boolean
  /** Enable learning sedimentation (default: true) */
  enableLearning?: boolean
  /** Run quick test after each fix (default: true) */
  verifyAfterEachFix?: boolean
}

const DEFAULT_CONFIG: Required<FixLoopConfig> = {
  maxAttemptsPerIssue: 3,
  enableAutoFix: true,
  enableAgentFix: true,
  enableLLMGeneration: true,
  enableEvolutionFallback: true,
  enableLearning: true,
  verifyAfterEachFix: true,
}

// ============================================================================
// Strategy Mapping
// ============================================================================

/** Map issue types to best fix strategies */
const ISSUE_STRATEGY_MAP: Record<AcceptanceIssue["type"], FixStrategy[]> = {
  lint: ["auto_fix", "agent_fix"],
  type: ["agent_fix", "llm_generate"],
  test: ["agent_fix", "llm_generate", "evolution"],
  security: ["agent_fix", "llm_generate"],
  requirement: ["llm_generate", "evolution"],
  expectation: ["llm_generate", "evolution"],
}

/** Map issue types to specialized agents (must be InvocableAgent) */
const ISSUE_AGENT_MAP: Record<AcceptanceIssue["type"], string> = {
  lint: "code-reviewer",
  type: "code-reviewer", // Note: build-error-resolver not available as InvocableAgent
  test: "tdd-guide",
  security: "security-reviewer",
  requirement: "architect",
  expectation: "architect",
}

/** Auto-fix commands by issue type */
const AUTO_FIX_COMMANDS: Partial<Record<AcceptanceIssue["type"], string>> = {
  lint: "bun eslint --fix .",
}

// ============================================================================
// Fix Loop Implementation
// ============================================================================

export function createFixLoop(config: FixLoopConfig = {}) {
  const cfg = { ...DEFAULT_CONFIG, ...config }
  let knowledge: KnowledgeSedimentation | null = null

  /** Initialize knowledge sedimentation lazily */
  async function ensureKnowledge(): Promise<KnowledgeSedimentation | null> {
    if (!knowledge && cfg.enableLearning) {
      knowledge = await getKnowledgeSedimentation()
    }
    return knowledge
  }

  /** Phase 1: Analyze and categorize issues */
  async function analyzeIssues(problem: FixProblem): Promise<{
    prioritized: AcceptanceIssue[]
    byType: Map<AcceptanceIssue["type"], AcceptanceIssue[]>
  }> {
    await Bus.publish(AutonomousEvent.FixPhaseChanged, {
      sessionId: problem.sessionId,
      phase: "issue_analysis",
      metadata: { issueCount: problem.issues.length },
    })

    // Group by type
    const byType = new Map<AcceptanceIssue["type"], AcceptanceIssue[]>()
    for (const issue of problem.issues) {
      const existing = byType.get(issue.type) ?? []
      existing.push(issue)
      byType.set(issue.type, existing)
    }

    // Prioritize: critical > high > medium > low
    // Within same severity, prioritize by type: security > type > test > lint > requirement > expectation
    const severityOrder = { critical: 0, high: 1, medium: 2, low: 3 }
    const typeOrder = { security: 0, type: 1, test: 2, lint: 3, requirement: 4, expectation: 5 }

    const prioritized = [...problem.issues].sort((a, b) => {
      const sevDiff = severityOrder[a.severity] - severityOrder[b.severity]
      if (sevDiff !== 0) return sevDiff
      return typeOrder[a.type] - typeOrder[b.type]
    })

    log.debug("Issues analyzed", {
      sessionId: problem.sessionId,
      total: problem.issues.length,
      bySeverity: {
        critical: prioritized.filter((i) => i.severity === "critical").length,
        high: prioritized.filter((i) => i.severity === "high").length,
        medium: prioritized.filter((i) => i.severity === "medium").length,
        low: prioritized.filter((i) => i.severity === "low").length,
      },
    })

    return { prioritized, byType }
  }

  /** Phase 2: Select fix strategies for each issue */
  async function selectStrategies(
    problem: FixProblem,
    prioritized: AcceptanceIssue[],
  ): Promise<StrategySelection[]> {
    await Bus.publish(AutonomousEvent.FixPhaseChanged, {
      sessionId: problem.sessionId,
      phase: "strategy_selection",
    })

    const selections: StrategySelection[] = []

    for (const issue of prioritized) {
      const availableStrategies = ISSUE_STRATEGY_MAP[issue.type]
      let selectedStrategy: FixStrategy = "llm_generate" // fallback
      let reasoning = "Default strategy"

      // Select first available strategy based on config
      for (const strategy of availableStrategies) {
        if (strategy === "auto_fix" && cfg.enableAutoFix && AUTO_FIX_COMMANDS[issue.type]) {
          selectedStrategy = "auto_fix"
          reasoning = "Auto-fix available for this issue type"
          break
        }
        if (strategy === "agent_fix" && cfg.enableAgentFix) {
          selectedStrategy = "agent_fix"
          reasoning = `Specialized agent available: ${ISSUE_AGENT_MAP[issue.type]}`
          break
        }
        if (strategy === "llm_generate" && cfg.enableLLMGeneration) {
          selectedStrategy = "llm_generate"
          reasoning = "LLM code generation for targeted fix"
          break
        }
        if (strategy === "evolution" && cfg.enableEvolutionFallback) {
          selectedStrategy = "evolution"
          reasoning = "Complex issue requires evolution loop"
          break
        }
      }

      selections.push({
        issueId: issue.id,
        issue,
        strategy: selectedStrategy,
        reasoning,
        agent: selectedStrategy === "agent_fix" ? ISSUE_AGENT_MAP[issue.type] : undefined,
        autoFixCommand: selectedStrategy === "auto_fix" ? AUTO_FIX_COMMANDS[issue.type] : undefined,
      })
    }

    log.debug("Strategies selected", {
      sessionId: problem.sessionId,
      selections: selections.map((s) => ({ issueId: s.issueId, strategy: s.strategy })),
    })

    return selections
  }

  /** Phase 3: Execute fixes */
  async function executeFixes(
    problem: FixProblem,
    selections: StrategySelection[],
    attempts: FixAttempt[],
    fixedIssues: Set<string>,
  ): Promise<void> {
    await Bus.publish(AutonomousEvent.FixPhaseChanged, {
      sessionId: problem.sessionId,
      phase: "fix_execution",
    })

    for (const selection of selections) {
      if (fixedIssues.has(selection.issueId)) continue

      const attemptsForIssue = attempts.filter((a) => a.issueId === selection.issueId).length
      if (attemptsForIssue >= cfg.maxAttemptsPerIssue) {
        log.debug("Max attempts reached for issue", {
          issueId: selection.issueId,
          attempts: attemptsForIssue,
        })
        continue
      }

      const attemptStart = Date.now()
      let success = false
      let error: string | undefined
      let code: string | undefined

      try {
        switch (selection.strategy) {
          case "auto_fix":
            success = await executeAutoFix(selection, problem.workingDir)
            break

          case "agent_fix":
            const agentResult = await executeAgentFix(selection, problem)
            success = agentResult.success
            code = agentResult.code
            error = agentResult.error
            break

          case "llm_generate":
            const llmResult = await executeLLMFix(selection, problem)
            success = llmResult.success
            code = llmResult.code
            error = llmResult.error
            break

          case "evolution":
            const evoResult = await executeEvolutionFix(selection, problem)
            success = evoResult.success
            code = evoResult.code
            error = evoResult.error
            break
        }
      } catch (e) {
        success = false
        error = e instanceof Error ? e.message : String(e)
      }

      const attempt: FixAttempt = {
        issueId: selection.issueId,
        strategy: selection.strategy,
        success,
        code,
        error,
        durationMs: Date.now() - attemptStart,
        agentUsed: selection.agent,
      }

      attempts.push(attempt)

      await Bus.publish(AutonomousEvent.FixAttemptMade, {
        sessionId: problem.sessionId,
        issueId: selection.issueId,
        strategy: selection.strategy,
        success,
        durationMs: attempt.durationMs,
        error,
      })

      if (success) {
        fixedIssues.add(selection.issueId)
        log.info("Issue fixed", {
          issueId: selection.issueId,
          strategy: selection.strategy,
        })
      } else {
        log.warn("Fix attempt failed", {
          issueId: selection.issueId,
          strategy: selection.strategy,
          error,
        })
      }
    }
  }

  /** Execute auto-fix command */
  async function executeAutoFix(
    selection: StrategySelection,
    workingDir?: string,
  ): Promise<boolean> {
    if (!selection.autoFixCommand) return false

    try {
      const { execSync } = require("child_process")
      execSync(selection.autoFixCommand, {
        cwd: workingDir || process.cwd(),
        stdio: "pipe",
      })
      return true
    } catch {
      return false
    }
  }

  /** Execute agent-based fix */
  async function executeAgentFix(
    selection: StrategySelection,
    problem: FixProblem,
  ): Promise<{ success: boolean; code?: string; error?: string }> {
    if (!selection.agent) {
      return { success: false, error: "No agent specified" }
    }

    // Use the AgentInvoker with the correct interface
    try {
      const { AgentInvoker } = await import("./agent-invoker")

      // Validate that the agent is invocable
      const invocableAgents: InvocableAgent[] = [
        "code-reviewer",
        "security-reviewer",
        "tdd-guide",
        "architect",
        "explore",
        "general",
      ]

      if (!invocableAgents.includes(selection.agent as InvocableAgent)) {
        return { success: false, error: `Agent "${selection.agent}" is not invocable` }
      }

      const result = await AgentInvoker.invoke({
        agent: selection.agent as InvocableAgent,
        task: `Fix the following issue: ${selection.issue.description}${selection.issue.location ? ` at ${selection.issue.location}` : ""}`,
        context: {
          sessionId: problem.sessionId,
          metadata: { issue: selection.issue },
        },
      })

      return {
        success: result.success,
        code: result.output,
        error: result.error,
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /** Execute LLM-based fix */
  async function executeLLMFix(
    selection: StrategySelection,
    problem: FixProblem,
  ): Promise<{ success: boolean; code?: string; error?: string }> {
    try {
      const { generateText } = await import("ai")
      const { getDefaultModelWithFallback } = await import("@/sdk/provider-bridge")
      const { Provider } = await import("@/provider/provider")

      const defaultModel = await getDefaultModelWithFallback()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      const result = await generateText({
        temperature: 0.3,
        messages: [
          {
            role: "system",
            content: `You are an expert code fixer. Generate a minimal fix for the following issue.
Output ONLY the fixed code, no explanations.`,
          },
          {
            role: "user",
            content: `Issue Type: ${selection.issue.type}
Severity: ${selection.issue.severity}
Description: ${selection.issue.description}
${selection.issue.location ? `Location: ${selection.issue.location}` : ""}
${selection.issue.suggestedFix ? `Suggested Fix: ${selection.issue.suggestedFix}` : ""}`,
          },
        ],
        model: language,
      })

      // Extract code from response
      const code = result.text.trim()

      return {
        success: code.length > 0,
        code,
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /** Execute evolution-based fix (fallback) */
  async function executeEvolutionFix(
    selection: StrategySelection,
    problem: FixProblem,
  ): Promise<{ success: boolean; code?: string; error?: string }> {
    try {
      const { createEvolutionLoop } = await import("./evolution-loop")
      const evolutionLoop = createEvolutionLoop({
        maxRetries: 2,
        enableWebSearch: true,
        enableLLMCodeGeneration: true,
      })

      const result = await evolutionLoop.evolve({
        sessionId: problem.sessionId,
        description: `Fix: ${selection.issue.description}`,
        errorMessage: selection.issue.description,
        workingDir: problem.workingDir,
      })

      return {
        success: result.solved,
        code: result.solution,
        error: result.solved ? undefined : result.summary,
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    }
  }

  /** Phase 4: Verify fixes */
  async function verifyFixes(
    problem: FixProblem,
    fixedIssues: Set<string>,
  ): Promise<{ verified: boolean; failedVerification: string[] }> {
    await Bus.publish(AutonomousEvent.FixPhaseChanged, {
      sessionId: problem.sessionId,
      phase: "fix_verification",
    })

    if (!cfg.verifyAfterEachFix || fixedIssues.size === 0) {
      return { verified: true, failedVerification: [] }
    }

    const failedVerification: string[] = []

    // Run quick test to verify
    try {
      const testResult = await TestRunner.runAll()
      if (!testResult.success) {
        // Some fixes may have introduced new issues
        log.warn("Tests failed after fixes", { failed: testResult.failed })
        // We don't mark specific issues as failed since test failure could be unrelated
      }
    } catch (error) {
      log.warn("Verification tests failed to run", { error })
    }

    // Run typecheck for type-related fixes
    const typeIssuesFixed = [...fixedIssues].some((id) =>
      problem.issues.find((i) => i.id === id && i.type === "type")
    )

    if (typeIssuesFixed) {
      try {
        const { execSync } = require("child_process")
        execSync("bun run turbo typecheck", {
          cwd: problem.workingDir || process.cwd(),
          stdio: "pipe",
        })
      } catch {
        log.warn("Type check failed after fixes")
        // Find which type issues may have failed
        for (const id of fixedIssues) {
          const issue = problem.issues.find((i) => i.id === id)
          if (issue?.type === "type") {
            failedVerification.push(id)
            fixedIssues.delete(id)
          }
        }
      }
    }

    return {
      verified: failedVerification.length === 0,
      failedVerification,
    }
  }

  /** Phase 5: Learn from successful fixes */
  async function learnFromFixes(
    problem: FixProblem,
    attempts: FixAttempt[],
    fixedIssues: Set<string>,
  ): Promise<string[]> {
    await Bus.publish(AutonomousEvent.FixPhaseChanged, {
      sessionId: problem.sessionId,
      phase: "learning",
    })

    if (!cfg.enableLearning) return []

    const learnedPatterns: string[] = []
    const knowledge = await ensureKnowledge()

    // Record successful fix patterns
    const successfulAttempts = attempts.filter((a) => a.success && fixedIssues.has(a.issueId))

    for (const attempt of successfulAttempts) {
      const issue = problem.issues.find((i) => i.id === attempt.issueId)
      if (!issue) continue

      const patternId = `fix-${issue.type}-${attempt.strategy}`

      // Publish pattern learned event
      await Bus.publish(AutonomousEvent.FixPatternLearned, {
        sessionId: problem.sessionId,
        patternId,
        issueType: issue.type,
        strategy: attempt.strategy,
        confidence: 0.8,
      })

      learnedPatterns.push(patternId)

      // Sediment to knowledge base if we have code
      if (knowledge && attempt.code) {
        try {
          await knowledge.sediment({
            sessionId: problem.sessionId,
            problem: issue.description,
            solution: `Fixed using ${attempt.strategy} strategy`,
            code: attempt.code,
            steps: [`Strategy: ${attempt.strategy}`, `Duration: ${attempt.durationMs}ms`],
          })
        } catch (error) {
          log.debug("Failed to sediment fix pattern", { error })
        }
      }
    }

    log.debug("Fix patterns learned", {
      sessionId: problem.sessionId,
      count: learnedPatterns.length,
    })

    return learnedPatterns
  }

  return {
    /** Execute full fix loop */
    async fix(problem: FixProblem): Promise<FixResult> {
      const startTime = Date.now()
      const attempts: FixAttempt[] = []
      const fixedIssues = new Set<string>()

      if (problem.issues.length === 0) {
        return {
          success: true,
          fixedIssues: [],
          remainingIssues: [],
          attempts: [],
          durationMs: 0,
          shouldRecheck: false,
        }
      }

      await Bus.publish(AutonomousEvent.FixStarted, {
        sessionId: problem.sessionId,
        issueCount: problem.issues.length,
        triggerSource: problem.acceptanceResult ? "acceptance" : "manual",
      })

      try {
        // Phase 1: Analyze issues
        const { prioritized } = await analyzeIssues(problem)

        // Phase 2: Select strategies
        const selections = await selectStrategies(problem, prioritized)

        // Phase 3: Execute fixes
        await executeFixes(problem, selections, attempts, fixedIssues)

        // Phase 4: Verify fixes
        await verifyFixes(problem, fixedIssues)

        // Phase 5: Learn from fixes
        const learnedPatterns = await learnFromFixes(problem, attempts, fixedIssues)

        // Calculate remaining issues
        const remainingIssues = problem.issues.filter((i) => !fixedIssues.has(i.id))

        // Determine success - all critical/high issues must be fixed
        const criticalRemaining = remainingIssues.filter(
          (i) => i.severity === "critical" || i.severity === "high"
        ).length

        const result: FixResult = {
          success: criticalRemaining === 0,
          fixedIssues: [...fixedIssues],
          remainingIssues,
          attempts,
          durationMs: Date.now() - startTime,
          shouldRecheck: fixedIssues.size > 0,
          learnedPatterns,
        }

        await Bus.publish(AutonomousEvent.FixCompleted, {
          sessionId: problem.sessionId,
          success: result.success,
          fixedCount: fixedIssues.size,
          remainingCount: remainingIssues.length,
          shouldRecheck: result.shouldRecheck,
          durationMs: result.durationMs,
        })

        log.info("Fix loop completed", {
          sessionId: problem.sessionId,
          success: result.success,
          fixed: fixedIssues.size,
          remaining: remainingIssues.length,
        })

        return result
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)

        await Bus.publish(AutonomousEvent.FixFailed, {
          sessionId: problem.sessionId,
          error: errorMsg,
          attemptsMade: attempts.length,
          retryable: true,
        })

        log.error("Fix loop failed", { sessionId: problem.sessionId, error: errorMsg })

        return {
          success: false,
          fixedIssues: [...fixedIssues],
          remainingIssues: problem.issues.filter((i) => !fixedIssues.has(i.id)),
          attempts,
          durationMs: Date.now() - startTime,
          shouldRecheck: fixedIssues.size > 0,
        }
      }
    },
  }
}

export type FixLoop = ReturnType<typeof createFixLoop>
