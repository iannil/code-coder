/**
 * Config Store
 * Manages application configuration state with localStorage persistence
 */

import { create } from "zustand"
// import { persist } from "zustand/middleware"
import { immer } from "zustand/middleware/immer"
import { useShallow } from "zustand/react/shallow"
import type { ConfigData } from "../lib/types"
import { api } from "../lib/api"

// ============================================================================
// State Interface
// ============================================================================

interface ConfigState {
  // Data
  config: ConfigData | null

  // Loading states
  isLoading: boolean
  isLoaded: boolean
  isUpdating: boolean

  // Error state
  error: string | null
}

interface ConfigActions {
  // Config management
  loadConfig: () => Promise<void>
  updateConfig: (updates: Partial<ConfigData>) => Promise<ConfigData>
  setConfig: (config: ConfigData) => void
  resetConfig: () => void

  // State management
  setError: (error: string | null) => void
  clearError: () => void
}

type ConfigStore = ConfigState & ConfigActions

// ============================================================================
// Initial State
// ============================================================================

const initialState: Omit<ConfigState, "config"> = {
  isLoading: false,
  isLoaded: false,
  isUpdating: false,
  error: null,
}

// ============================================================================
// Store Creation
// ============================================================================

const useConfigStoreBase = create<ConfigStore>()(
  // Temporarily disable persist to debug infinite loop
  // persist(
    immer((set) => ({
      // Initial state
      config: null,
      ...initialState,

      // ======================================================================
      // Config Management
      // ======================================================================

      /**
       * Load configuration from the API
       */
      loadConfig: async () => {
        set((state) => {
          state.isLoading = true
          state.error = null
        })

        try {
          const config = await api.getConfig()

          set((state) => {
            state.config = config
            state.isLoading = false
            state.isLoaded = true
          })
        } catch (error) {
          set((state) => {
            state.isLoading = false
            state.error = error instanceof Error ? error.message : "Failed to load config"
          })
        }
      },

      /**
       * Update configuration via the API
       */
      updateConfig: async (updates) => {
        set((state) => {
          state.isUpdating = true
          state.error = null
        })

        try {
          const updated = await api.updateConfig(updates)

          set((state) => {
            state.config = updated
            state.isUpdating = false
          })

          return updated
        } catch (error) {
          set((state) => {
            state.isUpdating = false
            state.error = error instanceof Error ? error.message : "Failed to update config"
          })
          throw error
        }
      },

      /**
       * Set configuration locally (without API call)
       */
      setConfig: (config) => {
        set((state) => {
          state.config = config
          state.isLoaded = true
        })
      },

      /**
       * Reset configuration to null
       */
      resetConfig: () => {
        set((state) => {
          state.config = null
          state.isLoaded = false
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
    }))
  // ),
  // {
  //   name: "codecoder-config-storage",
  //   partialize: (state) => ({
  //     config: state.config,
  //     isLoaded: state.isLoaded,
  //   }),
  // },
)

// ============================================================================
// Selector Hooks
// ============================================================================

/**
 * Get the current config
 */
export const useConfig = () => useConfigStoreBase((state) => state.config)

/**
 * Get a specific config value by key
 */
export const useConfigValue = <K extends keyof ConfigData>(key: K) =>
  useConfigStoreBase((state) => (state.config?.[key] ?? null) as ConfigData[K] | null)

/**
 * Get multiple config values by keys
 */
export const useConfigValues = <K extends keyof ConfigData>(keys: K[]) =>
  useConfigStoreBase(
    useShallow((state) => {
      const result: Partial<Pick<ConfigData, K>> = {}
      if (state.config) {
        for (const key of keys) {
          if (key in state.config) {
            result[key] = state.config[key] as ConfigData[K]
          }
        }
      }
      return result as Pick<ConfigData, K>
    })
  )

/**
 * Get loading state
 */
export const useConfigLoading = () =>
  useConfigStoreBase(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isLoaded: state.isLoaded,
      isUpdating: state.isUpdating,
    }))
  )

/**
 * Get error state
 */
export const useConfigError = () => useConfigStoreBase((state) => state.error)

// ============================================================================
// Export Store
// ============================================================================

export { useConfigStoreBase as useConfigStore }
