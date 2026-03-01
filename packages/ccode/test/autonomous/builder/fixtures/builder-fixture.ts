/**
 * Builder Module Test Fixtures
 *
 * Provides reusable test utilities, factories, and assertion helpers
 * for testing the autonomous builder module.
 *
 * @package test/autonomous/builder/fixtures
 */

import { expect } from "bun:test"
import { nanoid } from "nanoid"
import { tmpdir } from "../../../fixture/fixture"
import { Instance } from "@/project/instance"

import type {
  ConceptType,
  GapDetectionResult,
  BuildContext,
  BuildConstraints,
  BuildRequest,
  GeneratorInput,
  GeneratedConcept,
  ValidationResult,
  RegistrationResult,
  BuildPhaseResult,
  BuildResult,
  GapEvidence,
} from "@/autonomous/builder"
import type { CLOSEScore } from "@/autonomous/decision/criteria"

// ============================================================================
// CLOSE Score Factory
// ============================================================================

export function createTestCLOSEScore(overrides?: Partial<CLOSEScore>): CLOSEScore {
  return {
    convergence: 7,
    leverage: 8,
    optionality: 7,
    surplus: 6,
    evolution: 7,
    total: 7.0,
    ...overrides,
  }
}

// ============================================================================
// Gap Evidence Factory
// ============================================================================

export function createTestGapEvidence(overrides?: Partial<GapEvidence>): GapEvidence {
  return {
    type: "task_failure",
    description: "Test task failed after multiple attempts",
    timestamp: Date.now(),
    source: "Test error message",
    metadata: {
      webSearchUsed: false,
      toolSearchUsed: true,
    },
    ...overrides,
  }
}

// ============================================================================
// Gap Detection Result Factory
// ============================================================================

export function createTestGap(overrides?: Partial<GapDetectionResult>): GapDetectionResult {
  return {
    id: `gap_${nanoid(10)}`,
    type: "TOOL",
    description: "Need to automate CSV file analysis with Python",
    confidence: 0.8,
    evidence: [createTestGapEvidence()],
    closeScore: createTestCLOSEScore(),
    suggestedName: "csv_analyzer",
    technology: "python",
    detectedAt: Date.now(),
    ...overrides,
  }
}

// ============================================================================
// Build Context Factory
// ============================================================================

export function createTestBuildContext(overrides?: Partial<BuildContext>): BuildContext {
  return {
    sessionId: `session_${nanoid(8)}`,
    workingDir: "/tmp/test",
    triggeredBy: "system",
    taskDescription: "Analyze CSV file for anomalies",
    errorMessage: "No suitable tool found",
    technology: "python",
    ...overrides,
  }
}

// ============================================================================
// Build Constraints Factory
// ============================================================================

export function createTestBuildConstraints(overrides?: Partial<BuildConstraints>): BuildConstraints {
  return {
    maxTokens: 10000,
    maxCostUsd: 1.0,
    timeoutMs: 60000,
    autonomyLevel: "wild",
    skipApproval: false,
    preferMinimal: true,
    ...overrides,
  }
}

// ============================================================================
// Build Request Factory
// ============================================================================

export function createTestBuildRequest(overrides?: Partial<BuildRequest>): BuildRequest {
  return {
    gap: createTestGap(overrides?.gap as Partial<GapDetectionResult>),
    context: createTestBuildContext(overrides?.context as Partial<BuildContext>),
    constraints: createTestBuildConstraints(overrides?.constraints as Partial<BuildConstraints>),
    ...overrides,
  }
}

// ============================================================================
// Generator Input Factory
// ============================================================================

export function createTestGeneratorInput(
  type: ConceptType = "TOOL",
  overrides?: Partial<GeneratorInput>,
): GeneratorInput {
  return {
    gap: createTestGap({ type }),
    context: createTestBuildContext(),
    existingConcepts: [],
    hints: {},
    ...overrides,
  }
}

// ============================================================================
// Generated Concept Factory
// ============================================================================

