/**
 * SSE Store
 * Manages Server-Sent Events connection state and integrates with SSE client
 */

import { create } from "zustand"
import { immer } from "zustand/middleware/immer"
import { useShallow } from "zustand/react/shallow"
import type { SSEClient } from "../lib/sse"
import type { SSEEventType, SSEDataEvent } from "../lib/types"

// ============================================================================
// State Interface
// ============================================================================

type ConnectionState = "disconnected" | "connecting" | "connected" | "reconnecting" | "error"

interface SSEState {
  // Connection state
  connectionState: ConnectionState
  lastConnected: number | null
  reconnectAttempts: number

  // Channels
  subscribedChannels: Set<SSEEventType>

  // Events buffer (for recent events)
  events: SSEDataEvent[]
  maxEventBufferSize: number

  // Error state
  error: string | null
}

interface SSEActions {
  // Connection management
  connect: (clientFactory: () => SSEClient) => void
  disconnect: () => void
  reconnect: () => void

  // Channel management
  subscribeChannels: (channels: SSEEventType[]) => void
  unsubscribeChannels: (channels: SSEEventType[]) => void

  // Event handling
  addEvent: (event: SSEDataEvent) => void
  clearEvents: () => void

  // State management
  setError: (error: string | null) => void
  clearError: () => void
  reset: () => void
}

type SSEStore = SSEState & SSEActions

// ============================================================================
// SSE Client Reference
// ============================================================================

let sseClient: SSEClient | null = null

// ============================================================================
// Initial State
// ============================================================================

const initialState: Omit<SSEState, "subscribedChannels" | "events" | "maxEventBufferSize"> = {
  connectionState: "disconnected",
  lastConnected: null,
  reconnectAttempts: 0,
  error: null,
}

// ============================================================================
// Store Creation
// ============================================================================

const useSSEStoreBase = create<SSEStore>()(
  immer((set) => ({
    // Initial state
    subscribedChannels: new Set(),
    events: [],
    maxEventBufferSize: 100,
    ...initialState,

    // ======================================================================
    // Connection Management
    // ======================================================================

    /**
     * Connect to SSE endpoint using the provided client factory
     */
    connect: (clientFactory) => {
      // Disconnect existing client if any
      if (sseClient) {
        sseClient.disconnect()
      }

      set((state) => {
        state.connectionState = "connecting"
        state.error = null
      })

      // Create and connect client
      // Note: The client factory should create a client with proper handlers
      sseClient = clientFactory()
      sseClient.connect()
    },

    /**
     * Disconnect from SSE endpoint
     */
    disconnect: () => {
      if (sseClient) {
        sseClient.disconnect()
        sseClient = null
      }

      set((state) => {
        state.connectionState = "disconnected"
        state.lastConnected = null
        state.reconnectAttempts = 0
      })
    },

    /**
     * Reconnect to SSE endpoint
     */
    reconnect: () => {
      set((state) => {
        state.connectionState = "reconnecting"
        state.reconnectAttempts++
      })

      if (sseClient) {
        sseClient.disconnect()
        sseClient.connect()
      }
    },

    // ======================================================================
    // Channel Management
    // ======================================================================

    /**
     * Subscribe to additional channels
     */
    subscribeChannels: (channels) => {
      set((state) => {
        for (const channel of channels) {
          state.subscribedChannels.add(channel)
        }
      })

      // Update client if connected
      if (sseClient) {
        sseClient.subscribeChannels(channels)
      }
    },

    /**
     * Unsubscribe from channels
     */
    unsubscribeChannels: (channels) => {
      set((state) => {
        for (const channel of channels) {
          state.subscribedChannels.delete(channel)
        }
      })

      // Update client if connected
      if (sseClient) {
        sseClient.unsubscribeChannels(channels)
      }
    },

    // ======================================================================
    // Event Handling
    // ======================================================================

    /**
     * Add an event to the buffer
     */
    addEvent: (event) => {
      set((state) => {
        state.events.push(event)

        // Keep buffer size under limit
        if (state.events.length > state.maxEventBufferSize) {
          state.events.shift()
        }
      })
    },

    /**
     * Clear all events from buffer
     */
    clearEvents: () => {
      set((state) => {
        state.events = []
      })
    },

    // ======================================================================
    // State Management
    // ======================================================================

    setError: (error) => {
      set((state) => {
        state.error = error
      })
    },

    clearError: () => {
      set((state) => {
        state.error = null
      })
    },

    reset: () => {
      // Access store through a different method since get is not available
      const currentState = useSSEStoreBase.getState()
      currentState.disconnect()

      set((state) => {
        state.subscribedChannels.clear()
        state.events = []
        Object.assign(state, initialState)
      })
    },
  })),
)

// ============================================================================
// Selector Hooks
// ============================================================================

/**
 * Get the current connection state
 */
export const useSSEConnectionState = () =>
  useSSEStoreBase((state) => state.connectionState)

/**
 * Check if connected to SSE
 */
export const useSSEConnected = () =>
  useSSEStoreBase((state) => state.connectionState === "connected")

/**
 * Check if connecting to SSE
 */
export const useSSEConnecting = () =>
  useSSEStoreBase((state) =>
    state.connectionState === "connecting" || state.connectionState === "reconnecting",
  )

/**
 * Get the subscribed channels
 * Uses shallow comparison to prevent unnecessary re-renders
 */
export const useSSESubscribedChannels = () =>
  useSSEStoreBase(useShallow((state) => Array.from(state.subscribedChannels)))

/**
 * Get recent events
 * Uses shallow comparison to prevent unnecessary re-renders
 */
export const useSSEEvents = () => useSSEStoreBase(useShallow((state) => state.events))

/**
 * Get error state
 */
export const useSSEError = () => useSSEStoreBase((state) => state.error)

/**
 * Get reconnection info
 */
export const useSSEReconnectInfo = () =>
  useSSEStoreBase((state) => ({
    attempts: state.reconnectAttempts,
    lastConnected: state.lastConnected,
  }))

// ============================================================================
// Export Store
// ============================================================================

export { useSSEStoreBase as useSSEStore }
