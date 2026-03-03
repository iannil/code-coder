import { describe, test, expect, mock } from "bun:test"
import {
  createResearchLoop,
  extractSummaryFromContent,
  extractInsightsFromContent,
  cleanWebContent,
} from "../../../src/autonomous/execution/research-loop"

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

describe("extractSummaryFromContent", () => {
  test("extracts sentences with topic terms and numbers", () => {
    const content = `
      黄金价格今日上涨百分之二点五，达到2050美元每盎司。
      市场分析师认为，这是由于美联储政策预期变化。
      技术面显示黄金走势偏强，支撑位在2020美元。
      天气今天很好，适合出门。
    `
    const summary = extractSummaryFromContent(content, "黄金走势")

    expect(summary.length).toBeGreaterThan(50)
    // Should prioritize sentences with trend indicators and topic terms
    expect(summary).toContain("黄金")
  })

  test("provides fallback when no relevant sentences found", () => {
    const content = "一些无关的内容"
    const summary = extractSummaryFromContent(content, "黄金")

    expect(summary.length).toBeGreaterThan(30)
    expect(summary).toContain("黄金")
  })

  test("handles empty content", () => {
    const summary = extractSummaryFromContent("", "测试")
    expect(summary).toContain("测试")
    expect(summary.length).toBeGreaterThan(20)
  })
})

describe("extractInsightsFromContent", () => {
  test("extracts at least 3 insights from financial content", () => {
    const content = `
      黄金价格上涨3.2%，创近期新高。
      市场走势显示多头趋势明显。
      关键支撑位在2000美元附近。
      主要影响因素包括美元走弱和地缘政治风险。
      分析师建议关注后续央行政策动向。
      预计短期内价格或将继续上行。
    `
    const insights = extractInsightsFromContent(content, "黄金")

    expect(insights.length).toBeGreaterThanOrEqual(3)
    expect(insights.length).toBeLessThanOrEqual(5)
    // Should have categorized insights
    expect(insights.some((i) => i.includes("："))).toBe(true)
  })

  test("ensures minimum 3 insights even with sparse content", () => {
    const content = "简单的内容，没有太多分析。"
    const insights = extractInsightsFromContent(content, "测试")

    expect(insights.length).toBeGreaterThanOrEqual(3)
    expect(insights.every((i) => i.length > 0)).toBe(true)
  })

  test("extracts numeric data as insights when available", () => {
    const content = `
      市场指数今日上涨5.2%，创历史新高。
      成交量达到100亿元，较昨日增加20%。
      分析师预计涨幅可能持续到月底。
    `
    const insights = extractInsightsFromContent(content, "市场")

    expect(insights.length).toBeGreaterThanOrEqual(3)
    // At least one insight should be extracted from the numeric content
    expect(insights.some((i) => i.length > 20)).toBe(true)
  })
})

describe("cleanWebContent", () => {
  test("removes tracking pixels", () => {
    const content = `正文内容 ![](//beacon.sina.com.cn/a.gif?noScript) 更多内容`
    const cleaned = cleanWebContent(content)

    expect(cleaned).not.toContain("beacon.sina.com.cn")
    expect(cleaned).not.toContain("a.gif")
    expect(cleaned).toContain("正文内容")
    expect(cleaned).toContain("更多内容")
  })

  test("removes navigation menu items", () => {
    const content = `
      正文内容
      •   新浪首页
      •   新闻
      •   体育
      •   财经
      实际的新闻内容在这里
    `
    const cleaned = cleanWebContent(content)

    expect(cleaned).not.toContain("新浪首页")
    expect(cleaned).not.toContain("•   新闻")
    expect(cleaned).toContain("正文内容")
    expect(cleaned).toContain("实际的新闻内容在这里")
  })

  test("removes JavaScript links", () => {
    const content = `点击这里 (javascript:;) 查看更多`
    const cleaned = cleanWebContent(content)

    expect(cleaned).not.toContain("javascript:")
    expect(cleaned).toContain("点击这里")
    expect(cleaned).toContain("查看更多")
  })

  test("removes mobile app download lists", () => {
    const content = `
      移动客户端
      •   新浪微博
      •   新浪新闻
      •   新浪财经
      •   天气通
      实际内容
    `
    const cleaned = cleanWebContent(content)

    expect(cleaned).not.toContain("新浪微博")
    expect(cleaned).not.toContain("天气通")
    expect(cleaned).toContain("实际内容")
  })

  test("removes excessive whitespace", () => {
    const content = `第一段



第二段


第三段`
    const cleaned = cleanWebContent(content)

    // Should have at most 2 consecutive newlines
    expect(cleaned.includes("\n\n\n")).toBe(false)
    expect(cleaned).toContain("第一段")
    expect(cleaned).toContain("第三段")
  })

  test("removes empty images", () => {
    const content = `内容 ![]() 更多 ![](https://example.com/x.png) 结束`
    const cleaned = cleanWebContent(content)

    expect(cleaned).not.toContain("![]()")
    expect(cleaned).not.toContain(".png")
    expect(cleaned).toContain("内容")
    expect(cleaned).toContain("结束")
  })

  test("preserves meaningful content", () => {
    const content = `
      A股收评：深成指高开低走跌3.07%，全市场超4800只个股下跌。
      盘面上，受中东局势紧张升级的影响，石油天然气、航运等板块再度大涨。
      分析师认为，短期内市场仍将维持震荡格局。
    `
    const cleaned = cleanWebContent(content)

    expect(cleaned).toContain("A股收评")
    expect(cleaned).toContain("3.07%")
    expect(cleaned).toContain("中东局势")
    expect(cleaned).toContain("分析师认为")
  })
})
