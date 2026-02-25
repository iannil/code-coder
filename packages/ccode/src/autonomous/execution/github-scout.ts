/**
 * GitHub Scout - Intelligent Open-Source Solution Search
 *
 * Implements the GitHub Scout capability for CodeCoder's autonomous mode:
 * 1. SceneTrigger - Identifies when to search for open-source solutions
 * 2. GithubSearcher - Uses `gh` CLI to search GitHub repositories
 * 3. RepoEvaluator - STAR evaluation framework (Stars, Time, Alignment, Risk)
 * 4. IntegrationExecutor - Handles autonomous/recommend/ask integration modes
 *
 * Philosophy: "Don't reinvent the wheel" - leverage community-tested solutions
 */

import { z } from "zod"
import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import type { DecisionContext, DecisionResult } from "../decision/engine"
import { createDecisionEngine } from "../decision/engine"
import { buildCriteria, type AutonomousDecisionCriteria } from "../decision/criteria"

const log = Log.create({ service: "autonomous.github-scout" })

// ============================================================================
// Types & Schemas
// ============================================================================

export const IntegrationModeSchema = z.enum(["autonomous", "recommend", "ask"])
export type IntegrationMode = z.infer<typeof IntegrationModeSchema>

export const SystemPermissionSchema = z.enum(["global_install", "sudo", "system_config", "system_deps"])
export type SystemPermission = z.infer<typeof SystemPermissionSchema>

export const GithubScoutConfigSchema = z.object({
  /** Integration mode: autonomous(default) | recommend | ask */
  integrationMode: IntegrationModeSchema.default("autonomous"),
  /** Operations that require asking user confirmation */
  askForPermissions: z.array(SystemPermissionSchema).default(["global_install", "sudo", "system_config"]),
  /** Maximum number of dependencies to auto-install */
  maxAutoInstallDeps: z.number().min(1).max(50).default(10),
  /** Whether to allow packages with security warnings */
  allowSecurityWarnings: z.boolean().default(false),
  /** Minimum trigger confidence to activate search (0-1) */
  triggerThreshold: z.number().min(0).max(1).default(0.6),
  /** Minimum CLOSE decision score for searching */
  decisionThreshold: z.number().min(0).max(10).default(6.0),
  /** Maximum repositories to evaluate */
  maxReposToEvaluate: z.number().min(1).max(20).default(5),
  /** Enable caching of search results */
  enableCache: z.boolean().default(true),
  /** Cache TTL in milliseconds (default: 1 hour) */
  cacheTTLMs: z.number().default(3600000),
})
export type GithubScoutConfig = z.infer<typeof GithubScoutConfigSchema>

export const DEFAULT_GITHUB_SCOUT_CONFIG: GithubScoutConfig = {
  integrationMode: "autonomous",
  askForPermissions: ["global_install", "sudo", "system_config"],
  maxAutoInstallDeps: 10,
  allowSecurityWarnings: false,
  triggerThreshold: 0.6,
  decisionThreshold: 6.0,
  maxReposToEvaluate: 5,
  enableCache: true,
  cacheTTLMs: 3600000,
}

/** Scene trigger decision result */
export interface TriggerDecision {
  shouldSearch: boolean
  confidence: number
  reason: string
  suggestedQueries: string[]
  category: "high" | "medium" | "low"
  matchedKeywords: string[]
}

/** GitHub repository info from search */
export interface GithubRepo {
  fullName: string
  description: string
  url: string
  stars: number
  forks: number
  language: string | null
  license: string | null
  topics: string[]
  pushedAt: string
  createdAt: string
  openIssuesCount: number
  archived: boolean
  homepage: string | null
}

/** STAR evaluation result */
export interface STAREvaluation {
  repo: GithubRepo
  /** Stars/Popularity score (0-10) */
  starScore: number
  /** Time/Activity score (0-10) */
  timeScore: number
  /** Alignment score (0-10) */
  alignmentScore: number
  /** Risk score (0-10, higher = lower risk) */
  riskScore: number
  /** Weighted total score */
  totalScore: number
  /** Recommendation based on score */
  recommendation: "adopt" | "trial" | "assess" | "avoid"
  /** Human-readable reasoning */
  reasoning: string
  /** Individual dimension breakdowns */
  breakdown: {
    stars: string
    time: string
    alignment: string
    risk: string
  }
}

/** Integration action result */
export interface IntegrationResult {
  success: boolean
  mode: IntegrationMode
  repo: GithubRepo
  action: "installed" | "recommended" | "user_declined" | "skipped" | "failed"
  installCommand?: string
  installOutput?: string
  error?: string
  requiresSystemPermission?: SystemPermission
}

/** Complete GitHub Scout result */
export interface GithubScoutResult {
  triggered: boolean
  triggerDecision: TriggerDecision
  searchQueries?: string[]
  foundRepos?: GithubRepo[]
  evaluations?: STAREvaluation[]
  topRecommendation?: STAREvaluation
  closeDecision?: DecisionResult
  integration?: IntegrationResult
  durationMs: number
  summary: string
}

