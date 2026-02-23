/**
 * Knowledge Sedimentation System
 *
 * Extracts and stores learned solutions from autonomous problem-solving sessions.
 * Integrates with Zero-Memory for long-term knowledge retention.
 *
 * Part of Phase 3: Autonomous Problem-Solving Loop
 *
 * This implements Step 4 of the evolution cycle:
 * - Extract structured experience from solved problems
 * - Store in Zero-Memory vector database
 * - Enable retrieval for similar future problems
 */

import path from "path"
import fs from "fs/promises"
import { Global } from "@/global"

// ============================================================================
// Types
// ============================================================================

/** Category of knowledge for organization */
export type KnowledgeCategory =
  | "error_solution" // Solutions to specific errors
  | "api_pattern" // API usage patterns
  | "code_snippet" // Reusable code snippets
  | "architecture" // Architectural decisions
  | "configuration" // Configuration examples
  | "debugging" // Debugging techniques
  | "performance" // Performance optimizations
  | "security" // Security patterns
  | "lesson_learned" // General lessons

/** A knowledge entry to be stored */
export interface KnowledgeEntry {
  /** Unique ID */
  id: string
  /** Category of knowledge */
  category: KnowledgeCategory
  /** Title/summary */
  title: string
  /** Full content */
  content: string
  /** Tags for search */
  tags: string[]
  /** Technology/language context */
  technology?: string
  /** Original problem description */
  problem?: string
  /** Solution that worked */
  solution?: string
  /** Code examples */
  codeExamples?: Array<{
    language: string
    code: string
    description?: string
  }>
  /** Source of this knowledge */
  source: KnowledgeSource
  /** Confidence in this knowledge (0-1) */
  confidence: number
  /** Number of times this was successfully applied */
  successCount: number
  /** Creation timestamp */
  createdAt: string
  /** Last updated timestamp */
  updatedAt: string
}

/** Source of knowledge entry */
export interface KnowledgeSource {
  /** Type of source */
  type: "autonomous_session" | "web_search" | "documentation" | "user_input" | "code_analysis"
  /** Session ID if from autonomous mode */
  sessionId?: string
  /** URL if from web */
  url?: string
  /** Timestamp of extraction */
  extractedAt: string
}

/** Extraction context from a solved problem */
export interface ExtractionContext {
  sessionId: string
  /** The original problem */
  problem: string
  /** Error message if applicable */
  errorMessage?: string
  /** Technology/language */
  technology?: string
  /** The solution that worked */
  solution: string
  /** Code that solved the problem */
  code?: string
  /** Steps taken to solve */
  steps?: string[]
  /** Web sources used */
  webSources?: Array<{ url: string; title: string }>
  /** Reflection on why it worked */
  reflection?: string
}

/** Search result from knowledge base */
export interface KnowledgeSearchResult {
  entry: KnowledgeEntry
  relevanceScore: number
  matchedTags: string[]
}

// ============================================================================
// Configuration
// ============================================================================

const KNOWLEDGE_DIR = path.join(Global.Path.data, "knowledge")
const KNOWLEDGE_FILE = path.join(KNOWLEDGE_DIR, "entries.json")

// ============================================================================
// Knowledge Sedimentation System
// ============================================================================

/**
 * System for extracting and storing learned knowledge from problem-solving sessions
 */
export class KnowledgeSedimentation {
  private entries: Map<string, KnowledgeEntry> = new Map()
  private initialized = false

  /**
   * Initialize the knowledge system
   */
  async initialize(): Promise<void> {
    if (this.initialized) return

    await fs.mkdir(KNOWLEDGE_DIR, { recursive: true })

    try {
      const content = await fs.readFile(KNOWLEDGE_FILE, "utf-8")
      const entries = JSON.parse(content) as KnowledgeEntry[]
      for (const entry of entries) {
        this.entries.set(entry.id, entry)
      }
    } catch {
      // Start with empty knowledge base
    }

    this.initialized = true
  }

