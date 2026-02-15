/**
 * HTTP API Server Router
 * Handles route registration, pattern matching, and parameter extraction
 */

import type { HttpMethod, HttpRequest, Route, RouteHandler, RouteParams, HttpResponse } from "./types"

// ============================================================================
// Router State
// ============================================================================

class Router {
  private routes: Route[] = []

  register(method: HttpMethod, pattern: string, handler: RouteHandler): void {
    this.routes.push({ method, pattern, handler })
  }

  get(pattern: string, handler: RouteHandler): void {
    this.register("GET", pattern, handler)
  }

  post(pattern: string, handler: RouteHandler): void {
    this.register("POST", pattern, handler)
  }

  put(pattern: string, handler: RouteHandler): void {
    this.register("PUT", pattern, handler)
  }

  delete(pattern: string, handler: RouteHandler): void {
    this.register("DELETE", pattern, handler)
  }

  patch(pattern: string, handler: RouteHandler): void {
    this.register("PATCH", pattern, handler)
  }

  options(pattern: string, handler: RouteHandler): void {
    this.register("OPTIONS", pattern, handler)
  }

  async handle(req: HttpRequest): Promise<HttpResponse | null> {
    const method = req.method.toUpperCase() as HttpMethod
    const pathname = req.url.pathname

    for (const route of this.routes) {
      if (route.method !== method) {
        continue
      }

      const match = this.matchRoute(route.pattern, pathname)
      if (match) {
        try {
          return await route.handler(req, match.params)
        } catch (error) {
          return {
            status: 500,
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              success: false,
              error: error instanceof Error ? error.message : String(error),
            }),
          }
        }
      }
    }

    return null
  }

  private matchRoute(pattern: string, pathname: string): { params: RouteParams } | null {
    // Fast path for exact match
    if (pattern === pathname) {
      return { params: {} }
    }

    // Check if pattern has parameters
    if (!pattern.includes(":")) {
      return null
    }

    // Convert pattern to regex
    const regex = this.patternToRegex(pattern)
    const match = pathname.match(regex)

    if (!match) {
      return null
    }

    // Extract parameter names from pattern
    const paramNames: string[] = []
    let inParam = false
    let currentParam = ""

    for (const char of pattern) {
      if (char === ":") {
        inParam = true
        currentParam = ""
      } else if (inParam && (char === "/" || char === "?" || char === "&")) {
        paramNames.push(currentParam)
        inParam = false
      } else if (inParam) {
        currentParam += char
      }
    }

    if (inParam && currentParam) {
      paramNames.push(currentParam)
    }

    // Build params object
    const params: RouteParams = {}
    for (let i = 0; i < paramNames.length; i++) {
      params[paramNames[i]] = match[i + 1]
    }

    return { params }
  }

  private patternToRegex(pattern: string): RegExp {
    // Convert :param style to named groups
    let regex = pattern.replace(/\*/g, ".*")

    // Replace path parameters with regex capture groups
    regex = regex.replace(/:([a-zA-Z_][a-zA-Z0-9_]*)/g, "([^/?]+)")

    // Escape special regex characters except for our capture groups
    regex = "^" + regex.replace(/[.+?^${}()|[\]\\]/g, "\\$&") + "$"

    return new RegExp(regex)
  }
}

// ============================================================================
// Export singleton router instance
// ============================================================================

export const router = new Router()

export async function registerRoutes(): Promise<void> {
  // Import handlers
  const {
    listSessions,
    getSession,
    createSession,
    deleteSession,
    getSessionMessages,
    sendSessionMessage,
    getSessionChildren,
    forkSession,
  } = await import("./handlers/session")

  const { getConfig, updateConfig } = await import("./handlers/config")
  const { listPermissions, respondPermission, replyPermission } = await import("./handlers/permission")
  const { findFiles, findFilesCache } = await import("./handlers/find")
  const { streamEvents, listEventChannels } = await import("./handlers/event")

  // Session routes
  router.get("/api/sessions", listSessions)
  router.get("/api/sessions/:id", getSession)
  router.post("/api/sessions", createSession)
  router.delete("/api/sessions/:id", deleteSession)
  router.get("/api/sessions/:id/messages", getSessionMessages)
  router.post("/api/sessions/:id/messages", sendSessionMessage)
  router.get("/api/sessions/:id/children", getSessionChildren)
  router.post("/api/sessions/:id/fork", forkSession)

  // Config routes
  router.get("/api/config", getConfig)
  router.put("/api/config", updateConfig)

  // Permission routes
  router.get("/api/permissions", listPermissions)
  router.post("/api/permissions/:id/respond", respondPermission)
  router.post("/api/permissions/:id/reply", replyPermission)

  // Find routes
  router.get("/api/files", findFiles)
  router.get("/api/files/cache", findFilesCache)

  // Event routes
  router.get("/api/events", streamEvents)
  router.get("/api/events/channels", listEventChannels)
}

// ============================================================================
// Export router class for testing
// ============================================================================

export { Router }
