import "@testing-library/jest-dom"
import { afterEach, vi } from "vitest"
import { cleanup } from "@testing-library/react"

// Cleanup after each test
afterEach(() => {
  cleanup()
})

// Mock window.matchMedia
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
}
Object.defineProperty(window, "localStorage", { value: localStorageMock })

// Mock fetch
global.fetch = vi.fn()

// Mock EventSource
class MockEventSource {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSED = 2

  readyState = MockEventSource.OPEN
  url: string
  onopen: ((event: Event) => void) | null = null
  onerror: ((event: Event) => void) | null = null
  onmessage: ((event: MessageEvent) => void) | null = null

  private listeners: Map<string, ((event: Event) => void)[]> = new Map()

  constructor(url: string) {
    this.url = url
    // Simulate open event
    setTimeout(() => {
      if (this.onopen) {
        this.onopen(new Event("open"))
      }
    }, 0)
  }

  addEventListener(type: string, listener: (event: Event) => void) {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, [])
    }
    this.listeners.get(type)!.push(listener)
  }

  removeEventListener(type: string, listener: (event: Event) => void) {
    const listeners = this.listeners.get(type)
    if (listeners) {
      const index = listeners.indexOf(listener)
      if (index !== -1) {
        listeners.splice(index, 1)
      }
    }
  }

  close() {
    this.readyState = MockEventSource.CLOSED
  }

  // Test helper to simulate events
  simulateMessage(data: unknown, type = "message") {
    const event = new MessageEvent(type, {
      data: JSON.stringify(data),
    })
    if (type === "message" && this.onmessage) {
      this.onmessage(event)
    }
    const listeners = this.listeners.get(type) || []
    for (const listener of listeners) {
      listener(event)
    }
  }

  simulateError() {
    this.readyState = MockEventSource.CLOSED
    if (this.onerror) {
      this.onerror(new Event("error"))
    }
  }
}

Object.defineProperty(global, "EventSource", {
  value: MockEventSource,
  writable: true,
})
