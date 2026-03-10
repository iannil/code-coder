/**
 * World Model
 *
 * @deprecated This TypeScript implementation has been migrated to Rust.
 * Use `ObserverApiClient` from `@/observer/client` to call the Rust API.
 * See `services/zero-cli/src/observer/consensus/world_model.rs` for the new implementation.
 *
 * Creates and maintains the World Model - a convergent snapshot
 * of reality formed from multiple observation sources.
 *
 * Embodies "观察即收敛" (observation as convergence) - the act of
 * observing causes possibilities to collapse into a definite state.
 *
 * @module observer/consensus/world-model
 */

import { Log } from "@/util/log"
import type {
  Observation,
  WorldModel,
  CodeObservation,
  WorldObservation,
  SelfObservation,
  MetaObservation,
} from "../types"
import { generateWorldModelId } from "../types"

const log = Log.create({ service: "observer.consensus.world-model" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface WorldModelConfig {
  /** Time window for snapshot (ms) */
  windowMs: number
  /** Minimum observations for valid model */
  minObservations: number
  /** Confidence threshold for inclusion */
  minConfidence: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: WorldModelConfig = {
  windowMs: 60000, // 1 minute
  minObservations: 5,
  minConfidence: 0.3,
}

// ─────────────────────────────────────────────────────────────────────────────
// World Model Builder
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Builds and maintains the World Model.
 */
export class WorldModelBuilder {
  private config: WorldModelConfig
  private currentModel: WorldModel | null = null
  private modelHistory: WorldModel[] = []
  private maxHistory = 100

  constructor(config: Partial<WorldModelConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Build a world model from observations.
   */
  build(observations: Observation[]): WorldModel | null {
    const now = new Date()
    const windowStart = now.getTime() - this.config.windowMs

    // Filter to window and confidence
    const relevant = observations.filter(
      (o) =>
        o.timestamp.getTime() > windowStart &&
        o.confidence >= this.config.minConfidence,
    )

    if (relevant.length < this.config.minObservations) {
      log.debug("Insufficient observations for world model", {
        count: relevant.length,
        required: this.config.minObservations,
      })
      return this.currentModel
    }

    // Separate by watcher type
    const codeObs = relevant.filter((o) => o.watcherType === "code") as CodeObservation[]
    const worldObs = relevant.filter((o) => o.watcherType === "world") as WorldObservation[]
    const selfObs = relevant.filter((o) => o.watcherType === "self") as SelfObservation[]
    const metaObs = relevant.filter((o) => o.watcherType === "meta") as MetaObservation[]

    // Build model sections
    const code = this.aggregateCode(codeObs)
    const world = this.aggregateWorld(worldObs)
    const self = this.aggregateSelf(selfObs)
    const meta = this.aggregateMeta(metaObs)

    // Calculate overall confidence
    const confidence = this.calculateConfidence(relevant, {
      code: codeObs.length,
      world: worldObs.length,
      self: selfObs.length,
      meta: metaObs.length,
    })

    const model: WorldModel = {
      id: generateWorldModelId(),
      timestamp: now,
      observationIds: relevant.map((o) => o.id),
      code,
      world,
      self,
      meta,
      confidence,
    }

    // Update history
    this.currentModel = model
    this.modelHistory.push(model)
    if (this.modelHistory.length > this.maxHistory) {
      this.modelHistory.shift()
    }

    log.debug("World model built", {
      id: model.id,
      observations: relevant.length,
      confidence: model.confidence.toFixed(2),
    })

    return model
  }

  /**
   * Get current world model.
   */
  getCurrent(): WorldModel | null {
    return this.currentModel
  }

  /**
   * Get model history.
   */
  getHistory(limit?: number): WorldModel[] {
    return this.modelHistory.slice(-(limit ?? 20))
  }

  /**
   * Clear all state.
   */
  clear(): void {
    this.currentModel = null
    this.modelHistory = []
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Aggregation Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private aggregateCode(observations: CodeObservation[]): WorldModel["code"] {
    // Find most recent git change
    const gitChanges = observations.filter((o) => o.type === "git_change")
    const lastCommit = gitChanges.length > 0
      ? (gitChanges[gitChanges.length - 1].change.after as string) ?? undefined
      : undefined

    // Determine build status
    const buildObs = observations.filter((o) => o.type === "build_status")
    let buildStatus: WorldModel["code"]["buildStatus"] = "unknown"
    if (buildObs.length > 0) {
      const lastBuild = buildObs[buildObs.length - 1]
      const status = (lastBuild.change.after as any)?.status
      buildStatus = status === "passing" ? "passing" : status === "failing" ? "failing" : "unknown"
    }

    // Get test coverage
    const testObs = observations.filter((o) => o.type === "test_coverage")
    let testCoverage: number | undefined
    if (testObs.length > 0) {
      const lastTest = testObs[testObs.length - 1]
      testCoverage = (lastTest.change.after as any)?.coverage
    }

    // Assess tech debt
    const debtObs = observations.filter((o) => o.type === "tech_debt")
    let techDebtLevel: WorldModel["code"]["techDebtLevel"]
    if (debtObs.length > 0) {
      const avgSeverity = debtObs.reduce((sum, o) => {
        const sev = o.impact.severity
        return sum + (sev === "high" ? 3 : sev === "medium" ? 2 : 1)
      }, 0) / debtObs.length
      techDebtLevel = avgSeverity > 2.5 ? "high" : avgSeverity > 1.5 ? "medium" : "low"
    }

    return {
      lastCommit,
      buildStatus,
      testCoverage,
      techDebtLevel,
      recentChanges: observations.length,
    }
  }

  private aggregateWorld(observations: WorldObservation[]): WorldModel["world"] {
    // Determine market sentiment
    const marketObs = observations.filter((o) => o.type === "market_data")
    let marketSentiment: WorldModel["world"]["marketSentiment"]
    if (marketObs.length > 0) {
      const sentiments = marketObs.map((o) => o.sentiment).filter(Boolean)
      const positive = sentiments.filter((s) => s === "positive").length
      const negative = sentiments.filter((s) => s === "negative").length
      if (positive > negative * 1.5) marketSentiment = "bullish"
      else if (negative > positive * 1.5) marketSentiment = "bearish"
      else marketSentiment = "neutral"
    }

    // Collect relevant news
    const newsObs = observations.filter((o) => o.type === "news")
    const relevantNews = newsObs
      .filter((o) => o.relevance > 0.5)
      .sort((a, b) => b.relevance - a.relevance)
      .slice(0, 5)
      .map((o) => o.data.title ?? o.data.summary ?? "")
      .filter(Boolean)

    // Identify external risks
    const externalRisks: string[] = []
    const securityObs = observations.filter((o) => o.type === "security_advisory")
    if (securityObs.length > 0) {
      externalRisks.push(`${securityObs.length} security advisory(ies)`)
    }
    const apiChanges = observations.filter((o) => o.type === "api_change")
    const breakingChanges = apiChanges.filter((o) => o.sentiment === "negative")
    if (breakingChanges.length > 0) {
      externalRisks.push(`${breakingChanges.length} breaking API change(s)`)
    }

    // Identify opportunities
    const opportunities: string[] = []
    const depReleases = observations.filter((o) => o.type === "dependency_release")
    const positiveReleases = depReleases.filter((o) => o.sentiment === "positive")
    if (positiveReleases.length > 0) {
      opportunities.push(`${positiveReleases.length} dependency update(s) available`)
    }
    const trends = observations.filter((o) => o.type === "trend")
    const positiveTrends = trends.filter((o) => o.sentiment === "positive")
    if (positiveTrends.length > 0) {
      opportunities.push(...positiveTrends.map((o) => o.data.title ?? "Positive trend detected"))
    }

    return {
      marketSentiment,
      relevantNews,
      externalRisks,
      opportunities,
    }
  }

  private aggregateSelf(observations: SelfObservation[]): WorldModel["self"] {
    // Find current agent
    const agentObs = observations.filter((o) => o.type === "agent_behavior")
    const currentAgent = agentObs.length > 0
      ? agentObs[agentObs.length - 1].agentId
      : undefined

    // Aggregate resource usage
    const resourceObs = observations.filter((o) => o.type === "resource_usage")
    let resourceUsage = { tokens: 0, cost: 0, duration: 0 }
    for (const obs of resourceObs) {
      const input = obs.observation?.input as any
      if (input) {
        resourceUsage.tokens += input.tokens ?? 0
        resourceUsage.cost += input.cost ?? 0
        resourceUsage.duration += input.duration ?? 0
      }
    }

    // Count recent errors
    const errorObs = observations.filter((o) => o.observation && !o.observation.success)
    const recentErrors = errorObs.length

    // Calculate decision quality
    const decisionObs = observations.filter((o) => o.type === "decision_log")
    let decisionQuality: number | undefined
    if (decisionObs.length > 0) {
      const scores = decisionObs
        .map((o) => o.quality?.closeScore)
        .filter((s) => typeof s === "number") as number[]
      if (scores.length > 0) {
        decisionQuality = scores.reduce((a, b) => a + b, 0) / scores.length
      }
    }

    // Determine session health
    let sessionHealth: WorldModel["self"]["sessionHealth"] = "healthy"
    if (recentErrors > 5) sessionHealth = "critical"
    else if (recentErrors > 2) sessionHealth = "degraded"

    return {
      currentAgent,
      sessionHealth,
      resourceUsage,
      recentErrors,
      decisionQuality,
    }
  }

  private aggregateMeta(observations: MetaObservation[]): WorldModel["meta"] {
    // Determine observer health
    const healthObs = observations.filter((o) => o.type === "system_health")
    let observerHealth: WorldModel["meta"]["observerHealth"] = "healthy"
    if (healthObs.length > 0) {
      const lastHealth = healthObs[healthObs.length - 1].assessment.health
      observerHealth = lastHealth
    }

    // Collect coverage gaps
    const coverageGaps: string[] = []
    const gapObs = observations.filter((o) => o.type === "coverage_gap")
    for (const obs of gapObs) {
      for (const issue of obs.issues) {
        if (issue.type === "coverage_gap") {
          coverageGaps.push(issue.description)
        }
      }
    }

    // Calculate consensus strength (average of observation confidences)
    const avgConfidence = observations.length > 0
      ? observations.reduce((sum, o) => sum + o.confidence, 0) / observations.length
      : 0.5

    return {
      observerHealth,
      coverageGaps,
      consensusStrength: avgConfidence,
    }
  }

  private calculateConfidence(
    observations: Observation[],
    counts: { code: number; world: number; self: number; meta: number },
  ): number {
    // Base confidence from observation count
    const countFactor = Math.min(observations.length / 20, 1)

    // Coverage factor (how many watcher types have observations)
    const coverage = Object.values(counts).filter((c) => c > 0).length / 4

    // Average observation confidence
    const avgConfidence = observations.length > 0
      ? observations.reduce((sum, o) => sum + o.confidence, 0) / observations.length
      : 0

    // Combined confidence
    return (countFactor * 0.3 + coverage * 0.3 + avgConfidence * 0.4)
  }
}

/**
 * Create a world model builder.
 */
export function createWorldModelBuilder(
  config?: Partial<WorldModelConfig>,
): WorldModelBuilder {
  return new WorldModelBuilder(config)
}
