/**
 * Autonomous Evolution Loop
 *
 * Implements prioritized capability discovery for autonomous problem-solving:
 *
 * Phase 1: Internal Capabilities (Highest Priority)
 *   1.1 Agent Discovery - Find matching specialized agents
 *   1.2 Skill Discovery - Find matching Skills
 *   1.3 Hand Discovery - Find matching autonomous hands (cron/webhook/git)
 *   1.4 Tool Discovery - Internal + dynamic tools
 *
 * Phase 2: Learned Resources (High Priority)
 *   2.1 Knowledge Search - Sedimented solutions
 *   2.2 Memory Search - MEMORY.md + daily notes
 *
 * Phase 3: External Resources (Low Priority - Last Resort)
 *   3.1 Web Search - Documentation, StackOverflow
 *   3.2 GitHub Scout - Open-source libraries
 *   3.3 Code Generation - LLM-generated scripts
 *
 * Phase 4: Self-Improvement (Post-execution)
 *   4.1 Self-Reflection
 *   4.2 Knowledge Sedimentation
 *   4.3 Tool Learning
 *   4.4 Auto-Builder (Gap Detection)
 *
 * Part of Phase 3: Autonomous Problem-Solving Loop
 * Enhanced in Phase 13: Sandbox-Tool Registry Integration
 * Optimized in Phase 14: Capability Priority Optimization
 */

import { createEnhancedWebSearch, type EnhancedWebSearch } from "./enhanced-web-search"
import { createSandboxExecutor, type SandboxExecutor, type SandboxResult, type ReflectionResult } from "./sandbox"
import { getKnowledgeSedimentation, type KnowledgeSedimentation, type ExtractionContext } from "./knowledge-sedimentation"
import { createGithubScout, type GithubScout, type GithubScoutResult } from "./github-scout"
import { DynamicToolRegistry, type ToolTypes } from "@/memory/tools"
import type { FetchedContent } from "./web-search"
import { getLLMSolver, type LLMSolver, type ReflectionAnalysis } from "./llm-solver"
import { sedimentEvolutionSuccess, logEvolutionFailure } from "./memory-writer"
import { Log } from "@/util/log"
// NOTE: Builder imports are lazy to avoid circular dependency
// (autonomous/index.ts -> orchestrator -> evolution-loop -> builder -> validation -> memory/tools)
import type { GapDetectionResult, BuildResult, TaskFailure } from "../builder"

const log = Log.create({ service: "autonomous.evolution-loop" })

// Lazy import helpers to break circular dependency
const getBuilderModule = async () => import("../builder")

// Lazy imports for internal capability discovery (avoid circular deps)
const getAgentRegistry = async () => {
  const { getRegistry } = await import("@/agent/registry")
  return getRegistry()
}

const getSkillModule = async () => {
  const { Skill } = await import("@/skill/skill")
  return Skill
}

const getHandsBridge = async () => {
  const { getBridge } = await import("../hands/bridge")
  return getBridge()
}

// ============================================================================
// Types
// ============================================================================

/** Problem to solve autonomously */
export interface AutonomousProblem {
  /** Session ID for tracking */
  sessionId: string
  /** Problem description */
  description: string
  /** Error message if applicable */
  errorMessage?: string
  /** Technology/language context */
  technology?: string
  /** Working directory */
  workingDir?: string
  /** Maximum retries */
  maxRetries?: number
  /** Enable web search */
  enableWebSearch?: boolean
  /** Enable code execution */
  enableCodeExecution?: boolean
}

/** Solution attempt */
export interface SolutionAttempt {
  /** Attempt number */
  attempt: number
  /** Code that was tried */
  code?: string
  /** Execution result */
  executionResult?: SandboxResult
  /** Reflection on the attempt */
  reflection?: ReflectionResult
  /** Web sources consulted */
  webSources?: FetchedContent[]
  /** Whether this attempt succeeded */
  success: boolean
  /** Timestamp */
  timestamp: string
  /** Tool ID if using an existing tool */
  toolId?: string
  /** Tool name if using an existing tool */
  toolName?: string
}

/** Evolution loop result */
export interface EvolutionResult {
  /** Whether the problem was solved */
  solved: boolean
  /** Final solution (code or description) */
  solution?: string
  /** All attempts made */
  attempts: SolutionAttempt[]
  /** Knowledge entry ID if sedimented */
  knowledgeId?: string
  /** Tool ID if a new tool was learned */
  learnedToolId?: string
  /** Tool ID if an existing tool was used */
  usedToolId?: string
  /** GitHub Scout result if triggered */
  githubScoutResult?: GithubScoutResult
  /** Total duration in ms */
  durationMs: number
  /** Summary of the process */
  summary: string
  /** Gap detected from failure (auto-builder) */
  gapDetected?: GapDetectionResult
  /** Whether a build was attempted (auto-builder) */
  buildAttempted?: boolean
  /** Build result if attempted (auto-builder) */
  buildResult?: BuildResult

  // ─────────────────────────────────────────────────────────────────────────────
  // Capability Matching (New in Priority Optimization)
  // ─────────────────────────────────────────────────────────────────────────────

  /** Matched internal capability that solved the problem */
  matchedCapability?: {
    type: "agent" | "skill" | "hand" | "tool" | "knowledge" | "memory"
    identifier: string
    score: number
  }
  /** Summary of all capabilities searched during evolution */
  capabilitiesSearched?: Array<{
    type: string
    searched: boolean
    matchCount: number
    topMatchScore?: number
  }>
}

