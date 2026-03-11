/**
 * Rust Daemon SDK
 *
 * A lightweight TypeScript SDK for communicating with the zero-cli Rust daemon.
 * Provides both WebSocket (real-time) and HTTP (REST) interfaces.
 *
 * ## Quick Start
 *
 * ### WebSocket (Recommended for Agent Execution)
 *
 * ```typescript
 * import { getWebSocketClient } from "./sdk"
 *
 * const ws = getWebSocketClient({ url: "ws://127.0.0.1:4402/ws" })
 * await ws.connect()
 *
 * // Execute an agent with streaming
 * await ws.executeAgent(
 *   {
 *     session_id: "my-session",
 *     agent: "build",
 *     message: "Help me implement this feature",
 *   },
 *   (event) => {
 *     if (event.type === "text") {
 *       process.stdout.write(event.content)
 *     }
 *   }
 * )
 * ```
 *
 * ### HTTP Client (for Simple Requests)
 *
 * ```typescript
 * import { getHttpClient } from "./sdk"
 *
 * const http = getHttpClient({ baseUrl: "http://127.0.0.1:4402" })
 *
 * // List agents
 * const { agents } = await http.listAgents()
 *
 * // Get observer status
 * const { data } = await http.getObserverStatus()
 *
 * // Switch gear
 * await http.switchGear("S", "Need high autonomy mode")
 * ```
 *
 * @module sdk
 */

// Types
export * from "./types"

// WebSocket Client
export { WebSocketClient, getWebSocketClient, resetWebSocketClient } from "./websocket"
export type { WebSocketClientConfig } from "./websocket"

// HTTP Client
export { HttpClient, getHttpClient, resetHttpClient } from "./client"
export type { HttpClientConfig } from "./client"
