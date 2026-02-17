import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { renderHook, act } from "@testing-library/react"
import { useSSEStatus, useSSEMessages } from "@/hooks/use-sse"
import { useSSEStore } from "@/stores/sse"

describe("useSSEStatus Hook", () => {
  beforeEach(() => {
    useSSEStore.setState({
      connectionState: "disconnected",
      error: null,
    })
  })

  it("should return connection status", () => {
    const { result } = renderHook(() => useSSEStatus())

    expect(result.current.isConnected).toBe(false)
    expect(result.current.isConnecting).toBe(false)
    expect(result.current.error).toBeNull()
  })

  it("should update when connected", () => {
    const { result } = renderHook(() => useSSEStatus())

    act(() => {
      useSSEStore.setState({ connectionState: "connected" })
    })

    expect(result.current.isConnected).toBe(true)
  })

  it("should update when connecting", () => {
    const { result } = renderHook(() => useSSEStatus())

    act(() => {
      useSSEStore.setState({ connectionState: "connecting" })
    })

    expect(result.current.isConnecting).toBe(true)
  })

  it("should update when reconnecting", () => {
    const { result } = renderHook(() => useSSEStatus())

    act(() => {
      useSSEStore.setState({ connectionState: "reconnecting" })
    })

    expect(result.current.isConnecting).toBe(true)
  })

  it("should update when error occurs", () => {
    const { result } = renderHook(() => useSSEStatus())

    act(() => {
      useSSEStore.setState({ error: "Connection error" })
    })

    expect(result.current.error).toBe("Connection error")
  })
})

describe("useSSEMessages Hook", () => {
  beforeEach(() => {
    useSSEStore.setState({
      events: [],
    })
  })

  it("should return events and clearEvents", () => {
    const { result } = renderHook(() => useSSEMessages())

    expect(result.current.events).toEqual([])
    expect(result.current.clearEvents).toBeDefined()
  })

  it("should return events from store", () => {
    const events = [
      { type: "message", data: { content: "test" }, timestamp: Date.now() },
    ]
    useSSEStore.setState({ events })

    const { result } = renderHook(() => useSSEMessages())

    expect(result.current.events).toHaveLength(1)
  })

  it("should clear events when clearEvents is called", () => {
    useSSEStore.setState({
      events: [{ type: "message", data: {}, timestamp: Date.now() }],
    })

    const { result } = renderHook(() => useSSEMessages())

    act(() => {
      result.current.clearEvents()
    })

    expect(useSSEStore.getState().events).toEqual([])
  })
})
