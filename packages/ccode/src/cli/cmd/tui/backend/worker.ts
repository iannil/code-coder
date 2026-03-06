/**
 * Worker Backend Adapter
 *
 * Implements the TuiBackend interface using a Bun Web Worker and RPC communication.
 * This is the default backend mode that runs the LocalAPI in-process.
 *
 * ## How it works
 *
 * ```
 * ┌─────────────────────────────────────────────────────────────────────────────┐
 * │                       WorkerBackend                                         │
 * │   ┌─────────────────────────────────────────────────────────────────────┐   │
 * │   │                      Bun Web Worker                                  │   │
 * │   │  • Runs worker.ts in separate thread                                │   │
 * │   │  • Initializes Instance and LocalAPI                                │   │
 * │   │  • Subscribes to Bus events                                         │   │
 * │   └─────────────────────────────────────────────────────────────────────┘   │
 * │                              ↕ JSON RPC                                     │
 * │   ┌─────────────────────────────────────────────────────────────────────┐   │
 * │   │                      RPC Client                                      │   │
 * │   │  • call(method, input) → result                                     │   │
 * │   │  • on(event, handler) → unsubscribe                                 │   │
 * │   └─────────────────────────────────────────────────────────────────────┘   │
 * └─────────────────────────────────────────────────────────────────────────────┘
 * ```
 */

import { Rpc } from "@/util/rpc"
import { Log } from "@/util/log"
import { iife } from "@/util/iife"
import type { Event } from "@/types"
import type { rpc as workerRpc } from "../worker"
import type { TuiBackend, EventSource, RpcClient, WorkerBackendOptions } from "./index"

declare global {
  const CCODE_WORKER_PATH: string
}

// ══════════════════════════════════════════════════════════════════════════════
// WorkerBackend Class
// ══════════════════════════════════════════════════════════════════════════════

/**
 * Worker Backend Implementation
 *
 * Wraps a Bun Web Worker running the LocalAPI, providing the TuiBackend interface.
 */
export class WorkerBackend implements TuiBackend {
  readonly mode = "worker" as const
  private worker: Worker
  private rpcClient: ReturnType<typeof Rpc.client<typeof workerRpc>>
  private _events: EventSource
  private _rpc: RpcClient

  constructor(worker: Worker, rpcClient: ReturnType<typeof Rpc.client<typeof workerRpc>>) {
    this.worker = worker
    this.rpcClient = rpcClient

    // Create EventSource adapter
    this._events = {
      on: (handler) => {
        const unsub = this.rpcClient.on("event", handler as (data: unknown) => void)
        return () => unsub()
      },
    }

    // Create RpcClient adapter
    this._rpc = {
      call: async (input) => {
        return await this.rpcClient.call("call", input)
      },
      on: (event, handler) => this.rpcClient.on(event, handler),
    }
  }

  get events(): EventSource {
    return this._events
  }

  get rpc(): RpcClient {
    return this._rpc
  }

  async reload(): Promise<void> {
    await this.rpcClient.call("reload", undefined)
  }

  async shutdown(): Promise<void> {
    Log.Default.info("worker backend shutting down")
    await this.rpcClient.call("shutdown", undefined)
    this.worker.terminate()
  }

  isConnected(): boolean {
    // Worker is always "connected" while alive
    return true
  }
}

// ══════════════════════════════════════════════════════════════════════════════
// Factory Function
// ══════════════════════════════════════════════════════════════════════════════

/** Resolve the worker script path */
async function resolveWorkerPath(options?: WorkerBackendOptions): Promise<string | URL> {
  if (options?.workerPath) return options.workerPath

  const localWorker = new URL("../worker.ts", import.meta.url)
  const distWorker = new URL("../cli/cmd/tui/worker.js", import.meta.url)

  return await iife(async () => {
    if (typeof CCODE_WORKER_PATH !== "undefined") return CCODE_WORKER_PATH
    if (await Bun.file(distWorker).exists()) return distWorker
    return localWorker
  })
}

/**
 * Create a Worker backend instance.
 *
 * @param options - Worker configuration options
 * @returns Promise resolving to initialized WorkerBackend
 *
 * @example
 * ```typescript
 * const backend = await createWorkerBackend()
 *
 * // Subscribe to events
 * backend.events.on((event) => {
 *   console.log("Event:", event.type)
 * })
 *
 * // Make API calls
 * const sessions = await backend.rpc.call({
 *   namespace: "session",
 *   method: "list",
 *   args: []
 * })
 *
 * // Cleanup
 * await backend.shutdown()
 * ```
 */
export async function createWorkerBackend(options?: WorkerBackendOptions): Promise<WorkerBackend> {
  const workerPath = await resolveWorkerPath(options)

  const worker = new Worker(workerPath, {
    env: Object.fromEntries(
      Object.entries(process.env).filter((entry): entry is [string, string] => entry[1] !== undefined),
    ),
  })

  worker.onerror = (e) => {
    Log.Default.error("Worker error", { error: e })
  }

  const rpcClient = Rpc.client<typeof workerRpc>(worker)

  return new WorkerBackend(worker, rpcClient)
}
