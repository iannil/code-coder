/**
 * Causal Graph Unit Tests
 *
 * Tests for the Causal Graph system (Phase 16: 因果链图数据库)
 */

import { describe, it, expect, beforeEach, mock, spyOn } from "bun:test"
import { CausalGraph } from "@/memory/knowledge/causal-graph"
import { CausalAnalysis } from "@/memory/knowledge/causal-analysis"
import { Storage } from "@/storage/storage"
import type {
  CausalGraphData,
  DecisionNode,
  ActionNode,
  OutcomeNode,
} from "@/memory/knowledge/causal-types"

// ============================================================================
// Mock Setup
// ============================================================================

let mockGraph: CausalGraphData

function createEmptyGraph(): CausalGraphData {
  return {
    projectId: "test-project",
    nodes: {
      decisions: [],
      actions: [],
      outcomes: [],
    },
    edges: [],
    adjacencyMap: {
      outgoing: {},
      incoming: {},
    },
    time: {
      created: Date.now(),
      updated: Date.now(),
    },
  }
}

// Mock Storage
const mockStorage = {
  read: mock(() => Promise.resolve(mockGraph)),
  write: mock(() => Promise.resolve()),
  remove: mock(() => Promise.resolve()),
}

// Mock Instance
mock.module("@/project/instance", () => ({
  Instance: {
    project: { id: "test-project" },
  },
}))

mock.module("@/storage/storage", () => ({
  Storage: mockStorage,
}))

// ============================================================================
// CausalGraph Tests
// ============================================================================

