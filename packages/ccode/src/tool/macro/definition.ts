/**
 * Tool Macro Definitions
 *
 * Provides types and utilities for defining tool macros - composable
 * sequences of tool calls that can be executed as a single unit.
 *
 * ## Design Principle
 *
 * Macros enable **declarative workflow composition**. The definition is
 * pure data (deterministic), while execution may involve LLM reasoning
 * for complex input transformations or error recovery.
 *
 * ## Example
 *
 * ```typescript
 * const reviewMacro: ToolMacro = {
 *   id: "code-review",
 *   name: "Code Review",
 *   description: "Run linting, type checking, and tests",
 *   parameters: [
 *     { name: "path", type: "string", required: true },
 *   ],
 *   steps: [
 *     { tool: "bash", inputs: { command: "eslint ${path}" } },
 *     { tool: "bash", inputs: { command: "tsc --noEmit" } },
 *     { tool: "bash", inputs: { command: "bun test" } },
 *   ],
 * }
 * ```
 */

import z from "zod"

// ============================================================================
// Macro Parameter Types
// ============================================================================

/**
 * Supported parameter types for macros
 */
export const MacroParameterType = z.enum([
  "string",
  "number",
  "boolean",
  "array",
  "object",
])
export type MacroParameterType = z.infer<typeof MacroParameterType>

/**
 * A macro parameter definition
 */
export const MacroParameter = z.object({
  /** Parameter name (used in template references) */
  name: z.string(),
  /** Parameter type */
  type: MacroParameterType,
  /** Human-readable description */
  description: z.string().optional(),
  /** Whether the parameter is required */
  required: z.boolean().default(true),
  /** Default value if not provided */
  default: z.unknown().optional(),
  /** Validation pattern (for strings) */
  pattern: z.string().optional(),
  /** Allowed values (enum) */
  enum: z.array(z.unknown()).optional(),
})
export type MacroParameter = z.infer<typeof MacroParameter>

// ============================================================================
// Macro Reference Types
// ============================================================================

/**
 * Reference to a macro parameter: ${param.name}
 */
export const ParameterReference = z.object({
  type: z.literal("parameter"),
  name: z.string(),
  transform: z.string().optional(), // e.g., "uppercase", "trim"
})
export type ParameterReference = z.infer<typeof ParameterReference>

/**
 * Reference to a previous step output: ${step[0].output.key}
 */
export const StepOutputReference = z.object({
  type: z.literal("step_output"),
  stepIndex: z.number().int().nonnegative(),
  path: z.string(), // JSON path within output
})
export type StepOutputReference = z.infer<typeof StepOutputReference>

/**
 * Reference to environment variable: ${env.VAR_NAME}
 */
export const EnvReference = z.object({
  type: z.literal("env"),
  name: z.string(),
})
export type EnvReference = z.infer<typeof EnvReference>

/**
 * Reference to execution context: ${context.sessionId}
 */
export const ContextReference = z.object({
  type: z.literal("context"),
  key: z.enum(["sessionId", "workingDirectory", "timestamp", "agent"]),
})
export type ContextReference = z.infer<typeof ContextReference>

/**
 * Union of all reference types
 */
export const MacroReference = z.discriminatedUnion("type", [
  ParameterReference,
  StepOutputReference,
  EnvReference,
  ContextReference,
])
export type MacroReference = z.infer<typeof MacroReference>

// ============================================================================
// Macro Step Types
// ============================================================================

/**
 * Condition for step execution
 */
export const StepCondition = z.object({
  /** Expression to evaluate (simple JS-like syntax) */
  expression: z.string(),
  /** Whether to invert the condition */
  negate: z.boolean().default(false),
})
export type StepCondition = z.infer<typeof StepCondition>

/**
 * Error handling strategy for a step
 */
export const StepErrorHandling = z.object({
  /** What to do on error */
  onError: z.enum(["fail", "skip", "retry", "continue"]).optional(),
  /** Max retry attempts (for retry strategy) */
  maxRetries: z.number().int().positive().optional(),
  /** Delay between retries in ms */
  retryDelayMs: z.number().int().nonnegative().optional(),
  /** Fallback value on error (for continue strategy) */
  fallbackValue: z.unknown().optional(),
})
export type StepErrorHandling = z.infer<typeof StepErrorHandling>