/** Evolution loop configuration */
export interface EvolutionConfig {
  /** Maximum number of retries */
  maxRetries: number
  /** Enable web search */
  enableWebSearch: boolean
  /** Enable code execution */
  enableCodeExecution: boolean
  /** Confidence threshold for web search */
  webSearchThreshold: number
  /** Enable knowledge sedimentation */
  enableSedimentation: boolean
  /** Enable tool learning from successful executions */
  enableToolLearning: boolean
  /** Enable searching for existing tools before generating code */
  enableToolDiscovery: boolean
  /** Minimum similarity score for tool matching (0-1) */
  toolMatchThreshold: number
  /** Enable LLM-based code generation (recommended) */
  enableLLMCodeGeneration: boolean
  /** Enable LLM-based reflection (recommended) */
  enableLLMReflection: boolean
  /** Enable GitHub Scout for open-source solution search */
  enableGithubScout: boolean
  /** GitHub Scout integration mode */
  githubScoutMode: "autonomous" | "recommend" | "ask"
  /** GitHub Scout minimum trigger confidence */
  githubScoutTriggerThreshold: number
  /** Enable auto-builder for gap detection on failure */
  enableAutoBuilder: boolean
  /** Enable meta-builder for concept creation */
  enableAutoMetaBuilder: boolean
  /** Minimum attempts before triggering auto-builder */
  autoBuilderMinAttempts: number
  /** CLOSE score threshold for auto-build approval */
  autoBuilderCloseThreshold: number

  // ─────────────────────────────────────────────────────────────────────────────
  // Phase 1: Internal Capability Discovery (New in Priority Optimization)
  // ─────────────────────────────────────────────────────────────────────────────

  /** Enable Agent discovery - find matching specialized agents */
  enableAgentDiscovery: boolean
  /** Enable Skill discovery - find matching Skills */
  enableSkillDiscovery: boolean
  /** Enable Hand discovery - find matching autonomous hands (cron/webhook/git) */
  enableHandDiscovery: boolean
  /** Enable Memory search - search MEMORY.md and daily notes */
  enableMemorySearch: boolean

  /** Agent match threshold (0-1). Lower = more lenient matching */
  agentMatchThreshold: number
  /** Skill match threshold (0-1) */
  skillMatchThreshold: number
  /** Hand match threshold (0-1) */
  handMatchThreshold: number

  /** Skip external resources (web search, GitHub, code gen) if internal capability matches */
  skipExternalIfInternalMatch: boolean
}

// ============================================================================
// Capability Match Results (Phase 1: Internal Discovery)
// ============================================================================

/** Agent match result */
export interface AgentMatchResult {
  matched: boolean
  agentName?: string
  displayName?: string
  description?: string
  score: number
  matchType: "trigger" | "search" | "recommend"
  recommendation?: string
}

/** Skill match result */
export interface SkillMatchResult {
  matched: boolean
  skillName?: string
  description?: string
  score: number
  recommendation?: string
}

/** Hand match result - for scheduled/triggered autonomous tasks */
export interface HandMatchResult {
  matched: boolean
  handId?: string
  handName?: string
  score: number
  /** Trigger type that matched (cron, webhook, git, file_watch) */
  triggerType?: string
  recommendation?: string
}

/** Memory search result - MEMORY.md and daily notes */
export interface MemorySearchResult {
  matched: boolean
  content?: string
  source?: "memory_md" | "daily_note"
  date?: string
  score: number
}

/** Capability search summary for tracking */
export interface CapabilitySearchSummary {
  type: "agent" | "skill" | "hand" | "tool" | "knowledge" | "memory"
  searched: boolean
  matchCount: number
  topMatchScore?: number
  durationMs?: number
}

// ============================================================================
// Default Configuration
// ============================================================================

const DEFAULT_CONFIG: EvolutionConfig = {
  maxRetries: 3,
  enableWebSearch: true,
  enableCodeExecution: true,
  webSearchThreshold: 0.4,
  enableSedimentation: true,
  enableToolLearning: true,
  enableToolDiscovery: true,
  toolMatchThreshold: 0.7,
  enableLLMCodeGeneration: true,
  enableLLMReflection: true,
  enableGithubScout: true,
  githubScoutMode: "autonomous",
  githubScoutTriggerThreshold: 0.6,
  enableAutoBuilder: true,
  enableAutoMetaBuilder: true,
  autoBuilderMinAttempts: 2,
  autoBuilderCloseThreshold: 5.5,

  // Phase 1: Internal Capability Discovery (highest priority)
  enableAgentDiscovery: true,
  enableSkillDiscovery: true,
  enableHandDiscovery: true,
  enableMemorySearch: true,

  // Match thresholds (lower = more lenient)
  agentMatchThreshold: 0.7,
  skillMatchThreshold: 0.6,
  handMatchThreshold: 0.7,

  // Early exit: skip external resources if internal capability matched
  skipExternalIfInternalMatch: true,
}

// ============================================================================
// Evolution Loop
// ============================================================================

/**
 * Autonomous evolution loop for problem solving
 *
 * Implements prioritized capability discovery:
 * Phase 1: Internal Capabilities → Phase 2: Learned Resources → Phase 3: External Resources
 */
export class EvolutionLoop {
  private config: EvolutionConfig
  private webSearch: EnhancedWebSearch | null = null
  private sandbox: SandboxExecutor | null = null
  private knowledge: KnowledgeSedimentation | null = null
  private llmSolver: LLMSolver | null = null
  private githubScout: GithubScout | null = null
  private previousAttempts: Array<{ code: string; error: string }> = []

  // Capability search tracking for result reporting
  private capabilitiesSearched: CapabilitySearchSummary[] = []
  // Best internal match found (for early exit decision)
  private bestInternalMatch: { type: string; score: number } | null = null