describe("CausalGraph", () => {
  beforeEach(() => {
    mockGraph = createEmptyGraph()
    mockStorage.read.mockClear()
    mockStorage.write.mockClear()
    mockStorage.remove.mockClear()
  })

  describe("Storage Operations", () => {
    it("should get existing graph", async () => {
      mockGraph.nodes.decisions.push({
        id: "dec_1",
        type: "decision",
        sessionId: "session_1",
        agentId: "@decision",
        prompt: "Test prompt",
        reasoning: "Test reasoning",
        confidence: 0.9,
        timestamp: new Date().toISOString(),
      })

      const result = await CausalGraph.get()
      expect(result).toBeDefined()
      expect(result?.nodes.decisions.length).toBe(1)
    })

    it("should create new graph when none exists", async () => {
      mockStorage.read.mockImplementationOnce(() => {
        throw new Error("Not found")
      })

      const result = await CausalGraph.load()
      expect(result).toBeDefined()
      expect(result.projectId).toBe("test-project")
      expect(mockStorage.write).toHaveBeenCalled()
    })

    it("should save graph with updated timestamp", async () => {
      const graph = createEmptyGraph()
      const originalUpdated = graph.time.updated

      // Wait a bit to ensure timestamp changes
      await new Promise((r) => setTimeout(r, 5))

      await CausalGraph.save(graph)
      expect(mockStorage.write).toHaveBeenCalled()
    })

    it("should invalidate graph", async () => {
      await CausalGraph.invalidate()
      expect(mockStorage.remove).toHaveBeenCalledWith([
        "memory",
        "knowledge",
        "causal-graph",
        "test-project",
      ])
    })
  })

  describe("Record Operations", () => {
    it("should record a decision", async () => {
      const decision = await CausalGraph.recordDecision({
        sessionId: "session_1",
        agentId: "@decision",
        prompt: "Should we refactor this code?",
        reasoning: "The code is complex and needs simplification",
        confidence: 0.85,
        context: {
          files: ["src/main.ts"],
          tools: ["Read", "Grep"],
        },
      })

      expect(decision).toBeDefined()
      expect(decision.id).toMatch(/^dec_/)
      expect(decision.type).toBe("decision")
      expect(decision.agentId).toBe("@decision")
      expect(decision.confidence).toBe(0.85)
      expect(mockStorage.write).toHaveBeenCalled()
    })

    it("should record an action linked to a decision", async () => {
      // First record a decision
      const decision: DecisionNode = {
        id: "dec_test",
        type: "decision",
        sessionId: "session_1",
        agentId: "@decision",
        prompt: "Test",
        reasoning: "Test reasoning",
        confidence: 0.9,
        timestamp: new Date().toISOString(),
      }
      mockGraph.nodes.decisions.push(decision)
      mockGraph.adjacencyMap.outgoing["dec_test"] = []
      mockGraph.adjacencyMap.incoming["dec_test"] = []

      const action = await CausalGraph.recordAction({
        decisionId: "dec_test",
        actionType: "code_change",
        description: "Refactored the main function",
        input: { file: "src/main.ts" },
        output: { linesChanged: 42 },
        duration: 5000,
      })

      expect(action).toBeDefined()
      expect(action.id).toMatch(/^act_/)
      expect(action.type).toBe("action")
      expect(action.decisionId).toBe("dec_test")
      expect(action.actionType).toBe("code_change")
    })

    it("should throw error when recording action for non-existent decision", async () => {
      await expect(
        CausalGraph.recordAction({
          decisionId: "non_existent",
          actionType: "code_change",
          description: "Test",
          input: {},
        }),
      ).rejects.toThrow('Decision "non_existent" not found')
    })

    it("should record an outcome linked to an action", async () => {
      // Setup decision and action
      mockGraph.nodes.decisions.push({
        id: "dec_test",
        type: "decision",
        sessionId: "session_1",
        agentId: "@decision",
        prompt: "Test",
        reasoning: "Test",
        confidence: 0.9,
        timestamp: new Date().toISOString(),
      })

      mockGraph.nodes.actions.push({
        id: "act_test",
        type: "action",
        decisionId: "dec_test",
        actionType: "code_change",
        description: "Test action",
        input: {},
        timestamp: new Date().toISOString(),
      })
      mockGraph.adjacencyMap.outgoing["act_test"] = []
      mockGraph.adjacencyMap.incoming["act_test"] = ["dec_test"]

      const outcome = await CausalGraph.recordOutcome({
        actionId: "act_test",
        status: "success",
        description: "All tests passed",
        metrics: {
          testsPass: 42,
          testsFail: 0,
          coverageChange: 5,
        },
        feedback: "Good refactoring!",
      })

      expect(outcome).toBeDefined()
      expect(outcome.id).toMatch(/^out_/)
      expect(outcome.type).toBe("outcome")
      expect(outcome.actionId).toBe("act_test")
      expect(outcome.status).toBe("success")
    })

    it("should throw error when recording outcome for non-existent action", async () => {
      await expect(
        CausalGraph.recordOutcome({
          actionId: "non_existent",
          status: "success",
          description: "Test",
        }),
      ).rejects.toThrow('Action "non_existent" not found')
    })
  })

  describe("Query Operations", () => {
    beforeEach(() => {
      // Setup a complete causal chain
      mockGraph.nodes.decisions.push({
        id: "dec_1",
        type: "decision",
        sessionId: "session_1",
        agentId: "@decision",
        prompt: "Implement feature X",
        reasoning: "User requested feature X",
        confidence: 0.9,
        timestamp: "2026-02-24T10:00:00Z",
      })

      mockGraph.nodes.actions.push({
        id: "act_1",
        type: "action",
        decisionId: "dec_1",
        actionType: "code_change",
        description: "Created feature X implementation",
        input: { files: ["feature.ts"] },
        timestamp: "2026-02-24T10:01:00Z",
      })

      mockGraph.nodes.outcomes.push({
        id: "out_1",
        type: "outcome",
        actionId: "act_1",
        status: "success",
        description: "Feature X implemented successfully",
        timestamp: "2026-02-24T10:02:00Z",
      })

      mockGraph.edges.push(
        {
          id: "dec_1->act_1",
          source: "dec_1",
          target: "act_1",
          relationship: "causes",
          weight: 0.9,
        },
        {
          id: "act_1->out_1",
          source: "act_1",
          target: "out_1",
          relationship: "results_in",
          weight: 1.0,
        },
      )

      mockGraph.adjacencyMap.outgoing = {
        dec_1: ["act_1"],
        act_1: ["out_1"],
        out_1: [],
      }
      mockGraph.adjacencyMap.incoming = {
        dec_1: [],
        act_1: ["dec_1"],
        out_1: ["act_1"],
      }
    })

    it("should get complete causal chain", async () => {
      const chain = await CausalGraph.getCausalChain("dec_1")

      expect(chain).toBeDefined()
      expect(chain?.decision.id).toBe("dec_1")
      expect(chain?.actions.length).toBe(1)
      expect(chain?.outcomes.length).toBe(1)
      expect(chain?.edges.length).toBe(2)
    })

    it("should return null for non-existent decision", async () => {
      const chain = await CausalGraph.getCausalChain("non_existent")
      expect(chain).toBeNull()
    })

    it("should get chains for session", async () => {
      const chains = await CausalGraph.getCausalChainsForSession("session_1")
      expect(chains.length).toBe(1)
      expect(chains[0].decision.sessionId).toBe("session_1")
    })

    it("should query by agent", async () => {
      const chains = await CausalGraph.query({ agentId: "@decision" })
      expect(chains.length).toBe(1)
    })

    it("should query by minimum confidence", async () => {
      const chains = await CausalGraph.query({ minConfidence: 0.95 })
      expect(chains.length).toBe(0)
    })

    it("should query by date range", async () => {
      const chains = await CausalGraph.query({
        dateFrom: "2026-02-24T00:00:00Z",
        dateTo: "2026-02-24T23:59:59Z",
      })
      expect(chains.length).toBe(1)
    })

    it("should get decision by ID", async () => {
      const decision = await CausalGraph.getDecision("dec_1")
      expect(decision?.id).toBe("dec_1")
    })

    it("should get action by ID", async () => {
      const action = await CausalGraph.getAction("act_1")
      expect(action?.id).toBe("act_1")
    })

    it("should get outcome by ID", async () => {
      const outcome = await CausalGraph.getOutcome("out_1")
      expect(outcome?.id).toBe("out_1")
    })

    it("should get decisions by agent", async () => {
      const decisions = await CausalGraph.getDecisionsByAgent("@decision")
      expect(decisions.length).toBe(1)
    })
  })

  describe("Statistics", () => {
    beforeEach(() => {
      // Setup multiple decisions with different outcomes
      for (let i = 0; i < 5; i++) {
        mockGraph.nodes.decisions.push({
          id: `dec_${i}`,
          type: "decision",
          sessionId: "session_1",
          agentId: i < 3 ? "@decision" : "@macro",
          prompt: `Decision ${i}`,
          reasoning: "Reasoning",
          confidence: 0.8,
          timestamp: new Date().toISOString(),
        })

        mockGraph.nodes.actions.push({
          id: `act_${i}`,
          type: "action",
          decisionId: `dec_${i}`,
          actionType: i % 2 === 0 ? "code_change" : "tool_execution",
          description: `Action ${i}`,
          input: {},
          timestamp: new Date().toISOString(),
        })

        mockGraph.nodes.outcomes.push({
          id: `out_${i}`,
          type: "outcome",
          actionId: `act_${i}`,
          status: i < 3 ? "success" : "failure",
          description: `Outcome ${i}`,
          timestamp: new Date().toISOString(),
        })

        mockGraph.adjacencyMap.outgoing[`dec_${i}`] = [`act_${i}`]
        mockGraph.adjacencyMap.outgoing[`act_${i}`] = [`out_${i}`]
        mockGraph.adjacencyMap.incoming[`act_${i}`] = [`dec_${i}`]
        mockGraph.adjacencyMap.incoming[`out_${i}`] = [`act_${i}`]
      }
    })

    it("should get comprehensive stats", async () => {
      const stats = await CausalGraph.getStats()

      expect(stats.totalDecisions).toBe(5)
      expect(stats.totalActions).toBe(5)
      expect(stats.totalOutcomes).toBe(5)
      expect(stats.successRate).toBe(0.6) // 3 out of 5
      expect(stats.avgConfidence).toBe(0.8)
      expect(stats.topAgents.length).toBe(2)
    })

    it("should calculate success rate for agent", async () => {
      const successRate = await CausalGraph.getSuccessRate("@decision")
      expect(successRate).toBe(1.0) // All 3 @decision outcomes are success
    })

    it("should return 0 for agent with no decisions", async () => {
      const successRate = await CausalGraph.getSuccessRate("@nonexistent")
      expect(successRate).toBe(0)
    })
  })

  describe("Visualization", () => {
    beforeEach(() => {
      mockGraph.nodes.decisions.push({
        id: "dec_1",
        type: "decision",
        sessionId: "session_1",
        agentId: "@decision",
        prompt: "Test decision prompt",
        reasoning: "Test reasoning",
        confidence: 0.9,
        timestamp: new Date().toISOString(),
      })

      mockGraph.nodes.actions.push({
        id: "act_1",
        type: "action",
        decisionId: "dec_1",
        actionType: "code_change",
        description: "Test action description",
        input: {},
        timestamp: new Date().toISOString(),
      })

      mockGraph.nodes.outcomes.push({
        id: "out_1",
        type: "outcome",
        actionId: "act_1",
        status: "success",
        description: "Test outcome description",
        timestamp: new Date().toISOString(),
      })

      mockGraph.adjacencyMap.outgoing = {
        dec_1: ["act_1"],
        act_1: ["out_1"],
        out_1: [],
      }
    })

    it("should generate Mermaid diagram", async () => {
      const mermaid = await CausalGraph.toMermaid()

      expect(mermaid).toContain("graph TD")
      expect(mermaid).toContain("dec_1")
      expect(mermaid).toContain("act_1")
      expect(mermaid).toContain("out_1")
      expect(mermaid).toContain("-->|causes|")
      expect(mermaid).toContain("-->|results_in|")
    })

    it("should limit nodes in Mermaid diagram", async () => {
      // Add more nodes
      for (let i = 2; i <= 100; i++) {
        mockGraph.nodes.decisions.push({
          id: `dec_${i}`,
          type: "decision",
          sessionId: "session_1",
          agentId: "@decision",
          prompt: `Decision ${i}`,
          reasoning: "Reasoning",
          confidence: 0.8,
          timestamp: new Date().toISOString(),
        })
      }

      const mermaid = await CausalGraph.toMermaid({ maxNodes: 10 })
      const decisionCount = (mermaid.match(/dec_\d+/g) || []).length

      expect(decisionCount).toBeLessThanOrEqual(20) // Some overlap in edge definitions
    })

    it("should filter Mermaid by decision ID", async () => {
      mockGraph.nodes.decisions.push({
        id: "dec_2",
        type: "decision",
        sessionId: "session_1",
        agentId: "@macro",
        prompt: "Other decision",
        reasoning: "Reasoning",
        confidence: 0.7,
        timestamp: new Date().toISOString(),
      })

      const mermaid = await CausalGraph.toMermaid({ decisionId: "dec_1" })
      expect(mermaid).toContain("dec_1")
      expect(mermaid).not.toContain("dec_2")
    })
  })
})

