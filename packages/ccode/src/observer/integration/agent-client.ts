/**
 * Agent Client for Observer Network
 *
 * Provides agent invocation for Observer Network analysis tasks.
 * Supports calling agents like explore, security-reviewer, decision, etc.
 *
 * @module observer/integration/agent-client
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "observer.integration.agent" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface AgentInvocation {
  /** Agent ID to invoke */
  agentId: string
  /** Prompt for the agent */
  prompt: string
  /** Context data */
  context?: Record<string, unknown>
  /** Maximum turns/iterations */
  maxTurns?: number
  /** Timeout in milliseconds */
  timeoutMs?: number
}

export interface AgentResult {
  success: boolean
  output?: string
  findings?: AgentFinding[]
  recommendations?: string[]
  error?: string
  usage?: {
    tokens: number
    duration: number
  }
}

export interface AgentFinding {
  type: string
  severity: "info" | "warning" | "error" | "critical"
  description: string
  recommendation?: string
}

export interface AgentClientConfig {
  /** Base URL for CodeCoder API */
  baseUrl: string
  /** Request timeout in milliseconds */
  timeoutMs: number
  /** Maximum concurrent agent calls */
  maxConcurrent: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: AgentClientConfig = {
  baseUrl: "http://127.0.0.1:4400",
  timeoutMs: 300000, // 5 minutes
  maxConcurrent: 3,
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Client
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Client for invoking agents from Observer Network.
 */
export class AgentClient {
  private config: AgentClientConfig
  private runningCount = 0

  constructor(config: Partial<AgentClientConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Invoke an agent with a prompt.
   */
  async invoke(invocation: AgentInvocation): Promise<AgentResult> {
    if (this.runningCount >= this.config.maxConcurrent) {
      return {
        success: false,
        error: "Maximum concurrent agent calls reached",
      }
    }

    this.runningCount++

    try {
      const result = await this.executeAgentCall(invocation)
      return result
    } finally {
      this.runningCount--
    }
  }

  /**
   * Check if agents are available.
   */
  async isAvailable(): Promise<boolean> {
    try {
      const response = await fetch(`${this.config.baseUrl}/health`, {
        signal: AbortSignal.timeout(5000),
      })
      return response.ok
    } catch {
      return false
    }
  }

  /**
   * Get currently running count.
   */
  getRunningCount(): number {
    return this.runningCount
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private async executeAgentCall(invocation: AgentInvocation): Promise<AgentResult> {
    const startTime = Date.now()

    try {
      // Build the prompt with context
      const fullPrompt = this.buildPrompt(invocation)

      // Call the agent API
      // Using the session API to invoke agent
      const response = await fetch(`${this.config.baseUrl}/api/v1/chat`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          agent: invocation.agentId,
          message: fullPrompt,
          options: {
            maxTurns: invocation.maxTurns ?? 5,
          },
        }),
        signal: AbortSignal.timeout(invocation.timeoutMs ?? this.config.timeoutMs),
      })

      if (!response.ok) {
        const errorText = await response.text()
        log.error("Agent call failed", {
          agentId: invocation.agentId,
          status: response.status,
          error: errorText,
        })
        return {
          success: false,
          error: `HTTP ${response.status}: ${errorText}`,
        }
      }

      const data = await response.json() as {
        response?: string
        output?: string
        usage?: { tokens: number }
      }

      const duration = Date.now() - startTime
      const output = data.response ?? data.output ?? ""

      // Parse findings from output if available
      const findings = this.extractFindings(output)
      const recommendations = this.extractRecommendations(output)

      log.debug("Agent call completed", {
        agentId: invocation.agentId,
        duration,
        outputLength: output.length,
      })

      return {
        success: true,
        output,
        findings,
        recommendations,
        usage: {
          tokens: data.usage?.tokens ?? 0,
          duration,
        },
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      log.error("Agent call error", {
        agentId: invocation.agentId,
        error: errorMessage,
      })
      return {
        success: false,
        error: errorMessage,
      }
    }
  }

  private buildPrompt(invocation: AgentInvocation): string {
    let prompt = invocation.prompt

    if (invocation.context && Object.keys(invocation.context).length > 0) {
      prompt += "\n\n## Context\n"
      prompt += "```json\n"
      prompt += JSON.stringify(invocation.context, null, 2)
      prompt += "\n```"
    }

    return prompt
  }

  private extractFindings(output: string): AgentFinding[] {
    const findings: AgentFinding[] = []

    // Look for markdown findings patterns
    const findingPatterns = [
      /(?:#{1,3}\s*)?(?:Finding|Issue|Warning|Error|Critical)[:\s]+(.+?)(?=\n#{1,3}|\n\n|\z)/gis,
      /(?:^|\n)[-*]\s*\*\*(\w+)\*\*[:\s]+(.+?)(?=\n[-*]|\n\n|\z)/gis,
    ]

    for (const pattern of findingPatterns) {
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

    // Look for recommendation patterns
    const patterns = [
      /(?:#{1,3}\s*)?(?:Recommend(?:ation)?s?)[:\s]+(.+?)(?=\n#{1,3}|\n\n|\z)/gis,
      /(?:^|\n)[-*]\s*(?:should|consider|recommend)[:\s]+(.+?)(?=\n[-*]|\n\n|\z)/gis,
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

  private inferSeverity(text: string): AgentFinding["severity"] {
    const lower = text.toLowerCase()
    if (lower.includes("critical") || lower.includes("urgent")) return "critical"
    if (lower.includes("error") || lower.includes("fail")) return "error"
    if (lower.includes("warning") || lower.includes("caution")) return "warning"
    return "info"
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let clientInstance: AgentClient | null = null

/**
 * Get or create the agent client instance.
 */
export function getAgentClient(config?: Partial<AgentClientConfig>): AgentClient {
  if (!clientInstance) {
    clientInstance = new AgentClient(config)
  }
  return clientInstance
}

/**
 * Reset the agent client instance.
 */
export function resetAgentClient(): void {
  clientInstance = null
}

/**
 * Create a new agent client instance.
 */
export function createAgentClient(config?: Partial<AgentClientConfig>): AgentClient {
  return new AgentClient(config)
}
