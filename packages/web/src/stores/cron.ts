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
import { api } from "@/lib/api"
import type { SchedulerTask, SchedulerTaskCommand, SchedulerExecutionHistory } from "@/lib/types"

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
  updateJob: (id: string, updates: Partial<Pick<CronJob, "name" | "expression" | "command" | "enabled">>) => Promise<void>
  deleteJob: (id: string) => Promise<void>
  toggleJob: (id: string) => Promise<void>
  runJob: (id: string) => Promise<void>
  fetchHistory: () => Promise<void>
}

type CronStore = CronState & CronActions

// ============================================================================
// Transform Helpers
// ============================================================================

/**
 * Transform SchedulerTask to CronJob format
 */
function toCronJob(task: SchedulerTask): CronJob {
  const command =
    task.command.type === "shell"
      ? task.command.command
      : task.command.type === "agent"
        ? `agent:${task.command.agentName}`
        : JSON.stringify(task.command)

  return {
    id: task.id,
    name: task.name || task.id,
    expression: task.expression,
    command,
    enabled: task.enabled,
    nextRun: task.nextRun ? new Date(task.nextRun).getTime() : undefined,
    lastRun: task.lastRun ? new Date(task.lastRun).getTime() : undefined,
    lastStatus: task.lastStatus === "ok" ? "success" : task.lastStatus === "error" ? "failed" : undefined,
    lastError: task.lastOutput,
  }
}

/**
 * Transform shell command string to SchedulerTaskCommand
 */
function toTaskCommand(command: string): SchedulerTaskCommand {
  // Check if it's an agent command (agent:agentName format)
  if (command.startsWith("agent:")) {
    return {
      type: "agent",
      agentName: command.slice(6),
      prompt: "",
    }
  }

  // Try to parse as JSON for complex commands
  try {
    const parsed = JSON.parse(command)
    if (parsed.type && ["agent", "api", "channel_message"].includes(parsed.type)) {
      return parsed as SchedulerTaskCommand
    }
  } catch {
    // Not JSON, treat as shell command
  }

  return {
    type: "shell",
    command,
  }
}

/**
 * Transform SchedulerExecutionHistory to CronHistory format
 */
function toCronHistory(exec: SchedulerExecutionHistory, jobs: CronJob[]): CronHistory {
  const job = jobs.find((j) => j.id === exec.taskId)

  return {
    id: exec.id,
    jobId: exec.taskId,
    jobName: job?.name || exec.taskId,
    startTime: new Date(exec.startedAt).getTime(),
    endTime: exec.endedAt ? new Date(exec.endedAt).getTime() : undefined,
    status: exec.status === "ok" ? "success" : exec.status === "error" ? "failed" : "running",
    output: exec.output,
    error: exec.error,
  }
}

// ============================================================================
// Store
// ============================================================================

export const useCronStore = create<CronStore>((set, get) => ({
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
      const tasks = await api.listSchedulerTasks()
      const jobs = tasks.map(toCronJob)
      set({ jobs, isLoading: false })
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
      const taskId = `task-${Date.now()}`
      await api.createSchedulerTask({
        id: taskId,
        name: jobData.name,
        expression: jobData.expression,
        command: toTaskCommand(jobData.command),
        enabled: jobData.enabled,
      })

      // Refresh jobs list after creation
      const tasks = await api.listSchedulerTasks()
      const jobs = tasks.map(toCronJob)
      set({ jobs, isCreating: false })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to create cron job",
        isCreating: false,
      })
    }
  },

  updateJob: async (id, updates) => {
    set({ error: null })
    try {
      await api.updateSchedulerTask(id, {
        name: updates.name,
        expression: updates.expression,
        command: updates.command ? toTaskCommand(updates.command) : undefined,
        enabled: updates.enabled,
      })

      // Refresh jobs list after update
      const tasks = await api.listSchedulerTasks()
      const jobs = tasks.map(toCronJob)
      set({ jobs })
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to update cron job",
      })
    }
  },

  deleteJob: async (id) => {
    set({ error: null })
    try {
      await api.deleteSchedulerTask(id)
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
    const job = get().jobs.find((j) => j.id === id)
    if (!job) return

    try {
      await api.updateSchedulerTask(id, {
        enabled: !job.enabled,
      })

      // Optimistic update
      set((state) => ({
        jobs: state.jobs.map((j) =>
          j.id === id
            ? {
                ...j,
                enabled: !j.enabled,
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
      const result = await api.runSchedulerTask(id)

      // Update job with execution result
      set((state) => ({
        jobs: state.jobs.map((j) =>
          j.id === id
            ? {
                ...j,
                lastRun: Date.now(),
                lastStatus: result.status === "ok" ? ("success" as const) : ("failed" as const),
                lastError: result.error,
              }
            : j
        ),
        history: [
          {
            id: result.executionId,
            jobId: id,
            jobName: state.jobs.find((j) => j.id === id)?.name ?? "Unknown",
            startTime: Date.now() - 1000, // Approximate
            endTime: Date.now(),
            status: result.status === "ok" ? ("success" as const) : ("failed" as const),
            output: result.output,
            error: result.error,
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
      const executions = await api.getSchedulerHistory({ limit: 50 })
      const jobs = get().jobs
      const history = executions.map((exec) => toCronHistory(exec, jobs))
      set({ history })
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
