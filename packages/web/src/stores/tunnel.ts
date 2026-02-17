/**
 * Tunnel Store
 *
 * Manages tunnel/proxy state including:
 * - Tunnel connection status
 * - Public URL management
 * - Connect/disconnect operations
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
}

interface TunnelActions {
  fetchStatus: () => Promise<void>
  connect: (type: TunnelType) => Promise<void>
  disconnect: () => Promise<void>
}

type TunnelStore = TunnelState & TunnelActions

// ============================================================================
// Mock Data (replace with actual API calls when backend is ready)
// ============================================================================

const MOCK_STATUS: TunnelStatus = {
  type: "none",
  connected: false,
}

const AVAILABLE_TYPES: TunnelType[] = ["cloudflare", "ngrok", "tailscale", "custom"]

// ============================================================================
// Store
// ============================================================================

export const useTunnelStore = create<TunnelStore>((set) => ({
  // Initial State
  status: null,
  availableTypes: AVAILABLE_TYPES,
  isLoading: false,
  isConnecting: false,
  isDisconnecting: false,
  error: null,

  // Actions
  fetchStatus: async () => {
    set({ isLoading: true, error: null })
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 500))
      set({ status: MOCK_STATUS, isLoading: false })
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
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 2000))

      const publicUrls: Record<TunnelType, string> = {
        cloudflare: "https://random-words.trycloudflare.com",
        ngrok: "https://abc123.ngrok.io",
        tailscale: "https://machine.tailnet-name.ts.net",
        custom: "https://custom.example.com",
        none: "",
      }

      set({
        status: {
          type,
          connected: true,
          publicUrl: publicUrls[type],
          localUrl: "http://127.0.0.1:4096",
          latency: Math.floor(Math.random() * 50) + 10,
          startedAt: Date.now(),
        },
        isConnecting: false,
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
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 500))
      set({
        status: {
          type: "none",
          connected: false,
        },
        isDisconnecting: false,
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
export const useTunnelLoading = () =>
  useTunnelStore(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isConnecting: state.isConnecting,
      isDisconnecting: state.isDisconnecting,
      error: state.error,
    }))
  )
