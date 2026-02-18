/**
 * Project Store
 * Manages project state including project list, selected project, and loading states
 */

import { create } from "zustand"
import { immer } from "zustand/middleware/immer"
import { enableMapSet } from "immer"
import { useShallow } from "zustand/react/shallow"
import type { ProjectInfo, ProjectCreateInput } from "../lib/types"
import { api } from "../lib/api"

// Enable Immer support for Map and Set
enableMapSet()

// ============================================================================
// State Interface
// ============================================================================

interface ProjectState {
  // Data
  projects: Map<string, ProjectInfo>
  selectedProjectId: string | null

  // Loading states
  isLoading: boolean
  isLoaded: boolean
  isCreating: boolean
  isDeleting: Set<string>

  // Error state
  error: string | null
}

interface ProjectActions {
  // Project management
  loadProjects: () => Promise<void>
  selectProject: (projectId: string | null) => void
  createProject: (input: ProjectCreateInput) => Promise<ProjectInfo>
  deleteProject: (projectId: string) => Promise<void>
  updateProject: (
    projectId: string,
    input: { name?: string; icon?: { url?: string; override?: string; color?: string } },
  ) => Promise<void>

  // State management
  setError: (error: string | null) => void
  clearError: () => void
  reset: () => void
}

type ProjectStore = ProjectState & ProjectActions

// ============================================================================
// Initial State
// ============================================================================

const initialState: Omit<ProjectState, "projects" | "isDeleting"> = {
  selectedProjectId: null,
  isLoading: false,
  isLoaded: false,
  isCreating: false,
  error: null,
}

// ============================================================================
// Store Creation
// ============================================================================

const useProjectStoreBase = create<ProjectStore>()(
  immer((set) => ({
    // Initial state
    projects: new Map(),
    isDeleting: new Set(),
    ...initialState,

    // ======================================================================
    // Project Management
    // ======================================================================

    /**
     * Load all projects from the API
     */
    loadProjects: async () => {
      set((state) => {
        state.isLoading = true
        state.error = null
      })

      try {
        const projects = await api.listProjects()

        set((state) => {
          state.projects.clear()
          for (const project of projects) {
            state.projects.set(project.id, project)
          }
          state.isLoading = false
          state.isLoaded = true
        })
      } catch (error) {
        set((state) => {
          state.isLoading = false
          state.error = error instanceof Error ? error.message : "Failed to load projects"
        })
      }
    },

    /**
     * Select a project by ID
     */
    selectProject: (projectId) => {
      set((state) => {
        state.selectedProjectId = projectId
      })
    },

    /**
     * Create a new project
     */
    createProject: async (input) => {
      set((state) => {
        state.isCreating = true
        state.error = null
      })

      try {
        const project = await api.createProject(input)

        set((state) => {
          state.projects.set(project.id, project)
          state.selectedProjectId = project.id
          state.isCreating = false
        })

        return project
      } catch (error) {
        set((state) => {
          state.isCreating = false
          state.error = error instanceof Error ? error.message : "Failed to create project"
        })
        throw error
      }
    },

    /**
     * Delete a project by ID
     */
    deleteProject: async (projectId) => {
      set((state) => {
        state.isDeleting.add(projectId)
        state.error = null
      })

      try {
        await api.deleteProject(projectId)

        set((state) => {
          state.projects.delete(projectId)
          state.isDeleting.delete(projectId)

          // Clear selection if deleted project was selected
          if (state.selectedProjectId === projectId) {
            state.selectedProjectId = null
          }
        })
      } catch (error) {
        set((state) => {
          state.isDeleting.delete(projectId)
          state.error = error instanceof Error ? error.message : "Failed to delete project"
        })
        throw error
      }
    },

    /**
     * Update a project
     */
    updateProject: async (projectId, input) => {
      try {
        await api.updateProject(projectId, input)

        set((state) => {
          const project = state.projects.get(projectId)
          if (project) {
            if (input.name !== undefined) project.name = input.name
            if (input.icon !== undefined) project.icon = input.icon
            state.projects.set(projectId, project)
          }
        })
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : "Failed to update project"
        })
        throw error
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
        state.projects.clear()
        state.isDeleting.clear()
        Object.assign(state, initialState)
      })
    },
  })),
)

// ============================================================================
// Selector Hooks
// ============================================================================

/**
 * Get all projects as an array
 */
export const useProjects = () =>
  useProjectStoreBase(useShallow((state) => Array.from(state.projects.values())))

/**
 * Get a project by ID
 */
export const useProject = (projectId: string) =>
  useProjectStoreBase((state) => state.projects.get(projectId))

/**
 * Get the selected project
 */
export const useSelectedProject = () =>
  useProjectStoreBase((state) =>
    state.selectedProjectId ? state.projects.get(state.selectedProjectId) ?? null : null,
  )

/**
 * Get the selected project ID
 */
export const useSelectedProjectId = () => useProjectStoreBase((state) => state.selectedProjectId)

/**
 * Get loading state
 */
export const useProjectsLoading = () =>
  useProjectStoreBase(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isLoaded: state.isLoaded,
      isCreating: state.isCreating,
    })),
  )

/**
 * Get error state
 */
export const useProjectError = () => useProjectStoreBase((state) => state.error)

/**
 * Check if a project is being deleted
 */
export const useProjectDeleting = (projectId: string) =>
  useProjectStoreBase((state) => state.isDeleting.has(projectId))

// ============================================================================
// Export Store
// ============================================================================

export { useProjectStoreBase as useProjectStore }