// ============================================================================
// Scene Trigger Rules
// ============================================================================

interface TriggerRule {
  category: "high" | "medium" | "low"
  confidence: number
  keywords: string[]
  description: string
}

const TRIGGER_RULES: TriggerRule[] = [
  // High priority triggers (0.85-0.9)
  {
    category: "high",
    confidence: 0.9,
    keywords: ["cli", "command-line", "terminal", "shell tool"],
    description: "CLI tools and command-line applications",
  },
  {
    category: "high",
    confidence: 0.9,
    keywords: ["auth", "oauth", "jwt", "login", "authentication", "authorization", "sso"],
    description: "Authentication and authorization systems",
  },
  {
    category: "high",
    confidence: 0.85,
    keywords: ["chart", "graph", "visualization", "dashboard", "d3", "echarts"],
    description: "Data visualization and charting",
  },
  {
    category: "high",
    confidence: 0.85,
    keywords: ["parser", "converter", "transform", "compiler", "transpile", "ast"],
    description: "Parsers and format converters",
  },
  {
    category: "high",
    confidence: 0.8,
    keywords: ["database", "orm", "query builder", "sql", "prisma", "drizzle", "knex"],
    description: "Database and ORM solutions",
  },
  {
    category: "high",
    confidence: 0.8,
    keywords: ["http client", "api client", "rest client", "fetch wrapper"],
    description: "HTTP and API clients",
  },
  // Medium priority triggers (0.6-0.7)
  {
    category: "medium",
    confidence: 0.7,
    keywords: ["implement", "add", "feature", "functionality", "capability"],
    description: "General feature implementation",
  },
  {
    category: "medium",
    confidence: 0.65,
    keywords: ["utility", "helper", "utils", "tools", "library"],
    description: "Utility libraries and helpers",
  },
  {
    category: "medium",
    confidence: 0.6,
    keywords: ["queue", "cache", "rate limit", "retry", "timeout"],
    description: "Infrastructure utilities",
  },
  // Low priority triggers (0.2-0.4)
  {
    category: "low",
    confidence: 0.4,
    keywords: ["refactor", "optimize", "improve", "enhance"],
    description: "Code improvements",
  },
  {
    category: "low",
    confidence: 0.3,
    keywords: ["specific", "custom", "unique", "proprietary"],
    description: "Project-specific logic",
  },
  {
    category: "low",
    confidence: 0.2,
    keywords: ["fix", "bug", "typo", "error", "issue"],
    description: "Bug fixes and small repairs",
  },
]

// ============================================================================
// SceneTrigger
// ============================================================================

/**
 * Identifies when to trigger GitHub Scout based on task description
 */
export class SceneTrigger {
  /**
   * Analyze task description and decide if GitHub search should be triggered
   */
  analyze(taskDescription: string, technology?: string): TriggerDecision {
    const normalizedTask = taskDescription.toLowerCase()
    const matchedRules: Array<{ rule: TriggerRule; matchedKeywords: string[] }> = []

    // Check each rule for keyword matches
    for (const rule of TRIGGER_RULES) {
      const matchedKeywords = rule.keywords.filter((kw) => normalizedTask.includes(kw.toLowerCase()))
      if (matchedKeywords.length > 0) {
        matchedRules.push({ rule, matchedKeywords })
      }
    }

    // No matches - default to low confidence
    if (matchedRules.length === 0) {
      return {
        shouldSearch: false,
        confidence: 0.1,
        reason: "No matching patterns found for open-source search",
        suggestedQueries: [],
        category: "low",
        matchedKeywords: [],
      }
    }

    // Find the highest confidence match
    const bestMatch = matchedRules.reduce((best, current) =>
      current.rule.confidence > best.rule.confidence ? current : best,
    )

    // Calculate adjusted confidence based on multiple matches
    const confidenceBoost = Math.min(0.1, (matchedRules.length - 1) * 0.02)
    const finalConfidence = Math.min(1.0, bestMatch.rule.confidence + confidenceBoost)

    // Generate suggested search queries
    const suggestedQueries = this.generateQueries(taskDescription, bestMatch.matchedKeywords, technology)

    const allMatchedKeywords = matchedRules.flatMap((m) => m.matchedKeywords)

    return {
      shouldSearch: finalConfidence >= 0.5,
      confidence: finalConfidence,
      reason: `${bestMatch.rule.description} - matched: ${bestMatch.matchedKeywords.join(", ")}`,
      suggestedQueries,
      category: bestMatch.rule.category,
      matchedKeywords: [...new Set(allMatchedKeywords)],
    }
  }

