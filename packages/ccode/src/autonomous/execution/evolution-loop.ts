/**
 * Autonomous Evolution Loop
 *
 * Implements the 5-step autonomous problem-solving evolution cycle:
 * 1. Resource Retrieval - Search documentation when confidence is low
 * 2. Tool Discovery - Check for existing reusable tools
 * 3. Dynamic Code Generation - Write temporary scripts as fallback
 * 4. Self-Reflection & Retry - Analyze errors and correct code
 * 5. Knowledge Sedimentation - Store successful solutions + learn tools
 *
 * Part of Phase 3: Autonomous Problem-Solving Loop
 * Enhanced in Phase 13: Sandbox-Tool Registry Integration
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

const log = Log.create({ service: "autonomous.evolution-loop" })

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
}

// ============================================================================
// Evolution Loop
// ============================================================================

/**
 * Autonomous evolution loop for problem solving
 *
 * Implements the 4-step evolution cycle from goals.md 3.3:
 * 1. 主动资源检索 (Proactive Online Research)
 * 2. 动态编程保底 (Programming as Fallback)
 * 3. 自主反思与无限重试 (Self-Reflection & Retry)
 * 4. 沉淀与进化 (Knowledge Sedimentation)
 */
export class EvolutionLoop {
  private config: EvolutionConfig
  private webSearch: EnhancedWebSearch | null = null
  private sandbox: SandboxExecutor | null = null
  private knowledge: KnowledgeSedimentation | null = null
  private llmSolver: LLMSolver | null = null
  private githubScout: GithubScout | null = null
  private previousAttempts: Array<{ code: string; error: string }> = []

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
   * Implements the 4-step evolution cycle:
   * 1. 主动资源检索 - Search online when confidence is low
   * 2. 动态编程保底 - Generate code with LLM when no existing solution
   * 3. 自主反思与无限重试 - Analyze stderr like a human, fix and retry
   * 4. 沉淀与进化 - Store successful solutions for future reuse
   */
  async evolve(problem: AutonomousProblem): Promise<EvolutionResult> {
    const startTime = Date.now()
    const attempts: SolutionAttempt[] = []
    const maxRetries = problem.maxRetries ?? this.config.maxRetries

    await this.initialize(problem.sessionId)
    this.previousAttempts = []

    log.info("Starting evolution loop", {
      sessionId: problem.sessionId,
      problemPreview: problem.description.slice(0, 100),
      maxRetries,
    })

    // Step 1: 主动资源检索 (Proactive Online Research)
    let webSources: FetchedContent[] = []
    if (this.config.enableWebSearch && this.webSearch) {
      log.info("Step 1: Searching for solutions online")
      webSources = await this.searchForSolutions(problem)
      log.info("Web search completed", { sourceCount: webSources.length })
    }

    // Step 1.5: GitHub Scout - Search for open-source solutions
    let githubScoutResult: GithubScoutResult | undefined
    if (this.config.enableGithubScout && this.githubScout) {
      log.info("Step 1.5: GitHub Scout - searching for open-source solutions")
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

    // Step 2: Check existing knowledge
    const existingKnowledge = await this.searchExistingKnowledge(problem)
    if (existingKnowledge) {
      log.info("Found existing solution in knowledge base", { title: existingKnowledge.title })
      return {
        solved: true,
        solution: existingKnowledge.solution,
        attempts: [],
        knowledgeId: existingKnowledge.id,
        learnedToolId: undefined,
        usedToolId: undefined,
        githubScoutResult,
        durationMs: Date.now() - startTime,
        summary: `Found existing solution from knowledge base: ${existingKnowledge.title}`,
      }
    }

    // Step 2.5: Tool Discovery - Check for existing reusable tools
    if (this.config.enableToolDiscovery) {
      const toolResult = await this.tryExistingTool(problem)
      if (toolResult) {
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
          githubScoutResult,
          durationMs: Date.now() - startTime,
          summary: `Problem solved using existing tool: ${toolResult.toolName}`,
        }
      }
    }

    // Step 2: 动态编程保底 (Programming as Fallback)
    if (this.config.enableCodeExecution && this.sandbox) {
      log.info("Step 2: Generating solution code with LLM")

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
        }
      }

      let currentCode = codeResult.code
      let currentLanguage = codeResult.language
      let attemptNumber = 0

      // Step 3: 自主反思与无限重试 (Self-Reflection & Infinite Retry)
      log.info("Step 3: Executing with self-reflection loop")

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

