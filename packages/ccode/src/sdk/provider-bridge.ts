/**
 * Provider Bridge SDK
 *
 * Provides a bridge between the TypeScript Provider module and the Rust daemon
 * for read-only operations. The Provider module cannot be fully migrated because
 * `getLanguage()` returns AI SDK objects that can't be serialized over HTTP.
 *
 * ## What This Bridge Handles (Read-Only)
 * - list() - List connected providers
 * - listAll() - List all providers with defaults
 * - getProvider(id) - Get provider info
 * - getModel(providerID, modelID) - Get model info
 * - defaultModel() - Get default model
 * - getSmallModel(providerID) - Get small model for a provider
 *
 * ## What Stays in TS Provider Module
 * - getLanguage() - Returns AI SDK LanguageModel objects
 * - getSDK() - Returns provider SDK instances
 * - All AI SDK integration code
 *
 * ## Architecture Decision
 *
 * The Provider module is fundamentally different from Agent:
 * - Agent: Configuration/registry (fully bridgeable)
 * - Provider: Runtime infrastructure + AI SDK integration (partially bridgeable)
 *
 * The TS Provider module remains the authoritative source for AI SDK integration.
 * This bridge provides read-only access to provider/model metadata.
 *
 * @see packages/ccode/src/provider/provider.ts for full implementation
 */

import { Config } from "@/config/config"

// ══════════════════════════════════════════════════════════════════════════════
// Types (Mirror of Provider.Model and Provider.Info)
// ══════════════════════════════════════════════════════════════════════════════

export interface ModelInfo {
  id: string
  providerID: string
  name: string
  family?: string
  api: {
    id: string
    url: string
    npm: string
  }
  capabilities: {
    temperature: boolean
    reasoning: boolean
    attachment: boolean
    toolcall: boolean
    input: {
      text: boolean
      audio: boolean
      image: boolean
      video: boolean
      pdf: boolean
    }
    output: {
      text: boolean
      audio: boolean
      image: boolean
      video: boolean
      pdf: boolean
    }
    interleaved: boolean | { field: "reasoning_content" | "reasoning_details" }
  }
  cost: {
    input: number
    output: number
    cache: {
      read: number
      write: number
    }
  }
  limit: {
    context: number
    input?: number
    output: number
  }
  status: "alpha" | "beta" | "deprecated" | "active"
  options: Record<string, unknown>
  headers: Record<string, string>
  release_date: string
  variants?: Record<string, Record<string, unknown>>
}

export interface ProviderInfo {
  id: string
  name: string
  source: "env" | "config" | "custom" | "api"
  env: string[]
  key?: string
  options: Record<string, unknown>
  models: Record<string, ModelInfo>
}

export interface ModelReference {
  providerID: string
  modelID: string
}

export interface ListAllResponse {
  all: ProviderInfo[]
  default: Record<string, string>
  connected: string[]
}

// ══════════════════════════════════════════════════════════════════════════════
// Provider Bridge Implementation
// ══════════════════════════════════════════════════════════════════════════════

export class ProviderBridge {
  private baseUrl: string

  constructor(baseUrl?: string) {
    this.baseUrl = baseUrl || "http://localhost:4402"
  }

  /**
   * Initialize bridge with config-based URL
   */
  static async create(): Promise<ProviderBridge> {
    const cfg = await Config.get()
    const port = (cfg.ports as Record<string, { port?: number }> | undefined)?.daemon?.port || 4402
    const baseUrl = `http://localhost:${port}`
    return new ProviderBridge(baseUrl)
  }

  /**
   * Check if the daemon is running
   */
  async isHealthy(): Promise<boolean> {
    try {
      const res = await fetch(`${this.baseUrl}/health`, {
        signal: AbortSignal.timeout(2000),
      })
      return res.ok
    } catch {
      return false
    }
  }

  /**
   * List connected providers
   * Compatible with: Provider.list()
   */
  async list(): Promise<Record<string, ProviderInfo>> {
    const res = await fetch(`${this.baseUrl}/api/v1/providers`)
    if (!res.ok) {
      throw new Error(`Failed to list providers: ${res.status} ${res.statusText}`)
    }
    const data = await res.json()
    return data.providers ?? {}
  }

  /**
   * List all providers (connected and unconnected)
   * Compatible with: Provider.listAll()
   */
  async listAll(): Promise<ListAllResponse> {
    const res = await fetch(`${this.baseUrl}/api/v1/providers/all`)
    if (!res.ok) {
      throw new Error(`Failed to list all providers: ${res.status} ${res.statusText}`)
    }
    return res.json()
  }

