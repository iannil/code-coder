import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { renderHook, act } from "@testing-library/react"
import {
  useLocalStorage,
  useLocalStorageBoolean,
  useLocalStorageNumber,
  useLocalStorageString,
  isLocalStorageAvailable,
  getLocalStorageItem,
  setLocalStorageItem,
  removeLocalStorageItem,
} from "@/hooks/use-local-storage"

describe("useLocalStorage", () => {
  let mockStorage: Record<string, string>

  beforeEach(() => {
    mockStorage = {}
    vi.mocked(window.localStorage.getItem).mockImplementation((key) => mockStorage[key] ?? null)
    vi.mocked(window.localStorage.setItem).mockImplementation((key, value) => {
      mockStorage[key] = value
    })
    vi.mocked(window.localStorage.removeItem).mockImplementation((key) => {
      delete mockStorage[key]
    })
    vi.mocked(window.localStorage.clear).mockImplementation(() => {
      mockStorage = {}
    })
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it("should return initial value when storage is empty", () => {
    const { result } = renderHook(() => useLocalStorage("test-key", "default"))
    expect(result.current[0]).toBe("default")
  })

  it("should return stored value when available", () => {
    mockStorage["ccode:test-key"] = JSON.stringify("stored value")
    const { result } = renderHook(() => useLocalStorage("test-key", "default"))
    expect(result.current[0]).toBe("stored value")
  })

  it("should set value in localStorage", () => {
    const { result } = renderHook(() => useLocalStorage("test-key", "default"))

    act(() => {
      result.current[1]("new value")
    })

    expect(result.current[0]).toBe("new value")
    expect(mockStorage["ccode:test-key"]).toBe(JSON.stringify("new value"))
  })

  it("should handle function updater", () => {
    const { result } = renderHook(() => useLocalStorage<number>("count", 0))

    act(() => {
      result.current[1]((prev) => prev + 1)
    })

    expect(result.current[0]).toBe(1)
  })

  it("should remove value from localStorage", () => {
    mockStorage["ccode:test-key"] = JSON.stringify("stored")
    const { result } = renderHook(() => useLocalStorage("test-key", "default"))

    act(() => {
      result.current[2]() // removeValue
    })

    expect(result.current[0]).toBe("default")
    expect(mockStorage["ccode:test-key"]).toBeUndefined()
  })

  it("should use custom prefix", () => {
    const { result } = renderHook(() =>
      useLocalStorage("key", "value", { prefix: "custom" })
    )

    act(() => {
      result.current[1]("new value")
    })

    expect(mockStorage["custom:key"]).toBe(JSON.stringify("new value"))
  })

  it("should call onUpdate callback", () => {
    const onUpdate = vi.fn()
    const { result } = renderHook(() =>
      useLocalStorage("key", "initial", { onUpdate })
    )

    act(() => {
      result.current[1]("updated")
    })

    expect(onUpdate).toHaveBeenCalledWith("updated")
  })

  it("should use custom serializer", () => {
    const serializer = vi.fn((v: string) => v.toUpperCase())
    const { result } = renderHook(() =>
      useLocalStorage("key", "value", { serializer })
    )

    act(() => {
      result.current[1]("test")
    })

    expect(serializer).toHaveBeenCalledWith("test")
    expect(mockStorage["ccode:key"]).toBe("TEST")
  })

  it("should use custom deserializer", () => {
    mockStorage["ccode:key"] = "STORED"
    const deserializer = vi.fn((v: string) => v.toLowerCase())

    const { result } = renderHook(() =>
      useLocalStorage("key", "default", { deserializer })
    )

    expect(result.current[0]).toBe("stored")
    expect(deserializer).toHaveBeenCalledWith("STORED")
  })

  it("should handle storage errors gracefully", () => {
    const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {})
    vi.mocked(window.localStorage.getItem).mockImplementation(() => {
      throw new Error("Storage error")
    })

    const { result } = renderHook(() => useLocalStorage("key", "default"))
    expect(result.current[0]).toBe("default")

    consoleSpy.mockRestore()
  })

  it("should work with objects", () => {
    const { result } = renderHook(() =>
      useLocalStorage<{ name: string }>("key", { name: "default" })
    )

    act(() => {
      result.current[1]({ name: "updated" })
    })

    expect(result.current[0]).toEqual({ name: "updated" })
  })

  it("should work with arrays", () => {
    const { result } = renderHook(() =>
      useLocalStorage<number[]>("key", [1, 2, 3])
    )

    act(() => {
      result.current[1]([4, 5, 6])
    })

    expect(result.current[0]).toEqual([4, 5, 6])
  })
})

describe("useLocalStorageBoolean", () => {
  let mockStorage: Record<string, string>

  beforeEach(() => {
    mockStorage = {}
    vi.mocked(window.localStorage.getItem).mockImplementation((key) => mockStorage[key] ?? null)
    vi.mocked(window.localStorage.setItem).mockImplementation((key, value) => {
      mockStorage[key] = value
    })
  })

  it("should return boolean values", () => {
    const { result } = renderHook(() => useLocalStorageBoolean("bool-key", false))

    act(() => {
      result.current[1](true)
    })

    expect(result.current[0]).toBe(true)
    expect(mockStorage["ccode:bool-key"]).toBe("true")
  })

  it("should deserialize boolean correctly", () => {
    mockStorage["ccode:bool-key"] = "true"
    const { result } = renderHook(() => useLocalStorageBoolean("bool-key", false))
    expect(result.current[0]).toBe(true)
  })

  it("should handle false string", () => {
    mockStorage["ccode:bool-key"] = "false"
    const { result } = renderHook(() => useLocalStorageBoolean("bool-key", true))
    expect(result.current[0]).toBe(false)
  })
})

describe("useLocalStorageNumber", () => {
  let mockStorage: Record<string, string>

  beforeEach(() => {
    mockStorage = {}
    vi.mocked(window.localStorage.getItem).mockImplementation((key) => mockStorage[key] ?? null)
    vi.mocked(window.localStorage.setItem).mockImplementation((key, value) => {
      mockStorage[key] = value
    })
  })

  it("should return number values", () => {
    const { result } = renderHook(() => useLocalStorageNumber("num-key", 0))

    act(() => {
      result.current[1](42)
    })

    expect(result.current[0]).toBe(42)
    expect(mockStorage["ccode:num-key"]).toBe("42")
  })

  it("should deserialize number correctly", () => {
    mockStorage["ccode:num-key"] = "123"
    const { result } = renderHook(() => useLocalStorageNumber("num-key", 0))
    expect(result.current[0]).toBe(123)
  })

  it("should handle floating point numbers", () => {
    const { result } = renderHook(() => useLocalStorageNumber("num-key", 0))

    act(() => {
      result.current[1](3.14)
    })

    expect(result.current[0]).toBe(3.14)
  })
})

describe("useLocalStorageString", () => {
  let mockStorage: Record<string, string>

  beforeEach(() => {
    mockStorage = {}
    vi.mocked(window.localStorage.getItem).mockImplementation((key) => mockStorage[key] ?? null)
    vi.mocked(window.localStorage.setItem).mockImplementation((key, value) => {
      mockStorage[key] = value
    })
  })

  it("should store string without JSON serialization", () => {
    const { result } = renderHook(() => useLocalStorageString("str-key", ""))

    act(() => {
      result.current[1]("hello world")
    })

    // Should be raw string, not JSON stringified
    expect(mockStorage["ccode:str-key"]).toBe("hello world")
  })
})

describe("isLocalStorageAvailable", () => {
  it("should return true when localStorage is available", () => {
    expect(isLocalStorageAvailable()).toBe(true)
  })
})

describe("getLocalStorageItem", () => {
  let mockStorage: Record<string, string>

  beforeEach(() => {
    mockStorage = {}
    vi.mocked(window.localStorage.getItem).mockImplementation((key) => mockStorage[key] ?? null)
  })

  it("should get item from localStorage", () => {
    mockStorage["ccode:key"] = JSON.stringify({ value: "test" })
    const result = getLocalStorageItem<{ value: string }>("key")
    expect(result).toEqual({ value: "test" })
  })

  it("should return null for non-existent key", () => {
    const result = getLocalStorageItem("non-existent")
    expect(result).toBeNull()
  })

  it("should use custom prefix", () => {
    mockStorage["custom:key"] = JSON.stringify("value")
    const result = getLocalStorageItem("key", "custom")
    expect(result).toBe("value")
  })
})

describe("setLocalStorageItem", () => {
  let mockStorage: Record<string, string>

  beforeEach(() => {
    mockStorage = {}
    vi.mocked(window.localStorage.setItem).mockImplementation((key, value) => {
      mockStorage[key] = value
    })
  })

  it("should set item in localStorage", () => {
    setLocalStorageItem("key", { value: "test" })
    expect(mockStorage["ccode:key"]).toBe(JSON.stringify({ value: "test" }))
  })

  it("should use custom prefix", () => {
    setLocalStorageItem("key", "value", "custom")
    expect(mockStorage["custom:key"]).toBe(JSON.stringify("value"))
  })
})

describe("removeLocalStorageItem", () => {
  let mockStorage: Record<string, string>

  beforeEach(() => {
    mockStorage = { "ccode:key": "value" }
    vi.mocked(window.localStorage.removeItem).mockImplementation((key) => {
      delete mockStorage[key]
    })
  })

  it("should remove item from localStorage", () => {
    removeLocalStorageItem("key")
    expect(mockStorage["ccode:key"]).toBeUndefined()
  })
})
