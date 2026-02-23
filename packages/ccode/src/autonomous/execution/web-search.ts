/**
 * Web Search Integration for Autonomous Mode
 *
 * Provides retrieval-augmented problem solving capabilities:
 * - Automatic documentation lookup for unknown errors
 * - Official documentation fetching for unfamiliar libraries
 * - Stack Overflow search for common issues
 * - Knowledge confidence scoring
 */

import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"

const log = Log.create({ service: "autonomous.execution.web-search" })

// ============================================================================
// Types
// ============================================================================

/** Search result from web search */
export interface SearchResult {
  url: string
  title: string
  snippet: string
  source: "documentation" | "stackoverflow" | "github" | "other"
  relevanceScore: number
}

/** Fetched content with analysis */
export interface FetchedContent {
  url: string
  content: string
  summary: string
  relevantSections: string[]
  confidence: number
}

/** Web search configuration */
export interface WebSearchConfig {
  /** Minimum confidence threshold to consider searching (0-1) */
  confidenceThreshold: number
  /** Maximum number of search results to fetch */
  maxResults: number
  /** Timeout for fetch operations in ms */
  fetchTimeout: number
  /** Allowed domains for documentation */
  trustedDomains: string[]
  /** Whether to enable Stack Overflow search */
  enableStackOverflow: boolean
}

/** Search context for a problem */
export interface SearchContext {
  sessionId: string
  problem: string
  errorMessage?: string
  technology?: string
  previousAttempts?: string[]
}

/** Search decision result */
export interface SearchDecision {
  shouldSearch: boolean
  reason: string
  confidence: number
  suggestedQueries: string[]
}

/** Solution from web search */
export interface WebSolution {
  query: string
  sources: SearchResult[]
  fetchedContent: FetchedContent[]
  synthesizedSolution: string
  confidence: number
  codeSnippets: Array<{
    language: string
    code: string
    source: string
  }>
}

// ============================================================================
// Default Configuration
// ============================================================================

const DEFAULT_CONFIG: WebSearchConfig = {
  confidenceThreshold: 0.4,
  maxResults: 5,
  fetchTimeout: 10000,
  trustedDomains: [
    "docs.python.org",
    "developer.mozilla.org",
    "nodejs.org",
    "typescriptlang.org",
    "rust-lang.org",
    "doc.rust-lang.org",
    "docs.rs",
    "go.dev",
    "pkg.go.dev",
    "react.dev",
    "nextjs.org",
    "bun.sh",
    "deno.land",
    "github.com",
    "stackoverflow.com",
  ],
  enableStackOverflow: true,
}

// ============================================================================
// Web Searcher
// ============================================================================

/**
 * Web search integration for autonomous problem solving
 */
export class WebSearcher {
  private config: WebSearchConfig
  private sessionId: string
  private searchHistory: Map<string, WebSolution> = new Map()

