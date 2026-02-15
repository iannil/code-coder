/**
 * useSSE Hook
 *
 * Manages Server-Sent Events connection with automatic lifecycle management.
 * Integrates with the SSE store for state management.
 */

import * as React from "react"
import { useShallow } from "zustand/react/shallow"
import { useSSEStore } from "../stores/sse"
import { SSEClient, type SSEClientConfig, type SSEEventHandlers } from "../lib/sse"
import type { SSEEventType, SSEDataEvent } from "../lib/types"

// ============================================================================
// Hook Options
// ============================================================================

export interface UseSSEOptions extends SSEClientConfig {
  /**
   * Auto-connect on mount
   * @default true
   */
  autoConnect?: boolean

  /**
   * Channels to subscribe to
   * @default ["message", "status", "error", "permission", "progress"]
   */
  channels?: SSEEventType[]

  /**
   * Event handlers for specific SSE events
   */
  onOpen?: () => void
  onError?: (error: Error) => void
  onClose?: () => void
  onMessage?: (event: SSEDataEvent) => void
  onStatusChange?: (connected: boolean) => void
}

// ============================================================================
// Hook Return Value
// ============================================================================

export interface UseSSEReturn {
  /** Current connection state */
  connectionState: "disconnected" | "connecting" | "connected" | "reconnecting" | "error"

  /** Whether currently connected */
  isConnected: boolean

  /** Whether currently connecting */
  isConnecting: boolean

  /** Error message if connection failed */
  error: string | null

  /** Subscribed event channels */
  channels: SSEEventType[]

  /** Reconnection attempt count */
  reconnectAttempts: number

  /** Timestamp of last successful connection */
  lastConnected: number | null

  /** Manually connect to SSE endpoint */
  connect: () => void

  /** Manually disconnect from SSE endpoint */
  disconnect: () => void

  /** Reconnect to SSE endpoint */
  reconnect: () => void

  /** Subscribe to additional channels */
  subscribeChannels: (channels: SSEEventType[]) => void

  /** Unsubscribe from channels */
  unsubscribeChannels: (channels: SSEEventType[]) => void
}

// ============================================================================
// Hook Implementation
// ============================================================================

const DEFAULT_CHANNELS: SSEEventType[] = ["message", "status", "error", "permission", "progress"]

export function useSSE(options: UseSSEOptions = {}): UseSSEReturn {
  const {
    autoConnect = true,
    channels: channelOptions,
    baseUrl,
    apiKey,
    reconnectInterval,
    maxReconnectAttempts,
    headers,
    onOpen,
    onError,
    onClose,
    onMessage,
    onStatusChange,
  } = options

  // Use stable reference for channels - use default if not provided
  const channels = channelOptions ?? DEFAULT_CHANNELS

  // SSE store connection methods
  const connectStore = useSSEStore((state) => state.connect)
  const disconnectStore = useSSEStore((state) => state.disconnect)
  const reconnectStore = useSSEStore((state) => state.reconnect)
  const subscribeChannelsStore = useSSEStore((state) => state.subscribeChannels)
  const unsubscribeChannelsStore = useSSEStore((state) => state.unsubscribeChannels)

  // SSE store state
  const connectionState = useSSEStore((state) => state.connectionState)
  const subscribedChannels = useSSEStore(useShallow((state) => Array.from(state.subscribedChannels)))
  const reconnectAttempts = useSSEStore((state) => state.reconnectAttempts)
  const lastConnected = useSSEStore((state) => state.lastConnected)
  const error = useSSEStore((state) => state.error)

  // Store client factory ref to avoid recreating on every render
  const clientFactoryRef = React.useRef<SSEClient | null>(null)
  const channelsRef = React.useRef(channels)
  channelsRef.current = channels

  // Create client factory
  const createClient = React.useCallback(() => {
    if (clientFactoryRef.current) {
      return clientFactoryRef.current
    }

    const handlers: SSEEventHandlers = {
      onOpen: () => {
        onOpen?.()
      },
      onError: (error) => {
        onError?.(error)
      },
      onClose: () => {
        onClose?.()
      },
      onMessage: (event) => {
        onMessage?.(event)
      },
      onStatusChange: (connected) => {
        onStatusChange?.(connected)
      },
    }

    const config: SSEClientConfig = {
      baseUrl,
      apiKey,
      channels: channelsRef.current,
      reconnectInterval,
      maxReconnectAttempts,
      headers,
    }

    clientFactoryRef.current = new SSEClient(config, handlers)
    return clientFactoryRef.current
  }, [baseUrl, apiKey, reconnectInterval, maxReconnectAttempts, headers, onOpen, onError, onClose, onMessage, onStatusChange])

  // Connect method
  const connect = React.useCallback(() => {
    connectStore(() => createClient())
  }, [connectStore, createClient])

  // Disconnect method
  const disconnect = React.useCallback(() => {
    disconnectStore()
  }, [disconnectStore])

  // Reconnect method
  const reconnect = React.useCallback(() => {
    reconnectStore()
  }, [reconnectStore])

  // Subscribe channels method
  const subscribeChannels = React.useCallback((chs: SSEEventType[]) => {
    subscribeChannelsStore(chs)
  }, [subscribeChannelsStore])

  // Unsubscribe channels method
  const unsubscribeChannels = React.useCallback((chs: SSEEventType[]) => {
    unsubscribeChannelsStore(chs)
  }, [unsubscribeChannelsStore])

  // Auto-connect on mount - intentionally only depends on autoConnect
  // Using refs to avoid re-running effect when callbacks change
  const connectRef = React.useRef(connect)
  const disconnectRef = React.useRef(disconnect)
  connectRef.current = connect
  disconnectRef.current = disconnect

  React.useEffect(() => {
    if (autoConnect) {
      connectRef.current()
    }

    return () => {
      disconnectRef.current()
    }
  }, [autoConnect])

  // Update channels when options change - use ref to track previous channels
  const prevChannelsRef = React.useRef<SSEEventType[]>([])
  React.useEffect(() => {
    // Only subscribe if channels actually changed (shallow compare)
    const channelsChanged =
      channels.length !== prevChannelsRef.current.length ||
      channels.some((ch, i) => ch !== prevChannelsRef.current[i])

    if (channelsChanged && channels.length > 0) {
      subscribeChannelsStore(channels)
      prevChannelsRef.current = channels
    }
  }, [channels, subscribeChannelsStore])

  return {
    connectionState,
    isConnected: connectionState === "connected",
    isConnecting: connectionState === "connecting" || connectionState === "reconnecting",
    error,
    channels: subscribedChannels,
    reconnectAttempts,
    lastConnected,
    connect,
    disconnect,
    reconnect,
    subscribeChannels,
    unsubscribeChannels,
  }
}

// ============================================================================
// Convenience Hooks
// ============================================================================

/**
 * Hook that only provides SSE connection status
 */
export function useSSEStatus() {
  const isConnected = useSSEStore((state) => state.connectionState === "connected")
  const isConnecting = useSSEStore((state) =>
    state.connectionState === "connecting" || state.connectionState === "reconnecting",
  )
  const error = useSSEStore((state) => state.error)

  return { isConnected, isConnecting, error }
}

/**
 * Hook for SSE message events
 */
export function useSSEMessages() {
  const events = useSSEStore((state) => state.events)
  const clearEvents = useSSEStore((state) => state.clearEvents)

  return { events, clearEvents }
}
