/**
 * Observer API Client
 *
 * Lightweight TypeScript client for the Rust Observer Network API.
 * Replaces the local TypeScript implementation with HTTP calls to zero-cli daemon.
 *
 * @module observer/client
 */

import type {
  OperatingMode,
  GearPreset,
  Observation,
  WorldModel,
  Opportunity,
  WatcherStatus,
} from "./types"
import type { DialValues } from "./dial"
import type { CLOSEEvaluation } from "./controller/close-evaluator"
import type { Escalation, HumanDecision } from "./controller/escalation"
import type { ConsensusSnapshot } from "./consensus"

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

export interface ObserverClientConfig {
  /** Base URL for the Observer API (default: http://127.0.0.1:4402) */
  baseUrl?: string
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number
  /** Auth token if required */
  authToken?: string
}

const DEFAULT_CONFIG: Required<ObserverClientConfig> = {
  baseUrl: "http://127.0.0.1:4402",
  timeout: 30000,
  authToken: "",
}

// ─────────────────────────────────────────────────────────────────────────────
// Response Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export interface ObserverStatus {
  running: boolean
  enabled: boolean
  streamStats: {
    received: number
    processed: number
    dropped: number
    bufferSize: number
  }
  consensusConfidence: number
  activePatterns: number
  activeAnomalies: number
  activeOpportunities: number
  hasWorldModel: boolean
}

export interface GearStatus {
  currentGear: GearPreset
  dials: DialValues
  autoSwitch: boolean
}

export interface ApiGearPresetDetail {
  gear: GearPreset
  name: string
  description: string
  dials: DialValues
}

// ─────────────────────────────────────────────────────────────────────────────
// Event Source Types (SSE)
// ─────────────────────────────────────────────────────────────────────────────

export type ApiObserverEventType =
  | "started"
  | "stopped"
  | "observation_received"
  | "consensus_updated"
  | "gear_switch_recommended"
  | "world_model_updated"

export interface ApiObserverEvent {
  type: ApiObserverEventType
  data: unknown
  timestamp: Date
}

// ─────────────────────────────────────────────────────────────────────────────
// Observer API Client
// ─────────────────────────────────────────────────────────────────────────────

export class ObserverApiClient {
  private config: Required<ObserverClientConfig>

  constructor(config: ObserverClientConfig = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Core HTTP Methods
  // ─────────────────────────────────────────────────────────────────────────

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<ApiResponse<T>> {
    const url = `${this.config.baseUrl}${path}`
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    }

    if (this.config.authToken) {
      headers["Authorization"] = `Bearer ${this.config.authToken}`
    }

    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout)

    try {
      const response = await fetch(url, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      })

