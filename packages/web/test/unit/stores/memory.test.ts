import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useMemoryStore,
  useDailyDates,
  useSelectedDate,
  useDailyEntries,
  useLongTermContent,
  useMemorySections,
  useConsolidationStats,
  useMemorySummary,
} from "@/stores/memory"
import { api } from "@/lib/api"
import type { MemorySection, ConsolidationStats, MemorySummary } from "@/lib/types"
import { renderHook } from "@testing-library/react"

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    listDailyDates: vi.fn(),
    getDailyNotes: vi.fn(),
    appendDailyNote: vi.fn(),
    getLongTermMemory: vi.fn(),
    getMemorySections: vi.fn(),
    updateMemoryCategory: vi.fn(),
    mergeToMemoryCategory: vi.fn(),
    getConsolidationStats: vi.fn(),
    triggerConsolidation: vi.fn(),
    getMemorySummary: vi.fn(),
  },
}))

const mockDates = ["2026-03-01", "2026-02-28", "2026-02-27"]
const mockEntries = ["Entry 1", "Entry 2", "Entry 3"]
const mockSections: MemorySection[] = [
  { category: "User Preferences", content: "Some preferences", lastModified: Date.now() },
  { category: "Project Context", content: "Some context", lastModified: Date.now() },
]
const mockStats: ConsolidationStats = {
  lastRun: Date.now() - 3600000,
  entriesProcessed: 50,
  patternsMerged: 5,
}
const mockSummary: MemorySummary = {
  dailyNotesCount: 10,
  categoriesCount: 4,
  lastDailyEntry: "2026-03-01",
  lastConsolidation: Date.now() - 86400000,
}

