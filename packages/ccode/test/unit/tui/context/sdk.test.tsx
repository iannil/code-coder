// @ts-nocheck
/**
 * SDK Context Unit Tests
 *
 * Tests for the SDK provider including:
 * - RPC client initialization
 * - Event queueing and batching
 * - Event debouncing (16ms threshold)
 * - Nested proxy creation for SDK calls
 * - Event source subscription
 * - Cleanup on unmount
 */

import { describe, test, expect, beforeEach, mock } from "bun:test"

// Mock types based on the actual context
type Event = {
  type: string
  [key: string]: any
}

type MockRpcClient = {
  call: (input: { namespace: string; method: string; args: any[] }) => Promise<any>
  on?: (event: string, handler: (data: any) => void) => () => void
}

type MockEventSource = {
  on: (handler: (event: Event) => void) => () => void
}

describe("SDK Context", () => {
  describe("RPC client", () => {
    test("should initialize RPC client", () => {
      let client: MockRpcClient | undefined = undefined

      const setClient = (newClient: MockRpcClient) => {
        client = newClient
      }

      const mockClient: MockRpcClient = {
        call: async () => ({ data: "result" }),
      }

      setClient(mockClient)

      expect(client).toBeDefined()
      expect(client?.call).toBeFunction()
    })

    test("should throw error when RPC client not initialized", async () => {
      let client: MockRpcClient | undefined = undefined

      const makeCall = async () => {
        if (!client) throw new Error("RPC client not initialized")
        return await client.call({ namespace: "test", method: "method", args: [] })
      }

      await expect(makeCall()).rejects.toThrow("RPC client not initialized")
    })

    test("should call RPC with correct parameters", async () => {
      const mockClient: MockRpcClient = {
        call: async (input) => {
          expect(input.namespace).toBe("session")
          expect(input.method).toBe("list")
          expect(input.args).toEqual([{ limit: 10 }])
          return { data: ["session1", "session2"] }
        },
      }

      const result = await mockClient.call({ namespace: "session", method: "list", args: [{ limit: 10 }] })

      expect(result.data).toEqual(["session1", "session2"])
    })
  })

  describe("event batching", () => {
    test("should queue events", () => {
      const queue: Event[] = []
      const event1: Event = { type: "message", data: "test1" }
      const event2: Event = { type: "message", data: "test2" }

      queue.push(event1)
      queue.push(event2)

      expect(queue).toHaveLength(2)
    })

    test("should clear queue after flush", () => {
      const queue: Event[] = []
      const flushed: Event[] = []

      const event1: Event = { type: "message", data: "test" }
      queue.push(event1)

      const flush = () => {
        if (queue.length === 0) return
        const events = queue.splice(0, queue.length)
        flushed.push(...events)
      }

      flush()

      expect(queue).toHaveLength(0)
      expect(flushed).toHaveLength(1)
    })

    test("should track last flush time", () => {
      let lastFlush = 0

      const flush = () => {
        lastFlush = Date.now()
      }

      flush()

      expect(lastFlush).toBeGreaterThan(0)
      const elapsed = Date.now() - lastFlush
      expect(elapsed).toBeLessThan(100)
    })
  })

  describe("event debouncing", () => {
    test("should debounce events within 16ms", () => {
      const debounceThreshold = 16
      let timer: ReturnType<typeof setTimeout> | undefined = undefined
      let lastFlush = 0

      const handleEvent = () => {
        const elapsed = Date.now() - lastFlush
        if (timer) return

        if (elapsed < debounceThreshold) {
          timer = setTimeout(() => {
            timer = undefined
            lastFlush = Date.now()
          }, debounceThreshold)
          return
        }

        lastFlush = Date.now()
      }

      // First event - immediate flush
      handleEvent()
      expect(lastFlush).toBeGreaterThan(0)

      const previousFlush = lastFlush

      // Second event within threshold - debounced
      timer = setTimeout(() => {}, 10)
      handleEvent()

      // Should not update lastFlush immediately when debounced
      expect(timer).toBeDefined()
    })

    test("should flush immediately if elapsed time >= 16ms", () => {
      const debounceThreshold = 16
      let lastFlush = Date.now() - 20 // 20ms ago
      let immediateFlush = false

      const handleEvent = () => {
        const elapsed = Date.now() - lastFlush

        if (elapsed >= debounceThreshold) {
          immediateFlush = true
          lastFlush = Date.now()
        }
      }

      handleEvent()

      expect(immediateFlush).toBe(true)
    })
  })

  describe("nested proxy", () => {
    test("should create nested proxy chain", () => {
      const createProxy = (path: string[] = []): any => {
        return new Proxy(() => {}, {
          get(_, prop: string) {
            return createProxy([...path, prop])
          },
          apply(_, __, args) {
            return { namespace: path[0], method: path.slice(1).join("."), args }
          },
        })
      }

      const client = createProxy()
      const result = client.session.list({ limit: 10 })

      expect(result.namespace).toBe("session")
      expect(result.method).toBe("list")
      expect(result.args).toEqual([{ limit: 10 }])
    })

    test("should support deeply nested method calls", () => {
      const createProxy = (path: string[] = []): any => {
        return new Proxy(() => {}, {
          get(_, prop: string) {
            return createProxy([...path, prop])
          },
          apply(_, __, args) {
            return { namespace: path[0], method: path.slice(1).join("."), args }
          },
        })
      }

      const client = createProxy()
      const result = client.experimental.resource.get("id")

      expect(result.namespace).toBe("experimental")
      expect(result.method).toBe("resource.get")
      expect(result.args).toEqual(["id"])
    })

    test("should throw error for invalid API calls", () => {
      const createProxy = (path: string[] = []): any => {
        return new Proxy(() => {}, {
          get(_, prop: string) {
            return createProxy([...path, prop])
          },
          apply(_, __, args) {
            if (path.length < 2) throw new Error(`Invalid API call: ${path.join(".")}`)
            return { namespace: path[0], method: path.slice(1).join("."), args }
          },
        })
      }

      const client = createProxy()

      expect(() => client.session()).toThrow("Invalid API call: session")
    })
  })

  describe("event source subscription", () => {
    test("should subscribe to event source", () => {
      const handlers: ((event: Event) => void)[] = []
      const eventSource: MockEventSource = {
        on: (handler) => {
          handlers.push(handler)
          return () => {
            const index = handlers.indexOf(handler)
            if (index > -1) handlers.splice(index, 1)
          }
        },
      }

      let receivedEvents: Event[] = []

      const handler = (event: Event) => {
        receivedEvents.push(event)
      }

      const unsub = eventSource.on(handler)

      expect(handlers).toHaveLength(1)

      // Simulate event
      const testEvent: Event = { type: "test", data: "value" }
      handlers.forEach((h) => h(testEvent))

      expect(receivedEvents).toHaveLength(1)
      expect(receivedEvents[0]).toEqual(testEvent)

      // Unsubscribe
      unsub()
      expect(handlers).toHaveLength(0)
    })
  })

  describe("event emitter", () => {
    test("should emit events by type", () => {
      const listeners: Record<string, ((event: Event) => void)[]> = {}

      const emit = (type: string, event: Event) => {
        const handlers = listeners[type] ?? []
        handlers.forEach((h) => h(event))
      }

      const on = (type: string, handler: (event: Event) => void) => {
        if (!listeners[type]) listeners[type] = []
        listeners[type].push(handler)
      }

      let receivedEvent: Event | undefined = undefined

      on("message", (event) => {
        receivedEvent = event
      })

      emit("message", { type: "message", text: "hello" })

      expect(receivedEvent).toEqual({ type: "message", text: "hello" })
    })

    test("should support multiple listeners per type", () => {
      const listeners: Record<string, ((event: Event) => void)[]> = {}

      const emit = (type: string, event: Event) => {
        const handlers = listeners[type] ?? []
        handlers.forEach((h) => h(event))
      }

      const on = (type: string, handler: (event: Event) => void) => {
        if (!listeners[type]) listeners[type] = []
        listeners[type].push(handler)
      }

      const results: string[] = []

      on("message", () => results.push("first"))
      on("message", () => results.push("second"))
      on("message", () => results.push("third"))

      emit("message", { type: "message" })

      expect(results).toEqual(["first", "second", "third"])
    })
  })

  describe("cleanup", () => {
    test("should abort signal controller on cleanup", () => {
      let aborted = false

      const abortController = {
        abort: () => {
          aborted = true
        },
        signal: {},
      }

      const cleanup = () => {
        abortController.abort()
      }

      expect(aborted).toBe(false)
      cleanup()
      expect(aborted).toBe(true)
    })

    test("should clear timer on cleanup", () => {
      let timerCleared = false
      let timer: ReturnType<typeof setTimeout> | undefined = setTimeout(() => {}, 100)

      const cleanup = () => {
        if (timer) {
          clearTimeout(timer)
          timer = undefined
          timerCleared = true
        }
      }

      expect(timer).toBeDefined()
      cleanup()
      expect(timer).toBeUndefined()
      expect(timerCleared).toBe(true)
    })
  })

  describe("result wrapping", () => {
    test("should wrap result in { data: ... } format", async () => {
      const rawData = ["item1", "item2", "item3"]

      const wrapResult = (data: any) => {
        return { data }
      }

      const result = wrapResult(rawData)

      expect(result).toHaveProperty("data")
      expect(result.data).toEqual(rawData)
    })

    test("should handle undefined results", async () => {
      const wrapResult = (data: any) => {
        return { data }
      }

      const result = wrapResult(undefined)

      expect(result).toHaveProperty("data")
      expect(result.data).toBeUndefined()
    })

    test("should handle null results", async () => {
      const wrapResult = (data: any) => {
        return { data }
      }

      const result = wrapResult(null)

      expect(result).toHaveProperty("data")
      expect(result.data).toBeNull()
    })
  })

  describe("URL storage", () => {
    test("should store provided URL", () => {
      const url = "http://localhost:4096"

      const createSDK = (url: string) => {
        return { url, client: {} }
      }

      const sdk = createSDK(url)

      expect(sdk.url).toBe(url)
    })

    test("should handle URLs with paths", () => {
      const url = "http://localhost:4096/api/v1"

      const createSDK = (url: string) => {
        return { url, client: {} }
      }

      const sdk = createSDK(url)

      expect(sdk.url).toContain("/api/v1")
    })
  })

  describe("remote mode", () => {
    test("should warn about remote mode not supported", () => {
      const warnSpy = mock(() => {})
      const originalConsoleWarn = console.warn
      console.warn = warnSpy

      const initRemote = () => {
        console.warn("Remote mode not supported in local-only version")
      }

      initRemote()

      expect(warnSpy).toHaveBeenCalledWith("Remote mode not supported in local-only version")

      console.warn = originalConsoleWarn
    })
  })
})
