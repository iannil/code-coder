/**
 * Enhanced Web Search with Actual Fetch
 *
 * Extends the web search capability with real web fetching using Bun's fetch API.
 * Provides documentation lookup, Stack Overflow search, and content extraction.
 *
 * Part of Phase 3: Autonomous Problem-Solving Loop
 *
 * This implements Step 1 of the evolution cycle:
 * - Automatic documentation lookup when confidence is low
 * - Real web content fetching and parsing
 * - Structured information extraction
 */

import { createWebSearcher, type SearchResult, type FetchedContent, type SearchContext } from "./web-search"

// ============================================================================
// Types
// ============================================================================

/** Result from actual web fetch */
export interface WebFetchResult {
  url: string
  statusCode: number
  content: string
  contentType: string
  fetchedAt: string
  duration: number
}

/** Extracted documentation */
export interface ExtractedDoc {
  title: string
  sections: Array<{
    heading: string
    content: string
  }>
  codeBlocks: Array<{
    language: string
    code: string
  }>
  links: string[]
}

// ============================================================================
// Enhanced Web Fetcher
// ============================================================================

/**
 * Enhanced web search with actual fetch capability
 */
export class EnhancedWebSearch {
  private sessionId: string
  private baseSearcher: ReturnType<typeof createWebSearcher>
  private fetchCache: Map<string, WebFetchResult> = new Map()
  private cacheMaxAge = 15 * 60 * 1000 // 15 minutes

  constructor(sessionId: string) {
    this.sessionId = sessionId
    this.baseSearcher = createWebSearcher(sessionId)
  }

