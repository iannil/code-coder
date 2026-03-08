/**
 * Tool Macro Executor
 *
 * Executes tool macros by orchestrating step execution,
 * resolving references, and handling errors.
 *
 * ## Design Principle
 *
 * Execution involves both **deterministic** operations (reference
 * resolution, step sequencing) and **uncertain** decisions
 * (error recovery, conditional evaluation with complex expressions).
 * The executor handles the deterministic parts; complex cases can
 * be escalated to LLM reasoning.
 */

import { Log } from "@/util/log"
import type {
  ToolMacro,
  MacroStep,
  MacroParameter,
  MacroReference,
  MacroResult,
  StepResult,
  ValidationResult,
} from "./definition"
import { validateMacro, parseTemplateReferences } from "./definition"

const log = Log.create({ service: "tool.macro.executor" })

// ============================================================================
// Types
// ============================================================================

/**
 * Context available during macro execution
 */
export interface MacroExecutionContext {
  sessionId: string
  workingDirectory: string
  agent: string
  timestamp: number
  abort?: AbortSignal
}

/**
 * Tool executor interface - adapts to actual tool system
 */
export interface ToolExecutor {
  execute(
    tool: string,
    inputs: Record<string, unknown>,
    context: MacroExecutionContext
  ): Promise<{
    output: string
    metadata: Record<string, unknown>
  }>
}

/**
 * Configuration for the macro executor
 */
export interface MacroExecutorConfig {
  /** Default timeout for steps in ms */
  defaultTimeoutMs: number
  /** Maximum total execution time in ms */
  maxTotalTimeMs: number
  /** Whether to run in dry-run mode (no actual execution) */
  dryRun: boolean
  /** Custom tool executor */
  toolExecutor?: ToolExecutor
}

const DEFAULT_CONFIG: MacroExecutorConfig = {
  defaultTimeoutMs: 30000,
  maxTotalTimeMs: 300000,
  dryRun: false,
}

// ============================================================================
// Macro Executor Class
// ============================================================================

/**
 * Executes tool macros with reference resolution and error handling
 */
export class MacroExecutor {
  private config: MacroExecutorConfig
  private toolExecutor: ToolExecutor

  constructor(config: Partial<MacroExecutorConfig> = {}, toolExecutor?: ToolExecutor) {
    this.config = { ...DEFAULT_CONFIG, ...config }
    this.toolExecutor = toolExecutor || this.createDefaultExecutor()
  }

  /**
   * Validate a macro definition
   */
  validate(macro: ToolMacro): ValidationResult {
    return validateMacro(macro)
  }

  /**
   * Execute a macro with given parameters
   */
  async execute(
    macro: ToolMacro,
    params: Record<string, unknown>,
    context: MacroExecutionContext
  ): Promise<MacroResult> {
    const startTime = Date.now()
    const stepResults: StepResult[] = []
    const outputs: Record<string, unknown> = {}
    let lastError: string | undefined

    log.info("Executing macro", { macroId: macro.id, params: Object.keys(params) })

    // Validate macro first
    const validation = this.validate(macro)
    if (!validation.valid) {
      return {
        macroId: macro.id,
        status: "failed",
        stepResults: [],
        outputs: {},
        totalDurationMs: Date.now() - startTime,
        error: `Invalid macro: ${validation.errors.map((e) => e.message).join(", ")}`,
      }
    }

    // Validate required parameters
    const paramError = this.validateParameters(macro.parameters, params)
    if (paramError) {
      return {
        macroId: macro.id,
        status: "failed",
        stepResults: [],
        outputs: {},
        totalDurationMs: Date.now() - startTime,
        error: paramError,
      }
    }

    // Execute steps
    for (let i = 0; i < macro.steps.length; i++) {
      const step = macro.steps[i]

      // Check for abort
      if (context.abort?.aborted) {
        lastError = "Execution aborted"
        break
      }

      // Check total timeout
      if (Date.now() - startTime > this.config.maxTotalTimeMs) {
        lastError = `Total execution time exceeded ${this.config.maxTotalTimeMs}ms`
        break
      }

      // Execute step
      const stepResult = await this.executeStep(
        step,
        i,
        params,
        stepResults,
        outputs,
        context
      )

      stepResults.push(stepResult)

      // Handle step failure
      if (stepResult.status === "failed") {
        const errorHandling = step.errorHandling || { onError: "fail" }

        if (errorHandling.onError === "fail") {
          lastError = stepResult.error
          break
        }
        // For "continue" and "skip", we keep going
      }

      // Capture outputs
      if (step.outputs && stepResult.output) {
        for (const [outputName, path] of Object.entries(step.outputs)) {
          outputs[outputName] = this.extractPath(stepResult.output, path)
        }
      }
    }

    const totalDurationMs = Date.now() - startTime
    const failedSteps = stepResults.filter((r) => r.status === "failed")
    const status =
      lastError || failedSteps.length > 0
        ? failedSteps.length < stepResults.length
          ? "partial"
          : "failed"
        : "success"

    log.info("Macro execution complete", {
      macroId: macro.id,
      status,
      totalDurationMs,
      stepsExecuted: stepResults.length,
    })

    return {
      macroId: macro.id,
      status,
      stepResults,
      outputs,
      totalDurationMs,
      error: lastError,
    }
  }

