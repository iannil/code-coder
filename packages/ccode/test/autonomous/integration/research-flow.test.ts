import { describe, test, expect } from "bun:test"
import { classifyTask } from "../../../src/autonomous/classification"
import { createResearchLoop } from "../../../src/autonomous/execution/research-loop"

describe("Research Flow Integration", () => {
  test("classifies and routes research task correctly", async () => {
    // Step 1: Classify
    const classification = await classifyTask("梳理当前的黄金走势情况")
    expect(classification.type).toBe("research")
    expect(classification.confidence).toBeGreaterThan(0.5)

    // Step 2: Execute research (with mock to avoid network)
    const loop = createResearchLoop({
      maxSources: 2,
      enableLearning: false,
    })

    const result = await loop.research({
      sessionId: "test-integration",
      topic: classification.researchTopic ?? "黄金走势",
      maxSources: 2,
    })

    expect(result).toHaveProperty("topic")
    expect(result).toHaveProperty("durationMs")

    await loop.cleanup()
  })

  test("classifies implementation task correctly", async () => {
    const classification = await classifyTask("实现一个用户登录功能")
    expect(classification.type).toBe("implementation")
  })

  test("classifies query task correctly", async () => {
    const classification = await classifyTask("什么是 TypeScript")
    expect(classification.type).toBe("query")
  })

  test("extracts research topic from message", async () => {
    const classification = await classifyTask("分析今年的比特币走势")
    expect(classification.type).toBe("research")
    expect(classification.researchTopic).toBeDefined()
    expect(classification.researchTopic).toContain("比特币")
  })

  test("routes different task types correctly", async () => {
    const testCases = [
      { message: "梳理当前的黄金走势情况", expectedType: "research" },
      { message: "分析今年的经济趋势", expectedType: "research" },
      { message: "总结一下最近的市场行情", expectedType: "research" },
      { message: "实现一个用户登录功能", expectedType: "implementation" },
      { message: "创建一个新的API接口", expectedType: "implementation" },
      { message: "什么是TypeScript", expectedType: "query" },
      { message: "为什么React这么流行", expectedType: "query" },
    ]

    for (const tc of testCases) {
      const classification = await classifyTask(tc.message)
      expect(classification.type).toBe(tc.expectedType)
    }
  })
})
