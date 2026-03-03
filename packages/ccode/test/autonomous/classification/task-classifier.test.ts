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
