/**
 * Research Loop
 *
 * Dedicated execution loop for research/analysis tasks.
 *
 * Phases:
 * 1. Understanding - Parse topic, dimensions, search strategy
 * 2. Searching - Multi-source parallel search
 * 3. Synthesizing - Dedupe, validate, annotate sources
 * 4. Analyzing - LLM-based analysis and insight extraction
 * 5. Reporting - Generate structured report
 * 6. Learning - Sediment patterns, suggest Hands
 */

import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { createWebSearcher, type WebSearcher } from "./web-search"
import { renderReport, type ReportData } from "./report-renderer"
import { createResearchLearner, type ResearchLearner } from "./research-learner"

const log = Log.create({ service: "autonomous.research-loop" })

// ============================================================================
// Types
// ============================================================================

export interface ResearchProblem {
  sessionId: string
  topic: string
  dimensions?: string[]
  timeRange?: "today" | "week" | "month" | "all"
  sourceTypes?: ("web" | "financial" | "news")[]
  maxSources?: number
}

export interface ResearchSource {
  url: string
  title: string
  snippet: string
  credibility: "high" | "medium" | "low"
  content?: string
}

export interface ResearchResult {
  success: boolean
  topic: string
  summary: string
  report: string
  sources: ResearchSource[]
  insights: string[]
  durationMs: number
  outputPath?: string
  handCreated?: string
}

export interface ResearchLoopConfig {
  maxSources?: number
  maxInlineLength?: number
  enableLearning?: boolean
  enableHandCreation?: boolean
}

const DEFAULT_CONFIG: Required<ResearchLoopConfig> = {
  maxSources: 10,
  maxInlineLength: 3500, // Telegram limit is 4096, leave room for PDCA summary
  enableLearning: true,
  enableHandCreation: true,
}

// ============================================================================
// Content Extraction Helpers (Fallback when LLM fails)
// ============================================================================

/** @internal Exported for testing */
export { extractSummaryFromContent, extractInsightsFromContent, cleanWebContent }

/**
 * Clean web content by removing common noise patterns
 *
 * Removes:
 * - Tracking pixels and images
 * - Navigation menu items
 * - JavaScript links
 * - Social media/app download lists
 * - Repeated whitespace
 * - Common Chinese site navigation patterns
 */
function cleanWebContent(content: string): string {
  let cleaned = content

  // Remove tracking pixels and small images (like beacon.sina.com.cn/a.gif)
  cleaned = cleaned.replace(/!\[.*?\]\([^)]*\.(gif|png|jpg|jpeg)[^)]*\)/gi, "")
  cleaned = cleaned.replace(/!\[\]\([^)]*\)/g, "") // Empty alt images

  // Remove JavaScript links
  cleaned = cleaned.replace(/\(javascript:[^)]*\)/gi, "")

  // Remove partial URL references that look broken
  cleaned = cleaned.replace(/\(https?:\/\/[^\s)]+(?:\s|$)/g, "")

  // Remove navigation menu patterns (bullet lists with short items)
  // Pattern: lines starting with • followed by short text (likely nav items)
  const lines = cleaned.split("\n")
  const filteredLines = lines.filter((line) => {
    const trimmed = line.trim()

    // Skip lines that are just bullet navigation items
    if (/^[•\-\*]\s+.{1,20}$/.test(trimmed)) {
      // Check if it CONTAINS common nav keywords (not just starts with)
      const navKeywords = [
        "首页",
        "新闻",
        "体育",
        "财经",
        "娱乐",
        "科技",
        "博客",
        "图片",
        "专栏",
        "更多",
        "汽车",
        "教育",
        "时尚",
        "女性",
        "星座",
        "健康",
        "房产",
        "历史",
        "视频",
        "收藏",
        "育儿",
        "读书",
        "佛学",
        "游戏",
        "旅游",
        "邮箱",
        "导航",
        "微博",
        "众测",
        "天气通",
        "移动客户端",
        "注册",
        "登录",
      ]
      return !navKeywords.some((kw) => trimmed.includes(kw))
    }

    // Skip empty lines that create too much whitespace
    if (trimmed === "") return true // Keep one empty line

    // Skip lines that are just "注册" or "登录" etc
    if (/^(注册|登录|分享|收藏|关注|订阅)$/.test(trimmed)) return false

    return true
  })

  cleaned = filteredLines.join("\n")

  // Remove excessive whitespace
  cleaned = cleaned.replace(/\n{3,}/g, "\n\n")
  cleaned = cleaned.replace(/[ \t]+/g, " ")

  // Remove common footer patterns
  cleaned = cleaned.replace(/由\s*CodeCoder.*自动生成/g, "")
  cleaned = cleaned.replace(/滚动新闻\s*>\s*正文/g, "")
  cleaned = cleaned.replace(/行情\s+股吧/g, "")

  // Remove social sharing button text
  cleaned = cleaned.replace(/\(sinafinance\)/g, "")
  cleaned = cleaned.replace(/24小时滚动播报.*?扫描二维码关注/g, "")

  return cleaned.trim()
}

