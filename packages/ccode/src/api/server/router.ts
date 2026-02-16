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
  const { listAgents, invokeAgent, getAgent } = await import("./handlers/agent")
  const { createTask, listTasks, getTask, streamTaskEvents, interactTask, deleteTask } = await import(
    "./handlers/task"
  )
  const {
    listProviders,
    listConnectedProviders,
    getProviderAuthMethods,
    getProvider,
    getProviderModels,
  } = await import("./handlers/provider")
  const {
    getMcpStatus,
    getMcpTools,
    getMcpResources,
    connectMcp,
    disconnectMcp,
    toggleMcp,
    getMcpAuthStatus,
    startMcpAuth,
    finishMcpAuth,
  } = await import("./handlers/mcp")

  // Document/Writing handlers
  const {
    listDocuments,
    getDocument,
    createDocument,
    updateDocument,
    deleteDocument,
    getDocumentStats,
    exportDocument,
    listChapters,
    getChapter,
    updateChapter,
    listEntities,
    createEntity,
    updateEntity,
    deleteEntity,
    listVolumes,
    createVolume,
  } = await import("./handlers/document")

  // Memory handlers
  const {
    listDailyDates,
    getDailyNotes,
    appendDailyNoteHandler,
    getLongTermMemory,
    getMemorySectionsHandler,
    updateCategoryHandler,
    mergeToCategoryHandler,
    getConsolidationStatsHandler,
    triggerConsolidation,
    getMemorySummaryHandler,
  } = await import("./handlers/memory")

  // Hooks handlers
  const { listHooks, getHooksByLifecycle, getHooksSettings, getHookLocations, getActionTypes } = await import(
    "./handlers/hooks"
  )

  // LSP handlers
  const {
    getLspStatus,
    getLspDiagnostics,
    getLspConfig,
    checkLspAvailable,
    initLsp,
    touchFile,
    getHover,
    getDefinition,
    getReferences,
    getWorkspaceSymbols,
    getDocumentSymbols,
  } = await import("./handlers/lsp")

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

  // Agent routes (for ZeroBot integration - legacy)
  router.get("/api/agents", listAgents)
  router.get("/api/agent/:agentId", getAgent)
  router.post("/api/agent/invoke", invokeAgent)

  // Task routes (async task flow model for ZeroBot integration - v1)
  router.post("/api/v1/tasks", createTask)
  router.get("/api/v1/tasks", listTasks)
  router.get("/api/v1/tasks/:id", getTask)
  router.get("/api/v1/tasks/:id/events", streamTaskEvents)
  router.post("/api/v1/tasks/:id/interact", interactTask)
  router.delete("/api/v1/tasks/:id", deleteTask)

  // Provider routes (for Web UI model selection and provider management)
  router.get("/api/providers", listProviders)
  router.get("/api/providers/connected", listConnectedProviders)
  router.get("/api/providers/auth", getProviderAuthMethods)
  router.get("/api/providers/:providerId", getProvider)
  router.get("/api/providers/:providerId/models", getProviderModels)

  // MCP routes (for Web UI MCP server management)
  router.get("/api/mcp/status", getMcpStatus)
  router.get("/api/mcp/tools", getMcpTools)
  router.get("/api/mcp/resources", getMcpResources)
  router.post("/api/mcp/:name/connect", connectMcp)
  router.post("/api/mcp/:name/disconnect", disconnectMcp)
  router.post("/api/mcp/:name/toggle", toggleMcp)
  router.get("/api/mcp/:name/auth-status", getMcpAuthStatus)
  router.post("/api/mcp/:name/auth/start", startMcpAuth)
  router.post("/api/mcp/:name/auth/finish", finishMcpAuth)

  // Document/Writing routes
  router.get("/api/documents", listDocuments)
  router.get("/api/documents/:id", getDocument)
  router.post("/api/documents", createDocument)
  router.put("/api/documents/:id", updateDocument)
  router.delete("/api/documents/:id", deleteDocument)
  router.get("/api/documents/:id/stats", getDocumentStats)
  router.get("/api/documents/:id/export", exportDocument)
  router.get("/api/documents/:id/chapters", listChapters)
  router.get("/api/documents/:id/chapters/:chapterId", getChapter)
  router.put("/api/documents/:id/chapters/:chapterId", updateChapter)
  router.get("/api/documents/:id/entities", listEntities)
  router.post("/api/documents/:id/entities", createEntity)
  router.put("/api/documents/:id/entities/:entityId", updateEntity)
  router.delete("/api/documents/:id/entities/:entityId", deleteEntity)
  router.get("/api/documents/:id/volumes", listVolumes)
  router.post("/api/documents/:id/volumes", createVolume)

  // Memory routes
  router.get("/api/memory/daily", listDailyDates)
  router.get("/api/memory/daily/:date", getDailyNotes)
  router.post("/api/memory/daily", appendDailyNoteHandler)
  router.get("/api/memory/long-term", getLongTermMemory)
  router.get("/api/memory/sections", getMemorySectionsHandler)
  router.put("/api/memory/category/:category", updateCategoryHandler)
  router.post("/api/memory/category/:category/merge", mergeToCategoryHandler)
  router.get("/api/memory/consolidation/stats", getConsolidationStatsHandler)
  router.post("/api/memory/consolidation", triggerConsolidation)
  router.get("/api/memory/summary", getMemorySummaryHandler)

  // Hooks routes
  router.get("/api/hooks", listHooks)
  router.get("/api/hooks/settings", getHooksSettings)
  router.get("/api/hooks/locations", getHookLocations)
  router.get("/api/hooks/action-types", getActionTypes)
  router.get("/api/hooks/:lifecycle", getHooksByLifecycle)

  // LSP routes
  router.get("/api/lsp/status", getLspStatus)
  router.get("/api/lsp/diagnostics", getLspDiagnostics)
  router.get("/api/lsp/config", getLspConfig)
  router.get("/api/lsp/available", checkLspAvailable)
  router.post("/api/lsp/init", initLsp)
  router.post("/api/lsp/touch", touchFile)
  router.post("/api/lsp/hover", getHover)
  router.post("/api/lsp/definition", getDefinition)
  router.post("/api/lsp/references", getReferences)
  router.post("/api/lsp/workspace-symbols", getWorkspaceSymbols)
  router.post("/api/lsp/document-symbols", getDocumentSymbols)
}

// ============================================================================
// Export router class for testing
// ============================================================================

export { Router }