  /**
   * Execute a single step
   */
  private async executeStep(
    step: MacroStep,
    index: number,
    params: Record<string, unknown>,
    previousResults: StepResult[],
    outputs: Record<string, unknown>,
    context: MacroExecutionContext
  ): Promise<StepResult> {
    const startTime = Date.now()

    log.debug("Executing step", { index, tool: step.tool })

    // Check condition
    if (step.condition) {
      const shouldExecute = this.evaluateCondition(
        step.condition.expression,
        params,
        previousResults,
        outputs,
        context
      )

      if (step.condition.negate ? shouldExecute : !shouldExecute) {
        return {
          stepIndex: index,
          stepId: step.id,
          tool: step.tool,
          status: "skipped",
          durationMs: Date.now() - startTime,
          retryCount: 0,
        }
      }
    }

    // Resolve inputs
    const resolvedInputs = this.resolveInputs(
      step.inputs,
      params,
      previousResults,
      outputs,
      context
    )

    // Execute with retry logic - apply defaults to errorHandling
    const {
      onError = "fail",
      maxRetries = 3,
      retryDelayMs = 1000,
    } = step.errorHandling || {}
    let retryCount = 0
    let lastError: string | undefined

    while (retryCount <= (onError === "retry" ? maxRetries : 0)) {
      try {
        if (this.config.dryRun) {
          // Dry run - just return success without executing
          return {
            stepIndex: index,
            stepId: step.id,
            tool: step.tool,
            status: "success",
            output: { dryRun: true, inputs: resolvedInputs },
            durationMs: Date.now() - startTime,
            retryCount,
          }
        }

        const result = await this.executeWithTimeout(
          () => this.toolExecutor.execute(step.tool, resolvedInputs, context),
          step.timeoutMs || this.config.defaultTimeoutMs
        )

        return {
          stepIndex: index,
          stepId: step.id,
          tool: step.tool,
          status: retryCount > 0 ? "retried" : "success",
          output: result,
          durationMs: Date.now() - startTime,
          retryCount,
        }
      } catch (error) {
        lastError = error instanceof Error ? error.message : String(error)
        retryCount++

        if (
          onError === "retry" &&
          retryCount <= maxRetries
        ) {
          log.warn("Step failed, retrying", {
            index,
            tool: step.tool,
            retryCount,
            error: lastError,
          })
          await this.sleep(retryDelayMs)
        }
      }
    }

    // Step failed
    return {
      stepIndex: index,
      stepId: step.id,
      tool: step.tool,
      status: "failed",
      error: lastError,
      durationMs: Date.now() - startTime,
      retryCount: Math.max(0, retryCount - 1),
    }
  }

  /**
   * Resolve input values, substituting references
   */
  private resolveInputs(
    inputs: MacroStep["inputs"],
    params: Record<string, unknown>,
    previousResults: StepResult[],
    outputs: Record<string, unknown>,
    context: MacroExecutionContext
  ): Record<string, unknown> {
    const resolved: Record<string, unknown> = {}

    for (const [key, value] of Object.entries(inputs)) {
      resolved[key] = this.resolveValue(value, params, previousResults, outputs, context)
    }

    return resolved
  }