  constructor(sessionId: string, config: Partial<WebSearchConfig> = {}) {
    this.sessionId = sessionId
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Decide whether to search based on confidence and context
   */
  async evaluateSearchNeed(context: SearchContext): Promise<SearchDecision> {
    const { problem, errorMessage, technology, previousAttempts } = context

    // Calculate base confidence from problem characteristics
    let confidence = 1.0

    // Lower confidence for unknown errors
    if (errorMessage) {
      confidence -= 0.3
      if (errorMessage.includes("unknown") || errorMessage.includes("not found")) {
        confidence -= 0.2
      }
    }

    // Lower confidence for unfamiliar technologies
    const knownTechnologies = ["typescript", "javascript", "rust", "go", "python", "react", "nextjs"]
    if (technology && !knownTechnologies.some((t) => technology.toLowerCase().includes(t))) {
      confidence -= 0.3
    }

    // Lower confidence if previous attempts failed
    if (previousAttempts && previousAttempts.length > 0) {
      confidence -= 0.1 * Math.min(previousAttempts.length, 3)
    }

    confidence = Math.max(0, Math.min(1, confidence))

    const shouldSearch = confidence < this.config.confidenceThreshold

    // Generate search queries
    const suggestedQueries = this.generateSearchQueries(problem, errorMessage, technology)

    const decision: SearchDecision = {
      shouldSearch,
      reason: shouldSearch
        ? `Confidence ${(confidence * 100).toFixed(0)}% below threshold ${(this.config.confidenceThreshold * 100).toFixed(0)}%`
        : `Confidence ${(confidence * 100).toFixed(0)}% sufficient to proceed`,
      confidence,
      suggestedQueries,
    }

    log.info("Search need evaluated", {
      sessionId: this.sessionId,
      shouldSearch,
      confidence,
      queryCount: suggestedQueries.length,
    })

    return decision
  }

  /**
   * Generate search queries from problem context
   */
  private generateSearchQueries(problem: string, errorMessage?: string, technology?: string): string[] {
    const queries: string[] = []

    // Direct error query
    if (errorMessage) {
      const cleanError = errorMessage.replace(/['"]/g, "").slice(0, 100)
      queries.push(cleanError)

      // Add technology context if available
      if (technology) {
        queries.push(`${technology} ${cleanError}`)
      }
    }

    // Extract keywords from problem
    const keywords = problem
      .toLowerCase()
      .replace(/[^\w\s]/g, " ")
      .split(/\s+/)
      .filter((w) => w.length > 3)
      .slice(0, 5)

    if (keywords.length >= 2) {
      queries.push(keywords.join(" "))
    }

    // Documentation query
    if (technology) {
      queries.push(`${technology} documentation ${keywords.slice(0, 2).join(" ")}`)
    }

    return [...new Set(queries)].slice(0, 3)
  }

  /**
   * Search for solutions using web search
   */
  async search(context: SearchContext): Promise<WebSolution> {
    const startTime = Date.now()
    const cacheKey = this.getCacheKey(context)

    // Check cache
    const cached = this.searchHistory.get(cacheKey)
    if (cached) {
      log.debug("Returning cached search result", { sessionId: this.sessionId })
      return cached
    }

    log.info("Starting web search", {
      sessionId: this.sessionId,
      problem: context.problem.slice(0, 100),
    })

    // Evaluate search need
    const decision = await this.evaluateSearchNeed(context)

    if (!decision.shouldSearch) {
      log.info("Search not needed based on confidence", { confidence: decision.confidence })
      return {
        query: "",
        sources: [],
        fetchedContent: [],
        synthesizedSolution: "Proceeding without web search based on confidence level.",
        confidence: decision.confidence,
        codeSnippets: [],
      }
    }

    // Perform searches
    const allResults: SearchResult[] = []
    for (const query of decision.suggestedQueries) {
      try {
        const results = await this.performSearch(query)
        allResults.push(...results)
      } catch (error) {
        log.warn("Search query failed", {
          query,
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }

    // Deduplicate and rank results
    const rankedResults = this.rankResults(allResults)
    const topResults = rankedResults.slice(0, this.config.maxResults)

    // Fetch content from top results
    const fetchedContent: FetchedContent[] = []
    for (const result of topResults.slice(0, 3)) {
      try {
        const content = await this.fetchAndAnalyze(result.url, context.problem)
        if (content) {
          fetchedContent.push(content)
        }
      } catch (error) {
        log.warn("Content fetch failed", {
          url: result.url,
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }

    // Synthesize solution
    const solution = await this.synthesizeSolution(context, topResults, fetchedContent)

    // Cache result
    this.searchHistory.set(cacheKey, solution)

    // Publish event
    await Bus.publish(AutonomousEvent.WebSearchCompleted, {
      sessionId: this.sessionId,
      queriesRun: decision.suggestedQueries.length,
      resultsFound: allResults.length,
      contentFetched: fetchedContent.length,
      solutionConfidence: solution.confidence,
      duration: Date.now() - startTime,
    })

    log.info("Web search completed", {
      sessionId: this.sessionId,
      resultsFound: topResults.length,
      contentFetched: fetchedContent.length,
      confidence: solution.confidence,
      duration: Date.now() - startTime,
    })

    return solution
  }

  /**
   * Perform a single search query
   */
  private async performSearch(query: string): Promise<SearchResult[]> {
    // In a real implementation, this would call a search API
    // For now, we simulate with structured results based on query

    log.debug("Performing search", { query })

    // Simulated search - in production, use WebSearch tool or API
    const results: SearchResult[] = []

    // Check for documentation patterns
    if (query.includes("documentation") || query.includes("docs")) {
      results.push({
        url: `https://developer.mozilla.org/search?q=${encodeURIComponent(query)}`,
        title: `MDN Documentation: ${query}`,
        snippet: "Official documentation and guides...",
        source: "documentation",
        relevanceScore: 0.9,
      })
    }

    // Check for error patterns
    if (query.includes("error") || query.includes("Error")) {
      results.push({
        url: `https://stackoverflow.com/search?q=${encodeURIComponent(query)}`,
        title: `Stack Overflow: ${query}`,
        snippet: "Community solutions for this error...",
        source: "stackoverflow",
        relevanceScore: 0.8,
      })
    }

    // GitHub search for code patterns
    if (query.includes("example") || query.includes("how to")) {
      results.push({
        url: `https://github.com/search?q=${encodeURIComponent(query)}&type=code`,
        title: `GitHub Code Search: ${query}`,
        snippet: "Code examples from open source projects...",
        source: "github",
        relevanceScore: 0.7,
      })
    }

    return results
  }

  /**
   * Fetch and analyze content from a URL
   */
  private async fetchAndAnalyze(url: string, problem: string): Promise<FetchedContent | null> {
    try {
      // Check if domain is trusted
      const urlObj = new URL(url)
      const isTrusted = this.config.trustedDomains.some((d) => urlObj.hostname.includes(d))

      if (!isTrusted) {
        log.debug("Skipping untrusted domain", { url })
        return null
      }

      // In a real implementation, this would fetch the URL
      // For now, return a placeholder
      return {
        url,
        content: `Content from ${url}`,
        summary: `Summary of documentation relevant to: ${problem.slice(0, 50)}...`,
        relevantSections: ["Getting Started", "API Reference", "Examples"],
        confidence: 0.7,
      }
    } catch (error) {
      log.warn("Failed to fetch content", { url, error })
      return null
    }
  }

  /**
   * Rank and deduplicate search results
   */
  private rankResults(results: SearchResult[]): SearchResult[] {
    // Deduplicate by URL
    const urlMap = new Map<string, SearchResult>()
    for (const result of results) {
      const existing = urlMap.get(result.url)
      if (!existing || result.relevanceScore > existing.relevanceScore) {
        urlMap.set(result.url, result)
      }
    }

    // Sort by relevance score
    return Array.from(urlMap.values()).sort((a, b) => b.relevanceScore - a.relevanceScore)
  }

  /**
   * Synthesize a solution from search results
   */
  private async synthesizeSolution(
    context: SearchContext,
    results: SearchResult[],
    fetchedContent: FetchedContent[],
  ): Promise<WebSolution> {
    const codeSnippets: WebSolution["codeSnippets"] = []

    // Extract code snippets from fetched content
    for (const content of fetchedContent) {
      // In a real implementation, parse code blocks from content
      // For now, create placeholder
      if (content.relevantSections.includes("Examples")) {
        codeSnippets.push({
          language: "typescript",
          code: "// Example code from documentation",
          source: content.url,
        })
      }
    }

    // Calculate overall confidence
    const avgConfidence =
      fetchedContent.length > 0 ? fetchedContent.reduce((sum, c) => sum + c.confidence, 0) / fetchedContent.length : 0

    // Synthesize solution text
    let synthesizedSolution = ""

    if (results.length === 0) {
      synthesizedSolution = "No relevant search results found. Proceeding with current knowledge."
    } else if (fetchedContent.length === 0) {
      synthesizedSolution = `Found ${results.length} potential sources but could not fetch content. Consider visiting: ${results
        .slice(0, 3)
        .map((r) => r.url)
        .join(", ")}`
    } else {
      const summaries = fetchedContent.map((c) => c.summary).join("\n\n")
      synthesizedSolution = `Based on ${fetchedContent.length} sources:\n\n${summaries}\n\nRelevant sections: ${fetchedContent
        .flatMap((c) => c.relevantSections)
        .slice(0, 5)
        .join(", ")}`
    }

    return {
      query: context.problem,
      sources: results,
      fetchedContent,
      synthesizedSolution,
      confidence: avgConfidence,
      codeSnippets,
    }
  }

  /**
   * Get cache key for search context
   */
  private getCacheKey(context: SearchContext): string {
    return `${context.problem}:${context.errorMessage ?? ""}:${context.technology ?? ""}`
  }

  /**
   * Clear search cache
   */
  clearCache(): void {
    this.searchHistory.clear()
  }
}

// ============================================================================
// Factory
// ============================================================================

/**
 * Create a web searcher instance
 */
export function createWebSearcher(sessionId: string, config?: Partial<WebSearchConfig>): WebSearcher {
  return new WebSearcher(sessionId, config)
}

// ============================================================================
// Error Recovery Helper
// ============================================================================

/**
 * Search for solution to an error
 */
export async function searchForErrorSolution(
  sessionId: string,
  error: string,
  context: {
    technology?: string
    file?: string
    previousAttempts?: string[]
  },
): Promise<WebSolution> {
  const searcher = createWebSearcher(sessionId)

  return searcher.search({
    sessionId,
    problem: `Error while working on ${context.file ?? "unknown file"}`,
    errorMessage: error,
    technology: context.technology,
    previousAttempts: context.previousAttempts,
  })
}
