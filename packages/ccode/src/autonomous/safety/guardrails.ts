import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import {
  createSafetyGuardrails,
  type SafetyGuardrailsHandleType,
  type NapiGuardrailConfig,
  type NapiLoopDetection,
  type NapiSafetyCheckResult,
  type NapiGuardrailStats,
  type NapiToolResult,
} from "@codecoder-ai/core"

const log = Log.create({ service: "autonomous.safety.guardrails" })

/**
 * Loop detection patterns
 */
export interface LoopPattern {
  type: "state" | "tool" | "decision"
  pattern: unknown[]
  count: number
  window: number // time window in ms
}

/**
 * Guardrail configuration
 */
export interface GuardrailConfig {
  maxStateTransitions: number
  maxToolRetries: number
  maxDecisionHesitation: number
  loopDetectionEnabled: boolean
  loopThreshold: number
  autoBreakLoops: boolean
}

/**
 * Default guardrail config
 */
const DEFAULT_CONFIG: GuardrailConfig = {
  maxStateTransitions: 100,
  maxToolRetries: 3,
  maxDecisionHesitation: 5,
  loopDetectionEnabled: true,
  loopThreshold: 3,
  autoBreakLoops: true,
}

/**
 * State transition record (for serialization compatibility)
 */
interface StateTransition {
  from: string
  to: string
  timestamp: number
}

/**
 * Tool call record (for serialization compatibility)
 */
interface ToolCall {
  tool: string
  input: unknown
  timestamp: number
  result: "success" | "error"
}

/**
 * Decision record (for serialization compatibility)
 */
interface DecisionRecord {
  id: string
  type: string
  timestamp: number
  result: string
}

/**
 * Safety guardrails - Rust-backed implementation
 *
 * Detects and prevents dangerous patterns using native loop detection.
 * Event publishing is handled in TypeScript for Bus compatibility.
 */
export class SafetyGuardrails {
  private handle: SafetyGuardrailsHandleType
  private config: GuardrailConfig
  private sessionId: string

  constructor(sessionId: string, config: Partial<GuardrailConfig> = {}) {
    this.sessionId = sessionId
    this.config = { ...DEFAULT_CONFIG, ...config }

    // Convert config to NAPI format
    const napiConfig: NapiGuardrailConfig = {
      maxStateTransitions: this.config.maxStateTransitions,
      maxToolRetries: this.config.maxToolRetries,
      maxDecisionHesitation: this.config.maxDecisionHesitation,
      loopDetectionEnabled: this.config.loopDetectionEnabled,
      loopThreshold: this.config.loopThreshold,
      autoBreakLoops: this.config.autoBreakLoops,
    }

    // Create Rust-backed handle
    const handle = createSafetyGuardrails?.(sessionId, napiConfig)
    if (!handle) {
      throw new Error("Native SafetyGuardrails not available. Build native modules with `cargo build` in services/zero-core.")
    }
    this.handle = handle
  }

  /**
   * Record a state transition
   */
  recordStateTransition(from: string, to: string): void {
    const loop = this.handle.recordStateTransition(from, to)

    // Publish event if loop detected
    if (loop && this.config.loopDetectionEnabled) {
      this.publishLoopEvent(loop)
    }
  }

  /**
   * Record a tool call
   */
  recordToolCall(tool: string, input: unknown, result: "success" | "error"): void {
    const inputStr = typeof input === "string" ? input : JSON.stringify(input)
    const napiResult = (result === "success" ? "Success" : "Error") as NapiToolResult

    const loop = this.handle.recordToolCall(tool, inputStr, napiResult)

    // Publish event if loop detected
    if (loop && this.config.loopDetectionEnabled) {
      this.publishLoopEvent(loop)
    }
  }

  /**
   * Record a decision
   */
  recordDecision(id: string, type: string, result: string): void {
    const loop = this.handle.recordDecision(id, type, result)

    // Publish event if loop detected (decision hesitation)
    if (loop) {
      this.publishLoopEvent(loop)
    }
  }

