import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { renderHook, act } from "@testing-library/react"
import { useToast, toast, reducer } from "@/hooks/use-toast"

describe("Toast Reducer", () => {
  const initialState = { toasts: [] }

  describe("ADD_TOAST", () => {
    it("should add a toast", () => {
      const newToast = {
        id: "1",
        title: "Test Toast",
        description: "Test description",
        open: true,
      }

      const result = reducer(initialState, {
        type: "ADD_TOAST",
        toast: newToast,
      })

      expect(result.toasts).toHaveLength(1)
      expect(result.toasts[0]).toEqual(newToast)
    })

    it("should prepend new toast", () => {
      const existingToast = { id: "1", title: "First", open: true }
      const newToast = { id: "2", title: "Second", open: true }

      const result = reducer(
        { toasts: [existingToast] },
        { type: "ADD_TOAST", toast: newToast }
      )

      expect(result.toasts[0]).toEqual(newToast)
    })

    it("should limit toasts to TOAST_LIMIT", () => {
      const toast1 = { id: "1", title: "First", open: true }
      const toast2 = { id: "2", title: "Second", open: true }

      // TOAST_LIMIT is 1
      const result = reducer(
        { toasts: [toast1] },
        { type: "ADD_TOAST", toast: toast2 }
      )

      expect(result.toasts).toHaveLength(1)
      expect(result.toasts[0]).toEqual(toast2)
    })
  })

  describe("UPDATE_TOAST", () => {
    it("should update existing toast", () => {
      const existingToast = { id: "1", title: "Original", open: true }

      const result = reducer(
        { toasts: [existingToast] },
        { type: "UPDATE_TOAST", toast: { id: "1", title: "Updated" } }
      )

      expect(result.toasts[0].title).toBe("Updated")
      expect(result.toasts[0].open).toBe(true)
    })

    it("should not update non-existing toast", () => {
      const existingToast = { id: "1", title: "Original", open: true }

      const result = reducer(
        { toasts: [existingToast] },
        { type: "UPDATE_TOAST", toast: { id: "2", title: "Updated" } }
      )

      expect(result.toasts[0].title).toBe("Original")
    })
  })

  describe("DISMISS_TOAST", () => {
    it("should dismiss specific toast", () => {
      const toast1 = { id: "1", title: "First", open: true }

      const result = reducer(
        { toasts: [toast1] },
        { type: "DISMISS_TOAST", toastId: "1" }
      )

      expect(result.toasts[0].open).toBe(false)
    })

    it("should dismiss all toasts when no id provided", () => {
      const toast1 = { id: "1", title: "First", open: true }

      const result = reducer(
        { toasts: [toast1] },
        { type: "DISMISS_TOAST" }
      )

      expect(result.toasts.every((t) => t.open === false)).toBe(true)
    })
  })

  describe("REMOVE_TOAST", () => {
    it("should remove specific toast", () => {
      const toast1 = { id: "1", title: "First", open: true }
      const toast2 = { id: "2", title: "Second", open: true }

      const result = reducer(
        { toasts: [toast1, toast2] },
        { type: "REMOVE_TOAST", toastId: "1" }
      )

      expect(result.toasts).toHaveLength(1)
      expect(result.toasts[0].id).toBe("2")
    })

    it("should remove all toasts when no id provided", () => {
      const toast1 = { id: "1", title: "First", open: true }
      const toast2 = { id: "2", title: "Second", open: true }

      const result = reducer(
        { toasts: [toast1, toast2] },
        { type: "REMOVE_TOAST" }
      )

      expect(result.toasts).toHaveLength(0)
    })
  })
})

