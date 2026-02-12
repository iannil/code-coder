import { Log } from "@/util/log"
import { Agent } from "@/agent/agent"
import { Instance } from "@/project/instance"
import { Provider } from "@/provider/provider"
import { generateObject, streamObject, type ModelMessage } from "ai"
import { SystemPrompt } from "@/session/system"
import { Auth } from "@/auth"
import { ProviderTransform } from "@/provider/transform"
import { Config } from "@/config/config"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import z from "zod"

const log = Log.create({ service: "autonomous.execution.agent-invoker" })

/**
 * Available agents for Autonomous Mode to invoke
 */
export type InvocableAgent =
  | "code-reviewer"
  | "security-reviewer"
  | "tdd-guide"
  | "architect"
  | "explore"
  | "general"

/**
 * Agent invocation request
 */
export interface AgentInvocationRequest {
  agent: InvocableAgent
  task: string
  context?: {
    sessionId?: string
    files?: string[]
    previousOutput?: string
    metadata?: Record<string, unknown>
  }
  options?: {
    temperature?: number
    maxTokens?: number
    timeout?: number
  }
}

/**
 * Agent invocation result
 */
export interface AgentInvocationResult {
  success: boolean
  agent: InvocableAgent
  output: string
  duration: number
  tokensUsed?: number
  error?: string
  metadata?: Record<string, unknown>
}

/**
 * Code review result schema
 */
const CodeReviewSchema = z.object({
  summary: z.string(),
  issues: z.array(
    z.object({
      severity: z.enum(["critical", "high", "medium", "low", "info"]),
      file: z.string(),
      line: z.number().optional(),
      message: z.string(),
      suggestion: z.string().optional(),
    }),
  ),
  overallScore: z.number().min(0).max(100),
  shouldBlock: z.boolean(),
})

/**
 * Security review result schema
 */
const SecurityReviewSchema = z.object({
  summary: z.string(),
  vulnerabilities: z.array(
    z.object({
      severity: z.enum(["critical", "high", "medium", "low"]),
      category: z.string(),
      description: z.string(),
      recommendation: z.string(),
    }),
  ),
  overallRisk: z.enum(["critical", "high", "medium", "low", "minimal"]),
  shouldBlock: z.boolean(),
})

/**
 * TDD guidance result schema
 */
const TDDGuidanceSchema = z.object({
  phase: z.enum(["red", "green", "refactor"]),
  testFilePath: z.string().optional(),
  implementationFilePath: z.string().optional(),
  testCode: z.string().optional(),
  implementationCode: z.string().optional(),
  nextSteps: z.array(z.string()),
  reasoning: z.string(),
})

/**
 * Agent Invoker for Autonomous Mode
 *
 * Provides programmatic invocation of specialized agents
 * for autonomous execution workflows
 */