  constructor(config: Partial<EvolutionConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Initialize all components
   */
  async initialize(sessionId: string): Promise<void> {
    if (this.config.enableWebSearch) {
      this.webSearch = createEnhancedWebSearch(sessionId)
    }

    if (this.config.enableCodeExecution) {
      this.sandbox = await createSandboxExecutor()
    }

    if (this.config.enableSedimentation) {
      this.knowledge = await getKnowledgeSedimentation()
    }

    if (this.config.enableLLMCodeGeneration || this.config.enableLLMReflection) {
      this.llmSolver = getLLMSolver()
    }

    if (this.config.enableGithubScout) {
      this.githubScout = createGithubScout({
        integrationMode: this.config.githubScoutMode,
        triggerThreshold: this.config.githubScoutTriggerThreshold,
      })
    }
  }

  /**
   * Run the evolution loop to solve a problem
   *
   * Implements prioritized capability discovery:
   * Phase 1: Internal Capabilities → Phase 2: Learned Resources → Phase 3: External Resources
   */
  async evolve(problem: AutonomousProblem): Promise<EvolutionResult> {
    const startTime = Date.now()
    const attempts: SolutionAttempt[] = []
    const maxRetries = problem.maxRetries ?? this.config.maxRetries

    await this.initialize(problem.sessionId)
    this.previousAttempts = []
    this.capabilitiesSearched = []
    this.bestInternalMatch = null

    log.info("Starting evolution loop with prioritized capability discovery", {
      sessionId: problem.sessionId,
      problemPreview: problem.description.slice(0, 100),
      maxRetries,
    })

    // ═══════════════════════════════════════════════════════════════════════════
    // Phase 1: Internal Capabilities (Highest Priority)
    // ═══════════════════════════════════════════════════════════════════════════

    // 1.1 Agent Discovery - Find matching specialized agents
    if (this.config.enableAgentDiscovery) {
      const agentStart = Date.now()
      const agentResult = await this.tryAgentMatch(problem)
      this.recordCapabilitySearch("agent", true, agentResult.matched ? 1 : 0, agentResult.score, Date.now() - agentStart)

      if (agentResult.matched && agentResult.agentName) {
        log.info("Found matching agent", {
          agent: agentResult.agentName,
          score: agentResult.score,
          matchType: agentResult.matchType,
        })

        return {
          solved: true,
          solution: agentResult.recommendation,
          attempts: [],
          durationMs: Date.now() - startTime,
          summary: `Recommended agent: ${agentResult.displayName ?? agentResult.agentName}. ${agentResult.recommendation}`,
          matchedCapability: {
            type: "agent",
            identifier: agentResult.agentName,
            score: agentResult.score,
          },
          capabilitiesSearched: this.capabilitiesSearched,
        }
      }
    }

    // 1.2 Skill Discovery - Find matching Skills
    if (this.config.enableSkillDiscovery) {
      const skillStart = Date.now()
      const skillResult = await this.trySkillMatch(problem)
      this.recordCapabilitySearch("skill", true, skillResult.matched ? 1 : 0, skillResult.score, Date.now() - skillStart)

      if (skillResult.matched && skillResult.skillName) {
        log.info("Found matching skill", {
          skill: skillResult.skillName,
          score: skillResult.score,
        })

        return {
          solved: true,
          solution: skillResult.recommendation,
          attempts: [],
          durationMs: Date.now() - startTime,
          summary: `Recommended skill: /${skillResult.skillName}. ${skillResult.recommendation}`,
          matchedCapability: {
            type: "skill",
            identifier: skillResult.skillName,
            score: skillResult.score,
          },
          capabilitiesSearched: this.capabilitiesSearched,
        }
      }
    }

    // 1.3 Hand Discovery - Find matching autonomous hands (cron/webhook/git)
    if (this.config.enableHandDiscovery) {
      const handStart = Date.now()
      const handResult = await this.tryHandMatch(problem)
      this.recordCapabilitySearch("hand", true, handResult.matched ? 1 : 0, handResult.score, Date.now() - handStart)

      if (handResult.matched && handResult.handId) {
        log.info("Found matching hand", {
          hand: handResult.handName,
          triggerType: handResult.triggerType,
          score: handResult.score,
        })

        return {
          solved: true,
          solution: handResult.recommendation,
          attempts: [],
          durationMs: Date.now() - startTime,
          summary: `Recommended autonomous hand: ${handResult.handName}. ${handResult.recommendation}`,
          matchedCapability: {
            type: "hand",
            identifier: handResult.handId,
            score: handResult.score,
          },
          capabilitiesSearched: this.capabilitiesSearched,
        }
      }
    }

    // 1.4 Tool Discovery - Internal + dynamic tools
    if (this.config.enableToolDiscovery) {
      const toolStart = Date.now()
      const toolResult = await this.tryExistingTool(problem)

      if (toolResult) {
        this.recordCapabilitySearch("tool", true, 1, 1.0, Date.now() - toolStart)
        log.info("Problem solved with existing tool", { toolName: toolResult.toolName })

        return {
          solved: true,
          solution: toolResult.code,
          attempts: [{
            attempt: 1,
            code: toolResult.code,
            success: true,
            timestamp: new Date().toISOString(),
            toolId: toolResult.toolId,
            toolName: toolResult.toolName,
          }],
          usedToolId: toolResult.toolId,
          durationMs: Date.now() - startTime,
          summary: `Problem solved using existing tool: ${toolResult.toolName}`,
          matchedCapability: {
            type: "tool",
            identifier: toolResult.toolId,
            score: 1.0,
          },
          capabilitiesSearched: this.capabilitiesSearched,
        }
      } else {
        this.recordCapabilitySearch("tool", true, 0, 0, Date.now() - toolStart)
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Phase 2: Learned Resources (High Priority)
    // ═══════════════════════════════════════════════════════════════════════════

    // 2.1 Knowledge Search - Sedimented solutions
    const knowledgeStart = Date.now()
    const existingKnowledge = await this.searchExistingKnowledge(problem)

    if (existingKnowledge) {
      this.recordCapabilitySearch("knowledge", true, 1, 0.85, Date.now() - knowledgeStart)
      log.info("Found existing solution in knowledge base", { title: existingKnowledge.title })

      return {
        solved: true,
        solution: existingKnowledge.solution,
        attempts: [],
        knowledgeId: existingKnowledge.id,
        learnedToolId: undefined,
        usedToolId: undefined,
        durationMs: Date.now() - startTime,
        summary: `Found existing solution from knowledge base: ${existingKnowledge.title}`,
        matchedCapability: {
          type: "knowledge",
          identifier: existingKnowledge.id,
          score: 0.85,
        },
        capabilitiesSearched: this.capabilitiesSearched,
      }
    } else {
      this.recordCapabilitySearch("knowledge", true, 0, 0, Date.now() - knowledgeStart)
    }

    // 2.2 Memory Search - MEMORY.md and daily notes
    if (this.config.enableMemorySearch) {
      const memoryStart = Date.now()
      const memoryResult = await this.searchMemorySystem(problem)
      this.recordCapabilitySearch("memory", true, memoryResult.matched ? 1 : 0, memoryResult.score, Date.now() - memoryStart)

      if (memoryResult.matched && memoryResult.content) {
        log.info("Found relevant memory", {
          source: memoryResult.source,
          date: memoryResult.date,
          score: memoryResult.score,
        })

        // Memory provides context but doesn't fully solve - record as best internal match
        this.bestInternalMatch = { type: "memory", score: memoryResult.score }
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Early Exit Check
    // ═══════════════════════════════════════════════════════════════════════════

    if (this.config.skipExternalIfInternalMatch && this.bestInternalMatch && this.bestInternalMatch.score >= 0.8) {
      log.info("Strong internal match found - returning recommendation", this.bestInternalMatch)

      return {
        solved: false,
        attempts: [],
        durationMs: Date.now() - startTime,
        summary: `Strong internal capability match found (${this.bestInternalMatch.type}, score: ${this.bestInternalMatch.score.toFixed(2)}). Consider using the recommended capability instead of external resources.`,
        capabilitiesSearched: this.capabilitiesSearched,
      }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Phase 3: External Resources (Low Priority - Last Resort)
    // ═══════════════════════════════════════════════════════════════════════════

    log.info("No internal capability match - proceeding to external resources")

    // 3.1 Web Search - Documentation, StackOverflow
    let webSources: FetchedContent[] = []
    if (this.config.enableWebSearch && this.webSearch) {
      log.info("Phase 3.1: Searching for solutions online")
      webSources = await this.searchForSolutions(problem)
      log.info("Web search completed", { sourceCount: webSources.length })
    }

    // 3.2 GitHub Scout - Open-source libraries
    let githubScoutResult: GithubScoutResult | undefined
    if (this.config.enableGithubScout && this.githubScout) {
      log.info("Phase 3.2: GitHub Scout - searching for open-source solutions")
      githubScoutResult = await this.githubScout.scout({
        sessionId: problem.sessionId,
        description: problem.description,
        technology: problem.technology,
        workingDir: problem.workingDir,
      })

      // If GitHub Scout found and installed a solution, return early
      if (
        githubScoutResult.triggered &&
        githubScoutResult.integration?.success &&
        githubScoutResult.integration.action === "installed"
      ) {
        log.info("GitHub Scout found and installed solution", {
          repo: githubScoutResult.topRecommendation?.repo.fullName,
        })
        return {
          solved: true,
          solution: `Open-source solution installed: ${githubScoutResult.topRecommendation?.repo.fullName}`,
          attempts: [],
          githubScoutResult,
          durationMs: Date.now() - startTime,
          summary: githubScoutResult.summary,
          capabilitiesSearched: this.capabilitiesSearched,
        }
      }

      // Log if scout found recommendations but didn't auto-install
      if (githubScoutResult.triggered && githubScoutResult.topRecommendation) {
        log.info("GitHub Scout found recommendations", {
          topRepo: githubScoutResult.topRecommendation.repo.fullName,
          recommendation: githubScoutResult.topRecommendation.recommendation,
        })
      }
    }

    // 3.3 Code Generation - LLM-generated scripts (Programming as Fallback)
    if (this.config.enableCodeExecution && this.sandbox) {
      log.info("Phase 3.3: Generating solution code with LLM")

      // Generate initial code
      const codeResult = await this.generateSolutionCodeWithLLM(problem, webSources)
      if (!codeResult) {
        log.warn("Failed to generate solution code")
        return {
          solved: false,
          attempts,
          learnedToolId: undefined,
          usedToolId: undefined,
          githubScoutResult,
          durationMs: Date.now() - startTime,
          summary: "Could not generate solution code. Consider seeking human assistance.",
          capabilitiesSearched: this.capabilitiesSearched,
        }
      }

      let currentCode = codeResult.code
      let currentLanguage = codeResult.language
      let attemptNumber = 0

      // ═════════════════════════════════════════════════════════════════════════
      // Phase 4: Self-Reflection & Retry Loop
      // ═════════════════════════════════════════════════════════════════════════

      log.info("Phase 4: Executing with self-reflection loop")

      while (attemptNumber < maxRetries) {
        attemptNumber++
        log.info("Execution attempt", { attempt: attemptNumber, maxRetries })

        // Execute code in sandbox
        const execResult = await this.sandbox.execute({
          language: currentLanguage,
          code: currentCode,
          workingDir: problem.workingDir,
          timeoutMs: 30000,
        })

        // Reflect on result (like a human programmer analyzing stderr)
        const reflection = await this.reflectOnExecution({
          problem: problem.description,
          code: currentCode,
          exitCode: execResult.exitCode,
          stdout: execResult.stdout,
          stderr: execResult.stderr,
          durationMs: execResult.durationMs,
          timedOut: execResult.timedOut,
          technology: problem.technology,
          attemptNumber,
        })

        // Record attempt
        attempts.push({
          attempt: attemptNumber,
          code: currentCode,
          executionResult: execResult,
          reflection: {
            success: reflection.success,
            analysis: reflection.analysis,
            suggestedFix: reflection.suggestedCode,
            shouldRetry: reflection.shouldRetry,
            confidence: reflection.confidence,
          },
          webSources: attemptNumber === 1 ? webSources : undefined,
          success: reflection.success,
          timestamp: new Date().toISOString(),
        })

        // Exit Code 0 = Success (死磕到底，确保解决问题)
        if (execResult.exitCode === 0 && reflection.success) {
          log.info("Problem solved successfully", { attempts: attemptNumber })

          // Post-execution: Knowledge Sedimentation
          let knowledgeId: string | undefined
          if (this.config.enableSedimentation && this.knowledge) {
            log.info("Post-execution: Sedimenting solution to knowledge base")
            const entry = await this.sedimentSolution(
              problem,
              currentCode,
              webSources,
              attempts.map((a) => a.reflection!),
            )
            knowledgeId = entry.id
          }

          // Post-execution: Learn as reusable tool
          let learnedToolId: string | undefined
          if (this.config.enableToolLearning) {
            const learnedTool = await this.learnToolFromExecution(problem, currentCode, execResult)
            learnedToolId = learnedTool?.id
          }

          const successResult: EvolutionResult = {
            solved: true,
            solution: currentCode,
            attempts,
            knowledgeId,
            learnedToolId,
            githubScoutResult,
            durationMs: Date.now() - startTime,
            summary: `Problem solved after ${attemptNumber} attempt(s).${knowledgeId ? " Solution saved to knowledge base." : ""}${learnedToolId ? " Learned as reusable tool." : ""}`,
            capabilitiesSearched: this.capabilitiesSearched,
          }

          // Write to memory system (daily notes + MEMORY.md)
          await sedimentEvolutionSuccess(problem, successResult)

          return successResult
        }

        // Check if we should retry
        if (!reflection.shouldRetry || !reflection.suggestedCode) {
          log.info("Cannot retry - no suggested fix", { attemptNumber })
          break
        }

        // Record failed attempt for context
        this.previousAttempts.push({
          code: currentCode,
          error: execResult.stderr.slice(0, 500),
        })

        // Apply suggested fix for next iteration
        currentCode = reflection.suggestedCode
        log.info("Applying suggested fix for retry", {
          attempt: attemptNumber,
          rootCause: reflection.rootCause,
        })
      }
    }

    // No solution found
    log.warn("Could not solve problem", { attempts: attempts.length })
    const failureResult: EvolutionResult = {
      solved: false,
      attempts,
      learnedToolId: undefined,
      usedToolId: undefined,
      githubScoutResult,
      durationMs: Date.now() - startTime,
      summary: `Could not solve problem after ${attempts.length} attempts. Consider seeking human assistance.`,
      capabilitiesSearched: this.capabilitiesSearched,
    }

    // Auto-Builder: Detect capability gaps and attempt to build new concepts
    if (this.config.enableAutoBuilder && attempts.length >= this.config.autoBuilderMinAttempts) {
      const gap = await this.detectGapFromFailure(problem, attempts)

      if (gap) {
        failureResult.gapDetected = gap
        log.info("Capability gap detected", {
          gapId: gap.id,
          type: gap.type,
          confidence: gap.confidence,
          closeScore: gap.closeScore.total,
        })

        // Attempt auto-build if enabled and CLOSE score is sufficient
        if (
          this.config.enableAutoMetaBuilder &&
          gap.closeScore.total >= this.config.autoBuilderCloseThreshold
        ) {
          const buildResult = await this.attemptAutoBuild(gap, problem)
          failureResult.buildAttempted = true
          failureResult.buildResult = buildResult ?? undefined

          if (buildResult?.success) {
            log.info("Auto-built new concept", {
              type: buildResult.concept?.type,
              identifier: buildResult.concept?.identifier,
              durationMs: buildResult.durationMs,
            })
            failureResult.summary = `${failureResult.summary} Auto-built ${buildResult.concept?.type}: ${buildResult.concept?.identifier}.`
          } else {
            log.warn("Auto-build attempted but failed", {
              gapId: gap.id,
              summary: buildResult?.summary,
            })
          }
        }
      }
    }

    // Log failure to daily notes (for debugging and analysis)
    await logEvolutionFailure(problem, failureResult)

    return failureResult
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Phase 1: Internal Capability Discovery Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Try to find a matching agent for the problem
   *
   * Uses the Agent Registry to find specialized agents based on:
   * - Trigger keywords (e.g., "macro", "security", "tdd")
   * - Capability matching
   * - Fuzzy search
   */
  private async tryAgentMatch(problem: AutonomousProblem): Promise<AgentMatchResult> {
    try {
      const registry = await getAgentRegistry()
      const query = problem.description

      // First try trigger-based matching (highest confidence)
      const triggerMatches = registry.findByTrigger(query)
      if (triggerMatches.length > 0) {
        const bestMatch = triggerMatches[0]
        const score = 1.0 - (triggerMatches.indexOf(bestMatch) * 0.1)

        if (score >= this.config.agentMatchThreshold) {
          return {
            matched: true,
            agentName: bestMatch.name,
            displayName: bestMatch.displayName,
            description: bestMatch.shortDescription,
            score,
            matchType: "trigger",
            recommendation: `Use @${bestMatch.name} agent. ${bestMatch.shortDescription ?? ""}`,
          }
        }
      }

      // Then try fuzzy search
      const searchResults = registry.search(query, {
        limit: 3,
        threshold: 1 - this.config.agentMatchThreshold, // Fuse uses lower = better
      })

      if (searchResults.length > 0) {
        const best = searchResults[0]
        const score = 1 - best.score // Convert Fuse score (0=perfect) to our score (1=perfect)

        if (score >= this.config.agentMatchThreshold) {
          return {
            matched: true,
            agentName: best.agent.name,
            displayName: best.agent.displayName,
            description: best.agent.shortDescription,
            score,
            matchType: "search",
            recommendation: `Use @${best.agent.name} agent. ${best.agent.shortDescription ?? ""}`,
          }
        }
      }

      // Finally try recommendation (fallback for general queries)
      const recommended = registry.recommend(query)
      if (recommended && recommended.recommended) {
        return {
          matched: false,
          agentName: recommended.name,
          displayName: recommended.displayName,
          score: 0.5, // Lower score for general recommendation
          matchType: "recommend",
          recommendation: `Default agent: @${recommended.name}`,
        }
      }

      return { matched: false, score: 0, matchType: "search" }
    } catch (error) {
      log.error("Agent discovery failed", { error })
      return { matched: false, score: 0, matchType: "search" }
    }
  }

  /**
   * Try to find a matching skill for the problem
   *
   * Searches registered Skills based on name and description matching.
   */
  private async trySkillMatch(problem: AutonomousProblem): Promise<SkillMatchResult> {
    try {
      const Skill = await getSkillModule()
      const skills = await Skill.all()
      const query = problem.description.toLowerCase()

      let bestMatch: { name: string; description: string } | undefined
      let bestScore = 0

      for (const skill of skills) {
        // Simple keyword matching
        const nameMatch = query.includes(skill.name.toLowerCase()) ? 0.8 : 0
        const descMatch = this.calculateTextSimilarity(query, skill.description.toLowerCase())
        const score = Math.max(nameMatch, descMatch)

        if (score > bestScore) {
          bestScore = score
          bestMatch = skill
        }
      }

      if (bestMatch && bestScore >= this.config.skillMatchThreshold) {
        return {
          matched: true,
          skillName: bestMatch.name,
          description: bestMatch.description,
          score: bestScore,
          recommendation: `Use /${bestMatch.name} skill. ${bestMatch.description}`,
        }
      }

      return { matched: false, score: bestScore }
    } catch (error) {
      log.error("Skill discovery failed", { error })
      return { matched: false, score: 0 }
    }
  }

  /**
   * Try to find a matching hand for the problem
   *
   * Searches registered Hands (autonomous agents) based on:
   * - Schedule patterns (cron expressions)
   * - Trigger keywords
   * - Description matching
   */
  private async tryHandMatch(problem: AutonomousProblem): Promise<HandMatchResult> {
    try {
      const bridge = await getHandsBridge()

      // Check if hands service is healthy
      const isHealthy = await bridge.health()
      if (!isHealthy) {
        log.debug("Hands service not available - skipping hand discovery")
        return { matched: false, score: 0 }
      }

      const hands = await bridge.list()
      const query = problem.description.toLowerCase()

      // Keywords that suggest scheduled/automated tasks
      const scheduleKeywords = ["每天", "每周", "定时", "自动", "cron", "schedule", "daily", "weekly", "每小时", "hourly"]
      const webhookKeywords = ["webhook", "触发", "trigger", "api", "endpoint"]
      const gitKeywords = ["push", "pull request", "pr", "commit", "git"]

      let bestMatch: { id: string; name: string; agent: string; schedule?: string } | undefined
      let bestScore = 0
      let matchedTriggerType: string | undefined

      for (const hand of hands) {
        if (!hand.enabled) continue

        let score = 0
        let triggerType: string | undefined

        // Check for schedule-related keywords
        if (hand.schedule && scheduleKeywords.some(k => query.includes(k))) {
          score = 0.7
          triggerType = "cron"
        }

        // Check name matching
        if (query.includes(hand.name.toLowerCase())) {
          score = Math.max(score, 0.8)
        }

        // Check for webhook triggers
        if (webhookKeywords.some(k => query.includes(k))) {
          score = Math.max(score, 0.6)
          triggerType = "webhook"
        }

        // Check for git triggers
        if (gitKeywords.some(k => query.includes(k))) {
          score = Math.max(score, 0.6)
          triggerType = "git"
        }

        if (score > bestScore) {
          bestScore = score
          bestMatch = hand
          matchedTriggerType = triggerType
        }
      }

      if (bestMatch && bestScore >= this.config.handMatchThreshold) {
        return {
          matched: true,
          handId: bestMatch.id,
          handName: bestMatch.name,
          score: bestScore,
          triggerType: matchedTriggerType,
          recommendation: `Use autonomous hand: ${bestMatch.name} (Agent: ${bestMatch.agent}${bestMatch.schedule ? `, Schedule: ${bestMatch.schedule}` : ""})`,
        }
      }

      return { matched: false, score: bestScore }
    } catch (error) {
      log.error("Hand discovery failed", { error })
      return { matched: false, score: 0 }
    }
  }

  /**
   * Search the memory system (MEMORY.md and daily notes)
   *
   * This provides context from past interactions but typically
   * doesn't fully solve problems - it augments other solutions.
   */
  private async searchMemorySystem(problem: AutonomousProblem): Promise<MemorySearchResult> {
    try {
      // Memory search is handled by the knowledge sedimentation system
      // which already searches MEMORY.md and related files
      if (!this.knowledge) {
        return { matched: false, score: 0 }
      }

      const query = problem.errorMessage ?? problem.description
      const results = await this.knowledge.search(query, 1)

      if (results.length > 0 && results[0].relevanceScore > 0.6) {
        const entry = results[0].entry
        return {
          matched: true,
          content: entry.content,
          source: "memory_md",
          score: results[0].relevanceScore,
        }
      }

      return { matched: false, score: 0 }
    } catch (error) {
      log.error("Memory search failed", { error })
      return { matched: false, score: 0 }
    }
  }

  /**
   * Calculate simple text similarity using word overlap
   */
  private calculateTextSimilarity(text1: string, text2: string): number {
    const words1 = new Set(text1.split(/\s+/).filter(w => w.length > 2))
    const words2 = new Set(text2.split(/\s+/).filter(w => w.length > 2))

    if (words1.size === 0 || words2.size === 0) return 0

    let overlap = 0
    for (const word of words1) {
      if (words2.has(word)) overlap++
    }

    return overlap / Math.max(words1.size, words2.size)
  }

  /**
   * Record a capability search for result tracking
   */
  private recordCapabilitySearch(
    type: CapabilitySearchSummary["type"],
    searched: boolean,
    matchCount: number,
    topMatchScore: number,
    durationMs: number,
  ): void {
    this.capabilitiesSearched.push({
      type,
      searched,
      matchCount,
      topMatchScore,
      durationMs,
    })

    // Track best internal match for early exit decision
    if (matchCount > 0 && topMatchScore > (this.bestInternalMatch?.score ?? 0)) {
      this.bestInternalMatch = { type, score: topMatchScore }
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Existing Methods (unchanged)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Generate solution code using LLM
   */
  private async generateSolutionCodeWithLLM(
    problem: AutonomousProblem,
    webSources: FetchedContent[],
  ): Promise<{ code: string; language: "python" | "nodejs" | "shell" } | null> {
    if (this.config.enableLLMCodeGeneration && this.llmSolver) {
      const result = await this.llmSolver.generateCode({
        problem: problem.description,
        errorMessage: problem.errorMessage,
        technology: problem.technology,
        webSources,
        previousAttempts: this.previousAttempts,
      })

      if (result) {
        return {
          code: result.code,
          language: result.language,
        }
      }
    }

    const code = this.generateSolutionCodeFallback(problem, webSources)
    return code ? { code, language: this.detectLanguage(problem.technology) } : null
  }

  /**
   * Reflect on execution result using LLM
   */
  private async reflectOnExecution(context: {
    problem: string
    code: string
    exitCode: number
    stdout: string
    stderr: string
    durationMs: number
    timedOut: boolean
    technology?: string
    attemptNumber: number
  }): Promise<ReflectionAnalysis> {
    if (this.config.enableLLMReflection && this.llmSolver) {
      return this.llmSolver.reflect(context)
    }
    return this.reflectFallback(context)
  }

  /**
   * Fallback reflection using pattern matching
   */
  private reflectFallback(context: {
    exitCode: number
    stdout: string
    stderr: string
    timedOut: boolean
    attemptNumber: number
  }): ReflectionAnalysis {
    const success = context.exitCode === 0 && !context.timedOut

    let errorCategory: ReflectionAnalysis["errorCategory"] = "unknown"
    let rootCause: string | undefined

    if (context.timedOut) {
      errorCategory = "timeout"
      rootCause = "Execution timed out"
    } else if (context.stderr.toLowerCase().includes("syntaxerror")) {
      errorCategory = "syntax"
      rootCause = "Syntax error in code"
    } else if (
      context.stderr.toLowerCase().includes("modulenotfounderror") ||
      context.stderr.toLowerCase().includes("cannot find module")
    ) {
      errorCategory = "dependency"
      rootCause = "Missing dependency"
    } else if (context.exitCode !== 0) {
      errorCategory = "runtime"
      rootCause = `Exit code ${context.exitCode}`
    }

    return {
      success,
      analysis: success ? "Execution completed successfully" : context.stderr.slice(0, 200),
      rootCause,
      shouldRetry: !success && context.attemptNumber < 3,
      confidence: 0.3,
      errorCategory,
    }
  }

  /**
   * Search for solutions online
   */
  private async searchForSolutions(problem: AutonomousProblem): Promise<FetchedContent[]> {
    if (!this.webSearch) return []

    const { fetchedContent } = await this.webSearch.searchAndFetch({
      sessionId: problem.sessionId,
      problem: problem.description,
      errorMessage: problem.errorMessage,
      technology: problem.technology,
    })

    return fetchedContent
  }

  /**
   * Search existing knowledge base
   */
  private async searchExistingKnowledge(
    problem: AutonomousProblem,
  ): Promise<{ id: string; title: string; solution: string } | null> {
    if (!this.knowledge) return null

    const query = problem.errorMessage ?? problem.description
    const results = await this.knowledge.search(query, 1)

    if (results.length > 0 && results[0].relevanceScore > 0.8) {
      const entry = results[0].entry
      if (entry.codeExamples && entry.codeExamples.length > 0) {
        await this.knowledge.recordSuccess(entry.id)
        return {
          id: entry.id,
          title: entry.title,
          solution: entry.codeExamples[0].code,
        }
      }
    }

    return null
  }

  /**
   * Try to find and execute an existing tool
   */
  private async tryExistingTool(
    problem: AutonomousProblem,
  ): Promise<{ toolId: string; toolName: string; code: string } | null> {
    const query = problem.errorMessage ?? problem.description
    const language = this.detectLanguage(problem.technology)

    const results = await DynamicToolRegistry.search(query, {
      limit: 3,
      minScore: this.config.toolMatchThreshold,
      language: this.mapLanguageToToolLanguage(language),
    })

    if (results.length === 0) {
      return null
    }

    const bestMatch = results[0]
    const tool = bestMatch.tool

    if (this.sandbox) {
      const startTime = Date.now()
      const result = await this.sandbox.execute({
        language,
        code: tool.code,
        workingDir: problem.workingDir,
        timeoutMs: 30000,
      })

      const durationMs = Date.now() - startTime
      await DynamicToolRegistry.recordUsage(tool.id, result.exitCode === 0, durationMs)

      if (result.exitCode === 0) {
        return {
          toolId: tool.id,
          toolName: tool.name,
          code: tool.code,
        }
      }
    }

    return null
  }

  /**
   * Learn a tool from successful execution
   */
  private async learnToolFromExecution(
    problem: AutonomousProblem,
    code: string,
    result: SandboxResult,
  ): Promise<ToolTypes.DynamicTool | null> {
    const lines = code.split("\n").filter((l) => l.trim().length > 0)
    const isAutoGenerated = code.includes("Auto-generated verification script")
    const isTooShort = lines.length < 5
    const isTooLong = lines.length > 500
    const hasNoLogic = !code.includes("function") && !code.includes("def ") && !code.includes("const ")

    if (isAutoGenerated || isTooShort || isTooLong || hasNoLogic) {
      return null
    }

    const language = this.detectLanguage(problem.technology)

    const execution: ToolTypes.ExecutionRecord = {
      code,
      language: this.mapLanguageToToolLanguage(language),
      task: problem.description,
      output: result.stdout,
      exitCode: result.exitCode,
      durationMs: result.durationMs,
    }

    return DynamicToolRegistry.learnFromExecution(execution)
  }

  private mapLanguageToToolLanguage(language: "python" | "nodejs" | "shell"): "python" | "nodejs" | "bash" {
    return language === "shell" ? "bash" : language
  }

  /**
   * Fallback code generation using heuristics
   */
  private generateSolutionCodeFallback(problem: AutonomousProblem, webSources: FetchedContent[]): string | null {
    const codeExamples: string[] = []

    for (const source of webSources) {
      const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g
      let match: RegExpExecArray | null
      while ((match = codeBlockRegex.exec(source.content)) !== null) {
        const code = match[2].trim()
        if (code.length > 20) {
          codeExamples.push(code)
        }
      }
    }

    if (codeExamples.length > 0) {
      return codeExamples[0]
    }

    const language = this.detectLanguage(problem.technology)

    if (language === "python") {
      return `# Auto-generated verification script
# Problem: ${problem.description.slice(0, 100)}
${problem.errorMessage ? `# Error: ${problem.errorMessage.slice(0, 100)}` : ""}

def main():
    print("Verification script - implement solution here")
    # TODO: Implement solution
    return True

if __name__ == "__main__":
    result = main()
    exit(0 if result else 1)
`
    }

    if (language === "nodejs") {
      return `// Auto-generated verification script
// Problem: ${problem.description.slice(0, 100)}
${problem.errorMessage ? `// Error: ${problem.errorMessage.slice(0, 100)}` : ""}

async function main() {
  console.log("Verification script - implement solution here");
  // TODO: Implement solution
  return true;
}

main().then(result => process.exit(result ? 0 : 1));
`
    }

    return null
  }

  private detectLanguage(technology?: string): "python" | "nodejs" | "shell" {
    if (!technology) return "nodejs"

    const tech = technology.toLowerCase()

    if (tech.includes("python")) return "python"
    if (tech.includes("bash") || tech.includes("shell") || tech.includes("sh")) return "shell"

    return "nodejs"
  }

  /**
   * Sediment the successful solution
   */
  private async sedimentSolution(
    problem: AutonomousProblem,
    code: string,
    webSources: FetchedContent[],
    reflections: ReflectionResult[],
  ): Promise<{ id: string }> {
    if (!this.knowledge) {
      return { id: "" }
    }

    const context: ExtractionContext = {
      sessionId: problem.sessionId,
      problem: problem.description,
      errorMessage: problem.errorMessage,
      technology: problem.technology,
      solution: reflections.length > 0 ? reflections[reflections.length - 1].analysis : "Code execution successful",
      code,
      steps: reflections.map((r, i) => `Attempt ${i + 1}: ${r.analysis}`),
      webSources: webSources.map((s) => ({
        url: s.url,
        title: s.summary,
      })),
      reflection: reflections.length > 0 ? reflections[reflections.length - 1].analysis : undefined,
    }

    const entry = await this.knowledge.sediment(context)
    return { id: entry.id }
  }

  /**
   * Detect capability gap from a failed problem-solving attempt
   */
  private async detectGapFromFailure(
    problem: AutonomousProblem,
    attempts: SolutionAttempt[],
  ): Promise<GapDetectionResult | null> {
    const { getGapDetector } = await getBuilderModule()
    const gapDetector = getGapDetector()

    const taskFailure: TaskFailure = {
      sessionId: problem.sessionId,
      description: problem.description,
      errorMessage: problem.errorMessage,
      technology: problem.technology,
      attempts: attempts.length,
      webSearchUsed: this.config.enableWebSearch,
      toolSearchUsed: this.config.enableToolDiscovery,
      evolutionResult: {
        solved: false,
        attempts,
        durationMs: 0,
        summary: "Evolution failed",
      },
    }

    try {
      return await gapDetector.detectFromFailure(taskFailure)
    } catch (error) {
      log.error("Gap detection failed", { error })
      return null
    }
  }

  /**
   * Attempt to auto-build a new concept to address a detected gap
   */
  private async attemptAutoBuild(
    gap: GapDetectionResult,
    problem: AutonomousProblem,
  ): Promise<BuildResult | null> {
    const { getMetaBuilder } = await getBuilderModule()
    const metaBuilder = getMetaBuilder()

    try {
      await metaBuilder.initialize()

      return await metaBuilder.buildFromFailure(
        {
          sessionId: problem.sessionId,
          description: problem.description,
          errorMessage: problem.errorMessage,
          technology: problem.technology,
          attempts: this.previousAttempts.length,
          webSearchUsed: this.config.enableWebSearch,
          toolSearchUsed: this.config.enableToolDiscovery,
        },
        {
          workingDir: problem.workingDir,
        },
      )
    } catch (error) {
      log.error("Auto-build failed", { error, gapId: gap.id })
      return null
    }
  }

  /**
   * Cleanup resources
   */
  async cleanup(): Promise<void> {
    if (this.sandbox) {
      await this.sandbox.cleanup()
    }
  }
}

// ============================================================================
// Factory
// ============================================================================

/**
 * Create an evolution loop instance
 */
export function createEvolutionLoop(config?: Partial<EvolutionConfig>): EvolutionLoop {
  return new EvolutionLoop(config)
}

/**
 * Run a single evolution cycle for a problem
 */
export async function evolveProblem(problem: AutonomousProblem): Promise<EvolutionResult> {
  const loop = createEvolutionLoop()
  try {
    return await loop.evolve(problem)
  } finally {
    await loop.cleanup()
  }
}
