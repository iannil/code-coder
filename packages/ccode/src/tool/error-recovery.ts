/**
 * Tool Error Recovery System
 *
 * Provides LLM-powered recovery suggestions for tool execution errors.
 * This module bridges the deterministic error classification (Rust)
 * with intelligent recovery suggestion generation (LLM).
 *
 * @example
 * ```typescript
 * import { ErrorRecovery, classifyError, generateRecoverySuggestion } from './error-recovery'
 *
 * // Classify an error
 * const error = classifyError('grep', 1, 'grep: /nonexistent: No such file or directory')
 *
 * // Generate recovery suggestion
 * const suggestion = await generateRecoverySuggestion(error, { tool: 'grep', args: { path: '/nonexistent' } })
 * console.log(suggestion.suggestion)
 * ```
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "tool.error-recovery" })

// ============================================================================
// Types (matching Rust ClassifiedError)
// ============================================================================

export type ToolErrorType =
  | "validation"
  | "execution"
  | "permission"
  | "timeout"
  | "network"
  | "resource"
  | "unknown"

export interface ClassifiedError {
  errorType: ToolErrorType
  message: string
  rawOutput?: string
  exitCode?: number
  retryable: boolean
  retryDelayMs?: number
  field?: string
  reason?: string
  resource?: string
  elapsedMs?: number
  limitMs?: number
}

export interface ToolContext {
  tool: string
  args: Record<string, unknown>
  sessionId?: string
  previousAttempts?: number
  workingDirectory?: string
}

export interface RecoverySuggestion {
  suggestion: string
  confidence: number
  actions: RecoveryAction[]
  shouldRetry: boolean
  retryWithModifications?: Record<string, unknown>
}

export interface RecoveryAction {
  type: "modify_args" | "change_tool" | "escalate" | "retry" | "skip"
  description: string
  parameters?: Record<string, unknown>
}

// ============================================================================
// Error Classification (TypeScript implementation)
// ============================================================================

const PERMISSION_PATTERNS = [
  /permission denied/i,
  /access denied/i,
  /not permitted/i,
  /operation not allowed/i,
  /EACCES/,
  /EPERM/,
]

const NETWORK_PATTERNS = [
  /connection refused/i,
  /connection reset/i,
  /connection timed out/i,
  /network unreachable/i,
  /host not found/i,
  /ECONNREFUSED/,
  /ECONNRESET/,
  /ETIMEDOUT/,
  /ENETUNREACH/,
]

const RESOURCE_PATTERNS = [
  /out of memory/i,
  /no space left/i,
  /disk quota/i,
  /too many open files/i,
  /resource temporarily unavailable/i,
  /ENOMEM/,
  /ENOSPC/,
  /EMFILE/,
  /ENFILE/,
]

/**
 * Classify a tool execution error
 */
export function classifyError(
  tool: string,
  exitCode: number,
  stderr: string
): ClassifiedError {
  // Check for permission errors
  if (PERMISSION_PATTERNS.some((p) => p.test(stderr))) {
    return {
      errorType: "permission",
      message: "Permission denied",
      rawOutput: stderr,
      exitCode,
      retryable: false,
      reason: extractFirstLine(stderr),
      resource: extractResource(stderr),
    }
  }

  // Check for network errors
  if (NETWORK_PATTERNS.some((p) => p.test(stderr))) {
    return {
      errorType: "network",
      message: "Network error",
      rawOutput: stderr,
      exitCode,
      retryable: true,
      retryDelayMs: 2000,
      reason: extractFirstLine(stderr),
    }
  }

  // Check for resource errors
  if (RESOURCE_PATTERNS.some((p) => p.test(stderr))) {
    return {
      errorType: "resource",
      message: "Resource exhausted",
      rawOutput: stderr,
      exitCode,
      retryable: true,
      retryDelayMs: 10000,
      reason: extractFirstLine(stderr),
      resource: extractResource(stderr),
    }
  }

  // Check for timeout (exit code 124 is standard for timeout command)
  if (exitCode === 124) {
    return {
      errorType: "timeout",
      message: "Operation timed out",
      rawOutput: stderr,
      exitCode,
      retryable: true,
      retryDelayMs: 5000,
    }
  }

  // Default: execution error
  return {
    errorType: "execution",
    message: `Execution failed with exit code ${exitCode}`,
    rawOutput: stderr,
    exitCode,
    retryable: isExecutionRetryable(exitCode, stderr),
    retryDelayMs: isExecutionRetryable(exitCode, stderr) ? 1000 : undefined,
  }
}

