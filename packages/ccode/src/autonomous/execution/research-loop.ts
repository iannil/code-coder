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
  maxInlineLength: 1000,
  enableLearning: true,
  enableHandCreation: true,
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
      webSearcher = createWebSearcher(sessionId, { confidenceThreshold: 0.3 })
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

    // Run searches in parallel
    const searchPromises = queries.slice(0, 3).map(async (query) => {
      try {
        const result = await webSearcher?.search({
          sessionId,
          problem: query,
          previousAttempts: [],
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

    // Combine content from sources
    const combinedContent = sources
      .filter((s) => s.content || s.snippet)
      .map((s) => `[${s.title}]\n${s.content || s.snippet}`)
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
      const { generateObject } = await import("ai")
      const { Provider } = await import("@/provider/provider")
      const z = await import("zod").then((m) => m.default)

      const systemPrompt = `你是一个专业的研究分析师。基于提供的信息，生成结构化的分析报告。

请分析以下维度：${dimensions.join("、")}`

      const defaultModel = await Provider.defaultModel()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      const result = await generateObject({
        temperature: 0.3,
        messages: [
          { role: "system", content: systemPrompt },
          {
            role: "user",
            content: `主题：${topic}\n\n收集的信息：\n${synthesizedContent.slice(0, 8000)}`,
          },
        ],
        model: language,
        schema: z.object({
          summary: z.string().describe("3-5句话的摘要"),
          analysis: z.string().describe("详细分析（300-500字）"),
          insights: z.array(z.string()).describe("关键洞察列表"),
        }),
      })

      return {
        summary: result.object.summary ?? "无法生成摘要",
        analysis: result.object.analysis ?? "无法生成分析",
        insights: result.object.insights ?? [],
      }
    } catch (error) {
      log.error("LLM analysis failed", { error })
    }

    // Fallback
    return {
      summary: `关于"${topic}"的研究已完成，收集了${synthesizedContent.length}字的相关信息。`,
      analysis: synthesizedContent.slice(0, 1000),
      insights: ["需要进一步分析"],
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
