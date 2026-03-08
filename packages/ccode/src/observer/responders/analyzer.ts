/**
 * Analyzer Responder
 *
 * Triggers deep analysis using existing agents when observations
 * require further investigation.
 *
 * @module observer/responders/analyzer
 */

import { Log } from "@/util/log"
import type { Anomaly, Opportunity, EmergentPattern, WorldModel } from "../types"
import type { ConsensusSnapshot } from "../consensus"
import { ObserverEvent } from "../events"
import {
  getAgentClient,
  type AgentResult,
  type AgentClientConfig,
} from "../integration/agent-client"

const log = Log.create({ service: "observer.responders.analyzer" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type AnalysisType =
  | "anomaly_investigation"
  | "opportunity_assessment"
  | "pattern_analysis"
  | "security_review"
  | "performance_review"
  | "architecture_review"
  | "market_analysis"
  | "decision_review"

export type AnalysisStatus = "pending" | "running" | "completed" | "failed" | "cancelled"

export interface AnalysisRequest {
  id: string
  type: AnalysisType
  trigger: {
    type: "anomaly" | "opportunity" | "pattern" | "manual" | "scheduled"
    id?: string
  }
  context: Record<string, unknown>
  priority: "low" | "medium" | "high"
  createdAt: Date
  startedAt?: Date
  completedAt?: Date
  status: AnalysisStatus
  agentUsed?: string
  result?: AnalysisResult
  error?: string
}

export interface AnalysisResult {
  summary: string
  findings: Array<{
    type: string
    severity: "info" | "warning" | "error" | "critical"
    description: string
    recommendation?: string
  }>
  recommendations: string[]
  metrics?: Record<string, number>
  rawOutput?: unknown
}

export interface AnalyzerConfig {
  /** Enable automatic analysis */
  autoAnalyze: boolean
  /** Maximum concurrent analyses */
  maxConcurrent: number
  /** Agent mappings for analysis types */
  agentMappings: Partial<Record<AnalysisType, string>>
  /** Analysis timeout (ms) */
  timeoutMs: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: AnalyzerConfig = {
  autoAnalyze: true,
  maxConcurrent: 3,
  agentMappings: {
    anomaly_investigation: "explore",
    opportunity_assessment: "decision",
    pattern_analysis: "explore",
    security_review: "security-reviewer",
    performance_review: "architect",
    architecture_review: "architect",
    market_analysis: "macro",
    decision_review: "decision",
  },
  timeoutMs: 300000, // 5 minutes
}

// ─────────────────────────────────────────────────────────────────────────────
// Analyzer
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Triggers deep analysis using existing agents.
 */
export class Analyzer {
  private config: AnalyzerConfig
  private analyses: Map<string, AnalysisRequest> = new Map()
  private runningCount = 0
  private queue: AnalysisRequest[] = []
  private idCounter = 0
  private running = false
  private eventSubscriptions: Array<() => void> = []

  constructor(config: Partial<AnalyzerConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Start the analyzer.
   */
  async start(): Promise<void> {
    if (this.running) return

    this.running = true

    if (this.config.autoAnalyze) {
      const Bus = (await import("@/bus")).Bus

      // Auto-analyze critical anomalies
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.AnomalyDetected, async (event) => {
          const anomaly = event.properties as Anomaly
          if (anomaly.severity === "critical" || anomaly.severity === "high") {
            await this.analyzeAnomaly(anomaly)
          }
        }),
      )

      // Auto-analyze high-impact opportunities
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.OpportunityIdentified, async (event) => {
          const opportunity = event.properties as Opportunity
          if (opportunity.impact === "high") {
            await this.analyzeOpportunity(opportunity)
          }
        }),
      )
    }

    log.info("Analyzer started", {
      autoAnalyze: this.config.autoAnalyze,
      maxConcurrent: this.config.maxConcurrent,
    })
  }

  /**
   * Stop the analyzer.
   */
  stop(): void {
    if (!this.running) return

    this.running = false

    for (const unsubscribe of this.eventSubscriptions) {
      unsubscribe()
    }
    this.eventSubscriptions = []

    // Cancel pending analyses
    for (const analysis of this.queue) {
      analysis.status = "cancelled"
    }
    this.queue = []

    log.info("Analyzer stopped")
  }

  /**
   * Analyze an anomaly.
   */
  async analyzeAnomaly(anomaly: Anomaly): Promise<AnalysisRequest> {
    return this.requestAnalysis("anomaly_investigation", {
      trigger: { type: "anomaly", id: anomaly.id },
      context: { anomaly },
      priority: anomaly.severity === "critical" ? "high" : "medium",
    })
  }

  /**
   * Analyze an opportunity.
   */
  async analyzeOpportunity(opportunity: Opportunity): Promise<AnalysisRequest> {
    return this.requestAnalysis("opportunity_assessment", {
      trigger: { type: "opportunity", id: opportunity.id },
      context: { opportunity },
      priority: opportunity.impact === "high" ? "high" : "medium",
    })
  }

  /**
   * Analyze a pattern.
   */
  async analyzePattern(pattern: EmergentPattern): Promise<AnalysisRequest> {
    return this.requestAnalysis("pattern_analysis", {
      trigger: { type: "pattern", id: pattern.id },
      context: { pattern },
      priority: pattern.strength > 0.8 ? "high" : "medium",
    })
  }

  /**
   * Request a security review.
   */
  async requestSecurityReview(context: Record<string, unknown>): Promise<AnalysisRequest> {
    return this.requestAnalysis("security_review", {
      trigger: { type: "manual" },
      context,
      priority: "high",
    })
  }

  /**
   * Request an architecture review.
   */
  async requestArchitectureReview(context: Record<string, unknown>): Promise<AnalysisRequest> {
    return this.requestAnalysis("architecture_review", {
      trigger: { type: "manual" },
      context,
      priority: "medium",
    })
  }

  /**
   * Request market analysis.
   */
  async requestMarketAnalysis(worldModel: WorldModel): Promise<AnalysisRequest> {
    return this.requestAnalysis("market_analysis", {
      trigger: { type: "manual" },
      context: { worldModel },
      priority: "medium",
    })
  }

  /**
   * Request a general analysis.
   */
  async requestAnalysis(
    type: AnalysisType,
    options: {
      trigger: AnalysisRequest["trigger"]
      context: Record<string, unknown>
      priority: AnalysisRequest["priority"]
    },
  ): Promise<AnalysisRequest> {
    const request: AnalysisRequest = {
      id: `analysis_${Date.now()}_${++this.idCounter}`,
      type,
      trigger: options.trigger,
      context: options.context,
      priority: options.priority,
      createdAt: new Date(),
      status: "pending",
    }

    this.analyses.set(request.id, request)

    // Queue or run immediately
    if (this.runningCount < this.config.maxConcurrent) {
      void this.runAnalysis(request)
    } else {
      this.queue.push(request)
      this.sortQueue()
    }

    return request
  }

  /**
   * Get analysis by ID.
   */
  getAnalysis(id: string): AnalysisRequest | null {
    return this.analyses.get(id) ?? null
  }

  /**
   * Get analysis history.
   */
  getHistory(limit?: number): AnalysisRequest[] {
    return Array.from(this.analyses.values())
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime())
      .slice(0, limit ?? 50)
  }

  /**
   * Get running analyses.
   */
  getRunning(): AnalysisRequest[] {
    return Array.from(this.analyses.values())
      .filter((a) => a.status === "running")
  }

  /**
   * Get pending analyses.
   */
  getPending(): AnalysisRequest[] {
    return [...this.queue]
  }

  /**
   * Cancel an analysis.
   */
  cancel(id: string): boolean {
    const analysis = this.analyses.get(id)
    if (!analysis) return false

    if (analysis.status === "pending") {
      const index = this.queue.findIndex((a) => a.id === id)
      if (index >= 0) {
        this.queue.splice(index, 1)
      }
      analysis.status = "cancelled"
      return true
    }

    // Cannot cancel running analyses for now
    return false
  }

  /**
   * Clear analysis history.
   */
  clear(): void {
    this.analyses.clear()
    this.queue = []
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private sortQueue(): void {
    const priorityOrder: Record<AnalysisRequest["priority"], number> = {
      high: 0,
      medium: 1,
      low: 2,
    }
    this.queue.sort((a, b) => {
      const priorityDiff = priorityOrder[a.priority] - priorityOrder[b.priority]
      if (priorityDiff !== 0) return priorityDiff
      return a.createdAt.getTime() - b.createdAt.getTime()
    })
  }

  private async runAnalysis(request: AnalysisRequest): Promise<void> {
    request.status = "running"
    request.startedAt = new Date()
    this.runningCount++

    const agentId = this.config.agentMappings[request.type] ?? "explore"
    request.agentUsed = agentId

    log.info("Starting analysis", {
      id: request.id,
      type: request.type,
      agent: agentId,
    })

    // Publish started event
    const Bus = (await import("@/bus")).Bus
    await Bus.publish(ObserverEvent.AnalysisTriggered, {
      analysisType: request.type,
      agentUsed: agentId,
      triggeredBy: request.trigger.id ?? "manual",
      status: "started",
    })

    try {
      const result = await this.executeAnalysis(request, agentId)
      request.result = result
      request.status = "completed"
      request.completedAt = new Date()

      await Bus.publish(ObserverEvent.AnalysisTriggered, {
        analysisType: request.type,
        agentUsed: agentId,
        triggeredBy: request.trigger.id ?? "manual",
        status: "completed",
        result: result.summary,
      })

      log.info("Analysis completed", {
        id: request.id,
        findings: result.findings.length,
        recommendations: result.recommendations.length,
      })
    } catch (error) {
      request.status = "failed"
      request.error = String(error)
      request.completedAt = new Date()

      await Bus.publish(ObserverEvent.AnalysisTriggered, {
        analysisType: request.type,
        agentUsed: agentId,
        triggeredBy: request.trigger.id ?? "manual",
        status: "failed",
        result: request.error,
      })

      log.error("Analysis failed", {
        id: request.id,
        error: request.error,
      })
    } finally {
      this.runningCount--
      this.processQueue()
    }
  }

  private async executeAnalysis(request: AnalysisRequest, agentId: string): Promise<AnalysisResult> {
    const prompt = this.buildAnalysisPrompt(request)

    try {
      const agentClient = getAgentClient()
      const agentResult = await agentClient.invoke({
        agentId,
        prompt,
        context: request.context,
        timeoutMs: this.config.timeoutMs,
      })

      if (!agentResult.success) {
        log.warn("Agent analysis failed", {
          requestId: request.id,
          error: agentResult.error,
        })
        return {
          summary: `Analysis failed: ${agentResult.error}`,
          findings: [{
            type: "error",
            severity: "error",
            description: agentResult.error ?? "Unknown error",
          }],
          recommendations: [],
        }
      }

      // Use findings from agent or parse from output
      const findings: AnalysisResult["findings"] = agentResult.findings?.map((f) => ({
        type: f.type,
        severity: f.severity,
        description: f.description,
        recommendation: f.recommendation,
      })) ?? this.extractFindings(agentResult.output ?? "")

      // Use recommendations from agent or extract from output
      const recommendations = agentResult.recommendations?.length
        ? agentResult.recommendations
        : this.extractRecommendations(agentResult.output ?? "")

      return {
        summary: this.extractSummary(agentResult.output ?? "") || `${request.type} analysis completed`,
        findings,
        recommendations,
        metrics: agentResult.usage ? {
          tokens: agentResult.usage.tokens,
          duration: agentResult.usage.duration,
        } : undefined,
        rawOutput: agentResult.output,
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      log.error("Analysis execution error", {
        requestId: request.id,
        error: errorMessage,
      })
      return {
        summary: `Analysis failed: ${errorMessage}`,
        findings: [{
          type: "error",
          severity: "error",
          description: errorMessage,
        }],
        recommendations: [],
      }
    }
  }

  private buildAnalysisPrompt(request: AnalysisRequest): string {
    const lines: string[] = []

    lines.push(`# ${this.formatAnalysisType(request.type)} Analysis Request`)
    lines.push("")

    switch (request.type) {
      case "anomaly_investigation":
        lines.push("Investigate the following anomaly and provide root cause analysis:")
        break
      case "opportunity_assessment":
        lines.push("Assess the following opportunity using CLOSE evaluation framework:")
        break
      case "pattern_analysis":
        lines.push("Analyze the following pattern and its implications:")
        break
      case "security_review":
        lines.push("Perform a security review and identify potential vulnerabilities:")
        break
      case "performance_review":
        lines.push("Analyze performance characteristics and identify optimization opportunities:")
        break
      case "architecture_review":
        lines.push("Review the architecture and provide recommendations:")
        break
      case "market_analysis":
        lines.push("Analyze market conditions based on the world model:")
        break
      case "decision_review":
        lines.push("Review the decision using CLOSE framework (Convergence, Leverage, Optionality, Surplus, Evolution):")
        break
    }

    lines.push("")
    lines.push("## Request Details")
    lines.push("")
    lines.push(`- **Type**: ${request.type}`)
    lines.push(`- **Priority**: ${request.priority}`)
    lines.push(`- **Trigger**: ${request.trigger.type}${request.trigger.id ? ` (${request.trigger.id})` : ""}`)
    lines.push("")
    lines.push("## Expected Output")
    lines.push("")
    lines.push("Please provide:")
    lines.push("1. A summary of your findings")
    lines.push("2. Specific issues or findings with severity levels")
    lines.push("3. Actionable recommendations")
    lines.push("")

    return lines.join("\n")
  }

  private formatAnalysisType(type: AnalysisType): string {
    return type.split("_").map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" ")
  }

  private extractSummary(output: string): string | null {
    // Try to extract summary from markdown output
    const summaryMatch = output.match(/(?:#{1,3}\s*)?(?:Summary|Overview)[:\s]+(.+?)(?=\n#{1,3}|\n\n|\z)/is)
    if (summaryMatch?.[1]) {
      return summaryMatch[1].trim()
    }
    // Use first paragraph as summary
    const firstPara = output.split("\n\n")[0]?.trim()
    return firstPara?.length > 0 && firstPara.length < 500 ? firstPara : null
  }

  private extractFindings(output: string): AnalysisResult["findings"] {
    const findings: AnalysisResult["findings"] = []

    // Look for markdown findings patterns
    const patterns = [
      /(?:#{1,3}\s*)?(?:Finding|Issue|Warning|Error|Critical)[:\s]+(.+?)(?=\n#{1,3}|\n\n|\z)/gis,
      /[-*]\s*\*\*(\w+)\*\*[:\s]+(.+?)(?=\n[-*]|\n\n|\z)/gis,
    ]

    for (const pattern of patterns) {
      let match
      while ((match = pattern.exec(output)) !== null) {
        const text = match[1]?.trim() || match[2]?.trim()
        if (text) {
          findings.push({
            type: "extracted",
            severity: this.inferSeverity(text),
            description: text,
          })
        }
      }
    }

    return findings
  }

  private extractRecommendations(output: string): string[] {
    const recommendations: string[] = []

    const patterns = [
      /(?:#{1,3}\s*)?(?:Recommend(?:ation)?s?)[:\s]+(.+?)(?=\n#{1,3}|\n\n|\z)/gis,
      /[-*]\s*(?:should|consider|recommend)[:\s]+(.+?)(?=\n[-*]|\n\n|\z)/gis,
    ]

    for (const pattern of patterns) {
      let match
      while ((match = pattern.exec(output)) !== null) {
        const text = match[1]?.trim()
        if (text) {
          recommendations.push(text)
        }
      }
    }

    return recommendations
  }

  private inferSeverity(text: string): AnalysisResult["findings"][0]["severity"] {
    const lower = text.toLowerCase()
    if (lower.includes("critical") || lower.includes("urgent")) return "critical"
    if (lower.includes("error") || lower.includes("fail")) return "error"
    if (lower.includes("warning") || lower.includes("caution")) return "warning"
    return "info"
  }

  private processQueue(): void {
    while (this.runningCount < this.config.maxConcurrent && this.queue.length > 0) {
      const next = this.queue.shift()
      if (next) {
        void this.runAnalysis(next)
      }
    }
  }
}

/**
 * Create an analyzer.
 */
export function createAnalyzer(config?: Partial<AnalyzerConfig>): Analyzer {
  return new Analyzer(config)
}
