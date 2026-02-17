/**
 * Hooks Store
 *
 * Manages hooks configuration state:
 * - Hook entries by lifecycle
 * - Hook settings
 * - Hook locations
 * - Action types
 */

import { create } from "zustand"
import { useMemo } from "react"
import type {
  HookEntry,
  HookSettings,
  HookLocation,
  HookActionTypeInfo,
  HookLifecycle,
} from "@/lib/types"
import { api } from "@/lib/api"

// ============================================================================
// Types
// ============================================================================

interface HooksState {
  // Hooks
  hooks: HookEntry[]
  hooksLoading: boolean
  hooksError: string | null

  // Settings
  settings: HookSettings | null
  settingsLoading: boolean

  // Locations
  locations: HookLocation[]
  locationsLoading: boolean

  // Action types
  actionTypes: HookActionTypeInfo[]
  actionTypesLoading: boolean

  // Filters
  selectedLifecycle: HookLifecycle | null

  // Actions
  fetchHooks: () => Promise<void>
  fetchHooksByLifecycle: (lifecycle: HookLifecycle) => Promise<void>
  fetchSettings: () => Promise<void>
  fetchLocations: () => Promise<void>
  fetchActionTypes: () => Promise<void>
  setSelectedLifecycle: (lifecycle: HookLifecycle | null) => void
  reset: () => void
}

// ============================================================================
// Initial State
// ============================================================================

const initialState = {
  hooks: [],
  hooksLoading: false,
  hooksError: null,
  settings: null,
  settingsLoading: false,
  locations: [],
  locationsLoading: false,
  actionTypes: [],
  actionTypesLoading: false,
  selectedLifecycle: null,
}

// ============================================================================
// Store
// ============================================================================

export const useHooksStore = create<HooksState>((set) => ({
  ...initialState,

  fetchHooks: async () => {
    set({ hooksLoading: true, hooksError: null })
    try {
      const hooks = await api.listHooks()
      set({ hooks, hooksLoading: false })
    } catch (error) {
      set({
        hooksError: error instanceof Error ? error.message : "Failed to fetch hooks",
        hooksLoading: false,
      })
    }
  },

  fetchHooksByLifecycle: async (lifecycle) => {
    set({ hooksLoading: true, hooksError: null })
    try {
      const hooks = await api.getHooksByLifecycle(lifecycle)
      // Wrap in HookEntry format
      const entries: HookEntry[] = hooks.map((h) => ({
        lifecycle,
        name: h.name,
        definition: h.definition,
        source: "config",
      }))
      set({ hooks: entries, hooksLoading: false, selectedLifecycle: lifecycle })
    } catch (error) {
      set({
        hooksError: error instanceof Error ? error.message : "Failed to fetch hooks",
        hooksLoading: false,
      })
    }
  },

  fetchSettings: async () => {
    set({ settingsLoading: true })
    try {
      const settings = await api.getHooksSettings()
      set({ settings, settingsLoading: false })
    } catch {
      set({ settingsLoading: false })
    }
  },

  fetchLocations: async () => {
    set({ locationsLoading: true })
    try {
      const locations = await api.getHookLocations()
      set({ locations, locationsLoading: false })
    } catch {
      set({ locationsLoading: false })
    }
  },

  fetchActionTypes: async () => {
    set({ actionTypesLoading: true })
    try {
      const actionTypes = await api.getHookActionTypes()
      set({ actionTypes, actionTypesLoading: false })
    } catch {
      set({ actionTypesLoading: false })
    }
  },

  setSelectedLifecycle: (lifecycle) => {
    set({ selectedLifecycle: lifecycle })
  },

  reset: () => set(initialState),
}))

// ============================================================================
// Selectors
// ============================================================================

export const useHooks = () => useHooksStore((state) => state.hooks)
export const useHooksLoading = () => useHooksStore((state) => state.hooksLoading)
export const useHooksSettings = () => useHooksStore((state) => state.settings)
export const useHooksLocations = () => useHooksStore((state) => state.locations)
export const useHooksActionTypes = () => useHooksStore((state) => state.actionTypes)
export const useSelectedLifecycle = () => useHooksStore((state) => state.selectedLifecycle)

// Computed selectors
export const useHooksByLifecycle = (lifecycle: HookLifecycle): HookEntry[] => {
  const hooks = useHooks()
  return useMemo(() => hooks.filter((h) => h.lifecycle === lifecycle), [hooks, lifecycle])
}

export const useHookCounts = (): Record<HookLifecycle, number> => {
  const hooks = useHooks()
  return useMemo(() => {
    const counts: Record<HookLifecycle, number> = {
      PreToolUse: 0,
      PostToolUse: 0,
      PreResponse: 0,
      Stop: 0,
    }
    for (const hook of hooks) {
      counts[hook.lifecycle]++
    }
    return counts
  }, [hooks])
}
