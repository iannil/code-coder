import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { renderHook, act, waitFor } from "@testing-library/react"
import {
  useAPI,
  useAPILazy,
  useAPIClient,
  useAPIWithConfig,
  isApiError,
  isNetworkError,
  isTimeoutError,
} from "@/hooks/use-api"
import { ApiError, NetworkError, TimeoutError } from "@/lib/api"
import * as React from "react"

// Mock the toast hook
const mockToast = vi.fn()
vi.mock("@/hooks/use-toast", () => ({
  useToast: () => ({ toast: mockToast }),
}))

// Mock the api module
vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual("@/lib/api")
  return {
    ...actual,
    getClient: vi.fn().mockReturnValue({
      listSessions: vi.fn(),
      getSession: vi.fn(),
    }),
    ApiClient: vi.fn().mockImplementation(function (this: any) {
      this.listSessions = vi.fn()
      this.getSession = vi.fn()
      return this
    }),
  }
})

describe("useAPI Hook", () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe("initial state", () => {
    it("should have isLoading false", () => {
      const { result } = renderHook(() => useAPI())
      expect(result.current.isLoading).toBe(false)
    })

    it("should have no error", () => {
      const { result } = renderHook(() => useAPI())
      expect(result.current.error).toBeNull()
    })

    it("should have api client", () => {
      const { result } = renderHook(() => useAPI())
      expect(result.current.api).toBeDefined()
    })

    it("should have isPending equal to isLoading", () => {
      const { result } = renderHook(() => useAPI())
      expect(result.current.isPending).toBe(result.current.isLoading)
    })
  })

  describe("execute", () => {
    it("should execute function and return result", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: false }))
      const mockFn = vi.fn().mockResolvedValue({ data: "test" })

      let executeResult: { data: string } | null = null
      await act(async () => {
        executeResult = await result.current.execute(mockFn)
      })

      expect(executeResult).toEqual({ data: "test" })
      expect(mockFn).toHaveBeenCalled()
    })

    it("should set loading state during execution", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: false }))

      // Start the execution without awaiting
      let promise: Promise<unknown>
      act(() => {
        promise = result.current.execute(async () => {
          await new Promise((resolve) => setTimeout(resolve, 10))
          return { data: "test" }
        })
      })

      // Loading should be true while executing
      expect(result.current.isLoading).toBe(true)

      // Wait for completion
      await act(async () => {
        await promise
      })

      expect(result.current.isLoading).toBe(false)
    })

    it("should handle errors and set error state", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: false }))
      const error = new Error("Test error")
      const mockFn = vi.fn().mockRejectedValue(error)

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(result.current.error).toBe(error)
    })

    it("should return null on error", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: false }))
      const mockFn = vi.fn().mockRejectedValue(new Error("Test error"))

      let executeResult: unknown
      await act(async () => {
        executeResult = await result.current.execute(mockFn)
      })

      expect(executeResult).toBeNull()
    })

    it("should show error toast when showErrorToasts is true", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: true }))
      const mockFn = vi.fn().mockRejectedValue(new Error("Test error"))

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({
          variant: "destructive",
        })
      )
    })

    it("should not show error toast when showErrorToasts is false", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: false }))
      const mockFn = vi.fn().mockRejectedValue(new Error("Test error"))

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(mockToast).not.toHaveBeenCalled()
    })

    it("should show success toast when showSuccessToasts is true", async () => {
      const { result } = renderHook(() => useAPI({ showSuccessToasts: true, showErrorToasts: false }))
      const mockFn = vi.fn().mockResolvedValue({ data: "test" })

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({
          title: "Success",
        })
      )
    })

    it("should call custom onError handler", async () => {
      const onError = vi.fn()
      const { result } = renderHook(() => useAPI({ onError, showErrorToasts: false }))
      const error = new Error("Test error")
      const mockFn = vi.fn().mockRejectedValue(error)

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(onError).toHaveBeenCalledWith(error)
    })

    it("should call custom onSuccess handler", async () => {
      const onSuccess = vi.fn()
      const { result } = renderHook(() => useAPI({ onSuccess, showErrorToasts: false }))
      const mockFn = vi.fn().mockResolvedValue({ data: "test" })

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(onSuccess).toHaveBeenCalled()
    })

    it("should call execute-level onError handler", async () => {
      const onError = vi.fn()
      const { result } = renderHook(() => useAPI({ showErrorToasts: false }))
      const error = new Error("Test error")
      const mockFn = vi.fn().mockRejectedValue(error)

      await act(async () => {
        await result.current.execute(mockFn, { onError })
      })

      expect(onError).toHaveBeenCalledWith(error)
    })

    it("should call execute-level onSuccess handler", async () => {
      const onSuccess = vi.fn()
      const { result } = renderHook(() => useAPI({ showErrorToasts: false }))
      const mockFn = vi.fn().mockResolvedValue({ data: "test" })

      await act(async () => {
        await result.current.execute(mockFn, { onSuccess })
      })

      expect(onSuccess).toHaveBeenCalled()
    })

    it("should use custom success message", async () => {
      const { result } = renderHook(() => useAPI({ showSuccessToasts: true, showErrorToasts: false }))
      const mockFn = vi.fn().mockResolvedValue({ data: "test" })

      await act(async () => {
        await result.current.execute(mockFn, { successMessage: "Custom success!" })
      })

      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({
          description: "Custom success!",
        })
      )
    })

    it("should handle non-Error exceptions", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: false }))
      const mockFn = vi.fn().mockRejectedValue("String error")

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(result.current.error?.message).toBe("Unknown error occurred")
    })
  })

  describe("clearError", () => {
    it("should clear error state", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: false }))
      const mockFn = vi.fn().mockRejectedValue(new Error("Test error"))

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(result.current.error).not.toBeNull()

      act(() => {
        result.current.clearError()
      })

      expect(result.current.error).toBeNull()
    })
  })

  describe("error title handling", () => {
    it("should show 'Bad Request' for 400 errors", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: true }))
      const error = new ApiError(400, "BAD_REQUEST", "Bad request")
      const mockFn = vi.fn().mockRejectedValue(error)

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({
          title: "Bad Request",
        })
      )
    })

    it("should show 'Unauthorized' for 401 errors", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: true }))
      const error = new ApiError(401, "UNAUTHORIZED", "Unauthorized")
      const mockFn = vi.fn().mockRejectedValue(error)

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({
          title: "Unauthorized",
        })
      )
    })

    it("should show 'Network Error' for NetworkError", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: true }))
      const error = new NetworkError("Network failed")
      const mockFn = vi.fn().mockRejectedValue(error)

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({
          title: "Network Error",
        })
      )
    })

    it("should show 'Request Timeout' for TimeoutError", async () => {
      const { result } = renderHook(() => useAPI({ showErrorToasts: true }))
      const error = new TimeoutError("Timeout")
      const mockFn = vi.fn().mockRejectedValue(error)

      await act(async () => {
        await result.current.execute(mockFn)
      })

      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({
          title: "Request Timeout",
        })
      )
    })
  })
})