function extractFirstLine(stderr: string): string {
  const line = stderr
    .split("\n")
    .find((l) => l.trim().length > 0)
  return line?.trim() ?? "Unknown error"
}

function extractResource(stderr: string): string | undefined {
  // Look for quoted paths
  const quotedMatch = stderr.match(/'([^']+)'/)
  if (quotedMatch) {
    return quotedMatch[1]
  }

  // Look for paths before colons
  for (const line of stderr.split("\n")) {
    const colonIdx = line.indexOf(":")
    if (colonIdx > 0) {
      const potential = line.slice(0, colonIdx).trim()
      if (potential.startsWith("/") || potential.startsWith(".")) {
        return potential
      }
    }
  }

  return undefined
}

function isExecutionRetryable(exitCode: number, stderr: string): boolean {
  const retryablePatterns = [
    /connection refused/i,
    /connection reset/i,
    /connection timed out/i,
    /temporarily unavailable/i,
    /resource busy/i,
    /too many open files/i,
    /try again/i,
    /EAGAIN/,
    /EBUSY/,
    /ETIMEDOUT/,
  ]

  // Exit code 137 = OOM killed, 124 = timeout
  if (exitCode === 137 || exitCode === 124) {
    return true
  }

  return retryablePatterns.some((p) => p.test(stderr))
}

// ============================================================================
// Recovery Suggestion Generation (LLM-powered)
// ============================================================================

/**
 * Generate a recovery suggestion for a classified error.
 * This function uses heuristics and patterns rather than calling LLM directly,
 * making it fast and predictable. For complex cases, the agent loop will
 * use the error context to inform its next action.
 */
export function generateRecoverySuggestion(
  error: ClassifiedError,
  context: ToolContext
): RecoverySuggestion {
  log.debug("Generating recovery suggestion", {
    errorType: error.errorType,
    tool: context.tool,
  })

  switch (error.errorType) {
    case "validation":
      return generateValidationRecovery(error, context)
    case "permission":
      return generatePermissionRecovery(error, context)
    case "timeout":
      return generateTimeoutRecovery(error, context)
    case "network":
      return generateNetworkRecovery(error, context)
    case "resource":
      return generateResourceRecovery(error, context)
    case "execution":
      return generateExecutionRecovery(error, context)
    default:
      return generateGenericRecovery(error, context)
  }
}

function generateValidationRecovery(
  error: ClassifiedError,
  context: ToolContext
): RecoverySuggestion {
  const actions: RecoveryAction[] = []

  if (error.field) {
    actions.push({
      type: "modify_args",
      description: `Fix the '${error.field}' parameter`,
      parameters: { field: error.field, reason: error.reason },
    })
  }

  return {
    suggestion: `Validation error: ${error.reason ?? error.message}. Please correct the input parameters.`,
    confidence: 0.9,
    actions,
    shouldRetry: false,
  }
}

function generatePermissionRecovery(
  error: ClassifiedError,
  context: ToolContext
): RecoverySuggestion {
  const actions: RecoveryAction[] = [
    {
      type: "escalate",
      description: "Request elevated permissions",
      parameters: { resource: error.resource },
    },
  ]

  // Suggest alternative path if applicable
  if (error.resource && context.tool === "read") {
    actions.push({
      type: "change_tool",
      description: "Try reading a different file or use sudo",
    })
  }

  return {
    suggestion: `Permission denied for ${error.resource ?? "resource"}. The operation requires elevated privileges or the resource is restricted.`,
    confidence: 0.85,
    actions,
    shouldRetry: false,
  }
}

function generateTimeoutRecovery(
  error: ClassifiedError,
  context: ToolContext
): RecoverySuggestion {
  const currentTimeout = context.args.timeout as number | undefined
  const newTimeout = currentTimeout ? currentTimeout * 2 : 60000

  return {
    suggestion: `Operation timed out after ${error.elapsedMs ?? "unknown"}ms. Consider increasing the timeout or simplifying the operation.`,
    confidence: 0.7,
    actions: [
      {
        type: "retry",
        description: "Retry with increased timeout",
        parameters: { timeout: newTimeout },
      },
      {
        type: "modify_args",
        description: "Simplify the operation (e.g., reduce scope)",
      },
    ],
    shouldRetry: true,
    retryWithModifications: { timeout: newTimeout },
  }
}

