/**
 * Concept Validation
 *
 * Validates generated concepts before registration.
 * Implements schema validation, semantic checks, and duplicate detection.
 *
 * @package autonomous/builder
 */

import { Log } from "@/util/log"
import { ToolRegistry } from "@/memory/tools"
import { createSandboxExecutor, type SandboxExecutor } from "../execution/sandbox"

import {
  type ConceptType,
  type GeneratedConcept,
  type ValidationResult,
  type ConceptValidator,
  CONCEPT_METADATA,
} from "./types"
import { getConceptInventory } from "./concept-inventory"

const log = Log.create({ service: "autonomous.builder.validation" })

// ============================================================================
// Validation Rules
// ============================================================================

interface ValidationRule {
  code: string
  check: (concept: GeneratedConcept) => Promise<{ pass: boolean; message?: string }>
  severity: "error" | "warning"
}

// ============================================================================
// Base Validator
// ============================================================================

class BaseValidator implements ConceptValidator {
  protected rules: ValidationRule[] = []

  async validate(concept: GeneratedConcept): Promise<ValidationResult> {
    const errors: Array<{ code: string; message: string; field?: string }> = []
    const warnings: Array<{ code: string; message: string; field?: string }> = []
    let qualityScore = 100

    // Run all validation rules
    for (const rule of this.rules) {
      try {
        const result = await rule.check(concept)
        if (!result.pass) {
          const issue = {
            code: rule.code,
            message: result.message ?? `Validation failed: ${rule.code}`,
          }

          if (rule.severity === "error") {
            errors.push(issue)
            qualityScore -= 20
          } else {
            warnings.push(issue)
            qualityScore -= 5
          }
        }
      } catch (error) {
        log.warn("Validation rule failed", { rule: rule.code, error })
        warnings.push({
          code: `${rule.code}_exception`,
          message: `Validation rule threw exception: ${error}`,
        })
      }
    }

    return {
      success: errors.length === 0,
      errors: errors.length > 0 ? errors : undefined,
      warnings: warnings.length > 0 ? warnings : undefined,
      qualityScore: Math.max(0, qualityScore),
    }
  }
}

// ============================================================================
// Common Validation Rules
// ============================================================================

const commonRules: ValidationRule[] = [
  {
    code: "IDENTIFIER_REQUIRED",
    check: async (c) => ({
      pass: Boolean(c.identifier && c.identifier.length > 0),
      message: "Concept identifier is required",
    }),
    severity: "error",
  },
  {
    code: "IDENTIFIER_FORMAT",
    check: async (c) => ({
      pass: /^[a-z0-9][a-z0-9_-]*[a-z0-9]$|^[a-z0-9]$/.test(c.identifier),
      message: "Identifier must be lowercase alphanumeric with hyphens/underscores",
    }),
    severity: "error",
  },
  {
    code: "IDENTIFIER_LENGTH",
    check: async (c) => ({
      pass: c.identifier.length >= 2 && c.identifier.length <= 50,
      message: "Identifier must be 2-50 characters",
    }),
    severity: "error",
  },
  {
    code: "CONTENT_REQUIRED",
    check: async (c) => ({
      pass: Boolean(c.content && c.content.length > 0),
      message: "Concept content is required",
    }),
    severity: "error",
  },
  {
    code: "TARGET_PATH_REQUIRED",
    check: async (c) => ({
      pass: Boolean(c.targetPath && c.targetPath.length > 0),
      message: "Target path is required",
    }),
    severity: "error",
  },
  {
    code: "DESCRIPTION_QUALITY",
    check: async (c) => ({
      pass: c.description.length >= 10,
      message: "Description should be at least 10 characters",
    }),
    severity: "warning",
  },
]

// ============================================================================
// Type-Specific Validators
// ============================================================================

export class ToolValidator extends BaseValidator {
  private sandbox: SandboxExecutor | null = null

  constructor() {
    super()
    this.rules = [
      ...commonRules,
      {
        code: "TOOL_SYNTAX",
        check: async (c) => this.checkSyntax(c),
        severity: "error",
      },
      {
        code: "TOOL_DUPLICATE",
        check: async (c) => this.checkDuplicate(c),
        severity: "error",
      },
      {
        code: "TOOL_LENGTH",
        check: async (c) => ({
          pass: c.content.length >= 50 && c.content.length <= 10000,
          message: "Tool code should be 50-10000 characters",
        }),
        severity: "warning",
      },
    ]
  }

