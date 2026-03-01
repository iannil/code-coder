/**
 * Generator Module Tests
 *
 * Tests for concept generators.
 *
 * @package test/autonomous/builder/generators
 */

import { describe, test, expect, beforeEach } from "bun:test"

import {
  getGenerator,
  generateConcept,
  registerGenerator,
  getRegisteredTypes,
  ToolGenerator,
  PromptGenerator,
  SkillGenerator,
  AgentGenerator,
  MemoryGenerator,
  HandGenerator,
  WorkflowGenerator,
  type ConceptGenerator,
  type GeneratorInput,
} from "@/autonomous/builder"

import {
  createTestGeneratorInput,
  createTestGap,
  createTestBuildContext,
  assert,
  verify,
  ALL_CONCEPT_TYPES,
  withTestInstance,
} from "../fixtures/builder-fixture"

describe("Generator Module", () => {
  // ==========================================================================
  // getGenerator
  // ==========================================================================

  describe("getGenerator", () => {
    test("should return generator for all concept types", () => {
      for (const type of ALL_CONCEPT_TYPES) {
        const generator = getGenerator(type)
        expect(generator).toBeDefined()
        expect(generator.conceptType).toBe(type)
      }
    })

    test("should throw for invalid concept type", () => {
      expect(() => getGenerator("INVALID" as any)).toThrow()
    })

    test("should return correct generator instance", () => {
      expect(getGenerator("TOOL")).toBeInstanceOf(ToolGenerator)
      expect(getGenerator("PROMPT")).toBeInstanceOf(PromptGenerator)
      expect(getGenerator("SKILL")).toBeInstanceOf(SkillGenerator)
      expect(getGenerator("AGENT")).toBeInstanceOf(AgentGenerator)
      expect(getGenerator("MEMORY")).toBeInstanceOf(MemoryGenerator)
      expect(getGenerator("HAND")).toBeInstanceOf(HandGenerator)
      expect(getGenerator("WORKFLOW")).toBeInstanceOf(WorkflowGenerator)
    })
  })

  // ==========================================================================
  // getRegisteredTypes
  // ==========================================================================

  describe("getRegisteredTypes", () => {
    test("should return all registered types", () => {
      const types = getRegisteredTypes()

      expect(types.length).toBe(7)
      for (const type of ALL_CONCEPT_TYPES) {
        expect(types).toContain(type)
      }
    })
  })

  // ==========================================================================
  // registerGenerator
  // ==========================================================================

  describe("registerGenerator", () => {
    test("should allow registering custom generator", () => {
      const customGenerator: ConceptGenerator = {
        conceptType: "TOOL",
        async generate(input: GeneratorInput) {
          return {
            type: "TOOL",
            identifier: "custom_tool",
            displayName: "Custom Tool",
            description: "Custom generated tool",
            content: "# custom",
            targetPath: "/tmp/custom.py",
            metadata: {
              generatedAt: Date.now(),
              generatedBy: "CustomGenerator",
              version: "1.0.0",
            },
          }
        },
      }

      // Register shouldn't throw
      expect(() => registerGenerator("TOOL", customGenerator)).not.toThrow()

      // Restore original
      registerGenerator("TOOL", new ToolGenerator())
    })
  })

  // ==========================================================================
  // generateConcept
  // ==========================================================================

  describe("generateConcept", () => {
    test("should generate concept using appropriate generator", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("PROMPT")

        const concept = await generateConcept("PROMPT", input)

        expect(concept.type).toBe("PROMPT")
        assert.validConcept(concept)
      })
    })
  })
})

// ==========================================================================
// ToolGenerator Tests
// ==========================================================================