  /**
   * Generate GitHub search queries from task description
   */
  private generateQueries(task: string, matchedKeywords: string[], technology?: string): string[] {
    const queries: string[] = []

    // Extract meaningful terms (nouns and verbs)
    const meaningfulTerms = this.extractMeaningfulTerms(task)

    // Primary query: matched keywords + technology
    const primaryTerms = [...matchedKeywords.slice(0, 2), ...meaningfulTerms.slice(0, 2)]
    if (primaryTerms.length > 0) {
      queries.push(primaryTerms.join(" ") + (technology ? ` ${technology}` : ""))
    }

    // Secondary query: broader search with technology
    if (meaningfulTerms.length > 0 && technology) {
      queries.push(`${meaningfulTerms[0]} ${technology}`)
    }

    // Tertiary query: just matched keywords
    if (matchedKeywords.length >= 2) {
      queries.push(matchedKeywords.slice(0, 3).join(" "))
    }

    return [...new Set(queries)].slice(0, 3)
  }

  /**
   * Extract meaningful terms from task description
   */
  private extractMeaningfulTerms(task: string): string[] {
    // Remove common stop words and extract meaningful terms
    const stopWords = new Set([
      "a",
      "an",
      "the",
      "is",
      "are",
      "was",
      "were",
      "be",
      "been",
      "being",
      "have",
      "has",
      "had",
      "do",
      "does",
      "did",
      "will",
      "would",
      "could",
      "should",
      "may",
      "might",
      "must",
      "shall",
      "can",
      "need",
      "to",
      "of",
      "in",
      "for",
      "on",
      "with",
      "at",
      "by",
      "from",
      "up",
      "about",
      "into",
      "over",
      "after",
      "i",
      "me",
      "my",
      "we",
      "our",
      "you",
      "your",
      "it",
      "its",
      "this",
      "that",
      "these",
      "those",
      "and",
      "or",
      "but",
      "if",
      "then",
      "else",
      "when",
      "where",
      "why",
      "how",
      "all",
      "each",
      "every",
      "both",
      "few",
      "more",
      "most",
      "other",
      "some",
      "such",
      "no",
      "not",
      "only",
      "same",
      "so",
      "than",
      "too",
      "very",
      "just",
      "also",
    ])

    return task
      .toLowerCase()
      .replace(/[^a-z0-9\s-]/g, " ")
      .split(/\s+/)
      .filter((word) => word.length > 2 && !stopWords.has(word))
      .slice(0, 10)
  }
}

// ============================================================================
// GithubSearcher
// ============================================================================

/**
 * Searches GitHub repositories using the `gh` CLI
 */
export class GithubSearcher {
  private cache: Map<string, { repos: GithubRepo[]; timestamp: number }> = new Map()
  private cacheTTLMs: number

  constructor(config: { cacheTTLMs?: number } = {}) {
    this.cacheTTLMs = config.cacheTTLMs ?? 3600000
  }

  /**
   * Search GitHub repositories
   */
  async search(query: string, options: { language?: string; minStars?: number; limit?: number }): Promise<GithubRepo[]> {
    const { language, minStars = 100, limit = 10 } = options

    // Check cache
    const cacheKey = `${query}:${language ?? ""}:${minStars}:${limit}`
    const cached = this.cache.get(cacheKey)
    if (cached && Date.now() - cached.timestamp < this.cacheTTLMs) {
      log.debug("Using cached search results", { query })
      return cached.repos
    }

    // Build search query
    const searchParts = [query]
    if (language) searchParts.push(`language:${language}`)
    if (minStars > 0) searchParts.push(`stars:>=${minStars}`)
    searchParts.push("archived:false")

    const searchQuery = searchParts.join(" ")

    try {
      log.info("Searching GitHub", { query: searchQuery, limit })

      const proc = Bun.spawn([
        "gh",
        "search",
        "repos",
        searchQuery,
        "--limit",
        String(limit),
        "--json",
        "fullName,description,url,stargazersCount,forksCount,primaryLanguage,licenseInfo,repositoryTopics,pushedAt,createdAt,openIssuesCount,isArchived,homepageUrl",
      ])

      const output = await new Response(proc.stdout).text()
      const exitCode = await proc.exited

      if (exitCode !== 0) {
        log.warn("GitHub search failed", { exitCode })
        return []
      }

      const rawRepos = JSON.parse(output) as Array<{
        fullName: string
        description: string | null
        url: string
        stargazersCount: number
        forksCount: number
        primaryLanguage: { name: string } | null
        licenseInfo: { key: string } | null
        repositoryTopics: Array<{ name: string }>
        pushedAt: string
        createdAt: string
        openIssuesCount: number
        isArchived: boolean
        homepageUrl: string | null
      }>

      const repos: GithubRepo[] = rawRepos.map((r) => ({
        fullName: r.fullName,
        description: r.description ?? "",
        url: r.url,
        stars: r.stargazersCount,
        forks: r.forksCount,
        language: r.primaryLanguage?.name ?? null,
        license: r.licenseInfo?.key ?? null,
        topics: r.repositoryTopics?.map((t) => t.name) ?? [],
        pushedAt: r.pushedAt,
        createdAt: r.createdAt,
        openIssuesCount: r.openIssuesCount,
        archived: r.isArchived,
        homepage: r.homepageUrl,
      }))

      // Cache results
      this.cache.set(cacheKey, { repos, timestamp: Date.now() })

      log.info("GitHub search completed", { query, resultsCount: repos.length })
      return repos
    } catch (error) {
      log.error("GitHub search error", { query, error })
      return []
    }
  }

