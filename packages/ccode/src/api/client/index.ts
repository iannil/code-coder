/**
 * Zero API Client - TypeScript client for the Rust zero-api service
 *
 * This module provides a type-safe client for communicating with the zero-api
 * Rust backend, which exposes zero-core functionality via REST and WebSocket.
 */

import z from "zod"

/**
 * Client configuration
 */
export const ZeroClientConfig = z.object({
  /** Base URL for the zero-api service */
  baseUrl: z.string().default("http://localhost:4402"),
  /** Request timeout in milliseconds */
  timeout: z.number().default(30000),
  /** WebSocket URL (defaults to ws://localhost:4402/ws) */
  wsUrl: z.string().optional(),
})

export type ZeroClientConfig = z.infer<typeof ZeroClientConfig>

/**
 * API response wrapper
 */
export interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

/**
 * Base client for zero-api communication
 */
export class ZeroClient {
  private config: Required<ZeroClientConfig>
  private abortController: AbortController | null = null

  constructor(config: Partial<ZeroClientConfig> = {}) {
    const parsed = ZeroClientConfig.parse(config)
    this.config = {
      baseUrl: parsed.baseUrl,
      timeout: parsed.timeout,
      wsUrl: parsed.wsUrl ?? `ws://localhost:4402/ws`,
    }
  }

  /**
   * Get the base URL
   */
  get baseUrl(): string {
    return this.config.baseUrl
  }

  /**
   * Get the WebSocket URL
   */
  get wsUrl(): string {
    return this.config.wsUrl
  }

  /**
   * Make a GET request
   */
  async get<T>(path: string): Promise<T> {
    return this.request<T>("GET", path)
  }

  /**
   * Make a POST request
   */
  async post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("POST", path, body)
  }

  /**
   * Make a DELETE request
   */
  async delete<T>(path: string): Promise<T> {
    return this.request<T>("DELETE", path)
  }

  /**
   * Make an HTTP request to zero-api
   */
  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    this.abortController = new AbortController()
    const timeoutId = setTimeout(() => this.abortController?.abort(), this.config.timeout)

    try {
      const url = `${this.config.baseUrl}${path}`
      const headers: Record<string, string> = {
        "Content-Type": "application/json",
      }

      const response = await fetch(url, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: this.abortController.signal,
      })

      clearTimeout(timeoutId)

      if (!response.ok) {
        const errorText = await response.text().catch(() => "Unknown error")
        throw new Error(`HTTP ${response.status}: ${errorText}`)
      }

      return (await response.json()) as T
    } catch (error) {
      clearTimeout(timeoutId)
      if (error instanceof Error && error.name === "AbortError") {
        throw new Error(`Request timeout after ${this.config.timeout}ms`)
      }
      throw error
    } finally {
      this.abortController = null
    }
  }

  /**
   * Check if zero-api service is healthy
   */
  async health(): Promise<{ status: string }> {
    return this.get<{ status: string }>("/health")
  }

  /**
   * Cancel any pending requests
   */
  cancel(): void {
    this.abortController?.abort()
  }
}

/**
 * Singleton client instance
 */
let defaultClient: ZeroClient | null = null

/**
 * Get or create the default client instance
 */
export function getClient(config?: Partial<ZeroClientConfig>): ZeroClient {
  if (!defaultClient || config) {
    defaultClient = new ZeroClient(config)
  }
  return defaultClient
}

/**
 * Reset the default client (useful for testing)
 */
export function resetClient(): void {
  defaultClient = null
}

// Re-export sub-modules
export * from "./tools"
export * from "./session"
export * from "./mcp"
export * from "./ws"
