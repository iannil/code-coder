import { describe, test, expect } from "bun:test"
import { classifyTask, createTaskClassifier } from "../../../src/autonomous/classification/task-classifier"

describe("TaskClassifier", () => {
  describe("rule-based classification", () => {
    test("classifies research keywords as research", async () => {
      const result = await classifyTask("梳理当前的黄金走势情况")
      expect(result.type).toBe("research")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("classifies implementation keywords as implementation", async () => {
      const result = await classifyTask("实现一个用户登录功能")
      expect(result.type).toBe("implementation")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("classifies query keywords as query", async () => {
      const result = await classifyTask("什么是 TypeScript")
      expect(result.type).toBe("query")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("classifies decision keywords as decision (Chinese)", async () => {
      const result = await classifyTask("用CLOSE框架评估这个职业选择")
      expect(result.type).toBe("decision")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("classifies decision keywords as decision (English)", async () => {
      const result = await classifyTask("Evaluate the trade-offs between these options using CLOSE framework")
      expect(result.type).toBe("decision")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("classifies pros cons analysis as decision", async () => {
      const result = await classifyTask("Compare and weigh the options for the career decision")
      expect(result.type).toBe("decision")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("extracts research topic", async () => {
      const result = await classifyTask("分析今年的比特币走势")
      expect(result.type).toBe("research")
      expect(result.researchTopic).toBeDefined()
    })
  })

  describe("factory function", () => {
    test("creates classifier with config", () => {
      const classifier = createTaskClassifier({ useLLMFallback: false })
      expect(classifier).toBeDefined()
      expect(classifier.classify).toBeInstanceOf(Function)
    })
  })
})