  /**
   * Publish loop detection event to the Bus
   */
  private publishLoopEvent(loop: NapiLoopDetection): void {
    log.warn(`${loop.loopType} loop detected`, {
      pattern: loop.pattern,
      count: loop.count,
      broken: loop.broken,
    })

    Bus.publish(AutonomousEvent.LoopDetected, {
      sessionId: this.sessionId,
      loopType: loop.loopType as "state" | "tool" | "decision",
      pattern: loop.pattern,
      count: loop.count,
      broken: loop.broken,
    })
  }

  /**
   * Check if any safety limits are exceeded
   */
  checkLimits(): {
    safe: boolean
    reason?: string
    limitType?: "transitions" | "toolRetries" | "decisionHesitation"
  } {
    const result: NapiSafetyCheckResult = this.handle.checkLimits()

    if (!result.safe) {
      // Map Rust limitType to TS type
      let limitType: "transitions" | "toolRetries" | "decisionHesitation" | undefined
      if (result.limitType === "transitions") limitType = "transitions"
      else if (result.limitType === "toolRetries") limitType = "toolRetries"
      else if (result.limitType === "decisionHesitation") limitType = "decisionHesitation"

      return {
        safe: false,
        reason: result.reason ?? undefined,
        limitType,
      }
    }

    return { safe: true }
  }

  /**
   * Detect all current loops
   * Returns array of loop detections from the Rust implementation
   */
  detectLoops(): Array<{
    loopType: string
    pattern: string[]
    count: number
    broken: boolean
  }> {
    return this.handle.detectLoops()
  }

  /**
   * Get statistics
   */
  getStats(): {
    stateTransitions: number
    toolCalls: number
    decisions: number
    loopsBroken: number
  } {
    const stats: NapiGuardrailStats = this.handle.getStats()
    return {
      stateTransitions: stats.stateTransitions,
      toolCalls: stats.toolCalls,
      decisions: stats.decisions,
      loopsBroken: stats.loopsBroken,
    }
  }

  /**
   * Clear all records
   */
  clear(): void {
    this.handle.clear()
  }

  /**
   * Serialize
   */
  serialize(): {
    stateTransitions: StateTransition[]
    toolCalls: ToolCall[]
    decisions: DecisionRecord[]
    loopsBroken: string[]
  } {
    // Rust handle serializes to JSON string
    const json = this.handle.serialize()
    try {
      return JSON.parse(json)
    } catch {
      // Return empty state on parse error
      return {
        stateTransitions: [],
        toolCalls: [],
        decisions: [],
        loopsBroken: [],
      }
    }
  }

  /**
   * Deserialize - creates new guardrails from saved state
   *
   * Note: This creates a new Rust handle and doesn't restore internal state.
   * For full state restoration, consider persisting and restoring via constructor.
   */
  static deserialize(
    data: {
      stateTransitions: StateTransition[]
      toolCalls: ToolCall[]
      decisions: DecisionRecord[]
      loopsBroken: string[]
    },
    sessionId: string,
    config?: Partial<GuardrailConfig>,
  ): SafetyGuardrails {
    // Create new guardrails (Rust implementation doesn't support restore from state)
    // This is a simplified implementation for backward compatibility
    const guardrails = new SafetyGuardrails(sessionId, config)

    // Replay state transitions
    for (const t of data.stateTransitions) {
      guardrails.recordStateTransition(t.from, t.to)
    }

    // Replay tool calls
    for (const t of data.toolCalls) {
      guardrails.recordToolCall(t.tool, t.input, t.result)
    }

    // Replay decisions
    for (const d of data.decisions) {
      guardrails.recordDecision(d.id, d.type, d.result)
    }

    return guardrails
  }
}

/**
 * Create safety guardrails
 */
export function createGuardrails(sessionId: string, config?: Partial<GuardrailConfig>): SafetyGuardrails {
  return new SafetyGuardrails(sessionId, config)
}
