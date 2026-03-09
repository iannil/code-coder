/**
 * Observation Router
 *
 * Routes observations from the Observer Network to relevant agents.
 * Enables automatic agent invocation based on observation types.
 *
 * Design Philosophy:
 * - 观察即收敛: Observations trigger focused agent analysis
 * - 可用余量: Maintain processing capacity headroom
 * - 评估权: Human intervention points for critical observations
 *
 * @module observer/integration/observation-router
 */

import { Log } from "@/util/log"
import type { Observation, Anomaly, Opportunity } from "../types"
import type { WatcherType } from "../watchers"
import { getAgentsForWatcher, type ObserverAgentInfo } from "../agent-registry"
import { getAgentClient, type AgentInvocation, type AgentResult } from "./agent-client"
import { ObserverEvent } from "../events"

const log = Log.create({ service: "observer.router" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface RoutingRule {
  /** Rule identifier */
  id: string
  /** Observation types this rule applies to */
  observationTypes: string[]
  /** Watcher types to match */
  watcherTypes?: WatcherType[]
  /** Minimum confidence threshold */
  minConfidence?: number
  /** Target agent IDs */
  targetAgents: string[]
  /** Whether to run agents in parallel */
  parallel: boolean
  /** Priority (higher = processed first) */
  priority: number
  /** Whether this rule requires human approval */
  requiresApproval: boolean
  /** Custom prompt template (use {{observation}} for data) */
  promptTemplate?: string
}

export interface RoutingResult {
  observationId: string
  agentId: string
  rule: string
  result: AgentResult
  routedAt: Date
  completedAt?: Date
}

export interface RouterConfig {
  /** Enable automatic routing */
  enabled: boolean
  /** Maximum concurrent agent calls */
  maxConcurrent: number
  /** Default routing rules */
  defaultRules: RoutingRule[]
  /** Queue observations when at capacity */
  enableQueue: boolean
  /** Maximum queue size */
  maxQueueSize: number
  /** Minimum confidence for automatic routing */
  minConfidenceThreshold: number
  /** Enable anomaly-triggered routing */
  routeAnomalies: boolean
  /** Enable opportunity-triggered routing */
  routeOpportunities: boolean
  /** Enable pattern-triggered routing */
  routePatterns: boolean
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: RouterConfig = {
  enabled: true,
  maxConcurrent: 2,
  defaultRules: [
    // Code observations → explore agent
    {
      id: "code-to-explore",
      observationTypes: ["file_change", "build_status", "test_result"],
      watcherTypes: ["code"],
      minConfidence: 0.6,
      targetAgents: ["explore"],
      parallel: false,
      priority: 50,
      requiresApproval: false,
    },
    // World observations → macro/trader agents
    {
      id: "world-to-macro",
      observationTypes: ["market_data", "news_signal", "trend"],
      watcherTypes: ["world"],
      minConfidence: 0.7,
      targetAgents: ["macro", "trader"],
      parallel: true,
      priority: 60,
      requiresApproval: false,
    },
    // Self observations → code-reviewer/security-reviewer
    {
      id: "self-to-reviewers",
      observationTypes: ["agent_behavior", "decision_point"],
      watcherTypes: ["self"],
      minConfidence: 0.5,
      targetAgents: ["code-reviewer", "security-reviewer"],
      parallel: true,
      priority: 40,
      requiresApproval: false,
    },
    // Meta observations → observer agent
    {
      id: "meta-to-observer",
      observationTypes: ["system_health", "quality_report", "blind_spot"],
      watcherTypes: ["meta"],
      minConfidence: 0.8,
      targetAgents: ["observer"],
      parallel: false,
      priority: 70,
      requiresApproval: true,
    },
  ],
  enableQueue: true,
  maxQueueSize: 50,
  minConfidenceThreshold: 0.5,
  routeAnomalies: true,
  routeOpportunities: true,
  routePatterns: false, // Patterns are aggregated, not individually routed
}

// ─────────────────────────────────────────────────────────────────────────────
// Observation Router
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Routes observations to relevant agents for analysis.
 */
export class ObservationRouter {
  private config: RouterConfig
  private rules: Map<string, RoutingRule> = new Map()
  private queue: Array<{ observation: Observation; rule: RoutingRule }> = []
  private runningCount = 0
  private results: RoutingResult[] = []
  private running = false
  private eventSubscriptions: Array<() => void> = []
  private agentCache: Map<WatcherType, ObserverAgentInfo[]> = new Map()

  constructor(config: Partial<RouterConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
    this.initializeRules()
  }

  /**
   * Initialize default routing rules.
   */
  private initializeRules(): void {
    for (const rule of this.config.defaultRules) {
      this.rules.set(rule.id, rule)
    }
    log.debug("Routing rules initialized", { count: this.rules.size })
  }

  /**
   * Start the router.
   */
  async start(): Promise<void> {
    if (this.running) return

    this.running = true

    // Cache agent mappings
    await this.refreshAgentCache()

    // Subscribe to observation events
    const Bus = await import("@/bus").then((m) => m.Bus)

    // Subscribe to all observation events
    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.CodeObserved, async (event) => {
        if (!this.config.enabled) return
        const observation = event.properties
        await this.routeObservation(observation)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.WorldObserved, async (event) => {
        if (!this.config.enabled) return
        const observation = event.properties
        await this.routeObservation(observation)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.SelfObserved, async (event) => {
        if (!this.config.enabled) return
        const observation = event.properties
        await this.routeObservation(observation)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.MetaObserved, async (event) => {
        if (!this.config.enabled) return
        const observation = event.properties
        await this.routeObservation(observation)
      }),
    )

    // Subscribe to anomaly events if enabled
    if (this.config.routeAnomalies) {
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.AnomalyDetected, async (event) => {
          const anomaly = event.properties as Anomaly
          await this.routeAnomaly(anomaly)
        }),
      )
    }

    // Subscribe to opportunity events if enabled
    if (this.config.routeOpportunities) {
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.OpportunityIdentified, async (event) => {
          const opportunity = event.properties as Opportunity
          await this.routeOpportunity(opportunity)
        }),
      )
    }

    log.info("Observation router started", {
      rules: this.rules.size,
      maxConcurrent: this.config.maxConcurrent,
    })
  }

  /**
   * Stop the router.
   */
  stop(): void {
    if (!this.running) return

    this.running = false

    for (const unsubscribe of this.eventSubscriptions) {
      unsubscribe()
    }
    this.eventSubscriptions = []

    log.info("Observation router stopped", {
      processed: this.results.length,
      queued: this.queue.length,
    })
  }

  /**
   * Add a routing rule.
   */
  addRule(rule: RoutingRule): void {
    this.rules.set(rule.id, rule)
    log.debug("Routing rule added", { ruleId: rule.id })
  }

  /**
   * Remove a routing rule.
   */
  removeRule(ruleId: string): boolean {
    const removed = this.rules.delete(ruleId)
    if (removed) {
      log.debug("Routing rule removed", { ruleId })
    }
    return removed
  }

  /**
   * Get all routing rules.
   */
  getRules(): RoutingRule[] {
    return Array.from(this.rules.values())
  }

  /**
   * Get routing results history.
   */
  getResults(limit?: number): RoutingResult[] {
    return this.results.slice(-(limit ?? 100))
  }

  /**
   * Get current queue status.
   */
  getQueueStatus(): { size: number; running: number; maxConcurrent: number } {
    return {
      size: this.queue.length,
      running: this.runningCount,
      maxConcurrent: this.config.maxConcurrent,
    }
  }

  /**
   * Refresh agent cache from registry.
   */
  async refreshAgentCache(): Promise<void> {
    const watcherTypes: WatcherType[] = ["code", "world", "self", "meta"]

    for (const watcherType of watcherTypes) {
      const agents = await getAgentsForWatcher(watcherType)
      this.agentCache.set(watcherType, agents)
    }

    log.debug("Agent cache refreshed", {
      code: this.agentCache.get("code")?.length ?? 0,
      world: this.agentCache.get("world")?.length ?? 0,
      self: this.agentCache.get("self")?.length ?? 0,
      meta: this.agentCache.get("meta")?.length ?? 0,
    })
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Routing Logic
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Route an observation to relevant agents.
   */
  async routeObservation(observation: Observation): Promise<RoutingResult[]> {
    // Check confidence threshold
    if (observation.confidence < this.config.minConfidenceThreshold) {
      log.debug("Observation below confidence threshold", {
        observationId: observation.id,
        confidence: observation.confidence,
        threshold: this.config.minConfidenceThreshold,
      })
      return []
    }

    // Find matching rules
    const matchingRules = this.findMatchingRules(observation)

    if (matchingRules.length === 0) {
      log.debug("No matching rules for observation", {
        observationId: observation.id,
        type: observation.type,
      })
      return []
    }

    // Sort by priority (highest first)
    matchingRules.sort((a, b) => b.priority - a.priority)

    const results: RoutingResult[] = []

    for (const rule of matchingRules) {
      if (rule.requiresApproval) {
        // Queue for human approval
        this.queueForApproval(observation, rule)
        continue
      }

      const ruleResults = await this.executeRule(observation, rule)
      results.push(...ruleResults)
    }

    return results
  }

  /**
   * Route an anomaly to relevant agents.
   */
  async routeAnomaly(anomaly: Anomaly): Promise<RoutingResult[]> {
    log.info("Routing anomaly", {
      anomalyId: anomaly.id,
      severity: anomaly.severity,
      type: anomaly.type,
    })

    // Map anomaly to agents based on source watcher
    // Extract watcher type from first observation ID (format: obs_{type}_...)
    const sourceWatcher = anomaly.observationIds?.[0]?.split("_")[1] as WatcherType | undefined

    let targetAgents: string[] = []

    if (sourceWatcher === "code") {
      targetAgents = ["code-reviewer", "security-reviewer"]
    } else if (sourceWatcher === "world") {
      targetAgents = ["macro", "trader"]
    } else if (sourceWatcher === "self") {
      targetAgents = ["decision"]
    } else if (sourceWatcher === "meta") {
      targetAgents = ["observer"]
    } else {
      // Default to explore for unknown sources
      targetAgents = ["explore"]
    }

    const results: RoutingResult[] = []

    for (const agentId of targetAgents) {
      const result = await this.invokeAgent(agentId, this.buildAnomalyPrompt(anomaly))

      results.push({
        observationId: anomaly.id,
        agentId,
        rule: "anomaly-routing",
        result,
        routedAt: new Date(),
        completedAt: new Date(),
      })
    }

    this.results.push(...results)
    return results
  }

  /**
   * Route an opportunity to relevant agents.
   */
  async routeOpportunity(opportunity: Opportunity): Promise<RoutingResult[]> {
    log.info("Routing opportunity", {
      opportunityId: opportunity.id,
      type: opportunity.type,
      urgency: opportunity.urgency,
    })

    // Map opportunity type to agents
    let targetAgents: string[] = []

    if (opportunity.type === "optimization" || opportunity.type === "automation") {
      targetAgents = ["architect", "explore"]
    } else if (opportunity.type === "learning" || opportunity.type === "improvement") {
      targetAgents = ["code-reviewer", "decision"]
    } else if (opportunity.type === "market" || opportunity.type === "timing") {
      targetAgents = ["macro", "trader"]
    } else {
      targetAgents = ["explore"]
    }

    const results: RoutingResult[] = []

    for (const agentId of targetAgents) {
      const result = await this.invokeAgent(agentId, this.buildOpportunityPrompt(opportunity))

      results.push({
        observationId: opportunity.id,
        agentId,
        rule: "opportunity-routing",
        result,
        routedAt: new Date(),
        completedAt: new Date(),
      })
    }

    this.results.push(...results)
    return results
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private findMatchingRules(observation: Observation): RoutingRule[] {
    const matching: RoutingRule[] = []

    for (const rule of this.rules.values()) {
      // Check observation type
      if (!rule.observationTypes.includes(observation.type)) {
        continue
      }

      // Check watcher type if specified
      if (rule.watcherTypes && !rule.watcherTypes.includes(observation.watcherType as WatcherType)) {
        continue
      }

      // Check confidence threshold
      if (rule.minConfidence && observation.confidence < rule.minConfidence) {
        continue
      }

      matching.push(rule)
    }

    return matching
  }

  private async executeRule(observation: Observation, rule: RoutingRule): Promise<RoutingResult[]> {
    const results: RoutingResult[] = []
    const prompt = this.buildObservationPrompt(observation, rule.promptTemplate)

    if (rule.parallel) {
      // Execute agents in parallel
      const promises = rule.targetAgents.map(async (agentId) => {
        const result = await this.invokeAgent(agentId, prompt)
        return {
          observationId: observation.id,
          agentId,
          rule: rule.id,
          result,
          routedAt: new Date(),
          completedAt: new Date(),
        }
      })

      const parallelResults = await Promise.all(promises)
      results.push(...parallelResults)
    } else {
      // Execute agents sequentially
      for (const agentId of rule.targetAgents) {
        const result = await this.invokeAgent(agentId, prompt)

        results.push({
          observationId: observation.id,
          agentId,
          rule: rule.id,
          result,
          routedAt: new Date(),
          completedAt: new Date(),
        })

        // Stop on first success if sequential
        if (result.success) break
      }
    }

    this.results.push(...results)
    return results
  }

  private async invokeAgent(agentId: string, prompt: string): Promise<AgentResult> {
    // Check capacity
    if (this.runningCount >= this.config.maxConcurrent) {
      if (this.config.enableQueue && this.queue.length < this.config.maxQueueSize) {
        log.debug("Agent call queued", { agentId, queueSize: this.queue.length + 1 })
        return {
          success: false,
          error: "Queued for later execution",
        }
      }

      return {
        success: false,
        error: "Maximum concurrent calls reached and queue full",
      }
    }

    this.runningCount++

    try {
      const client = getAgentClient()
      const invocation: AgentInvocation = {
        agentId,
        prompt,
        maxTurns: 3,
        timeoutMs: 60000,
      }

      const result = await client.invoke(invocation)

      log.debug("Agent invocation completed", {
        agentId,
        success: result.success,
      })

      return result
    } finally {
      this.runningCount--
      this.processQueue()
    }
  }

  private processQueue(): void {
    if (this.queue.length === 0 || this.runningCount >= this.config.maxConcurrent) {
      return
    }

    const next = this.queue.shift()
    if (next) {
      void this.routeObservation(next.observation)
    }
  }

  private queueForApproval(observation: Observation, rule: RoutingRule): void {
    log.info("Observation queued for approval", {
      observationId: observation.id,
      rule: rule.id,
    })

    // Could emit an event for UI to handle
    // For now, just log it
  }

  private buildObservationPrompt(observation: Observation, template?: string): string {
    if (template) {
      return template.replace("{{observation}}", JSON.stringify(observation, null, 2))
    }

    return `Analyze the following observation from the Observer Network:

## Observation
- ID: ${observation.id}
- Type: ${observation.type}
- Watcher: ${observation.watcherType}
- Confidence: ${(observation.confidence * 100).toFixed(1)}%
- Timestamp: ${observation.timestamp.toISOString()}

## Data
\`\`\`json
${JSON.stringify(observation, null, 2)}
\`\`\`

Please provide:
1. A brief analysis of what this observation indicates
2. Any concerns or issues identified
3. Recommended actions if any`
  }

  private buildAnomalyPrompt(anomaly: Anomaly): string {
    return `An anomaly has been detected in the Observer Network:

## Anomaly Details
- ID: ${anomaly.id}
- Type: ${anomaly.type}
- Severity: ${anomaly.severity}
- Status: ${anomaly.status}
- Confidence: ${(anomaly.confidence * 100).toFixed(1)}%
- Detected: ${anomaly.detectedAt.toISOString()}

## Description
${anomaly.description}

## Related Observations
${anomaly.observationIds.join(", ")}

Please analyze this anomaly and provide:
1. Root cause assessment
2. Potential impact
3. Recommended remediation steps`
  }

  private buildOpportunityPrompt(opportunity: Opportunity): string {
    return `An opportunity has been identified by the Observer Network:

## Opportunity Details
- ID: ${opportunity.id}
- Type: ${opportunity.type}
- Impact: ${opportunity.impact}
- Urgency: ${opportunity.urgency}
- Confidence: ${(opportunity.confidence * 100).toFixed(1)}%
- Detected: ${opportunity.detectedAt.toISOString()}

## Description
${opportunity.description}

## Related Observations
${opportunity.observationIds.join(", ")}

## Suggested Actions
${opportunity.suggestedActions.join("\n- ")}

Please evaluate this opportunity and provide:
1. Feasibility assessment
2. Estimated effort/impact
3. Recommended approach`
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let routerInstance: ObservationRouter | null = null

/**
 * Get or create the observation router instance.
 */
export function getObservationRouter(config?: Partial<RouterConfig>): ObservationRouter {
  if (!routerInstance) {
    routerInstance = new ObservationRouter(config)
  }
  return routerInstance
}

/**
 * Reset the observation router instance.
 */
export function resetObservationRouter(): void {
  if (routerInstance) {
    routerInstance.stop()
  }
  routerInstance = null
}

/**
 * Create a new observation router instance.
 */
export function createObservationRouter(config?: Partial<RouterConfig>): ObservationRouter {
  return new ObservationRouter(config)
}
