/**
 * Self Watcher (SelfWatch)
 *
 * Observes the system's own behavior including:
 * - Agent decisions and actions
 * - Resource usage (tokens, cost, time)
 * - Error patterns
 * - Quality metrics
 * - Tool invocations
 *
 * This implements the "观察自己" (self-observation) aspect of
 * the Observer Network, enabling meta-cognitive awareness.
 *
 * @deprecated This TypeScript implementation is deprecated in favor of the Rust
 * implementation in services/zero-cli/src/observer/watchers/self_watch.rs. The Rust
 * implementation provides better integration with the daemon lifecycle.
 * Migration was completed in Phase 6-7 of the architecture refactoring.
 *
 * @module observer/watchers/self-watch
 */

import { Log } from "@/util/log"
import { BaseWatcher, type WatcherOptions } from "./base-watcher"
import type { SelfObservation, SelfObservationType } from "../types"
import { Bus } from "@/bus"
import { AutonomousEvent } from "@/autonomous/events"

const log = Log.create({ service: "observer.self-watch" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface SelfWatchOptions extends WatcherOptions {
  /** Session ID to monitor (if not provided, monitors all) */
  sessionId?: string
  /** Track resource usage */
  trackResources?: boolean
  /** Track decision history */
  trackDecisions?: boolean
  /** Track tool invocations */
  trackTools?: boolean
  /** Error pattern window in ms */
  errorWindowMs?: number
  /** Cost spike multiplier threshold (default: 2.0) */
  costSpikeThreshold?: number
  /** Cost history window size for spike detection */
  costHistorySize?: number
}

interface AgentAction {
  agentId: string
  action: string
  input?: unknown
  output?: unknown
  duration: number
  success: boolean
  error?: string
  timestamp: Date
}

interface ResourceSnapshot {
  tokens: number
  cost: number
  duration: number
  timestamp: Date
}

interface SessionCostRecord {
  sessionId: string
  tokensUsed: number
  costUSD: number
  duration: number
  timestamp: Date
}

// ─────────────────────────────────────────────────────────────────────────────
// SelfWatch Implementation
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Watcher that observes the system's own behavior.
 */
export class SelfWatch extends BaseWatcher<SelfObservation> {
  private sessionId: string | null
  private recentActions: AgentAction[] = []
  private recentErrors: Array<{ error: string; timestamp: Date }> = []
  private resourceSnapshots: ResourceSnapshot[] = []
  private sessionCostHistory: SessionCostRecord[] = []
  private eventSubscriptions: Array<() => void> = []
  private maxHistorySize = 100
  private errorWindowMs: number
  private costSpikeThreshold: number
  private costHistorySize: number

  constructor(options: SelfWatchOptions = {}) {
    super("self", {
      intervalMs: 0, // Event-driven primarily
      ...options,
    })
    this.sessionId = options.sessionId ?? null
    this.errorWindowMs = options.errorWindowMs ?? 300000 // 5 minutes
    this.costSpikeThreshold = options.costSpikeThreshold ?? 2.0
    this.costHistorySize = options.costHistorySize ?? 50
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────────────────

  protected override async onStart(): Promise<void> {
    // Subscribe to autonomous events
    this.eventSubscriptions.push(
      Bus.subscribe(AutonomousEvent.DecisionMade, async (event) => {
        const props = event.properties as any
        if (this.sessionId && props.sessionId !== this.sessionId) return
        await this.observeDecision(props)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(AutonomousEvent.TaskCompleted, async (event) => {
        const props = event.properties as any
        if (this.sessionId && props.sessionId !== this.sessionId) return
        await this.observeTaskCompletion(props)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(AutonomousEvent.AgentInvoked, async (event) => {
        const props = event.properties as any
        if (this.sessionId && props.sessionId !== this.sessionId) return
        await this.observeAgentInvocation(props)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(AutonomousEvent.ResourceWarning, async (event) => {
        const props = event.properties as any
        if (this.sessionId && props.sessionId !== this.sessionId) return
        await this.observeResourceWarning(props)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(AutonomousEvent.SafetyTriggered, async (event) => {
        const props = event.properties as any
        if (this.sessionId && props.sessionId !== this.sessionId) return
        await this.observeSafetyTrigger(props)
      }),
    )

    // Subscribe to SessionCompleted for cost tracking
    this.eventSubscriptions.push(
      Bus.subscribe(AutonomousEvent.SessionCompleted, async (event) => {
        const props = event.properties as any
        if (this.sessionId && props.sessionId !== this.sessionId) return
        await this.observeSessionCost(props)
      }),
    )

    // Subscribe to MetricsUpdated for real-time resource tracking
    this.eventSubscriptions.push(
      Bus.subscribe(AutonomousEvent.MetricsUpdated, async (event) => {
        const props = event.properties as any
        if (this.sessionId && props.sessionId !== this.sessionId) return
        await this.observeMetricsUpdate(props)
      }),
    )

    log.info("SelfWatch initialized", {
      sessionId: this.sessionId,
      subscriptionCount: this.eventSubscriptions.length,
    })
  }

  protected override async onStop(): Promise<void> {
    // Unsubscribe from all events
    for (const unsubscribe of this.eventSubscriptions) {
      unsubscribe()
    }
    this.eventSubscriptions = []
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Observation
  // ─────────────────────────────────────────────────────────────────────────────

  protected async observe(): Promise<SelfObservation | null> {
    // Periodic self-check for error patterns
    const errorPattern = this.detectErrorPattern()
    if (errorPattern) {
      return errorPattern
    }

    return null
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Public Methods
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Observe an agent action (manually triggered).
   */
  async observeAgentAction(action: {
    agentId: string
    action: string
    input?: unknown
    output?: unknown
    duration: number
    success: boolean
    error?: string
  }): Promise<void> {
    if (!this.isRunning()) return

    const agentAction: AgentAction = {
      ...action,
      timestamp: new Date(),
    }

    this.recentActions.push(agentAction)
    if (this.recentActions.length > this.maxHistorySize) {
      this.recentActions.shift()
    }

    if (!action.success && action.error) {
      this.recentErrors.push({
        error: action.error,
        timestamp: new Date(),
      })
      if (this.recentErrors.length > this.maxHistorySize) {
        this.recentErrors.shift()
      }
    }

    const observation = this.createObservation(
      "agent_behavior",
      action.agentId,
      {
        action: action.action,
        input: action.input,
        output: action.output,
        duration: action.duration,
        success: action.success,
        error: action.error,
      },
    )

    observation.confidence = action.success ? 0.9 : 0.7

    await this.emit(observation)
  }

  /**
   * Observe tool invocation.
   */
  async observeToolInvocation(invocation: {
    toolName: string
    agentId: string
    input: unknown
    output?: unknown
    duration: number
    success: boolean
    error?: string
  }): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation(
      "tool_invocation",
      invocation.agentId,
      {
        action: `tool:${invocation.toolName}`,
        input: invocation.input,
        output: invocation.output,
        duration: invocation.duration,
        success: invocation.success,
        error: invocation.error,
      },
    )

    await this.emit(observation)
  }

  /**
   * Observe resource usage.
   */
  async observeResourceUsage(usage: {
    agentId: string
    tokens: number
    cost: number
    duration: number
  }): Promise<void> {
    if (!this.isRunning()) return

    this.resourceSnapshots.push({
      ...usage,
      timestamp: new Date(),
    })
    if (this.resourceSnapshots.length > this.maxHistorySize) {
      this.resourceSnapshots.shift()
    }

    const observation = this.createObservation("resource_usage", usage.agentId, {
      action: "resource_update",
      input: usage,
      output: undefined,
      duration: usage.duration,
      success: true,
    })

    observation.quality = {
      efficiency: this.calculateEfficiency(usage),
    }

    await this.emit(observation)
  }

  /**
   * Observe quality metric.
   */
  async observeQualityMetric(metric: {
    agentId: string
    metricName: string
    value: number
    closeScore?: number
  }): Promise<void> {
    if (!this.isRunning()) return

    const observation = this.createObservation(
      "quality_metric",
      metric.agentId,
      {
        action: `metric:${metric.metricName}`,
        output: metric.value,
        duration: 0,
        success: true,
      },
    )

    observation.quality = {
      closeScore: metric.closeScore,
    }

    await this.emit(observation)
  }

  /**
   * Get recent actions.
   */
  getRecentActions(limit?: number): AgentAction[] {
    return this.recentActions.slice(-(limit ?? 20))
  }

  /**
   * Get recent errors.
   */
  getRecentErrors(limit?: number): Array<{ error: string; timestamp: Date }> {
    return this.recentErrors.slice(-(limit ?? 20))
  }

  /**
   * Get resource usage summary.
   */
  getResourceSummary(): {
    totalTokens: number
    totalCost: number
    totalDuration: number
    avgEfficiency: number
  } {
    const totals = this.resourceSnapshots.reduce(
      (acc, snap) => ({
        tokens: acc.tokens + snap.tokens,
        cost: acc.cost + snap.cost,
        duration: acc.duration + snap.duration,
      }),
      { tokens: 0, cost: 0, duration: 0 },
    )

    return {
      totalTokens: totals.tokens,
      totalCost: totals.cost,
      totalDuration: totals.duration,
      avgEfficiency:
        totals.duration > 0 ? totals.tokens / totals.duration : 0,
    }
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods - Event Handlers
  // ─────────────────────────────────────────────────────────────────────────────

  private async observeDecision(props: {
    sessionId: string
    decisionId: string
    type: string
    description: string
    score: number
    approved: boolean
    closeScores: {
      convergence: number
      leverage: number
      optionality: number
      surplus: number
      evolution: number
      total: number
    }
  }): Promise<void> {
    const observation = this.createObservation("decision_log", "decision-engine", {
      action: `decision:${props.type}`,
      input: props.description,
      output: { approved: props.approved, score: props.score },
      duration: 0,
      success: props.approved,
    })

    observation.quality = {
      closeScore: props.closeScores.total,
    }

    await this.emit(observation)
  }

  private async observeTaskCompletion(props: {
    sessionId: string
    taskId: string
    success: boolean
    duration: number
    metadata?: Record<string, unknown>
  }): Promise<void> {
    const observation = this.createObservation("agent_behavior", "task", {
      action: `task:${props.taskId}`,
      duration: props.duration,
      success: props.success,
      output: props.metadata,
    })

    await this.emit(observation)
  }

  private async observeAgentInvocation(props: {
    sessionId: string
    agentName: string
    task: string
    success: boolean
    duration: number
    error?: string
  }): Promise<void> {
    await this.observeAgentAction({
      agentId: props.agentName,
      action: props.task,
      duration: props.duration,
      success: props.success,
      error: props.error,
    })
  }

  private async observeResourceWarning(props: {
    sessionId: string
    resource: string
    current: number
    limit: number
    percentage: number
  }): Promise<void> {
    const observation = this.createObservation("resource_usage", "system", {
      action: `warning:${props.resource}`,
      input: { current: props.current, limit: props.limit },
      output: { percentage: props.percentage },
      duration: 0,
      success: false,
    })

    observation.quality = {
      efficiency: 1 - props.percentage / 100,
    }
    observation.confidence = 1.0

    await this.emit(observation)
  }

  private async observeSafetyTrigger(props: {
    sessionId: string
    rule: string
    severity: string
    action: string
    metadata?: Record<string, unknown>
  }): Promise<void> {
    const observation = this.createObservation("error_pattern", "safety", {
      action: `safety:${props.rule}`,
      input: props.metadata,
      output: { action: props.action },
      duration: 0,
      success: false,
      error: `Safety rule triggered: ${props.rule}`,
    })

    observation.tags = ["safety", props.severity, props.rule]

    await this.emit(observation)
  }

  private async observeSessionCost(props: {
    sessionId: string
    requestId: string
    result: {
      success: boolean
      qualityScore: number
      crazinessScore: number
      duration: number
      tokensUsed: number
      costUSD: number
    }
  }): Promise<void> {
    const record: SessionCostRecord = {
      sessionId: props.sessionId,
      tokensUsed: props.result.tokensUsed,
      costUSD: props.result.costUSD,
      duration: props.result.duration,
      timestamp: new Date(),
    }

    // Add to history
    this.sessionCostHistory.push(record)
    if (this.sessionCostHistory.length > this.costHistorySize) {
      this.sessionCostHistory.shift()
    }

    // Calculate efficiency metrics
    const tokensPerSecond = record.duration > 0
      ? record.tokensUsed / (record.duration / 1000)
      : 0
    const costEfficiency = record.costUSD > 0
      ? record.tokensUsed / record.costUSD
      : 0

    // Check for cost spike
    const avgCost = this.calculateAverageCost()
    const isCostSpike = avgCost > 0 && record.costUSD > avgCost * this.costSpikeThreshold

    const observation = this.createObservation("cost", "session", {
      action: "session_completed",
      input: { sessionId: props.sessionId },
      output: {
        tokensUsed: record.tokensUsed,
        costUSD: record.costUSD,
        duration: record.duration,
        tokensPerSecond,
        costEfficiency,
        isCostSpike,
      },
      duration: record.duration,
      success: props.result.success,
    })

    observation.quality = {
      efficiency: this.normalizeEfficiency(costEfficiency),
    }

    // Add cost spike tag if detected
    if (isCostSpike) {
      observation.tags = ["cost_spike"]
      observation.confidence = 0.95
      log.warn("Cost spike detected", {
        sessionId: props.sessionId,
        cost: record.costUSD,
        avgCost,
        threshold: this.costSpikeThreshold,
      })
    }

    await this.emit(observation)
  }

  private async observeMetricsUpdate(props: {
    sessionId: string
    metrics: {
      qualityScore: number
      crazinessScore: number
      autonomyLevel: string
      tasksCompleted: number
      tasksTotal: number
    }
  }): Promise<void> {
    const taskProgress = props.metrics.tasksTotal > 0
      ? props.metrics.tasksCompleted / props.metrics.tasksTotal
      : 0

    const observation = this.createObservation("resource_usage", "session", {
      action: "metrics_update",
      input: { sessionId: props.sessionId },
      output: props.metrics,
      duration: 0,
      success: true,
    })

    observation.quality = {
      closeScore: props.metrics.qualityScore,
      efficiency: taskProgress,
    }

    await this.emit(observation)
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods - Analysis
  // ─────────────────────────────────────────────────────────────────────────────

  private createObservation(
    type: SelfObservationType,
    agentId: string,
    observation: SelfObservation["observation"],
  ): SelfObservation {
    const base = this.createBaseObservation()

    return {
      ...base,
      watcherType: "self" as const,
      type,
      agentId,
      observation,
      quality: {},
    }
  }

  private detectErrorPattern(): SelfObservation | null {
    const now = Date.now()
    const windowStart = now - this.errorWindowMs

    // Filter recent errors
    const recentErrors = this.recentErrors.filter(
      (e) => e.timestamp.getTime() > windowStart,
    )

    if (recentErrors.length < 3) return null

    // Group by error message prefix
    const errorGroups = new Map<string, number>()
    for (const error of recentErrors) {
      const prefix = error.error.slice(0, 50)
      errorGroups.set(prefix, (errorGroups.get(prefix) ?? 0) + 1)
    }

    // Find patterns (errors occurring 3+ times)
    for (const [prefix, count] of errorGroups.entries()) {
      if (count >= 3) {
        return this.createObservation("error_pattern", "system", {
          action: "pattern_detected",
          input: { errorPrefix: prefix, count },
          output: undefined,
          duration: 0,
          success: false,
          error: `Repeated error pattern: ${prefix} (${count} occurrences)`,
        })
      }
    }

    return null
  }

  private calculateEfficiency(usage: {
    tokens: number
    cost: number
    duration: number
  }): number {
    // Simple efficiency metric: tokens per second per dollar
    if (usage.cost === 0 || usage.duration === 0) return 1

    const tokensPerSecond = usage.tokens / (usage.duration / 1000)
    const costEfficiency = tokensPerSecond / usage.cost

    // Normalize to 0-1 range (assuming 1000 tokens/sec/$ is excellent)
    return Math.min(costEfficiency / 1000, 1)
  }

  private calculateAverageCost(): number {
    if (this.sessionCostHistory.length === 0) return 0

    const totalCost = this.sessionCostHistory.reduce(
      (sum, record) => sum + record.costUSD,
      0,
    )
    return totalCost / this.sessionCostHistory.length
  }

  private normalizeEfficiency(tokensPerDollar: number): number {
    // Normalize tokens/$ to 0-1 range
    // Assuming 100,000 tokens/$ is excellent efficiency
    return Math.min(tokensPerDollar / 100000, 1)
  }

  /**
   * Get session cost history.
   */
  getSessionCostHistory(limit?: number): SessionCostRecord[] {
    return this.sessionCostHistory.slice(-(limit ?? 20))
  }

  /**
   * Get cost statistics.
   */
  getCostStatistics(): {
    totalCost: number
    avgCost: number
    maxCost: number
    minCost: number
    sessionCount: number
  } {
    if (this.sessionCostHistory.length === 0) {
      return { totalCost: 0, avgCost: 0, maxCost: 0, minCost: 0, sessionCount: 0 }
    }

    const costs = this.sessionCostHistory.map((r) => r.costUSD)
    const totalCost = costs.reduce((sum, c) => sum + c, 0)

    return {
      totalCost,
      avgCost: totalCost / costs.length,
      maxCost: Math.max(...costs),
      minCost: Math.min(...costs),
      sessionCount: costs.length,
    }
  }
}

/**
 * Create a SelfWatch instance.
 */
export function createSelfWatch(options?: SelfWatchOptions): SelfWatch {
  return new SelfWatch(options)
}