/**
 * A single step in a macro
 */
export const MacroStep = z.object({
  /** Unique identifier for this step (for references) */
  id: z.string().optional(),
  /** Tool to invoke */
  tool: z.string(),
  /** Input values - can be literals or references */
  inputs: z.record(z.string(), z.union([z.string(), z.number(), z.boolean(), MacroReference])),
  /** Named outputs to capture from the result */
  outputs: z.record(z.string(), z.string()).optional(),
  /** Condition for execution */
  condition: StepCondition.optional(),
  /** Error handling strategy */
  errorHandling: StepErrorHandling.optional(),
  /** Human-readable description */
  description: z.string().optional(),
  /** Timeout for this step in ms */
  timeoutMs: z.number().int().positive().optional(),
})
export type MacroStep = z.infer<typeof MacroStep>

// ============================================================================
// Macro Definition
// ============================================================================

/**
 * A complete tool macro definition
 */
export const ToolMacro = z.object({
  /** Unique identifier */
  id: z.string(),
  /** Human-readable name */
  name: z.string(),
  /** Description of what the macro does */
  description: z.string(),
  /** Macro parameters */
  parameters: z.array(MacroParameter).optional().default([]),
  /** Sequence of steps to execute */
  steps: z.array(MacroStep).min(1),
  /** Tags for categorization */
  tags: z.array(z.string()).optional().default([]),
  /** Author or source */
  author: z.string().optional(),
  /** Version string */
  version: z.string().optional(),
  /** Whether this macro requires user confirmation */
  requiresConfirmation: z.boolean().optional().default(false),
  /** Estimated execution time in ms */
  estimatedDurationMs: z.number().int().positive().optional(),
})
export type ToolMacro = z.infer<typeof ToolMacro>

// ============================================================================
// Execution Types
// ============================================================================

/**
 * Result of a single step execution
 */
export const StepResult = z.object({
  stepIndex: z.number().int().nonnegative(),
  stepId: z.string().optional(),
  tool: z.string(),
  status: z.enum(["success", "skipped", "failed", "retried"]),
  output: z.unknown().optional(),
  error: z.string().optional(),
  durationMs: z.number().int().nonnegative(),
  retryCount: z.number().int().nonnegative().default(0),
})
export type StepResult = z.infer<typeof StepResult>

/**
 * Result of a macro execution
 */
export const MacroResult = z.object({
  macroId: z.string(),
  status: z.enum(["success", "partial", "failed"]),
  stepResults: z.array(StepResult),
  outputs: z.record(z.string(), z.unknown()),
  totalDurationMs: z.number().int().nonnegative(),
  error: z.string().optional(),
})
export type MacroResult = z.infer<typeof MacroResult>

/**
 * Validation result for a macro definition
 */
export const ValidationResult = z.object({
  valid: z.boolean(),
  errors: z.array(z.object({
    path: z.string(),
    message: z.string(),
  })),
  warnings: z.array(z.object({
    path: z.string(),
    message: z.string(),
  })),
})
export type ValidationResult = z.infer<typeof ValidationResult>

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a macro parameter
 */
export function parameter(
  name: string,
  type: MacroParameterType,
  options: Partial<Omit<MacroParameter, "name" | "type">> = {}
): MacroParameter {
  return {
    name,
    type,
    required: true,
    ...options,
  }
}

/**
 * Create a parameter reference
 */
export function paramRef(name: string, transform?: string): ParameterReference {
  return {
    type: "parameter",
    name,
    ...(transform && { transform }),
  }
}

/**
 * Create a step output reference
 */
export function stepRef(stepIndex: number, path: string): StepOutputReference {
  return {
    type: "step_output",
    stepIndex,
    path,
  }
}

/**
 * Create an environment variable reference
 */
export function envRef(name: string): EnvReference {
  return {
    type: "env",
    name,
  }
}

/**
 * Create a context reference
 */
export function contextRef(key: ContextReference["key"]): ContextReference {
  return {
    type: "context",
    key,
  }
}

/**
 * Create a macro step
 */