  /**
   * Get detailed repository info
   */
  async getRepoDetails(fullName: string): Promise<GithubRepo | null> {
    try {
      const proc = Bun.spawn([
        "gh",
        "repo",
        "view",
        fullName,
        "--json",
        "name,owner,description,url,stargazerCount,forkCount,primaryLanguage,licenseInfo,repositoryTopics,pushedAt,createdAt,openIssuesCount,isArchived,homepageUrl",
      ])

      const output = await new Response(proc.stdout).text()
      const exitCode = await proc.exited

      if (exitCode !== 0) return null

      const r = JSON.parse(output)
      return {
        fullName: `${r.owner?.login ?? ""}/${r.name}`,
        description: r.description ?? "",
        url: r.url,
        stars: r.stargazerCount ?? 0,
        forks: r.forkCount ?? 0,
        language: r.primaryLanguage?.name ?? null,
        license: r.licenseInfo?.key ?? null,
        topics: r.repositoryTopics?.map((t: { name: string }) => t.name) ?? [],
        pushedAt: r.pushedAt,
        createdAt: r.createdAt,
        openIssuesCount: r.openIssuesCount ?? 0,
        archived: r.isArchived ?? false,
        homepage: r.homepageUrl ?? null,
      }
    } catch {
      return null
    }
  }

  clearCache(): void {
    this.cache.clear()
  }
}

// ============================================================================
// RepoEvaluator (STAR Framework)
// ============================================================================

/**
 * Evaluates repositories using the STAR framework:
 * - S: Stars/Popularity
 * - T: Time/Activity
 * - A: Alignment
 * - R: Risk
 */
export class RepoEvaluator {
  private weights = {
    stars: 1.0,
    time: 1.2,
    alignment: 1.5,
    risk: 1.3,
  }

  /**
   * Evaluate a single repository
   */
  evaluate(
    repo: GithubRepo,
    context: {
      taskDescription: string
      technology?: string
      keywords: string[]
    },
  ): STAREvaluation {
    const starScore = this.evaluateStars(repo)
    const timeScore = this.evaluateTime(repo)
    const alignmentScore = this.evaluateAlignment(repo, context)
    const riskScore = this.evaluateRisk(repo)

    // Calculate weighted total
    const maxScore = 10 * (this.weights.stars + this.weights.time + this.weights.alignment + this.weights.risk)
    const weightedSum =
      starScore * this.weights.stars +
      timeScore * this.weights.time +
      alignmentScore * this.weights.alignment +
      riskScore * this.weights.risk
    const totalScore = Math.round(((weightedSum / maxScore) * 10) * 100) / 100

    // Determine recommendation
    const recommendation = this.getRecommendation(totalScore, riskScore)

    // Build reasoning
    const breakdown = {
      stars: this.getStarsReasoning(repo, starScore),
      time: this.getTimeReasoning(repo, timeScore),
      alignment: this.getAlignmentReasoning(repo, alignmentScore, context),
      risk: this.getRiskReasoning(repo, riskScore),
    }

    const reasoning = this.buildReasoning(repo, totalScore, recommendation, breakdown)

    return {
      repo,
      starScore,
      timeScore,
      alignmentScore,
      riskScore,
      totalScore,
      recommendation,
      reasoning,
      breakdown,
    }
  }

  /**
   * Evaluate multiple repos and rank them
   */
  evaluateAll(
    repos: GithubRepo[],
    context: {
      taskDescription: string
      technology?: string
      keywords: string[]
    },
  ): STAREvaluation[] {
    return repos.map((repo) => this.evaluate(repo, context)).sort((a, b) => b.totalScore - a.totalScore)
  }

  /**
   * S - Stars/Popularity score (0-10)
   */
  private evaluateStars(repo: GithubRepo): number {
    const { stars, forks } = repo

    // Log scale for stars (1k = 5, 10k = 7, 100k = 9)
    let score = 0
    if (stars >= 100000) score = 10
    else if (stars >= 50000) score = 9
    else if (stars >= 10000) score = 8
    else if (stars >= 5000) score = 7
    else if (stars >= 1000) score = 6
    else if (stars >= 500) score = 5
    else if (stars >= 100) score = 4
    else if (stars >= 50) score = 3
    else score = 2

    // Bonus for forks (indicates active use)
    const forkBonus = Math.min(1, forks / 1000)
    return Math.min(10, score + forkBonus)
  }