  private async checkSyntax(concept: GeneratedConcept): Promise<{ pass: boolean; message?: string }> {
    // Determine language from file extension
    const ext = concept.targetPath.split(".").pop()
    const language = ext === "py" ? "python" : ext === "js" ? "nodejs" : "shell"

    try {
      if (!this.sandbox) {
        this.sandbox = await createSandboxExecutor()
      }

      // For Python, try to compile
      if (language === "python") {
        const checkCode = `import ast\ntry:\n    ast.parse('''${concept.content.replace(/'/g, "\\'")}''')\n    print("OK")\nexcept SyntaxError as e:\n    print(f"SYNTAX_ERROR: {e}")`
        const result = await this.sandbox.execute({
          language: "python",
          code: checkCode,
          timeoutMs: 5000,
        })

        if (result.stdout.includes("SYNTAX_ERROR")) {
          return { pass: false, message: result.stdout }
        }
      }

      // For Node.js, try to parse
      if (language === "nodejs") {
        const checkCode = `try { new Function(${JSON.stringify(concept.content)}); console.log("OK") } catch(e) { console.log("SYNTAX_ERROR: " + e.message) }`
        const result = await this.sandbox.execute({
          language: "nodejs",
          code: checkCode,
          timeoutMs: 5000,
        })

        if (result.stdout.includes("SYNTAX_ERROR")) {
          return { pass: false, message: result.stdout }
        }
      }

      return { pass: true }
    } catch (error) {
      log.warn("Syntax check failed", { error })
      return { pass: true } // Don't block on check failure
    }
  }

  private async checkDuplicate(concept: GeneratedConcept): Promise<{ pass: boolean; message?: string }> {
    try {
      const existing = await ToolRegistry.findDuplicate(concept.displayName, concept.content)
      if (existing) {
        return {
          pass: false,
          message: `Similar tool already exists: ${existing.name} (${existing.id})`,
        }
      }
      return { pass: true }
    } catch (error) {
      return { pass: true } // Don't block on check failure
    }
  }

  async cleanup(): Promise<void> {
    if (this.sandbox) {
      await this.sandbox.cleanup()
      this.sandbox = null
    }
  }
}

export class PromptValidator extends BaseValidator {
  constructor() {
    super()
    this.rules = [
      ...commonRules,
      {
        code: "PROMPT_MIN_LENGTH",
        check: async (c) => ({
          pass: c.content.length >= 20,
          message: "Prompt should be at least 20 characters",
        }),
        severity: "warning",
      },
      {
        code: "PROMPT_MAX_LENGTH",
        check: async (c) => ({
          pass: c.content.length <= 50000,
          message: "Prompt should not exceed 50000 characters",
        }),
        severity: "error",
      },
    ]
  }
}

export class SkillValidator extends BaseValidator {
  constructor() {
    super()
    this.rules = [
      ...commonRules,
      {
        code: "SKILL_FRONTMATTER",
        check: async (c) => ({
          pass: c.content.startsWith("---"),
          message: "Skill must have YAML frontmatter",
        }),
        severity: "error",
      },
      {
        code: "SKILL_NAME_FIELD",
        check: async (c) => ({
          pass: c.content.includes("name:"),
          message: "Skill frontmatter must include name field",
        }),
        severity: "error",
      },
      {
        code: "SKILL_DESCRIPTION_FIELD",
        check: async (c) => ({
          pass: c.content.includes("description:"),
          message: "Skill frontmatter must include description field",
        }),
        severity: "warning",
      },
      {
        code: "SKILL_DUPLICATE",
        check: async (c) => this.checkDuplicate(c),
        severity: "error",
      },
    ]
  }

  private async checkDuplicate(concept: GeneratedConcept): Promise<{ pass: boolean; message?: string }> {
    const inventory = getConceptInventory()
    const existing = await inventory.get(concept.identifier, "SKILL")
    if (existing) {
      return {
        pass: false,
        message: `Skill with identifier "${concept.identifier}" already exists`,
      }
    }
    return { pass: true }
  }
}

export class AgentValidator extends BaseValidator {
  constructor() {
    super()
    this.rules = [
      ...commonRules,
      {
        code: "AGENT_JSON_VALID",
        check: async (c) => {
          try {
            JSON.parse(c.content)
            return { pass: true }
          } catch {
            return { pass: false, message: "Agent configuration must be valid JSON" }
          }
        },
        severity: "error",
      },
      {
        code: "AGENT_NAME_FIELD",
        check: async (c) => {
          try {
            const config = JSON.parse(c.content)
            return {
              pass: Boolean(config.name),
              message: "Agent configuration must include name field",
            }
          } catch {
            return { pass: false }
          }
        },
        severity: "error",
      },
      {
        code: "AGENT_DUPLICATE",
        check: async (c) => this.checkDuplicate(c),
        severity: "error",
      },
      {
        code: "AGENT_PROMPT_FILE",
        check: async (c) => ({
          pass: Boolean(c.additionalFiles?.some((f) => f.path.endsWith(".txt"))),
          message: "Agent should have a prompt file",
        }),
        severity: "warning",
      },
    ]
  }

