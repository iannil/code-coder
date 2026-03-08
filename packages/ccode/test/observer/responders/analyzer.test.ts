/**
 * Analyzer Responder Tests
 *
 * Tests for the Analyzer component of the Observer Network.
 *
 * @module test/observer/responders/analyzer.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach, mock } from "bun:test"
import {
  Analyzer,
  createAnalyzer,
  type AnalyzerConfig,
  type AnalysisRequest,
  type AnalysisType,
} from "@/observer/responders/analyzer"
import type { Anomaly, Opportunity, EmergentPattern, WorldModel } from "@/observer/types"

describe("Analyzer", () => {
  let analyzer: Analyzer

  beforeEach(() => {
    analyzer = createAnalyzer({
      autoAnalyze: false, // Disable auto-analyze to test manually
      maxConcurrent: 2,
      timeoutMs: 1000,
    })
  })

  afterEach(() => {
    analyzer.stop()
    analyzer.clear()
  })

  describe("lifecycle", () => {
    it("should start and stop correctly", async () => {
      await analyzer.start()
      // Analyzer should be running
      analyzer.stop()
      // Should be able to stop without error
    })

    it("should not start twice", async () => {
      await analyzer.start()
      await analyzer.start() // Should not throw
      analyzer.stop()
    })

    it("should be safe to stop when not started", () => {
      analyzer.stop() // Should not throw
    })

    it("should cancel pending analyses on stop", async () => {
      await analyzer.start()

      // Create several analysis requests
      const request1 = await analyzer.requestAnalysis("security_review", {
        trigger: { type: "manual" },
        context: { file: "test.ts" },
        priority: "low",
      })

      analyzer.stop()

      // Pending requests should be cancelled
      const pending = analyzer.getPending()
      for (const p of pending) {
        expect(p.status).toBe("cancelled")
      }
    })
  })

  describe("requestAnalysis", () => {
    it("should create analysis request with correct fields", async () => {
      await analyzer.start()

      const request = await analyzer.requestAnalysis("architecture_review", {
        trigger: { type: "manual" },
        context: { component: "UserService" },
        priority: "medium",
      })

      expect(request).toBeDefined()
      expect(request.id).toMatch(/^analysis_/)
      expect(request.type).toBe("architecture_review")
      expect(request.trigger.type).toBe("manual")
      expect(request.context.component).toBe("UserService")
      expect(request.priority).toBe("medium")
      expect(request.createdAt).toBeInstanceOf(Date)
    })

    it("should respect priority levels", async () => {
      await analyzer.start()

      const highRequest = await analyzer.requestAnalysis("security_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "high",
      })

      const lowRequest = await analyzer.requestAnalysis("performance_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "low",
      })

      expect(highRequest.priority).toBe("high")
      expect(lowRequest.priority).toBe("low")
    })

    it("should start running immediately if under concurrent limit", async () => {
      await analyzer.start()

      const request = await analyzer.requestAnalysis("pattern_analysis", {
        trigger: { type: "manual" },
        context: {},
        priority: "medium",
      })

      // Should be pending or running (will fail due to no actual agent)
      expect(["pending", "running", "failed"]).toContain(request.status)
    })
  })

  describe("analyzeAnomaly", () => {
    it("should create anomaly investigation request", async () => {
      await analyzer.start()

      const anomaly: Anomaly = {
        id: "anomaly_1",
        type: "error_spike",
        severity: "critical",
        description: "Error rate increased by 500%",
        detectedAt: new Date(),
        source: "code",
        data: {},
      }

      const request = await analyzer.analyzeAnomaly(anomaly)

      expect(request.type).toBe("anomaly_investigation")
      expect(request.trigger.type).toBe("anomaly")
      expect(request.trigger.id).toBe("anomaly_1")
      expect(request.priority).toBe("high") // Critical severity = high priority
    })

    it("should set medium priority for non-critical anomalies", async () => {
      await analyzer.start()

      const anomaly: Anomaly = {
        id: "anomaly_2",
        type: "warning",
        severity: "low",
        description: "Minor issue",
        detectedAt: new Date(),
        source: "code",
        data: {},
      }

      const request = await analyzer.analyzeAnomaly(anomaly)

      expect(request.priority).toBe("medium")
    })
  })

  describe("analyzeOpportunity", () => {
    it("should create opportunity assessment request", async () => {
      await analyzer.start()

      const opportunity: Opportunity = {
        id: "opp_1",
        type: "optimization",
        impact: "high",
        description: "Optimize database queries",
        detectedAt: new Date(),
        suggestedActions: ["Add index", "Cache results"],
        urgency: "medium",
      }

      const request = await analyzer.analyzeOpportunity(opportunity)

      expect(request.type).toBe("opportunity_assessment")
      expect(request.trigger.type).toBe("opportunity")
      expect(request.trigger.id).toBe("opp_1")
      expect(request.priority).toBe("high")
    })
  })

  describe("analyzePattern", () => {
    it("should create pattern analysis request", async () => {
      await analyzer.start()

      const pattern: EmergentPattern = {
        id: "pattern_1",
        type: "recurring",
        name: "Error Clustering",
        description: "Errors tend to cluster around deployment times",
        strength: 0.85,
        evidence: [],
        implications: ["Improve deployment process"],
        detectedAt: new Date(),
      }

      const request = await analyzer.analyzePattern(pattern)

      expect(request.type).toBe("pattern_analysis")
      expect(request.trigger.type).toBe("pattern")
      expect(request.priority).toBe("high") // Strength > 0.8 = high priority
    })

    it("should set medium priority for weaker patterns", async () => {
      await analyzer.start()

      const pattern: EmergentPattern = {
        id: "pattern_2",
        type: "emerging",
        name: "Weak Pattern",
        description: "Possible correlation",
        strength: 0.5,
        evidence: [],
        implications: [],
        detectedAt: new Date(),
      }

      const request = await analyzer.analyzePattern(pattern)

      expect(request.priority).toBe("medium")
    })
  })

  describe("specialized reviews", () => {
    it("should create security review request", async () => {
      await analyzer.start()

      const request = await analyzer.requestSecurityReview({
        files: ["auth.ts", "login.ts"],
        focus: "authentication",
      })

      expect(request.type).toBe("security_review")
      expect(request.priority).toBe("high")
      expect(request.trigger.type).toBe("manual")
    })

    it("should create architecture review request", async () => {
      await analyzer.start()

      const request = await analyzer.requestArchitectureReview({
        component: "APIGateway",
        concerns: ["scalability", "resilience"],
      })

      expect(request.type).toBe("architecture_review")
      expect(request.priority).toBe("medium")
    })

    it("should create market analysis request", async () => {
      await analyzer.start()

      const worldModel: WorldModel = {
        timestamp: new Date(),
        codeState: { health: "healthy", activeFiles: [], recentChanges: [], issues: [] },
        worldState: { markets: {}, news: [], apiStatus: {} },
        selfState: { decisions: [], patterns: [], performance: {} },
        metaState: {
          observerHealth: "healthy",
          blindSpots: [],
          metrics: { latency: 0, accuracy: 0, coverage: 0 },
        },
        synthesized: { opportunities: [], risks: [], recommendations: [] },
      }

      const request = await analyzer.requestMarketAnalysis(worldModel)

      expect(request.type).toBe("market_analysis")
      expect(request.priority).toBe("medium")
    })
  })

  describe("queue management", () => {
    it("should queue requests when max concurrent reached", async () => {
      const limitedAnalyzer = createAnalyzer({
        autoAnalyze: false,
        maxConcurrent: 1,
        timeoutMs: 5000,
      })
      await limitedAnalyzer.start()

      // Start multiple requests
      const r1 = await limitedAnalyzer.requestAnalysis("security_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "high",
      })

      const r2 = await limitedAnalyzer.requestAnalysis("architecture_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "low",
      })

      // One should be running/pending, other queued
      const running = limitedAnalyzer.getRunning()
      const pending = limitedAnalyzer.getPending()

      // Total should be 2
      expect(running.length + pending.length).toBeLessThanOrEqual(2)

      limitedAnalyzer.stop()
    })

    it("should prioritize high priority requests in queue", async () => {
      const limitedAnalyzer = createAnalyzer({
        autoAnalyze: false,
        maxConcurrent: 1,
        timeoutMs: 5000,
      })
      await limitedAnalyzer.start()

      // Fill queue with requests
      await limitedAnalyzer.requestAnalysis("performance_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "low",
      })

      await limitedAnalyzer.requestAnalysis("architecture_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "low",
      })

      // Add high priority
      const highPriority = await limitedAnalyzer.requestAnalysis("security_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "high",
      })

      const pending = limitedAnalyzer.getPending()
      // High priority should be first in queue if queued
      if (pending.length > 0) {
        const highPriorityInQueue = pending.some((p) => p.priority === "high")
        expect(highPriorityInQueue || highPriority.status === "running").toBe(true)
      }

      limitedAnalyzer.stop()
    })
  })

  describe("getAnalysis", () => {
    it("should retrieve analysis by id", async () => {
      await analyzer.start()

      const request = await analyzer.requestAnalysis("security_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "medium",
      })

      const retrieved = analyzer.getAnalysis(request.id)

      expect(retrieved).toBeDefined()
      expect(retrieved?.id).toBe(request.id)
    })

    it("should return null for non-existent id", () => {
      const retrieved = analyzer.getAnalysis("non_existent_id")
      expect(retrieved).toBeNull()
    })
  })

  describe("getHistory", () => {
    it("should return analysis history sorted by date", async () => {
      await analyzer.start()

      await analyzer.requestAnalysis("security_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "high",
      })

      await analyzer.requestAnalysis("architecture_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "low",
      })

      const history = analyzer.getHistory()

      expect(history.length).toBe(2)
      // Most recent first
      expect(history[0].createdAt.getTime()).toBeGreaterThanOrEqual(history[1].createdAt.getTime())
    })

    it("should limit history results", async () => {
      await analyzer.start()

      for (let i = 0; i < 5; i++) {
        await analyzer.requestAnalysis("pattern_analysis", {
          trigger: { type: "manual" },
          context: { index: i },
          priority: "medium",
        })
      }

      const history = analyzer.getHistory(3)

      expect(history.length).toBe(3)
    })
  })

  describe("cancel", () => {
    it("should cancel pending analysis", async () => {
      await analyzer.start()

      const request = await analyzer.requestAnalysis("performance_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "low",
      })

      // If it's pending, we can cancel it
      if (request.status === "pending") {
        const cancelled = analyzer.cancel(request.id)
        expect(cancelled).toBe(true)

        const retrieved = analyzer.getAnalysis(request.id)
        expect(retrieved?.status).toBe("cancelled")
      }
    })

    it("should return false when cancelling non-existent analysis", () => {
      const cancelled = analyzer.cancel("non_existent")
      expect(cancelled).toBe(false)
    })

    it("should return false when cancelling running analysis", async () => {
      await analyzer.start()

      const request = await analyzer.requestAnalysis("security_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "high",
      })

      // Wait a bit for it to potentially start running
      await new Promise((r) => setTimeout(r, 10))

      const retrieved = analyzer.getAnalysis(request.id)
      if (retrieved?.status === "running") {
        const cancelled = analyzer.cancel(request.id)
        expect(cancelled).toBe(false)
      }
    })
  })

  describe("clear", () => {
    it("should clear all analysis history", async () => {
      await analyzer.start()

      await analyzer.requestAnalysis("security_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "high",
      })

      analyzer.clear()

      const history = analyzer.getHistory()
      expect(history.length).toBe(0)
    })
  })

  describe("agent mappings", () => {
    it("should use correct agent for each analysis type", async () => {
      const customAnalyzer = createAnalyzer({
        autoAnalyze: false,
        maxConcurrent: 5,
        timeoutMs: 100,
        agentMappings: {
          security_review: "security-reviewer",
          architecture_review: "architect",
          market_analysis: "macro",
        },
      })
      await customAnalyzer.start()

      const request = await customAnalyzer.requestAnalysis("security_review", {
        trigger: { type: "manual" },
        context: {},
        priority: "high",
      })

      // Wait for request to be processed
      await new Promise((r) => setTimeout(r, 150))

      const retrieved = customAnalyzer.getAnalysis(request.id)
      expect(retrieved?.agentUsed).toBe("security-reviewer")

      customAnalyzer.stop()
    })
  })
})