  /**
   * T - Time/Activity score (0-10)
   */
  private evaluateTime(repo: GithubRepo): number {
    const lastPush = new Date(repo.pushedAt)
    const now = new Date()
    const daysSinceUpdate = (now.getTime() - lastPush.getTime()) / (1000 * 60 * 60 * 24)

    // Recent activity scoring
    let score = 0
    if (daysSinceUpdate <= 7) score = 10
    else if (daysSinceUpdate <= 30) score = 9
    else if (daysSinceUpdate <= 90) score = 8
    else if (daysSinceUpdate <= 180) score = 7
    else if (daysSinceUpdate <= 365) score = 5
    else if (daysSinceUpdate <= 730) score = 3
    else score = 1

    return score
  }

  /**
   * A - Alignment score (0-10)
   */
  private evaluateAlignment(
    repo: GithubRepo,
    context: { taskDescription: string; technology?: string; keywords: string[] },
  ): number {
    let score = 5 // Base score

    const repoText = `${repo.fullName} ${repo.description} ${repo.topics.join(" ")}`.toLowerCase()

    // Keyword matching
    const matchedKeywords = context.keywords.filter((kw) => repoText.includes(kw.toLowerCase()))
    score += Math.min(2, matchedKeywords.length * 0.5)

    // Technology/language matching
    if (context.technology && repo.language) {
      const techMatch =
        repo.language.toLowerCase().includes(context.technology.toLowerCase()) ||
        context.technology.toLowerCase().includes(repo.language.toLowerCase())
      if (techMatch) score += 2
    }

    // Topic relevance
    const relevantTopics = repo.topics.filter(
      (topic) =>
        context.keywords.some((kw) => topic.includes(kw)) ||
        (context.technology && topic.includes(context.technology.toLowerCase())),
    )
    score += Math.min(1, relevantTopics.length * 0.5)

    return Math.min(10, Math.max(0, score))
  }

  /**
   * R - Risk score (0-10, higher = lower risk)
   */
  private evaluateRisk(repo: GithubRepo): number {
    let score = 10 // Start with max (no risk)

    // Archived repos are high risk
    if (repo.archived) return 1

    // No license is risky
    if (!repo.license) score -= 2
    // Restrictive licenses are slightly risky
    else if (["gpl-3.0", "agpl-3.0", "lgpl-3.0"].includes(repo.license)) {
      score -= 1
    }

    // High open issues count is concerning
    if (repo.openIssuesCount > 500) score -= 2
    else if (repo.openIssuesCount > 200) score -= 1

    // Very new repos are riskier
    const created = new Date(repo.createdAt)
    const age = (Date.now() - created.getTime()) / (1000 * 60 * 60 * 24 * 365)
    if (age < 0.5) score -= 2
    else if (age < 1) score -= 1

    // Low star count despite age is concerning
    if (repo.stars < 100 && age > 2) score -= 1

    return Math.max(0, score)
  }

  private getRecommendation(totalScore: number, riskScore: number): "adopt" | "trial" | "assess" | "avoid" {
    if (riskScore < 4) return "avoid"
    if (totalScore >= 7.5) return "adopt"
    if (totalScore >= 6.0) return "trial"
    if (totalScore >= 4.0) return "assess"
    return "avoid"
  }

  private getStarsReasoning(repo: GithubRepo, score: number): string {
    return `${repo.stars.toLocaleString()} stars, ${repo.forks.toLocaleString()} forks (score: ${score.toFixed(1)})`
  }

  private getTimeReasoning(repo: GithubRepo, score: number): string {
    const lastPush = new Date(repo.pushedAt)
    const daysAgo = Math.floor((Date.now() - lastPush.getTime()) / (1000 * 60 * 60 * 24))
    return `Last updated ${daysAgo} days ago (score: ${score.toFixed(1)})`
  }

  private getAlignmentReasoning(
    repo: GithubRepo,
    score: number,
    context: { keywords: string[]; technology?: string },
  ): string {
    const parts: string[] = []
    if (repo.language) parts.push(`Language: ${repo.language}`)
    if (repo.topics.length > 0) parts.push(`Topics: ${repo.topics.slice(0, 3).join(", ")}`)
    parts.push(`(score: ${score.toFixed(1)})`)
    return parts.join("; ")
  }

  private getRiskReasoning(repo: GithubRepo, score: number): string {
    const parts: string[] = []
    parts.push(`License: ${repo.license ?? "none"}`)
    if (repo.archived) parts.push("ARCHIVED")
    if (repo.openIssuesCount > 100) parts.push(`${repo.openIssuesCount} open issues`)
    parts.push(`(score: ${score.toFixed(1)})`)
    return parts.join("; ")
  }

