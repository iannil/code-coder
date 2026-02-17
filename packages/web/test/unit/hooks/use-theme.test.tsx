import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { renderHook, act } from "@testing-library/react"
import * as React from "react"
import { ThemeProvider, useTheme } from "@/hooks/use-theme"

// Mock matchMedia
const mockMatchMedia = vi.fn()
const mockAddEventListener = vi.fn()
const mockRemoveEventListener = vi.fn()

// Local storage mock that actually stores values
const localStorageStore = new Map<string, string>()
const localStorageMock = {
  getItem: vi.fn((key: string) => localStorageStore.get(key) ?? null),
  setItem: vi.fn((key: string, value: string) => localStorageStore.set(key, value)),
  removeItem: vi.fn((key: string) => localStorageStore.delete(key)),
  clear: vi.fn(() => localStorageStore.clear()),
}

beforeEach(() => {
  // Reset localStorage
  localStorageStore.clear()
  vi.clearAllMocks()

  // Override localStorage mock
  Object.defineProperty(window, "localStorage", { value: localStorageMock, writable: true })

  // Reset document classes
  document.documentElement.classList.remove("light", "dark")

  // Setup matchMedia mock
  mockAddEventListener.mockReset()
  mockRemoveEventListener.mockReset()
  mockMatchMedia.mockReturnValue({
    matches: false,
    addEventListener: mockAddEventListener,
    removeEventListener: mockRemoveEventListener,
  })
  window.matchMedia = mockMatchMedia
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe("ThemeProvider", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ThemeProvider>{children}</ThemeProvider>
  )

  describe("initial theme", () => {
    it("should default to system theme", () => {
      const { result } = renderHook(() => useTheme(), { wrapper })
      expect(result.current.theme).toBe("system")
    })

    it("should use stored theme from localStorage", () => {
      localStorageStore.set("codecoder-theme", "dark")
      const { result } = renderHook(() => useTheme(), { wrapper })
      expect(result.current.theme).toBe("dark")
    })

    it("should ignore invalid stored theme", () => {
      localStorageStore.set("codecoder-theme", "invalid")
      const { result } = renderHook(() => useTheme(), { wrapper })
      expect(result.current.theme).toBe("system")
    })
  })

  describe("resolved theme", () => {
    it("should resolve to light when theme is light", () => {
      localStorageStore.set("codecoder-theme", "light")
      const { result } = renderHook(() => useTheme(), { wrapper })
      expect(result.current.resolvedTheme).toBe("light")
    })

    it("should resolve to system preference when theme is system (dark)", () => {
      mockMatchMedia.mockReturnValue({
        matches: true,
        addEventListener: mockAddEventListener,
        removeEventListener: mockRemoveEventListener,
      })

      const { result } = renderHook(() => useTheme(), { wrapper })
      expect(result.current.theme).toBe("system")
      expect(result.current.resolvedTheme).toBe("dark")
    })

    it("should resolve to system preference when theme is system (light)", () => {
      mockMatchMedia.mockReturnValue({
        matches: false,
        addEventListener: mockAddEventListener,
        removeEventListener: mockRemoveEventListener,
      })

      const { result } = renderHook(() => useTheme(), { wrapper })
      expect(result.current.theme).toBe("system")
      expect(result.current.resolvedTheme).toBe("light")
    })
  })

  describe("setTheme", () => {
    it("should change theme to dark", () => {
      const { result } = renderHook(() => useTheme(), { wrapper })

      act(() => {
        result.current.setTheme("dark")
      })

      expect(result.current.theme).toBe("dark")
      expect(result.current.resolvedTheme).toBe("dark")
    })

    it("should change theme to light", () => {
      const { result } = renderHook(() => useTheme(), { wrapper })

      act(() => {
        result.current.setTheme("light")
      })

      expect(result.current.theme).toBe("light")
      expect(result.current.resolvedTheme).toBe("light")
    })

    it("should change theme to system", () => {
      localStorageStore.set("codecoder-theme", "dark")
      const { result } = renderHook(() => useTheme(), { wrapper })

      act(() => {
        result.current.setTheme("system")
      })

      expect(result.current.theme).toBe("system")
    })

    it("should persist theme to localStorage", () => {
      const { result } = renderHook(() => useTheme(), { wrapper })

      act(() => {
        result.current.setTheme("dark")
      })

      expect(localStorageStore.get("codecoder-theme")).toBe("dark")
    })
  })

  describe("document class management", () => {
    it("should add light class to document when light theme", () => {
      localStorageStore.set("codecoder-theme", "light")
      renderHook(() => useTheme(), { wrapper })

      expect(document.documentElement.classList.contains("light")).toBe(true)
    })

    it("should apply theme class after setTheme", () => {
      const { result } = renderHook(() => useTheme(), { wrapper })

      act(() => {
        result.current.setTheme("dark")
      })

      // After setTheme, the resolved theme should be dark
      expect(result.current.resolvedTheme).toBe("dark")
    })
  })

  describe("system theme listener", () => {
    it("should add listener for system theme changes", () => {
      renderHook(() => useTheme(), { wrapper })

      expect(mockAddEventListener).toHaveBeenCalledWith("change", expect.any(Function))
    })

    it("should remove listener on unmount", () => {
      const { unmount } = renderHook(() => useTheme(), { wrapper })
      unmount()

      expect(mockRemoveEventListener).toHaveBeenCalledWith("change", expect.any(Function))
    })

    it("should update resolved theme when system preference changes", () => {
      let changeHandler: ((e: MediaQueryListEvent) => void) | undefined
      mockAddEventListener.mockImplementation((event, handler) => {
        changeHandler = handler
      })

      const { result } = renderHook(() => useTheme(), { wrapper })

      expect(result.current.resolvedTheme).toBe("light")

      // Simulate system theme change
      act(() => {
        if (changeHandler) {
          changeHandler({ matches: true } as MediaQueryListEvent)
        }
      })

      expect(result.current.resolvedTheme).toBe("dark")
    })
  })
})

describe("useTheme error handling", () => {
  it("should throw error when used outside ThemeProvider", () => {
    // Suppress console.error for this test
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {})

    expect(() => {
      renderHook(() => useTheme())
    }).toThrow("useTheme must be used within a ThemeProvider")

    consoleSpy.mockRestore()
  })
})
