/**
 * Session Store
 * Manages session state including sessions list, active session, loading states, and persistence
 */

import { create } from "zustand"
// import { persist } from "zustand/middleware"
import { immer } from "zustand/middleware/immer"
import { enableMapSet } from "immer"
import { useShallow } from "zustand/react/shallow"
import type { SessionInfo } from "../lib/types"
import { api } from "../lib/api"

// Enable Immer support for Map and Set
enableMapSet()

// ============================================================================
// State Interface
// ============================================================================

interface SessionState {
  // Data
  sessions: Map<string, SessionInfo>
  activeSessionId: string | null

  // Loading states
  isLoading: boolean
  isLoaded: boolean
  isCreating: boolean
  isDeleting: Set<string>

  // Error state
  error: string | null
}

interface SessionActions {
  // Session management
  loadSessions: () => Promise<void>
  setActiveSession: (sessionId: string | null) => void
  createSession: (input?: {
    title?: string
    parentID?: string
    projectID?: string
    directory?: string
    agent?: string
    model?: string
  }) => Promise<SessionInfo>
  deleteSession: (sessionId: string) => Promise<void>
  renameSession: (sessionId: string, title: string) => Promise<void>
  refreshSession: (sessionId: string) => Promise<void>

  // State management
  setError: (error: string | null) => void
  clearError: () => void
  reset: () => void
}

type SessionStore = SessionState & SessionActions

// ============================================================================
// Initial State
// ============================================================================

const initialState: Omit<SessionState, "sessions" | "isDeleting"> = {
  activeSessionId: null,
  isLoading: false,
  isLoaded: false,
  isCreating: false,
  error: null,
}

// ============================================================================
// Store Creation
// ============================================================================

// Temporarily disable persist to debug infinite loop
const useSessionStoreBase = create<SessionStore>()(
  // persist(
    immer((set) => ({
      // Initial state
      sessions: new Map(),
      isDeleting: new Set(),
      ...initialState,

      // ======================================================================
      // Session Management
      // ======================================================================

      /**
       * Load all sessions from the API
       */
      loadSessions: async () => {
        set((state) => {
          state.isLoading = true
          state.error = null
        })

        try {
          const sessions = await api.listSessions()

          set((state) => {
            state.sessions.clear()
            for (const session of sessions) {
              state.sessions.set(session.id, session)
            }
            state.isLoading = false
            state.isLoaded = true
          })
        } catch (error) {
          set((state) => {
            state.isLoading = false
            state.error = error instanceof Error ? error.message : "Failed to load sessions"
          })
        }
      },

      /**
       * Set the active session by ID
       */
      setActiveSession: (sessionId) => {
        set((state) => {
          state.activeSessionId = sessionId
        })
      },

      /**
       * Create a new session
       */
      createSession: async (input) => {
        set((state) => {
          state.isCreating = true
          state.error = null
        })

        try {
          const session = await api.createSession(input ?? {})

          set((state) => {
            state.sessions.set(session.id, session)
            state.activeSessionId = session.id
            state.isCreating = false
          })

          return session
        } catch (error) {
          set((state) => {
            state.isCreating = false
            state.error = error instanceof Error ? error.message : "Failed to create session"
          })
          throw error
        }
      },

      /**
       * Delete a session by ID
       */
      deleteSession: async (sessionId) => {
        set((state) => {
          state.isDeleting.add(sessionId)
          state.error = null
        })

        try {
          await api.deleteSession(sessionId)

          set((state) => {
            state.sessions.delete(sessionId)
            state.isDeleting.delete(sessionId)

            // Clear active session if it was the deleted one
            if (state.activeSessionId === sessionId) {
              state.activeSessionId = null
            }
          })
        } catch (error) {
          set((state) => {
            state.isDeleting.delete(sessionId)
            state.error = error instanceof Error ? error.message : "Failed to delete session"
          })
          throw error
        }
      },

      /**
       * Rename a session by ID
       */
      renameSession: async (sessionId, title) => {
        try {
          const session = await api.updateSession(sessionId, { title })

          set((state) => {
            state.sessions.set(sessionId, session)
          })
        } catch (error) {
          set((state) => {
            state.error = error instanceof Error ? error.message : "Failed to rename session"
          })
          throw error
        }
      },

      /**
       * Refresh a single session from the API
       */
      refreshSession: async (sessionId) => {
        try {
          const session = await api.getSession(sessionId)

          set((state) => {
            state.sessions.set(sessionId, session)
          })
        } catch (error) {
          set((state) => {
            state.error = error instanceof Error ? error.message : "Failed to refresh session"
          })
        }
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
        set((state) => {
          state.sessions.clear()
          state.isDeleting.clear()
          Object.assign(state, initialState)
        })
      },
    }))
  // ),
  // {
  //   name: "codecoder-sessions-storage",
  //   partialize: (state) => ({
  //     activeSessionId: state.activeSessionId,
  //     // Store sessions as array for serialization
  //     sessions: Array.from(state.sessions.values()),
  //   }),
  //   merge: (persistedState: any, currentState) => {
  //     // Create a new state with merged values - don't mutate immer proxy
  //     const state = currentState as SessionStore
  //
  //     // Build result with all required fields
  //     const result = {
  //       activeSessionId: persistedState.activeSessionId ?? state.activeSessionId,
  //       sessions: persistedState.sessions
  //         ? new Map((persistedState.sessions as SessionInfo[]).map((s) => [s.id, s]))
  //         : state.sessions,
  //       isLoading: state.isLoading,
  //       isLoaded: state.isLoaded,
  //       isCreating: state.isCreating,
  //       isDeleting: state.isDeleting,
  //       error: state.error,
  //       loadSessions: state.loadSessions,
  //       setActiveSession: state.setActiveSession,
  //       createSession: state.createSession,
  //       deleteSession: state.deleteSession,
  //       refreshSession: state.refreshSession,
  //       setError: state.setError,
  //       clearError: state.clearError,
  //       reset: state.reset,
  //     }
  //
  //     return result
  //   },
  // },
)

// ============================================================================
// Selector Hooks
// ============================================================================

/**
 * Get all sessions as an array
 * Uses shallow comparison to prevent unnecessary re-renders
 */
export const useSessions = () =>
  useSessionStoreBase(useShallow((state) => Array.from(state.sessions.values())))

/**
 * Get a session by ID
 */
export const useSession = (sessionId: string) =>
  useSessionStoreBase((state) => state.sessions.get(sessionId))

/**
 * Get the active session
 */
export const useActiveSession = () =>
  useSessionStoreBase((state) =>
    state.activeSessionId ? state.sessions.get(state.activeSessionId) ?? null : null,
  )

/**
 * Get the active session ID
 */
export const useActiveSessionId = () => useSessionStoreBase((state) => state.activeSessionId)

/**
 * Get loading state
 */
export const useSessionsLoading = () =>
  useSessionStoreBase(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isLoaded: state.isLoaded,
      isCreating: state.isCreating,
    }))
  )

/**
 * Get error state
 */
export const useSessionError = () => useSessionStoreBase((state) => state.error)

/**
 * Check if a session is being deleted
 */
export const useSessionDeleting = (sessionId: string) =>
  useSessionStoreBase((state) => state.isDeleting.has(sessionId))

// ============================================================================
// Export Store
// ============================================================================

export { useSessionStoreBase as useSessionStore }