export function createMockGeneratedConcept(
  type: ConceptType = "TOOL",
  overrides?: Partial<GeneratedConcept>,
): GeneratedConcept {
  const identifier = overrides?.identifier ?? `test_${type.toLowerCase()}_${nanoid(6)}`

  const content = getDefaultContentForType(type, identifier)

  return {
    type,
    identifier,
    displayName: toDisplayName(identifier),
    description: `A test ${type.toLowerCase()} for unit testing`,
    content,
    targetPath: getDefaultPathForType(type, identifier),
    additionalFiles: [],
    metadata: {
      generatedAt: Date.now(),
      generatedBy: "test",
      version: "1.0.0",
      tags: ["test"],
    },
    ...overrides,
  }
}

function getDefaultContentForType(type: ConceptType, identifier: string): string {
  switch (type) {
    case "TOOL":
      return `#!/usr/bin/env python3
"""${identifier} - Generated test tool"""

def main():
    print("Test tool executed")

if __name__ == "__main__":
    main()
`
    case "PROMPT":
      return `You are a helpful assistant that performs ${identifier} tasks.

## Instructions
1. Analyze the input
2. Process accordingly
3. Return structured output
`
    case "SKILL":
      return `---
name: ${identifier}
description: A test skill for unit testing
---

# ${toDisplayName(identifier)}

This skill provides functionality for testing.

## Usage

Run this skill with the appropriate arguments.
`
    case "AGENT":
      return JSON.stringify(
        {
          name: identifier,
          description: "Test agent for unit testing",
          mode: "standard",
          permissions: ["read", "write"],
        },
        null,
        2,
      )
    case "MEMORY":
      return JSON.stringify(
        {
          $schema: "http://json-schema.org/draft-07/schema#",
          $id: identifier,
          type: "object",
          title: toDisplayName(identifier),
          properties: {
            id: { type: "string" },
            data: { type: "object" },
          },
          required: ["id"],
        },
        null,
        2,
      )
    case "HAND":
      return `---
name: ${identifier}
agent: build
enabled: false
schedule: "0 0 * * *"
autonomy: bold
---

# ${toDisplayName(identifier)}

This hand is for testing purposes.

## Tasks
- Perform scheduled task
`
    case "WORKFLOW":
      return `---
name: ${identifier}
enabled: false
initial_step: start
---

# ${toDisplayName(identifier)}

Test workflow for unit testing.

## steps:
  - id: start
    action: log
    params:
      message: "Workflow started"
  - id: end
    action: complete
`
    default:
      return `Test content for ${type}`
  }
}

function getDefaultPathForType(type: ConceptType, identifier: string): string {
  const base = "/tmp/test"
  switch (type) {
    case "TOOL":
      return `${base}/tools/${identifier}.py`
    case "PROMPT":
      return `${base}/prompts/${identifier}.txt`
    case "SKILL":
      return `${base}/skills/${identifier}/SKILL.md`
    case "AGENT":
      return `${base}/agents/${identifier}.json`
    case "MEMORY":
      return `${base}/memory/${identifier}.schema.json`
    case "HAND":
      return `${base}/hands/${identifier}/HAND.md`
    case "WORKFLOW":
      return `${base}/workflows/${identifier}/WORKFLOW.md`
    default:
      return `${base}/${identifier}`
  }
}

function toDisplayName(identifier: string): string {
  return identifier
    .split(/[_-]/)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ")
}

// ============================================================================
// Validation Result Factory
// ============================================================================

export function createTestValidationResult(
  success: boolean,
  overrides?: Partial<ValidationResult>,
): ValidationResult {
  return {
    success,
    errors: success
      ? undefined
      : [
          {
            code: "TEST_ERROR",
            message: "Test validation error",
          },
        ],
    warnings: [],
    qualityScore: success ? 85 : 30,
    ...overrides,
  }
}

// ============================================================================
// Registration Result Factory
// ============================================================================

export function createTestRegistrationResult(
  success: boolean,
  overrides?: Partial<RegistrationResult>,
): RegistrationResult {
  return {
    success,
    conceptId: success ? `concept_${nanoid(8)}` : undefined,
    storagePath: success ? "/tmp/test/concept.ts" : undefined,
    error: success ? undefined : "Test registration error",
    backupCreated: false,
    ...overrides,
  }
}

// ============================================================================
// Build Phase Result Factory
// ============================================================================

