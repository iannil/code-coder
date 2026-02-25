/**
 * LLM-Based Problem Solver
 *
 * Provides LLM-enhanced capabilities for the autonomous evolution loop:
 * 1. Code Generation: Generate solution code based on problem description, error, and web context
 * 2. Reflection: Analyze execution errors and suggest fixes like a human programmer
 *
 * This implements the "编程保底" (Programming Fallback) and "自主反思" (Self-Reflection)
 * mechanisms described in goals.md section 3.3.
 *
 * Part of Phase 1: Autonomous Problem-Solving Loop Enhancement
 */

import { Log } from "@/util/log"
import { Provider } from "@/provider/provider"
import { generateText } from "ai"
import type { FetchedContent } from "./web-search"

const log = Log.create({ service: "autonomous.llm-solver" })

// ============================================================================
// Types
// ============================================================================

/** Context for code generation */
export interface CodeGenerationContext {
  /** Problem description */
  problem: string
  /** Error message if applicable */
  errorMessage?: string
  /** Technology/language context */
  technology?: string
  /** Web sources with relevant information */
  webSources?: FetchedContent[]
  /** Previous failed attempts and their errors */
  previousAttempts?: Array<{
    code: string
    error: string
  }>
}

/** Result of code generation */
export interface CodeGenerationResult {
  /** Generated code */
  code: string
  /** Language of the generated code */
  language: "python" | "nodejs" | "shell"
  /** Explanation of the approach */
  explanation: string
  /** Confidence in the solution (0-1) */
  confidence: number
}

/** Context for reflection */
export interface ReflectionContext {
  /** Original problem */
  problem: string
  /** Code that was executed */
  code: string
  /** Exit code from execution */
  exitCode: number
  /** Standard output */
  stdout: string
  /** Standard error */
  stderr: string
  /** Execution duration in ms */
  durationMs: number
  /** Whether execution timed out */
  timedOut: boolean
  /** Technology context */
  technology?: string
  /** Attempt number */
  attemptNumber: number
}

/** Result of reflection */
export interface ReflectionAnalysis {
  /** Whether the execution succeeded */
  success: boolean
  /** Detailed analysis of what happened */
  analysis: string
  /** Root cause of the error (if failed) */
  rootCause?: string
  /** Suggested fix as corrected code */
  suggestedCode?: string
  /** Whether retry is recommended */
  shouldRetry: boolean
  /** Confidence in the analysis (0-1) */
  confidence: number
  /** Specific error category for knowledge extraction */
  errorCategory?: "syntax" | "runtime" | "dependency" | "timeout" | "logic" | "environment" | "unknown"
}

/** Configuration for LLM solver */
export interface LLMSolverConfig {
  /** Temperature for code generation (lower = more deterministic) */
  codeGenTemperature: number
  /** Temperature for reflection (slightly higher for creative analysis) */
  reflectionTemperature: number
  /** Maximum output tokens */
  maxOutputTokens: number
  /** Timeout in ms */
  timeout: number
}

// ============================================================================
// Prompts
// ============================================================================

const CODE_GENERATION_PROMPT = `You are an expert programmer solving a problem by writing executable code.

## Problem
{{problem}}
{{errorSection}}
{{technologySection}}
{{webSourcesSection}}
{{previousAttemptsSection}}

## Requirements
1. Write a COMPLETE, EXECUTABLE script that solves this problem
2. The code must be self-contained and runnable without modification
3. Include proper error handling
4. Print clear output that indicates success or failure
5. Exit with code 0 on success, non-zero on failure

## Output Format
Return ONLY a JSON object (no markdown, no explanation outside JSON):
{
  "language": "python|nodejs|shell",
  "explanation": "brief explanation of your approach",
  "confidence": 0.8,
  "code": "the complete executable code"
}

Important: The code field should contain the raw code, not markdown-formatted code.`

const REFLECTION_PROMPT = `You are an expert programmer analyzing a code execution result.

## Original Problem
{{problem}}

## Code Executed
\`\`\`{{language}}
{{code}}
\`\`\`

## Execution Result
- Exit Code: {{exitCode}}
- Duration: {{durationMs}}ms
- Timed Out: {{timedOut}}

### Standard Output
\`\`\`
{{stdout}}
\`\`\`

### Standard Error
\`\`\`
{{stderr}}
\`\`\`

## Task
Analyze this execution result like an experienced programmer would:
1. Determine if the execution succeeded (exit code 0 AND no errors in stderr AND output looks correct)
2. If failed, identify the ROOT CAUSE of the error
3. If failed, provide CORRECTED code that fixes the issue
4. Assess whether retry is worthwhile

## Output Format
Return ONLY a JSON object (no markdown, no explanation outside JSON):
{
  "success": true|false,
  "analysis": "detailed analysis of what happened",
  "rootCause": "root cause if failed (null if success)",
  "errorCategory": "syntax|runtime|dependency|timeout|logic|environment|unknown",
  "shouldRetry": true|false,
  "confidence": 0.8,
  "suggestedCode": "complete corrected code if shouldRetry is true, null otherwise"
}

Important: The suggestedCode field should contain the raw corrected code, not markdown-formatted.`