describe("toast function", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("should create a toast and return controls", () => {
    const result = toast({ title: "Test" })

    expect(result).toHaveProperty("id")
    expect(result).toHaveProperty("dismiss")
    expect(result).toHaveProperty("update")
  })

  it("should generate unique IDs", () => {
    const result1 = toast({ title: "Toast 1" })
    const result2 = toast({ title: "Toast 2" })

    expect(result1.id).not.toBe(result2.id)
  })

  it("should allow dismissing the toast", () => {
    const { result } = renderHook(() => useToast())

    let toastResult: ReturnType<typeof toast>
    act(() => {
      toastResult = result.current.toast({ title: "Test" })
    })

    expect(result.current.toasts.some((t) => t.open === true)).toBe(true)

    act(() => {
      toastResult!.dismiss()
    })

    expect(result.current.toasts.every((t) => t.open === false)).toBe(true)
  })

  it("should allow updating the toast", () => {
    const { result } = renderHook(() => useToast())

    let toastResult: ReturnType<typeof toast>
    act(() => {
      toastResult = result.current.toast({ title: "Original" })
    })

    expect(result.current.toasts[0].title).toBe("Original")

    act(() => {
      toastResult!.update({ id: toastResult!.id, title: "Updated" })
    })

    expect(result.current.toasts[0].title).toBe("Updated")
  })
})

describe("useToast hook", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("should return toasts array", () => {
    const { result } = renderHook(() => useToast())
    expect(result.current.toasts).toBeDefined()
    expect(Array.isArray(result.current.toasts)).toBe(true)
  })

  it("should return toast function", () => {
    const { result } = renderHook(() => useToast())
    expect(result.current.toast).toBeDefined()
    expect(typeof result.current.toast).toBe("function")
  })

  it("should return dismiss function", () => {
    const { result } = renderHook(() => useToast())
    expect(result.current.dismiss).toBeDefined()
    expect(typeof result.current.dismiss).toBe("function")
  })

  it("should add toast when toast() is called", () => {
    const { result } = renderHook(() => useToast())

    act(() => {
      result.current.toast({ title: "Test Toast" })
    })

    expect(result.current.toasts).toHaveLength(1)
    expect(result.current.toasts[0].title).toBe("Test Toast")
  })

  it("should dismiss toast when dismiss() is called", () => {
    const { result } = renderHook(() => useToast())

    act(() => {
      result.current.toast({ title: "Test Toast" })
    })

    const toastId = result.current.toasts[0].id

    act(() => {
      result.current.dismiss(toastId)
    })

    expect(result.current.toasts[0].open).toBe(false)
  })

  it("should dismiss all toasts when dismiss() is called without id", () => {
    const { result } = renderHook(() => useToast())

    act(() => {
      result.current.toast({ title: "Toast 1" })
    })

    act(() => {
      result.current.dismiss()
    })

    expect(result.current.toasts.every((t) => t.open === false)).toBe(true)
  })

  it("should sync state between multiple hook instances", () => {
    const { result: result1 } = renderHook(() => useToast())
    const { result: result2 } = renderHook(() => useToast())

    act(() => {
      result1.current.toast({ title: "Shared Toast" })
    })

    expect(result2.current.toasts).toHaveLength(1)
    expect(result2.current.toasts[0].title).toBe("Shared Toast")
  })

  it("should cleanup listener on unmount", () => {
    const { result, unmount } = renderHook(() => useToast())

    act(() => {
      result.current.toast({ title: "Test" })
    })

    unmount()

    // After unmount, adding a toast shouldn't cause errors
    expect(() => {
      act(() => {
        toast({ title: "After unmount" })
      })
    }).not.toThrow()
  })
})

describe("Toast with variant", () => {
  it("should accept variant prop", () => {
    const { result } = renderHook(() => useToast())

    act(() => {
      result.current.toast({
        title: "Error",
        description: "Something went wrong",
        variant: "destructive",
      })
    })

    expect(result.current.toasts[0].variant).toBe("destructive")
  })

  it("should accept action prop", () => {
    const { result } = renderHook(() => useToast())
    const mockAction = { altText: "Undo", onClick: vi.fn() }

    act(() => {
      result.current.toast({
        title: "Action Toast",
        action: mockAction as any,
      })
    })

    expect(result.current.toasts[0].action).toBeDefined()
  })
})
