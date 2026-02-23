// @ts-nocheck
/**
 * Unit Tests for CausalRecorder
 *
 * Tests the causal recording hook that integrates agent decisions
 * with the CausalGraph database.
 *
 * Part of Phase 18: Agent 因果链集成
 */

import { describe, test, expect, beforeEach, afterEach, mock, spyOn } from "bun:test"
import { CausalRecorder } from "@/agent/hooks/causal-recorder"
import { CausalGraph } from "@/memory/knowledge/causal-graph"
import { CausalAnalysis } from "@/memory/knowledge/causal-analysis"

describe("CausalRecorder", () => {
  beforeEach(() => {
    // Clear all tracking data before each test
    CausalRecorder.clearAll()
  })

  describe("recordAgentDecision", () => {
    test("should record a decision and return decision ID", async () => {
      const mockDecision = {
        id: "dec_123456_abc",
        type: "decision" as const,
        sessionId: "session-1",
        agentId: "decision",
        prompt: "Should I invest in stocks?",
        reasoning: "CLOSE analysis: High optionality, moderate risk",
        confidence: 0.85,
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)

      const decisionId = await CausalRecorder.recordAgentDecision({
        sessionId: "session-1",
        agentId: "decision",
        prompt: "Should I invest in stocks?",
        reasoning: "CLOSE analysis: High optionality, moderate risk",
        confidence: 0.85,
      })

      expect(decisionId).toBe("dec_123456_abc")
      expect(recordDecisionSpy).toHaveBeenCalledWith({
        sessionId: "session-1",
        agentId: "decision",
        prompt: "Should I invest in stocks?",
        reasoning: "CLOSE analysis: High optionality, moderate risk",
        confidence: 0.85,
        context: undefined,
      })

      recordDecisionSpy.mockRestore()
    })

    test("should track active decision for session", async () => {
      const mockDecision = {
        id: "dec_789_xyz",
        type: "decision" as const,
        sessionId: "session-2",
        agentId: "decision",
        prompt: "Career change?",
        reasoning: "Analysis...",
        confidence: 0.7,
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-2",
        agentId: "decision",
        prompt: "Career change?",
        reasoning: "Analysis...",
        confidence: 0.7,
      })

      const activeId = CausalRecorder.getActiveDecisionId("session-2")
      expect(activeId).toBe("dec_789_xyz")

      recordDecisionSpy.mockRestore()
    })

    test("should include context when provided", async () => {
      const mockDecision = {
        id: "dec_context_test",
        type: "decision" as const,
        sessionId: "session-3",
        agentId: "decision",
        prompt: "Test prompt",
        reasoning: "Test reasoning",
        confidence: 0.9,
        timestamp: new Date().toISOString(),
        context: {
          files: ["file1.ts", "file2.ts"],
          tools: ["read", "grep"],
        },
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-3",
        agentId: "decision",
        prompt: "Test prompt",
        reasoning: "Test reasoning",
        confidence: 0.9,
        context: {
          files: ["file1.ts", "file2.ts"],
          tools: ["read", "grep"],
        },
      })

      expect(recordDecisionSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          context: {
            files: ["file1.ts", "file2.ts"],
            tools: ["read", "grep"],
          },
        }),
      )

      recordDecisionSpy.mockRestore()
    })
  })

  describe("recordToolAction", () => {
    test("should return null when no active decision exists", async () => {
      const actionId = await CausalRecorder.recordToolAction({
        sessionId: "session-no-decision",
        toolName: "read",
        toolInput: { file_path: "/test/file.ts" },
      })

      expect(actionId).toBeNull()
    })

    test("should record action when decision exists", async () => {
      // First, set up an active decision
      const mockDecision = {
        id: "dec_for_action",
        type: "decision" as const,
        sessionId: "session-action",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
        timestamp: new Date().toISOString(),
      }

      const mockAction = {
        id: "act_123",
        type: "action" as const,
        decisionId: "dec_for_action",
        actionType: "file_operation" as const,
        description: "read: test/file.ts",
        input: { file_path: "/test/file.ts" },
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)
      const recordActionSpy = spyOn(CausalGraph, "recordAction").mockResolvedValue(mockAction)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-action",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
      })

      const actionId = await CausalRecorder.recordToolAction({
        sessionId: "session-action",
        toolName: "read",
        toolInput: { file_path: "/test/file.ts" },
      })

      expect(actionId).toBe("act_123")
      expect(recordActionSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          decisionId: "dec_for_action",
          actionType: "file_operation",
        }),
      )

      recordDecisionSpy.mockRestore()
      recordActionSpy.mockRestore()
    })

    test("should map tool names to correct action types", async () => {
      const mockDecision = {
        id: "dec_mapping_test",
        type: "decision" as const,
        sessionId: "session-mapping",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-mapping",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
      })

      const toolMappings = [
        { tool: "write", expectedType: "file_operation" },
        { tool: "edit", expectedType: "file_operation" },
        { tool: "read", expectedType: "file_operation" },
        { tool: "grep", expectedType: "search" },
        { tool: "glob", expectedType: "search" },
        { tool: "bash", expectedType: "tool_execution" },
        { tool: "webfetch", expectedType: "api_call" },
        { tool: "websearch", expectedType: "search" },
      ]

      for (const { tool, expectedType } of toolMappings) {
        const mockAction = {
          id: `act_${tool}`,
          type: "action" as const,
          decisionId: "dec_mapping_test",
          actionType: expectedType as any,
          description: `${tool}: test`,
          input: { test: true },
          timestamp: new Date().toISOString(),
        }

        const recordActionSpy = spyOn(CausalGraph, "recordAction").mockResolvedValue(mockAction)

        await CausalRecorder.recordToolAction({
          sessionId: "session-mapping",
          toolName: tool,
          toolInput: { test: true },
        })

        expect(recordActionSpy).toHaveBeenCalledWith(
          expect.objectContaining({
            actionType: expectedType,
          }),
        )

        recordActionSpy.mockRestore()
      }

      recordDecisionSpy.mockRestore()
    })

    test("should include output when provided", async () => {
      const mockDecision = {
        id: "dec_output_test",
        type: "decision" as const,
        sessionId: "session-output",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
        timestamp: new Date().toISOString(),
      }

      const mockAction = {
        id: "act_output",
        type: "action" as const,
        decisionId: "dec_output_test",
        actionType: "file_operation" as const,
        description: "read: test.ts",
        input: { file_path: "/test.ts" },
        output: { result: "file content here" },
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)
      const recordActionSpy = spyOn(CausalGraph, "recordAction").mockResolvedValue(mockAction)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-output",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
      })

      await CausalRecorder.recordToolAction({
        sessionId: "session-output",
        toolName: "read",
        toolInput: { file_path: "/test.ts" },
        toolOutput: "file content here",
      })

      expect(recordActionSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          output: { result: "file content here" },
        }),
      )

      recordDecisionSpy.mockRestore()
      recordActionSpy.mockRestore()
    })
  })

  describe("recordOutcome", () => {
    test("should record success outcome", async () => {
      const recordOutcomeSpy = spyOn(CausalGraph, "recordOutcome").mockResolvedValue({
        id: "out_success",
        type: "outcome" as const,
        actionId: "act_123",
        status: "success",
        description: "Completed successfully",
        timestamp: new Date().toISOString(),
      })

      await CausalRecorder.recordOutcome({
        actionId: "act_123",
        success: true,
      })

      expect(recordOutcomeSpy).toHaveBeenCalledWith({
        actionId: "act_123",
        status: "success",
        description: "Completed successfully",
        metrics: undefined,
      })

      recordOutcomeSpy.mockRestore()
    })

    test("should record failure outcome with error", async () => {
      const recordOutcomeSpy = spyOn(CausalGraph, "recordOutcome").mockResolvedValue({
        id: "out_failure",
        type: "outcome" as const,
        actionId: "act_456",
        status: "failure",
        description: "File not found",
        timestamp: new Date().toISOString(),
      })

      await CausalRecorder.recordOutcome({
        actionId: "act_456",
        success: false,
        error: "File not found",
      })

      expect(recordOutcomeSpy).toHaveBeenCalledWith({
        actionId: "act_456",
        status: "failure",
        description: "File not found",
        metrics: undefined,
      })

      recordOutcomeSpy.mockRestore()
    })

    test("should include metrics when provided", async () => {
      const recordOutcomeSpy = spyOn(CausalGraph, "recordOutcome").mockResolvedValue({
        id: "out_metrics",
        type: "outcome" as const,
        actionId: "act_789",
        status: "success",
        description: "Completed successfully",
        timestamp: new Date().toISOString(),
        metrics: {
          filesModified: 3,
          testsPass: 10,
          testsFail: 0,
        },
      })

      await CausalRecorder.recordOutcome({
        actionId: "act_789",
        success: true,
        metrics: {
          filesModified: 3,
          testsPass: 10,
          testsFail: 0,
        },
      })

      expect(recordOutcomeSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          metrics: {
            filesModified: 3,
            testsPass: 10,
            testsFail: 0,
          },
        }),
      )

      recordOutcomeSpy.mockRestore()
    })
  })

  describe("getSuggestions", () => {
    test("should return formatted suggestions", async () => {
      const mockSuggestions = [
        {
          id: "sug_1",
          type: "similar_decision" as const,
          confidence: 0.85,
          reasoning: "Similar decision succeeded with file_operation approach",
          basedOn: ["dec_1"],
        },
        {
          id: "sug_2",
          type: "avoid_pattern" as const,
          confidence: 0.7,
          reasoning: "Similar decision failed with bash approach",
          basedOn: ["dec_2"],
        },
      ]

      const suggestSpy = spyOn(CausalAnalysis, "suggestFromHistory").mockResolvedValue(mockSuggestions)

      const suggestions = await CausalRecorder.getSuggestions({
        agentId: "decision",
        prompt: "Should I invest?",
      })

      expect(suggestions).toHaveLength(2)
      expect(suggestions[0]).toEqual({
        id: "sug_1",
        type: "similar_decision",
        confidence: 0.85,
        reasoning: "Similar decision succeeded with file_operation approach",
      })

      suggestSpy.mockRestore()
    })

    test("should return empty array when no suggestions", async () => {
      const suggestSpy = spyOn(CausalAnalysis, "suggestFromHistory").mockResolvedValue([])

      const suggestions = await CausalRecorder.getSuggestions({
        agentId: "decision",
        prompt: "Unique question",
      })

      expect(suggestions).toHaveLength(0)

      suggestSpy.mockRestore()
    })
  })

  describe("getHistory", () => {
    test("should format chains for display", async () => {
      const mockChains = [
        {
          decision: {
            id: "dec_1",
            type: "decision" as const,
            sessionId: "session-1",
            agentId: "decision",
            prompt: "Should I change careers?",
            reasoning: "CLOSE analysis...",
            confidence: 0.8,
            timestamp: new Date().toISOString(),
          },
          actions: [
            {
              id: "act_1",
              type: "action" as const,
              decisionId: "dec_1",
              actionType: "file_operation" as const,
              description: "read: resume.txt",
              input: {},
              timestamp: new Date().toISOString(),
            },
          ],
          outcomes: [
            {
              id: "out_1",
              type: "outcome" as const,
              actionId: "act_1",
              status: "success" as const,
              description: "Completed",
              timestamp: new Date().toISOString(),
            },
          ],
          edges: [],
        },
      ]

      const querySpy = spyOn(CausalGraph, "query").mockResolvedValue(mockChains)

      const history = await CausalRecorder.getHistory({ agentId: "decision", limit: 5 })

      expect(history).toContain("Should I change careers?")
      expect(history).toContain("decision")
      expect(history).toContain("80%")
      expect(history).toContain("1 成功")

      querySpy.mockRestore()
    })

    test("should return message when no history", async () => {
      const querySpy = spyOn(CausalGraph, "query").mockResolvedValue([])

      const history = await CausalRecorder.getHistory({})

      expect(history).toBe("暂无决策历史记录。")

      querySpy.mockRestore()
    })
  })

  describe("session management", () => {
    test("should clear session tracking", async () => {
      const mockDecision = {
        id: "dec_clear_test",
        type: "decision" as const,
        sessionId: "session-clear",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-clear",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
      })

      expect(CausalRecorder.getActiveDecisionId("session-clear")).toBe("dec_clear_test")

      CausalRecorder.clearSession("session-clear")

      expect(CausalRecorder.getActiveDecisionId("session-clear")).toBeUndefined()

      recordDecisionSpy.mockRestore()
    })

    test("should clear all tracking data", async () => {
      const mockDecision1 = {
        id: "dec_all_1",
        type: "decision" as const,
        sessionId: "session-1",
        agentId: "decision",
        prompt: "Test1",
        reasoning: "Test",
        confidence: 0.8,
        timestamp: new Date().toISOString(),
      }

      const mockDecision2 = {
        id: "dec_all_2",
        type: "decision" as const,
        sessionId: "session-2",
        agentId: "decision",
        prompt: "Test2",
        reasoning: "Test",
        confidence: 0.8,
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision")
        .mockResolvedValueOnce(mockDecision1)
        .mockResolvedValueOnce(mockDecision2)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-1",
        agentId: "decision",
        prompt: "Test1",
        reasoning: "Test",
        confidence: 0.8,
      })

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-2",
        agentId: "decision",
        prompt: "Test2",
        reasoning: "Test",
        confidence: 0.8,
      })

      expect(CausalRecorder.getActiveDecisionId("session-1")).toBe("dec_all_1")
      expect(CausalRecorder.getActiveDecisionId("session-2")).toBe("dec_all_2")

      CausalRecorder.clearAll()

      expect(CausalRecorder.getActiveDecisionId("session-1")).toBeUndefined()
      expect(CausalRecorder.getActiveDecisionId("session-2")).toBeUndefined()

      recordDecisionSpy.mockRestore()
    })
  })

  describe("input summarization", () => {
    test("should summarize file_path input", async () => {
      const mockDecision = {
        id: "dec_summary_1",
        type: "decision" as const,
        sessionId: "session-summary",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
        timestamp: new Date().toISOString(),
      }

      const mockAction = {
        id: "act_summary_1",
        type: "action" as const,
        decisionId: "dec_summary_1",
        actionType: "file_operation" as const,
        description: "read: deep/nested/file.ts",
        input: { file_path: "/very/deep/nested/file.ts" },
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)
      const recordActionSpy = spyOn(CausalGraph, "recordAction").mockResolvedValue(mockAction)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-summary",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
      })

      await CausalRecorder.recordToolAction({
        sessionId: "session-summary",
        toolName: "read",
        toolInput: { file_path: "/very/deep/nested/file.ts" },
      })

      // The description should contain a summarized file path
      expect(recordActionSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          description: expect.stringContaining("read:"),
        }),
      )

      recordDecisionSpy.mockRestore()
      recordActionSpy.mockRestore()
    })

    test("should truncate long command input", async () => {
      const mockDecision = {
        id: "dec_cmd_1",
        type: "decision" as const,
        sessionId: "session-cmd",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
        timestamp: new Date().toISOString(),
      }

      const mockAction = {
        id: "act_cmd_1",
        type: "action" as const,
        decisionId: "dec_cmd_1",
        actionType: "tool_execution" as const,
        description: "bash: npm run test --coverage --watch...",
        input: { command: "npm run test --coverage --watch --passWithNoTests --verbose" },
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)
      const recordActionSpy = spyOn(CausalGraph, "recordAction").mockResolvedValue(mockAction)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-cmd",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
      })

      await CausalRecorder.recordToolAction({
        sessionId: "session-cmd",
        toolName: "bash",
        toolInput: { command: "npm run test --coverage --watch --passWithNoTests --verbose" },
      })

      expect(recordActionSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          description: expect.stringContaining("bash:"),
        }),
      )

      recordDecisionSpy.mockRestore()
      recordActionSpy.mockRestore()
    })
  })

  describe("MCP tool handling", () => {
    test("should map MCP tools to tool_execution type", async () => {
      const mockDecision = {
        id: "dec_mcp_1",
        type: "decision" as const,
        sessionId: "session-mcp",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
        timestamp: new Date().toISOString(),
      }

      const mockAction = {
        id: "act_mcp_1",
        type: "action" as const,
        decisionId: "dec_mcp_1",
        actionType: "tool_execution" as const,
        description: "mcp__custom__tool: test",
        input: { param: "value" },
        timestamp: new Date().toISOString(),
      }

      const recordDecisionSpy = spyOn(CausalGraph, "recordDecision").mockResolvedValue(mockDecision)
      const recordActionSpy = spyOn(CausalGraph, "recordAction").mockResolvedValue(mockAction)

      await CausalRecorder.recordAgentDecision({
        sessionId: "session-mcp",
        agentId: "decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.8,
      })

      await CausalRecorder.recordToolAction({
        sessionId: "session-mcp",
        toolName: "mcp__custom__tool",
        toolInput: { param: "value" },
      })

      expect(recordActionSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          actionType: "tool_execution",
        }),
      )

      recordDecisionSpy.mockRestore()
      recordActionSpy.mockRestore()
    })
  })
})
