import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import z from "zod"
import {
  createSafetyGuard,
  type SafetyGuardHandleType,
  type NapiResourceBudget,
  type NapiResourceUsage,
  type NapiConstraintCheckResult,
  type NapiResourceWarning,
} from "@codecoder-ai/core"

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
 * Map resource type string to ResourceBudget key
 */
function mapResourceToKey(resource: string | null | undefined): keyof ResourceBudget | undefined {
  switch (resource) {
    case "tokens":
      return "maxTokens"
    case "cost":
      return "maxCostUSD"
    case "time":
    case "duration":
      return "maxDurationMinutes"
    case "files":
      return "maxFilesChanged"
    case "actions":
      return "maxActions"
    default:
      return undefined
  }
}

/**
 * Safety guard for resource limits - Rust-backed implementation
 *
 * Monitors and enforces resource constraints using native implementation.
 * Event publishing is handled in TypeScript for Bus compatibility.
 */
export class SafetyGuard {
  private handle: SafetyGuardHandleType
  private config: SafetyConfig
  private sessionId: string
  private warningSent: Set<keyof ResourceBudget> = new Set()

  constructor(sessionId: string, budget?: Partial<ResourceBudget>) {
    this.sessionId = sessionId
    const fullBudget = { ...DEFAULT_BUDGET, ...budget }
    this.config = {
      budget: fullBudget,
      warnThreshold: 80,
      hardLimit: true,
    }

    // Convert to NAPI format
    const napiBudget: NapiResourceBudget = {
      maxTokens: fullBudget.maxTokens,
      maxCostUsd: fullBudget.maxCostUSD,
      maxDurationMinutes: fullBudget.maxDurationMinutes,
      maxFilesChanged: fullBudget.maxFilesChanged,
      maxActions: fullBudget.maxActions,
    }

    // Create Rust-backed handle
    const handle = createSafetyGuard?.(sessionId, napiBudget)
    if (!handle) {
      throw new Error("Native SafetyGuard not available. Build native modules with `cargo build` in services/zero-core.")
    }
    this.handle = handle
  }

  /**
   * Check if an action is safe to perform
   */
  async check(action?: keyof ResourceBudget, additionalCost?: Partial<ResourceUsage>): Promise<SafetyCheckResult> {
    // Convert additional cost to NAPI format
    const napiUsage: NapiResourceUsage | undefined = additionalCost
      ? {
          tokensUsed: additionalCost.tokensUsed,
          costUsd: additionalCost.costUSD,
          durationMinutes: additionalCost.durationMinutes,
          filesChanged: additionalCost.filesChanged,
          actionsPerformed: additionalCost.actionsPerformed,
        }
      : undefined

    // Call Rust check with warnings
    const { result, warnings } = this.handle.check(napiUsage)

    // Publish warnings to Bus (TypeScript layer handles events)
    for (const warning of warnings) {
      this.publishWarning(warning)
    }

    // Convert result
    if (!result.safe && this.config.hardLimit) {
      const resourceKey = mapResourceToKey(result.resource)
      return {
        safe: false,
        reason: result.reason ?? undefined,
        resource: resourceKey,
        current: result.current ?? undefined,
        limit: result.limit ?? undefined,
      }
    }

    return { safe: true }
  }

  /**
   * Publish resource warning to Bus
   */
  private publishWarning(warning: NapiResourceWarning): void {
    const resourceKey = mapResourceToKey(warning.resource)
    if (!resourceKey || this.warningSent.has(resourceKey)) {
      return
    }

    this.warningSent.add(resourceKey)

    log.warn("Resource warning", {
      resource: warning.resource,
      current: warning.current,
      limit: warning.limit,
      percentage: warning.percentage,
    })

    Bus.publish(AutonomousEvent.ResourceWarning, {
      sessionId: this.sessionId,
      resource: warning.resource as "tokens" | "cost" | "time" | "files" | "actions",
      current: warning.current,
      limit: warning.limit,
      percentage: warning.percentage,
    })
  }

  /**
   * Record resource usage
   */
  record(resource: keyof ResourceUsage, value: number): void {
    const usage: NapiResourceUsage = {}
    switch (resource) {
      case "tokensUsed":
        usage.tokensUsed = value
        break
      case "costUSD":
        usage.costUsd = value
        break
      case "durationMinutes":
        usage.durationMinutes = value
        break
      case "filesChanged":
        usage.filesChanged = value
        break
      case "actionsPerformed":
        usage.actionsPerformed = value
        break
    }
    this.handle.record(usage)
  }

  /**
   * Get current usage
   */
  getCurrentUsage(): ResourceUsage {
    const usage = this.handle.getUsage()
    return {
      tokensUsed: usage.tokensUsed ?? 0,
      costUSD: usage.costUsd ?? 0,
      durationMinutes: usage.durationMinutes ?? 0,
      filesChanged: usage.filesChanged ?? 0,
      actionsPerformed: usage.actionsPerformed ?? 0,
    }
  }

  /**
   * Get remaining budget
   */
  getRemaining(): Partial<ResourceBudget> {
    const remaining = this.handle.getRemaining()
    return {
      maxTokens: remaining.maxTokens ?? 0,
      maxCostUSD: remaining.maxCostUsd ?? 0,
      maxDurationMinutes: remaining.maxDurationMinutes ?? 0,
      maxFilesChanged: remaining.maxFilesChanged ?? 0,
      maxActions: remaining.maxActions ?? 0,
    }
  }

  /**
   * Get surplus ratio (0-1)
   */
  getSurplusRatio(): number {
    return this.handle.getSurplusRatio()
  }

  /**
   * Get warnings count (number of resources that have triggered warnings)
   */
  getWarningsCount(): number {
    return this.warningSent.size
  }

  /**
   * Reset usage
   */
  reset(): void {
    this.handle.reset()
    this.warningSent.clear()
  }

  /**
   * Update budget
   */
  updateBudget(budget: Partial<ResourceBudget>): void {
    this.config.budget = { ...this.config.budget, ...budget }

    const napiBudget: NapiResourceBudget = {
      maxTokens: budget.maxTokens,
      maxCostUsd: budget.maxCostUSD,
      maxDurationMinutes: budget.maxDurationMinutes,
      maxFilesChanged: budget.maxFilesChanged,
      maxActions: budget.maxActions,
    }
    this.handle.updateBudget(napiBudget)
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
    const json = this.handle.serialize()
    try {
      const data = JSON.parse(json)
      return {
        usage: this.getCurrentUsage(),
        startTime: data.startTime ?? Date.now(),
        config: this.config,
      }
    } catch {
      return {
        usage: this.getCurrentUsage(),
        startTime: Date.now(),
        config: this.config,
      }
    }
  }

  /**
   * Restore from serialized data
   */
  static deserialize(
    data: {
      usage: ResourceUsage
      startTime: number
      config: SafetyConfig
    },
    sessionId: string,
  ): SafetyGuard {
    const guard = new SafetyGuard(sessionId, data.config.budget)

    // Add recorded usage to the new guard
    if (data.usage.tokensUsed > 0) {
      guard.handle.addTokens(data.usage.tokensUsed)
    }
    if (data.usage.costUSD > 0) {
      guard.handle.addCost(data.usage.costUSD)
    }
    if (data.usage.filesChanged > 0) {
      guard.handle.addFilesChanged(data.usage.filesChanged)
    }
    if (data.usage.actionsPerformed > 0) {
      guard.handle.addActions(data.usage.actionsPerformed)
    }

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
