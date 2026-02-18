/**
 * HTTP API Server Entry Point
 * Uses Bun.serve for high-performance HTTP server with graceful shutdown
 */

import type { ServerConfig, HttpRequest, HttpResponse } from "./types"
import { router, registerRoutes } from "./router"
import { setMiddlewareConfig, corsMiddleware, authMiddleware, loggingMiddleware } from "./middleware"
import { VERSION } from "../../version.js"
import { Instance } from "../../project/instance"

// ============================================================================
// Server State
// ============================================================================

let server: ReturnType<typeof Bun.serve> | undefined
let startTime = Date.now()
let serverDirectory: string | undefined

// ============================================================================
// Server Configuration
// ============================================================================

interface StartOptions {
  port?: number
  hostname?: string
  cors?: string | string[]
  apiKey?: string
}

// ============================================================================
// Graceful Shutdown
// ============================================================================

function setupShutdownHandlers(): void {
  const shutdown = async (signal: string) => {
    console.log(JSON.stringify({ timestamp: new Date().toISOString(), event: "shutdown", signal }))
    if (server) {
      server.stop()
      server = undefined
    }
    process.exit(0)
  }

  process.on("SIGINT", () => shutdown("SIGINT"))
  process.on("SIGTERM", () => shutdown("SIGTERM"))
}

// ============================================================================
// Health Check Handler
// ============================================================================

async function healthCheck(): Promise<HttpResponse> {
  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      status: "ok",
      version: VERSION,
      uptime: Date.now() - startTime,
    }),
  }
}

// ============================================================================
// API Discovery Handler
// ============================================================================

async function apiDiscovery(): Promise<HttpResponse> {
  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      version: VERSION,
      endpoints: [
        "GET /",
        "GET /api",
        "GET /api/sessions",
        "GET /api/sessions/:id",
        "POST /api/sessions",
        "DELETE /api/sessions/:id",
        "GET /api/sessions/:id/messages",
        "POST /api/sessions/:id/messages",
        "GET /api/config",
        "PUT /api/config",
        "GET /api/permissions",
        "POST /api/permissions/:id/respond",
        "GET /api/files",
        "GET /api/events",
      ],
    }),
  }
}

// ============================================================================
// 404 Handler
// ============================================================================

async function notFound(): Promise<HttpResponse> {
  return {
    status: 404,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      success: false,
      error: "Not found",
    }),
  }
}

// ============================================================================
// Request Handler Factory
// ============================================================================

function createRequestHandler() {
  return async (req: Request): Promise<Response> => {
    // Wrap request handling in Instance context to ensure proper project context
    if (!serverDirectory) {
      throw new Error("Server not properly initialized")
    }

    return Instance.provide({
      directory: serverDirectory,
      fn: async () => {
        const url = new URL(req.url)
        const method = req.method
        const headers = req.headers

        const httpRequest: HttpRequest = {
          method,
          url,
          headers,
          body: req.body as ReadableStream | null,
        }

        // Health check
        if (url.pathname === "/" && method === "GET") {
          const response = await healthCheck()
          return new Response(response.body ?? undefined, {
            status: response.status,
            headers: response.headers,
          })
        }

        // API discovery
        if (url.pathname === "/api" && method === "GET") {
          const response = await apiDiscovery()
          return new Response(response.body ?? undefined, {
            status: response.status,
            headers: response.headers,
          })
        }

        // Try to match a route
        const routeResponse = await router.handle(httpRequest)
        if (routeResponse) {
          return new Response(routeResponse.body ?? undefined, {
            status: routeResponse.status,
            headers: routeResponse.headers,
          })
        }

        // 404
        const response = await notFound()
        return new Response(response.body ?? undefined, {
          status: response.status,
          headers: response.headers,
        })
      },
    })
  }
}

// ============================================================================
// Server Start Function
// ============================================================================

export async function start(options: StartOptions = {}): Promise<void> {
  const port = options.port ?? 4400
  // Use "::" to listen on both IPv4 and IPv6, avoiding connection issues
  // when clients resolve "localhost" to IPv6 (::1) first
  const hostname = options.hostname ?? "::"
  const cors = typeof options.cors === "string" ? [options.cors] : (options.cors ?? [])

  // Save the current directory for request handling context
  serverDirectory = Instance.directory

  // Configure middleware
  setMiddlewareConfig({
    port,
    hostname,
    cors,
    apiKey: options.apiKey,
  })

  // Register routes
  await registerRoutes()

  // Check if port is in use
  const portInUse = await isPortInUse(port)
  if (portInUse) {
    throw new Error(`Port ${port} is already in use`)
  }

  // Start server
  server = Bun.serve({
    port,
    hostname,
    fetch: createRequestHandler(),
  })

  // Setup shutdown handlers
  setupShutdownHandlers()

  // Log server start
  console.log(
    JSON.stringify({
      timestamp: new Date().toISOString(),
      event: "server_started",
      port,
      hostname,
      version: VERSION,
    }),
  )
}

// ============================================================================
// Server Stop Function
// ============================================================================

export async function stop(): Promise<void> {
  if (server) {
    server.stop()
    server = undefined
  }
}

// ============================================================================
// Port Check Utility
// ============================================================================

async function isPortInUse(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    Bun.connect({
      hostname: "127.0.0.1",
      port,
      socket: {
        data() {},
        open(socket) {
          socket.end()
          resolve(true)
        },
        error() {
          resolve(false)
        },
      },
    }).catch(() => resolve(false))
  })
}

// ============================================================================
// Export for testing
// ============================================================================

export { router }
