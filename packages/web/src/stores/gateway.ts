/**
 * Gateway Store
 *
 * Manages HTTP gateway state including:
 * - Gateway running status
 * - Endpoint information
 * - Recent request history
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
}

interface GatewayActions {
  fetchStatus: () => Promise<void>
  start: () => Promise<void>
  stop: () => Promise<void>
  fetchRequests: () => Promise<void>
}

type GatewayStore = GatewayState & GatewayActions

// ============================================================================
// Mock Data (replace with actual API calls when backend is ready)
// ============================================================================

const MOCK_STATUS: GatewayStatus = {
  running: true,
  host: "127.0.0.1",
  port: 4402,
  uptime: 3600000,
  endpoints: [
    { path: "/webhook/telegram", method: "POST", description: "Telegram webhook endpoint" },
    { path: "/webhook/discord", method: "POST", description: "Discord webhook endpoint" },
    { path: "/webhook/slack", method: "POST", description: "Slack webhook endpoint" },
    { path: "/webhook/feishu", method: "POST", description: "Feishu webhook endpoint" },
    { path: "/health", method: "GET", description: "Health check endpoint" },
  ],
  requestCount: 142,
  recentRequests: [
    { id: "1", method: "POST", path: "/webhook/telegram", status: 200, timestamp: Date.now() - 60000, duration: 45 },
    { id: "2", method: "POST", path: "/webhook/discord", status: 200, timestamp: Date.now() - 120000, duration: 32 },
    { id: "3", method: "GET", path: "/health", status: 200, timestamp: Date.now() - 180000, duration: 5 },
  ],
}

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

  // Actions
  fetchStatus: async () => {
    set({ isLoading: true, error: null })
    try {
      // TODO: Replace with actual API call
      // const response = await api.getGatewayStatus()
      await new Promise((resolve) => setTimeout(resolve, 500))
      set({ status: MOCK_STATUS, isLoading: false })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch gateway status",
        isLoading: false,
      })
    }
  },

  start: async () => {
    set({ isStarting: true, error: null })
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 1000))
      set((state) => ({
        status: state.status
          ? { ...state.status, running: true, uptime: 0 }
          : {
              running: true,
              host: "127.0.0.1",
              port: 4402,
              uptime: 0,
              endpoints: [],
              requestCount: 0,
              recentRequests: [],
            },
        isStarting: false,
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to start gateway",
        isStarting: false,
      })
    }
  },

  stop: async () => {
    set({ isStopping: true, error: null })
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 500))
      set((state) => ({
        status: state.status ? { ...state.status, running: false, uptime: undefined } : null,
        isStopping: false,
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to stop gateway",
        isStopping: false,
      })
    }
  },

  fetchRequests: async () => {
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 200))
      // Would update recentRequests in status
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch requests",
      })
    }
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
    }))
  )
