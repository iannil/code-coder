/**
 * Agent Store
 * Manages agent state including available agents and selected agent with localStorage persistence
 */

import { create } from "zustand"
// import { persist } from "zustand/middleware"
import { immer } from "zustand/middleware/immer"
import { enableMapSet } from "immer"
import { useShallow } from "zustand/react/shallow"
import { useMemo } from "react"
import type { AgentInfo } from "../lib/types"

// Enable Immer support for Map and Set
enableMapSet()

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
// Static Agent List (since no API endpoint exists yet)
// ============================================================================

const STATIC_AGENTS: AgentInfo[] = [
  // Main modes
  { id: "build", name: "build", description: "Primary build and development mode", category: "primary" },
  { id: "plan", name: "plan", description: "Plan mode for architecture and design", category: "primary" },

  // Engineering
  { id: "code-reviewer", name: "code-reviewer", description: "Review code for quality and best practices", category: "engineering" },
  { id: "security-reviewer", name: "security-reviewer", description: "Review code for security vulnerabilities", category: "engineering" },
  { id: "tdd-guide", name: "tdd-guide", description: "Test-driven development guidance", category: "engineering" },
  { id: "architect", name: "architect", description: "System architecture and design", category: "engineering" },
  { id: "explore", name: "explore", description: "Explore and understand codebase", category: "engineering" },
  { id: "general", name: "general", description: "General-purpose assistant", category: "engineering" },

  // Content
  { id: "writer", name: "writer", description: "Long-form content writing (20k+ words)", category: "content" },
  { id: "proofreader", name: "proofreader", description: "Proofread and edit content", category: "content" },
  { id: "expander", name: "expander", description: "Expand content from outlines", category: "content" },
  { id: "expander-fiction", name: "expander-fiction", description: "Expand fiction content", category: "content" },
  { id: "expander-nonfiction", name: "expander-nonfiction", description: "Expand non-fiction content", category: "content" },

  // ZRS (Zhurong Say) Analysis
  { id: "observer", name: "observer", description: "Observer perspective analysis", category: "zrs" },
  { id: "decision", name: "decision", description: "CLOSE framework decision analysis", category: "zrs" },
  { id: "macro", name: "macro", description: "Macro-economic data interpretation", category: "zrs" },
  { id: "trader", name: "trader", description: "Trading and market analysis", category: "zrs" },
  { id: "picker", name: "picker", description: "Product selection using 七宗罪选品法", category: "zrs" },
  { id: "miniproduct", name: "miniproduct", description: "Minimal product development guide", category: "zrs" },
  { id: "ai-engineer", name: "ai-engineer", description: "AI engineering specialist", category: "zrs" },

  // Reverse Engineering
  { id: "code-reverse", name: "code-reverse", description: "Code reverse engineering", category: "reverse" },
  { id: "jar-code-reverse", name: "jar-code-reverse", description: "JAR file reverse engineering", category: "reverse" },
]

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
          // Use static agents list since no API endpoint exists yet
          const agents = STATIC_AGENTS

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
 * Uses useMemo for stable reference
 */
export const useAgentsByCategory = (): Record<string, AgentInfo[]> => {
  const agents = useAgents()

  return useMemo(() => {
    const grouped: Record<string, AgentInfo[]> = {}
    for (const agent of agents) {
      const category = agent.category ?? "general"
      if (!grouped[category]) {
        grouped[category] = []
      }
      grouped[category].push(agent)
    }
    return grouped
  }, [agents])
}

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