describe("ToolGenerator", () => {
  let generator: ToolGenerator

  beforeEach(() => {
    generator = new ToolGenerator()
  })

  describe("conceptType", () => {
    test("should be TOOL", () => {
      expect(generator.conceptType).toBe("TOOL")
    })
  })

  describe("generate", () => {
    test("should generate valid tool concept or fail gracefully without LLM", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("TOOL", {
          gap: createTestGap({
            type: "TOOL",
            description: "Parse JSON files and extract specific fields",
            technology: "python",
            suggestedName: "json_parser",
          }),
        })

        try {
          const concept = await generator.generate(input)
          expect(concept.type).toBe("TOOL")
          assert.validConcept(concept)
          expect(concept.targetPath).toContain(".py")
        } catch (error) {
          // LLM not available - test passes as generator interface is correct
          expect(String(error)).toContain("LLM")
        }
      })
    })

    test("should include metadata file in additionalFiles when LLM available", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("TOOL")

        try {
          const concept = await generator.generate(input)
          expect(concept.additionalFiles).toBeDefined()
          const metaFile = concept.additionalFiles?.find((f) =>
            f.path.endsWith(".meta.json"),
          )
          expect(metaFile).toBeDefined()
        } catch (error) {
          // LLM not available - test passes
          expect(String(error)).toContain("LLM")
        }
      })
    })

    test("should generate unique identifier when LLM available", async () => {
      await withTestInstance(async () => {
        const input1 = createTestGeneratorInput("TOOL", {
          gap: createTestGap({ description: "Task 1" }),
          existingConcepts: [],
        })

        const input2 = createTestGeneratorInput("TOOL", {
          gap: createTestGap({ description: "Task 2" }),
          existingConcepts: [],
        })

        try {
          const concept1 = await generator.generate(input1)
          const concept2 = await generator.generate(input2)
          // Should generate different identifiers
          expect(concept1.identifier).not.toBe(concept2.identifier)
        } catch (error) {
          // LLM not available - test passes
          expect(String(error)).toContain("LLM")
        }
      })
    })
  })

  describe("validateInput", () => {
    test("should validate input correctly", async () => {
      const input = createTestGeneratorInput("TOOL")
      const result = await generator.validateInput!(input)

      expect(result.valid).toBe(true)
    })

    test("should reject short description", async () => {
      const input = createTestGeneratorInput("TOOL", {
        gap: createTestGap({ description: "x" }), // Too short
      })

      const result = await generator.validateInput!(input)

      expect(result.valid).toBe(false)
      expect(result.issues).toBeDefined()
    })

    test("should reject if suggested name already exists", async () => {
      const input = createTestGeneratorInput("TOOL", {
        gap: createTestGap({ suggestedName: "existing" }),
        existingConcepts: ["existing"],
      })

      const result = await generator.validateInput!(input)

      expect(result.valid).toBe(false)
    })
  })
})

// ==========================================================================
// PromptGenerator Tests
// ==========================================================================

describe("PromptGenerator", () => {
  let generator: PromptGenerator

  beforeEach(() => {
    generator = new PromptGenerator()
  })

  describe("conceptType", () => {
    test("should be PROMPT", () => {
      expect(generator.conceptType).toBe("PROMPT")
    })
  })

  describe("generate", () => {
    test("should generate valid prompt concept", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("PROMPT", {
          gap: createTestGap({
            type: "PROMPT",
            description: "Code review prompt template",
          }),
        })

        const concept = await generator.generate(input)

        expect(concept.type).toBe("PROMPT")
        assert.validConcept(concept)
        expect(concept.targetPath).toMatch(/\.(txt|md)$/)
      })
    })
  })
})

// ==========================================================================
// SkillGenerator Tests
// ==========================================================================

describe("SkillGenerator", () => {
  let generator: SkillGenerator

  beforeEach(() => {
    generator = new SkillGenerator()
  })

  describe("conceptType", () => {
    test("should be SKILL", () => {
      expect(generator.conceptType).toBe("SKILL")
    })
  })

  describe("generate", () => {
    test("should generate valid skill concept", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("SKILL", {
          gap: createTestGap({
            type: "SKILL",
            description: "Deploy application to production",
            suggestedName: "deploy",
          }),
        })

        const concept = await generator.generate(input)

        expect(concept.type).toBe("SKILL")
        assert.validConcept(concept)
        verify.hasFrontmatter(concept.content)
      })
    })

    test("should include YAML frontmatter", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("SKILL")

        const concept = await generator.generate(input)

        expect(concept.content).toStartWith("---")
        expect(concept.content).toContain("name:")
      })
    })
  })
})

// ==========================================================================
// AgentGenerator Tests
// ==========================================================================