  /**
   * Extract and sediment knowledge from a solved problem
   */
  async sediment(context: ExtractionContext): Promise<KnowledgeEntry> {
    await this.initialize()

    // Extract structured knowledge
    const entry = this.extractKnowledge(context)

    // Check for duplicates/similar entries
    const similar = await this.findSimilar(entry.title, entry.tags)
    if (similar.length > 0 && similar[0].relevanceScore > 0.9) {
      // Update existing entry instead of creating new
      const existing = similar[0].entry
      existing.successCount++
      existing.updatedAt = new Date().toISOString()

      // Merge code examples
      if (entry.codeExamples && existing.codeExamples) {
        for (const example of entry.codeExamples) {
          const isDuplicate = existing.codeExamples.some(
            (e) => e.code.trim() === example.code.trim()
          )
          if (!isDuplicate) {
            existing.codeExamples.push(example)
          }
        }
      }

      this.entries.set(existing.id, existing)
      await this.save()
      return existing
    }

    // Store new entry
    this.entries.set(entry.id, entry)
    await this.save()

    return entry
  }

  /**
   * Extract knowledge from context
   */
  private extractKnowledge(context: ExtractionContext): KnowledgeEntry {
    const now = new Date().toISOString()
    const id = `knowledge-${Date.now()}`

    // Determine category based on content
    const category = this.categorize(context)

    // Extract tags from problem and solution
    const tags = this.extractTags(context)

    // Build title
    const title = this.generateTitle(context, category)

    // Build content
    const content = this.formatContent(context)

    // Extract code examples
    const codeExamples = this.extractCodeExamples(context)

    return {
      id,
      category,
      title,
      content,
      tags,
      technology: context.technology,
      problem: context.problem,
      solution: context.solution,
      codeExamples,
      source: {
        type: "autonomous_session",
        sessionId: context.sessionId,
        extractedAt: now,
      },
      confidence: 0.8, // Initial confidence
      successCount: 1,
      createdAt: now,
      updatedAt: now,
    }
  }

  /**
   * Categorize the knowledge based on context
   */
  private categorize(context: ExtractionContext): KnowledgeCategory {
    const combined = `${context.problem} ${context.solution}`.toLowerCase()

    if (context.errorMessage) return "error_solution"
    if (combined.includes("api") || combined.includes("endpoint")) return "api_pattern"
    if (combined.includes("performance") || combined.includes("optimize")) return "performance"
    if (combined.includes("security") || combined.includes("auth")) return "security"
    if (combined.includes("config") || combined.includes("setup")) return "configuration"
    if (combined.includes("debug") || combined.includes("trace")) return "debugging"
    if (combined.includes("architecture") || combined.includes("design")) return "architecture"
    if (context.code) return "code_snippet"

    return "lesson_learned"
  }

  /**
   * Extract tags from context
   */
  private extractTags(context: ExtractionContext): string[] {
    const tags = new Set<string>()

    // Add technology
    if (context.technology) {
      tags.add(context.technology.toLowerCase())
    }

    // Extract keywords from problem
    const keywords = this.extractKeywords(context.problem)
    keywords.forEach((k) => tags.add(k))

    // Extract from error message
    if (context.errorMessage) {
      // Extract error type
      const errorType = context.errorMessage.match(/(\w+Error|\w+Exception)/)?.[1]
      if (errorType) tags.add(errorType.toLowerCase())
    }

    // Extract from solution
    const solutionKeywords = this.extractKeywords(context.solution)
    solutionKeywords.slice(0, 5).forEach((k) => tags.add(k))

    return Array.from(tags).slice(0, 10)
  }

  /**
   * Extract keywords from text
   */
  private extractKeywords(text: string): string[] {
    const stopWords = new Set([
      "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
      "have", "has", "had", "do", "does", "did", "will", "would", "could", "should",
      "may", "might", "must", "shall", "can", "to", "of", "in", "for", "on", "with",
      "at", "by", "from", "as", "into", "through", "during", "before", "after",
      "above", "below", "between", "under", "again", "further", "then", "once",
      "this", "that", "these", "those", "it", "its", "itself",
    ])

    return text
      .toLowerCase()
      .replace(/[^\w\s]/g, " ")
      .split(/\s+/)
      .filter((w) => w.length > 2 && !stopWords.has(w))
      .filter((w, i, arr) => arr.indexOf(w) === i)
      .slice(0, 15)
  }

