import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import {
  useCronStore,
  useCronJobs,
  useCronHistory,
  useCronLoading,
  useCronCounts,
} from "@/stores/cron"
import type { CronJob, CronHistory } from "@/lib/types"
import { renderHook } from "@testing-library/react"

const mockJob1: CronJob = {
  id: "1",
  name: "Daily Backup",
  expression: "0 2 * * *",
  command: "backup-memory",
  enabled: true,
  nextRun: Date.now() + 3600000,
  lastRun: Date.now() - 3600000,
  lastStatus: "success",
}

const mockJob2: CronJob = {
  id: "2",
  name: "Weekly Report",
  expression: "0 9 * * 1",
  command: "generate-report",
  enabled: false,
  lastStatus: "failed",
  lastError: "Connection timeout",
}

const mockHistory: CronHistory[] = [
  {
    id: "h1",
    jobId: "1",
    jobName: "Daily Backup",
    startTime: Date.now() - 3600000,
    endTime: Date.now() - 3600000 + 45000,
    status: "success",
    output: "Backup completed",
  },
]

describe("Cron Store", () => {
  beforeEach(() => {
    // Reset the store before each test
    useCronStore.setState({
      jobs: [],
      history: [],
      isLoading: false,
      isCreating: false,
      isRunning: null,
      error: null,
    })
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe("initial state", () => {
    it("should have empty jobs array", () => {
      const state = useCronStore.getState()
      expect(state.jobs).toEqual([])
    })

    it("should have empty history array", () => {
      const state = useCronStore.getState()
      expect(state.history).toEqual([])
    })

    it("should have isLoading false", () => {
      const state = useCronStore.getState()
      expect(state.isLoading).toBe(false)
    })

    it("should have isCreating false", () => {
      const state = useCronStore.getState()
      expect(state.isCreating).toBe(false)
    })

    it("should have isRunning null", () => {
      const state = useCronStore.getState()
      expect(state.isRunning).toBeNull()
    })

    it("should have null error", () => {
      const state = useCronStore.getState()
      expect(state.error).toBeNull()
    })
  })

  describe("fetchJobs", () => {
    it("should set isLoading during fetch", async () => {
      const fetchPromise = useCronStore.getState().fetchJobs()

      expect(useCronStore.getState().isLoading).toBe(true)
      expect(useCronStore.getState().error).toBeNull()

      await vi.advanceTimersByTimeAsync(500)
      await fetchPromise

      expect(useCronStore.getState().isLoading).toBe(false)
    })

    it("should load mock jobs", async () => {
      const fetchPromise = useCronStore.getState().fetchJobs()
      await vi.advanceTimersByTimeAsync(500)
      await fetchPromise

      const state = useCronStore.getState()
      expect(state.jobs.length).toBeGreaterThan(0)
    })
  })

  describe("createJob", () => {
    it("should set isCreating during creation", async () => {
      const createPromise = useCronStore.getState().createJob({
        name: "New Job",
        expression: "0 * * * *",
        command: "test-command",
        enabled: true,
      })

      expect(useCronStore.getState().isCreating).toBe(true)

      await vi.advanceTimersByTimeAsync(300)
      await createPromise

      expect(useCronStore.getState().isCreating).toBe(false)
    })

    it("should add new job to list", async () => {
      const createPromise = useCronStore.getState().createJob({
        name: "New Job",
        expression: "0 * * * *",
        command: "test-command",
        enabled: true,
      })
      await vi.advanceTimersByTimeAsync(300)
      await createPromise

      const state = useCronStore.getState()
      expect(state.jobs).toHaveLength(1)
      expect(state.jobs[0].name).toBe("New Job")
      expect(state.jobs[0].id).toBeDefined()
    })
  })

  describe("deleteJob", () => {
    it("should remove job from list", async () => {
      useCronStore.setState({ jobs: [mockJob1, mockJob2] })

      const deletePromise = useCronStore.getState().deleteJob("1")
      await vi.advanceTimersByTimeAsync(200)
      await deletePromise

      const state = useCronStore.getState()
      expect(state.jobs).toHaveLength(1)
      expect(state.jobs[0].id).toBe("2")
    })
  })

  describe("toggleJob", () => {
    it("should toggle job enabled state", async () => {
      useCronStore.setState({ jobs: [mockJob1] })

      const togglePromise = useCronStore.getState().toggleJob("1")
      await vi.advanceTimersByTimeAsync(200)
      await togglePromise

      expect(useCronStore.getState().jobs[0].enabled).toBe(false)
    })

    it("should set nextRun when enabling", async () => {
      useCronStore.setState({ jobs: [mockJob2] })

      const togglePromise = useCronStore.getState().toggleJob("2")
      await vi.advanceTimersByTimeAsync(200)
      await togglePromise

      const job = useCronStore.getState().jobs[0]
      expect(job.enabled).toBe(true)
      expect(job.nextRun).toBeDefined()
    })
  })

  describe("runJob", () => {
    it("should set isRunning during execution", async () => {
      useCronStore.setState({ jobs: [mockJob1] })

      const runPromise = useCronStore.getState().runJob("1")

      expect(useCronStore.getState().isRunning).toBe("1")

      await vi.advanceTimersByTimeAsync(1500)
      await runPromise

      expect(useCronStore.getState().isRunning).toBeNull()
    })

    it("should update job lastRun and add to history", async () => {
      useCronStore.setState({ jobs: [mockJob1] })

      const runPromise = useCronStore.getState().runJob("1")
      await vi.advanceTimersByTimeAsync(1500)
      await runPromise

      const state = useCronStore.getState()
      expect(state.jobs[0].lastStatus).toBe("success")
      expect(state.history).toHaveLength(1)
      expect(state.history[0].jobName).toBe("Daily Backup")
    })
  })

  describe("fetchHistory", () => {
    it("should load mock history", async () => {
      const fetchPromise = useCronStore.getState().fetchHistory()
      await vi.advanceTimersByTimeAsync(300)
      await fetchPromise

      expect(useCronStore.getState().history.length).toBeGreaterThan(0)
    })
  })

  describe("hooks", () => {
    it("useCronJobs should return jobs", () => {
      useCronStore.setState({ jobs: [mockJob1, mockJob2] })

      const { result } = renderHook(() => useCronJobs())
      expect(result.current).toHaveLength(2)
    })

    it("useCronHistory should return history", () => {
      useCronStore.setState({ history: mockHistory })

      const { result } = renderHook(() => useCronHistory())
      expect(result.current).toHaveLength(1)
    })

    it("useCronLoading should return loading states", () => {
      useCronStore.setState({
        isLoading: true,
        isCreating: false,
        isRunning: "1",
        error: "Test error",
      })

      const { result } = renderHook(() => useCronLoading())
      expect(result.current.isLoading).toBe(true)
      expect(result.current.isCreating).toBe(false)
      expect(result.current.isRunning).toBe("1")
      expect(result.current.error).toBe("Test error")
    })

    it("useCronCounts should return counts", () => {
      useCronStore.setState({ jobs: [mockJob1, mockJob2] })

      const { result } = renderHook(() => useCronCounts())
      expect(result.current.total).toBe(2)
      expect(result.current.enabled).toBe(1)
      expect(result.current.failed).toBe(1)
    })
  })
})
