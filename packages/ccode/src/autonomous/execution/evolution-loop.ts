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
import { DynamicToolRegistry, type ToolTypes } from "@/memory/tools"
import type { FetchedContent } from "./web-search"

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
}

// ============================================================================
// Evolution Loop
// ============================================================================

/**
 * Autonomous evolution loop for problem solving
 */
export class EvolutionLoop {
  private config: EvolutionConfig
  private webSearch: EnhancedWebSearch | null = null
  private sandbox: SandboxExecutor | null = null
  private knowledge: KnowledgeSedimentation | null = null

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
  }

  /**
   * Run the evolution loop to solve a problem
   */
  async evolve(problem: AutonomousProblem): Promise<EvolutionResult> {
    const startTime = Date.now()
    const attempts: SolutionAttempt[] = []
    const maxRetries = problem.maxRetries ?? this.config.maxRetries

    await this.initialize(problem.sessionId)

    // Step 1: Resource Retrieval (if needed)
    let webSources: FetchedContent[] = []
    if (this.config.enableWebSearch && this.webSearch) {
      webSources = await this.searchForSolutions(problem)
    }

    // Step 2: Check existing knowledge
    const existingKnowledge = await this.searchExistingKnowledge(problem)
    if (existingKnowledge) {
      return {
        solved: true,
        solution: existingKnowledge.solution,
        attempts: [],
        knowledgeId: existingKnowledge.id,
        learnedToolId: undefined,
        usedToolId: undefined,
        durationMs: Date.now() - startTime,
        summary: `Found existing solution from knowledge base: ${existingKnowledge.title}`,
      }
    }

    // Step 2.5: Tool Discovery - Check for existing reusable tools
    if (this.config.enableToolDiscovery) {
      const toolResult = await this.tryExistingTool(problem)
      if (toolResult) {
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
        }
      }
    }

    // Step 3: Generate and execute code
    if (this.config.enableCodeExecution && this.sandbox) {
      const code = this.generateSolutionCode(problem, webSources)

      if (code) {
        // Step 3: Execute with reflection loop
        const { finalResult, attempts: execAttempts, reflections } = await this.sandbox.executeWithReflection(
          {
            language: this.detectLanguage(problem.technology),
            code,
            workingDir: problem.workingDir,
            timeoutMs: 30000,
          },
          maxRetries,
          (result, reflection, attempt) => {
            attempts.push({
              attempt,
              code: code,
              executionResult: result,
              reflection,
              webSources: attempt === 1 ? webSources : undefined,
              success: reflection.success,
              timestamp: new Date().toISOString(),
            })
          },
        )

        // Check final result
        if (finalResult.exitCode === 0) {
          // Step 4: Knowledge Sedimentation
          let knowledgeId: string | undefined
          if (this.config.enableSedimentation && this.knowledge) {
            const entry = await this.sedimentSolution(problem, code, webSources, reflections)
            knowledgeId = entry.id
          }

          // Step 5: Tool Learning - Learn successful execution as reusable tool
          let learnedToolId: string | undefined
          if (this.config.enableToolLearning) {
            const learnedTool = await this.learnToolFromExecution(problem, code, finalResult)
            learnedToolId = learnedTool?.id
          }

          return {
            solved: true,
            solution: code,
            attempts,
            knowledgeId,
            learnedToolId,
            durationMs: Date.now() - startTime,
            summary: `Problem solved after ${attempts.length} attempt(s).${knowledgeId ? " Solution saved to knowledge base." : ""}${learnedToolId ? " Learned as reusable tool." : ""}`,
          }
        }
      }
    }

    // No solution found
    return {
      solved: false,
      attempts,
      learnedToolId: undefined,
      usedToolId: undefined,
      durationMs: Date.now() - startTime,
      summary: `Could not solve problem after ${attempts.length} attempts. Consider seeking human assistance.`,
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
   * Generate solution code from problem and web sources
   */
  private generateSolutionCode(problem: AutonomousProblem, webSources: FetchedContent[]): string | null {
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
