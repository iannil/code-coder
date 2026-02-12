import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"

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
 * State transition record
 */
interface StateTransition {
  from: string
  to: string
  timestamp: number
}

/**
 * Tool call record
 */
interface ToolCall {
  tool: string
  input: unknown
  timestamp: number
  result: "success" | "error"
}

/**
 * Decision record
 */
interface DecisionRecord {
  id: string
  type: string
  timestamp: number
  result: string
}

/**
 * Safety guardrails
 *
 * Detects and prevents dangerous patterns
 */
export class SafetyGuardrails {
  private config: GuardrailConfig
  private sessionId: string
  private stateTransitions: StateTransition[] = []
  private toolCalls: ToolCall[] = []
  private decisions: DecisionRecord[] = []
  private loopsBroken: Set<string> = new Set()

  constructor(sessionId: string, config: Partial<GuardrailConfig> = {}) {
    this.sessionId = sessionId
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Record a state transition
   */
  recordStateTransition(from: string, to: string): void {
    this.stateTransitions.push({
      from,
      to,
      timestamp: Date.now(),
    })

    // Trim old records
    this.trimRecords()

    // Check for loops
    if (this.config.loopDetectionEnabled) {
      this.detectStateLoop()
    }
  }

  /**
   * Record a tool call
   */
  recordToolCall(tool: string, input: unknown, result: "success" | "error"): void {
    this.toolCalls.push({
      tool,
      input,
      timestamp: Date.now(),
      result,
    })

    // Trim old records
    this.trimRecords()

    // Check for loops
    if (this.config.loopDetectionEnabled) {
      this.detectToolLoop()
    }
  }

  /**
   * Record a decision
   */
  recordDecision(id: string, type: string, result: string): void {
    this.decisions.push({
      id,
      type,
      timestamp: Date.now(),
      result,
    })

    // Trim old records
    this.trimRecords()

    // Check for hesitation
    this.detectDecisionHesitation()
  }

  /**
   * Detect state oscillation loops
   */
  private detectStateLoop(): boolean {
    const recent = this.stateTransitions.slice(-this.config.loopThreshold * 2)

    if (recent.length < this.config.loopThreshold * 2) {
      return false
    }

    // Look for A -> B -> A -> B patterns
    for (let i = 0; i <= recent.length - 4; i++) {
      const t1 = recent[i]
      const t2 = recent[i + 1]
      const t3 = recent[i + 2]
      const t4 = recent[i + 3]

      if (
        t1 &&
        t2 &&
        t3 &&
        t4 &&
        t1.from === t3.from &&
        t1.to === t3.to &&
        t2.from === t4.from &&
        t2.to === t4.to &&
        t1.from === t2.to && // Oscillating between two states
        t2.from === t1.to
      ) {
        const loopKey = `state:${t1.from}<->${t2.from}`

        if (this.loopsBroken.has(loopKey)) {
          continue
        }

        log.warn("State loop detected", {
          pattern: `${t1.from} <-> ${t2.from}`,
          count: this.config.loopThreshold,
        })

        if (this.config.autoBreakLoops) {
          this.loopsBroken.add(loopKey)
        }

        Bus.publish(AutonomousEvent.LoopDetected, {
          sessionId: this.sessionId,
          loopType: "state",
          pattern: [t1.from, t2.from],
          count: this.config.loopThreshold,
          broken: this.config.autoBreakLoops,
        })

        return true
      }
    }

    return false
  }

  /**
   * Detect tool call loops
   */
  private detectToolLoop(): boolean {
    const recent = this.toolCalls.slice(-this.config.loopThreshold)

    if (recent.length < this.config.loopThreshold) {
      return false
    }

    // Check if same tool with same input called repeatedly
    const last = recent[recent.length - 1]
    if (!last) return false

    let count = 0
    for (const call of recent) {
      if (
        call.tool === last.tool &&
        JSON.stringify(call.input) === JSON.stringify(last.input) &&
        call.result === "error"
      ) {
        count++
      }
    }

    if (count >= this.config.loopThreshold) {
      const loopKey = `tool:${last.tool}`

      if (this.loopsBroken.has(loopKey)) {
        return true
      }

      log.warn("Tool loop detected", {
        tool: last.tool,
        count,
      })

      if (this.config.autoBreakLoops) {
        this.loopsBroken.add(loopKey)
      }

      Bus.publish(AutonomousEvent.LoopDetected, {
        sessionId: this.sessionId,
        loopType: "tool",
        pattern: [last.tool, last.input],
        count,
        broken: this.config.autoBreakLoops,
      })

      return true
    }

    return false
  }

  /**
   * Detect decision hesitation (repeated decision calls)
   */
  private detectDecisionHesitation(): boolean {
    const recent = this.decisions.slice(-this.config.maxDecisionHesitation)

    if (recent.length < this.config.maxDecisionHesitation) {
      return false
    }

    // Count unique decision types
    const uniqueTypes = new Set(recent.map((d) => d.type))

    if (uniqueTypes.size === 1 && recent.length >= this.config.maxDecisionHesitation) {
      const type = Array.from(uniqueTypes)[0]

      log.warn("Decision hesitation detected", {
        type,
        count: recent.length,
      })

      Bus.publish(AutonomousEvent.LoopDetected, {
        sessionId: this.sessionId,
        loopType: "decision",
        pattern: [type],
        count: recent.length,
        broken: false, // Don't auto-break decision loops
      })

      return true
    }

    return false
  }

  /**
   * Check if any safety limits are exceeded
   */
  checkLimits(): {
    safe: boolean
    reason?: string
    limitType?: "transitions" | "toolRetries" | "decisionHesitation"
  } {
    if (this.stateTransitions.length >= this.config.maxStateTransitions) {
      return {
        safe: false,
        reason: `Maximum state transitions exceeded: ${this.stateTransitions.length}/${this.config.maxStateTransitions}`,
        limitType: "transitions",
      }
    }

    // Count consecutive tool failures
    let consecutiveFailures = 0
    for (let i = this.toolCalls.length - 1; i >= 0; i--) {
      if (this.toolCalls[i]?.result === "error") {
        consecutiveFailures++
      } else {
        break
      }
    }

    if (consecutiveFailures >= this.config.maxToolRetries) {
      return {
        safe: false,
        reason: `Maximum tool retries exceeded: ${consecutiveFailures}/${this.config.maxToolRetries}`,
        limitType: "toolRetries",
      }
    }

    return { safe: true }
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
    return {
      stateTransitions: this.stateTransitions.length,
      toolCalls: this.toolCalls.length,
      decisions: this.decisions.length,
      loopsBroken: this.loopsBroken.size,
    }
  }

  /**
   * Trim old records
   */
  private trimRecords(): void {
    const maxRecords = this.config.maxStateTransitions * 2

    if (this.stateTransitions.length > maxRecords) {
      this.stateTransitions = this.stateTransitions.slice(-maxRecords)
    }

    if (this.toolCalls.length > maxRecords) {
      this.toolCalls = this.toolCalls.slice(-maxRecords)
    }

    if (this.decisions.length > maxRecords) {
      this.decisions = this.decisions.slice(-maxRecords)
    }
  }

  /**
   * Clear all records
   */
  clear(): void {
    this.stateTransitions = []
    this.toolCalls = []
    this.decisions = []
    this.loopsBroken.clear()
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
    return {
      stateTransitions: this.stateTransitions,
      toolCalls: this.toolCalls,
      decisions: this.decisions,
      loopsBroken: Array.from(this.loopsBroken),
    }
  }

  /**
   * Deserialize
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
    const guardrails = new SafetyGuardrails(sessionId, config)
    guardrails.stateTransitions = data.stateTransitions
    guardrails.toolCalls = data.toolCalls
    guardrails.decisions = data.decisions
    guardrails.loopsBroken = new Set(data.loopsBroken)
    return guardrails
  }
}

/**
 * Create safety guardrails
 */
export function createGuardrails(sessionId: string, config?: Partial<GuardrailConfig>): SafetyGuardrails {
  return new SafetyGuardrails(sessionId, config)
}