export namespace AgentInvoker {
  /**
   * Invoke an agent with a task
   */
  export async function invoke(request: AgentInvocationRequest): Promise<AgentInvocationResult> {
    const startTime = Date.now()

    log.info("Invoking agent", {
      agent: request.agent,
      task: request.task.slice(0, 100),
    })

    try {
      // Get agent configuration
      const agentInfo = await Agent.get(request.agent)
      if (!agentInfo) {
        throw new Error(`Agent "${request.agent}" not found`)
      }

      // Get model
      const defaultModel = agentInfo.model ?? (await Provider.defaultModel())
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      // Build system prompt
      const system = SystemPrompt.header(defaultModel.providerID)
      if (agentInfo.prompt) {
        system.push(agentInfo.prompt)
      }

      // Build user prompt with context
      let userPrompt = request.task
      if (request.context) {
        const contextParts: string[] = []
        if (request.context.files && request.context.files.length > 0) {
          contextParts.push(`Files to consider: ${request.context.files.join(", ")}`)
        }
        if (request.context.previousOutput) {
          contextParts.push(`Previous output:\n${request.context.previousOutput}`)
        }
        if (contextParts.length > 0) {
          userPrompt = `${contextParts.join("\n\n")}\n\nTask: ${request.task}`
        }
      }

      // Select schema based on agent type
      let responseSchema: z.ZodTypeAny = z.object({
        response: z.string(),
      })

      if (request.agent === "code-reviewer") {
        responseSchema = CodeReviewSchema
      } else if (request.agent === "security-reviewer") {
        responseSchema = SecurityReviewSchema
      } else if (request.agent === "tdd-guide") {
        responseSchema = TDDGuidanceSchema
      }

      const cfg = await Config.get()

      // Build telemetry metadata without undefined values
      const telemetryMetadata: Record<string, string> = {
        userId: cfg.username ?? "unknown",
      }
      if (request.context?.sessionId) {
        telemetryMetadata.sessionId = request.context.sessionId
      }

      // Generate response
      const params = {
        experimental_telemetry: {
          isEnabled: cfg.experimental?.openTelemetry,
          metadata: telemetryMetadata,
        },
        temperature: request.options?.temperature ?? agentInfo.temperature ?? 0.6,
        messages: [
          ...system.map(
            (item): ModelMessage => ({
              role: "system",
              content: item,
            }),
          ),
          {
            role: "user",
            content: userPrompt,
          },
        ],
        model: language,
        schema: responseSchema,
      } satisfies Parameters<typeof generateObject>[0]

      let result: z.infer<typeof responseSchema>

      if (defaultModel.providerID === "openai" && (await Auth.get(defaultModel.providerID))?.type === "oauth") {
        const streamResult = streamObject({
          ...params,
          providerOptions: ProviderTransform.providerOptions(model, {
            instructions: SystemPrompt.instructions(),
            store: false,
          }),
          onError: (error) => {
            log.error("Agent invocation error", { error })
          },
        })

        // Collect full stream
        for await (const part of streamResult.fullStream) {
          if (part.type === "error") throw part.error
        }

        result = streamResult.object
      } else {
        const generateResult = await generateObject(params)
        result = generateResult.object
      }

      const duration = Date.now() - startTime

      // Format output based on result type
      let output: string
      let metadata: Record<string, unknown> = {}

      if (request.agent === "code-reviewer") {
        const review = result as z.infer<typeof CodeReviewSchema>
        output = formatCodeReview(review)
        metadata = {
          issuesCount: review.issues.length,
          score: review.overallScore,
          shouldBlock: review.shouldBlock,
        }
      } else if (request.agent === "security-reviewer") {
        const review = result as z.infer<typeof SecurityReviewSchema>
        output = formatSecurityReview(review)
        metadata = {
          vulnerabilitiesCount: review.vulnerabilities.length,
          risk: review.overallRisk,
          shouldBlock: review.shouldBlock,
        }
      } else if (request.agent === "tdd-guide") {
        const guidance = result as z.infer<typeof TDDGuidanceSchema>
        output = formatTDDGuidance(guidance)
        metadata = {
          phase: guidance.phase,
          testFilePath: guidance.testFilePath ?? null,
          implementationFilePath: guidance.implementationFilePath ?? null,
        }
      } else {
        // Generic response
        const hasResponse = typeof result === "object" && result !== null && "response" in result
        output = hasResponse
          ? (result as { response: string }).response ?? JSON.stringify(result)
          : JSON.stringify(result ?? {})
      }

      log.info("Agent invocation completed", {
        agent: request.agent,
        duration,
        success: true,
      })

      // Publish event
      await Bus.publish(AutonomousEvent.AgentInvoked, {
        sessionId: request.context?.sessionId ?? "unknown",
        agentName: request.agent,
        task: request.task.slice(0, 200),
        success: true,
        duration,
      })

      return {
        success: true,
        agent: request.agent,
        output,
        duration,
        metadata,
      }
    } catch (error) {
      const duration = Date.now() - startTime
      const errorMessage = error instanceof Error ? error.message : String(error)

      log.error("Agent invocation failed", {
        agent: request.agent,
        error: errorMessage,
        duration,
      })

      // Publish event
      await Bus.publish(AutonomousEvent.AgentInvoked, {
        sessionId: request.context?.sessionId ?? "unknown",
        agentName: request.agent,
        task: request.task.slice(0, 200),
        success: false,
        duration,
        error: errorMessage,
      })

      return {
        success: false,
        agent: request.agent,
        output: "",
        duration,
        error: errorMessage,
      }
    }
  }

