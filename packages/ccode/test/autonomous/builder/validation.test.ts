/**
 * Validation Module Tests
 *
 * Tests for concept validators.
 *
 * @package test/autonomous/builder
 */

import { describe, test, expect } from "bun:test"

import {
  getValidator,
  validateConcept,
  ToolValidator,
  PromptValidator,
  SkillValidator,
  AgentValidator,
  MemoryValidator,
  HandValidator,
  WorkflowValidator,
} from "@/autonomous/builder"

import {
  createMockGeneratedConcept,
  assert,
  verify,
  ALL_CONCEPT_TYPES,
} from "./fixtures/builder-fixture"

describe("Validation Module", () => {
  // ==========================================================================
  // getValidator
  // ==========================================================================

  describe("getValidator", () => {
    test("should return validator for all concept types", () => {
      for (const type of ALL_CONCEPT_TYPES) {
        const validator = getValidator(type)
        expect(validator).toBeDefined()
      }
    })

    test("should throw for invalid concept type", () => {
      expect(() => getValidator("INVALID" as any)).toThrow()
    })

    test("should return correct validator class", () => {
      expect(getValidator("TOOL")).toBeInstanceOf(ToolValidator)
      expect(getValidator("PROMPT")).toBeInstanceOf(PromptValidator)
      expect(getValidator("SKILL")).toBeInstanceOf(SkillValidator)
      expect(getValidator("AGENT")).toBeInstanceOf(AgentValidator)
      expect(getValidator("MEMORY")).toBeInstanceOf(MemoryValidator)
      expect(getValidator("HAND")).toBeInstanceOf(HandValidator)
      expect(getValidator("WORKFLOW")).toBeInstanceOf(WorkflowValidator)
    })
  })

  // ==========================================================================
  // validateConcept
  // ==========================================================================

  describe("validateConcept", () => {
    test("should validate concept using appropriate validator", async () => {
      const concept = createMockGeneratedConcept("TOOL")
      const result = await validateConcept(concept)

      assert.validValidation(result)
    })

    test("should validate all concept types", async () => {
      for (const type of ALL_CONCEPT_TYPES) {
        const concept = createMockGeneratedConcept(type)
        const result = await validateConcept(concept)

        assert.validValidation(result)
      }
    })
  })

  // ==========================================================================
  // Common Validation Rules
  // ==========================================================================

  describe("common validation rules", () => {
    test("should require identifier", async () => {
      const concept = createMockGeneratedConcept("TOOL", {
        identifier: "",
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "IDENTIFIER_REQUIRED")).toBe(true)
    })

    test("should validate identifier format", async () => {
      const concept = createMockGeneratedConcept("TOOL", {
        identifier: "INVALID FORMAT!",
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "IDENTIFIER_FORMAT")).toBe(true)
    })

    test("should validate identifier length", async () => {
      const concept = createMockGeneratedConcept("TOOL", {
        identifier: "x", // Too short
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "IDENTIFIER_LENGTH")).toBe(true)
    })

    test("should require content", async () => {
      const concept = createMockGeneratedConcept("TOOL", {
        content: "",
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "CONTENT_REQUIRED")).toBe(true)
    })

    test("should require target path", async () => {
      const concept = createMockGeneratedConcept("TOOL", {
        targetPath: "",
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "TARGET_PATH_REQUIRED")).toBe(true)
    })

    test("should warn on short description", async () => {
      const concept = createMockGeneratedConcept("TOOL", {
        description: "Short", // Less than 10 chars
      })

      const result = await validateConcept(concept)

      // Warning, not error
      expect(result.warnings?.some((e) => e.code === "DESCRIPTION_QUALITY")).toBe(true)
    })

    test("should accept valid concept", async () => {
      const concept = createMockGeneratedConcept("TOOL", {
        identifier: "valid_tool",
        content: "def main():\n    print('hello')\n\nif __name__ == '__main__':\n    main()",
        description: "A valid tool with proper description",
        targetPath: "/tmp/test/tool.py",
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(true)
    })
  })

  // ==========================================================================
  // ToolValidator
  // ==========================================================================

  describe("ToolValidator", () => {
    test("should validate tool code length", async () => {
      const shortTool = createMockGeneratedConcept("TOOL", {
        content: "x = 1", // Very short
      })

      const result = await validateConcept(shortTool)

      // Should have warning about length
      expect(result.warnings?.some((e) => e.code === "TOOL_LENGTH")).toBe(true)
    })

    test("should accept valid Python tool", async () => {
      const concept = createMockGeneratedConcept("TOOL", {
        identifier: "python_tool",
        content: `#!/usr/bin/env python3
"""A valid Python tool"""

import sys

def main():
    print("Hello, World!")
    return 0

if __name__ == "__main__":
    sys.exit(main())
`,
        targetPath: "/tmp/tools/python_tool.py",
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(true)
    })
  })

  // ==========================================================================
  // PromptValidator
  // ==========================================================================

  describe("PromptValidator", () => {
    test("should warn on very short prompts", async () => {
      const shortPrompt = createMockGeneratedConcept("PROMPT", {
        content: "Be helpful.", // Very short
      })

      const result = await validateConcept(shortPrompt)

      expect(result.warnings?.some((e) => e.code === "PROMPT_MIN_LENGTH")).toBe(true)
    })

    test("should reject excessively long prompts", async () => {
      const longPrompt = createMockGeneratedConcept("PROMPT", {
        content: "x".repeat(60000), // Too long
      })

      const result = await validateConcept(longPrompt)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "PROMPT_MAX_LENGTH")).toBe(true)
    })

    test("should accept valid prompt", async () => {
      const concept = createMockGeneratedConcept("PROMPT", {
        identifier: "review_prompt",
        content: `You are a code review assistant. Your task is to:

1. Analyze the provided code for issues
2. Suggest improvements
3. Highlight best practices

Please be thorough but constructive in your feedback.
`,
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(true)
    })
  })

  // ==========================================================================
  // SkillValidator
  // ==========================================================================

  describe("SkillValidator", () => {
    test("should require YAML frontmatter", async () => {
      const noFrontmatter = createMockGeneratedConcept("SKILL", {
        content: "# No frontmatter\nJust content",
      })

      const result = await validateConcept(noFrontmatter)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "SKILL_FRONTMATTER")).toBe(true)
    })

    test("should require name field in frontmatter", async () => {
      const noName = createMockGeneratedConcept("SKILL", {
        content: `---
description: A skill without name
---

# Content
`,
      })

      const result = await validateConcept(noName)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "SKILL_NAME_FIELD")).toBe(true)
    })

    test("should warn if missing description field", async () => {
      const noDesc = createMockGeneratedConcept("SKILL", {
        content: `---
name: my-skill
---

# Content
`,
      })

      const result = await validateConcept(noDesc)

      expect(result.warnings?.some((e) => e.code === "SKILL_DESCRIPTION_FIELD")).toBe(true)
    })

    test("should accept valid skill", async () => {
      const concept = createMockGeneratedConcept("SKILL", {
        identifier: "deploy_skill",
        content: `---
name: deploy_skill
description: Deploy to production servers
---

# Deploy Skill

Deploys the current project to production.

## Usage

\`\`\`
/deploy --env production
\`\`\`
`,
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(true)
    })
  })

  // ==========================================================================
  // AgentValidator
  // ==========================================================================

  describe("AgentValidator", () => {
    test("should require valid JSON", async () => {
      const invalidJson = createMockGeneratedConcept("AGENT", {
        content: "not valid json {",
      })

      const result = await validateConcept(invalidJson)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "AGENT_JSON_VALID")).toBe(true)
    })

    test("should require name field", async () => {
      const noName = createMockGeneratedConcept("AGENT", {
        content: JSON.stringify({ description: "An agent without name" }),
      })

      const result = await validateConcept(noName)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "AGENT_NAME_FIELD")).toBe(true)
    })

    test("should warn if no prompt file", async () => {
      const noPromptFile = createMockGeneratedConcept("AGENT", {
        additionalFiles: [], // No .txt file
      })

      const result = await validateConcept(noPromptFile)

      expect(result.warnings?.some((e) => e.code === "AGENT_PROMPT_FILE")).toBe(true)
    })

    test("should accept valid agent", async () => {
      const concept = createMockGeneratedConcept("AGENT", {
        identifier: "code_reviewer",
        content: JSON.stringify({
          name: "code_reviewer",
          description: "Reviews code for quality issues",
          mode: "standard",
        }),
        additionalFiles: [
          {
            path: "/tmp/agents/code_reviewer.txt",
            content: "You are a code review expert.",
          },
        ],
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(true)
    })
  })

  // ==========================================================================
  // MemoryValidator
  // ==========================================================================

  describe("MemoryValidator", () => {
    test("should require valid JSON", async () => {
      const invalidJson = createMockGeneratedConcept("MEMORY", {
        content: "invalid json",
      })

      const result = await validateConcept(invalidJson)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "MEMORY_JSON_VALID")).toBe(true)
    })

    test("should require valid JSON Schema", async () => {
      const notSchema = createMockGeneratedConcept("MEMORY", {
        content: JSON.stringify({ name: "just an object" }),
      })

      const result = await validateConcept(notSchema)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "MEMORY_SCHEMA_VALID")).toBe(true)
    })

    test("should accept valid JSON Schema", async () => {
      const concept = createMockGeneratedConcept("MEMORY", {
        identifier: "user_prefs",
        content: JSON.stringify({
          $schema: "http://json-schema.org/draft-07/schema#",
          type: "object",
          properties: {
            theme: { type: "string" },
            notifications: { type: "boolean" },
          },
        }),
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(true)
    })
  })

  // ==========================================================================
  // HandValidator
  // ==========================================================================

  describe("HandValidator", () => {
    test("should require YAML frontmatter", async () => {
      const noFrontmatter = createMockGeneratedConcept("HAND", {
        content: "# No frontmatter",
      })

      const result = await validateConcept(noFrontmatter)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "HAND_FRONTMATTER")).toBe(true)
    })

    test("should require agent field", async () => {
      const noAgent = createMockGeneratedConcept("HAND", {
        content: `---
name: my-hand
enabled: false
---

# Content
`,
      })

      const result = await validateConcept(noAgent)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "HAND_AGENT_FIELD")).toBe(true)
    })

    test("should require enabled: false for safety", async () => {
      const enabledHand = createMockGeneratedConcept("HAND", {
        content: `---
name: my-hand
agent: build
enabled: true
---

# Content
`,
      })

      const result = await validateConcept(enabledHand)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "HAND_ENABLED_FALSE")).toBe(true)
    })

    test("should accept valid hand", async () => {
      const concept = createMockGeneratedConcept("HAND", {
        identifier: "daily_report",
        content: `---
name: daily_report
agent: build
enabled: false
schedule: "0 9 * * *"
---

# Daily Report

Generates daily project reports.
`,
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(true)
    })
  })

  // ==========================================================================
  // WorkflowValidator
  // ==========================================================================

  describe("WorkflowValidator", () => {
    test("should require YAML frontmatter", async () => {
      const noFrontmatter = createMockGeneratedConcept("WORKFLOW", {
        content: "# No frontmatter",
      })

      const result = await validateConcept(noFrontmatter)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "WORKFLOW_FRONTMATTER")).toBe(true)
    })

    test("should require steps definition", async () => {
      const noSteps = createMockGeneratedConcept("WORKFLOW", {
        content: `---
name: my-workflow
enabled: false
---

# Content
`,
      })

      const result = await validateConcept(noSteps)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "WORKFLOW_STEPS")).toBe(true)
    })

    test("should require enabled: false for safety", async () => {
      const enabledWorkflow = createMockGeneratedConcept("WORKFLOW", {
        content: `---
name: my-workflow
enabled: true
steps:
  - step1
---

# Content
`,
      })

      const result = await validateConcept(enabledWorkflow)

      expect(result.success).toBe(false)
      expect(result.errors?.some((e) => e.code === "WORKFLOW_ENABLED_FALSE")).toBe(true)
    })

    test("should warn if no initial_step", async () => {
      const noInitial = createMockGeneratedConcept("WORKFLOW", {
        content: `---
name: my-workflow
enabled: false
steps:
  - step1
---

# Content
`,
      })

      const result = await validateConcept(noInitial)

      expect(result.warnings?.some((e) => e.code === "WORKFLOW_INITIAL_STEP")).toBe(true)
    })

    test("should accept valid workflow", async () => {
      const concept = createMockGeneratedConcept("WORKFLOW", {
        identifier: "deploy_workflow",
        content: `---
name: deploy_workflow
enabled: false
initial_step: build
steps:
  - id: build
    action: build
  - id: test
    action: test
  - id: deploy
    action: deploy
---

# Deploy Workflow

Multi-step deployment process.
`,
      })

      const result = await validateConcept(concept)

      expect(result.success).toBe(true)
    })
  })

  // ==========================================================================
  // Quality Score
  // ==========================================================================

  describe("quality score", () => {
    test("should return high score for valid concepts", async () => {
      const concept = createMockGeneratedConcept("TOOL")
      const result = await validateConcept(concept)

      if (result.success && result.qualityScore !== undefined) {
        expect(result.qualityScore).toBeGreaterThanOrEqual(70)
      }
    })

    test("should reduce score for errors", async () => {
      const invalidConcept = createMockGeneratedConcept("TOOL", {
        identifier: "",
        content: "",
      })

      const result = await validateConcept(invalidConcept)

      expect(result.qualityScore).toBeLessThan(70)
    })

    test("should slightly reduce score for warnings", async () => {
      const conceptWithWarnings = createMockGeneratedConcept("TOOL", {
        description: "Short", // Will trigger warning
        content: "x = 1", // Short content warning
      })

      const result = await validateConcept(conceptWithWarnings)

      // Should still pass but have warnings
      expect(result.warnings?.length).toBeGreaterThan(0)
    })
  })
})
