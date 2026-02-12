import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import z from "zod"

const log = Log.create({ service: "autonomous.safety.constraints" })

/**
 * Resource budget limits
 */
export interface ResourceBudget {
  maxTokens: number
  maxCostUSD: number
  maxDurationMinutes: number
  maxFilesChanged: number
  maxActions: number
}

/**
 * Current resource usage
 */
export interface ResourceUsage {
  tokensUsed: number
  costUSD: number
  durationMinutes: number
  filesChanged: number
  actionsPerformed: number
}

/**
 * Safety check result
 */
export interface SafetyCheckResult {
  safe: boolean
  reason?: string
  resource?: keyof ResourceBudget
  current?: number
  limit?: number
}

/**
 * Safety configuration
 */
export interface SafetyConfig {
  budget: ResourceBudget
  warnThreshold: number // Percentage (0-100) to warn at
  hardLimit: boolean // Enforce hard limits or just warn
}

/**
 * Default resource budget
 */
export const DEFAULT_BUDGET: ResourceBudget = {
  maxTokens: 1_000_000,
  maxCostUSD: 10.0,
  maxDurationMinutes: 30,
  maxFilesChanged: 50,
  maxActions: 100,
}

/**
 * Safety guard for resource limits
 *
 * Monitors and enforces resource constraints
 */
export class SafetyGuard {
  private config: SafetyConfig
  private usage: ResourceUsage
  private startTime: number
  private sessionId: string
  private warningSent: Set<keyof ResourceBudget> = new Set()

  constructor(sessionId: string, budget?: Partial<ResourceBudget>) {
    this.sessionId = sessionId
    this.startTime = Date.now()
    this.config = {
      budget: { ...DEFAULT_BUDGET, ...budget },
      warnThreshold: 80,
      hardLimit: true,
    }
    this.usage = {
      tokensUsed: 0,
      costUSD: 0,
      durationMinutes: 0,
      filesChanged: 0,
      actionsPerformed: 0,
    }
  }

  /**
   * Check if an action is safe to perform
   */
  async check(action?: keyof ResourceBudget, additionalCost?: Partial<ResourceUsage>): Promise<SafetyCheckResult> {
    // Update duration
    this.usage.durationMinutes = (Date.now() - this.startTime) / 60000

    // Add additional costs if provided
    if (additionalCost) {
      if (additionalCost.tokensUsed) this.usage.tokensUsed += additionalCost.tokensUsed
      if (additionalCost.costUSD) this.usage.costUSD += additionalCost.costUSD
      if (additionalCost.filesChanged) this.usage.filesChanged += additionalCost.filesChanged
      if (additionalCost.actionsPerformed) this.usage.actionsPerformed += additionalCost.actionsPerformed
    }

    // Check each resource
    const checks: Array<{ resource: keyof ResourceBudget; current: number; limit: number }> = [
      { resource: "maxTokens", current: this.usage.tokensUsed, limit: this.config.budget.maxTokens },
      { resource: "maxCostUSD", current: this.usage.costUSD, limit: this.config.budget.maxCostUSD },
      {
        resource: "maxDurationMinutes",
        current: this.usage.durationMinutes,
        limit: this.config.budget.maxDurationMinutes,
      },
      {
        resource: "maxFilesChanged",
        current: this.usage.filesChanged,
        limit: this.config.budget.maxFilesChanged,
      },
      { resource: "maxActions", current: this.usage.actionsPerformed, limit: this.config.budget.maxActions },
    ]

    // If specific action requested, check that
    if (action) {
      const check = checks.find((c) => c.resource === `max${this.capitalize(action)}` as keyof ResourceBudget)
      if (check) {
        return this.checkResource(check.resource, check.current, check.limit)
      }
    }

    // Check all resources
    for (const check of checks) {
      const result = this.checkResource(check.resource, check.current, check.limit)
      if (!result.safe && this.config.hardLimit) {
        return result
      }
    }

    return { safe: true }
  }

  /**
   * Record resource usage
   */
  record(resource: keyof ResourceUsage, value: number): void {
    this.usage[resource] = (this.usage[resource] ?? 0) + value
  }

  /**
   * Get current usage
   */
  getCurrentUsage(): ResourceUsage {
    // Update duration
    this.usage.durationMinutes = (Date.now() - this.startTime) / 60000
    return { ...this.usage }
  }

  /**
   * Get remaining budget
   */
  getRemaining(): Partial<ResourceBudget> {
    const usage = this.getCurrentUsage()
    return {
      maxTokens: Math.max(0, this.config.budget.maxTokens - usage.tokensUsed),
      maxCostUSD: Math.max(0, this.config.budget.maxCostUSD - usage.costUSD),
      maxDurationMinutes: Math.max(0, this.config.budget.maxDurationMinutes - usage.durationMinutes),
      maxFilesChanged: Math.max(0, this.config.budget.maxFilesChanged - usage.filesChanged),
      maxActions: Math.max(0, this.config.budget.maxActions - usage.actionsPerformed),
    }
  }

