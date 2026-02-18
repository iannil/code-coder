/**
 * Credential Store
 * Manages credential state and CRUD operations
 */

import { create } from "zustand"
import { immer } from "zustand/middleware/immer"
import { useShallow } from "zustand/react/shallow"
import type { CredentialSummary, CredentialCreateInput } from "../lib/types"
import { api } from "../lib/api"

// ============================================================================
// State Interface
// ============================================================================

interface CredentialState {
  credentials: CredentialSummary[]
  isLoading: boolean
  isLoaded: boolean
  isAdding: boolean
  isDeleting: string | null
  error: string | null
}

interface CredentialActions {
  fetchCredentials: () => Promise<void>
  addCredential: (input: CredentialCreateInput) => Promise<string>
  deleteCredential: (id: string) => Promise<void>
  clearError: () => void
}

type CredentialStore = CredentialState & CredentialActions

// ============================================================================
// Initial State
// ============================================================================

const initialState: CredentialState = {
  credentials: [],
  isLoading: false,
  isLoaded: false,
  isAdding: false,
  isDeleting: null,
  error: null,
}

// ============================================================================
// Store Creation
// ============================================================================

export const useCredentialStore = create<CredentialStore>()(
  immer((set) => ({
    ...initialState,

    fetchCredentials: async () => {
      set((state) => {
        state.isLoading = true
        state.error = null
      })
      try {
        const credentials = await api.listCredentials()
        set((state) => {
          state.credentials = credentials
          state.isLoading = false
          state.isLoaded = true
        })
      } catch (error) {
        set((state) => {
          state.isLoading = false
          state.error = error instanceof Error ? error.message : "Failed to load credentials"
        })
      }
    },

    addCredential: async (input) => {
      set((state) => {
        state.isAdding = true
        state.error = null
      })
      try {
        const { id } = await api.addCredential(input)
        // Refresh credentials list after adding
        const credentials = await api.listCredentials()
        set((state) => {
          state.credentials = credentials
          state.isAdding = false
        })
        return id
      } catch (error) {
        set((state) => {
          state.isAdding = false
          state.error = error instanceof Error ? error.message : "Failed to add credential"
        })
        throw error
      }
    },

    deleteCredential: async (id) => {
      set((state) => {
        state.isDeleting = id
        state.error = null
      })
      try {
        await api.deleteCredential(id)
        set((state) => {
          state.credentials = state.credentials.filter((c) => c.id !== id)
          state.isDeleting = null
        })
      } catch (error) {
        set((state) => {
          state.isDeleting = null
          state.error = error instanceof Error ? error.message : "Failed to delete credential"
        })
        throw error
      }
    },

    clearError: () =>
      set((state) => {
        state.error = null
      }),
  })),
)

// ============================================================================
// Selector Hooks
// ============================================================================

export const useCredentials = () => useCredentialStore((s) => s.credentials)

export const useCredentialLoading = () =>
  useCredentialStore(
    useShallow((s) => ({
      isLoading: s.isLoading,
      isLoaded: s.isLoaded,
      isAdding: s.isAdding,
      isDeleting: s.isDeleting,
    })),
  )

export const useCredentialError = () => useCredentialStore((s) => s.error)