// ============================================================================
// CausalAnalysis Tests
// ============================================================================

describe("CausalAnalysis", () => {
  beforeEach(() => {
    mockGraph = createEmptyGraph()
    mockStorage.read.mockClear()
    mockStorage.write.mockClear()

    // Setup multiple causal chains for pattern analysis
    for (let i = 0; i < 10; i++) {
      const agentId = i < 6 ? "@decision" : "@macro"
      const actionType = i % 3 === 0 ? "code_change" : i % 3 === 1 ? "tool_execution" : "search"
      const status = i < 7 ? "success" : "failure"

      mockGraph.nodes.decisions.push({
        id: `dec_${i}`,
        type: "decision",
        sessionId: `session_${Math.floor(i / 2)}`,
        agentId,
        prompt: `Decision ${i}: Should we ${actionType === "code_change" ? "refactor" : "search for"} something?`,
        reasoning: `Reasoning for decision ${i}`,
        confidence: 0.7 + i * 0.03,
        timestamp: new Date(Date.now() - (10 - i) * 86400000).toISOString(),
      })

      mockGraph.nodes.actions.push({
        id: `act_${i}`,
        type: "action",
        decisionId: `dec_${i}`,
        actionType: actionType as any,
        description: `Action ${i}`,
        input: { index: i },
        timestamp: new Date(Date.now() - (10 - i) * 86400000 + 1000).toISOString(),
      })

      mockGraph.nodes.outcomes.push({
        id: `out_${i}`,
        type: "outcome",
        actionId: `act_${i}`,
        status: status as any,
        description: `Outcome ${i}`,
        timestamp: new Date(Date.now() - (10 - i) * 86400000 + 2000).toISOString(),
      })

      mockGraph.edges.push(
        {
          id: `dec_${i}->act_${i}`,
          source: `dec_${i}`,
          target: `act_${i}`,
          relationship: "causes",
          weight: mockGraph.nodes.decisions[i].confidence,
        },
        {
          id: `act_${i}->out_${i}`,
          source: `act_${i}`,
          target: `out_${i}`,
          relationship: "results_in",
          weight: status === "success" ? 1.0 : 0.0,
        },
      )

      mockGraph.adjacencyMap.outgoing[`dec_${i}`] = [`act_${i}`]
      mockGraph.adjacencyMap.outgoing[`act_${i}`] = [`out_${i}`]
      mockGraph.adjacencyMap.outgoing[`out_${i}`] = []
      mockGraph.adjacencyMap.incoming[`dec_${i}`] = []
      mockGraph.adjacencyMap.incoming[`act_${i}`] = [`dec_${i}`]
      mockGraph.adjacencyMap.incoming[`out_${i}`] = [`act_${i}`]
    }
  })

  describe("Pattern Recognition", () => {
    it("should find patterns", async () => {
      const patterns = await CausalAnalysis.findPatterns({ minOccurrences: 1 })
      expect(patterns.length).toBeGreaterThan(0)
    })

    it("should find patterns for specific agent", async () => {
      const patterns = await CausalAnalysis.findPatterns({
        agentId: "@decision",
        minOccurrences: 1,
      })

      for (const pattern of patterns) {
        expect(pattern.agentId).toBe("@decision")
      }
    })

    it("should find success patterns", async () => {
      const patterns = await CausalAnalysis.findSuccessPatterns({
        minSuccessRate: 0.5,
        minOccurrences: 1,
      })

      for (const pattern of patterns) {
        expect(pattern.successRate).toBeGreaterThanOrEqual(0.5)
      }
    })

    it("should find failure patterns", async () => {
      const patterns = await CausalAnalysis.findFailurePatterns({
        maxSuccessRate: 0.5,
        minOccurrences: 1,
      })

      for (const pattern of patterns) {
        expect(pattern.successRate).toBeLessThanOrEqual(0.5)
      }
    })
  })

  describe("Suggestion Generation", () => {
    it("should generate suggestions based on history", async () => {
      const suggestions = await CausalAnalysis.suggestFromHistory({
        prompt: "Should we refactor the code?",
        agentId: "@decision",
      })

      // May or may not have suggestions depending on similarity
      expect(Array.isArray(suggestions)).toBe(true)
    })

    it("should sort suggestions by confidence", async () => {
      const suggestions = await CausalAnalysis.suggestFromHistory({
        prompt: "refactor something",
        agentId: "@decision",
      })

      for (let i = 1; i < suggestions.length; i++) {
        expect(suggestions[i - 1].confidence).toBeGreaterThanOrEqual(suggestions[i].confidence)
      }
    })
  })

  describe("Trend Analysis", () => {
    it("should analyze trends", async () => {
      const trends = await CausalAnalysis.analyzeTrends({ periodDays: 7 })

      expect(trends.totalDecisions).toBe(10)
      expect(trends.successRateTrend).toHaveLength(2)
      expect(trends.confidenceTrend).toHaveLength(2)
      expect(typeof trends.actionTypeShifts).toBe("object")
    })

    it("should analyze trends for specific agent", async () => {
      const trends = await CausalAnalysis.analyzeTrends({
        agentId: "@decision",
        periodDays: 7,
      })

      expect(trends.totalDecisions).toBeLessThanOrEqual(10)
    })
  })

  describe("Lesson Extraction", () => {
    it("should extract lessons from outcome", async () => {
      const lesson = await CausalAnalysis.extractLessons("out_0")

      expect(lesson).toBeDefined()
      expect(lesson?.status).toBe("success")
      expect(lesson?.lesson).toContain("code_change")
    })

    it("should return null for non-existent outcome", async () => {
      const lesson = await CausalAnalysis.extractLessons("non_existent")
      expect(lesson).toBeNull()
    })
  })

  describe("Agent Insights", () => {
    it("should get agent insights", async () => {
      const insights = await CausalAnalysis.getAgentInsights("@decision")

      expect(insights.totalDecisions).toBe(6)
      expect(typeof insights.successRate).toBe("number")
      expect(["improving", "declining", "stable"]).toContain(insights.recentTrend)
      expect(Array.isArray(insights.suggestions)).toBe(true)
    })

    it("should handle agent with no data", async () => {
      const insights = await CausalAnalysis.getAgentInsights("@nonexistent")

      expect(insights.totalDecisions).toBe(0)
      expect(insights.successRate).toBe(0)
      expect(insights.recentTrend).toBe("stable")
      expect(insights.suggestions.length).toBeGreaterThan(0)
    })
  })
})

