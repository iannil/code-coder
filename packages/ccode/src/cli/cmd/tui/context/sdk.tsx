import { createSimpleContext } from "./helper"
import { createGlobalEmitter } from "@solid-primitives/event-bus"
import { batch, onCleanup, onMount } from "solid-js"
import type { Event } from "@/types"

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

    const flush = () => {
      if (queue.length === 0) return
      const events = queue
      queue = []
      timer = undefined
      last = Date.now()
      batch(() => {
        for (const event of events) {
          emitter.emit(event.type, event)
        }
      })
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
    }
  },
})