  /**
   * Invoke code-reviewer agent
   */
  export async function codeReview(files: string[], context?: {
    sessionId?: string
    focus?: string[]
  }): Promise<AgentInvocationResult> {
    const task = `Review the following files for code quality, bugs, and improvements:
${files.join("\n")}${
  context?.focus
    ? `
\n\nFocus areas: ${context.focus.join(", ")}`
    : ""
}`
    return invoke({
      agent: "code-reviewer",
      task,
      context: {
        sessionId: context?.sessionId,
        files,
      },
    })
  }

  /**
   * Invoke security-reviewer agent
   */
  export async function securityReview(
    files: string[],
    context?: { sessionId?: string; threatModel?: boolean },
  ): Promise<AgentInvocationResult> {
    const task = `Analyze the following files for security vulnerabilities:
${files.join("\n")}
${
  context?.threatModel
    ? "Include threat modeling considerations."
    : ""
}`
    return invoke({
      agent: "security-reviewer",
      task,
      context: {
        sessionId: context?.sessionId,
        files,
      },
    })
  }

  /**
   * Invoke tdd-guide agent for RED phase (write failing test)
   */
  export async function tddRed(requirement: string, context?: {
    sessionId?: string
    existingTests?: string[]
  }): Promise<AgentInvocationResult> {
    const task = `TDD RED Phase: Write a failing test for this requirement:
"${requirement}"
${
  context?.existingTests && context.existingTests.length > 0
    ? `\n\nExisting test files to reference:\n${context.existingTests.join("\n")}`
    : ""
}

Generate a test file with appropriate test cases that will fail before implementation.`
    return invoke({
      agent: "tdd-guide",
      task,
      context: {
        sessionId: context?.sessionId,
      },
      options: {
        temperature: 0.5,
      },
    })
  }

  /**
   * Invoke tdd-guide agent for GREEN phase (make test pass)
   */
  export async function tddGreen(
    testFile: string,
    testError: string,
    context?: { sessionId?: string },
  ): Promise<AgentInvocationResult> {
    const task = `TDD GREEN Phase: Implement the minimal code to make this test pass:
Test file: ${testFile}

Test error:
${testError}

Write only the minimal implementation needed - no refactoring yet.`
    return invoke({
      agent: "tdd-guide",
      task,
      context: {
        sessionId: context?.sessionId,
        files: [testFile],
      },
      options: {
        temperature: 0.4,
      },
    })
  }

  /**
   * Invoke tdd-guide agent for REFACTOR phase
   */
  export async function tddRefactor(files: string[], context?: {
    sessionId?: string
    focus?: string[]
  }): Promise<AgentInvocationResult> {
    const task = `TDD REFACTOR Phase: Suggest refactoring for these files while ensuring tests continue to pass:
${files.join("\n")}
${
  context?.focus && context.focus.length > 0
    ? `\n\nFocus areas: ${context.focus.join(", ")}`
    : ""
}`
    return invoke({
      agent: "tdd-guide",
      task,
      context: {
        sessionId: context?.sessionId,
        files,
      },
      options: {
        temperature: 0.6,
      },
    })
  }

  /**
   * Invoke architect agent for design guidance
   */
  export async function architect(
    requirement: string,
    context?: {
      sessionId?: string
      existingFiles?: string[]
    },
  ): Promise<AgentInvocationResult> {
    const task = `Design the architecture for this requirement:
"${requirement}"
${
  context?.existingFiles && context.existingFiles.length > 0
    ? `\n\nExisting files to consider:\n${context.existingFiles.join("\n")}`
    : ""
}

Provide: system design, interfaces, patterns to use, and file structure.`
    return invoke({
      agent: "architect",
      task,
      context: {
        sessionId: context?.sessionId,
        files: context?.existingFiles,
      },
    })
  }

