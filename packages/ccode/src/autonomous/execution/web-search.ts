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
import * as TurndownModule from "turndown"
const TurndownService = TurndownModule.default || TurndownModule

const log = Log.create({ service: "autonomous.execution.web-search" })

// ============================================================================
// Exa MCP API Configuration
// ============================================================================

const EXA_API_CONFIG = {
  BASE_URL: "https://mcp.exa.ai",
  ENDPOINTS: {
    SEARCH: "/mcp",
  },
  DEFAULT_NUM_RESULTS: 5,
  TIMEOUT_MS: 25000,
} as const

interface ExaMcpSearchRequest {
  jsonrpc: string
  id: number
  method: string
  params: {
    name: string
    arguments: {
      query: string
      numResults?: number
      livecrawl?: "fallback" | "preferred"
      type?: "auto" | "fast" | "deep"
      contextMaxCharacters?: number
    }
  }
}

interface ExaMcpSearchResponse {
  jsonrpc: string
  result: {
    content: Array<{
      type: string
      text: string
    }>
  }
}

interface ExaSearchResultItem {
  title: string
  url: string
  text?: string
  highlights?: string[]
  score?: number
}

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
   * Perform a single search query using Exa MCP API
   */
  private async performSearch(query: string): Promise<SearchResult[]> {
    log.debug("Performing search via Exa MCP API", { query })

    const searchRequest: ExaMcpSearchRequest = {
      jsonrpc: "2.0",
      id: Date.now(),
      method: "tools/call",
      params: {
        name: "web_search_exa",
        arguments: {
          query,
          type: "auto",
          numResults: EXA_API_CONFIG.DEFAULT_NUM_RESULTS,
          livecrawl: "fallback",
          contextMaxCharacters: 5000,
        },
      },
    }

    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), EXA_API_CONFIG.TIMEOUT_MS)

    try {
      const response = await fetch(`${EXA_API_CONFIG.BASE_URL}${EXA_API_CONFIG.ENDPOINTS.SEARCH}`, {
        method: "POST",
        headers: {
          accept: "application/json, text/event-stream",
          "content-type": "application/json",
        },
        body: JSON.stringify(searchRequest),
        signal: controller.signal,
      })

      clearTimeout(timeoutId)

      if (!response.ok) {
        const errorText = await response.text()
        log.warn("Exa search request failed", { status: response.status, error: errorText })
        return []
      }

      const responseText = await response.text()

      // Parse SSE response
      const lines = responseText.split("\n")
      for (const line of lines) {
        if (line.startsWith("data: ")) {
          try {
            const data: ExaMcpSearchResponse = JSON.parse(line.substring(6))
            if (data.result?.content?.[0]?.text) {
              return this.parseExaResults(data.result.content[0].text, query)
            }
          } catch (parseError) {
            log.warn("Failed to parse Exa response", { error: parseError })
          }
        }
      }

      log.debug("No search results found", { query })
      return []
    } catch (error) {
      clearTimeout(timeoutId)

      if (error instanceof Error && error.name === "AbortError") {
        log.warn("Search request timed out", { query })
      } else {
        log.warn("Search request failed", { query, error: error instanceof Error ? error.message : String(error) })
      }

      return []
    }
  }

  /**
   * Parse Exa API response text into SearchResult array
   */
  private parseExaResults(responseText: string, query: string): SearchResult[] {
    const results: SearchResult[] = []

    try {
      // Try to parse as JSON first (structured response)
      const parsed = JSON.parse(responseText)
      if (Array.isArray(parsed)) {
        for (const item of parsed as ExaSearchResultItem[]) {
          results.push({
            url: item.url,
            title: item.title || "Untitled",
            snippet: item.text?.slice(0, 300) || item.highlights?.join(" ") || "",
            source: this.detectSource(item.url),
            relevanceScore: item.score ?? 0.7,
          })
        }
        return results
      }
    } catch {
      // Not JSON, parse as text
    }

    // Parse as markdown/text format (Links: [...] format)
    const linksMatch = responseText.match(/Links:\s*(\[[\s\S]*?\])/i)
    if (linksMatch) {
      try {
        const links = JSON.parse(linksMatch[1])
        for (const link of links) {
          results.push({
            url: link.url,
            title: link.title || "Untitled",
            snippet: link.snippet || link.description || "",
            source: this.detectSource(link.url),
            relevanceScore: 0.7,
          })
        }
        return results
      } catch {
        // Continue with text parsing
      }
    }

    // Extract URLs from plain text response
    const urlRegex = /https?:\/\/[^\s\])"']+/g
    const urls = responseText.match(urlRegex) || []
    const uniqueUrls = Array.from(new Set(urls)).slice(0, 5)

    for (const url of uniqueUrls) {
      results.push({
        url,
        title: `Search result for: ${query.slice(0, 50)}`,
        snippet: "Extracted from search results",
        source: this.detectSource(url),
        relevanceScore: 0.6,
      })
    }

    return results
  }

  /**
   * Detect source type from URL
   */
  private detectSource(url: string): SearchResult["source"] {
    const lowerUrl = url.toLowerCase()

    if (lowerUrl.includes("stackoverflow.com") || lowerUrl.includes("stackexchange.com")) {
      return "stackoverflow"
    }
    if (lowerUrl.includes("github.com") || lowerUrl.includes("gitlab.com")) {
      return "github"
    }
    if (
      lowerUrl.includes("docs.") ||
      lowerUrl.includes("/docs/") ||
      lowerUrl.includes("developer.") ||
      lowerUrl.includes("documentation")
    ) {
      return "documentation"
    }

    return "other"
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

      log.debug("Fetching content from URL", { url })

      const controller = new AbortController()
      const timeoutId = setTimeout(() => controller.abort(), this.config.fetchTimeout)

      try {
        const response = await fetch(url, {
          signal: controller.signal,
          headers: {
            "User-Agent": "CodeCoder/1.0 (Autonomous Agent; +https://codecoder.dev)",
            Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
          },
        })

        clearTimeout(timeoutId)

        if (!response.ok) {
          log.debug("Failed to fetch URL", { url, status: response.status })
          return null
        }

        const html = await response.text()
        const contentType = response.headers.get("content-type") || ""

        // Convert HTML to markdown for better analysis
        let content = html
        if (contentType.includes("text/html")) {
          content = this.convertToMarkdown(html)
        }

        // Limit content size
        content = content.slice(0, 50000)

        // Extract relevant sections
        const relevantSections = this.extractRelevantSections(content, problem)

        // Generate summary
        const summary = this.generateSummary(content, url, problem)

        // Calculate confidence based on content quality
        const confidence = this.calculateContentConfidence(content, relevantSections, problem)

        return {
          url,
          content,
          summary,
          relevantSections,
          confidence,
        }
      } finally {
        clearTimeout(timeoutId)
      }
    } catch (error) {
      log.warn("Failed to fetch content", {
        url,
        error: error instanceof Error ? error.message : String(error),
      })
      return null
    }
  }

  /**
   * Convert HTML to markdown for easier analysis
   */
  private convertToMarkdown(html: string): string {
    try {
      const turndownService = new TurndownService({
        headingStyle: "atx",
        hr: "---",
        bulletListMarker: "-",
        codeBlockStyle: "fenced",
        emDelimiter: "*",
      })
      turndownService.remove(["script", "style", "meta", "link", "nav", "footer", "header"])
      return turndownService.turndown(html)
    } catch {
      // Fallback: strip HTML tags
      return html
        .replace(/<script[^>]*>[\s\S]*?<\/script>/gi, "")
        .replace(/<style[^>]*>[\s\S]*?<\/style>/gi, "")
        .replace(/<[^>]+>/g, " ")
        .replace(/\s+/g, " ")
        .trim()
    }
  }

  /**
   * Extract sections relevant to the problem
   */
  private extractRelevantSections(content: string, problem: string): string[] {
    const sections: string[] = []
    const problemKeywords = problem
      .toLowerCase()
      .split(/\s+/)
      .filter((w) => w.length > 3)

    // Split by headings
    const headingPattern = /^#{1,3}\s+(.+)$/gm
    let match: RegExpExecArray | null
    const headings: Array<{ title: string; index: number }> = []

    while ((match = headingPattern.exec(content)) !== null) {
      headings.push({ title: match[1], index: match.index })
    }

    // Find sections containing problem keywords
    for (let i = 0; i < headings.length; i++) {
      const start = headings[i].index
      const end = headings[i + 1]?.index ?? content.length
      const sectionContent = content.slice(start, end).toLowerCase()

      const keywordMatches = problemKeywords.filter((kw) => sectionContent.includes(kw)).length
      if (keywordMatches >= 2 || sectionContent.includes("example") || sectionContent.includes("solution")) {
        sections.push(headings[i].title)
      }
    }

    // Fallback: look for common useful sections
    const commonSections = ["Getting Started", "Installation", "Usage", "Examples", "API", "Error", "Troubleshooting"]
    for (const cs of commonSections) {
      if (content.toLowerCase().includes(cs.toLowerCase()) && !sections.some((s) => s.toLowerCase().includes(cs.toLowerCase()))) {
        sections.push(cs)
      }
    }

    return sections.slice(0, 10)
  }

  /**
   * Generate a summary of the content
   */
  private generateSummary(content: string, url: string, problem: string): string {
    const hostname = new URL(url).hostname

    // Extract first meaningful paragraph
    const paragraphs = content.split(/\n\n+/).filter((p) => p.length > 50 && !p.startsWith("#"))
    const firstParagraph = paragraphs[0]?.slice(0, 200) || ""

    return `Documentation from ${hostname}: ${firstParagraph}...`
  }

  /**
   * Calculate confidence based on content quality and relevance
   */
  private calculateContentConfidence(content: string, sections: string[], problem: string): number {
    let confidence = 0.5

    // Content length contributes
    if (content.length > 1000) confidence += 0.1
    if (content.length > 5000) confidence += 0.1

    // Code blocks indicate practical solutions
    const codeBlockCount = (content.match(/```/g) || []).length / 2
    confidence += Math.min(codeBlockCount / 10, 0.15)

    // Relevant sections found
    confidence += Math.min(sections.length / 10, 0.1)

    // Problem keywords in content
    const problemKeywords = problem
      .toLowerCase()
      .split(/\s+/)
      .filter((w) => w.length > 3)
    const keywordMatches = problemKeywords.filter((kw) => content.toLowerCase().includes(kw)).length
    confidence += Math.min(keywordMatches / problemKeywords.length, 0.15)

    return Math.min(confidence, 1.0)
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