  /**
   * Get surplus ratio (0-1)
   */
  getSurplusRatio(): number {
    const remaining = this.getRemaining()
    const total = this.config.budget

    // Calculate average remaining ratio
    const ratios = [
      (remaining.maxTokens ?? 0) / total.maxTokens,
      (remaining.maxCostUSD ?? 0) / total.maxCostUSD,
      (remaining.maxDurationMinutes ?? 0) / total.maxDurationMinutes,
      (remaining.maxFilesChanged ?? 0) / total.maxFilesChanged,
      (remaining.maxActions ?? 0) / total.maxActions,
    ]

    return ratios.reduce((sum, r) => sum + r, 0) / ratios.length
  }

  /**
   * Check a specific resource
   */
  private checkResource(
    resource: keyof ResourceBudget,
    current: number,
    limit: number,
  ): SafetyCheckResult {
    const percentage = (current / limit) * 100

    // Hard limit exceeded
    if (current >= limit) {
      log.warn("Resource limit exceeded", { resource, current, limit })

      return {
        safe: false,
        reason: `Resource limit exceeded for ${resource}`,
        resource,
        current,
        limit,
      }
    }

    // Warning threshold
    if (percentage >= this.config.warnThreshold && !this.warningSent.has(resource)) {
      this.warningSent.add(resource)

      Bus.publish(AutonomousEvent.ResourceWarning, {
        sessionId: this.sessionId,
        resource: this.normalizeResourceName(resource),
        current,
        limit,
        percentage: Math.round(percentage),
      })

      log.warn("Resource warning", { resource, current, limit, percentage: Math.round(percentage) })
    }

    // Approaching limit - send warning once
    if (percentage >= this.config.warnThreshold) {
      if (!this.warningSent.has(resource)) {
        this.warningSent.add(resource)

        Bus.publish(AutonomousEvent.ResourceWarning, {
          sessionId: this.sessionId,
          resource: this.normalizeResourceName(resource),
          current,
          limit,
          percentage: Math.round(percentage),
        })
      }
    }

    return { safe: true }
  }

  /**
   * Normalize resource name for events
   */
  private normalizeResourceName(resource: keyof ResourceBudget): "tokens" | "cost" | "time" | "files" | "actions" {
    switch (resource) {
      case "maxTokens":
        return "tokens"
      case "maxCostUSD":
        return "cost"
      case "maxDurationMinutes":
        return "time"
      case "maxFilesChanged":
        return "files"
      case "maxActions":
        return "actions"
    }
  }

  /**
   * Capitalize string
   */
  private capitalize(str: string): string {
    return str.charAt(0).toUpperCase() + str.slice(1)
  }

  /**
   * Reset usage
   */
  reset(): void {
    this.usage = {
      tokensUsed: 0,
      costUSD: 0,
      durationMinutes: 0,
      filesChanged: 0,
      actionsPerformed: 0,
    }
    this.startTime = Date.now()
    this.warningSent.clear()
  }

  /**
   * Update budget
   */
  updateBudget(budget: Partial<ResourceBudget>): void {
    this.config.budget = { ...this.config.budget, ...budget }
  }

  /**
   * Get config
   */
  getConfig(): SafetyConfig {
    return { ...this.config }
  }

  /**
   * Serialize guard state
   */
  serialize(): { usage: ResourceUsage; startTime: number; config: SafetyConfig } {
    return {
      usage: this.getCurrentUsage(),
      startTime: this.startTime,
      config: this.config,
    }
  }

  /**
   * Restore from serialized data
   */
  static deserialize(data: {
    usage: ResourceUsage
    startTime: number
    config: SafetyConfig
  }, sessionId: string): SafetyGuard {
    const guard = new SafetyGuard(sessionId, data.config.budget)
    guard.usage = data.usage
    guard.startTime = data.startTime
    guard.config = data.config
    return guard
  }
}

/**
 * Schema for resource budget configuration
 */
export const ResourceBudgetSchema = z.object({
  maxTokens: z.number().min(0).optional(),
  maxCostUSD: z.number().min(0).optional(),
  maxDurationMinutes: z.number().min(0).optional(),
  maxFilesChanged: z.number().min(0).optional(),
  maxActions: z.number().min(0).optional(),
})

/**
 * Parse resource budget from config
 */
export function parseResourceBudget(config: unknown): ResourceBudget {
  const parsed = ResourceBudgetSchema.partial().parse(config)
  return {
    ...DEFAULT_BUDGET,
    ...parsed,
  }
}
