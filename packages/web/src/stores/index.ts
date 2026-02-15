/**
 * Stores Index
 * Exports all Zustand stores for the CodeCoder web application
 */

// ============================================================================
// Session Store
// ============================================================================

export {
  useSessionStore,
  useSessions,
  useSession,
  useActiveSession,
  useActiveSessionId,
  useSessionsLoading,
  useSessionError,
  useSessionDeleting,
} from "./session"

// ============================================================================
// Message Store
// ============================================================================

export {
  useMessageStore,
  useMessages,
  useMessage,
  useMessagesLoading,
  useMessagesLoaded,
  useMessagesError,
  useAllMessagesLoading,
  useAllMessagesErrors,
} from "./message"

// ============================================================================
// Config Store
// ============================================================================

export {
  useConfigStore,
  useConfig,
  useConfigValue,
  useConfigValues,
  useConfigLoading,
  useConfigError,
} from "./config"

// ============================================================================
// Agent Store
// ============================================================================

export {
  useAgentStore,
  useAgents,
  useAgent,
  useSelectedAgent,
  useSelectedAgentId,
  useAgentsByCategory,
  useAgentsLoading,
  useAgentError,
} from "./agent"

// ============================================================================
// SSE Store
// ============================================================================

export {
  useSSEStore,
  useSSEConnectionState,
  useSSEConnected,
  useSSEConnecting,
  useSSESubscribedChannels,
  useSSEEvents,
  useSSEError,
  useSSEReconnectInfo,
} from "./sse"
