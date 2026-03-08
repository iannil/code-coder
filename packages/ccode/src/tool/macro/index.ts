/**
 * Tool Macro System
 *
 * Enables composable workflows by defining sequences of tool calls
 * that can be executed as a single unit with data flow between steps.
 *
 * ## Quick Start
 *
 * ```typescript
 * import { MacroSystem, createMacro, step, parameter, paramRef } from "@/tool/macro"
 *
 * // Define a macro
 * const reviewMacro = createMacro(
 *   "code-review",
 *   "Code Review",
 *   "Run linting and type checking",
 *   [
 *     step("bash", { command: "eslint ${path}" }),
 *     step("bash", { command: "tsc --noEmit" }),
 *   ],
 *   {
 *     parameters: [
 *       parameter("path", "string", { description: "Path to review" }),
 *     ],
 *   }
 * )
 *
 * // Execute the macro
 * const result = await MacroSystem.execute(reviewMacro, { path: "./src" }, context)
 * ```
 *
 * ## Design Principle
 *
 * Macro definitions are **deterministic** (pure data). Execution
 * involves both deterministic operations (reference resolution,
 * step sequencing) and potentially uncertain decisions (error
 * recovery for complex failures).
 */

// Re-export types only
export type {
  MacroParameterType,
  MacroParameter,
  ParameterReference,
  StepOutputReference,
  EnvReference,
  ContextReference,
  MacroReference,
  StepCondition,
  StepErrorHandling,
  MacroStep,
  ToolMacro,
  StepResult,
  MacroResult,
  ValidationResult,
} from "./definition"

// Re-export Zod schemas and factory functions
export {
  MacroParameterType as MacroParameterTypeSchema,
  MacroParameter as MacroParameterSchema,
  ParameterReference as ParameterReferenceSchema,
  StepOutputReference as StepOutputReferenceSchema,
  EnvReference as EnvReferenceSchema,
  ContextReference as ContextReferenceSchema,
  MacroReference as MacroReferenceSchema,
  StepCondition as StepConditionSchema,
  StepErrorHandling as StepErrorHandlingSchema,
  MacroStep as MacroStepSchema,
  ToolMacro as ToolMacroSchema,
  StepResult as StepResultSchema,
  MacroResult as MacroResultSchema,
  ValidationResult as ValidationResultSchema,
  // Factory functions
  parameter,
  paramRef,
  stepRef,
  envRef,
  contextRef,
  step,
  createMacro,
  // Validation
  validateMacro,
  parseTemplateReferences,
} from "./definition"

// Re-export executor
export {
  MacroExecutor,
  createExecutor,
  executeMacro,
  validateMacroDefinition,
} from "./executor"

export type {
  MacroExecutionContext,
  MacroExecutorConfig,
  ToolExecutor,
} from "./executor"

// ============================================================================
// Convenience Namespace
// ============================================================================

import {
  parameter,
  paramRef,
  stepRef,
  envRef,
  contextRef,
  step,
  createMacro,
  validateMacro,
  parseTemplateReferences,
  type ToolMacro,
  type MacroResult,
  type ValidationResult,
} from "./definition"

import {
  createExecutor,
  executeMacro,
  type MacroExecutionContext,
} from "./executor"

/**
 * Tool Macro System namespace for convenient access
 */