  private buildReasoning(
    repo: GithubRepo,
    totalScore: number,
    recommendation: string,
    breakdown: STAREvaluation["breakdown"],
  ): string {
    return `
**${repo.fullName}** - Score: ${totalScore.toFixed(2)}/10 → **${recommendation.toUpperCase()}**

STAR Breakdown:
- Stars: ${breakdown.stars}
- Time: ${breakdown.time}
- Alignment: ${breakdown.alignment}
- Risk: ${breakdown.risk}

${repo.description ? `Description: ${repo.description}` : ""}
`.trim()
  }
}

// ============================================================================
// IntegrationExecutor
// ============================================================================

/**
 * Handles the integration of recommended packages based on mode:
 * - autonomous: Auto-install without confirmation
 * - recommend: Output report only
 * - ask: Ask user for confirmation
 */
export class IntegrationExecutor {
  private config: GithubScoutConfig

  constructor(config: GithubScoutConfig) {
    this.config = config
  }

  /**
   * Execute integration based on mode
   */
  async execute(
    evaluation: STAREvaluation,
    options: {
      workingDir?: string
      onAskUser?: (question: string) => Promise<boolean>
    } = {},
  ): Promise<IntegrationResult> {
    const { repo, recommendation } = evaluation
    const effectiveMode = this.determineEffectiveMode(repo)

    log.info("Executing integration", { repo: repo.fullName, mode: effectiveMode, recommendation })

    // Skip if recommendation is avoid
    if (recommendation === "avoid") {
      return {
        success: false,
        mode: effectiveMode,
        repo,
        action: "skipped",
        error: "Repository not recommended for adoption",
      }
    }

    switch (effectiveMode) {
      case "recommend":
        return this.executeRecommend(evaluation)

      case "ask":
        return this.executeWithConfirmation(evaluation, options)

      case "autonomous":
      default:
        return this.executeAutonomous(evaluation, options)
    }
  }

  /**
   * Determine effective mode based on repo and permissions
   */
  private determineEffectiveMode(repo: GithubRepo): IntegrationMode {
    // Check if installation requires system permissions
    const requiresSystemPermission = this.checkSystemPermissionRequired(repo)

    if (requiresSystemPermission && this.config.askForPermissions.includes(requiresSystemPermission)) {
      return "ask"
    }

    return this.config.integrationMode
  }

  /**
   * Check if installing this repo requires system permissions
   */
  private checkSystemPermissionRequired(repo: GithubRepo): SystemPermission | null {
    const repoName = repo.fullName.toLowerCase()
    const description = repo.description?.toLowerCase() ?? ""
    const combined = `${repoName} ${description}`

    // Global CLI tools typically need global install
    if (combined.includes("cli") || combined.includes("command-line") || combined.includes("globally")) {
      return "global_install"
    }

    // System-level tools
    if (combined.includes("sudo") || combined.includes("root")) {
      return "sudo"
    }

    // System config tools
    if (combined.includes("system config") || combined.includes("dotfiles")) {
      return "system_config"
    }

    return null
  }

  /**
   * Execute in recommend mode - just return the report
   */
  private executeRecommend(evaluation: STAREvaluation): IntegrationResult {
    return {
      success: true,
      mode: "recommend",
      repo: evaluation.repo,
      action: "recommended",
    }
  }

  /**
   * Execute with user confirmation
   */
  private async executeWithConfirmation(
    evaluation: STAREvaluation,
    options: { onAskUser?: (question: string) => Promise<boolean>; workingDir?: string },
  ): Promise<IntegrationResult> {
    const { repo } = evaluation

    // If no ask callback, fall back to recommend mode
    if (!options.onAskUser) {
      return {
        success: true,
        mode: "ask",
        repo,
        action: "recommended",
        error: "No user confirmation callback available",
      }
    }

    const question = this.buildConfirmationQuestion(evaluation)
    const confirmed = await options.onAskUser(question)

    if (!confirmed) {
      return {
        success: false,
        mode: "ask",
        repo,
        action: "user_declined",
      }
    }

    // User confirmed, proceed with installation
    return this.installPackage(repo, options.workingDir)
  }

  /**
   * Execute autonomous installation
   */
  private async executeAutonomous(
    evaluation: STAREvaluation,
    options: { workingDir?: string },
  ): Promise<IntegrationResult> {
    const { repo, recommendation } = evaluation

    // Only auto-install for adopt or trial recommendations
    if (recommendation !== "adopt" && recommendation !== "trial") {
      return {
        success: true,
        mode: "autonomous",
        repo,
        action: "recommended",
      }
    }

    return this.installPackage(repo, options.workingDir)
  }

