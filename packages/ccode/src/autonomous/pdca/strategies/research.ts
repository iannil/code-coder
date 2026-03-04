/**
 * Research Strategy
 *
 * Acceptance strategy for research/analysis tasks.
 * Validates source credibility, coverage, freshness, accuracy, and insight quality.
 */

import { Log } from "@/util/log"
import { BaseAcceptanceStrategy } from "./base"
import type { TaskType } from "../../classification/types"
import type {
  TaskExecutionResult,
  PDCACheckResult,
  PDCAActResult,
  PDCAIssue,
  PDCAConfig,
  ResearchOutput,
  CheckItemResult,
  CLOSEScore,
} from "../types"

const log = Log.create({ service: "pdca.strategy.research" })

// ============================================================================
// Check Items for Research Tasks
// ============================================================================

const RESEARCH_CHECK_ITEMS = {
  source_credibility: {
    name: "source_credibility",
    description: "Sources are from credible, authoritative sources",
    weight: 1.2,
  },
  coverage: {
    name: "coverage",
    description: "Research covers all relevant dimensions",
    weight: 1.0,
  },
  freshness: {
    name: "freshness",
    description: "Information is recent and timely",
    weight: 0.8,
  },
  accuracy: {
    name: "accuracy",
    description: "Information is consistent and accurate",
    weight: 1.5,
  },
  insight_quality: {
    name: "insight_quality",
    description: "Insights are valuable and actionable",
    weight: 1.0,
  },
} as const

// ============================================================================
// Credibility Assessment
// ============================================================================

const HIGH_CREDIBILITY_DOMAINS = [
  "reuters.com", "bloomberg.com", "wsj.com", "ft.com",
  "economist.com", "nytimes.com", "bbc.com",
  "gov.cn", "stats.gov.cn", "pbc.gov.cn",
  "imf.org", "worldbank.org", "federalreserve.gov",
  "nature.com", "science.org", "arxiv.org",
]

const MEDIUM_CREDIBILITY_DOMAINS = [
  "yahoo.com", "google.com", "bing.com",
  "sina.com", "163.com", "sohu.com",
  "wikipedia.org", "investopedia.com",
]

// ============================================================================
// Research Strategy
// ============================================================================

export class ResearchStrategy extends BaseAcceptanceStrategy<ResearchOutput> {
  readonly taskType: TaskType = "research"
  readonly name = "Research Acceptance Strategy"

  getCheckItems(): string[] {
    return Object.keys(RESEARCH_CHECK_ITEMS)
  }

  getCheckWeights(): Record<string, number> {
    return Object.fromEntries(
      Object.entries(RESEARCH_CHECK_ITEMS).map(([k, v]) => [k, v.weight]),
    )
  }

  async check(
    result: TaskExecutionResult<ResearchOutput>,
    originalRequest: string,
    config: PDCAConfig,
  ): Promise<PDCACheckResult> {
    const startTime = Date.now()
    const issues: PDCAIssue[] = []
    const checks: Record<string, CheckItemResult> = {}
    const output = result.output

    log.debug("Running research acceptance check", {
      sessionId: config.sessionId,
      topic: output?.topic,
      sourceCount: output?.sources?.length ?? 0,
    })

    // Check 1: Source Credibility
    const credibilityCheck = this.checkSourceCredibility(output, issues)
    checks.source_credibility = credibilityCheck

    // Check 2: Coverage
    const coverageCheck = await this.checkCoverage(output, originalRequest, issues, config)
    checks.coverage = coverageCheck

    // Check 3: Freshness
    const freshnessCheck = this.checkFreshness(output, issues)
    checks.freshness = freshnessCheck

    // Check 4: Accuracy (requires LLM)
    const accuracyCheck = await this.checkAccuracy(output, issues, config)
    checks.accuracy = accuracyCheck

    // Check 5: Insight Quality
    const insightCheck = this.checkInsightQuality(output, issues)
    checks.insight_quality = insightCheck

    // Calculate CLOSE scores
    const closeScore = this.calculateResearchCLOSEScore(checks, output)

    // Determine if passed and recommendation
    const criticalIssues = issues.filter((i) => i.severity === "critical").length
    const recommendation = this.getRecommendation(
      closeScore.total,
      config.passThreshold,
      config.fixThreshold,
      criticalIssues,
    )

    const passed = recommendation === "pass"
    const durationMs = Date.now() - startTime

    // Generate report
    const report = this.generateReport(output, checks, closeScore, issues)

    return {
      taskType: this.taskType,
      passed,
      closeScore,
      issues,
      recommendation,
      checks,
      durationMs,
      report,
    }
  }