  /**
   * Generate a title for the knowledge entry
   */
  private generateTitle(context: ExtractionContext, category: KnowledgeCategory): string {
    if (context.errorMessage) {
      const errorType = context.errorMessage.match(/(\w+Error|\w+Exception)/)?.[1] ?? "Error"
      return `Solution: ${errorType} in ${context.technology ?? "code"}`
    }

    const categoryTitles: Record<KnowledgeCategory, string> = {
      error_solution: "Error Solution",
      api_pattern: "API Pattern",
      code_snippet: "Code Snippet",
      architecture: "Architecture Pattern",
      configuration: "Configuration",
      debugging: "Debugging Technique",
      performance: "Performance Optimization",
      security: "Security Pattern",
      lesson_learned: "Lesson Learned",
    }

    const prefix = categoryTitles[category]
    const summary = context.problem.slice(0, 50).replace(/\s+/g, " ")

    return `${prefix}: ${summary}${context.problem.length > 50 ? "..." : ""}`
  }

  /**
   * Format content for storage
   */
  private formatContent(context: ExtractionContext): string {
    const sections: string[] = []

    sections.push(`## Problem\n${context.problem}`)

    if (context.errorMessage) {
      sections.push(`## Error\n\`\`\`\n${context.errorMessage}\n\`\`\``)
    }

    sections.push(`## Solution\n${context.solution}`)

    if (context.steps && context.steps.length > 0) {
      const stepsList = context.steps.map((s, i) => `${i + 1}. ${s}`).join("\n")
      sections.push(`## Steps\n${stepsList}`)
    }

    if (context.reflection) {
      sections.push(`## Reflection\n${context.reflection}`)
    }

    if (context.webSources && context.webSources.length > 0) {
      const sources = context.webSources.map((s) => `- [${s.title}](${s.url})`).join("\n")
      sections.push(`## Sources\n${sources}`)
    }

    return sections.join("\n\n")
  }

  /**
   * Extract code examples from context
   */
  private extractCodeExamples(context: ExtractionContext): KnowledgeEntry["codeExamples"] {
    if (!context.code) return undefined

    // Try to detect language
    const language = context.technology ?? this.detectLanguage(context.code)

    return [
      {
        language,
        code: context.code,
        description: "Working solution",
      },
    ]
  }

  /**
   * Simple language detection from code
   */
  private detectLanguage(code: string): string {
    if (code.includes("def ") && code.includes(":")) return "python"
    if (code.includes("function") || code.includes("const ") || code.includes("=>")) return "javascript"
    if (code.includes("fn ") && code.includes("->")) return "rust"
    if (code.includes("func ") && code.includes("{")) return "go"
    if (code.startsWith("#!/bin/") || code.includes("$")) return "shell"
    return "text"
  }

  /**
   * Find similar knowledge entries
   */
  async findSimilar(title: string, tags: string[]): Promise<KnowledgeSearchResult[]> {
    await this.initialize()

    const results: KnowledgeSearchResult[] = []
    const titleWords = new Set(title.toLowerCase().split(/\s+/))
    const tagSet = new Set(tags.map((t) => t.toLowerCase()))

    for (const entry of this.entries.values()) {
      // Calculate tag overlap
      const entryTags = new Set(entry.tags.map((t) => t.toLowerCase()))
      const matchedTags = tags.filter((t) => entryTags.has(t.toLowerCase()))
      const tagScore = matchedTags.length / Math.max(tags.length, entry.tags.length)

      // Calculate title similarity (Jaccard)
      const entryTitleWords = new Set(entry.title.toLowerCase().split(/\s+/))
      const intersection = [...titleWords].filter((w) => entryTitleWords.has(w)).length
      const union = new Set([...titleWords, ...entryTitleWords]).size
      const titleScore = union > 0 ? intersection / union : 0

      // Combined score
      const relevanceScore = tagScore * 0.6 + titleScore * 0.4

      if (relevanceScore > 0.3) {
        results.push({
          entry,
          relevanceScore,
          matchedTags,
        })
      }
    }

    return results.sort((a, b) => b.relevanceScore - a.relevanceScore)
  }