export function createTestPhaseResult(
  phase: BuildPhaseResult["phase"],
  status: BuildPhaseResult["status"] = "completed",
  overrides?: Partial<BuildPhaseResult>,
): BuildPhaseResult {
  return {
    phase,
    status,
    durationMs: 100,
    output: status === "completed" ? {} : undefined,
    error: status === "failed" ? "Test phase error" : undefined,
    ...overrides,
  }
}

// ============================================================================
// Build Result Factory
// ============================================================================

export function createTestBuildResult(
  success: boolean,
  overrides?: Partial<BuildResult>,
): BuildResult {
  return {
    success,
    concept: success ? createMockGeneratedConcept() : undefined,
    gap: createTestGap(),
    closeScore: createTestCLOSEScore(),
    phases: [
      createTestPhaseResult("evaluation", "completed"),
      createTestPhaseResult("generation", "completed"),
      createTestPhaseResult("validation", "completed"),
      createTestPhaseResult("approval", success ? "completed" : "failed"),
      createTestPhaseResult("registration", success ? "completed" : "skipped"),
    ],
    durationMs: 500,
    summary: success ? "Build completed successfully" : "Build failed",
    approvalRequired: false,
    approvalGranted: success,
    registration: success ? createTestRegistrationResult(true) : undefined,
    ...overrides,
  }
}

// ============================================================================
// Task Failure Factory
// ============================================================================

export function createTestTaskFailure(overrides?: Record<string, unknown>) {
  return {
    sessionId: `session_${nanoid(8)}`,
    description: "Need to analyze CSV file for anomalies using Python",
    errorMessage: "No suitable tool found for CSV analysis",
    technology: "python",
    attempts: 3,
    webSearchUsed: true,
    toolSearchUsed: true,
    ...overrides,
  }
}

// ============================================================================
// Assertion Helpers
// ============================================================================

export const assert = {
  /**
   * Assert a valid gap detection result
   */
  validGap: (gap: GapDetectionResult) => {
    expect(gap.id).toBeDefined()
    expect(gap.id).toStartWith("gap_")
    expect(gap.type).toBeDefined()
    expect(["AGENT", "PROMPT", "SKILL", "TOOL", "HAND", "MEMORY", "WORKFLOW"]).toContain(gap.type)
    expect(gap.confidence).toBeGreaterThanOrEqual(0)
    expect(gap.confidence).toBeLessThanOrEqual(1)
    expect(gap.evidence).toBeDefined()
    expect(gap.evidence.length).toBeGreaterThan(0)
    expect(gap.closeScore).toBeDefined()
    expect(gap.closeScore.total).toBeGreaterThan(0)
    expect(gap.detectedAt).toBeDefined()
  },

  /**
   * Assert a valid generated concept
   */
  validConcept: (concept: GeneratedConcept) => {
    expect(concept.type).toBeDefined()
    expect(concept.identifier).toBeDefined()
    expect(concept.identifier.length).toBeGreaterThanOrEqual(2)
    expect(concept.displayName).toBeDefined()
    expect(concept.content).toBeDefined()
    expect(concept.content.length).toBeGreaterThan(0)
    expect(concept.targetPath).toBeDefined()
    expect(concept.metadata).toBeDefined()
    expect(concept.metadata.generatedAt).toBeDefined()
    expect(concept.metadata.version).toBeDefined()
  },

  /**
   * Assert a successful build result
   */
  buildSuccess: (result: BuildResult) => {
    expect(result.success).toBe(true)
    expect(result.concept).toBeDefined()
    expect(result.phases.length).toBeGreaterThan(0)
    expect(result.summary).toBeDefined()
  },

  /**
   * Assert a failed build result
   */
  buildFailure: (result: BuildResult, expectedReason?: string) => {
    expect(result.success).toBe(false)
    if (expectedReason) {
      expect(result.summary).toContain(expectedReason)
    }
  },

  /**
   * Assert valid validation result
   */
  validValidation: (result: ValidationResult) => {
    expect(result.success).toBeDefined()
    if (!result.success) {
      expect(result.errors).toBeDefined()
      expect(result.errors!.length).toBeGreaterThan(0)
    }
    if (result.qualityScore !== undefined) {
      expect(result.qualityScore).toBeGreaterThanOrEqual(0)
      expect(result.qualityScore).toBeLessThanOrEqual(100)
    }
  },

  /**
   * Assert valid registration result
   */
  validRegistration: (result: RegistrationResult) => {
    expect(result.success).toBeDefined()
    if (result.success) {
      expect(result.conceptId).toBeDefined()
    } else {
      expect(result.error).toBeDefined()
    }
  },
}