describe("Memory Store", () => {
  beforeEach(() => {
    // Reset the store before each test
    useMemoryStore.getState().reset()
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have empty dailyDates", () => {
      const state = useMemoryStore.getState()
      expect(state.dailyDates).toEqual([])
    })

    it("should have null selectedDate", () => {
      const state = useMemoryStore.getState()
      expect(state.selectedDate).toBeNull()
    })

    it("should have empty dailyEntries", () => {
      const state = useMemoryStore.getState()
      expect(state.dailyEntries).toEqual([])
    })

    it("should have empty longTermContent", () => {
      const state = useMemoryStore.getState()
      expect(state.longTermContent).toBe("")
    })

    it("should have empty sections", () => {
      const state = useMemoryStore.getState()
      expect(state.sections).toEqual([])
    })

    it("should have null consolidationStats", () => {
      const state = useMemoryStore.getState()
      expect(state.consolidationStats).toBeNull()
    })
  })

  describe("fetchDailyDates", () => {
    it("should load dates from API", async () => {
      vi.mocked(api.listDailyDates).mockResolvedValueOnce(mockDates)

      await useMemoryStore.getState().fetchDailyDates()

      const state = useMemoryStore.getState()
      expect(state.dailyDates).toEqual(mockDates)
      expect(state.dailyLoading).toBe(false)
    })

    it("should set dailyLoading during fetch", async () => {
      vi.mocked(api.listDailyDates).mockImplementationOnce(async () => {
        expect(useMemoryStore.getState().dailyLoading).toBe(true)
        return mockDates
      })

      await useMemoryStore.getState().fetchDailyDates()
    })

    it("should handle API errors", async () => {
      vi.mocked(api.listDailyDates).mockRejectedValueOnce(new Error("Network error"))

      await useMemoryStore.getState().fetchDailyDates()

      const state = useMemoryStore.getState()
      expect(state.dailyError).toBe("Network error")
      expect(state.dailyLoading).toBe(false)
    })
  })

  describe("selectDate", () => {
    it("should load entries for selected date", async () => {
      vi.mocked(api.getDailyNotes).mockResolvedValueOnce(mockEntries)

      await useMemoryStore.getState().selectDate("2026-03-01")

      const state = useMemoryStore.getState()
      expect(state.selectedDate).toBe("2026-03-01")
      expect(state.dailyEntries).toEqual(mockEntries)
      expect(state.dailyLoading).toBe(false)
    })

    it("should clear entries when selecting null", async () => {
      useMemoryStore.setState({ selectedDate: "2026-03-01", dailyEntries: mockEntries })

      await useMemoryStore.getState().selectDate(null)

      const state = useMemoryStore.getState()
      expect(state.selectedDate).toBeNull()
      expect(state.dailyEntries).toEqual([])
    })

    it("should handle API errors", async () => {
      vi.mocked(api.getDailyNotes).mockRejectedValueOnce(new Error("Not found"))

      await useMemoryStore.getState().selectDate("2026-03-01")

      const state = useMemoryStore.getState()
      expect(state.dailyError).toBe("Not found")
    })
  })

  describe("appendDailyNote", () => {
    it("should append note via API", async () => {
      vi.mocked(api.appendDailyNote).mockResolvedValueOnce(undefined)

      await useMemoryStore.getState().appendDailyNote("New note", "note", { key: "value" })

      expect(api.appendDailyNote).toHaveBeenCalledWith({
        type: "note",
        content: "New note",
        metadata: { key: "value" },
      })
    })

    it("should refresh today if selected", async () => {
      const today = new Date().toISOString().split("T")[0]
      useMemoryStore.setState({ selectedDate: today })
      vi.mocked(api.appendDailyNote).mockResolvedValueOnce(undefined)
      vi.mocked(api.getDailyNotes).mockResolvedValueOnce(["New entry"])

      await useMemoryStore.getState().appendDailyNote("New note")

      expect(api.getDailyNotes).toHaveBeenCalledWith(today)
    })
  })

  describe("fetchLongTermMemory", () => {
    it("should load long-term memory from API", async () => {
      vi.mocked(api.getLongTermMemory).mockResolvedValueOnce({ content: "Long-term content" })

      await useMemoryStore.getState().fetchLongTermMemory()

      const state = useMemoryStore.getState()
      expect(state.longTermContent).toBe("Long-term content")
      expect(state.longTermLoading).toBe(false)
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.getLongTermMemory).mockRejectedValueOnce(new Error("Failed"))

      await useMemoryStore.getState().fetchLongTermMemory()

      expect(useMemoryStore.getState().longTermLoading).toBe(false)
    })
  })

  describe("fetchSections", () => {
    it("should load sections from API", async () => {
      vi.mocked(api.getMemorySections).mockResolvedValueOnce(mockSections)

      await useMemoryStore.getState().fetchSections()

      const state = useMemoryStore.getState()
      expect(state.sections).toEqual(mockSections)
      expect(state.longTermLoading).toBe(false)
    })
  })

  describe("updateCategory", () => {
    it("should update category and refresh sections", async () => {
      vi.mocked(api.updateMemoryCategory).mockResolvedValueOnce(undefined)
      vi.mocked(api.getMemorySections).mockResolvedValueOnce(mockSections)

      await useMemoryStore.getState().updateCategory("User Preferences", "Updated content")

      expect(api.updateMemoryCategory).toHaveBeenCalledWith("User Preferences", "Updated content")
      expect(api.getMemorySections).toHaveBeenCalled()
    })
  })

  describe("mergeToCategory", () => {
    it("should merge to category and refresh sections", async () => {
      vi.mocked(api.mergeToMemoryCategory).mockResolvedValueOnce(undefined)
      vi.mocked(api.getMemorySections).mockResolvedValueOnce(mockSections)

      await useMemoryStore.getState().mergeToCategory("User Preferences", "New content")

      expect(api.mergeToMemoryCategory).toHaveBeenCalledWith("User Preferences", "New content")
      expect(api.getMemorySections).toHaveBeenCalled()
    })
  })

  describe("fetchConsolidationStats", () => {
    it("should load stats from API", async () => {
      vi.mocked(api.getConsolidationStats).mockResolvedValueOnce(mockStats)

      await useMemoryStore.getState().fetchConsolidationStats()

      expect(useMemoryStore.getState().consolidationStats).toEqual(mockStats)
    })

    it("should handle errors silently", async () => {
      vi.mocked(api.getConsolidationStats).mockRejectedValueOnce(new Error("Failed"))

      await useMemoryStore.getState().fetchConsolidationStats()

      // Should not throw
    })
  })

  describe("triggerConsolidation", () => {
    it("should trigger consolidation and refresh", async () => {
      vi.mocked(api.triggerConsolidation).mockResolvedValueOnce(undefined)
      vi.mocked(api.getConsolidationStats).mockResolvedValueOnce(mockStats)
      vi.mocked(api.getMemorySections).mockResolvedValueOnce(mockSections)

      await useMemoryStore.getState().triggerConsolidation(7)

      expect(api.triggerConsolidation).toHaveBeenCalledWith({ days: 7 })
      expect(useMemoryStore.getState().consolidating).toBe(false)
    })

    it("should set consolidating during operation", async () => {
      vi.mocked(api.triggerConsolidation).mockImplementationOnce(async () => {
        expect(useMemoryStore.getState().consolidating).toBe(true)
      })
      vi.mocked(api.getConsolidationStats).mockResolvedValueOnce(mockStats)
      vi.mocked(api.getMemorySections).mockResolvedValueOnce(mockSections)

      await useMemoryStore.getState().triggerConsolidation()
    })
  })

  describe("fetchSummary", () => {
    it("should load summary from API", async () => {
      vi.mocked(api.getMemorySummary).mockResolvedValueOnce(mockSummary)

      await useMemoryStore.getState().fetchSummary()

      expect(useMemoryStore.getState().summary).toEqual(mockSummary)
    })
  })

  describe("reset", () => {
    it("should reset to initial state", () => {
      useMemoryStore.setState({
        dailyDates: mockDates,
        selectedDate: "2026-03-01",
        dailyEntries: mockEntries,
        longTermContent: "content",
        sections: mockSections,
      })

      useMemoryStore.getState().reset()

      const state = useMemoryStore.getState()
      expect(state.dailyDates).toEqual([])
      expect(state.selectedDate).toBeNull()
      expect(state.dailyEntries).toEqual([])
      expect(state.longTermContent).toBe("")
      expect(state.sections).toEqual([])
    })
  })

  describe("hooks", () => {
    it("useDailyDates should return dates", () => {
      useMemoryStore.setState({ dailyDates: mockDates })

      const { result } = renderHook(() => useDailyDates())
      expect(result.current).toEqual(mockDates)
    })

    it("useSelectedDate should return selected date", () => {
      useMemoryStore.setState({ selectedDate: "2026-03-01" })

      const { result } = renderHook(() => useSelectedDate())
      expect(result.current).toBe("2026-03-01")
    })

    it("useDailyEntries should return entries", () => {
      useMemoryStore.setState({ dailyEntries: mockEntries })

      const { result } = renderHook(() => useDailyEntries())
      expect(result.current).toEqual(mockEntries)
    })

    it("useLongTermContent should return content", () => {
      useMemoryStore.setState({ longTermContent: "Test content" })

      const { result } = renderHook(() => useLongTermContent())
      expect(result.current).toBe("Test content")
    })

    it("useMemorySections should return sections", () => {
      useMemoryStore.setState({ sections: mockSections })

      const { result } = renderHook(() => useMemorySections())
      expect(result.current).toEqual(mockSections)
    })

    it("useConsolidationStats should return stats", () => {
      useMemoryStore.setState({ consolidationStats: mockStats })

      const { result } = renderHook(() => useConsolidationStats())
      expect(result.current).toEqual(mockStats)
    })

    it("useMemorySummary should return summary", () => {
      useMemoryStore.setState({ summary: mockSummary })

      const { result } = renderHook(() => useMemorySummary())
      expect(result.current).toEqual(mockSummary)
    })
  })
})
