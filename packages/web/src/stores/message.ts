/**
 * Message Store
 * Manages message state for sessions including loading, adding, and updating messages
 */

import { create } from "zustand"
import { immer } from "zustand/middleware/immer"
import { enableMapSet } from "immer"
import { useShallow } from "zustand/react/shallow"
import type { MessageWithParts, MessagePart } from "../lib/types"
import { api } from "../lib/api"

// Enable Immer support for Map and Set
enableMapSet()

// ============================================================================
// State Interface
// ============================================================================

interface MessageState {
  // Data: messages grouped by session ID
  messagesBySession: Map<string, MessageWithParts[]>

  // Loading states
  loadingStates: Map<string, boolean>
  loadedStates: Map<string, boolean>

  // Error state
  errors: Map<string, string | null>
}

interface MessageActions {
  // Message management
  loadMessages: (sessionId: string, limit?: number) => Promise<void>
  addMessage: (sessionId: string, message: MessageWithParts) => void
  updateMessage: (sessionId: string, messageId: string, updates: Partial<MessageWithParts>) => void
  appendPart: (sessionId: string, messageId: string, part: MessagePart) => void
  updatePart: (sessionId: string, messageId: string, partId: string, updates: Partial<MessagePart>) => void
  clearMessages: (sessionId: string) => void

  // State management
  setError: (sessionId: string, error: string | null) => void
  clearSessionErrors: () => void
  reset: () => void
}

type MessageStore = MessageState & MessageActions

// ============================================================================
// Initial State
// ============================================================================

const initialState: MessageState = {
  messagesBySession: new Map(),
  loadingStates: new Map(),
  loadedStates: new Map(),
  errors: new Map(),
}

// ============================================================================
// Store Creation
// ============================================================================

const useMessageStoreBase = create<MessageStore>()(
  immer((set) => ({
    ...initialState,

    // ======================================================================
    // Message Management
    // ======================================================================

    /**
     * Load messages for a session from the API
     */
    loadMessages: async (sessionId, limit) => {
      set((state) => {
        state.loadingStates.set(sessionId, true)
        state.errors.set(sessionId, null)
      })

      try {
        const messages = await api.getSessionMessages(sessionId, { limit })

        set((state) => {
          state.messagesBySession.set(sessionId, messages)
          state.loadingStates.set(sessionId, false)
          state.loadedStates.set(sessionId, true)
        })
      } catch (error) {
        set((state) => {
          state.loadingStates.set(sessionId, false)
          state.errors.set(sessionId, error instanceof Error ? error.message : "Failed to load messages")
        })
      }
    },

    /**
     * Add a message to a session
     */
    addMessage: (sessionId, message) => {
      set((state) => {
        const messages = state.messagesBySession.get(sessionId) ?? []
        messages.push(message)
        state.messagesBySession.set(sessionId, messages)
      })
    },

    /**
     * Update a message in a session
     */
    updateMessage: (sessionId, messageId, updates) => {
      set((state) => {
        const messages = state.messagesBySession.get(sessionId)
        if (!messages) return

        const index = messages.findIndex((m) => m.info.id === messageId)
        if (index === -1) return

        Object.assign(messages[index].info, updates)
      })
    },

    /**
     * Append a part to a message
     */
    appendPart: (sessionId, messageId, part) => {
      set((state) => {
        const messages = state.messagesBySession.get(sessionId)
        if (!messages) return

        const message = messages.find((m) => m.info.id === messageId)
        if (!message) return

        message.parts.push(part)
      })
    },

    /**
     * Update a part in a message
     */
    updatePart: (sessionId, messageId, partId, updates) => {
      set((state) => {
        const messages = state.messagesBySession.get(sessionId)
        if (!messages) return

        const message = messages.find((m) => m.info.id === messageId)
        if (!message) return

        const part = message.parts.find((p) => p.id === partId)
        if (!part) return

        Object.assign(part, updates)
      })
    },

    /**
     * Clear all messages for a session
     */
    clearMessages: (sessionId) => {
      set((state) => {
        state.messagesBySession.delete(sessionId)
        state.loadedStates.delete(sessionId)
        state.loadingStates.delete(sessionId)
        state.errors.delete(sessionId)
      })
    },

    // ======================================================================
    // State Management
    // ======================================================================

    setError: (sessionId, error) => {
      set((state) => {
        state.errors.set(sessionId, error)
      })
    },

    clearSessionErrors: () => {
      set((state) => {
        state.errors.clear()
      })
    },

    reset: () => {
      set((state) => {
        state.messagesBySession.clear()
        state.loadingStates.clear()
        state.loadedStates.clear()
        state.errors.clear()
      })
    },
  })),
)

// ============================================================================
// Selector Hooks
// ============================================================================

// Empty array constant to prevent creating new references
const EMPTY_MESSAGES: MessageWithParts[] = []

/**
 * Get all messages for a session
 * Uses shallow comparison to prevent unnecessary re-renders
 */
export const useMessages = (sessionId: string) =>
  useMessageStoreBase(useShallow((state) => state.messagesBySession.get(sessionId) ?? EMPTY_MESSAGES))

/**
 * Get a specific message by ID
 */
export const useMessage = (sessionId: string, messageId: string) =>
  useMessageStoreBase((state) => {
    const messages = state.messagesBySession.get(sessionId)
    return messages?.find((m) => m.info.id === messageId)
  })

/**
 * Get loading state for a session's messages
 */
export const useMessagesLoading = (sessionId: string) =>
  useMessageStoreBase((state) => state.loadingStates.get(sessionId) ?? false)

/**
 * Get loaded state for a session's messages
 */
export const useMessagesLoaded = (sessionId: string) =>
  useMessageStoreBase((state) => state.loadedStates.get(sessionId) ?? false)

/**
 * Get error state for a session's messages
 */
export const useMessagesError = (sessionId: string) =>
  useMessageStoreBase((state) => state.errors.get(sessionId) ?? null)

/**
 * Get all loading states
 */
export const useAllMessagesLoading = () =>
  useMessageStoreBase((state) => new Map(state.loadingStates))

/**
 * Get all errors
 */
export const useAllMessagesErrors = () =>
  useMessageStoreBase((state) => new Map(state.errors))

// ============================================================================
// Export Store
// ============================================================================

export { useMessageStoreBase as useMessageStore }
