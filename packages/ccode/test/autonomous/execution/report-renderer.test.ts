import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { renderReport, createReportRenderer } from "../../../src/autonomous/execution/report-renderer"
import { existsSync, rmSync, mkdirSync } from "fs"
import { join } from "path"

const TEST_DIR = "/tmp/test-reports"

describe("ReportRenderer", () => {
  beforeEach(() => {
    if (existsSync(TEST_DIR)) rmSync(TEST_DIR, { recursive: true })
    mkdirSync(TEST_DIR, { recursive: true })
  })

  afterEach(() => {
    if (existsSync(TEST_DIR)) rmSync(TEST_DIR, { recursive: true })
  })

  test("renders short report inline", async () => {
    const result = await renderReport({
      topic: "Test Topic",
      summary: "Short summary",
      analysis: "Brief analysis",
      insights: ["insight 1"],
      sources: [],
    }, { maxInlineLength: 1000, outputDir: TEST_DIR })

    expect(result.mode).toBe("inline")
    expect(result.content).toContain("Test Topic")
    expect(result.filePath).toBeUndefined()
  })

  test("saves long report to file", async () => {
    const longAnalysis = "x".repeat(1500)
    const result = await renderReport({
      topic: "Long Topic",
      summary: "Summary",
      analysis: longAnalysis,
      insights: [],
      sources: [],
    }, { maxInlineLength: 1000, outputDir: TEST_DIR })

    expect(result.mode).toBe("file")
    expect(result.filePath).toBeDefined()
    expect(existsSync(result.filePath!)).toBe(true)
  })

  test("factory creates renderer with config", () => {
    const renderer = createReportRenderer({ maxInlineLength: 500 })
    expect(renderer.render).toBeInstanceOf(Function)
  })
})
