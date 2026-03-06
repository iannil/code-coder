/**
 * TUI Backend Abstraction Layer
 *
 * This module provides a unified interface for the TUI to communicate with its backend,
 * supporting two modes:
 * - **Worker mode**: Direct in-process communication via Web Worker + RPC (default)
 * - **IPC mode**: Communication with zero-cli via Unix Domain Socket
 *
 * The adapter pattern allows the TUI to switch between modes transparently,
 * enabling gradual migration from TypeScript to Rust backend.
 *
 * ## Architecture
 *
 * ```
 * ┌─────────────────────────────────────────────────────────────────────────────┐
 * │                         TUI Thread (thread.ts)                              │
 * │                              │                                              │
 * │                    ┌─────────┴─────────┐                                    │
 * │                    │    TuiBackend     │  (Unified Interface)               │
 * │                    └─────────┬─────────┘                                    │
 * │               ┌──────────────┴──────────────┐                               │
 * │               ▼                             ▼                               │
 * │    ┌─────────────────────┐       ┌─────────────────────┐                   │
 * │    │   WorkerBackend     │       │    IpcBackend       │                   │
 * │    │  (Web Worker + RPC) │       │  (Unix Socket IPC)  │                   │
 * │    └─────────────────────┘       └─────────────────────┘                   │
 * └─────────────────────────────────────────────────────────────────────────────┘
 * ```
 */

import type { Event } from "@/types"

// ══════════════════════════════════════════════════════════════════════════════
// Type Definitions
// ══════════════════════════════════════════════════════════════════════════════

/** Backend mode selection */
export type BackendMode = "worker" | "ipc"

/** Event handler for backend events */
export type BackendEventHandler = (event: Event) => void

/** Event source interface for TUI components */
export interface EventSource {
  on: (handler: BackendEventHandler) => () => void
}

/** RPC client interface for SDK calls */
export interface RpcClient {
  call: (input: { namespace: string; method: string; args: unknown[] }) => Promise<unknown>
  on: (event: string, handler: (data: unknown) => void) => () => void
}

/**
 * Unified TUI Backend Interface
 *
 * This interface abstracts the communication layer between the TUI and its backend.
 * Both Worker and IPC backends implement this interface, allowing transparent switching.
 */
export interface TuiBackend {
  /** Backend mode identifier */
  readonly mode: BackendMode

  /** Event source for subscribing to backend events */
  readonly events: EventSource

  /** RPC client for SDK-style API calls */
  readonly rpc: RpcClient

  /**
   * Reload the backend (e.g., for hot-reloading).
   * In worker mode: disposes and reinitializes the worker.
   * In IPC mode: sends a reload signal to zero-cli.
   */
  reload(): Promise<void>

  /**
   * Shutdown the backend gracefully.
   * Cleans up resources and closes connections.
   */
  shutdown(): Promise<void>

  /**
   * Check if the backend is connected and ready.
   */
  isConnected(): boolean
}

// ══════════════════════════════════════════════════════════════════════════════
// Configuration Types
// ══════════════════════════════════════════════════════════════════════════════

/** Options for creating a Worker backend */
export interface WorkerBackendOptions {
  /** Worker script path */
  workerPath?: string | URL
}

/** Options for creating an IPC backend */
export interface IpcBackendOptions {
  /** Path to IPC socket (default: ~/.codecoder/ipc.sock) */
  socketPath?: string
  /** Path to zero-cli binary (default: "zero-cli") */
  cliBinary?: string
  /** Auto-start CLI if not running (default: true) */
  autoStart?: boolean
  /** Request timeout in ms (default: 30000) */
  timeout?: number
}

// ══════════════════════════════════════════════════════════════════════════════
// Re-exports
// ══════════════════════════════════════════════════════════════════════════════

export { createWorkerBackend } from "./worker"
export { createIpcBackend } from "./ipc"
