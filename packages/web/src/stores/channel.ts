/**
 * Channel Store
 *
 * Manages messaging channel state including:
 * - All configured channels from ZeroBot config
 * - Channel health status
 * - Channel enable/disable operations
 */

import { create } from "zustand"
import { useShallow } from "zustand/react/shallow"
import { api } from "@/lib/api"
import type { ChannelStatus, ChannelType, ChannelHealth } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface ChannelState {
  channels: ChannelStatus[]
  isLoading: boolean
  isToggling: string | null
  error: string | null
  zeroBotRunning: boolean
}

interface ChannelActions {
  fetchChannels: () => Promise<void>
  addChannel: (type: ChannelType, name: string, config: Record<string, unknown>) => Promise<void>
  removeChannel: (name: string) => Promise<void>
  toggleChannel: (name: string) => Promise<void>
  checkHealth: (name: string) => Promise<void>
  getChannelsByHealth: (health: ChannelHealth) => ChannelStatus[]
  getEnabledChannels: () => ChannelStatus[]
}

type ChannelStore = ChannelState & ChannelActions

// ============================================================================
// Store
// ============================================================================

export const useChannelStore = create<ChannelStore>((set, get) => ({
  // Initial State
  channels: [],
  isLoading: false,
  isToggling: null,
  error: null,
  zeroBotRunning: false,

  // Actions
  fetchChannels: async () => {
    set({ isLoading: true, error: null })
    try {
      const channels = await api.listChannels()
      set({
        channels,
        isLoading: false,
        zeroBotRunning: channels.some((c) => c.health === "healthy"),
      })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch channels",
        isLoading: false,
      })
    }
  },

  addChannel: async (_type, _name, _config) => {
    set({ isLoading: true, error: null })
    try {
      // Note: Adding channels requires editing config.toml
      // For now, we just refresh the channel list
      await get().fetchChannels()
      set({ isLoading: false })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to add channel",
        isLoading: false,
      })
    }
  },

  removeChannel: async (_name) => {
    set({ isLoading: true, error: null })
    try {
      // Note: Removing channels requires editing config.toml
      // For now, we just refresh the channel list
      await get().fetchChannels()
      set({ isLoading: false })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to remove channel",
        isLoading: false,
      })
    }
  },

  toggleChannel: async (name) => {
    set({ isToggling: name, error: null })
    try {
      // Note: Toggle is informational only - actual enable/disable
      // requires editing config.toml and restarting ZeroBot
      // For now, we just check health
      const updatedChannel = await api.checkChannelHealth(name)
      set((state) => ({
        channels: state.channels.map((c) => (c.name === name ? updatedChannel : c)),
        isToggling: null,
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to toggle channel",
        isToggling: null,
      })
    }
  },

  checkHealth: async (name) => {
    try {
      const updatedChannel = await api.checkChannelHealth(name)
      set((state) => ({
        channels: state.channels.map((c) => (c.name === name ? updatedChannel : c)),
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to check health",
      })
    }
  },

  getChannelsByHealth: (health) => {
    return get().channels.filter((c) => c.health === health)
  },

  getEnabledChannels: () => {
    return get().channels.filter((c) => c.enabled)
  },
}))

// ============================================================================
// Hooks
// ============================================================================

export const useChannels = () => useChannelStore(useShallow((state) => state.channels))
export const useChannelLoading = () =>
  useChannelStore(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isToggling: state.isToggling,
      error: state.error,
    }))
  )
export const useEnabledChannels = () => useChannelStore((state) => state.getEnabledChannels())
export const useZeroBotStatus = () => useChannelStore((state) => state.zeroBotRunning)
export const useChannelCounts = () =>
  useChannelStore(
    useShallow((state) => ({
      total: state.channels.length,
      enabled: state.channels.filter((c) => c.enabled).length,
      healthy: state.channels.filter((c) => c.health === "healthy").length,
    }))
  )
