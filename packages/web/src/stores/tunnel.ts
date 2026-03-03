/**
 * Tunnel Store
 *
 * Manages tunnel/proxy state including:
 * - Tunnel connection status
 * - Public URL management
 * - Connect/disconnect operations
 *
 * Note: Tunnel management is planned for zero-gateway but not yet implemented.
 * The supported tunnel types are: Cloudflare, ngrok, Tailscale, custom.
 *
 * API Status: NOT IMPLEMENTED
 * Required endpoints (planned):
 * - GET /api/v1/tunnel/status - Get tunnel status
 * - POST /api/v1/tunnel/connect - Connect to a tunnel provider
 * - POST /api/v1/tunnel/disconnect - Disconnect from tunnel
 */

import { create } from "zustand"
import { useShallow } from "zustand/react/shallow"
import type { TunnelStatus, TunnelType } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface TunnelState {
  status: TunnelStatus | null
  availableTypes: TunnelType[]
  isLoading: boolean
  isConnecting: boolean
  isDisconnecting: boolean
  error: string | null
  apiImplemented: boolean
}

interface TunnelActions {
  fetchStatus: () => Promise<void>
  connect: (type: TunnelType) => Promise<void>
  disconnect: () => Promise<void>
}

type TunnelStore = TunnelState & TunnelActions

// ============================================================================
// Constants
// ============================================================================

const AVAILABLE_TYPES: TunnelType[] = ["cloudflare", "ngrok", "tailscale", "custom"]

const API_NOT_IMPLEMENTED_ERROR =
  "Tunnel management API is not yet implemented. This feature is planned for zero-gateway."

// ============================================================================
// Store
// ============================================================================

export const useTunnelStore = create<TunnelStore>((set) => ({
  // Initial State
  status: {
    type: "none",
    connected: false,
  },
  availableTypes: AVAILABLE_TYPES,
  isLoading: false,
  isConnecting: false,
  isDisconnecting: false,
  error: null,
  apiImplemented: false,

  // Actions
  fetchStatus: async () => {
    set({ isLoading: true, error: null })
    try {
      // Backend API not yet implemented. When ready:
      // - Add getTunnelStatus() method to ApiClient in api.ts
      // - Use TunnelStatus type from types.ts
      // - Call: const response = await api.getTunnelStatus()
      // - Endpoint: GET /api/v1/tunnel/status

      // For now, return disconnected state since API doesn't exist
      set({
        status: {
          type: "none",
          connected: false,
        },
        isLoading: false,
        error: API_NOT_IMPLEMENTED_ERROR,
        apiImplemented: false,
      })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch tunnel status",
        isLoading: false,
      })
    }
  },

  connect: async (type) => {
    set({ isConnecting: true, error: null })
    try {
      // Backend API not yet implemented. When ready:
      // - Add connectTunnel(type: TunnelType) method to ApiClient in api.ts
      // - Call: await api.connectTunnel({ type })
      // - Endpoint: POST /api/v1/tunnel/connect

      // For now, show error since API doesn't exist
      await new Promise((resolve) => setTimeout(resolve, 500))

      set({
        isConnecting: false,
        error: `${API_NOT_IMPLEMENTED_ERROR}\n\nTo connect a ${type} tunnel manually:\n` +
          (type === "cloudflare"
            ? "Run: cloudflared tunnel run --url http://localhost:4430"
            : type === "ngrok"
              ? "Run: ngrok http 4430"
              : type === "tailscale"
                ? "Run: tailscale funnel 4430"
                : "Configure your custom tunnel to forward to localhost:4430"),
      })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to connect tunnel",
        isConnecting: false,
      })
    }
  },

  disconnect: async () => {
    set({ isDisconnecting: true, error: null })
    try {
      // Backend API not yet implemented. When ready:
      // - Add disconnectTunnel() method to ApiClient in api.ts
      // - Call: await api.disconnectTunnel()
      // - Endpoint: POST /api/v1/tunnel/disconnect

      await new Promise((resolve) => setTimeout(resolve, 500))

      set({
        isDisconnecting: false,
        error: API_NOT_IMPLEMENTED_ERROR,
      })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to disconnect tunnel",
        isDisconnecting: false,
      })
    }
  },
}))

// ============================================================================
// Hooks
// ============================================================================

export const useTunnelStatus = () => useTunnelStore((state) => state.status)
export const useTunnelConnected = () => useTunnelStore((state) => state.status?.connected ?? false)
export const useTunnelPublicUrl = () => useTunnelStore((state) => state.status?.publicUrl)
export const useAvailableTunnelTypes = () => useTunnelStore((state) => state.availableTypes)
export const useTunnelApiImplemented = () => useTunnelStore((state) => state.apiImplemented)
export const useTunnelLoading = () =>
  useTunnelStore(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isConnecting: state.isConnecting,
      isDisconnecting: state.isDisconnecting,
      error: state.error,
    }))
  )