  /**
   * Install package using appropriate package manager
   */
  private async installPackage(repo: GithubRepo, workingDir?: string): Promise<IntegrationResult> {
    const installCommand = this.buildInstallCommand(repo)

    if (!installCommand) {
      return {
        success: false,
        mode: this.config.integrationMode,
        repo,
        action: "failed",
        error: "Could not determine install command",
      }
    }

    try {
      log.info("Installing package", { repo: repo.fullName, command: installCommand })

      const proc = Bun.spawn(["sh", "-c", installCommand], {
        cwd: workingDir ?? process.cwd(),
      })

      const output = await new Response(proc.stdout).text()
      const exitCode = await proc.exited

      if (exitCode !== 0) {
        const stderr = await new Response(proc.stderr).text()
        return {
          success: false,
          mode: this.config.integrationMode,
          repo,
          action: "failed",
          installCommand,
          error: stderr || "Installation failed",
        }
      }

      return {
        success: true,
        mode: this.config.integrationMode,
        repo,
        action: "installed",
        installCommand,
        installOutput: output,
      }
    } catch (error) {
      return {
        success: false,
        mode: this.config.integrationMode,
        repo,
        action: "failed",
        installCommand,
        error: String(error),
      }
    }
  }

  /**
   * Build install command based on repo language and package ecosystem
   */
  private buildInstallCommand(repo: GithubRepo): string | null {
    const name = repo.fullName.split("/")[1]

    // Infer package manager from language
    switch (repo.language?.toLowerCase()) {
      case "typescript":
      case "javascript":
        return `bun add ${name}`

      case "python":
        return `pip install ${name}`

      case "rust":
        return `cargo add ${name}`

      case "go":
        return `go get ${repo.fullName}`

      default:
        // Try npm as default for unknown
        return `bun add ${name}`
    }
  }

  /**
   * Build confirmation question for user
   */
  private buildConfirmationQuestion(evaluation: STAREvaluation): string {
    const { repo, totalScore, recommendation, breakdown } = evaluation

    return `
Would you like to install **${repo.fullName}**?

STAR Score: ${totalScore.toFixed(2)}/10 → ${recommendation.toUpperCase()}
- Stars: ${breakdown.stars}
- Time: ${breakdown.time}
- Alignment: ${breakdown.alignment}
- Risk: ${breakdown.risk}

${repo.description ?? ""}
URL: ${repo.url}

Install command: ${this.buildInstallCommand(repo) ?? "unknown"}
`.trim()
  }
}

// ============================================================================
// GithubScout Main Service
// ============================================================================

/**
 * Main GitHub Scout service that orchestrates all components
 */
export class GithubScout {
  private config: GithubScoutConfig
  private sceneTrigger: SceneTrigger
  private searcher: GithubSearcher
  private evaluator: RepoEvaluator
  private integrationExecutor: IntegrationExecutor
  private decisionEngine = createDecisionEngine()

  constructor(config: Partial<GithubScoutConfig> = {}) {
    this.config = { ...DEFAULT_GITHUB_SCOUT_CONFIG, ...config }
    this.sceneTrigger = new SceneTrigger()
    this.searcher = new GithubSearcher({ cacheTTLMs: this.config.cacheTTLMs })
    this.evaluator = new RepoEvaluator()
    this.integrationExecutor = new IntegrationExecutor(this.config)
  }

  /**
   * Run the complete GitHub Scout flow
   */
  async scout(
    problem: {
      sessionId: string
      description: string
      technology?: string
      workingDir?: string
    },
    options: {
      onAskUser?: (question: string) => Promise<boolean>
    } = {},
  ): Promise<GithubScoutResult> {
    const startTime = Date.now()

    // Step 1: Scene Trigger Analysis
    const triggerDecision = this.sceneTrigger.analyze(problem.description, problem.technology)

    log.info("Scene trigger analysis", {
      shouldSearch: triggerDecision.shouldSearch,
      confidence: triggerDecision.confidence,
      category: triggerDecision.category,
    })

    // Check if trigger confidence meets threshold
    if (!triggerDecision.shouldSearch || triggerDecision.confidence < this.config.triggerThreshold) {
      return {
        triggered: false,
        triggerDecision,
        durationMs: Date.now() - startTime,
        summary: `GitHub Scout not triggered: ${triggerDecision.reason}`,
      }
    }

    // Step 2: CLOSE Decision Evaluation
    const closeDecision = await this.evaluateSearchDecision(problem.sessionId, triggerDecision)

    if (!closeDecision.approved) {
      return {
        triggered: false,
        triggerDecision,
        closeDecision,
        durationMs: Date.now() - startTime,
        summary: `CLOSE decision blocked search: ${closeDecision.reasoning}`,
      }
    }

    // Publish trigger event
    await Bus.publish(AutonomousEvent.GithubScoutTriggered, {
      sessionId: problem.sessionId,
      confidence: triggerDecision.confidence,
      category: triggerDecision.category,
      queries: triggerDecision.suggestedQueries,
    })

    // Step 3: GitHub Search
    const searchQueries = triggerDecision.suggestedQueries
    let foundRepos: GithubRepo[] = []

    for (const query of searchQueries) {
      const repos = await this.searcher.search(query, {
        language: problem.technology,
        minStars: 100,
        limit: this.config.maxReposToEvaluate,
      })
      foundRepos = [...foundRepos, ...repos]
    }

    // Deduplicate by fullName
    const seen = new Set<string>()
    foundRepos = foundRepos.filter((repo) => {
      if (seen.has(repo.fullName)) return false
      seen.add(repo.fullName)
      return true
    })

    if (foundRepos.length === 0) {
      return {
        triggered: true,
        triggerDecision,
        closeDecision,
        searchQueries,
        foundRepos: [],
        durationMs: Date.now() - startTime,
        summary: "No relevant repositories found on GitHub",
      }
    }

    // Step 4: STAR Evaluation
    const evaluations = this.evaluator.evaluateAll(foundRepos.slice(0, this.config.maxReposToEvaluate), {
      taskDescription: problem.description,
      technology: problem.technology,
      keywords: triggerDecision.matchedKeywords,
    })

    const topRecommendation = evaluations[0]

    // Publish evaluation event
    await Bus.publish(AutonomousEvent.RepoEvaluated, {
      sessionId: problem.sessionId,
      reposEvaluated: evaluations.length,
      topRepo: topRecommendation?.repo.fullName,
      topScore: topRecommendation?.totalScore,
      recommendation: topRecommendation?.recommendation,
    })

    // Step 5: Integration (if applicable)
    let integration: IntegrationResult | undefined

    if (topRecommendation && topRecommendation.recommendation !== "avoid") {
      integration = await this.integrationExecutor.execute(topRecommendation, {
        workingDir: problem.workingDir,
        onAskUser: options.onAskUser,
      })

      // Publish integration event
      await Bus.publish(AutonomousEvent.IntegrationExecuted, {
        sessionId: problem.sessionId,
        repo: topRecommendation.repo.fullName,
        mode: integration.mode,
        action: integration.action,
        success: integration.success,
      })
    }

    // Build summary
    const summary = this.buildSummary(evaluations, integration)

    return {
      triggered: true,
      triggerDecision,
      searchQueries,
      foundRepos,
      evaluations,
      topRecommendation,
      closeDecision,
      integration,
      durationMs: Date.now() - startTime,
      summary,
    }
  }