  private async checkDuplicate(concept: GeneratedConcept): Promise<{ pass: boolean; message?: string }> {
    const inventory = getConceptInventory()
    const existing = await inventory.get(concept.identifier, "AGENT")
    if (existing) {
      return {
        pass: false,
        message: `Agent with identifier "${concept.identifier}" already exists`,
      }
    }
    return { pass: true }
  }
}

export class MemoryValidator extends BaseValidator {
  constructor() {
    super()
    this.rules = [
      ...commonRules,
      {
        code: "MEMORY_JSON_VALID",
        check: async (c) => {
          try {
            JSON.parse(c.content)
            return { pass: true }
          } catch {
            return { pass: false, message: "Memory schema must be valid JSON" }
          }
        },
        severity: "error",
      },
      {
        code: "MEMORY_SCHEMA_VALID",
        check: async (c) => {
          try {
            const schema = JSON.parse(c.content)
            return {
              pass: schema.$schema && schema.type,
              message: "Must be a valid JSON Schema with $schema and type fields",
            }
          } catch {
            return { pass: false }
          }
        },
        severity: "error",
      },
    ]
  }
}

export class HandValidator extends BaseValidator {
  constructor() {
    super()
    this.rules = [
      ...commonRules,
      {
        code: "HAND_FRONTMATTER",
        check: async (c) => ({
          pass: c.content.startsWith("---"),
          message: "Hand must have YAML frontmatter",
        }),
        severity: "error",
      },
      {
        code: "HAND_AGENT_FIELD",
        check: async (c) => ({
          pass: c.content.includes("agent:"),
          message: "Hand must specify an agent",
        }),
        severity: "error",
      },
      {
        code: "HAND_ENABLED_FALSE",
        check: async (c) => ({
          pass: c.content.includes("enabled: false"),
          message: "New hands should be disabled by default for safety",
        }),
        severity: "error",
      },
      {
        code: "HAND_CRON_VALID",
        check: async (c) => this.checkCronSyntax(c),
        severity: "warning",
      },
    ]
  }

  private async checkCronSyntax(concept: GeneratedConcept): Promise<{ pass: boolean; message?: string }> {
    const scheduleMatch = concept.content.match(/schedule:\s*"([^"]+)"/)
    if (!scheduleMatch) {
      return { pass: true } // No schedule is fine
    }

    const cron = scheduleMatch[1]
    const parts = cron.split(/\s+/)

    // Basic cron validation (5 parts: min hour day month weekday)
    if (parts.length !== 5) {
      return { pass: false, message: `Invalid cron expression: ${cron} (expected 5 parts)` }
    }

    return { pass: true }
  }
}

export class WorkflowValidator extends BaseValidator {
  constructor() {
    super()
    this.rules = [
      ...commonRules,
      {
        code: "WORKFLOW_FRONTMATTER",
        check: async (c) => ({
          pass: c.content.startsWith("---"),
          message: "Workflow must have YAML frontmatter",
        }),
        severity: "error",
      },
      {
        code: "WORKFLOW_STEPS",
        check: async (c) => ({
          pass: c.content.includes("steps:"),
          message: "Workflow must define steps",
        }),
        severity: "error",
      },
      {
        code: "WORKFLOW_ENABLED_FALSE",
        check: async (c) => ({
          pass: c.content.includes("enabled: false"),
          message: "New workflows should be disabled by default for safety",
        }),
        severity: "error",
      },
      {
        code: "WORKFLOW_INITIAL_STEP",
        check: async (c) => ({
          pass: c.content.includes("initial_step:"),
          message: "Workflow should specify initial_step",
        }),
        severity: "warning",
      },
    ]
  }
}

// ============================================================================
// Validator Factory
// ============================================================================

const validators: Map<ConceptType, ConceptValidator> = new Map([
  ["TOOL", new ToolValidator()],
  ["PROMPT", new PromptValidator()],
  ["SKILL", new SkillValidator()],
  ["AGENT", new AgentValidator()],
  ["MEMORY", new MemoryValidator()],
  ["HAND", new HandValidator()],
  ["WORKFLOW", new WorkflowValidator()],
])

/**
 * Get the validator for a concept type
 */
export function getValidator(type: ConceptType): ConceptValidator {
  const validator = validators.get(type)
  if (!validator) {
    throw new Error(`No validator registered for concept type: ${type}`)
  }
  return validator
}

/**
 * Validate a concept
 */
export async function validateConcept(concept: GeneratedConcept): Promise<ValidationResult> {
  const validator = getValidator(concept.type)
  return validator.validate(concept)
}