  /**
   * Fetch a URL and extract content
   */
  async fetchUrl(url: string, timeout = 10000): Promise<WebFetchResult> {
    // Check cache
    const cached = this.fetchCache.get(url)
    if (cached) {
      const age = Date.now() - new Date(cached.fetchedAt).getTime()
      if (age < this.cacheMaxAge) {
        return cached
      }
    }

    const startTime = Date.now()
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), timeout)

    try {
      const response = await fetch(url, {
        signal: controller.signal,
        headers: {
          "User-Agent": "CodeCoder/1.0 (Autonomous Agent; +https://codecoder.dev)",
          Accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        },
      })

      const content = await response.text()
      const contentType = response.headers.get("content-type") ?? "text/html"

      const result: WebFetchResult = {
        url,
        statusCode: response.status,
        content: content.slice(0, 500000), // Limit to 500KB
        contentType,
        fetchedAt: new Date().toISOString(),
        duration: Date.now() - startTime,
      }

      // Cache successful fetches
      if (response.ok) {
        this.fetchCache.set(url, result)
      }

      return result
    } finally {
      clearTimeout(timeoutId)
    }
  }

  /**
   * Fetch and extract documentation from a URL
   */
  async fetchDocumentation(url: string): Promise<FetchedContent | null> {
    try {
      const result = await this.fetchUrl(url)

      if (result.statusCode !== 200) {
        return null
      }

      // Extract structured content
      const extracted = this.extractDocContent(result.content, result.url)

      // Calculate relevance sections
      const relevantSections = extracted.sections
        .filter((s) => s.content.length > 50)
        .map((s) => s.heading)
        .slice(0, 5)

      return {
        url: result.url,
        content: extracted.sections.map((s) => `## ${s.heading}\n${s.content}`).join("\n\n"),
        summary: `Documentation from ${new URL(url).hostname}: ${extracted.title}`,
        relevantSections,
        confidence: this.calculateDocConfidence(extracted),
      }
    } catch (error) {
      console.error(`Failed to fetch documentation from ${url}:`, error)
      return null
    }
  }

  /**
   * Extract structured content from HTML
   */
  private extractDocContent(html: string, _url: string): ExtractedDoc {
    const sections: ExtractedDoc["sections"] = []
    const codeBlocks: ExtractedDoc["codeBlocks"] = []
    const links: string[] = []

    // Extract title
    const titleMatch = html.match(/<title[^>]*>([^<]+)<\/title>/i)
    const title = titleMatch ? this.cleanHtml(titleMatch[1]) : "Untitled"

    // Extract headings and their content
    const headingRegex = /<h([1-6])[^>]*>([^<]+)<\/h\1>/gi
    let lastIndex = 0
    let match: RegExpExecArray | null

    while ((match = headingRegex.exec(html)) !== null) {
      const heading = this.cleanHtml(match[2])
      const startIndex = match.index + match[0].length
      const nextHeadingMatch = html.slice(startIndex).match(/<h[1-6][^>]*>/i)
      const endIndex = nextHeadingMatch ? startIndex + (nextHeadingMatch.index ?? 0) : html.length

      const sectionHtml = html.slice(startIndex, endIndex)
      const content = this.extractTextFromHtml(sectionHtml)

      if (content.length > 20) {
        sections.push({ heading, content: content.slice(0, 2000) })
      }

      lastIndex = endIndex
    }

    // Extract code blocks
    const codeRegex = /<pre[^>]*><code[^>]*(?:class="[^"]*language-(\w+)[^"]*")?[^>]*>([\s\S]*?)<\/code><\/pre>/gi
    while ((match = codeRegex.exec(html)) !== null) {
      const language = match[1] ?? "text"
      const code = this.cleanHtml(match[2])

      if (code.length > 10) {
        codeBlocks.push({ language, code })
      }
    }

    // Also check for simple <code> blocks
    const simpleCodeRegex = /<code[^>]*>([\s\S]*?)<\/code>/gi
    while ((match = simpleCodeRegex.exec(html)) !== null) {
      const code = this.cleanHtml(match[1])
      if (code.length > 50 && code.includes("\n")) {
        codeBlocks.push({ language: "text", code })
      }
    }

    // Extract links
    const linkRegex = /href="(https?:\/\/[^"]+)"/gi
    while ((match = linkRegex.exec(html)) !== null) {
      links.push(match[1])
    }

    return {
      title,
      sections: sections.slice(0, 20),
      codeBlocks: codeBlocks.slice(0, 10),
      links: [...new Set(links)].slice(0, 20),
    }
  }

  /**
   * Extract plain text from HTML
   */
  private extractTextFromHtml(html: string): string {
    return (
      html
        // Remove script and style tags
        .replace(/<script[^>]*>[\s\S]*?<\/script>/gi, "")
        .replace(/<style[^>]*>[\s\S]*?<\/style>/gi, "")
        // Remove HTML tags
        .replace(/<[^>]+>/g, " ")
        // Decode HTML entities
        .replace(/&nbsp;/g, " ")
        .replace(/&lt;/g, "<")
        .replace(/&gt;/g, ">")
        .replace(/&amp;/g, "&")
        .replace(/&quot;/g, '"')
        .replace(/&#39;/g, "'")
        // Clean whitespace
        .replace(/\s+/g, " ")
        .trim()
    )
  }

  /**
   * Clean HTML entities and tags from text
   */
  private cleanHtml(text: string): string {
    return text
      .replace(/<[^>]+>/g, "")
      .replace(/&nbsp;/g, " ")
      .replace(/&lt;/g, "<")
      .replace(/&gt;/g, ">")
      .replace(/&amp;/g, "&")
      .replace(/&quot;/g, '"')
      .replace(/&#39;/g, "'")
      .replace(/&#(\d+);/g, (_, code) => String.fromCharCode(parseInt(code, 10)))
      .trim()
  }

  /**
   * Calculate confidence score for extracted documentation
   */
  private calculateDocConfidence(doc: ExtractedDoc): number {
    let confidence = 0.5 // Base confidence

    // More sections = more comprehensive
    confidence += Math.min(doc.sections.length / 20, 0.2)

    // Code examples increase confidence
    confidence += Math.min(doc.codeBlocks.length / 10, 0.2)

    // Longer content = more detailed
    const totalContent = doc.sections.reduce((sum, s) => sum + s.content.length, 0)
    confidence += Math.min(totalContent / 10000, 0.1)

    return Math.min(confidence, 1.0)
  }

  /**
   * Search documentation sites for a topic
   */
  async searchDocumentation(query: string, technology?: string): Promise<FetchedContent[]> {
    const results: FetchedContent[] = []

    // Build documentation URLs based on technology
    const docUrls = this.getDocumentationUrls(query, technology)

    // Fetch in parallel (limit concurrency)
    const fetchPromises = docUrls.map((url) => this.fetchDocumentation(url))
    const fetched = await Promise.allSettled(fetchPromises)

    for (const result of fetched) {
      if (result.status === "fulfilled" && result.value) {
        results.push(result.value)
      }
    }

    return results
  }

  /**
   * Get relevant documentation URLs for a query
   */
  private getDocumentationUrls(query: string, technology?: string): string[] {
    const urls: string[] = []
    const encodedQuery = encodeURIComponent(query)

    // Generic documentation sites
    urls.push(`https://developer.mozilla.org/en-US/search?q=${encodedQuery}`)

    // Technology-specific documentation
    if (technology) {
      const tech = technology.toLowerCase()

      if (tech.includes("typescript") || tech.includes("javascript")) {
        urls.push(`https://www.typescriptlang.org/docs/handbook/${encodedQuery}.html`)
        urls.push(`https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/${encodedQuery}`)
      }

      if (tech.includes("python")) {
        urls.push(`https://docs.python.org/3/search.html?q=${encodedQuery}`)
      }

      if (tech.includes("rust")) {
        urls.push(`https://doc.rust-lang.org/std/${encodedQuery}/index.html`)
        urls.push(`https://docs.rs/search?query=${encodedQuery}`)
      }

      if (tech.includes("go")) {
        urls.push(`https://pkg.go.dev/search?q=${encodedQuery}`)
      }

      if (tech.includes("react")) {
        urls.push(`https://react.dev/reference/react/${encodedQuery}`)
      }

      if (tech.includes("node") || tech.includes("bun")) {
        urls.push(`https://nodejs.org/api/${encodedQuery}.html`)
        urls.push(`https://bun.sh/docs/${encodedQuery}`)
      }
    }

    return urls.slice(0, 5) // Limit to 5 URLs
  }

  /**
   * Search and fetch with enhanced capabilities
   */
  async searchAndFetch(context: SearchContext): Promise<{
    searchResults: SearchResult[]
    fetchedContent: FetchedContent[]
  }> {
    // Use base searcher for search suggestions
    const decision = await this.baseSearcher.evaluateSearchNeed(context)

    if (!decision.shouldSearch) {
      return { searchResults: [], fetchedContent: [] }
    }

    // Search documentation
    const fetchedContent = await this.searchDocumentation(context.problem, context.technology)

    // Convert to search results format
    const searchResults: SearchResult[] = fetchedContent.map((fc) => ({
      url: fc.url,
      title: fc.summary,
      snippet: fc.content.slice(0, 200),
      source: "documentation" as const,
      relevanceScore: fc.confidence,
    }))

    return { searchResults, fetchedContent }
  }

  /**
   * Clear the fetch cache
   */
  clearCache(): void {
    this.fetchCache.clear()
  }
}

// ============================================================================
// Factory
// ============================================================================

/**
 * Create an enhanced web search instance
 */
export function createEnhancedWebSearch(sessionId: string): EnhancedWebSearch {
  return new EnhancedWebSearch(sessionId)
}