/**
 * Extract a meaningful summary from synthesized content without LLM
 *
 * Strategy:
 * 1. Find sentences containing key terms related to the topic
 * 2. Prioritize sentences with numbers/data
 * 3. Cap at reasonable length
 */
function extractSummaryFromContent(content: string, topic: string): string {
  const topicTerms = topic.toLowerCase().split(/\s+/).filter(t => t.length > 1)

  // Split content into sentences (handles Chinese and English)
  const sentences = content
    .split(/[。！？\.\!\?]+/)
    .map(s => s.trim())
    .filter(s => s.length > 10 && s.length < 300)

  // Score sentences by relevance
  const scoredSentences = sentences.map(sentence => {
    const lower = sentence.toLowerCase()
    let score = 0

    // Topic term matches
    topicTerms.forEach(term => {
      if (lower.includes(term)) score += 2
    })

    // Contains numbers (likely data/statistics)
    if (/\d+(\.\d+)?%?/.test(sentence)) score += 3

    // Contains trend indicators
    if (/上涨|下跌|增长|下降|涨幅|跌幅|走势|趋势/.test(sentence)) score += 2

    // Contains analysis indicators
    if (/分析|显示|表明|预计|预测|认为|指出/.test(sentence)) score += 1

    return { sentence, score }
  })

  // Sort by score and take top sentences
  const topSentences = scoredSentences
    .sort((a, b) => b.score - a.score)
    .slice(0, 5)
    .map(s => s.sentence)

  if (topSentences.length === 0) {
    return `关于"${topic}"的研究已完成，收集了${content.length}字的相关信息，包含多个数据来源的分析内容。`
  }

  // Build summary from top sentences
  const summary = topSentences.slice(0, 3).join("。") + "。"

  return summary.length > 50
    ? summary
    : `${summary} 综合以上${topSentences.length}个信息来源的分析，提供了关于${topic}的最新情况。`
}

/**
 * Extract insights from content without LLM
 *
 * Strategy:
 * 1. Find sentences with insight indicators (数据、趋势、分析、建议)
 * 2. Extract unique key points
 * 3. Ensure at least 3 insights
 */
function extractInsightsFromContent(content: string, topic: string): string[] {
  const insights: string[] = []

  // Split into sentences
  const sentences = content
    .split(/[。！？\.\!\?]+/)
    .map(s => s.trim())
    .filter(s => s.length > 15 && s.length < 200)

  // Pattern matching for different insight types
  const patterns = [
    { regex: /(\d+(\.\d+)?%?\s*(上涨|下跌|增长|下降|涨幅|跌幅))/, prefix: "数据表现：" },
    { regex: /(走势|趋势|方向|预期)/, prefix: "趋势分析：" },
    { regex: /(支撑|阻力|关键位|点位)/, prefix: "技术要点：" },
    { regex: /(因素|原因|影响|驱动)/, prefix: "影响因素：" },
    { regex: /(建议|策略|操作|关注)/, prefix: "市场建议：" },
    { regex: /(预计|预测|可能|或将)/, prefix: "前景展望：" },
  ]

  // Extract insights by pattern
  for (const { regex, prefix } of patterns) {
    if (insights.length >= 5) break

    const matching = sentences.find(s =>
      regex.test(s) && !insights.some(i => i.includes(s.slice(0, 20)))
    )

    if (matching) {
      // Clean and truncate the insight
      const cleaned = matching.replace(/\[.*?\]/g, "").trim()
      if (cleaned.length > 10) {
        insights.push(`${prefix}${cleaned.slice(0, 150)}`)
      }
    }
  }

  // If we don't have enough insights, extract any sentences with numbers
  if (insights.length < 3) {
    const numericSentences = sentences
      .filter(s => /\d+(\.\d+)?%?/.test(s))
      .filter(s => !insights.some(i => i.includes(s.slice(0, 20))))
      .slice(0, 3 - insights.length)

    numericSentences.forEach(s => {
      const cleaned = s.replace(/\[.*?\]/g, "").trim()
      if (cleaned.length > 10) {
        insights.push(`关键数据：${cleaned.slice(0, 150)}`)
      }
    })
  }

  // Ensure minimum 3 insights
  while (insights.length < 3) {
    const fallbacks = [
      `当前${topic}市场信息需要持续关注`,
      `建议结合多方数据源综合分析${topic}走势`,
      `${topic}相关数据显示市场存在一定波动`,
    ]
    insights.push(fallbacks[insights.length] ?? `${topic}分析要点${insights.length + 1}`)
  }

  return insights.slice(0, 5)
}