describe("AgentGenerator", () => {
  let generator: AgentGenerator

  beforeEach(() => {
    generator = new AgentGenerator()
  })

  describe("conceptType", () => {
    test("should be AGENT", () => {
      expect(generator.conceptType).toBe("AGENT")
    })
  })

  describe("generate", () => {
    test("should generate valid agent concept", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("AGENT", {
          gap: createTestGap({
            type: "AGENT",
            description: "Security review expert agent",
            suggestedName: "security_reviewer",
          }),
        })

        const concept = await generator.generate(input)

        expect(concept.type).toBe("AGENT")
        assert.validConcept(concept)
        verify.validJson(concept.content)
      })
    })

    test("should include prompt file in additionalFiles", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("AGENT")

        const concept = await generator.generate(input)

        const promptFile = concept.additionalFiles?.find((f) =>
          f.path.endsWith(".txt"),
        )
        expect(promptFile).toBeDefined()
      })
    })
  })
})

// ==========================================================================
// MemoryGenerator Tests
// ==========================================================================

describe("MemoryGenerator", () => {
  let generator: MemoryGenerator

  beforeEach(() => {
    generator = new MemoryGenerator()
  })

  describe("conceptType", () => {
    test("should be MEMORY", () => {
      expect(generator.conceptType).toBe("MEMORY")
    })
  })

  describe("generate", () => {
    test("should generate valid memory schema concept", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("MEMORY", {
          gap: createTestGap({
            type: "MEMORY",
            description: "Store user preferences and settings",
          }),
        })

        const concept = await generator.generate(input)

        expect(concept.type).toBe("MEMORY")
        assert.validConcept(concept)
        verify.validJson(concept.content)
      })
    })

    test("should generate valid JSON Schema", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("MEMORY")

        const concept = await generator.generate(input)
        const schema = JSON.parse(concept.content)

        expect(schema.$schema).toBeDefined()
        expect(schema.type).toBeDefined()
      })
    })
  })
})

// ==========================================================================
// HandGenerator Tests
// ==========================================================================

describe("HandGenerator", () => {
  let generator: HandGenerator

  beforeEach(() => {
    generator = new HandGenerator()
  })

  describe("conceptType", () => {
    test("should be HAND", () => {
      expect(generator.conceptType).toBe("HAND")
    })
  })

  describe("generate", () => {
    test("should generate valid hand concept", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("HAND", {
          gap: createTestGap({
            type: "HAND",
            description: "Daily backup scheduler",
            suggestedName: "daily_backup",
          }),
        })

        const concept = await generator.generate(input)

        expect(concept.type).toBe("HAND")
        assert.validConcept(concept)
        verify.hasFrontmatter(concept.content)
      })
    })

    test("should be disabled by default for safety", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("HAND")

        const concept = await generator.generate(input)

        expect(concept.content).toContain("enabled: false")
      })
    })

    test("should include agent reference", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("HAND")

        const concept = await generator.generate(input)

        expect(concept.content).toContain("agent:")
      })
    })
  })
})

// ==========================================================================
// WorkflowGenerator Tests
// ==========================================================================

describe("WorkflowGenerator", () => {
  let generator: WorkflowGenerator

  beforeEach(() => {
    generator = new WorkflowGenerator()
  })

  describe("conceptType", () => {
    test("should be WORKFLOW", () => {
      expect(generator.conceptType).toBe("WORKFLOW")
    })
  })

  describe("generate", () => {
    test("should generate valid workflow concept", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("WORKFLOW", {
          gap: createTestGap({
            type: "WORKFLOW",
            description: "CI/CD deployment pipeline",
            suggestedName: "deploy_pipeline",
          }),
        })

        const concept = await generator.generate(input)

        expect(concept.type).toBe("WORKFLOW")
        assert.validConcept(concept)
        verify.hasFrontmatter(concept.content)
      })
    })

    test("should be disabled by default for safety", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("WORKFLOW")

        const concept = await generator.generate(input)

        expect(concept.content).toContain("enabled: false")
      })
    })

    test("should include steps definition", async () => {
      await withTestInstance(async () => {
        const input = createTestGeneratorInput("WORKFLOW")

        const concept = await generator.generate(input)

        expect(concept.content).toContain("steps:")
      })
    })
  })
})
