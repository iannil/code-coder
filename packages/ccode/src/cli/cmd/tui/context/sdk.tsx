import { createSimpleContext } from "./helper"
import { createGlobalEmitter } from "@solid-primitives/event-bus"
import { batch, onCleanup, onMount } from "solid-js"
import type { Event } from "@/types"
import * as fs from "fs"
import { GlobalErrorHandler } from "@/util/global-error-handler"

export type EventSource = {
  on: (handler: (event: Event) => void) => () => void
}

type RpcClient = {
  call: (input: { namespace: string; method: string; args: any[] }) => Promise<any>
  on: (event: string, handler: (data: any) => void) => () => void
}

// Global RPC client (set by thread.ts)
let rpcClient: RpcClient | undefined

export function setRpcClient(client: RpcClient) {
  rpcClient = client
}

// Helper to safely stringify values for logging
function safeStringify(value: unknown, maxDepth = 3): string {
  const seen = new WeakSet()
  const stringify = (val: unknown, depth: number): string => {
    if (depth > maxDepth) return "[MAX_DEPTH]"
    if (val === null) return "null"
    if (val === undefined) return "undefined"
    if (typeof val === "string") return `"${val.slice(0, 200)}${val.length > 200 ? "..." : ""}"`
    if (typeof val === "number" || typeof val === "boolean") return String(val)
    if (typeof val === "bigint") return `${val}n`
    if (typeof val === "function") return `[Function: ${val.name || "anonymous"}]`
    if (typeof val === "symbol") return val.toString()
    if (Array.isArray(val)) {
      if (seen.has(val)) return "[Circular]"
      seen.add(val)
      const items = val.slice(0, 10).map((v) => stringify(v, depth + 1))
      return `[${items.join(", ")}${val.length > 10 ? `, ... +${val.length - 10} more` : ""}]`
    }
    if (typeof val === "object") {
      if (seen.has(val)) return "[Circular]"
      seen.add(val)
      const entries = Object.entries(val).slice(0, 15)
      const items = entries.map(([k, v]) => `${k}: ${stringify(v, depth + 1)}`)
      const extra = Object.keys(val).length > 15 ? `, ... +${Object.keys(val).length - 15} more` : ""
      return `{${items.join(", ")}${extra}}`
    }
    return String(val)
  }
  try {
    return stringify(value, 0)
  } catch {
    return "[STRINGIFY_ERROR]"
  }
}

// Log to dev.log with context
function logToDevLog(entry: string) {
  const logPath = process.cwd() + "/dev.log"
  try {
    fs.appendFileSync(logPath, entry)
  } catch {
    // Ignore write errors
  }
}

// Create a proxy for the SDK client that calls RPC
function createSDKClient() {
  const call = async (namespace: string, method: string, ...args: any[]) => {
    if (!rpcClient) throw new Error("RPC client not initialized")
    const result = await rpcClient.call({ namespace, method, args })
    // Wrap result in { data: ... } to match original SDK client format
    return { data: result }
  }

  // Create a recursive proxy that supports nested access like experimental.resource.list
  const createNestedProxy = (path: string[] = []): any => {
    return new Proxy(() => {}, {
      get(_, prop: string) {
        return createNestedProxy([...path, prop])
      },
      apply(_, __, args) {
        if (path.length < 2) throw new Error(`Invalid API call: ${path.join(".")}`)
        const namespace = path[0]
        const method = path.slice(1).join(".")
        // If single arg is an object with sessionID/permissionID/etc., pass it directly
        // Otherwise spread the args array (for backward compatibility)
        if (args.length === 1 && typeof args[0] === "object" && Object.keys(args[0]).length > 1) {
          return call(namespace, method, args[0])
        }
        return call(namespace, method, ...args)
      },
    })
  }

  return createNestedProxy()
}

