import { describe, test, expect } from "bun:test"
import { createResearchLearner, type LearnedResearchPattern } from "../../../src/autonomous/execution/research-learner"

describe("ResearchLearner", () => {
  test("detects periodic pattern from multiple similar requests", () => {
    const learner = createResearchLearner()

    // Simulate multiple similar requests
    learner.recordResearch({ topic: "黄金走势", keywords: ["黄金", "走势"], sources: ["yahoo"] })
    learner.recordResearch({ topic: "黄金走势", keywords: ["黄金", "行情"], sources: ["yahoo"] })
    learner.recordResearch({ topic: "黄金走势", keywords: ["黄金", "价格"], sources: ["yahoo"] })

    const patterns = learner.getPatterns()
    expect(patterns.length).toBeGreaterThan(0)

    const goldPattern = patterns.find(p => p.topic.includes("黄金"))
    expect(goldPattern).toBeDefined()
    expect(goldPattern?.confidence).toBeGreaterThan(0.5)
  })

  test("suggests Hand creation for periodic tasks", () => {
    const learner = createResearchLearner()

    // Simulate daily pattern
    for (let i = 0; i < 5; i++) {
      learner.recordResearch({
        topic: "每日财经新闻",
        keywords: ["财经", "新闻"],
        sources: ["yahoo"],
        timestamp: new Date(Date.now() - i * 24 * 60 * 60 * 1000).toISOString(),
      })
    }

    const suggestion = learner.suggestHandCreation("每日财经新闻")
    expect(suggestion).toBeDefined()
    expect(suggestion?.frequency).toBe("daily")
  })

  test("merges similar keywords across research sessions", () => {
    const learner = createResearchLearner()

    learner.recordResearch({ topic: "股票分析", keywords: ["股票", "A股"] })
    learner.recordResearch({ topic: "股票分析", keywords: ["股票", "大盘"] })

    const pattern = learner.getPattern("股票分析")
    expect(pattern?.keywords).toContain("股票")
    expect(pattern?.keywords.length).toBeGreaterThanOrEqual(2)
  })
})
