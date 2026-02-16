/**
 * Memory Store
 *
 * Manages memory system state:
 * - Daily notes (flow layer)
 * - Long-term memory (sediment layer)
 * - Consolidation stats
 */

import { create } from "zustand"
import type {
  MemorySection,
  MemorySummary,
  ConsolidationStats,
} from "@/lib/types"
import { api } from "@/lib/api"

// ============================================================================
// Types
// ============================================================================

interface MemoryState {
  // Daily notes
  dailyDates: string[]
  selectedDate: string | null
  dailyEntries: string[]
  dailyLoading: boolean
  dailyError: string | null

  // Long-term memory
  longTermContent: string
  sections: MemorySection[]
  longTermLoading: boolean

  // Consolidation
  consolidationStats: ConsolidationStats | null
  consolidating: boolean

  // Summary
  summary: MemorySummary | null

  // Actions
  fetchDailyDates: () => Promise<void>
  selectDate: (date: string | null) => Promise<void>
  appendDailyNote: (content: string, type?: string, metadata?: Record<string, unknown>) => Promise<void>
  fetchLongTermMemory: () => Promise<void>
  fetchSections: () => Promise<void>
  updateCategory: (category: string, content: string) => Promise<void>
  mergeToCategory: (category: string, content: string) => Promise<void>
  fetchConsolidationStats: () => Promise<void>
  triggerConsolidation: (days?: number) => Promise<void>
  fetchSummary: () => Promise<void>
  reset: () => void
}

// ============================================================================
// Initial State
// ============================================================================

const initialState = {
  dailyDates: [],
  selectedDate: null,
  dailyEntries: [],
  dailyLoading: false,
  dailyError: null,
  longTermContent: "",
  sections: [],
  longTermLoading: false,
  consolidationStats: null,
  consolidating: false,
  summary: null,
}

// ============================================================================
// Store
// ============================================================================

export const useMemoryStore = create<MemoryState>((set, get) => ({
  ...initialState,

  fetchDailyDates: async () => {
    set({ dailyLoading: true, dailyError: null })
    try {
      const dates = await api.listDailyDates()
      set({ dailyDates: dates, dailyLoading: false })
    } catch (error) {
      set({
        dailyError: error instanceof Error ? error.message : "Failed to fetch dates",
        dailyLoading: false,
      })
    }
  },

  selectDate: async (date) => {
    if (!date) {
      set({ selectedDate: null, dailyEntries: [] })
      return
    }

    set({ selectedDate: date, dailyLoading: true })
    try {
      const entries = await api.getDailyNotes(date)
      set({ dailyEntries: entries, dailyLoading: false })
    } catch (error) {
      set({
        dailyError: error instanceof Error ? error.message : "Failed to fetch daily notes",
        dailyLoading: false,
      })
    }
  },

  appendDailyNote: async (content, type = "note", metadata) => {
    await api.appendDailyNote({ type, content, metadata })
    // Refresh current day if selected
    const { selectedDate, selectDate } = get()
    const today = new Date().toISOString().split("T")[0]
    if (selectedDate === today) {
      await selectDate(today)
    }
  },

  fetchLongTermMemory: async () => {
    set({ longTermLoading: true })
    try {
      const result = await api.getLongTermMemory()
      set({ longTermContent: result.content, longTermLoading: false })
    } catch {
      set({ longTermLoading: false })
    }
  },

  fetchSections: async () => {
    set({ longTermLoading: true })
    try {
      const sections = await api.getMemorySections()
      set({ sections, longTermLoading: false })
    } catch {
      set({ longTermLoading: false })
    }
  },

  updateCategory: async (category, content) => {
    await api.updateMemoryCategory(category, content)
    await get().fetchSections()
  },

  mergeToCategory: async (category, content) => {
    await api.mergeToMemoryCategory(category, content)
    await get().fetchSections()
  },

  fetchConsolidationStats: async () => {
    try {
      const stats = await api.getConsolidationStats()
      set({ consolidationStats: stats })
    } catch {
      // Ignore errors
    }
  },

  triggerConsolidation: async (days) => {
    set({ consolidating: true })
    try {
      await api.triggerConsolidation(days ? { days } : undefined)
      await get().fetchConsolidationStats()
      await get().fetchSections()
    } finally {
      set({ consolidating: false })
    }
  },

  fetchSummary: async () => {
    try {
      const summary = await api.getMemorySummary()
      set({ summary })
    } catch {
      // Ignore errors
    }
  },

  reset: () => set(initialState),
}))

// ============================================================================
// Selectors
// ============================================================================

export const useDailyDates = () => useMemoryStore((state) => state.dailyDates)
export const useSelectedDate = () => useMemoryStore((state) => state.selectedDate)
export const useDailyEntries = () => useMemoryStore((state) => state.dailyEntries)
export const useLongTermContent = () => useMemoryStore((state) => state.longTermContent)
export const useMemorySections = () => useMemoryStore((state) => state.sections)
export const useConsolidationStats = () => useMemoryStore((state) => state.consolidationStats)
export const useMemorySummary = () => useMemoryStore((state) => state.summary)