export const { use: useSDK, provider: SDKProvider } = createSimpleContext({
  name: "SDK",
  init: (props: { url: string; directory?: string; fetch?: typeof fetch; events?: EventSource }) => {
    const abort = new AbortController()

    const emitter = createGlobalEmitter<{
      [key in Event["type"]]: Extract<Event, { type: key }>
    }>()

    let queue: Event[] = []
    let timer: Timer | undefined
    let last = 0
    let currentEvent: Event | null = null

    const flush = () => {
      if (queue.length === 0) return
      const events = queue
      queue = []
      timer = undefined
      last = Date.now()

      // Log events being processed for debugging
      for (const event of events) {
        GlobalErrorHandler.addContext(`flush:${event.type}`, {
          type: event.type,
          props: event.properties,
        })
      }

      try {
        batch(() => {
          for (const event of events) {
            currentEvent = event
            try {
              emitter.emit(event.type, event)
            } catch (err) {
              const timestamp = new Date().toISOString()
              const errorMessage = err instanceof Error ? err.message : String(err)
              const errorStack = err instanceof Error ? err.stack : undefined

              // Build detailed context log
              const logLines = [
                `[${timestamp}] [ERROR] Event Emit Error`,
                `  Event Type: ${event.type}`,
                `  Event Properties: ${safeStringify(event.properties)}`,
                `  Error: ${errorMessage}`,
              ]

              if (errorStack) {
                logLines.push(`  Stack: ${errorStack}`)
              }

              // Add type analysis for debugging TextNodeRenderable errors
              if (errorMessage.includes("TextNodeRenderable")) {
                logLines.push(`  [DEBUG] Analyzing event properties for non-string values:`)
                const props = event.properties as Record<string, unknown>
                for (const [key, value] of Object.entries(props)) {
                  const valueType = value === null ? "null" : Array.isArray(value) ? "array" : typeof value
                  if (valueType !== "string" && valueType !== "undefined") {
                    logLines.push(`    ${key}: type=${valueType}, value=${safeStringify(value)}`)
                  }
                }
              }

              logLines.push("")
              logToDevLog(logLines.join("\n"))

              console.error(`[dev.log] Error emitting event ${event.type}:`, err)
            } finally {
              currentEvent = null
            }
          }
        })
      } catch (batchError) {
        // Catch errors that happen after the batch completes (during SolidJS reconciliation)
        const timestamp = new Date().toISOString()
        const errorMessage = batchError instanceof Error ? batchError.message : String(batchError)
        const errorStack = batchError instanceof Error ? batchError.stack : undefined

        const logLines = [
          `[${timestamp}] [ERROR] Batch Reconciliation Error`,
          `  Events in batch: ${events.map(e => e.type).join(", ")}`,
          `  Error: ${errorMessage}`,
        ]

        if (errorStack) {
          logLines.push(`  Stack: ${errorStack}`)
        }

        // Log all event details
        logLines.push(`  Event Details:`)
        for (const event of events) {
          logLines.push(`    ${event.type}: ${safeStringify(event.properties)}`)
        }

        logLines.push("")
        logToDevLog(logLines.join("\n"))

        // Re-throw to let ErrorBoundary catch it
        throw batchError
      }
    }

    const handleEvent = (event: Event) => {
      queue.push(event)
      const elapsed = Date.now() - last

      if (timer) return
      if (elapsed < 16) {
        timer = setTimeout(flush, 16)
        return
      }
      flush()
    }

    onMount(async () => {
      // If an event source is provided (local mode via RPC), use it
      if (props.events) {
        const unsub = props.events.on(handleEvent)
        onCleanup(unsub)
        return
      }

      // Remote mode via SSE - not supported in simplified version
      console.warn("Remote mode not supported in local-only version")
    })

    onCleanup(() => {
      abort.abort()
      if (timer) clearTimeout(timer)
    })

    // Create the local SDK client
    const client = createSDKClient()

    return {
      url: props.url,
      event: emitter,
      client,
      // Expose current event for debugging
      getCurrentEvent: () => currentEvent,
    }
  },
})
