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

// ============================================================================
// Provider Store
// ============================================================================

export {
  useProviderStore,
  useProviders,
  useConnectedProviders,
  useSelectedModel,
  useModelFavorites,
  useModelRecents,
  useProviderLoading,
} from "./provider"

// ============================================================================
// MCP Store
// ============================================================================

export {
  useMcpStore,
  useMcpStatus,
  useMcpTools,
  useMcpResources,
  useMcpLoading,
} from "./mcp"

// ============================================================================
// Document Store
// ============================================================================

export {
  useDocumentStore,
  useDocuments,
  useSelectedDocument,
  useDocumentChapters,
  useSelectedChapter,
  useDocumentEntities,
  useDocumentVolumes,
  useDocumentStats,
  useDocumentsLoading,
} from "./document"

// ============================================================================
// Memory Store
// ============================================================================

export {
  useMemoryStore,
  useDailyDates,
  useSelectedDate,
  useDailyEntries,
  useLongTermContent,
  useMemorySections,
  useConsolidationStats,
  useMemorySummary,
} from "./memory"

// ============================================================================
// Hooks Store
// ============================================================================

export {
  useHooksStore,
  useHooks,
  useHooksLoading,
  useHooksSettings,
  useHooksLocations,
  useHooksActionTypes,
  useSelectedLifecycle,
  useHooksByLifecycle,
  useHookCounts,
} from "./hooks"

// ============================================================================
// LSP Store
// ============================================================================

export {
  useLspStore,
  useLspServers,
  useLspStatusLoading,
  useLspDiagnostics,
  useLspConfig,
  useHoverContent,
  useDefinitions,
  useReferences,
  useWorkspaceSymbols,
  useDocumentSymbols,
  useConnectedServers,
  useErrorServers,
  useConnectedServersCount,
  useErrorServersCount,
  useTotalDiagnostics,
  useErrorDiagnostics,
} from "./lsp"

// ============================================================================
// Channel Store (Infrastructure)
// ============================================================================

export {
  useChannelStore,
  useChannels,
  useChannelLoading,
  useEnabledChannels,
  useChannelCounts,
  useZeroBotStatus,
} from "./channel"

// ============================================================================
// Gateway Store (Infrastructure)
// ============================================================================

export {
  useGatewayStore,
  useGatewayStatus,
  useGatewayRunning,
  useGatewayEndpoints,
  useGatewayRequests,
  useGatewayLoading,
} from "./gateway"

// ============================================================================
// Cron Store (Infrastructure)
// ============================================================================

export {
  useCronStore,
  useCronJobs,
  useCronHistory,
  useCronLoading,
  useEnabledJobs,
  useCronCounts,
} from "./cron"

// ============================================================================
// Tunnel Store (Infrastructure)
// ============================================================================

export {
  useTunnelStore,
  useTunnelStatus,
  useTunnelConnected,
  useTunnelPublicUrl,
  useAvailableTunnelTypes,
  useTunnelLoading,
} from "./tunnel"

// ============================================================================
// Project Store
// ============================================================================

export {
  useProjectStore,
  useProjects,
  useProject,
  useSelectedProject,
  useSelectedProjectId,
  useProjectsLoading,
  useProjectError,
  useProjectDeleting,
} from "./project"

// ============================================================================
// Credential Store
// ============================================================================

export {
  useCredentialStore,
  useCredentials,
  useCredentialLoading,
  useCredentialError,
} from "./credential"