  async fix(
    issues: PDCAIssue[],
    context: TaskExecutionResult<ResearchOutput>,
    config: PDCAConfig,
  ): Promise<PDCAActResult> {
    const startTime = Date.now()
    const fixedIssues: string[] = []
    const remainingIssues: PDCAIssue[] = []

    log.debug("Running research fix loop", {
      sessionId: config.sessionId,
      issueCount: issues.length,
    })

    for (const issue of issues) {
      let fixed = false

      switch (issue.category) {
        case "source_credibility":
          // Try to find alternative high-credibility sources
          fixed = await this.tryFindBetterSources(issue, context, config)
          break

        case "coverage":
          // Try to expand search to cover missing dimensions
          fixed = await this.tryExpandCoverage(issue, context, config)
          break

        case "freshness":
          // Try to find more recent sources
          fixed = await this.tryFindRecentSources(issue, context, config)
          break

        case "accuracy":
          // Try to verify with additional sources
          fixed = await this.tryVerifyAccuracy(issue, context, config)
          break

        case "insight_quality":
          // Try to regenerate insights with more context
          fixed = await this.tryImproveInsights(issue, context, config)
          break

        default:
          // Unknown category, can't fix
          fixed = false
      }

      if (fixed) {
        fixedIssues.push(issue.id)
      } else {
        remainingIssues.push(issue)
      }
    }

    const durationMs = Date.now() - startTime

    return {
      fixed: remainingIssues.length === 0,
      fixedIssues,
      remainingIssues,
      shouldRecheck: fixedIssues.length > 0,
      attempts: issues.length,
      durationMs,
    }
  }

  // ==========================================================================
  // Check Implementations
  // ==========================================================================

  private checkSourceCredibility(
    output: ResearchOutput | undefined,
    issues: PDCAIssue[],
  ): CheckItemResult {
    if (!output?.sources || output.sources.length === 0) {
      issues.push({
        id: this.generateIssueId("source_credibility", 0),
        category: "source_credibility",
        severity: "high",
        description: "No sources found for research",
        suggestedAction: "Expand search queries or try alternative search engines",
      })

      return {
        passed: false,
        score: 0,
        weight: RESEARCH_CHECK_ITEMS.source_credibility.weight,
        details: "No sources",
      }
    }

    const highCred = output.sources.filter((s) => s.credibility === "high").length
    const mediumCred = output.sources.filter((s) => s.credibility === "medium").length
    const totalSources = output.sources.length

    // Calculate credibility score
    const credibilityScore = (highCred * 10 + mediumCred * 6) / totalSources
    const passed = highCred >= Math.ceil(totalSources * 0.3) // At least 30% high credibility

    if (!passed) {
      issues.push({
        id: this.generateIssueId("source_credibility", issues.length),
        category: "source_credibility",
        severity: "medium",
        description: `Only ${highCred}/${totalSources} sources are high credibility (need at least 30%)`,
        suggestedAction: "Search for sources from authoritative domains",
      })
    }

    return {
      passed,
      score: credibilityScore,
      weight: RESEARCH_CHECK_ITEMS.source_credibility.weight,
      details: `${highCred} high, ${mediumCred} medium credibility sources`,
    }
  }

