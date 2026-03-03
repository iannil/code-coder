import { describe, test, expect, mock } from "bun:test"
import { createResearchLoop } from "../../../src/autonomous/execution/research-loop"

describe("ResearchLoop", () => {
  test("creates research loop with config", () => {
    const loop = createResearchLoop({ maxSources: 5 })
    expect(loop).toBeDefined()
    expect(loop.research).toBeInstanceOf(Function)
    expect(loop.cleanup).toBeInstanceOf(Function)
  })

  test("research returns result structure", async () => {
    const loop = createResearchLoop({
      maxSources: 3,
      enableLearning: false,
    })

    // Mock the actual research to avoid network calls
    const result = await loop.research({
      sessionId: "test-session",
      topic: "测试主题",
      maxSources: 2,
    })

    expect(result).toHaveProperty("success")
    expect(result).toHaveProperty("topic")
    expect(result).toHaveProperty("summary")
    expect(result).toHaveProperty("report")
    expect(result).toHaveProperty("sources")
    expect(result).toHaveProperty("insights")
    expect(result).toHaveProperty("durationMs")
  })
})