          // Step 4: 沉淀与进化 (Knowledge Sedimentation)
          let knowledgeId: string | undefined
          if (this.config.enableSedimentation && this.knowledge) {
            log.info("Step 4: Sedimenting solution to knowledge base")
            const entry = await this.sedimentSolution(
              problem,
              currentCode,
              webSources,
              attempts.map((a) => a.reflection!),
            )
            knowledgeId = entry.id
          }

          // Learn as reusable tool
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
    }

    // Log failure to daily notes (for debugging and analysis)
    await logEvolutionFailure(problem, failureResult)

    return failureResult
  }

  /**
   * Generate solution code using LLM
   *
   * This implements the "动态编程保底" (Programming as Fallback) mechanism.
   * Uses LLM to generate a complete, executable script based on the problem,
   * web sources, and previous failed attempts.
   */
  private async generateSolutionCodeWithLLM(
    problem: AutonomousProblem,
    webSources: FetchedContent[],
  ): Promise<{ code: string; language: "python" | "nodejs" | "shell" } | null> {
    // Try LLM-based generation first
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

    // Fallback to heuristic-based generation
    const code = this.generateSolutionCodeFallback(problem, webSources)
    return code ? { code, language: this.detectLanguage(problem.technology) } : null
  }

  /**
   * Reflect on execution result using LLM
   *
   * This implements the "自主反思" (Self-Reflection) mechanism.
   * Analyzes stderr like a human programmer, identifying root causes
   * and suggesting fixes.
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
    // Try LLM-based reflection first
    if (this.config.enableLLMReflection && this.llmSolver) {
      return this.llmSolver.reflect(context)
    }

    // Fallback to simple pattern-based reflection
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
   * Step 1: Search for solutions online
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
   * Step 2.5: Try to find and execute an existing tool
   *
   * Searches the dynamic tool registry for a matching tool,
   * then executes it if found and records usage statistics.
   */
  private async tryExistingTool(
    problem: AutonomousProblem,
  ): Promise<{ toolId: string; toolName: string; code: string } | null> {
    const query = problem.errorMessage ?? problem.description
    const language = this.detectLanguage(problem.technology)

    // Search for matching tools
    const results = await DynamicToolRegistry.search(query, {
      limit: 3,
      minScore: this.config.toolMatchThreshold,
      language: this.mapLanguageToToolLanguage(language),
    })

    if (results.length === 0) {
      return null
    }

    // Try the best matching tool
    const bestMatch = results[0]
    const tool = bestMatch.tool

    // Execute the tool
    if (this.sandbox) {
      const startTime = Date.now()
      const result = await this.sandbox.execute({
        language,
        code: tool.code,
        workingDir: problem.workingDir,
        timeoutMs: 30000,
      })

      const durationMs = Date.now() - startTime

      // Record usage regardless of success
      await DynamicToolRegistry.recordUsage(tool.id, result.exitCode === 0, durationMs)

      // Only return if successful
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
   * Step 5: Learn a tool from successful execution
   *
   * Creates a reusable tool from code that successfully solved a problem.
   * Applies quality gates to ensure only meaningful tools are learned.
   */
  private async learnToolFromExecution(
    problem: AutonomousProblem,
    code: string,
    result: SandboxResult,
  ): Promise<ToolTypes.DynamicTool | null> {
    // Quality gates - skip trivial or test code
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

  /**
   * Map sandbox language to tool registry language
   */
  private mapLanguageToToolLanguage(language: "python" | "nodejs" | "shell"): "python" | "nodejs" | "bash" {
    return language === "shell" ? "bash" : language
  }

  /**
   * Fallback code generation using heuristics
   *
   * Used when LLM-based generation is disabled or fails.
   */
  private generateSolutionCodeFallback(problem: AutonomousProblem, webSources: FetchedContent[]): string | null {
    // Extract code examples from web sources
    const codeExamples: string[] = []

    for (const source of webSources) {
      // Simple extraction of code blocks from content
      const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g
      let match: RegExpExecArray | null
      while ((match = codeBlockRegex.exec(source.content)) !== null) {
        const code = match[2].trim()
        if (code.length > 20) {
          codeExamples.push(code)
        }
      }
    }

    // Use first relevant code example, or generate a test stub
    if (codeExamples.length > 0) {
      return codeExamples[0]
    }

    // Generate a simple test/verification script
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

  /**
   * Detect language from technology string
   */
  private detectLanguage(technology?: string): "python" | "nodejs" | "shell" {
    if (!technology) return "nodejs"

    const tech = technology.toLowerCase()

    if (tech.includes("python")) return "python"
    if (tech.includes("bash") || tech.includes("shell") || tech.includes("sh")) return "shell"

    return "nodejs"
  }

  /**
   * Step 4: Sediment the successful solution
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