export const MacroSystem = {
  // Factory functions
  parameter,
  paramRef,
  stepRef,
  envRef,
  contextRef,
  step,
  createMacro,

  // Validation
  validate: validateMacro,
  parseReferences: parseTemplateReferences,

  // Execution
  createExecutor,
  execute: executeMacro,

  /**
   * Create and validate a macro in one step
   */
  define(
    id: string,
    name: string,
    description: string,
    steps: Parameters<typeof createMacro>[3],
    options?: Parameters<typeof createMacro>[4]
  ): { macro: ToolMacro; validation: ValidationResult } {
    const macro = createMacro(id, name, description, steps, options)
    const validation = validateMacro(macro)
    return { macro, validation }
  },

  /**
   * Load macros from a configuration object
   */
  loadFromConfig(config: {
    macros: Array<{
      id: string
      name: string
      description: string
      parameters?: Array<{ name: string; type: string; required?: boolean }>
      steps: Array<{ tool: string; inputs: Record<string, unknown> }>
    }>
  }): { macros: ToolMacro[]; errors: Array<{ id: string; error: string }> } {
    const macros: ToolMacro[] = []
    const errors: Array<{ id: string; error: string }> = []

    for (const def of config.macros) {
      try {
        const params = (def.parameters || []).map((p) =>
          parameter(p.name, p.type as "string" | "number" | "boolean" | "array" | "object", { required: p.required ?? true })
        )

        const steps = def.steps.map((s) =>
          step(s.tool, s.inputs as Record<string, string | number | boolean>)
        )

        const macro = createMacro(def.id, def.name, def.description, steps, {
          parameters: params,
        })

        const validation = validateMacro(macro)
        if (!validation.valid) {
          errors.push({
            id: def.id,
            error: validation.errors.map((e) => e.message).join(", "),
          })
        } else {
          macros.push(macro)
        }
      } catch (e) {
        errors.push({
          id: def.id,
          error: e instanceof Error ? e.message : String(e),
        })
      }
    }

    return { macros, errors }
  },
}

// ============================================================================
// Pre-built Macros
// ============================================================================

/**
 * Common pre-built macros for development workflows
 */
export const BuiltinMacros = {
  /**
   * TypeScript build and type check
   */
  typescriptBuild: createMacro(
    "typescript-build",
    "TypeScript Build",
    "Run TypeScript compiler and report errors",
    [
      step("bash", { command: "tsc --noEmit" }, {
        description: "Type check TypeScript files",
        errorHandling: { onError: "continue" },
      }),
    ]
  ),

  /**
   * Code formatting with Prettier
   */
  prettierFormat: createMacro(
    "prettier-format",
    "Prettier Format",
    "Format code with Prettier",
    [
      step("bash", { command: "prettier --write ${path}" }, {
        description: "Format files with Prettier",
      }),
    ],
    {
      parameters: [
        parameter("path", "string", { description: "Path to format", default: "." }),
      ],
    }
  ),

  /**
   * Git status and diff
   */
  gitStatus: createMacro(
    "git-status",
    "Git Status",
    "Show git status and staged changes",
    [
      step("bash", { command: "git status" }, {
        id: "status",
        description: "Show working tree status",
      }),
      step("bash", { command: "git diff --staged" }, {
        id: "diff",
        description: "Show staged changes",
      }),
    ]
  ),

  /**
   * Run tests
   */
  runTests: createMacro(
    "run-tests",
    "Run Tests",
    "Execute test suite",
    [
      step("bash", { command: "${testCommand}" }, {
        description: "Run test suite",
        timeoutMs: 300000, // 5 minutes
      }),
    ],
    {
      parameters: [
        parameter("testCommand", "string", {
          description: "Test command to run",
          default: "bun test",
        }),
      ],
    }
  ),

  /**
   * Code review workflow
   */
  codeReview: createMacro(
    "code-review",
    "Code Review",
    "Run linting, type checking, and tests",
    [
      step("bash", { command: "eslint ${path}" }, {
        id: "lint",
        description: "Run ESLint",
        errorHandling: { onError: "continue" },
      }),
      step("bash", { command: "tsc --noEmit" }, {
        id: "typecheck",
        description: "Run TypeScript type check",
        errorHandling: { onError: "continue" },
      }),
      step("bash", { command: "bun test" }, {
        id: "test",
        description: "Run tests",
        errorHandling: { onError: "continue" },
        timeoutMs: 300000,
      }),
    ],
    {
      parameters: [
        parameter("path", "string", { description: "Path to review", default: "." }),
      ],
      tags: ["quality", "ci"],
    }
  ),
}
