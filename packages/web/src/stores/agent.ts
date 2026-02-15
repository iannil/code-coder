/**
 * Agent Store
 * Manages agent state including available agents and selected agent with localStorage persistence
 */

import { create } from "zustand"
// import { persist } from "zustand/middleware"
import { immer } from "zustand/middleware/immer"
import { useShallow } from "zustand/react/shallow"
import type { AgentInfo } from "../lib/types"

// ============================================================================
// State Interface
// ============================================================================

interface AgentState {
  // Data
  // Note: Using 'agents' as the key for consistency with existing code
  agents: Map<string, AgentInfo>
  selectedAgentId: string | null

  // Loading states
  isLoading: boolean
  isLoaded: boolean

  // Error state
  error: string | null
}

interface AgentActions {
  // Agent management
  loadAgents: () => Promise<void>
  selectAgent: (agentId: string | null) => void
  setSelectedAgent: (agentId: string | null) => void
  reset: () => void

  // State management
  setError: (error: string | null) => void
  clearError: () => void
}

type AgentStore = AgentState & AgentActions

// ============================================================================
// Initial State
// ============================================================================

const initialState: Omit<AgentState, "agents"> = {
  selectedAgentId: null,
  isLoading: false,
  isLoaded: false,
  error: null,
}

// ============================================================================
// Store Creation
// ============================================================================

// Temporarily disable persist to debug infinite loop
const useAgentStoreBase = create<AgentStore>()(
  // persist(
    immer((set) => ({
      // Initial state - use consistent key name
      agents: new Map(),
      ...initialState,

      // ======================================================================
      // Agent Management
      // ======================================================================

      /**
       * Load all agents from the API
       */
      loadAgents: async () => {
        set((state) => {
          state.isLoading = true
          state.error = null
        })

        try {
          // Note: The API client may not have an agents endpoint yet
          // This is a placeholder implementation
          // const agents = await api.listAgents()

          // For now, use a mock implementation or empty array
          const agents: AgentInfo[] = []

          set((state) => {
            state.agents.clear()
            for (const agent of agents) {
              state.agents.set(agent.id, agent)
            }
            state.isLoading = false
            state.isLoaded = true
          })
        } catch (error) {
          set((state) => {
            state.isLoading = false
            state.error = error instanceof Error ? error.message : "Failed to load agents"
          })
        }
      },

      /**
       * Select an agent by ID
       * This is the public method that should be used by components
       */
      selectAgent: (agentId) => {
        set((state) => {
          // Validate that the agent exists
          if (agentId && !state.agents.has(agentId)) {
            console.warn(`Agent with ID "${agentId}" not found`)
          }
          state.selectedAgentId = agentId
        })
      },

      /**
       * Set the selected agent ID without validation
       * This is used internally (e.g., when restoring from persistence)
       */
      setSelectedAgent: (agentId) => {
        set((state) => {
          state.selectedAgentId = agentId
        })
      },

      /**
       * Reset the store to initial state
       */
      reset: () => {
        set((state) => {
          state.agents.clear()
          Object.assign(state, initialState)
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
  // ,
  // {
  //   name: "codecoder-agents-storage",
  //   partialize: (state) => ({
  //     selectedAgentId: state.selectedAgentId,
  //     // Store agents as array for serialization
  //     agents: Array.from(state.agents.values()),
  //   }),
  //   merge: (persistedState: any, currentState) => {
  //     // Create a new state with merged values - don't mutate immer proxy
  //     const state = currentState as AgentStore
  //
  //     // Build result with all required fields
  //     const result = {
  //       selectedAgentId: persistedState.selectedAgentId ?? state.selectedAgentId,
  //       agents: persistedState.agents
  //         ? new Map((persistedState.agents as AgentInfo[]).map((a) => [a.id, a]))
  //         : state.agents,
  //       isLoading: state.isLoading,
  //       isLoaded: state.isLoaded,
  //       error: state.error,
  //       loadAgents: state.loadAgents,
  //       selectAgent: state.selectAgent,
  //       setSelectedAgent: state.setSelectedAgent,
  //       reset: state.reset,
  //       setError: state.setError,
  //       clearError: state.clearError,
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
 * Get all agents as an array
 * Uses shallow comparison to prevent unnecessary re-renders
 */
export const useAgents = () =>
  useAgentStoreBase(useShallow((state) => Array.from(state.agents.values())))

/**
 * Get an agent by ID
 */
export const useAgent = (agentId: string) =>
  useAgentStoreBase((state) => state.agents.get(agentId))

/**
 * Get the selected agent
 */
export const useSelectedAgent = () =>
  useAgentStoreBase((state) =>
    state.selectedAgentId ? state.agents.get(state.selectedAgentId) ?? null : null,
  )

/**
 * Get the selected agent ID
 */
export const useSelectedAgentId = () => useAgentStoreBase((state) => state.selectedAgentId)

/**
 * Get agents grouped by category
 */
export const useAgentsByCategory = () =>
  useAgentStoreBase((state) => {
    const grouped = new Map<string, AgentInfo[]>()
    const agents = Array.from(state.agents.values())
    for (const agent of agents) {
      const category = agent.category ?? "general"
      const currentAgents = grouped.get(category) ?? []
      currentAgents.push(agent)
      grouped.set(category, currentAgents)
    }
    return grouped
  })

/**
 * Get loading state
 */
export const useAgentsLoading = () =>
  useAgentStoreBase(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isLoaded: state.isLoaded,
    }))
  )

/**
 * Get error state
 */
export const useAgentError = () => useAgentStoreBase((state) => state.error)

// ============================================================================
// Export Store
// ============================================================================

export { useAgentStoreBase as useAgentStore }