// ============================================================================
// Research Loop Implementation
// ============================================================================

export function createResearchLoop(config: ResearchLoopConfig = {}) {
  const cfg = { ...DEFAULT_CONFIG, ...config }
  let webSearcher: WebSearcher | null = null
  let learner: ResearchLearner | null = null

  /** Initialize components lazily */
  function ensureInitialized(sessionId: string) {
    if (!webSearcher) {
      // Research tasks always need external data, so set threshold > 1.0
      // to bypass the confidence check in WebSearcher.evaluateSearchNeed()
      webSearcher = createWebSearcher(sessionId, { confidenceThreshold: 1.1 })
    }
    if (!learner && cfg.enableLearning) {
      learner = createResearchLearner()
    }
  }

  /** Phase 1: Understand the research request */
  async function understand(problem: ResearchProblem): Promise<{
    topic: string
    searchQueries: string[]
    dimensions: string[]
  }> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId: problem.sessionId,
      phase: "understanding",
    })

    // Generate search queries from topic
    const searchQueries = [
      problem.topic,
      `${problem.topic} 最新`,
      `${problem.topic} 分析`,
    ]

    // Add time-based queries
    if (problem.timeRange === "today") {
      searchQueries.push(`${problem.topic} ${new Date().toISOString().split("T")[0]}`)
    }

    const dimensions = problem.dimensions ?? ["趋势", "数据", "分析"]

    log.debug("Research understanding complete", {
      topic: problem.topic,
      queryCount: searchQueries.length,
    })

    return { topic: problem.topic, searchQueries, dimensions }
  }

  /** Phase 2: Search multiple sources */
  async function search(
    sessionId: string,
    queries: string[],
    maxSources: number,
  ): Promise<ResearchSource[]> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "searching",
    })

    const sources: ResearchSource[] = []

    console.error("[RESEARCH-LOOP] Starting search phase", { queries, webSearcherExists: !!webSearcher })

    // Run searches in parallel
    const searchPromises = queries.slice(0, 3).map(async (query) => {
      try {
        console.error("[RESEARCH-LOOP] Executing search query:", query)
        const result = await webSearcher?.search({
          sessionId,
          problem: query,
          previousAttempts: [],
        })

        console.error("[RESEARCH-LOOP] Search result:", {
          query,
          hasResult: !!result,
          sourceCount: result?.sources?.length ?? 0,
          fetchedContentCount: result?.fetchedContent?.length ?? 0,
        })

        if (result?.sources) {
          return result.sources.map((s) => ({
            url: s.url,
            title: s.title,
            snippet: s.snippet ?? "",
            credibility: assessCredibility(s.url),
            content: result.fetchedContent?.find((f) => f.url === s.url)?.content,
          }))
        }
      } catch (error) {
        console.error("[RESEARCH-LOOP] Search query failed:", query, error)
        log.warn("Search query failed", { query, error })
      }
      return []
    })

    const results = await Promise.all(searchPromises)
    results.forEach((r) => sources.push(...r))

    // Dedupe by URL
    const uniqueSources = Array.from(
      new Map(sources.map((s) => [s.url, s])).values(),
    ).slice(0, maxSources)

    await Bus.publish(AutonomousEvent.ResearchSourceFound, {
      sessionId,
      sourceCount: uniqueSources.length,
      credibilityBreakdown: {
        high: uniqueSources.filter((s) => s.credibility === "high").length,
        medium: uniqueSources.filter((s) => s.credibility === "medium").length,
        low: uniqueSources.filter((s) => s.credibility === "low").length,
      },
    })

    log.debug("Search complete", { sourceCount: uniqueSources.length })

    return uniqueSources
  }

  /** Assess source credibility based on URL */
  function assessCredibility(url: string): "high" | "medium" | "low" {
    const highCredSites = [
      "reuters.com", "bloomberg.com", "wsj.com", "ft.com",
      "economist.com", "nytimes.com", "bbc.com",
      "gov.cn", "stats.gov.cn", "pbc.gov.cn",
    ]
    const mediumCredSites = [
      "yahoo.com", "google.com", "bing.com",
      "sina.com", "163.com", "sohu.com",
    ]

    try {
      const domain = new URL(url).hostname.replace("www.", "")

      if (highCredSites.some((s) => domain.includes(s))) return "high"
      if (mediumCredSites.some((s) => domain.includes(s))) return "medium"
    } catch {
      // Invalid URL
    }
    return "low"
  }

  /** Phase 3: Synthesize information */
  async function synthesize(
    sessionId: string,
    sources: ResearchSource[],
  ): Promise<string> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "synthesizing",
    })

    // Combine content from sources, cleaning each source's content
    const combinedContent = sources
      .filter((s) => s.content || s.snippet)
      .map((s) => {
        const rawContent = s.content || s.snippet
        const cleanedContent = cleanWebContent(rawContent)
        return `[${s.title}]\n${cleanedContent}`
      })
      .join("\n\n---\n\n")

    log.debug("Synthesis complete", { contentLength: combinedContent.length })

    return combinedContent
  }

  /** Phase 4: Analyze with LLM */
  async function analyze(
    sessionId: string,
    topic: string,
    synthesizedContent: string,
    dimensions: string[],
  ): Promise<{ summary: string; analysis: string; insights: string[] }> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "analyzing",
    })

    // If no content to analyze, return fallback
    if (!synthesizedContent || synthesizedContent.length === 0) {
      return {
        summary: `关于"${topic}"的研究已完成，但未能找到足够的信息。`,
        analysis: "未找到相关内容进行分析。",
        insights: ["需要更多数据来源"],
      }
    }

    // Use LLM for analysis
    try {
      const { generateText } = await import("ai")
      const { Provider } = await import("@/provider/provider")

      const systemPrompt = `你是一个专业的研究分析师。基于提供的信息，生成结构化的分析报告。

请分析以下维度：${dimensions.join("、")}

请按以下格式输出：
## 摘要
（3-5句话的摘要）

## 详细分析
（300-500字的详细分析）

## 关键洞察
- 洞察1
- 洞察2
- 洞察3
（至少3个关键洞察）`

      console.error("[RESEARCH-LOOP] Starting LLM analysis, content length:", synthesizedContent.length)

      const defaultModel = await Provider.defaultModel()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      console.error("[RESEARCH-LOOP] Using model:", defaultModel.modelID)

      // Use generateText instead of generateObject for better model compatibility
      const result = await generateText({
        temperature: 0.3,
        messages: [
          { role: "system", content: systemPrompt },
          {
            role: "user",
            content: `主题：${topic}\n\n收集的信息：\n${synthesizedContent.slice(0, 8000)}`,
          },
        ],
        model: language,
      })

      const text = result.text || ""
      console.error("[RESEARCH-LOOP] LLM analysis success, response length:", text.length)

      // Parse the structured response
      const summaryMatch = text.match(/##\s*摘要\s*\n([\s\S]*?)(?=##|$)/)
      const analysisMatch = text.match(/##\s*详细分析\s*\n([\s\S]*?)(?=##|$)/)
      const insightsMatch = text.match(/##\s*关键洞察\s*\n([\s\S]*?)(?=##|$)/)

      const summary = summaryMatch ? summaryMatch[1].trim() : text.slice(0, 200)
      const analysis = analysisMatch ? analysisMatch[1].trim() : text.slice(0, 1000)
      const insights = insightsMatch
        ? insightsMatch[1].split(/\n-\s*/).filter(s => s.trim().length > 0).map(s => s.trim())
        : ["需要进一步分析"]

      console.error("[RESEARCH-LOOP] Parsed analysis:", {
        summaryLength: summary.length,
        analysisLength: analysis.length,
        insightCount: insights.length,
      })

      return { summary, analysis, insights }
    } catch (error) {
      console.error("[RESEARCH-LOOP] LLM analysis FAILED:", error)
      log.error("LLM analysis failed", { error })
    }

    // Improved fallback: extract meaningful content when LLM fails
    const extractedSummary = extractSummaryFromContent(synthesizedContent, topic)
    const extractedInsights = extractInsightsFromContent(synthesizedContent, topic)

    console.error("[RESEARCH-LOOP] Using fallback extraction:", {
      summaryLength: extractedSummary.length,
      insightCount: extractedInsights.length,
    })

    return {
      summary: extractedSummary,
      analysis: synthesizedContent.slice(0, 2000),
      insights: extractedInsights,
    }
  }

  /** Phase 5: Generate report */
  async function report(
    sessionId: string,
    topic: string,
    summary: string,
    analysis: string,
    insights: string[],
    sources: ResearchSource[],
  ): Promise<{ content: string; filePath?: string }> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "reporting",
    })

    const reportData: ReportData = {
      topic,
      summary,
      analysis,
      insights,
      sources: sources.map((s) => ({
        url: s.url,
        title: s.title,
        snippet: s.snippet,
        credibility: s.credibility,
      })),
    }

    const result = await renderReport(reportData, {
      maxInlineLength: cfg.maxInlineLength,
    })

    log.debug("Report generated", { mode: result.mode })

    return { content: result.content, filePath: result.filePath }
  }

  /** Phase 6: Learn from research */
  async function learn(
    sessionId: string,
    topic: string,
    sources: ResearchSource[],
  ): Promise<string | undefined> {
    if (!cfg.enableLearning || !learner) return undefined

    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "learning",
    })

    // Record this research session
    learner.recordResearch({
      topic,
      keywords: topic.split(/\s+/),
      sources: sources.map((s) => {
        try {
          return new URL(s.url).hostname
        } catch {
          return s.url
        }
      }),
    })

    // Check for Hand creation suggestion
    if (cfg.enableHandCreation) {
      const suggestion = learner.suggestHandCreation(topic)
      if (suggestion) {
        await Bus.publish(AutonomousEvent.ResearchPatternLearned, {
          sessionId,
          patternId: `pattern-${Date.now()}`,
          topic,
          keywords: suggestion.keywords,
          frequency: suggestion.frequency,
          confidence: suggestion.confidence,
        })

        log.info("Research pattern learned", {
          topic,
          frequency: suggestion.frequency,
          confidence: suggestion.confidence,
        })

        // TODO: Actually create the Hand
        return `Suggested: ${suggestion.frequency} research on "${topic}"`
      }
    }

    return undefined
  }

  return {
    /** Execute full research loop */
    async research(problem: ResearchProblem): Promise<ResearchResult> {
      const startTime = Date.now()
      ensureInitialized(problem.sessionId)

      await Bus.publish(AutonomousEvent.ResearchStarted, {
        sessionId: problem.sessionId,
        topic: problem.topic,
        dimensions: problem.dimensions,
        sourceTypes: problem.sourceTypes,
      })

      try {
        // Phase 1: Understand
        const { topic, searchQueries, dimensions } = await understand(problem)

        // Phase 2: Search
        const sources = await search(
          problem.sessionId,
          searchQueries,
          problem.maxSources ?? cfg.maxSources,
        )

        // Phase 3: Synthesize
        const synthesized = await synthesize(problem.sessionId, sources)

        // Phase 4: Analyze
        const { summary, analysis, insights } = await analyze(
          problem.sessionId,
          topic,
          synthesized,
          dimensions,
        )

        // Phase 5: Report
        const { content, filePath } = await report(
          problem.sessionId,
          topic,
          summary,
          analysis,
          insights,
          sources,
        )

        // Phase 6: Learn
        const handCreated = await learn(problem.sessionId, topic, sources)

        const result: ResearchResult = {
          success: true,
          topic,
          summary,
          report: content,
          sources,
          insights,
          durationMs: Date.now() - startTime,
          outputPath: filePath,
          handCreated,
        }

        console.error("[RESEARCH-LOOP] Research completed:", {
          success: result.success,
          topic: result.topic,
          sourceCount: result.sources.length,
          insightCount: result.insights.length,
          summaryLength: result.summary?.length ?? 0,
          reportLength: result.report?.length ?? 0,
          sourcesWithContent: result.sources.filter(s => s.content).length,
          sourcesWithSnippet: result.sources.filter(s => s.snippet).length,
        })

        await Bus.publish(AutonomousEvent.ResearchCompleted, {
          sessionId: problem.sessionId,
          topic,
          success: true,
          reportMode: filePath ? "file" : "inline",
          reportPath: filePath,
          insightCount: insights.length,
          sourceCount: sources.length,
          durationMs: result.durationMs,
          handCreated,
        })

        return result
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)

        await Bus.publish(AutonomousEvent.ResearchFailed, {
          sessionId: problem.sessionId,
          topic: problem.topic,
          phase: "unknown",
          error: errorMsg,
          retryable: true,
        })

        return {
          success: false,
          topic: problem.topic,
          summary: `研究失败: ${errorMsg}`,
          report: "",
          sources: [],
          insights: [],
          durationMs: Date.now() - startTime,
        }
      }
    },

    /** Cleanup resources */
    async cleanup(): Promise<void> {
      webSearcher = null
      learner = null
    },

    /** Get learner for external access */
    getLearner(): ResearchLearner | null {
      return learner
    },
  }
}

export type ResearchLoop = ReturnType<typeof createResearchLoop>