  /**
   * Resolve a single value (literal or reference)
   */
  private resolveValue(
    value: string | number | boolean | MacroReference,
    params: Record<string, unknown>,
    previousResults: StepResult[],
    outputs: Record<string, unknown>,
    context: MacroExecutionContext
  ): unknown {
    // Primitive values
    if (typeof value !== "object" || value === null) {
      // Check for template strings
      if (typeof value === "string" && value.includes("${")) {
        return this.interpolateTemplate(value, params, previousResults, outputs, context)
      }
      return value
    }

    // Reference values
    const ref = value as MacroReference

    switch (ref.type) {
      case "parameter": {
        const paramValue = params[ref.name]
        if (ref.transform) {
          return this.applyTransform(paramValue, ref.transform)
        }
        return paramValue
      }

      case "step_output": {
        const result = previousResults[ref.stepIndex]
        if (!result || result.status === "skipped" || result.status === "failed") {
          return undefined
        }
        return this.extractPath(result.output, ref.path)
      }

      case "env": {
        return process.env[ref.name]
      }

      case "context": {
        switch (ref.key) {
          case "sessionId":
            return context.sessionId
          case "workingDirectory":
            return context.workingDirectory
          case "timestamp":
            return context.timestamp
          case "agent":
            return context.agent
        }
      }
    }

    return undefined
  }