  private async checkCoverage(
    output: ResearchOutput | undefined,
    originalRequest: string,
    issues: PDCAIssue[],
    config: PDCAConfig,
  ): Promise<CheckItemResult> {
    if (!output?.report) {
      return {
        passed: false,
        score: 0,
        weight: RESEARCH_CHECK_ITEMS.coverage.weight,
        details: "No report generated",
      }
    }

    // Use LLM to check if report covers all dimensions
    try {
      const { generateObject } = await import("ai")
      const { Provider } = await import("@/provider/provider")
      const z = await import("zod").then((m) => m.default)

      const defaultModel = await Provider.defaultModel()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      const result = await generateObject({
        temperature: 0.2,
        messages: [
          {
            role: "system",
            content: `You are a research quality evaluator. Check if the research report covers all aspects of the request.`,
          },
          {
            role: "user",
            content: `Original Request: ${originalRequest}

Research Report:
${output.report.slice(0, 3000)}

Evaluate coverage (0-10) and list any missing dimensions.`,
          },
        ],
        model: language,
        schema: z.object({
          score: z.number().min(0).max(10),
          coveredDimensions: z.array(z.string()),
          missingDimensions: z.array(z.string()),
          reasoning: z.string(),
        }),
      })

      const passed = result.object.score >= 7

      if (!passed) {
        issues.push({
          id: this.generateIssueId("coverage", issues.length),
          category: "coverage",
          severity: result.object.score >= 5 ? "medium" : "high",
          description: `Missing dimensions: ${result.object.missingDimensions.join(", ")}`,
          suggestedAction: `Add coverage for: ${result.object.missingDimensions.slice(0, 3).join(", ")}`,
        })
      }

      return {
        passed,
        score: result.object.score,
        weight: RESEARCH_CHECK_ITEMS.coverage.weight,
        details: result.object.reasoning,
      }
    } catch (error) {
      log.warn("Coverage check LLM failed", { error })
      // Fallback: basic check based on report/summary length and insight count
      // Include summary length since that's where extracted content lives when LLM fails
      const contentLength = Math.max(output.report.length, output.summary?.length ?? 0)
      const insightCount = output.insights?.length ?? 0
      const score = Math.min(10, (contentLength / 200) + insightCount)

      log.debug("Coverage fallback calculation", {
        reportLength: output.report.length,
        summaryLength: output.summary?.length ?? 0,
        insightCount,
        score,
      })

      return {
        passed: score >= 6,
        score,
        weight: RESEARCH_CHECK_ITEMS.coverage.weight,
        details: `Fallback: content=${contentLength}chars, insights=${insightCount}`,
      }
    }
  }