// ============================================================================
// LLM Solver
// ============================================================================

const DEFAULT_CONFIG: LLMSolverConfig = {
  codeGenTemperature: 0.3,
  reflectionTemperature: 0.4,
  maxOutputTokens: 4000,
  timeout: 60000,
}

/**
 * LLM-based problem solver for autonomous execution
 */
export class LLMSolver {
  private config: LLMSolverConfig

  constructor(config?: Partial<LLMSolverConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Generate solution code using LLM
   *
   * This implements the "动态编程保底" (Dynamic Programming Fallback) mechanism.
   * When no existing tool or knowledge can solve the problem, the LLM generates
   * a custom script to attempt the solution.
   */
  async generateCode(context: CodeGenerationContext): Promise<CodeGenerationResult | null> {
    try {
      log.info("Generating solution code with LLM", {
        problemPreview: context.problem.slice(0, 100),
        hasError: !!context.errorMessage,
        webSourceCount: context.webSources?.length ?? 0,
        previousAttempts: context.previousAttempts?.length ?? 0,
      })

      const prompt = this.buildCodeGenerationPrompt(context)
      const result = await this.callLLM(prompt, this.config.codeGenTemperature)

      const parsed = this.parseCodeGenerationResponse(result)
      if (!parsed) {
        log.warn("Failed to parse code generation response", {
          responsePreview: result.slice(0, 200),
        })
        return null
      }

      log.info("Code generation completed", {
        language: parsed.language,
        codeLength: parsed.code.length,
        confidence: parsed.confidence,
      })

      return parsed
    } catch (error) {
      log.error("Code generation failed", {
        error: error instanceof Error ? error.message : String(error),
      })
      return null
    }
  }

  /**
   * Reflect on execution result using LLM
   *
   * This implements the "自主反思与无限重试" (Self-Reflection & Infinite Retry) mechanism.
   * The LLM analyzes stderr like a human programmer, identifying root causes and
   * suggesting fixes.
   */
  async reflect(context: ReflectionContext): Promise<ReflectionAnalysis> {
    try {
      log.info("Reflecting on execution with LLM", {
        exitCode: context.exitCode,
        timedOut: context.timedOut,
        stderrLength: context.stderr.length,
        attemptNumber: context.attemptNumber,
      })

      const prompt = this.buildReflectionPrompt(context)
      const result = await this.callLLM(prompt, this.config.reflectionTemperature)

      const parsed = this.parseReflectionResponse(result)
      if (!parsed) {
        log.warn("Failed to parse reflection response, using fallback", {
          responsePreview: result.slice(0, 200),
        })
        return this.fallbackReflection(context)
      }

      log.info("Reflection completed", {
        success: parsed.success,
        shouldRetry: parsed.shouldRetry,
        errorCategory: parsed.errorCategory,
        confidence: parsed.confidence,
      })

      return parsed
    } catch (error) {
      log.error("Reflection failed, using fallback", {
        error: error instanceof Error ? error.message : String(error),
      })
      return this.fallbackReflection(context)
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private buildCodeGenerationPrompt(context: CodeGenerationContext): string {
    const errorSection = context.errorMessage
      ? `\n## Error to Fix\n\`\`\`\n${context.errorMessage.slice(0, 1000)}\n\`\`\``
      : ""

    const technologySection = context.technology
      ? `\n## Technology Context\nUse ${context.technology} if possible.`
      : ""

    const webSourcesSection =
      context.webSources && context.webSources.length > 0
        ? `\n## Reference Information\n${context.webSources
            .slice(0, 3)
            .map((s) => `### ${s.summary}\n${s.content.slice(0, 500)}`)
            .join("\n\n")}`
        : ""

    const previousAttemptsSection =
      context.previousAttempts && context.previousAttempts.length > 0
        ? `\n## Previous Failed Attempts (AVOID THESE MISTAKES)\n${context.previousAttempts
            .slice(-2)
            .map(
              (a, i) =>
                `### Attempt ${i + 1}\nCode:\n\`\`\`\n${a.code.slice(0, 300)}\n\`\`\`\nError:\n\`\`\`\n${a.error.slice(0, 200)}\n\`\`\``,
            )
            .join("\n\n")}`
        : ""

    return CODE_GENERATION_PROMPT.replace("{{problem}}", context.problem.slice(0, 2000))
      .replace("{{errorSection}}", errorSection)
      .replace("{{technologySection}}", technologySection)
      .replace("{{webSourcesSection}}", webSourcesSection)
      .replace("{{previousAttemptsSection}}", previousAttemptsSection)
  }

  private buildReflectionPrompt(context: ReflectionContext): string {
    const language = context.technology?.includes("python")
      ? "python"
      : context.technology?.includes("shell")
        ? "shell"
        : "javascript"

    return REFLECTION_PROMPT.replace("{{problem}}", context.problem.slice(0, 1000))
      .replace("{{language}}", language)
      .replace("{{code}}", context.code.slice(0, 3000))
      .replace("{{exitCode}}", String(context.exitCode))
      .replace("{{durationMs}}", String(context.durationMs))
      .replace("{{timedOut}}", String(context.timedOut))
      .replace("{{stdout}}", context.stdout.slice(0, 2000) || "(empty)")
      .replace("{{stderr}}", context.stderr.slice(0, 2000) || "(empty)")
  }

  private async callLLM(prompt: string, temperature: number): Promise<string> {
    const model = await Provider.defaultModel()
    const languageModel = await Provider.getLanguage(
      await Provider.getModel(model.providerID, model.modelID),
    )

    const result = await generateText({
      model: languageModel,
      prompt,
      maxOutputTokens: this.config.maxOutputTokens,
      temperature,
    })

    return result.text
  }

  private parseCodeGenerationResponse(text: string): CodeGenerationResult | null {
    try {
      const jsonMatch = text.match(/\{[\s\S]*\}/)
      if (!jsonMatch) return null

      const raw = JSON.parse(jsonMatch[0])

      const language = this.normalizeLanguage(raw.language)
      if (!language) return null

      const code = typeof raw.code === "string" ? raw.code.trim() : null
      if (!code || code.length < 10) return null

      return {
        code,
        language,
        explanation: String(raw.explanation ?? ""),
        confidence: Math.max(0, Math.min(1, Number(raw.confidence) || 0.5)),
      }
    } catch {
      return null
    }
  }

  private parseReflectionResponse(text: string): ReflectionAnalysis | null {
    try {
      const jsonMatch = text.match(/\{[\s\S]*\}/)
      if (!jsonMatch) return null

      const raw = JSON.parse(jsonMatch[0])

      const errorCategories = ["syntax", "runtime", "dependency", "timeout", "logic", "environment", "unknown"] as const
      const errorCategory = errorCategories.includes(raw.errorCategory) ? raw.errorCategory : "unknown"

      return {
        success: Boolean(raw.success),
        analysis: String(raw.analysis ?? "Analysis unavailable"),
        rootCause: raw.rootCause ? String(raw.rootCause) : undefined,
        suggestedCode: raw.suggestedCode ? String(raw.suggestedCode).trim() : undefined,
        shouldRetry: Boolean(raw.shouldRetry),
        confidence: Math.max(0, Math.min(1, Number(raw.confidence) || 0.5)),
        errorCategory,
      }
    } catch {
      return null
    }
  }

  private normalizeLanguage(lang: unknown): "python" | "nodejs" | "shell" | null {
    const str = String(lang).toLowerCase()
    if (str.includes("python")) return "python"
    if (str.includes("node") || str.includes("javascript") || str.includes("js")) return "nodejs"
    if (str.includes("shell") || str.includes("bash") || str.includes("sh")) return "shell"
    return null
  }

  private fallbackReflection(context: ReflectionContext): ReflectionAnalysis {
    const success = context.exitCode === 0 && !context.timedOut

    let errorCategory: ReflectionAnalysis["errorCategory"] = "unknown"
    let rootCause: string | undefined

    if (context.timedOut) {
      errorCategory = "timeout"
      rootCause = "Execution timed out - possible infinite loop or slow operation"
    } else if (context.stderr.toLowerCase().includes("syntaxerror")) {
      errorCategory = "syntax"
      rootCause = "Syntax error in code"
    } else if (
      context.stderr.toLowerCase().includes("modulenotfounderror") ||
      context.stderr.toLowerCase().includes("cannot find module")
    ) {
      errorCategory = "dependency"
      rootCause = "Missing dependency or module"
    } else if (context.exitCode !== 0) {
      errorCategory = "runtime"
      rootCause = `Execution failed with exit code ${context.exitCode}`
    }

    return {
      success,
      analysis: success
        ? "Execution completed successfully"
        : `Execution failed: ${context.stderr.slice(0, 200)}`,
      rootCause,
      shouldRetry: !success && context.attemptNumber < 3,
      confidence: 0.3,
      errorCategory,
    }
  }
}

// ============================================================================
// Factory
// ============================================================================

let instance: LLMSolver | undefined

/**
 * Get the LLM solver singleton
 */
export function getLLMSolver(): LLMSolver {
  if (!instance) {
    instance = new LLMSolver()
  }
  return instance
}

/**
 * Create a new LLM solver instance
 */
export function createLLMSolver(config?: Partial<LLMSolverConfig>): LLMSolver {
  return new LLMSolver(config)
}