function generateNetworkRecovery(
  error: ClassifiedError,
  context: ToolContext
): RecoverySuggestion {
  const previousAttempts = context.previousAttempts ?? 0

  return {
    suggestion: `Network error: ${error.reason ?? error.message}. This may be a temporary connectivity issue.`,
    confidence: 0.75,
    actions: [
      {
        type: "retry",
        description: "Retry after a short delay",
        parameters: { delay: error.retryDelayMs ?? 2000 },
      },
    ],
    shouldRetry: previousAttempts < 3,
  }
}

function generateResourceRecovery(
  error: ClassifiedError,
  context: ToolContext
): RecoverySuggestion {
  return {
    suggestion: `Resource exhausted: ${error.reason ?? error.message}. The system may need time to free up resources.`,
    confidence: 0.6,
    actions: [
      {
        type: "retry",
        description: "Retry after resources are freed",
        parameters: { delay: error.retryDelayMs ?? 10000 },
      },
      {
        type: "modify_args",
        description: "Reduce resource requirements",
      },
    ],
    shouldRetry: true,
  }
}

function generateExecutionRecovery(
  error: ClassifiedError,
  context: ToolContext
): RecoverySuggestion {
  const actions: RecoveryAction[] = []

  // Analyze stderr for common patterns
  const stderr = error.rawOutput ?? ""

  if (/command not found/i.test(stderr) || /not recognized/i.test(stderr)) {
    actions.push({
      type: "change_tool",
      description: "The required command is not installed",
    })
  } else if (/no such file/i.test(stderr) || /does not exist/i.test(stderr)) {
    actions.push({
      type: "modify_args",
      description: "Check file path - the file may not exist",
    })
  } else if (/syntax error/i.test(stderr)) {
    actions.push({
      type: "modify_args",
      description: "Fix syntax error in the command or arguments",
    })
  } else {
    actions.push({
      type: "skip",
      description: "This operation failed - consider an alternative approach",
    })
  }

  return {
    suggestion: `Execution failed (exit code ${error.exitCode}): ${extractFirstLine(stderr)}`,
    confidence: 0.5,
    actions,
    shouldRetry: error.retryable,
  }
}

function generateGenericRecovery(
  error: ClassifiedError,
  context: ToolContext
): RecoverySuggestion {
  return {
    suggestion: `An error occurred: ${error.message}`,
    confidence: 0.3,
    actions: [
      {
        type: "skip",
        description: "Skip this operation and try an alternative approach",
      },
    ],
    shouldRetry: error.retryable,
  }
}

// ============================================================================
// Error Feedback Integration
// ============================================================================

export interface ErrorFeedback {
  tool: string
  error: ClassifiedError
  suggestion: RecoverySuggestion
  context: ToolContext
  timestamp: number
}

/**
 * Create error feedback for injection into agent context
 */
export function createErrorFeedback(
  tool: string,
  exitCode: number,
  stderr: string,
  context: Omit<ToolContext, "tool">
): ErrorFeedback {
  const fullContext: ToolContext = { ...context, tool }
  const error = classifyError(tool, exitCode, stderr)
  const suggestion = generateRecoverySuggestion(error, fullContext)

  return {
    tool,
    error,
    suggestion,
    context: fullContext,
    timestamp: Date.now(),
  }
}

/**
 * Format error feedback as a message for the agent
 */
export function formatErrorForAgent(feedback: ErrorFeedback): string {
  const lines: string[] = [
    `## Tool Error: ${feedback.tool}`,
    "",
    `**Type**: ${feedback.error.errorType}`,
    `**Retryable**: ${feedback.error.retryable ? "Yes" : "No"}`,
    "",
    `**Error**: ${feedback.error.message}`,
  ]

  if (feedback.error.rawOutput) {
    const truncated =
      feedback.error.rawOutput.length > 500
        ? feedback.error.rawOutput.slice(0, 500) + "..."
        : feedback.error.rawOutput
    lines.push("", "**Output**:", "```", truncated, "```")
  }

  lines.push("", `**Suggestion** (confidence: ${Math.round(feedback.suggestion.confidence * 100)}%):`)
  lines.push(feedback.suggestion.suggestion)

  if (feedback.suggestion.actions.length > 0) {
    lines.push("", "**Possible Actions**:")
    for (const action of feedback.suggestion.actions) {
      lines.push(`- ${action.description}`)
    }
  }

  return lines.join("\n")
}

// ============================================================================
// Exports
// ============================================================================

export const ErrorRecovery = {
  classifyError,
  generateRecoverySuggestion,
  createErrorFeedback,
  formatErrorForAgent,
}
