/**
 * World Watcher (WorldWatch)
 *
 * Observes external world data including:
 * - Market data (via macro/trader agents)
 * - News and headlines
 * - API changes and updates
 * - Dependency releases
 * - Security advisories
 *
 * @module observer/watchers/world-watch
 */

import { Log } from "@/util/log"
import { BaseWatcher, type WatcherOptions } from "./base-watcher"
import type { WorldObservation, WorldObservationType } from "../types"

const log = Log.create({ service: "observer.world-watch" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface WorldWatchOptions extends WatcherOptions {
  /** Data sources to monitor */
  sources?: Array<{
    type: WorldObservationType
    url?: string
    apiKey?: string
    refreshInterval?: number
  }>
  /** Keywords to filter news */
  newsKeywords?: string[]
  /** Dependencies to track */
  trackedDependencies?: string[]
  /** Enable Agent polling for macro data (default: false) */
  enableAgentPolling?: boolean
  /** Agent polling interval (in observation cycles, default: 5) */
  agentPollingCycles?: number
}

export interface MarketDataPoint {
  symbol: string
  price: number
  change: number
  changePercent: number
  volume?: number
  timestamp: Date
}

export interface NewsItem {
  title: string
  summary?: string
  source: string
  url: string
  publishedAt: Date
  sentiment?: "positive" | "negative" | "neutral"
  relevanceScore?: number
}

// ─────────────────────────────────────────────────────────────────────────────
// WorldWatch Implementation
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Watcher that observes external world data.
 */
export class WorldWatch extends BaseWatcher<WorldObservation> {
  private sources: NonNullable<WorldWatchOptions["sources"]>
  private newsKeywords: string[]
  private trackedDependencies: string[]
  private lastChecks: Map<string, Date> = new Map()
  private observationCycle = 0
  private enableAgentPolling: boolean
  private agentPollingCycles: number

  constructor(options: WorldWatchOptions = {}) {
    super("world", {
      intervalMs: 300000, // Check every 5 minutes by default
      ...options,
    })
    this.sources = options.sources ?? []
    this.newsKeywords = options.newsKeywords ?? []
    this.trackedDependencies = options.trackedDependencies ?? []
    this.enableAgentPolling = options.enableAgentPolling ?? false
    this.agentPollingCycles = options.agentPollingCycles ?? 5
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────────────────

  protected override async onStart(): Promise<void> {
    log.info("WorldWatch initialized", {
      sourceCount: this.sources.length,
      newsKeywords: this.newsKeywords,
      trackedDependencies: this.trackedDependencies.length,
      enableAgentPolling: this.enableAgentPolling,
    })
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Observation
  // ─────────────────────────────────────────────────────────────────────────────

  protected async observe(): Promise<WorldObservation | null> {
    // Increment observation cycle counter
    this.observationCycle++

    // Check for dependency updates
    if (this.trackedDependencies.length > 0) {
      const updates = await this.checkDependencyUpdates()
      if (updates.length > 0) {
        return this.createDependencyObservation(updates)
      }
    }

    // Agent polling for macro data (every N observation cycles)
    if (this.enableAgentPolling && this.observationCycle % this.agentPollingCycles === 0) {
      await this.pollAgentData()
    }

    return null
  }

  /**
   * Poll agent for macro data.
   */
  private async pollAgentData(): Promise<void> {
    const { getAgentClient } = await import("../integration/agent-client")
    const agentClient = getAgentClient()

    try {
      const result = await agentClient.invoke({
        agentId: "macro",
        prompt: "获取最新宏观经济指标摘要 (简洁版，100字以内)",
        timeoutMs: 30000,
      })

      if (result.success && result.output) {
        await this.observeTrend({
          name: "macro_summary",
          description: result.output.slice(0, 500), // Limit length
          direction: "stable",
          strength: 0.5,
        })
      }
    } catch (error) {
      log.debug("Agent polling failed", {
        error: error instanceof Error ? error.message : String(error),
      })
      // Silently fail - Agent polling is best-effort
    }
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Public Methods
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Observe market data (called by macro/trader agents).
   */
  async observeMarketData(data: MarketDataPoint[]): Promise<void> {
    if (!this.isRunning()) return

    for (const point of data) {
      const observation = this.createObservation("market_data", point.symbol, {
        title: `${point.symbol}: ${point.changePercent >= 0 ? "+" : ""}${point.changePercent.toFixed(2)}%`,
        summary: `Price: ${point.price}, Change: ${point.change}`,
        content: point,
      })

      // Determine sentiment from change
      observation.sentiment =
        point.changePercent > 1
          ? "positive"
          : point.changePercent < -1
            ? "negative"
            : "neutral"

      // Higher relevance for larger moves
      observation.relevance = Math.min(Math.abs(point.changePercent) / 10, 1)

      await this.emit(observation)
    }
  }

  /**
   * Observe news item.
   */
  async observeNews(item: NewsItem): Promise<void> {
    if (!this.isRunning()) return

    // Check keyword relevance
    const relevance = this.calculateNewsRelevance(item)
    if (relevance < 0.1) return // Skip irrelevant news

    const observation = this.createObservation("news", item.source, {
      title: item.title,
      summary: item.summary,
      content: item,
      sourceUrl: item.url,
      publishedAt: item.publishedAt,
    })

    observation.relevance = relevance
    observation.sentiment = item.sentiment ?? "neutral"

    await this.emit(observation)
  }

  /**
   * Observe API change (e.g., from dependency update).
   */
  async observeApiChange(change: {
    api: string
    version: string
    breakingChanges?: string[]
    deprecations?: string[]
    newFeatures?: string[]
  }): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation("api_change", change.api, {
      title: `${change.api} updated to ${change.version}`,
      summary: this.summarizeApiChange(change),
      content: change,
    })

    observation.relevance = change.breakingChanges?.length ? 1.0 : 0.5
    observation.sentiment = change.breakingChanges?.length
      ? "negative"
      : "positive"

    await this.emit(observation)
  }

  /**
   * Observe security advisory.
   */
  async observeSecurityAdvisory(advisory: {
    id: string
    severity: "low" | "medium" | "high" | "critical"
    package: string
    title: string
    description?: string
    fixedIn?: string
    cve?: string
  }): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation(
      "security_advisory",
      advisory.package,
      {
        title: advisory.title,
        summary: advisory.description,
        content: advisory,
      },
    )

    // Security advisories are always highly relevant
    observation.relevance = advisory.severity === "critical" ? 1.0 : 0.8
    observation.sentiment = "negative"
    observation.confidence = 1.0
    observation.tags = ["security", advisory.severity, advisory.package]
    if (advisory.cve) {
      observation.tags.push(advisory.cve)
    }

    await this.emit(observation)
  }

  /**
   * Observe dependency release.
   */
  async observeDependencyRelease(release: {
    package: string
    version: string
    previousVersion?: string
    releaseNotes?: string
    isBreaking?: boolean
  }): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation(
      "dependency_release",
      release.package,
      {
        title: `${release.package}@${release.version} released`,
        summary: release.releaseNotes?.slice(0, 200),
        content: release,
      },
    )

    observation.relevance = release.isBreaking ? 0.9 : 0.5
    observation.sentiment = release.isBreaking ? "negative" : "positive"
    observation.tags = [release.package, release.version]

    await this.emit(observation)
  }

  /**
   * Observe a trend or pattern in external data.
   */
  async observeTrend(trend: {
    name: string
    description: string
    direction: "up" | "down" | "stable"
    strength: number // 0-1
    dataPoints?: unknown[]
  }): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation("trend", trend.name, {
      title: trend.name,
      summary: trend.description,
      content: trend,
    })

    observation.relevance = trend.strength
    observation.sentiment =
      trend.direction === "up"
        ? "positive"
        : trend.direction === "down"
          ? "negative"
          : "neutral"

    await this.emit(observation)
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private createObservation(
    type: WorldObservationType,
    source: string,
    data: {
      title?: string
      summary?: string
      content: unknown
      sourceUrl?: string
      publishedAt?: Date
    },
  ): WorldObservation {
    const base = this.createBaseObservation()

    return {
      ...base,
      watcherType: "world" as const,
      type,
      source,
      data: {
        title: data.title,
        summary: data.summary,
        content: data.content,
        sourceUrl: data.sourceUrl,
        publishedAt: data.publishedAt,
      },
      relevance: 0.5,
      sentiment: "neutral",
    }
  }

  private calculateNewsRelevance(item: NewsItem): number {
    if (this.newsKeywords.length === 0) return 0.5

    const titleLower = item.title.toLowerCase()
    const summaryLower = (item.summary ?? "").toLowerCase()
    const content = `${titleLower} ${summaryLower}`

    let matchCount = 0
    for (const keyword of this.newsKeywords) {
      if (content.includes(keyword.toLowerCase())) {
        matchCount++
      }
    }

    // Base relevance + keyword matches
    return Math.min(0.3 + (matchCount / this.newsKeywords.length) * 0.7, 1)
  }

  private summarizeApiChange(change: {
    breakingChanges?: string[]
    deprecations?: string[]
    newFeatures?: string[]
  }): string {
    const parts: string[] = []

    if (change.breakingChanges?.length) {
      parts.push(`${change.breakingChanges.length} breaking changes`)
    }
    if (change.deprecations?.length) {
      parts.push(`${change.deprecations.length} deprecations`)
    }
    if (change.newFeatures?.length) {
      parts.push(`${change.newFeatures.length} new features`)
    }

    return parts.join(", ") || "Minor update"
  }

  private async checkDependencyUpdates(): Promise<
    Array<{ package: string; current: string; latest: string }>
  > {
    // In a real implementation, this would check npm/cargo/etc. for updates
    // For now, return empty array
    return []
  }

  private createDependencyObservation(
    updates: Array<{ package: string; current: string; latest: string }>,
  ): WorldObservation {
    return this.createObservation("dependency_release", "npm", {
      title: `${updates.length} dependency updates available`,
      summary: updates.map((u) => `${u.package}: ${u.current} → ${u.latest}`).join(", "),
      content: updates,
    })
  }
}

/**
 * Create a WorldWatch instance.
 */
export function createWorldWatch(options?: WorldWatchOptions): WorldWatch {
  return new WorldWatch(options)
}
