/**
 * Gateway Store
 *
 * Manages HTTP gateway state including:
 * - Gateway running status
 * - Endpoint information
 * - Recent request history
 *
 * Note: The HTTP Gateway is a Rust service (zero-gateway) managed by zero-cli daemon.
 * Start/stop operations require the CLI: `zero start` / `zero stop`
 * The TypeScript API provides status monitoring only.
 */

import { create } from "zustand"
import { useShallow } from "zustand/react/shallow"
import type { GatewayStatus } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface GatewayState {
  status: GatewayStatus | null
  isLoading: boolean
  isStarting: boolean
  isStopping: boolean
  error: string | null
  serviceUnavailable: boolean
}

interface GatewayActions {
  fetchStatus: () => Promise<void>
  start: () => Promise<void>
  stop: () => Promise<void>
  fetchRequests: () => Promise<void>
}

type GatewayStore = GatewayState & GatewayActions

// ============================================================================
// Constants
// ============================================================================

const GATEWAY_HOST = "127.0.0.1"
const GATEWAY_PORT = 4430

const DEFAULT_ENDPOINTS = [
  { path: "/webhook/telegram", method: "POST", description: "Telegram webhook endpoint" },
  { path: "/webhook/discord", method: "POST", description: "Discord webhook endpoint" },
  { path: "/webhook/slack", method: "POST", description: "Slack webhook endpoint" },
  { path: "/webhook/feishu", method: "POST", description: "Feishu webhook endpoint" },
  { path: "/health", method: "GET", description: "Health check endpoint" },
  { path: "/mcp", method: "POST", description: "MCP JSON-RPC endpoint" },
]

// ============================================================================
// Store
// ============================================================================

export const useGatewayStore = create<GatewayStore>((set) => ({
  // Initial State
  status: null,
  isLoading: false,
  isStarting: false,
  isStopping: false,
  error: null,
  serviceUnavailable: false,

  // Actions
  fetchStatus: async () => {
    set({ isLoading: true, error: null, serviceUnavailable: false })
    try {
      // Try to reach the zero-gateway health endpoint directly
      const controller = new AbortController()
      const timeout = setTimeout(() => controller.abort(), 3000)

      try {
        const response = await fetch(`http://${GATEWAY_HOST}:${GATEWAY_PORT}/health`, {
          signal: controller.signal,
        })
        clearTimeout(timeout)

        if (response.ok) {
          // Gateway is running
          set({
            status: {
              running: true,
              host: GATEWAY_HOST,
              port: GATEWAY_PORT,
              endpoints: DEFAULT_ENDPOINTS,
              requestCount: 0, // Would need metrics endpoint
              recentRequests: [], // Would need logging endpoint
            },
            isLoading: false,
          })
        } else {
          // Gateway returned error
          set({
            status: {
              running: false,
              host: GATEWAY_HOST,
              port: GATEWAY_PORT,
              endpoints: DEFAULT_ENDPOINTS,
              requestCount: 0,
              recentRequests: [],
            },
            error: `Gateway returned status ${response.status}`,
            isLoading: false,
          })
        }
      } catch (fetchError) {
        clearTimeout(timeout)

        // Gateway is not reachable - it's not running
        set({
          status: {
            running: false,
            host: GATEWAY_HOST,
            port: GATEWAY_PORT,
            endpoints: DEFAULT_ENDPOINTS,
            requestCount: 0,
            recentRequests: [],
          },
          error: "Gateway is not running. Use 'zero start' to start the daemon.",
          isLoading: false,
          serviceUnavailable: true,
        })
      }
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch gateway status",
        isLoading: false,
        serviceUnavailable: true,
      })
    }
  },

  start: async () => {
    set({ isStarting: true, error: null })

    // Gateway start/stop is managed by zero-cli daemon, not by the TypeScript API
    // Show informative message about how to start the gateway
    await new Promise((resolve) => setTimeout(resolve, 500))

    set({
      isStarting: false,
      error: "Gateway control requires the CLI. Run 'zero daemon start' to start the gateway service.",
    })

    // Trigger a status refresh after showing the message
    const fetchStatus = useGatewayStore.getState().fetchStatus
    setTimeout(() => fetchStatus(), 2000)
  },

  stop: async () => {
    set({ isStopping: true, error: null })

    // Gateway start/stop is managed by zero-cli daemon, not by the TypeScript API
    await new Promise((resolve) => setTimeout(resolve, 500))

    set({
      isStopping: false,
      error: "Gateway control requires the CLI. Run 'zero daemon stop' to stop the gateway service.",
    })

    // Trigger a status refresh after showing the message
    const fetchStatus = useGatewayStore.getState().fetchStatus
    setTimeout(() => fetchStatus(), 2000)
  },

  fetchRequests: async () => {
    // Recent requests would require a logging/metrics endpoint on the gateway
    // This is not currently implemented in zero-gateway
    set({
      error: "Request history requires the gateway metrics endpoint (not yet implemented)",
    })
  },
}))

// ============================================================================
// Hooks
// ============================================================================

export const useGatewayStatus = () => useGatewayStore((state) => state.status)
export const useGatewayRunning = () => useGatewayStore((state) => state.status?.running ?? false)
export const useGatewayEndpoints = () => useGatewayStore((state) => state.status?.endpoints ?? [])
export const useGatewayRequests = () => useGatewayStore((state) => state.status?.recentRequests ?? [])
export const useGatewayLoading = () =>
  useGatewayStore(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isStarting: state.isStarting,
      isStopping: state.isStopping,
      error: state.error,
      serviceUnavailable: state.serviceUnavailable,
    }))
  )