  /**
   * Interpolate a template string with references
   */
  private interpolateTemplate(
    template: string,
    params: Record<string, unknown>,
    previousResults: StepResult[],
    outputs: Record<string, unknown>,
    context: MacroExecutionContext
  ): string {
    return template.replace(/\$\{([^}]+)\}/g, (match, expr) => {
      const trimmed = expr.trim()

      // Parameter reference
      if (trimmed.startsWith("param.") || !trimmed.includes(".") && !trimmed.includes("[")) {
        const name = trimmed.startsWith("param.") ? trimmed.slice(6) : trimmed
        const value = params[name]
        return value !== undefined ? String(value) : match
      }

      // Step output reference
      const stepMatch = trimmed.match(/^step\[(\d+)\]\.(.+)$/)
      if (stepMatch) {
        const result = previousResults[parseInt(stepMatch[1], 10)]
        if (result && result.output) {
          const value = this.extractPath(result.output, stepMatch[2])
          return value !== undefined ? String(value) : match
        }
        return match
      }

      // Environment reference
      if (trimmed.startsWith("env.")) {
        const value = process.env[trimmed.slice(4)]
        return value !== undefined ? value : match
      }

      // Context reference
      if (trimmed.startsWith("context.")) {
        const key = trimmed.slice(8) as keyof MacroExecutionContext
        if (key in context) {
          const value = context[key]
          return value !== undefined ? String(value) : match
        }
        return match
      }

      // Output reference
      if (trimmed.startsWith("output.")) {
        const value = outputs[trimmed.slice(7)]
        return value !== undefined ? String(value) : match
      }

      return match
    })
  }

  /**
   * Extract a value from an object using a dot-path
   */
  private extractPath(obj: unknown, path: string): unknown {
    if (obj === undefined || obj === null) return undefined

    const parts = path.split(".")
    let current: unknown = obj

    for (const part of parts) {
      if (current === undefined || current === null) return undefined

      // Handle array access: items[0]
      const arrayMatch = part.match(/^(\w+)\[(\d+)\]$/)
      if (arrayMatch) {
        current = (current as Record<string, unknown>)[arrayMatch[1]]
        if (Array.isArray(current)) {
          current = current[parseInt(arrayMatch[2], 10)]
        } else {
          return undefined
        }
      } else {
        current = (current as Record<string, unknown>)[part]
      }
    }

    return current
  }

  /**
   * Apply a transform to a value
   */
  private applyTransform(value: unknown, transform: string): unknown {
    if (typeof value !== "string") return value

    switch (transform) {
      case "uppercase":
        return value.toUpperCase()
      case "lowercase":
        return value.toLowerCase()
      case "trim":
        return value.trim()
      case "json":
        try {
          return JSON.parse(value)
        } catch {
          return value
        }
      default:
        return value
    }
  }

  /**
   * Evaluate a condition expression
   * Simple expressions only - complex logic should use LLM
   */
  private evaluateCondition(
    expression: string,
    params: Record<string, unknown>,
    previousResults: StepResult[],
    outputs: Record<string, unknown>,
    context: MacroExecutionContext
  ): boolean {
    // Interpolate the expression first
    const interpolated = this.interpolateTemplate(
      expression,
      params,
      previousResults,
      outputs,
      context
    )

    // Simple evaluations
    if (interpolated === "true") return true
    if (interpolated === "false") return false

    // Comparison: "value == expected"
    const eqMatch = interpolated.match(/^(.+?)\s*==\s*(.+)$/)
    if (eqMatch) {
      return eqMatch[1].trim() === eqMatch[2].trim()
    }

    // Not equal: "value != expected"
    const neqMatch = interpolated.match(/^(.+?)\s*!=\s*(.+)$/)
    if (neqMatch) {
      return neqMatch[1].trim() !== neqMatch[2].trim()
    }

    // Existence check: "value"
    return !!interpolated && interpolated !== "undefined" && interpolated !== "null"
  }

  /**
   * Validate macro parameters against provided values
   */
  private validateParameters(
    parameters: MacroParameter[],
    values: Record<string, unknown>
  ): string | null {
    for (const param of parameters) {
      const value = values[param.name]

      if (param.required && value === undefined) {
        if (param.default === undefined) {
          return `Missing required parameter: ${param.name}`
        }
      }

      if (value !== undefined) {
        // Type checking
        const actualType = Array.isArray(value) ? "array" : typeof value
        if (param.type === "array" && !Array.isArray(value)) {
          return `Parameter ${param.name} must be an array`
        }
        if (param.type !== "array" && param.type !== actualType) {
          return `Parameter ${param.name} must be ${param.type}, got ${actualType}`
        }

        // Pattern validation
        if (param.pattern && typeof value === "string") {
          const regex = new RegExp(param.pattern)
          if (!regex.test(value)) {
            return `Parameter ${param.name} does not match pattern: ${param.pattern}`
          }
        }

        // Enum validation
        if (param.enum && !param.enum.includes(value)) {
          return `Parameter ${param.name} must be one of: ${param.enum.join(", ")}`
        }
      }
    }

    return null
  }

  /**
   * Execute with timeout
   */
  private async executeWithTimeout<T>(
    fn: () => Promise<T>,
    timeoutMs: number
  ): Promise<T> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        reject(new Error(`Execution timed out after ${timeoutMs}ms`))
      }, timeoutMs)

      fn()
        .then((result) => {
          clearTimeout(timer)
          resolve(result)
        })
        .catch((error) => {
          clearTimeout(timer)
          reject(error)
        })
    })
  }

  /**
   * Sleep for a given duration
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms))
  }

  /**
   * Create default tool executor (placeholder)
   */
  private createDefaultExecutor(): ToolExecutor {
    return {
      async execute(tool, inputs, context) {
        // This is a placeholder - real implementation would
        // integrate with the actual tool registry
        log.warn("Using default executor (placeholder)", { tool })
        return {
          output: JSON.stringify({ tool, inputs }),
          metadata: { placeholder: true },
        }
      },
    }
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a macro executor with default configuration
 */
export function createExecutor(
  config?: Partial<MacroExecutorConfig>,
  toolExecutor?: ToolExecutor
): MacroExecutor {
  return new MacroExecutor(config, toolExecutor)
}

/**
 * Execute a macro with default executor
 */
export async function executeMacro(
  macro: ToolMacro,
  params: Record<string, unknown>,
  context: MacroExecutionContext,
  config?: Partial<MacroExecutorConfig>
): Promise<MacroResult> {
  const executor = createExecutor(config)
  return executor.execute(macro, params, context)
}

/**
 * Validate a macro definition
 */
export function validateMacroDefinition(macro: ToolMacro): ValidationResult {
  const executor = createExecutor()
  return executor.validate(macro)
}