  /**
   * Evaluate whether to search using CLOSE framework
   */
  private async evaluateSearchDecision(sessionId: string, trigger: TriggerDecision): Promise<DecisionResult> {
    // Build decision criteria based on "search open source vs build" template
    const criteria: AutonomousDecisionCriteria = buildCriteria({
      type: "resource_acquisition",
      description: `Search GitHub for open-source solution: ${trigger.reason}`,
      riskLevel: "low",
      // High score = preserve options (searching keeps "build from scratch" option open)
      convergence: 8,
      // High leverage = small effort (search) could save large effort (building)
      leverage: 9,
      // Fully reversible = can always ignore search results
      optionality: 9,
      // Low cost = just a few API calls
      surplus: 9,
      // Medium learning = learn about ecosystem
      evolution: 7,
    })

    const context: DecisionContext = {
      sessionId,
      currentState: "evolution_resource_search",
      errorCount: 0,
      recentDecisions: [],
    }

    return this.decisionEngine.evaluate(criteria, context)
  }

  /**
   * Build human-readable summary
   */
  private buildSummary(evaluations: STAREvaluation[], integration?: IntegrationResult): string {
    if (evaluations.length === 0) {
      return "No suitable open-source solutions found"
    }

    const top = evaluations[0]
    const parts = [`Found ${evaluations.length} repositories. Top recommendation: **${top.repo.fullName}**`]
    parts.push(`STAR Score: ${top.totalScore.toFixed(2)}/10 → ${top.recommendation.toUpperCase()}`)

    if (integration) {
      switch (integration.action) {
        case "installed":
          parts.push(`Installed successfully: \`${integration.installCommand}\``)
          break
        case "recommended":
          parts.push("Recommended for manual review")
          break
        case "user_declined":
          parts.push("User declined installation")
          break
        case "skipped":
          parts.push("Skipped: repository not recommended")
          break
        case "failed":
          parts.push(`Installation failed: ${integration.error}`)
          break
      }
    }

    return parts.join("\n")
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<GithubScoutConfig>): void {
    this.config = { ...this.config, ...config }
    this.integrationExecutor = new IntegrationExecutor(this.config)
  }

  /**
   * Get current configuration
   */
  getConfig(): GithubScoutConfig {
    return { ...this.config }
  }

  /**
   * Clear search cache
   */
  clearCache(): void {
    this.searcher.clearCache()
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a GithubScout instance
 */
export function createGithubScout(config?: Partial<GithubScoutConfig>): GithubScout {
  return new GithubScout(config)
}

/**
 * Run a single GitHub Scout search
 */
export async function scoutGithub(
  problem: {
    sessionId: string
    description: string
    technology?: string
    workingDir?: string
  },
  config?: Partial<GithubScoutConfig>,
): Promise<GithubScoutResult> {
  const scout = createGithubScout(config)
  return scout.scout(problem)
}
