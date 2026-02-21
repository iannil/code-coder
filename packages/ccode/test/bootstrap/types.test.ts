import { describe, test, expect } from "bun:test"
import { BootstrapTypes } from "@/bootstrap/types"

describe("BootstrapTypes", () => {
  describe("SkillType", () => {
    test("accepts valid skill types", () => {
      expect(BootstrapTypes.SkillType.parse("pattern")).toBe("pattern")
      expect(BootstrapTypes.SkillType.parse("workflow")).toBe("workflow")
      expect(BootstrapTypes.SkillType.parse("tool")).toBe("tool")
      expect(BootstrapTypes.SkillType.parse("agent")).toBe("agent")
    })

    test("rejects invalid skill types", () => {
      expect(() => BootstrapTypes.SkillType.parse("invalid")).toThrow()
    })
  })

  describe("TriggerType", () => {
    test("accepts valid trigger types", () => {
      expect(BootstrapTypes.TriggerType.parse("auto")).toBe("auto")
      expect(BootstrapTypes.TriggerType.parse("session_end")).toBe("session_end")
      expect(BootstrapTypes.TriggerType.parse("manual")).toBe("manual")
      expect(BootstrapTypes.TriggerType.parse("scheduled")).toBe("scheduled")
    })
  })

  describe("VerificationStatus", () => {
    test("accepts valid verification statuses", () => {
      expect(BootstrapTypes.VerificationStatus.parse("pending")).toBe("pending")
      expect(BootstrapTypes.VerificationStatus.parse("passed")).toBe("passed")
      expect(BootstrapTypes.VerificationStatus.parse("failed")).toBe("failed")
    })
  })

  describe("ConfidenceLevel", () => {
    test("accepts valid confidence levels", () => {
      expect(BootstrapTypes.ConfidenceLevel.parse("experimental")).toBe("experimental")
      expect(BootstrapTypes.ConfidenceLevel.parse("stable")).toBe("stable")
      expect(BootstrapTypes.ConfidenceLevel.parse("mature")).toBe("mature")
    })
  })

  describe("SkillContent", () => {
    test("accepts empty content", () => {
      const result = BootstrapTypes.SkillContent.parse({})
      expect(result).toEqual({})
    })

    test("accepts code content", () => {
      const result = BootstrapTypes.SkillContent.parse({
        code: "const x = 1",
      })
      expect(result.code).toBe("const x = 1")
    })

    test("accepts workflow steps", () => {
      const result = BootstrapTypes.SkillContent.parse({
        steps: ["step1", "step2", "step3"],
      })
      expect(result.steps).toEqual(["step1", "step2", "step3"])
    })

    test("accepts tool definition", () => {
      const result = BootstrapTypes.SkillContent.parse({
        toolDefinition: '{"name": "test"}',
      })
      expect(result.toolDefinition).toBe('{"name": "test"}')
    })

    test("accepts agent prompt", () => {
      const result = BootstrapTypes.SkillContent.parse({
        agentPrompt: "You are a helpful assistant",
      })
      expect(result.agentPrompt).toBe("You are a helpful assistant")
    })
  })

  describe("SkillSource", () => {
    test("accepts valid source", () => {
      const result = BootstrapTypes.SkillSource.parse({
        sessionId: "session_123",
        toolCalls: ["tc_1", "tc_2"],
        problem: "How to fix bug",
        solution: "Use try-catch",
      })
      expect(result.sessionId).toBe("session_123")
      expect(result.toolCalls).toHaveLength(2)
    })

    test("requires all fields", () => {
      expect(() =>
        BootstrapTypes.SkillSource.parse({
          sessionId: "session_123",
          // missing toolCalls, problem, solution
        }),
      ).toThrow()
    })
  })

  describe("SkillVerification", () => {
    test("accepts minimal verification", () => {
      const result = BootstrapTypes.SkillVerification.parse({
        status: "pending",
        attempts: 0,
        confidence: 0.5,
      })
      expect(result.status).toBe("pending")
      expect(result.attempts).toBe(0)
      expect(result.confidence).toBe(0.5)
    })

    test("accepts full verification", () => {
      const result = BootstrapTypes.SkillVerification.parse({
        status: "passed",
        attempts: 3,
        lastResult: "all tests passed",
        confidence: 0.9,
        testScenarios: ["scenario_1", "scenario_2"],
      })
      expect(result.lastResult).toBe("all tests passed")
      expect(result.testScenarios).toHaveLength(2)
    })

    test("validates confidence range", () => {
      expect(() =>
        BootstrapTypes.SkillVerification.parse({
          status: "pending",
          attempts: 0,
          confidence: 1.5, // > 1
        }),
      ).toThrow()

      expect(() =>
        BootstrapTypes.SkillVerification.parse({
          status: "pending",
          attempts: 0,
          confidence: -0.1, // < 0
        }),
      ).toThrow()
    })

    test("validates attempts is non-negative integer", () => {
      expect(() =>
        BootstrapTypes.SkillVerification.parse({
          status: "pending",
          attempts: -1,
          confidence: 0.5,
        }),
      ).toThrow()
    })
  })

  describe("SkillMetadata", () => {
    test("accepts minimal metadata", () => {
      const result = BootstrapTypes.SkillMetadata.parse({
        created: Date.now(),
        updated: Date.now(),
        usageCount: 0,
      })
      expect(result.usageCount).toBe(0)
    })

    test("accepts full metadata", () => {
      const result = BootstrapTypes.SkillMetadata.parse({
        created: Date.now(),
        updated: Date.now(),
        usageCount: 10,
        successCount: 8,
        failureCount: 2,
        avgTokensSaved: 100,
        avgStepsSaved: 2.5,
      })
      expect(result.successCount).toBe(8)
      expect(result.avgTokensSaved).toBe(100)
    })
  })

  describe("SkillCandidate", () => {
    const validCandidate = {
      id: "cand_123",
      type: "pattern",
      name: "test-skill",
      description: "A test skill",
      trigger: {
        type: "auto",
        context: "testing context",
      },
      content: {
        code: "const x = 1",
      },
      source: {
        sessionId: "session_123",
        toolCalls: ["tc_1"],
        problem: "test problem",
        solution: "test solution",
      },
      verification: {
        status: "pending",
        attempts: 0,
        confidence: 0.3,
      },
      metadata: {
        created: Date.now(),
        updated: Date.now(),
        usageCount: 0,
      },
    }

    test("accepts valid candidate", () => {
      const result = BootstrapTypes.SkillCandidate.parse(validCandidate)
      expect(result.id).toBe("cand_123")
      expect(result.name).toBe("test-skill")
    })

    test("requires all fields", () => {
      const { id, ...missingId } = validCandidate
      expect(() => BootstrapTypes.SkillCandidate.parse(missingId)).toThrow()
    })
  })

  describe("CandidateStore", () => {
    test("accepts empty store", () => {
      const result = BootstrapTypes.CandidateStore.parse({
        version: 1,
        candidates: [],
        time: {
          created: Date.now(),
          updated: Date.now(),
        },
      })
      expect(result.candidates).toHaveLength(0)
    })

    test("accepts store with candidates", () => {
      const result = BootstrapTypes.CandidateStore.parse({
        version: 1,
        candidates: [
          {
            id: "cand_1",
            type: "pattern",
            name: "test",
            description: "desc",
            trigger: { type: "auto", context: "ctx" },
            content: {},
            source: {
              sessionId: "s1",
              toolCalls: [],
              problem: "p",
              solution: "s",
            },
            verification: { status: "pending", attempts: 0, confidence: 0.5 },
            metadata: { created: 0, updated: 0, usageCount: 0 },
          },
        ],
        time: { created: 0, updated: 0 },
      })
      expect(result.candidates).toHaveLength(1)
    })
  })

  describe("ToolCallRecord", () => {
    test("accepts valid tool call", () => {
      const result = BootstrapTypes.ToolCallRecord.parse({
        id: "tc_123",
        tool: "bash",
        input: { command: "ls -la" },
        output: "file1\nfile2",
        duration: 100,
        timestamp: Date.now(),
      })
      expect(result.tool).toBe("bash")
      expect(result.input).toEqual({ command: "ls -la" })
    })

    test("accepts minimal tool call", () => {
      const result = BootstrapTypes.ToolCallRecord.parse({
        id: "tc_123",
        tool: "read",
        input: {},
        timestamp: Date.now(),
      })
      expect(result.output).toBeUndefined()
      expect(result.duration).toBeUndefined()
    })
  })

  describe("AgentCapabilities", () => {
    test("accepts minimal capabilities", () => {
      const result = BootstrapTypes.AgentCapabilities.parse({
        name: "build",
        tools: ["bash", "read"],
        skills: [],
        mcpServers: [],
        permissions: {},
      })
      expect(result.name).toBe("build")
      expect(result.tools).toHaveLength(2)
    })

    test("accepts full capabilities", () => {
      const result = BootstrapTypes.AgentCapabilities.parse({
        name: "autonomous",
        description: "Autonomous agent",
        tools: ["bash", "read", "edit"],
        skills: ["tdd", "debugging"],
        mcpServers: ["github", "slack"],
        permissions: { bash: true, edit: true },
        model: { providerID: "anthropic", modelID: "claude-3-opus" },
      })
      expect(result.description).toBe("Autonomous agent")
      expect(result.model?.providerID).toBe("anthropic")
    })
  })

  describe("CanHandleResult", () => {
    test("accepts confident result", () => {
      const result = BootstrapTypes.CanHandleResult.parse({
        confident: true,
        confidence: 0.9,
      })
      expect(result.confident).toBe(true)
    })

    test("accepts result with missing capabilities", () => {
      const result = BootstrapTypes.CanHandleResult.parse({
        confident: false,
        confidence: 0.3,
        missingCapabilities: ["mcp:github", "skill:tdd"],
        suggestedResources: ["github-mcp-server"],
      })
      expect(result.missingCapabilities).toHaveLength(2)
    })
  })

  describe("TestScenario", () => {
    test("accepts scenario without result", () => {
      const result = BootstrapTypes.TestScenario.parse({
        id: "scenario_1",
        name: "Basic test",
        description: "Test basic functionality",
        input: "test input",
        expectedBehavior: "should work",
      })
      expect(result.result).toBeUndefined()
    })

    test("accepts scenario with result", () => {
      const result = BootstrapTypes.TestScenario.parse({
        id: "scenario_1",
        name: "Basic test",
        description: "Test basic functionality",
        input: "test input",
        expectedBehavior: "should work",
        result: {
          passed: true,
          actual: "it worked",
        },
      })
      expect(result.result?.passed).toBe(true)
    })

    test("accepts failed scenario with error", () => {
      const result = BootstrapTypes.TestScenario.parse({
        id: "scenario_1",
        name: "Failing test",
        description: "Test error handling",
        input: "bad input",
        expectedBehavior: "should fail gracefully",
        result: {
          passed: false,
          error: "Unexpected error",
        },
      })
      expect(result.result?.passed).toBe(false)
      expect(result.result?.error).toBe("Unexpected error")
    })
  })

  describe("VerificationResult", () => {
    test("accepts passed result", () => {
      const result = BootstrapTypes.VerificationResult.parse({
        passed: true,
        confidence: 0.85,
        scenarios: [],
      })
      expect(result.passed).toBe(true)
    })

    test("accepts failed result with corrections", () => {
      const result = BootstrapTypes.VerificationResult.parse({
        passed: false,
        confidence: 0.4,
        scenarios: [],
        error: "Test failed",
        corrections: "Fix the implementation",
      })
      expect(result.error).toBe("Test failed")
      expect(result.corrections).toBe("Fix the implementation")
    })
  })

  describe("ConfidenceFactors", () => {
    test("accepts minimal factors", () => {
      const result = BootstrapTypes.ConfidenceFactors.parse({
        verificationPassed: true,
        usageCount: 5,
        successRate: 0.8,
        scenarioCoverage: 0.6,
      })
      expect(result.verificationPassed).toBe(true)
    })

    test("accepts all factors", () => {
      const result = BootstrapTypes.ConfidenceFactors.parse({
        verificationPassed: true,
        usageCount: 10,
        successRate: 0.9,
        scenarioCoverage: 0.8,
        codeQuality: 0.85,
        userFeedback: 0.5,
      })
      expect(result.codeQuality).toBe(0.85)
    })

    test("validates rate ranges", () => {
      expect(() =>
        BootstrapTypes.ConfidenceFactors.parse({
          verificationPassed: true,
          usageCount: 5,
          successRate: 1.5, // > 1
          scenarioCoverage: 0.6,
        }),
      ).toThrow()
    })

    test("allows negative user feedback", () => {
      const result = BootstrapTypes.ConfidenceFactors.parse({
        verificationPassed: false,
        usageCount: 2,
        successRate: 0.5,
        scenarioCoverage: 0.3,
        userFeedback: -0.8,
      })
      expect(result.userFeedback).toBe(-0.8)
    })
  })
})