describe("useAPILazy Hook", () => {
  it("should have executeLazy method", () => {
    const { result } = renderHook(() => useAPILazy())
    expect(result.current.executeLazy).toBeDefined()
  })

  it("should execute function via executeLazy", async () => {
    const { result } = renderHook(() => useAPILazy())
    const mockFn = vi.fn().mockResolvedValue({ data: "test" })

    let executeResult: { data: string } | null = null
    await act(async () => {
      executeResult = await result.current.executeLazy(mockFn)
    })

    expect(executeResult).toEqual({ data: "test" })
  })
})

describe("useAPIClient Hook", () => {
  it("should return api client", () => {
    const { result } = renderHook(() => useAPIClient())
    expect(result.current).toBeDefined()
  })
})

describe("useAPIWithConfig Hook", () => {
  it("should return useAPI with config", () => {
    const { result } = renderHook(() => useAPIWithConfig({ baseUrl: "http://localhost:8080" }))
    expect(result.current.api).toBeDefined()
    expect(result.current.execute).toBeDefined()
  })
})

describe("Type Guards", () => {
  describe("isApiError", () => {
    it("should return true for ApiError", () => {
      const error = new ApiError("Test", 400)
      expect(isApiError(error)).toBe(true)
    })

    it("should return false for regular Error", () => {
      const error = new Error("Test")
      expect(isApiError(error)).toBe(false)
    })
  })

  describe("isNetworkError", () => {
    it("should return true for NetworkError", () => {
      const error = new NetworkError("Test")
      expect(isNetworkError(error)).toBe(true)
    })

    it("should return false for regular Error", () => {
      const error = new Error("Test")
      expect(isNetworkError(error)).toBe(false)
    })
  })

  describe("isTimeoutError", () => {
    it("should return true for TimeoutError", () => {
      const error = new TimeoutError("Test")
      expect(isTimeoutError(error)).toBe(true)
    })

    it("should return false for regular Error", () => {
      const error = new Error("Test")
      expect(isTimeoutError(error)).toBe(false)
    })
  })
})
