/**
 * HTTP API Server Middleware
 * Handles CORS, authentication, JSON parsing, and error responses
 */

import type { ServerConfig, HttpRequest, HttpResponse } from "./types"

// ============================================================================
// Configuration
// ============================================================================

let serverConfig: ServerConfig = {
  port: 4400,
  hostname: "127.0.0.1",
  cors: [],
}

export function setMiddlewareConfig(config: ServerConfig): void {
  serverConfig = config
}

// ============================================================================
// CORS Middleware
// ============================================================================

const ALLOWED_ORIGINS = ["http://localhost:*", "http://127.0.0.1:*"]

export function handleCors(origin: string | null): Headers {
  const headers = new Headers({
    "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type, Authorization, X-API-Key",
    "Access-Control-Expose-Headers": "Content-Type",
  })

  if (!origin) {
    return headers
  }

  // Check if origin is allowed
  const isAllowed =
    // Local development
    origin.startsWith("http://localhost:") ||
    origin.startsWith("http://127.0.0.1:") ||
    // Configured CORS origins
    serverConfig.cors?.some((allowed) => origin === allowed) ||
    // Wildcard patterns
    ALLOWED_ORIGINS.some((pattern) => {
      const regex = new RegExp(`^${pattern.replace("*", ".*")}$`)
      return regex.test(origin)
    })

  if (isAllowed) {
    headers.set("Access-Control-Allow-Origin", origin)
  }

  return headers
}

// ============================================================================
// Authentication Middleware
// ============================================================================

export function checkAuth(req: HttpRequest): { authenticated: boolean; error?: string } {
  // If no API key is configured, allow all requests
  if (!serverConfig.apiKey) {
    return { authenticated: true }
  }

  const authHeader = req.headers.get("Authorization")
  const apiKeyHeader = req.headers.get("X-API-Key")

  const providedKey = authHeader?.replace("Bearer ", "") || apiKeyHeader

  if (!providedKey) {
    return { authenticated: false, error: "Missing API key" }
  }

  if (providedKey !== serverConfig.apiKey) {
    return { authenticated: false, error: "Invalid API key" }
  }

  return { authenticated: true }
}

// ============================================================================
// JSON Response Helpers
// ============================================================================

export function jsonResponse(data: unknown, status = 200, headers?: HeadersInit): HttpResponse {
  return {
    status,
    headers: {
      "Content-Type": "application/json",
      ...headers,
    },
    body: JSON.stringify(data),
  }
}

export function errorResponse(error: string, status = 500, details?: unknown): HttpResponse {
  return jsonResponse(
    {
      success: false,
      error,
      details,
    },
    status,
  )
}

// ============================================================================
// Middleware Chain
// ============================================================================

export interface MiddlewareContext {
  req: HttpRequest
  params: Record<string, string>
}

export type MiddlewareFn = (ctx: MiddlewareContext, next: () => Promise<HttpResponse>) => Promise<HttpResponse>

export async function applyMiddleware(
  req: HttpRequest,
  params: Record<string, string>,
  middlewares: MiddlewareFn[],
  handler: (req: HttpRequest, params: Record<string, string>) => Promise<HttpResponse>,
): Promise<HttpResponse> {
  let index = 0

  const ctx: MiddlewareContext = { req, params }

  const next = async (): Promise<HttpResponse> => {
    if (index < middlewares.length) {
      const middleware = middlewares[index++]
      return middleware(ctx, next)
    }
    return handler(req, params)
  }

  return next()
}

// ============================================================================
// Built-in Middlewares
// ============================================================================

export const corsMiddleware: MiddlewareFn = async (ctx, next) => {
  const origin = ctx.req.headers.get("Origin")
  const corsHeaders = handleCors(origin)

  // Handle preflight requests
  if (ctx.req.method === "OPTIONS") {
    return {
      status: 204,
      headers: corsHeaders,
      body: null,
    }
  }

  const response = await next()

  // Merge CORS headers into response
  const responseHeaders = new Headers(response.headers)
  corsHeaders.forEach((value, key) => {
    responseHeaders.set(key, value)
  })

  return {
    ...response,
    headers: responseHeaders,
  }
}

export const authMiddleware: MiddlewareFn = async (ctx, next) => {
  const auth = checkAuth(ctx.req)

  if (!auth.authenticated) {
    return errorResponse(auth.error || "Authentication failed", 401)
  }

  return next()
}

export const loggingMiddleware: MiddlewareFn = async (ctx, next) => {
  const start = Date.now()

  const response = await next()

  const duration = Date.now() - start
  console.log(
    JSON.stringify({
      timestamp: new Date().toISOString(),
      method: ctx.req.method,
      path: ctx.req.url.pathname,
      status: response.status,
      duration,
    }),
  )

  return response
}