  /**
   * Search knowledge base
   */
  async search(query: string, limit = 10): Promise<KnowledgeSearchResult[]> {
    await this.initialize()

    const queryWords = new Set(this.extractKeywords(query))
    const results: KnowledgeSearchResult[] = []

    for (const entry of this.entries.values()) {
      // Score based on tag matches
      const matchedTags = entry.tags.filter((t) => queryWords.has(t.toLowerCase()))
      const tagScore = matchedTags.length / Math.max(queryWords.size, 1)

      // Score based on content matches
      const contentWords = new Set(this.extractKeywords(entry.content))
      const contentMatches = [...queryWords].filter((w) => contentWords.has(w)).length
      const contentScore = contentMatches / Math.max(queryWords.size, 1)

      // Combined score with success weighting
      const baseScore = tagScore * 0.5 + contentScore * 0.5
      const successBoost = Math.min(entry.successCount / 10, 0.2)
      const relevanceScore = Math.min(baseScore + successBoost, 1.0)

      if (relevanceScore > 0.2) {
        results.push({
          entry,
          relevanceScore,
          matchedTags,
        })
      }
    }

    return results.sort((a, b) => b.relevanceScore - a.relevanceScore).slice(0, limit)
  }

  /**
   * Get knowledge entry by ID
   */
  async get(id: string): Promise<KnowledgeEntry | undefined> {
    await this.initialize()
    return this.entries.get(id)
  }

  /**
   * List all entries by category
   */
  async listByCategory(category: KnowledgeCategory): Promise<KnowledgeEntry[]> {
    await this.initialize()
    return Array.from(this.entries.values())
      .filter((e) => e.category === category)
      .sort((a, b) => b.successCount - a.successCount)
  }

  /**
   * Update success count when knowledge is applied
   */
  async recordSuccess(id: string): Promise<void> {
    await this.initialize()
    const entry = this.entries.get(id)
    if (entry) {
      entry.successCount++
      entry.confidence = Math.min(entry.confidence + 0.05, 1.0)
      entry.updatedAt = new Date().toISOString()
      await this.save()
    }
  }

  /**
   * Save knowledge to disk
   */
  private async save(): Promise<void> {
    const entries = Array.from(this.entries.values())
    await fs.writeFile(KNOWLEDGE_FILE, JSON.stringify(entries, null, 2))
  }

  /**
   * Get statistics about the knowledge base
   */
  async getStats(): Promise<{
    totalEntries: number
    byCategory: Record<KnowledgeCategory, number>
    avgConfidence: number
    topTechnologies: Array<{ technology: string; count: number }>
  }> {
    await this.initialize()

    const entries = Array.from(this.entries.values())
    const byCategory: Record<string, number> = {}
    const technologies: Record<string, number> = {}
    let totalConfidence = 0

    for (const entry of entries) {
      byCategory[entry.category] = (byCategory[entry.category] ?? 0) + 1
      totalConfidence += entry.confidence
      if (entry.technology) {
        technologies[entry.technology] = (technologies[entry.technology] ?? 0) + 1
      }
    }

    const topTechnologies = Object.entries(technologies)
      .map(([technology, count]) => ({ technology, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 5)

    return {
      totalEntries: entries.length,
      byCategory: byCategory as Record<KnowledgeCategory, number>,
      avgConfidence: entries.length > 0 ? totalConfidence / entries.length : 0,
      topTechnologies,
    }
  }
}

// ============================================================================
// Factory
// ============================================================================

let instance: KnowledgeSedimentation | undefined

/**
 * Get the knowledge sedimentation system singleton
 */
export async function getKnowledgeSedimentation(): Promise<KnowledgeSedimentation> {
  if (!instance) {
    instance = new KnowledgeSedimentation()
    await instance.initialize()
  }
  return instance
}

/**
 * Create a new knowledge sedimentation instance
 */
export function createKnowledgeSedimentation(): KnowledgeSedimentation {
  return new KnowledgeSedimentation()
}