  /**
   * Get a specific provider by ID
   * Compatible with: Provider.getProvider(id)
   */
  async getProvider(providerID: string): Promise<ProviderInfo | undefined> {
    const res = await fetch(`${this.baseUrl}/api/v1/providers/${encodeURIComponent(providerID)}`)
    if (res.status === 404) {
      return undefined
    }
    if (!res.ok) {
      throw new Error(`Failed to get provider: ${res.status} ${res.statusText}`)
    }
    const data = await res.json()
    return data.provider
  }

  /**
   * Get a specific model
   * Compatible with: Provider.getModel(providerID, modelID)
   */
  async getModel(providerID: string, modelID: string): Promise<ModelInfo | undefined> {
    const res = await fetch(
      `${this.baseUrl}/api/v1/providers/${encodeURIComponent(providerID)}/models/${encodeURIComponent(modelID)}`
    )
    if (res.status === 404) {
      return undefined
    }
    if (!res.ok) {
      throw new Error(`Failed to get model: ${res.status} ${res.statusText}`)
    }
    const data = await res.json()
    return data.model
  }

  /**
   * Get the default model
   * Compatible with: Provider.defaultModel()
   */
  async defaultModel(): Promise<ModelReference> {
    const res = await fetch(`${this.baseUrl}/api/v1/providers/default-model`)
    if (!res.ok) {
      throw new Error(`Failed to get default model: ${res.status} ${res.statusText}`)
    }
    const data = await res.json()
    return {
      providerID: data.provider_id,
      modelID: data.model_id,
    }
  }

  /**
   * Get a small/fast model for a provider
   * Compatible with: Provider.getSmallModel(providerID)
   */
  async getSmallModel(providerID: string): Promise<ModelInfo | undefined> {
    const res = await fetch(`${this.baseUrl}/api/v1/providers/${encodeURIComponent(providerID)}/small-model`)
    if (res.status === 404) {
      return undefined
    }
    if (!res.ok) {
      throw new Error(`Failed to get small model: ${res.status} ${res.statusText}`)
    }
    const data = await res.json()
    return data.model
  }

  /**
   * Parse a model string into provider/model reference
   * This is a pure function that doesn't need the daemon.
   */
  parseModel(model: string): ModelReference {
    const [providerID, ...rest] = model.split("/")
    return {
      providerID,
      modelID: rest.join("/"),
    }
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// Singleton Instance
// ══════════════════════════════════════════════════════════════════════════════

let _instance: ProviderBridge | null = null

/**
 * Get the shared ProviderBridge instance
 */
export async function getProviderBridge(): Promise<ProviderBridge> {
  if (!_instance) {
    _instance = await ProviderBridge.create()
  }
  return _instance
}

/**
 * Reset the shared instance (for testing)
 */
export function resetProviderBridge(): void {
  _instance = null
}

// ══════════════════════════════════════════════════════════════════════════════
// Fallback to TS Provider
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Hybrid access pattern: Try bridge first, fall back to TS Provider
 *
 * This allows gradual migration - when the Rust API is ready, calls will
 * use the bridge. Otherwise, they fall back to the TS Provider module.
 */
export async function getProviderBridgeWithFallback(): Promise<{
  bridge: ProviderBridge
  useFallback: boolean
}> {
  const bridge = await getProviderBridge()
  const healthy = await bridge.isHealthy()

  return {
    bridge,
    useFallback: !healthy,
  }
}

/**
 * Helper to get default model with fallback
 */
export async function getDefaultModelWithFallback(): Promise<ModelReference> {
  const { bridge, useFallback } = await getProviderBridgeWithFallback()

  if (useFallback) {
    // Dynamically import to avoid circular deps
    const { Provider } = await import("../provider/provider")
    return Provider.defaultModel()
  }

  return bridge.defaultModel()
}

/**
 * Helper to get model with fallback
 */
export async function getModelWithFallback(
  providerID: string,
  modelID: string
): Promise<ModelInfo | undefined> {
  const { bridge, useFallback } = await getProviderBridgeWithFallback()

  if (useFallback) {
    const { Provider } = await import("../provider/provider")
    try {
      const model = await Provider.getModel(providerID, modelID)
      // Convert Provider.Model to ModelInfo
      return model as unknown as ModelInfo
    } catch {
      return undefined
    }
  }

  return bridge.getModel(providerID, modelID)
}