// ============================================================================
// Integration Tests
// ============================================================================

describe("CausalGraph Integration", () => {
  beforeEach(() => {
    mockGraph = createEmptyGraph()
    mockStorage.read.mockClear()
    mockStorage.write.mockClear()
  })

  it("should record complete causal chain", async () => {
    // Record decision
    const decision = await CausalGraph.recordDecision({
      sessionId: "integration_session",
      agentId: "@decision",
      prompt: "Should we implement caching?",
      reasoning: "Performance is slow due to repeated API calls",
      confidence: 0.85,
      context: {
        files: ["src/api/client.ts"],
        tools: ["Read", "Grep"],
      },
    })

    // Update mock graph with the recorded decision
    mockGraph.nodes.decisions.push(decision)
    mockGraph.adjacencyMap.outgoing[decision.id] = []
    mockGraph.adjacencyMap.incoming[decision.id] = []

    // Record action
    const action = await CausalGraph.recordAction({
      decisionId: decision.id,
      actionType: "code_change",
      description: "Added Redis caching layer",
      input: {
        files: ["src/cache/redis.ts", "src/api/client.ts"],
        linesAdded: 150,
      },
      duration: 30000,
    })

    // Update mock graph
    mockGraph.nodes.actions.push(action)
    mockGraph.adjacencyMap.outgoing[action.id] = []
    mockGraph.adjacencyMap.incoming[action.id] = [decision.id]
    mockGraph.adjacencyMap.outgoing[decision.id].push(action.id)
    mockGraph.edges.push({
      id: `${decision.id}->${action.id}`,
      source: decision.id,
      target: action.id,
      relationship: "causes",
      weight: 0.85,
    })

    // Record outcome
    const outcome = await CausalGraph.recordOutcome({
      actionId: action.id,
      status: "success",
      description: "API response time reduced by 80%",
      metrics: {
        testsPass: 45,
        testsFail: 0,
        linesAdded: 150,
        filesModified: 2,
      },
      feedback: "Great improvement!",
    })

    // Verify chain
    expect(decision.id).toMatch(/^dec_/)
    expect(action.decisionId).toBe(decision.id)
    expect(outcome.actionId).toBe(action.id)
    expect(outcome.status).toBe("success")
  })

  it("should track failure patterns", async () => {
    // Record a failed chain
    const decision: DecisionNode = {
      id: "dec_fail",
      type: "decision",
      sessionId: "fail_session",
      agentId: "@decision",
      prompt: "Try risky refactoring",
      reasoning: "Quick fix attempt",
      confidence: 0.3,
      timestamp: new Date().toISOString(),
    }
    mockGraph.nodes.decisions.push(decision)
    mockGraph.adjacencyMap.outgoing[decision.id] = []
    mockGraph.adjacencyMap.incoming[decision.id] = []

    const action: ActionNode = {
      id: "act_fail",
      type: "action",
      decisionId: decision.id,
      actionType: "code_change",
      description: "Aggressive code deletion",
      input: { linesDeleted: 500 },
      timestamp: new Date().toISOString(),
    }
    mockGraph.nodes.actions.push(action)
    mockGraph.adjacencyMap.outgoing[action.id] = []
    mockGraph.adjacencyMap.incoming[action.id] = [decision.id]
    mockGraph.adjacencyMap.outgoing[decision.id].push(action.id)

    const outcome: OutcomeNode = {
      id: "out_fail",
      type: "outcome",
      actionId: action.id,
      status: "failure",
      description: "Build broken, tests failing",
      metrics: { testsPass: 0, testsFail: 42 },
      timestamp: new Date().toISOString(),
    }
    mockGraph.nodes.outcomes.push(outcome)
    mockGraph.adjacencyMap.outgoing[outcome.id] = []
    mockGraph.adjacencyMap.incoming[outcome.id] = [action.id]
    mockGraph.adjacencyMap.outgoing[action.id].push(outcome.id)

    // Get stats
    const stats = await CausalGraph.getStats()
    expect(stats.totalDecisions).toBe(1)
    expect(stats.totalOutcomes).toBe(1)
    expect(stats.successRate).toBe(0) // 0 successes out of 1
  })
})
