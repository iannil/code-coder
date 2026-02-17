import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { renderHook, act, waitFor } from "@testing-library/react"
import { useDebounce, useDebouncedCallback, useDebounceValue } from "@/hooks/use-debounce"

describe("useDebounce", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("should return initial value immediately", () => {
    const { result } = renderHook(() => useDebounce("initial", 500))
    expect(result.current).toBe("initial")
  })

  it("should debounce value changes", async () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 500),
      { initialProps: { value: "initial" } }
    )

    expect(result.current).toBe("initial")

    // Change the value
    rerender({ value: "updated" })

    // Should still be initial immediately after change
    expect(result.current).toBe("initial")

    // Advance time by 500ms
    await act(async () => {
      vi.advanceTimersByTime(500)
    })

    // Now should be updated
    expect(result.current).toBe("updated")
  })

  it("should cancel pending debounce on new value", async () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 500),
      { initialProps: { value: "first" } }
    )

    // Change value
    rerender({ value: "second" })

    // Advance part way
    await act(async () => {
      vi.advanceTimersByTime(250)
    })

    // Change again before debounce fires
    rerender({ value: "third" })

    // Advance past original debounce time
    await act(async () => {
      vi.advanceTimersByTime(250)
    })

    // Should still be "first" since second change reset timer
    expect(result.current).toBe("first")

    // Complete the debounce
    await act(async () => {
      vi.advanceTimersByTime(250)
    })

    // Now should be "third"
    expect(result.current).toBe("third")
  })

  it("should use default delay of 500ms", async () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value),
      { initialProps: { value: "initial" } }
    )

    rerender({ value: "updated" })

    await act(async () => {
      vi.advanceTimersByTime(499)
    })
    expect(result.current).toBe("initial")

    await act(async () => {
      vi.advanceTimersByTime(1)
    })
    expect(result.current).toBe("updated")
  })

  it("should work with objects", async () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 500),
      { initialProps: { value: { count: 0 } } }
    )

    rerender({ value: { count: 1 } })

    await act(async () => {
      vi.advanceTimersByTime(500)
    })

    expect(result.current).toEqual({ count: 1 })
  })

  it("should work with arrays", async () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 500),
      { initialProps: { value: [1, 2, 3] } }
    )

    rerender({ value: [4, 5, 6] })

    await act(async () => {
      vi.advanceTimersByTime(500)
    })

    expect(result.current).toEqual([4, 5, 6])
  })
})

describe("useDebouncedCallback", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("should debounce callback execution", async () => {
    const callback = vi.fn()
    const { result } = renderHook(() => useDebouncedCallback(callback, 500))

    // Call run multiple times
    act(() => {
      result.current.run()
      result.current.run()
      result.current.run()
    })

    // Callback should not be called yet
    expect(callback).not.toHaveBeenCalled()

    // Advance time
    await act(async () => {
      vi.advanceTimersByTime(500)
    })

    // Should be called once
    expect(callback).toHaveBeenCalledTimes(1)
  })

  it("should cancel pending callback", async () => {
    const callback = vi.fn()
    const { result } = renderHook(() => useDebouncedCallback(callback, 500))

    act(() => {
      result.current.run()
    })

    // Cancel before it fires
    act(() => {
      result.current.cancel()
    })

    await act(async () => {
      vi.advanceTimersByTime(500)
    })

    // Should not be called
    expect(callback).not.toHaveBeenCalled()
  })

  it("should flush pending callback immediately", async () => {
    const callback = vi.fn()
    const { result } = renderHook(() => useDebouncedCallback(callback, 500))

    act(() => {
      result.current.run()
    })

    // Flush before timer
    act(() => {
      result.current.flush()
    })

    // Should be called immediately
    expect(callback).toHaveBeenCalledTimes(1)
  })

  it("should not call callback on flush if not pending", () => {
    const callback = vi.fn()
    const { result } = renderHook(() => useDebouncedCallback(callback, 500))

    // Flush without calling run
    act(() => {
      result.current.flush()
    })

    expect(callback).not.toHaveBeenCalled()
  })

  it("should use default delay", async () => {
    const callback = vi.fn()
    const { result } = renderHook(() => useDebouncedCallback(callback))

    act(() => {
      result.current.run()
    })

    await act(async () => {
      vi.advanceTimersByTime(499)
    })
    expect(callback).not.toHaveBeenCalled()

    await act(async () => {
      vi.advanceTimersByTime(1)
    })
    expect(callback).toHaveBeenCalledTimes(1)
  })
})

describe("useDebounceValue", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("should return initial value", () => {
    const { result } = renderHook(() => useDebounceValue("initial", 500))
    expect(result.current.value).toBe("initial")
    expect(result.current.isPending).toBe(false)
  })

  it("should flush value immediately", () => {
    const { result } = renderHook(() => useDebounceValue("initial", 500))

    act(() => {
      result.current.flush("flushed")
    })

    expect(result.current.value).toBe("flushed")
    expect(result.current.isPending).toBe(false)
  })

  it("should cancel pending value", () => {
    const { result } = renderHook(() => useDebounceValue("initial", 500))

    // Note: We need to trigger a pending state first
    // The hook doesn't have a direct "set" method for pending values
    // This tests that cancel works without errors
    act(() => {
      result.current.cancel()
    })

    expect(result.current.value).toBe("initial")
    expect(result.current.isPending).toBe(false)
  })
})