export function step(
  tool: string,
  inputs: MacroStep["inputs"],
  options: Partial<Omit<MacroStep, "tool" | "inputs">> = {}
): MacroStep {
  return {
    tool,
    inputs,
    ...options,
  }
}

/**
 * Create a tool macro
 */
export function createMacro(
  id: string,
  name: string,
  description: string,
  steps: MacroStep[],
  options: Partial<Omit<ToolMacro, "id" | "name" | "description" | "steps">> = {}
): ToolMacro {
  return {
    id,
    name,
    description,
    steps,
    parameters: [],
    tags: [],
    requiresConfirmation: false,
    ...options,
  }
}

// ============================================================================
// Validation Helpers
// ============================================================================

/**
 * Validate a macro definition
 */
export function validateMacro(macro: unknown): ValidationResult {
  const errors: ValidationResult["errors"] = []
  const warnings: ValidationResult["warnings"] = []

  // Parse with Zod
  const parseResult = ToolMacro.safeParse(macro)
  if (!parseResult.success) {
    for (const issue of parseResult.error.issues) {
      errors.push({
        path: issue.path.join("."),
        message: issue.message,
      })
    }
    return { valid: false, errors, warnings }
  }

  const m = parseResult.data

  // Validate step references
  for (let i = 0; i < m.steps.length; i++) {
    const step = m.steps[i]

    for (const [key, value] of Object.entries(step.inputs)) {
      if (typeof value === "object" && value !== null && "type" in value) {
        const ref = value as MacroReference

        // Check step output references
        if (ref.type === "step_output") {
          if (ref.stepIndex >= i) {
            errors.push({
              path: `steps[${i}].inputs.${key}`,
              message: `Step output reference cannot refer to current or future step (step ${ref.stepIndex})`,
            })
          }
        }

        // Check parameter references
        if (ref.type === "parameter") {
          const paramExists = m.parameters.some((p) => p.name === ref.name)
          if (!paramExists) {
            errors.push({
              path: `steps[${i}].inputs.${key}`,
              message: `Parameter "${ref.name}" is not defined`,
            })
          }
        }
      }
    }
  }

  // Check for duplicate step IDs
  const stepIds = m.steps.map((s) => s.id).filter(Boolean) as string[]
  const duplicateIds = stepIds.filter((id, i) => stepIds.indexOf(id) !== i)
  for (const id of duplicateIds) {
    errors.push({
      path: `steps`,
      message: `Duplicate step ID: "${id}"`,
    })
  }

  // Warnings
  if (m.steps.length > 10) {
    warnings.push({
      path: "steps",
      message: `Macro has ${m.steps.length} steps, consider breaking into smaller macros`,
    })
  }

  if (!m.version) {
    warnings.push({
      path: "version",
      message: "No version specified",
    })
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  }
}

/**
 * Parse a template string for references
 * Format: ${param.name} or ${step[0].output.key} or ${env.VAR}
 */
export function parseTemplateReferences(template: string): MacroReference[] {
  const references: MacroReference[] = []
  const regex = /\$\{([^}]+)\}/g
  let match

  while ((match = regex.exec(template)) !== null) {
    const expr = match[1].trim()

    // Parameter reference: param.name or just name
    if (expr.startsWith("param.") || !expr.includes(".") && !expr.includes("[")) {
      const name = expr.startsWith("param.") ? expr.slice(6) : expr
      references.push({ type: "parameter", name })
      continue
    }

    // Step output reference: step[0].output.key
    const stepMatch = expr.match(/^step\[(\d+)\]\.(.+)$/)
    if (stepMatch) {
      references.push({
        type: "step_output",
        stepIndex: parseInt(stepMatch[1], 10),
        path: stepMatch[2],
      })
      continue
    }

    // Environment reference: env.VAR_NAME
    if (expr.startsWith("env.")) {
      references.push({ type: "env", name: expr.slice(4) })
      continue
    }

    // Context reference: context.key
    if (expr.startsWith("context.")) {
      const key = expr.slice(8) as ContextReference["key"]
      if (["sessionId", "workingDirectory", "timestamp", "agent"].includes(key)) {
        references.push({ type: "context", key })
      }
    }
  }

  return references
}