  /**
   * Invoke explore agent for codebase exploration
   */
  export async function explore(
    query: string,
    thoroughness: "quick" | "medium" | "very thorough" = "medium",
    context?: { sessionId?: string },
  ): Promise<AgentInvocationResult> {
    const task = `Explore the codebase to answer this query:
"${query}"

Thoroughness level: ${thoroughness}

Search for relevant files, code patterns, and provide a comprehensive answer.`
    return invoke({
      agent: "explore",
      task,
      context: {
        sessionId: context?.sessionId,
      },
    })
  }

  /**
   * Format code review result
   */
  function formatCodeReview(review: z.infer<typeof CodeReviewSchema>): string {
    const lines: string[] = []

    lines.push(`# Code Review Summary`)
    lines.push(review.summary)
    lines.push("")

    lines.push(`**Overall Score:** ${review.overallScore}/100`)
    lines.push("")

    if (review.issues.length > 0) {
      lines.push(`## Issues (${review.issues.length})`)
      for (const issue of review.issues) {
        lines.push(`\n### [${issue.severity.toUpperCase()}] ${issue.file}${issue.line ? `:${issue.line}` : ""}`)
        lines.push(issue.message)
        if (issue.suggestion) {
          lines.push(`**Suggestion:** ${issue.suggestion}`)
        }
      }
      lines.push("")
    }

    if (review.shouldBlock) {
      lines.push("**⚠️ This change should be blocked until critical issues are resolved.**")
    }

    return lines.join("\n")
  }

  /**
   * Format security review result
   */
  function formatSecurityReview(review: z.infer<typeof SecurityReviewSchema>): string {
    const lines: string[] = []

    lines.push(`# Security Review Summary`)
    lines.push(review.summary)
    lines.push("")

    lines.push(`**Overall Risk:** ${review.overallRisk.toUpperCase()}`)
    lines.push("")

    if (review.vulnerabilities.length > 0) {
      lines.push(`## Vulnerabilities (${review.vulnerabilities.length})`)
      for (const vuln of review.vulnerabilities) {
        lines.push(`\n### [${vuln.severity.toUpperCase()}] ${vuln.category}`)
        lines.push(vuln.description)
        lines.push(`**Recommendation:** ${vuln.recommendation}`)
      }
      lines.push("")
    }

    if (review.shouldBlock) {
      lines.push("**⚠️ This change should be blocked until security issues are resolved.**")
    }

    return lines.join("\n")
  }

  /**
   * Format TDD guidance result
   */
  function formatTDDGuidance(guidance: z.infer<typeof TDDGuidanceSchema>): string {
    const lines: string[] = []

    lines.push(`# TDD Guidance - ${guidance.phase.toUpperCase()} Phase`)
    lines.push("")
    lines.push(guidance.reasoning)
    lines.push("")

    if (guidance.testFilePath) {
      lines.push(`**Test File:** ${guidance.testFilePath}`)
    }
    if (guidance.implementationFilePath) {
      lines.push(`**Implementation File:** ${guidance.implementationFilePath}`)
    }
    lines.push("")

    if (guidance.testCode) {
      lines.push("## Test Code")
      lines.push("```typescript")
      lines.push(guidance.testCode)
      lines.push("```")
      lines.push("")
    }

    if (guidance.implementationCode) {
      lines.push("## Implementation Code")
      lines.push("```typescript")
      lines.push(guidance.implementationCode)
      lines.push("```")
      lines.push("")
    }

    if (guidance.nextSteps.length > 0) {
      lines.push("## Next Steps")
      for (const step of guidance.nextSteps) {
        lines.push(`- ${step}`)
      }
    }

    return lines.join("\n")
  }
}