// ============================================================================
// Verification Helpers
// ============================================================================

export const verify = {
  /**
   * Verify concept type matches expected
   */
  conceptType: (gap: GapDetectionResult, expected: ConceptType) => {
    expect(gap.type).toBe(expected)
  },

  /**
   * Verify CLOSE score meets threshold
   */
  closeThreshold: (gap: GapDetectionResult, threshold: number) => {
    expect(gap.closeScore.total).toBeGreaterThanOrEqual(threshold)
  },

  /**
   * Verify phase was completed
   */
  phaseCompleted: (result: BuildResult, phase: BuildPhaseResult["phase"]) => {
    const phaseResult = result.phases.find((p) => p.phase === phase)
    expect(phaseResult).toBeDefined()
    expect(phaseResult?.status).toBe("completed")
  },

  /**
   * Verify phase was skipped
   */
  phaseSkipped: (result: BuildResult, phase: BuildPhaseResult["phase"]) => {
    const phaseResult = result.phases.find((p) => p.phase === phase)
    expect(phaseResult).toBeDefined()
    expect(phaseResult?.status).toBe("skipped")
  },

  /**
   * Verify phase failed
   */
  phaseFailed: (result: BuildResult, phase: BuildPhaseResult["phase"]) => {
    const phaseResult = result.phases.find((p) => p.phase === phase)
    expect(phaseResult).toBeDefined()
    expect(phaseResult?.status).toBe("failed")
  },

  /**
   * Verify identifier format
   */
  validIdentifier: (identifier: string) => {
    expect(identifier).toMatch(/^[a-z0-9][a-z0-9_-]*[a-z0-9]$|^[a-z0-9]$/)
    expect(identifier.length).toBeGreaterThanOrEqual(2)
    expect(identifier.length).toBeLessThanOrEqual(50)
  },

  /**
   * Verify content includes required frontmatter
   */
  hasFrontmatter: (content: string) => {
    expect(content).toStartWith("---")
    expect(content.split("---").length).toBeGreaterThanOrEqual(3)
  },

  /**
   * Verify valid JSON content
   */
  validJson: (content: string) => {
    expect(() => JSON.parse(content)).not.toThrow()
  },
}

// ============================================================================
// Test Instance Wrapper
// ============================================================================

/**
 * Run a test function within a temporary Instance context
 *
 * Use this wrapper for tests that access storage or require Instance.project.id
 */
export async function withTestInstance<T>(fn: (tmpDir: string) => Promise<T>): Promise<T> {
  const tmp = await tmpdir({ git: true })
  try {
    return await Instance.provide({
      directory: tmp.path,
      fn: () => fn(tmp.path),
    })
  } finally {
    await tmp[Symbol.asyncDispose]()
  }
}

/**
 * Wraps a test function for use in describe() blocks that need Instance context
 */
export function withTestInstanceWrapper() {
  return <T>(fn: () => Promise<T>) => {
    return () =>
      withTestInstance(async () => {
        await fn()
      })
  }
}

// ============================================================================
// Mock Helpers
// ============================================================================

/**
 * Create a mock LLM response for code generation
 */
export function createMockLLMCodeResponse(language: "python" | "nodejs" | "shell" = "python") {
  const code =
    language === "python"
      ? `def main():\n    print("Generated code")\n\nif __name__ == "__main__":\n    main()`
      : language === "nodejs"
        ? `function main() {\n  console.log("Generated code");\n}\n\nmain();`
        : `#!/bin/bash\necho "Generated code"`

  return {
    code,
    language,
    confidence: 0.9,
  }
}

/**
 * All concept types for iteration
 */
export const ALL_CONCEPT_TYPES: ConceptType[] = [
  "AGENT",
  "PROMPT",
  "SKILL",
  "TOOL",
  "HAND",
  "MEMORY",
  "WORKFLOW",
]