      const data = await response.json()
      return data as ApiResponse<T>
    } catch (error) {
      if (error instanceof Error && error.name === "AbortError") {
        return { success: false, error: "Request timeout" }
      }
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      }
    } finally {
      clearTimeout(timeoutId)
    }
  }

  private async get<T>(path: string): Promise<ApiResponse<T>> {
    return this.request<T>("GET", path)
  }

  private async post<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
    return this.request<T>("POST", path, body)
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Observer Network Control
  // ─────────────────────────────────────────────────────────────────────────

  /** Start the Observer Network */
  async start(): Promise<ApiResponse<ObserverStatus>> {
    return this.post<ObserverStatus>("/api/v1/observer/start")
  }

  /** Stop the Observer Network */
  async stop(): Promise<ApiResponse<ObserverStatus>> {
    return this.post<ObserverStatus>("/api/v1/observer/stop")
  }

  /** Get Observer Network status */
  async getStatus(): Promise<ApiResponse<ObserverStatus>> {
    return this.get<ObserverStatus>("/api/v1/observer/status")
  }

  // ─────────────────────────────────────────────────────────────────────────
  // World Model & Consensus
  // ─────────────────────────────────────────────────────────────────────────

  /** Get current world model */
  async getWorldModel(): Promise<ApiResponse<WorldModel | null>> {
    return this.get<WorldModel | null>("/api/v1/observer/world-model")
  }

  /** Get consensus snapshot */
  async getConsensus(): Promise<ApiResponse<ConsensusSnapshot>> {
    return this.get<ConsensusSnapshot>("/api/v1/observer/consensus")
  }

  /** Get active patterns */
  async getPatterns(): Promise<ApiResponse<unknown[]>> {
    return this.get<unknown[]>("/api/v1/observer/patterns")
  }

  /** Get active anomalies */
  async getAnomalies(): Promise<ApiResponse<unknown[]>> {
    return this.get<unknown[]>("/api/v1/observer/anomalies")
  }

  /** Get active opportunities */
  async getOpportunities(): Promise<ApiResponse<Opportunity[]>> {
    return this.get<Opportunity[]>("/api/v1/observer/opportunities")
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Observation Ingestion
  // ─────────────────────────────────────────────────────────────────────────

  /** Ingest observations into the network */
  async ingest(observations: Observation[]): Promise<ApiResponse<{ ingested: number }>> {
    return this.post<{ ingested: number }>("/api/v1/observer/ingest", { observations })
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Gear Control
  // ─────────────────────────────────────────────────────────────────────────

  /** Get current gear status */
  async getGear(): Promise<ApiResponse<GearStatus>> {
    return this.get<GearStatus>("/api/v1/gear/current")
  }

  /** Switch gear preset */
  async switchGear(gear: GearPreset, reason?: string): Promise<ApiResponse<GearStatus>> {
    return this.post<GearStatus>("/api/v1/gear/switch", { gear, reason })
  }

  /** Set dial values */
  async setDials(dials: DialValues): Promise<ApiResponse<GearStatus>> {
    return this.post<GearStatus>("/api/v1/gear/dials", dials)
  }

  /** Set a single dial */
  async setDial(
    dial: "observe" | "decide" | "act",
    value: number,
  ): Promise<ApiResponse<GearStatus>> {
    return this.post<GearStatus>("/api/v1/gear/dial", { dial, value })
  }

  /** Get all gear presets */
  async getPresets(): Promise<ApiResponse<ApiGearPresetDetail[]>> {
    return this.get<ApiGearPresetDetail[]>("/api/v1/gear/presets")
  }

  /** Get CLOSE evaluation */
  async getCLOSE(): Promise<ApiResponse<CLOSEEvaluation>> {
    return this.get<CLOSEEvaluation>("/api/v1/gear/close")
  }

  /** Run CLOSE evaluation */
  async runCLOSE(): Promise<ApiResponse<CLOSEEvaluation>> {
    return this.post<CLOSEEvaluation>("/api/v1/gear/close")
  }

  /** Enable/disable auto gear switch */
  async setAutoSwitch(enabled: boolean): Promise<ApiResponse<GearStatus>> {
    return this.post<GearStatus>("/api/v1/gear/auto-switch", { enabled })
  }

  // ─────────────────────────────────────────────────────────────────────────
  // SSE Event Stream
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Subscribe to real-time events via Server-Sent Events.
   * Returns an EventSource that can be used to listen for events.
   */
  subscribeToEvents(
    onEvent: (event: ApiObserverEvent) => void,
    onError?: (error: Event) => void,
  ): EventSource {
    const url = `${this.config.baseUrl}/api/v1/observer/events`
    const eventSource = new EventSource(url)

    // Handle different event types
    const eventTypes: ApiObserverEventType[] = [
      "started",
      "stopped",
      "observation_received",
      "consensus_updated",
      "gear_switch_recommended",
      "world_model_updated",
    ]

    for (const eventType of eventTypes) {
      eventSource.addEventListener(eventType, (e) => {
        const messageEvent = e as MessageEvent
        try {
          const data = JSON.parse(messageEvent.data)
          onEvent({
            type: eventType,
            data,
            timestamp: new Date(),
          })
        } catch {
          onEvent({
            type: eventType,
            data: messageEvent.data,
            timestamp: new Date(),
          })
        }
      })
    }

    if (onError) {
      eventSource.onerror = onError
    }

    return eventSource
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let clientInstance: ObserverApiClient | null = null

/**
 * Get or create the Observer API client singleton.
 */
export function getObserverClient(config?: ObserverClientConfig): ObserverApiClient {
  if (!clientInstance || config) {
    clientInstance = new ObserverApiClient(config)
  }
  return clientInstance
}

/**
 * Reset the client singleton (useful for testing).
 */
export function resetObserverClient(): void {
  clientInstance = null
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Check if Observer Network is running.
 */
export async function isObserverRunning(): Promise<boolean> {
  const client = getObserverClient()
  const response = await client.getStatus()
  return response.success && response.data?.running === true
}

/**
 * Get current gear preset.
 */
export async function getCurrentGear(): Promise<GearPreset | null> {
  const client = getObserverClient()
  const response = await client.getGear()
  return response.success ? response.data?.currentGear ?? null : null
}

/**
 * Get current dial values.
 */
export async function getCurrentDials(): Promise<DialValues | null> {
  const client = getObserverClient()
  const response = await client.getGear()
  return response.success ? response.data?.dials ?? null : null
}
