/**
 * Cron Store
 *
 * Manages scheduled task state including:
 * - Cron job list
 * - Job creation/deletion
 * - Job execution history
 */

import { create } from "zustand"
import { useShallow } from "zustand/react/shallow"
import type { CronJob, CronHistory } from "@/lib/types"

// ============================================================================
// Types
// ============================================================================

interface CronState {
  jobs: CronJob[]
  history: CronHistory[]
  isLoading: boolean
  isCreating: boolean
  isRunning: string | null
  error: string | null
}

interface CronActions {
  fetchJobs: () => Promise<void>
  createJob: (job: Omit<CronJob, "id" | "nextRun" | "lastRun" | "lastStatus">) => Promise<void>
  deleteJob: (id: string) => Promise<void>
  toggleJob: (id: string) => Promise<void>
  runJob: (id: string) => Promise<void>
  fetchHistory: () => Promise<void>
}

type CronStore = CronState & CronActions

// ============================================================================
// Mock Data (replace with actual API calls when backend is ready)
// ============================================================================

const MOCK_JOBS: CronJob[] = [
  {
    id: "1",
    name: "Daily Backup",
    expression: "0 2 * * *",
    command: "backup-memory",
    enabled: true,
    nextRun: Date.now() + 3600000 * 8,
    lastRun: Date.now() - 3600000 * 16,
    lastStatus: "success",
  },
  {
    id: "2",
    name: "Weekly Report",
    expression: "0 9 * * 1",
    command: "generate-report",
    enabled: true,
    nextRun: Date.now() + 3600000 * 24 * 3,
    lastRun: Date.now() - 3600000 * 24 * 4,
    lastStatus: "success",
  },
  {
    id: "3",
    name: "Health Check",
    expression: "*/15 * * * *",
    command: "health-check --all",
    enabled: false,
    lastRun: Date.now() - 3600000,
    lastStatus: "failed",
    lastError: "Connection timeout",
  },
]

const MOCK_HISTORY: CronHistory[] = [
  {
    id: "h1",
    jobId: "1",
    jobName: "Daily Backup",
    startTime: Date.now() - 3600000 * 16,
    endTime: Date.now() - 3600000 * 16 + 45000,
    status: "success",
    output: "Backup completed: 256 files, 1.2GB",
  },
  {
    id: "h2",
    jobId: "3",
    jobName: "Health Check",
    startTime: Date.now() - 3600000,
    endTime: Date.now() - 3600000 + 30000,
    status: "failed",
    error: "Connection timeout",
  },
]

// ============================================================================
// Store
// ============================================================================

export const useCronStore = create<CronStore>((set) => ({
  // Initial State
  jobs: [],
  history: [],
  isLoading: false,
  isCreating: false,
  isRunning: null,
  error: null,

  // Actions
  fetchJobs: async () => {
    set({ isLoading: true, error: null })
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 500))
      set({ jobs: MOCK_JOBS, isLoading: false })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch cron jobs",
        isLoading: false,
      })
    }
  },

  createJob: async (jobData) => {
    set({ isCreating: true, error: null })
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 300))
      const newJob: CronJob = {
        ...jobData,
        id: `job-${Date.now()}`,
        nextRun: Date.now() + 3600000,
      }
      set((state) => ({
        jobs: [...state.jobs, newJob],
        isCreating: false,
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to create cron job",
        isCreating: false,
      })
    }
  },

  deleteJob: async (id) => {
    set({ error: null })
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 200))
      set((state) => ({
        jobs: state.jobs.filter((j) => j.id !== id),
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to delete cron job",
      })
    }
  },

  toggleJob: async (id) => {
    set({ error: null })
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 200))
      set((state) => ({
        jobs: state.jobs.map((j) =>
          j.id === id
            ? {
                ...j,
                enabled: !j.enabled,
                nextRun: !j.enabled ? Date.now() + 3600000 : undefined,
              }
            : j
        ),
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to toggle cron job",
      })
    }
  },

  runJob: async (id) => {
    set({ isRunning: id, error: null })
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 1500))
      set((state) => ({
        jobs: state.jobs.map((j) =>
          j.id === id
            ? {
                ...j,
                lastRun: Date.now(),
                lastStatus: "success" as const,
                lastError: undefined,
              }
            : j
        ),
        history: [
          {
            id: `h-${Date.now()}`,
            jobId: id,
            jobName: state.jobs.find((j) => j.id === id)?.name ?? "Unknown",
            startTime: Date.now() - 1500,
            endTime: Date.now(),
            status: "success" as const,
            output: "Manual execution completed",
          },
          ...state.history,
        ],
        isRunning: null,
      }))
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to run cron job",
        isRunning: null,
      })
    }
  },

  fetchHistory: async () => {
    try {
      // TODO: Replace with actual API call
      await new Promise((resolve) => setTimeout(resolve, 300))
      set({ history: MOCK_HISTORY })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch history",
      })
    }
  },
}))

// ============================================================================
// Hooks
// ============================================================================

export const useCronJobs = () => useCronStore(useShallow((state) => state.jobs))
export const useCronHistory = () => useCronStore(useShallow((state) => state.history))
export const useCronLoading = () =>
  useCronStore(
    useShallow((state) => ({
      isLoading: state.isLoading,
      isCreating: state.isCreating,
      isRunning: state.isRunning,
      error: state.error,
    }))
  )
export const useEnabledJobs = () => useCronStore((state) => state.jobs.filter((j) => j.enabled))
export const useCronCounts = () =>
  useCronStore(
    useShallow((state) => ({
      total: state.jobs.length,
      enabled: state.jobs.filter((j) => j.enabled).length,
      failed: state.jobs.filter((j) => j.lastStatus === "failed").length,
    }))
  )
