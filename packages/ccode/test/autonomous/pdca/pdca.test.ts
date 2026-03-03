/**
 * PDCA Framework Unit Tests
 */

import { describe, it, expect, beforeEach, mock } from "bun:test"

describe("PDCA Framework", () => {
  describe("types", () => {
    it("should export all required types", async () => {
      const types = await import("@/autonomous/pdca/types")

      // Core types should exist
      expect(types.DEFAULT_PDCA_CONFIG).toBeDefined()
      expect(types.PDCAIssueSchema).toBeDefined()
      expect(types.CheckItemResultSchema).toBeDefined()
      expect(types.PDCACheckResultSchema).toBeDefined()
      expect(types.PDCAActResultSchema).toBeDefined()
      expect(types.PDCACycleResultSchema).toBeDefined()
    })

    it("should have correct DEFAULT_PDCA_CONFIG values", async () => {
      const { DEFAULT_PDCA_CONFIG } = await import("@/autonomous/pdca/types")

      expect(DEFAULT_PDCA_CONFIG.maxCycles).toBe(3)
      expect(DEFAULT_PDCA_CONFIG.passThreshold).toBe(6.0)
      expect(DEFAULT_PDCA_CONFIG.fixThreshold).toBe(4.0)
      expect(DEFAULT_PDCA_CONFIG.enableFix).toBe(true)
      expect(DEFAULT_PDCA_CONFIG.enableLearning).toBe(true)
    })
  })

  describe("strategies", () => {
    it("should export StrategyFactory", async () => {
      const { StrategyFactory } = await import("@/autonomous/pdca/strategies")

      expect(StrategyFactory).toBeDefined()
      expect(StrategyFactory.create).toBeDefined()
      expect(StrategyFactory.clearCache).toBeDefined()
      expect(StrategyFactory.isSupported).toBeDefined()
      expect(StrategyFactory.getSupportedTypes).toBeDefined()
    })

    it("should support all required task types", async () => {
      const { StrategyFactory } = await import("@/autonomous/pdca/strategies")

      expect(StrategyFactory.isSupported("implementation")).toBe(true)
      expect(StrategyFactory.isSupported("research")).toBe(true)
      expect(StrategyFactory.isSupported("query")).toBe(true)
      expect(StrategyFactory.isSupported("other")).toBe(true)
    })

    it("should create correct strategy for each task type", async () => {
      const { StrategyFactory } = await import("@/autonomous/pdca/strategies")

      const implStrategy = StrategyFactory.create("implementation", false)
      expect(implStrategy.taskType).toBe("implementation")
      expect(implStrategy.name).toContain("Implementation")

      const researchStrategy = StrategyFactory.create("research", false)
      expect(researchStrategy.taskType).toBe("research")
      expect(researchStrategy.name).toContain("Research")

      const queryStrategy = StrategyFactory.create("query", false)
      expect(queryStrategy.taskType).toBe("query")
      expect(queryStrategy.name).toContain("Query")

      const genericStrategy = StrategyFactory.create("other", false)
      expect(genericStrategy.taskType).toBe("other")
      expect(genericStrategy.name).toContain("Generic")
    })

    it("should cache strategies by default", async () => {
      const { StrategyFactory } = await import("@/autonomous/pdca/strategies")

      StrategyFactory.clearCache()

      const strategy1 = StrategyFactory.create("research")
      const strategy2 = StrategyFactory.create("research")

      expect(strategy1).toBe(strategy2) // Same instance
    })

    it("should return different strategies when cache disabled", async () => {
      const { StrategyFactory } = await import("@/autonomous/pdca/strategies")

      const strategy1 = StrategyFactory.create("research", false)
      const strategy2 = StrategyFactory.create("research", false)

      expect(strategy1).not.toBe(strategy2) // Different instances
    })
  })

  describe("ResearchStrategy", () => {
    it("should define check items", async () => {
      const { createResearchStrategy } = await import("@/autonomous/pdca/strategies/research")

      const strategy = createResearchStrategy()
      const checkItems = strategy.getCheckItems()

      expect(checkItems).toContain("source_credibility")
      expect(checkItems).toContain("coverage")
      expect(checkItems).toContain("freshness")
      expect(checkItems).toContain("accuracy")
      expect(checkItems).toContain("insight_quality")
    })

    it("should define weights for check items", async () => {
      const { createResearchStrategy } = await import("@/autonomous/pdca/strategies/research")

      const strategy = createResearchStrategy()
      const weights = strategy.getCheckWeights()

      expect(weights.source_credibility).toBeGreaterThan(0)
      expect(weights.accuracy).toBeGreaterThan(weights.source_credibility) // Accuracy has higher weight
    })
  })

  describe("QueryStrategy", () => {
    it("should define check items", async () => {
      const { createQueryStrategy } = await import("@/autonomous/pdca/strategies/query")

      const strategy = createQueryStrategy()
      const checkItems = strategy.getCheckItems()

      expect(checkItems).toContain("relevance")
      expect(checkItems).toContain("completeness")
      expect(checkItems).toContain("accuracy")
      expect(checkItems).toContain("clarity")
    })
  })

  describe("UnifiedPDCAController", () => {
    it("should create controller with correct config", async () => {
      const { createPDCAController } = await import("@/autonomous/pdca")

      const controller = createPDCAController({
        taskType: "research",
        sessionId: "test-session",
        maxCycles: 2,
        passThreshold: 7.0,
      })

      expect(controller.getCycles()).toBe(0)
      expect(controller.getConfig().maxCycles).toBe(2)
      expect(controller.getConfig().passThreshold).toBe(7.0)
      expect(controller.getStrategy().taskType).toBe("research")
    })

    it("should execute PDCA cycle with mock do function", async () => {
      const { createPDCAController } = await import("@/autonomous/pdca")

      const controller = createPDCAController({
        taskType: "query",
        sessionId: "test-session",
        maxCycles: 1,
        passThreshold: 5.0, // Low threshold for testing
      })

      const mockDoFn = async () => ({
        taskType: "query" as const,
        success: true,
        output: {
          answer: "Test answer with sufficient content to pass basic checks.",
          confidence: 0.9,
          context: ["source1", "source2"],
        },
        durationMs: 500,
      })

      const result = await controller.execute(mockDoFn, "Test query")

      // Should complete at least one cycle
      expect(result.cycles).toBeGreaterThanOrEqual(1)
      expect(result.result).toBeDefined()
      expect(result.checkResult).toBeDefined()
    })
  })

  describe("module exports", () => {
    it("should export all required items from pdca/index", async () => {
      const pdca = await import("@/autonomous/pdca")

      // Types
      expect(pdca.DEFAULT_PDCA_CONFIG).toBeDefined()

      // Controller
      expect(pdca.UnifiedPDCAController).toBeDefined()
      expect(pdca.createPDCAController).toBeDefined()

      // Strategies
      expect(pdca.BaseAcceptanceStrategy).toBeDefined()
      expect(pdca.StrategyFactory).toBeDefined()
      expect(pdca.createStrategy).toBeDefined()
      expect(pdca.createImplementationStrategy).toBeDefined()
      expect(pdca.createResearchStrategy).toBeDefined()
      expect(pdca.createQueryStrategy).toBeDefined()
      expect(pdca.createGenericStrategy).toBeDefined()
    })

    it("should export PDCA from autonomous/index", async () => {
      const autonomous = await import("@/autonomous")

      expect(autonomous.createPDCAController).toBeDefined()
      expect(autonomous.StrategyFactory).toBeDefined()
    })
  })
})
