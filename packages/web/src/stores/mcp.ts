/**
 * MCP Store
 *
 * Manages MCP (Model Context Protocol) server state including:
 * - Server statuses
 * - Available tools
 * - Available resources
 */

import { create } from "zustand"
import { api } from "@/lib/api"
import type { McpStatus, McpTool, McpResource, McpAuthStatus } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface McpState {
  // Data
  status: Record<string, McpStatus>
  tools: McpTool[]
  resources: Record<string, McpResource>

  // Loading State
  isLoading: boolean
  isToggling: string | null // Name of MCP being toggled
  error: string | null
}

interface McpActions {
  // Data Loading
  fetchStatus: () => Promise<void>
  fetchTools: () => Promise<void>
  fetchResources: () => Promise<void>
  fetchAll: () => Promise<void>

  // MCP Operations
  toggle: (name: string) => Promise<void>
  connect: (name: string) => Promise<void>
  disconnect: (name: string) => Promise<void>

  // Auth Operations
  getAuthStatus: (name: string) => Promise<McpAuthStatus>
  startAuth: (name: string) => Promise<{ authorizationUrl: string }>
  finishAuth: (name: string, code: string) => Promise<void>

  // Helpers
  isEnabled: (name: string) => boolean
  getStatusLabel: (name: string) => string
}

type McpStore = McpState & McpActions

// ============================================================================
// Store
// ============================================================================

export const useMcpStore = create<McpStore>((set, get) => ({
  // Initial State
  status: {},
  tools: [],
  resources: {},
  isLoading: false,
  isToggling: null,
  error: null,

  // Actions
  fetchStatus: async () => {
    try {
      const status = await api.getMcpStatus()
      set({ status })
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to fetch MCP status" })
    }
  },

  fetchTools: async () => {
    try {
      const tools = await api.getMcpTools()
      set({ tools })
    } catch (error) {
      console.error("Failed to fetch MCP tools:", error)
    }
  },

  fetchResources: async () => {
    try {
      const resources = await api.getMcpResources()
      set({ resources })
    } catch (error) {
      console.error("Failed to fetch MCP resources:", error)
    }
  },

  fetchAll: async () => {
    set({ isLoading: true, error: null })
    try {
      await Promise.all([get().fetchStatus(), get().fetchTools(), get().fetchResources()])
      set({ isLoading: false })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch MCP data",
        isLoading: false,
      })
    }
  },

  toggle: async (name) => {
    set({ isToggling: name })
    try {
      const result = await api.toggleMcp(name)
      set((state) => ({
        status: { ...state.status, [name]: result.status },
        isToggling: null,
      }))
      // Refresh tools if we connected
      if (result.status.status === "connected") {
        await get().fetchTools()
      }
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : `Failed to toggle ${name}`,
        isToggling: null,
      })
    }
  },

  connect: async (name) => {
    set({ isToggling: name })
    try {
      const result = await api.connectMcp(name)
      set((state) => ({
        status: { ...state.status, [name]: result.status },
        isToggling: null,
      }))
      // Refresh tools after connecting
      await get().fetchTools()
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : `Failed to connect ${name}`,
        isToggling: null,
      })
    }
  },

  disconnect: async (name) => {
    set({ isToggling: name })
    try {
      const result = await api.disconnectMcp(name)
      set((state) => ({
        status: { ...state.status, [name]: result.status },
        isToggling: null,
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : `Failed to disconnect ${name}`,
        isToggling: null,
      })
    }
  },

  getAuthStatus: async (name) => {
    return api.getMcpAuthStatus(name)
  },

  startAuth: async (name) => {
    return api.startMcpAuth(name)
  },

  finishAuth: async (name, code) => {
    const result = await api.finishMcpAuth(name, code)
    set((state) => ({
      status: { ...state.status, [name]: result.status },
    }))
  },

  isEnabled: (name) => {
    const status = get().status[name]
    return status?.status === "connected"
  },

  getStatusLabel: (name) => {
    const status = get().status[name]
    if (!status) return "Unknown"

    switch (status.status) {
      case "connected":
        return "Connected"
      case "disabled":
        return "Disabled"
      case "failed":
        return `Failed: ${status.error}`
      case "needs_auth":
        return "Needs Authentication"
      case "needs_client_registration":
        return "Needs Client Registration"
      default:
        return "Unknown"
    }
  },
}))

// ============================================================================
// Hooks
// ============================================================================

export const useMcpStatus = () => useMcpStore((state) => state.status)
export const useMcpTools = () => useMcpStore((state) => state.tools)
export const useMcpResources = () => useMcpStore((state) => state.resources)

export const useMcpLoading = () =>
  useMcpStore((state) => ({
    isLoading: state.isLoading,
    isToggling: state.isToggling,
    error: state.error,
  }))