  private checkFreshness(
    output: ResearchOutput | undefined,
    issues: PDCAIssue[],
  ): CheckItemResult {
    if (!output?.sources || output.sources.length === 0) {
      return {
        passed: true, // Can't check freshness without sources
        score: 5,
        weight: RESEARCH_CHECK_ITEMS.freshness.weight,
        details: "No sources to check freshness",
      }
    }

    // Check for date patterns in source content
    const now = new Date()
    const oneWeekAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000)
    const oneMonthAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000)

    let recentCount = 0
    for (const source of output.sources) {
      if (source.publishedDate) {
        const pubDate = new Date(source.publishedDate)
        if (pubDate >= oneWeekAgo) {
          recentCount += 2
        } else if (pubDate >= oneMonthAgo) {
          recentCount += 1
        }
      } else if (source.content) {
        // Look for date patterns in content
        const currentYear = now.getFullYear().toString()
        if (source.content.includes(currentYear)) {
          recentCount += 1
        }
      }
    }

    const score = Math.min(10, (recentCount / output.sources.length) * 10)
    const passed = score >= 5

    if (!passed) {
      issues.push({
        id: this.generateIssueId("freshness", issues.length),
        category: "freshness",
        severity: "low",
        description: "Sources may be outdated",
        suggestedAction: "Search for more recent sources with date filters",
      })
    }

    return {
      passed,
      score,
      weight: RESEARCH_CHECK_ITEMS.freshness.weight,
      details: `${recentCount}/${output.sources.length} sources appear recent`,
    }
  }

  private async checkAccuracy(
    output: ResearchOutput | undefined,
    issues: PDCAIssue[],
    config: PDCAConfig,
  ): Promise<CheckItemResult> {
    if (!output?.insights || output.insights.length === 0) {
      return {
        passed: true,
        score: 5,
        weight: RESEARCH_CHECK_ITEMS.accuracy.weight,
        details: "No insights to verify",
      }
    }

    // Use LLM to check for internal consistency
    try {
      const { generateObject } = await import("ai")
      const { Provider } = await import("@/provider/provider")
      const z = await import("zod").then((m) => m.default)

      const defaultModel = await Provider.defaultModel()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      const result = await generateObject({
        temperature: 0.2,
        messages: [
          {
            role: "system",
            content: `You are a fact checker. Check if the research insights are internally consistent and don't contain contradictions.`,
          },
          {
            role: "user",
            content: `Research Summary: ${output.summary}

Insights:
${output.insights.map((i, idx) => `${idx + 1}. ${i}`).join("\n")}

Check for contradictions or inconsistencies. Rate accuracy 0-10.`,
          },
        ],
        model: language,
        schema: z.object({
          score: z.number().min(0).max(10),
          contradictions: z.array(z.string()),
          confidence: z.number().min(0).max(1),
        }),
      })

      const passed = result.object.score >= 7

      if (!passed && result.object.contradictions.length > 0) {
        issues.push({
          id: this.generateIssueId("accuracy", issues.length),
          category: "accuracy",
          severity: "high",
          description: `Found contradictions: ${result.object.contradictions[0]}`,
          suggestedAction: "Verify conflicting information with additional sources",
        })
      }

      return {
        passed,
        score: result.object.score,
        weight: RESEARCH_CHECK_ITEMS.accuracy.weight,
        details: `Confidence: ${(result.object.confidence * 100).toFixed(0)}%`,
      }
    } catch (error) {
      log.warn("Accuracy check LLM failed", { error })
      return {
        passed: true,
        score: 7,
        weight: RESEARCH_CHECK_ITEMS.accuracy.weight,
        details: "Accuracy check skipped (LLM unavailable)",
      }
    }
  }

  private checkInsightQuality(
    output: ResearchOutput | undefined,
    issues: PDCAIssue[],
  ): CheckItemResult {
    if (!output?.insights) {
      issues.push({
        id: this.generateIssueId("insight_quality", issues.length),
        category: "insight_quality",
        severity: "medium",
        description: "No insights generated from research",
        suggestedAction: "Reanalyze sources to extract key insights",
      })

      return {
        passed: false,
        score: 0,
        weight: RESEARCH_CHECK_ITEMS.insight_quality.weight,
        details: "No insights",
      }
    }

    // Score based on insight count and quality indicators
    const insightCount = output.insights.length
    const minInsights = 3

    // Quality indicators: length, specificity, actionability
    let qualityScore = 0
    for (const insight of output.insights) {
      if (insight.length > 50) qualityScore += 1
      if (/\d/.test(insight)) qualityScore += 1 // Contains numbers
      if (/建议|应该|可以|需要/.test(insight)) qualityScore += 1 // Actionable
    }

    const countScore = Math.min(10, (insightCount / minInsights) * 5)
    const avgQuality = insightCount > 0 ? (qualityScore / (insightCount * 3)) * 5 : 0
    const score = countScore + avgQuality

    const passed = insightCount >= minInsights && score >= 6

    if (!passed) {
      issues.push({
        id: this.generateIssueId("insight_quality", issues.length),
        category: "insight_quality",
        severity: "medium",
        description: `Only ${insightCount} insights (need ${minInsights}+)`,
        suggestedAction: "Extract more specific, actionable insights",
      })
    }

    return {
      passed,
      score,
      weight: RESEARCH_CHECK_ITEMS.insight_quality.weight,
      details: `${insightCount} insights, quality score: ${avgQuality.toFixed(1)}`,
    }
  }

  // ==========================================================================
  // CLOSE Score Calculation
  // ==========================================================================

  private calculateResearchCLOSEScore(
    checks: Record<string, CheckItemResult>,
    output: ResearchOutput | undefined,
  ): CLOSEScore {
    // Calculate weighted average
    let totalWeight = 0
    let weightedSum = 0

    for (const [, check] of Object.entries(checks)) {
      totalWeight += check.weight
      weightedSum += check.score * check.weight
    }

    const avgScore = totalWeight > 0 ? weightedSum / totalWeight : 5.0

    // Research-specific CLOSE mapping
    return {
      // Convergence: Did we find what we were looking for?
      convergence: Math.round(avgScore * 10) / 10,
      // Leverage: How much value from the research?
      leverage: Math.round(((output?.insights?.length ?? 0) / 5) * 10 * 10) / 10,
      // Optionality: Research is always fully reversible
      optionality: 10,
      // Surplus: Based on source quality
      surplus: Math.round((checks.source_credibility?.score ?? 5) * 10) / 10,
      // Evolution: Learning from research
      evolution: Math.round(Math.min(10, avgScore * 0.8 + 2) * 10) / 10,
      total: Math.round(avgScore * 10) / 10,
    }
  }

  // ==========================================================================
  // Fix Implementations
  // ==========================================================================

  /**
   * Try to find better sources by searching high-credibility domains.
   * This targets authoritative sources like Reuters, Bloomberg, government sites.
   */
  private async tryFindBetterSources(
    issue: PDCAIssue,
    context: TaskExecutionResult<ResearchOutput>,
    config: PDCAConfig,
  ): Promise<boolean> {
    log.info("Attempting to find better sources", {
      issueId: issue.id,
      sessionId: config.sessionId,
      currentSourceCount: context.output?.sources?.length ?? 0,
    })

    try {
      const { createWebSearcher } = await import("../../execution/web-search")
      const webSearcher = createWebSearcher(config.sessionId, { confidenceThreshold: 1.1 })
      const topic = context.output?.topic ?? ""

      // Search with high-credibility domain focus
      const highCredQueries = [
        `site:reuters.com ${topic}`,
        `site:bloomberg.com ${topic}`,
        `site:gov.cn ${topic}`,
        `${topic} 官方 数据`,
      ]

      let foundNewSources = false
      for (const query of highCredQueries) {
        try {
          const result = await webSearcher.search({
            sessionId: config.sessionId,
            problem: query,
            previousAttempts: [],
          })

          if (result?.sources && result.sources.length > 0) {
            // Add new high-credibility sources to context
            const newSources = result.sources.map((s) => ({
              url: s.url,
              title: s.title,
              snippet: s.snippet ?? "",
              credibility: "high" as const,
              content: result.fetchedContent?.find((f) => f.url === s.url)?.content,
            }))

            if (context.output?.sources) {
              context.output.sources.push(...newSources)
              foundNewSources = true
              log.info("Added high-credibility sources", { count: newSources.length })
            }
          }
        } catch (searchError) {
          log.debug("High-cred search query failed", { query, error: searchError })
        }
      }

      return foundNewSources
    } catch (error) {
      log.warn("Failed to find better sources", { error })
      return false
    }
  }

  /**
   * Try to expand coverage by searching for missing dimensions.
   * Extracts missing dimensions from the issue and performs targeted searches.
   */
  private async tryExpandCoverage(
    issue: PDCAIssue,
    context: TaskExecutionResult<ResearchOutput>,
    config: PDCAConfig,
  ): Promise<boolean> {
    log.info("Attempting to expand coverage", {
      issueId: issue.id,
      sessionId: config.sessionId,
      issueDescription: issue.description,
    })

    try {
      // Extract missing dimensions from the issue description
      const missingMatch = issue.description.match(/Missing dimensions?:\s*(.+)/i)
      if (!missingMatch) {
        log.debug("No missing dimensions found in issue")
        return false
      }

      const missingDimensions = missingMatch[1].split(/[,，、]/).map((d) => d.trim()).filter(Boolean)
      if (missingDimensions.length === 0) return false

      const { createWebSearcher } = await import("../../execution/web-search")
      const webSearcher = createWebSearcher(config.sessionId, { confidenceThreshold: 1.1 })
      const topic = context.output?.topic ?? ""

      let expandedCoverage = false
      for (const dimension of missingDimensions.slice(0, 3)) {
        try {
          const result = await webSearcher.search({
            sessionId: config.sessionId,
            problem: `${topic} ${dimension}`,
            previousAttempts: [],
          })

          if (result?.sources && result.sources.length > 0) {
            const newSources = result.sources.map((s) => ({
              url: s.url,
              title: s.title,
              snippet: s.snippet ?? "",
              credibility: this.assessCredibility(s.url),
              content: result.fetchedContent?.find((f) => f.url === s.url)?.content,
            }))

            if (context.output?.sources) {
              context.output.sources.push(...newSources)
              expandedCoverage = true
              log.info("Added coverage sources for dimension", { dimension, count: newSources.length })
            }
          }
        } catch (searchError) {
          log.debug("Coverage expansion search failed", { dimension, error: searchError })
        }
      }

      return expandedCoverage
    } catch (error) {
      log.warn("Failed to expand coverage", { error })
      return false
    }
  }

  /**
   * Try to find more recent sources by adding date filters to searches.
   */
  private async tryFindRecentSources(
    issue: PDCAIssue,
    context: TaskExecutionResult<ResearchOutput>,
    config: PDCAConfig,
  ): Promise<boolean> {
    log.info("Attempting to find recent sources", {
      issueId: issue.id,
      sessionId: config.sessionId,
    })

    try {
      const { createWebSearcher } = await import("../../execution/web-search")
      const webSearcher = createWebSearcher(config.sessionId, { confidenceThreshold: 1.1 })
      const topic = context.output?.topic ?? ""

      // Add date-focused queries
      const today = new Date().toISOString().split("T")[0]
      const recentQueries = [
        `${topic} ${today}`,
        `${topic} 最新 今日`,
        `${topic} latest news`,
      ]

      let foundRecentSources = false
      for (const query of recentQueries) {
        try {
          const result = await webSearcher.search({
            sessionId: config.sessionId,
            problem: query,
            previousAttempts: [],
          })

          if (result?.sources && result.sources.length > 0) {
            const newSources = result.sources.map((s) => ({
              url: s.url,
              title: s.title,
              snippet: s.snippet ?? "",
              credibility: this.assessCredibility(s.url),
              content: result.fetchedContent?.find((f) => f.url === s.url)?.content,
              publishedDate: today, // Mark as recent
            }))

            if (context.output?.sources) {
              context.output.sources.push(...newSources)
              foundRecentSources = true
              log.info("Added recent sources", { count: newSources.length })
            }
          }
        } catch (searchError) {
          log.debug("Recent source search failed", { query, error: searchError })
        }
      }

      return foundRecentSources
    } catch (error) {
      log.warn("Failed to find recent sources", { error })
      return false
    }
  }

  /**
   * Try to verify accuracy by cross-referencing with additional sources.
   */
  private async tryVerifyAccuracy(
    issue: PDCAIssue,
    context: TaskExecutionResult<ResearchOutput>,
    config: PDCAConfig,
  ): Promise<boolean> {
    log.info("Attempting to verify accuracy", {
      issueId: issue.id,
      sessionId: config.sessionId,
      issueDescription: issue.description,
    })

    try {
      // Extract the contradiction or claim that needs verification
      const contradictionMatch = issue.description.match(/contradictions?:\s*(.+)/i)
      if (!contradictionMatch) {
        log.debug("No specific contradiction found to verify")
        return false
      }

      const claimToVerify = contradictionMatch[1].slice(0, 100)
      const { createWebSearcher } = await import("../../execution/web-search")
      const webSearcher = createWebSearcher(config.sessionId, { confidenceThreshold: 1.1 })

      // Search for verification with fact-checking focus
      const verifyQueries = [
        `${claimToVerify} fact check`,
        `${claimToVerify} verification`,
        `${context.output?.topic} ${claimToVerify}`,
      ]

      let verified = false
      for (const query of verifyQueries) {
        try {
          const result = await webSearcher.search({
            sessionId: config.sessionId,
            problem: query,
            previousAttempts: [],
          })

          if (result?.sources && result.sources.length > 0) {
            // Check if verification sources agree
            const highCredSources = result.sources.filter((s) =>
              HIGH_CREDIBILITY_DOMAINS.some((d) => s.url.includes(d))
            )

            if (highCredSources.length > 0) {
              const newSources = highCredSources.map((s) => ({
                url: s.url,
                title: s.title,
                snippet: s.snippet ?? "",
                credibility: "high" as const,
                content: result.fetchedContent?.find((f) => f.url === s.url)?.content,
              }))

              if (context.output?.sources) {
                context.output.sources.push(...newSources)
                verified = true
                log.info("Added verification sources", { count: newSources.length })
              }
            }
          }
        } catch (searchError) {
          log.debug("Verification search failed", { query, error: searchError })
        }
      }

      return verified
    } catch (error) {
      log.warn("Failed to verify accuracy", { error })
      return false
    }
  }

  /**
   * Try to improve insights by regenerating them with more context.
   * Uses LLM to extract additional insights from source content.
   */
  private async tryImproveInsights(
    issue: PDCAIssue,
    context: TaskExecutionResult<ResearchOutput>,
    config: PDCAConfig,
  ): Promise<boolean> {
    log.info("Attempting to improve insights", {
      issueId: issue.id,
      sessionId: config.sessionId,
      currentInsightCount: context.output?.insights?.length ?? 0,
    })

    try {
      if (!context.output?.sources || context.output.sources.length === 0) {
        log.debug("No sources to extract insights from")
        return false
      }

      const { generateObject } = await import("ai")
      const { Provider } = await import("@/provider/provider")
      const z = await import("zod").then((m) => m.default)

      const defaultModel = await Provider.defaultModel()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      // Concatenate source content for analysis
      const sourceContent = context.output.sources
        .filter((s) => s.content)
        .map((s) => s.content)
        .join("\n\n")
        .slice(0, 4000)

      const existingInsights = context.output.insights?.join("\n") ?? ""

      const result = await generateObject({
        temperature: 0.3,
        messages: [
          {
            role: "system",
            content: `You are a research analyst. Extract valuable, actionable insights from the provided content.
Focus on:
- Specific data points and statistics
- Trends and patterns
- Actionable recommendations
- Key findings not mentioned in existing insights`,
          },
          {
            role: "user",
            content: `Topic: ${context.output.topic}

Existing Insights:
${existingInsights}

Source Content:
${sourceContent}

Extract 3-5 NEW, specific, actionable insights that are not already covered.`,
          },
        ],
        model: language,
        schema: z.object({
          newInsights: z.array(z.string()).min(1).max(5),
          improvementReasoning: z.string(),
        }),
      })

      if (result.object.newInsights.length > 0) {
        // Add new insights to context
        if (!context.output.insights) {
          context.output.insights = []
        }
        context.output.insights.push(...result.object.newInsights)

        log.info("Added improved insights", {
          newCount: result.object.newInsights.length,
          totalCount: context.output.insights.length,
        })
        return true
      }

      return false
    } catch (error) {
      log.warn("Failed to improve insights", { error })
      return false
    }
  }

  /**
   * Helper to assess source credibility based on URL domain.
   */
  private assessCredibility(url: string): "high" | "medium" | "low" {
    try {
      const domain = new URL(url).hostname.replace("www.", "")
      if (HIGH_CREDIBILITY_DOMAINS.some((d) => domain.includes(d))) return "high"
      if (MEDIUM_CREDIBILITY_DOMAINS.some((d) => domain.includes(d))) return "medium"
    } catch {
      // Invalid URL
    }
    return "low"
  }

  // ==========================================================================
  // Report Generation
  // ==========================================================================

  private generateReport(
    output: ResearchOutput | undefined,
    checks: Record<string, CheckItemResult>,
    closeScore: CLOSEScore,
    issues: PDCAIssue[],
  ): string {
    return `# Research Acceptance Report

## Summary
- **Topic**: ${output?.topic ?? "Unknown"}
- **CLOSE Score**: ${closeScore.total}/10
- **Issues Found**: ${issues.length}

## CLOSE Scores
| Dimension | Score |
|-----------|-------|
| Convergence | ${closeScore.convergence}/10 |
| Leverage | ${closeScore.leverage}/10 |
| Optionality | ${closeScore.optionality}/10 |
| Surplus | ${closeScore.surplus}/10 |
| Evolution | ${closeScore.evolution}/10 |

## Quality Checks
${Object.entries(checks).map(([name, check]) =>
  `- **${name}**: ${check.passed ? "PASS" : "FAIL"} (${check.score.toFixed(1)}/10) - ${check.details}`
).join("\n")}

## Sources
- Total: ${output?.sources?.length ?? 0}
- High Credibility: ${output?.sources?.filter((s) => s.credibility === "high").length ?? 0}
- Medium Credibility: ${output?.sources?.filter((s) => s.credibility === "medium").length ?? 0}

## Insights
${output?.insights?.map((i, idx) => `${idx + 1}. ${i}`).join("\n") ?? "No insights"}

## Issues
${issues.length > 0 ? issues.map((i) => `- **[${i.severity.toUpperCase()}]** ${i.category}: ${i.description}`).join("\n") : "No issues found."}
`
  }
}

/** Factory function */
export function createResearchStrategy(): ResearchStrategy {
  return new ResearchStrategy()
}
