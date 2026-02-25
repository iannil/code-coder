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
    // First, escape special regex characters (but not : or * which we'll handle separately)
    let regex = pattern.replace(/[.+?^${}()|[\]\\]/g, "\\$&")

    // Replace path parameters with regex capture groups
    regex = regex.replace(/:([a-zA-Z_][a-zA-Z0-9_]*)/g, "([^/?]+)")

    // Convert * to .*
    regex = regex.replace(/\*/g, ".*")

    return new RegExp("^" + regex + "$")
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
    updateSession,
    getSessionMessages,
    sendSessionMessage,
    getSessionChildren,
    forkSession,
  } = await import("./handlers/session")

  const { getConfig, updateConfig } = await import("./handlers/config")
  const { listPermissions, respondPermission, replyPermission, addToAllowlist, getAllowlist, removeFromAllowlist, clearAllowlist } = await import("./handlers/permission")
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

  // Channel handlers (ZeroBot integration)
  const { listChannels, getChannel, checkChannelHealth } = await import("./handlers/channel")

  // Directory handlers
  const { listDirectories } = await import("./handlers/directory")

  // Project handlers
  const { listProjects, getProject, createProject, updateProject, deleteProject, getProjectSessions } = await import(
    "./handlers/project"
  )

  // Credential handlers
  const {
    listCredentials,
    getCredential,
    addCredential,
    updateCredential,
    deleteCredential,
    resolveCredential,
    listSessions: listCredentialSessions,
    getSession: getCredentialSession,
    saveSession: saveCredentialSession,
    clearSession: clearCredentialSession,
    cleanupSessions: cleanupCredentialSessions,
  } = await import("./handlers/credential")

  // Skill handlers
  const {
    listSkills,
    getSkill,
    installSkill,
    uninstallSkill,
    updateSkill,
    listSkillCategories,
  } = await import("./handlers/skill")

  // Session routes
  router.get("/api/sessions", listSessions)
  router.get("/api/sessions/:id", getSession)
  router.post("/api/sessions", createSession)
  router.patch("/api/sessions/:id", updateSession)
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

  // Allowlist management routes (v1)
  router.post("/api/v1/permission/allowlist", addToAllowlist)
  router.get("/api/v1/permission/allowlist/:userID", getAllowlist)
  router.delete("/api/v1/permission/allowlist/:userID/:tool", removeFromAllowlist)
  router.delete("/api/v1/permission/allowlist/:userID", clearAllowlist)

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

  // Channel routes (ZeroBot integration)
  router.get("/api/channels", listChannels)
  router.get("/api/channels/:name", getChannel)
  router.post("/api/channels/:name/health", checkChannelHealth)

  // Directory routes
  router.get("/api/directories", listDirectories)

  // Project routes
  router.get("/api/projects", listProjects)
  router.get("/api/projects/:id", getProject)
  router.post("/api/projects", createProject)
  router.patch("/api/projects/:id", updateProject)
  router.delete("/api/projects/:id", deleteProject)
  router.get("/api/projects/:id/sessions", getProjectSessions)

  // Credential routes
  router.get("/api/credentials", listCredentials)
  router.get("/api/credentials/resolve", resolveCredential)
  router.get("/api/credentials/sessions", listCredentialSessions)
  router.post("/api/credentials/sessions/cleanup", cleanupCredentialSessions)
  router.get("/api/credentials/:id", getCredential)
  router.post("/api/credentials", addCredential)
  router.put("/api/credentials/:id", updateCredential)
  router.delete("/api/credentials/:id", deleteCredential)
  router.get("/api/credentials/:id/session", getCredentialSession)
  router.put("/api/credentials/:id/session", saveCredentialSession)
  router.delete("/api/credentials/:id/session", clearCredentialSession)

  // Skill routes
  router.get("/api/skills", listSkills)
  router.get("/api/skills/categories", listSkillCategories)
  router.get("/api/skills/:id", getSkill)
  router.post("/api/skills/install", installSkill)
  router.patch("/api/skills/:id", updateSkill)
  router.delete("/api/skills/:id", uninstallSkill)

  // Chat routes (for ZeroBot bridge)
  const { chat, chatHealth, clearConversation, compactConversation } = await import("./handlers/chat")
  router.post("/api/v1/chat", chat)
  router.get("/api/v1/chat/health", chatHealth)
  router.post("/api/v1/chat/clear", clearConversation)
  router.post("/api/v1/chat/compact", compactConversation)

  // Metering routes (for Admin dashboard)
  const { getUsage, getUsersUsage, getQuotas, updateQuota, recordUsage } = await import("./handlers/metering")
  router.get("/api/v1/metering/usage", getUsage)
  router.get("/api/v1/metering/users", getUsersUsage)
  router.get("/api/v1/metering/quotas", getQuotas)
  router.put("/api/v1/metering/quotas/:userId", updateQuota)
  router.post("/api/v1/metering/record", recordUsage)

  // Registry routes (for Chat page and agent discovery)
  const {
    listAgents: registryListAgents,
    getAgent: registryGetAgent,
    recommendAgent,
    searchAgents,
    listCategories,
    listRecommended,
  } = await import("./handlers/registry")
  router.get("/api/v1/registry/agents", registryListAgents)
  router.get("/api/v1/registry/agents/:name", registryGetAgent)
  router.post("/api/v1/registry/recommend", recommendAgent)
  router.get("/api/v1/registry/search", searchAgents)
  router.get("/api/v1/registry/categories", listCategories)
  router.get("/api/v1/registry/recommended", listRecommended)

  // Assess routes (Technical Feasibility Assessment API for ZeroBot integration)
  const { assessFeasibility, assessHealth } = await import("./handlers/assess")
  router.post("/api/v1/assess/feasibility", assessFeasibility)
  router.get("/api/v1/assess/health", assessHealth)

  // Compare routes (Multi-model A/B testing for parallel model comparison)
  const { compare, compareHealth, listCompareModels, getCompareHistory, getCompareEntry, voteForModel, deleteCompareEntry } = await import("./handlers/compare")
  router.post("/api/v1/compare", compare)
  router.get("/api/v1/compare/health", compareHealth)
  router.get("/api/v1/compare/models", listCompareModels)
  router.get("/api/v1/compare/history", getCompareHistory)
  router.get("/api/v1/compare/history/:id", getCompareEntry)
  router.post("/api/v1/compare/:id/vote", voteForModel)
  router.delete("/api/v1/compare/history/:id", deleteCompareEntry)

  // Executive Dashboard routes (for executive-level analytics)
  const { getTrends, getTeams, getActivity, getSummary, executiveHealth } = await import("./handlers/executive")
  router.get("/api/v1/executive/trends", getTrends)
  router.get("/api/v1/executive/teams", getTeams)
  router.get("/api/v1/executive/activity", getActivity)
  router.get("/api/v1/executive/summary", getSummary)
  router.get("/api/v1/executive/health", executiveHealth)

  // Knowledge Base routes (RAG for ZeroBot)
  const {
    uploadDocument: uploadKnowledge,
    listDocuments: listKnowledge,
    deleteDocument: deleteKnowledge,
    searchKnowledge,
    knowledgeHealth,
  } = await import("./handlers/knowledge")
  router.post("/api/v1/knowledge/upload", uploadKnowledge)
  router.get("/api/v1/knowledge/documents", listKnowledge)
  router.delete("/api/v1/knowledge/documents/:id", deleteKnowledge)
  router.post("/api/v1/knowledge/search", searchKnowledge)
  router.get("/api/v1/knowledge/health", knowledgeHealth)

  // Trading Review routes (for @trader agent integration)
  const {
    handleCreateTrade,
    handleListTrades,
    handleGetTrade,
    handleUpdateTrade,
    handleSaveJournal,
    handleGetJournal,
    handleListJournals,
    handleGenerateReview,
    handleListReviews,
    handleGetStats,
    handleGetSummary,
  } = await import("./handlers/trading")
  router.post("/api/v1/trading/trades", handleCreateTrade)
  router.get("/api/v1/trading/trades", handleListTrades)
  router.get("/api/v1/trading/trades/:id", handleGetTrade)
  router.put("/api/v1/trading/trades/:id", handleUpdateTrade)
  router.post("/api/v1/trading/journal", handleSaveJournal)
  router.get("/api/v1/trading/journal/:date", handleGetJournal)
  router.get("/api/v1/trading/journal", handleListJournals)
  router.post("/api/v1/trading/review", handleGenerateReview)
  router.get("/api/v1/trading/reviews", handleListReviews)
  router.get("/api/v1/trading/stats", handleGetStats)
  router.get("/api/v1/trading/summary", handleGetSummary)

  // PRD Generator routes (meeting minutes to PRD conversion)
  const {
    handleGeneratePRD,
    handleFromMeeting,
    handleListTemplates,
    handleListHistory: handleListPRDHistory,
    handleGetPRD,
  } = await import("./handlers/prd")
  router.post("/api/v1/prd/generate", handleGeneratePRD)
  router.post("/api/v1/prd/from-meeting", handleFromMeeting)
  router.get("/api/v1/prd/templates", handleListTemplates)
  router.get("/api/v1/prd/history", handleListPRDHistory)
  router.get("/api/v1/prd/:id", handleGetPRD)

  // Compliance and Audit routes (for regulatory requirements)
  const {
    handleListLogs,
    handleGetLog,
    handleGenerateReport: handleGenerateComplianceReport,
    handleListReports: handleListComplianceReports,
    handleGetReport,
    handleGetStatus: handleGetComplianceStatus,
    handleExport,
  } = await import("./handlers/compliance")
  router.get("/api/v1/compliance/logs", handleListLogs)
  router.get("/api/v1/compliance/logs/:id", handleGetLog)
  router.post("/api/v1/compliance/report", handleGenerateComplianceReport)
  router.get("/api/v1/compliance/reports", handleListComplianceReports)
  router.get("/api/v1/compliance/reports/:id", handleGetReport)
  router.get("/api/v1/compliance/status", handleGetComplianceStatus)
  router.post("/api/v1/compliance/export", handleExport)

  // Budget Management routes (for Admin dashboard cost control)
  const {
    getBudgetSummary,
    listBudgets,
    getBudget,
    createBudget,
    updateBudget,
    deleteBudget,
    listBudgetAlerts,
    acknowledgeBudgetAlert,
    recordBudgetSpend,
  } = await import("./handlers/budget")
  router.get("/api/v1/budgets/summary", getBudgetSummary)
  router.get("/api/v1/budgets", listBudgets)
  router.get("/api/v1/budgets/alerts", listBudgetAlerts)
  router.post("/api/v1/budgets/alerts/:id/acknowledge", acknowledgeBudgetAlert)
  router.get("/api/v1/budgets/:id", getBudget)
  router.post("/api/v1/budgets", createBudget)
  router.put("/api/v1/budgets/:id", updateBudget)
  router.delete("/api/v1/budgets/:id", deleteBudget)
  router.post("/api/v1/budgets/:id/record", recordBudgetSpend)

  // DLP (Data Leakage Prevention) routes (for Admin dashboard security)
  const {
    getDlpSummary,
    getDlpConfig,
    updateDlpConfig,
    listDlpRules,
    createDlpRule,
    updateDlpRule,
    deleteDlpRule,
    listDlpWhitelist,
    addDlpWhitelist,
    deleteDlpWhitelist,
    listDlpIncidents,
    scanContent,
  } = await import("./handlers/dlp")
  router.get("/api/v1/dlp/summary", getDlpSummary)
  router.get("/api/v1/dlp/config", getDlpConfig)
  router.put("/api/v1/dlp/config", updateDlpConfig)
  router.get("/api/v1/dlp/rules", listDlpRules)
  router.post("/api/v1/dlp/rules", createDlpRule)
  router.put("/api/v1/dlp/rules/:id", updateDlpRule)
  router.delete("/api/v1/dlp/rules/:id", deleteDlpRule)
  router.get("/api/v1/dlp/whitelist", listDlpWhitelist)
  router.post("/api/v1/dlp/whitelist", addDlpWhitelist)
  router.delete("/api/v1/dlp/whitelist/:id", deleteDlpWhitelist)
  router.get("/api/v1/dlp/incidents", listDlpIncidents)
  router.post("/api/v1/dlp/scan", scanContent)

  // Context Hub routes (Global Context Hub for cross-user/cross-department knowledge sharing)
  const {
    getHubStats,
    listHubEntries,
    getHubEntry,
    createHubEntry,
    updateHubEntry,
    deleteHubEntry,
    searchHub,
    markHelpful,
    listHubTags,
    listHubCategories,
    getAgentContext,
  } = await import("./handlers/context-hub")
  router.get("/api/v1/hub/stats", getHubStats)
  router.get("/api/v1/hub/entries", listHubEntries)
  router.get("/api/v1/hub/entries/:id", getHubEntry)
  router.post("/api/v1/hub/entries", createHubEntry)
  router.put("/api/v1/hub/entries/:id", updateHubEntry)
  router.delete("/api/v1/hub/entries/:id", deleteHubEntry)
  router.post("/api/v1/hub/search", searchHub)
  router.post("/api/v1/hub/entries/:id/helpful", markHelpful)
  router.get("/api/v1/hub/tags", listHubTags)
  router.get("/api/v1/hub/categories", listHubCategories)
  router.post("/api/v1/hub/agent-context", getAgentContext)

  // Unified Token Gateway routes (Department-level AI quota management)
  const {
    getGatewayStats,
    getGatewayConfig,
    updateGatewayConfig,
    listPools,
    getPool,
    createPool,
    updatePool,
    deletePool,
    listAllocations,
    getAllocation,
    upsertAllocation,
    deleteAllocation,
    recordUsage: recordGatewayUsage,
    checkQuota,
    listAlerts: listGatewayAlerts,
    acknowledgeAlert: acknowledgeGatewayAlert,
    resetDailyUsage,
    resetMonthlyUsage,
    getUsageHistory,
    gatewayHealth,
  } = await import("./handlers/token-gateway")
  router.get("/api/v1/gateway/stats", getGatewayStats)
  router.get("/api/v1/gateway/config", getGatewayConfig)
  router.put("/api/v1/gateway/config", updateGatewayConfig)
  router.get("/api/v1/gateway/pools", listPools)
  router.get("/api/v1/gateway/pools/:id", getPool)
  router.post("/api/v1/gateway/pools", createPool)
  router.put("/api/v1/gateway/pools/:id", updatePool)
  router.delete("/api/v1/gateway/pools/:id", deletePool)
  router.get("/api/v1/gateway/allocations", listAllocations)
  router.get("/api/v1/gateway/allocations/:userId", getAllocation)
  router.post("/api/v1/gateway/allocations", upsertAllocation)
  router.delete("/api/v1/gateway/allocations/:userId", deleteAllocation)
  router.post("/api/v1/gateway/record", recordGatewayUsage)
  router.get("/api/v1/gateway/check/:userId", checkQuota)
  router.get("/api/v1/gateway/alerts", listGatewayAlerts)
  router.post("/api/v1/gateway/alerts/:id/acknowledge", acknowledgeGatewayAlert)
  router.post("/api/v1/gateway/reset-daily", resetDailyUsage)
  router.post("/api/v1/gateway/reset-monthly", resetMonthlyUsage)
  router.get("/api/v1/gateway/usage", getUsageHistory)
  router.get("/api/v1/gateway/health", gatewayHealth)

  // Intelligent LLM Router routes (Phase 14: Task-based model routing with RBAC)
  const {
    routeRequest,
    getRouterConfig,
    updateRouterConfig,
    listRolePermissions,
    getRolePermission,
    updateRolePermission,
    listRouterModels,
    updateRouterModel,
    listClassificationRules,
    addClassificationRule,
    deleteClassificationRule,
    getRouterStats,
    getRouterHistory,
    classifyContent,
    routerHealth,
  } = await import("./handlers/llm-router")
  router.post("/api/v1/router/route", routeRequest)
  router.get("/api/v1/router/config", getRouterConfig)
  router.put("/api/v1/router/config", updateRouterConfig)
  router.get("/api/v1/router/roles", listRolePermissions)
  router.get("/api/v1/router/roles/:role", getRolePermission)
  router.put("/api/v1/router/roles/:role", updateRolePermission)
  router.get("/api/v1/router/models", listRouterModels)
  router.put("/api/v1/router/models/:modelId", updateRouterModel)
  router.get("/api/v1/router/rules", listClassificationRules)
  router.post("/api/v1/router/rules", addClassificationRule)
  router.delete("/api/v1/router/rules/:ruleId", deleteClassificationRule)
  router.get("/api/v1/router/stats", getRouterStats)
  router.get("/api/v1/router/history", getRouterHistory)
  router.post("/api/v1/router/classify", classifyContent)
  router.get("/api/v1/router/health", routerHealth)

  // Scheduler routes (Phase 15: Scheduled Task API Integration)
  const {
    listSchedulerTasks,
    createSchedulerTask,
    getSchedulerTask,
    updateSchedulerTask,
    deleteSchedulerTask,
    runSchedulerTask,
    getSchedulerHistory,
    getSchedulerExecution,
    getSchedulerConfig,
    updateSchedulerConfig,
    schedulerHealth,
  } = await import("./handlers/scheduler")
  router.get("/api/v1/scheduler/tasks", listSchedulerTasks)
  router.post("/api/v1/scheduler/tasks", createSchedulerTask)
  router.get("/api/v1/scheduler/tasks/:id", getSchedulerTask)
  router.put("/api/v1/scheduler/tasks/:id", updateSchedulerTask)
  router.delete("/api/v1/scheduler/tasks/:id", deleteSchedulerTask)
  router.post("/api/v1/scheduler/tasks/:id/run", runSchedulerTask)
  router.get("/api/v1/scheduler/history", getSchedulerHistory)
  router.get("/api/v1/scheduler/history/:id", getSchedulerExecution)
  router.get("/api/v1/scheduler/config", getSchedulerConfig)
  router.put("/api/v1/scheduler/config", updateSchedulerConfig)
  router.get("/api/v1/scheduler/health", schedulerHealth)

  // Causal Graph routes (Phase 16: 因果链图数据库)
  const {
    recordDecision,
    getDecision,
    recordAction,
    getAction,
    recordOutcome,
    getOutcome,
    getCausalChain,
    getCausalChains,
    queryCausalGraph,
    getCausalPatterns,
    getSuccessPatterns,
    getFailurePatterns,
    getCausalStats,
    getCausalSuggestions,
    getCausalTrends,
    getAgentInsights,
    getLessons,
    getCausalGraphData,
    getCausalMermaid,
    causalHealth,
  } = await import("./handlers/causal")
  router.post("/api/v1/causal/decisions", recordDecision)
  router.get("/api/v1/causal/decisions/:id", getDecision)
  router.post("/api/v1/causal/actions", recordAction)
  router.get("/api/v1/causal/actions/:id", getAction)
  router.post("/api/v1/causal/outcomes", recordOutcome)
  router.get("/api/v1/causal/outcomes/:id", getOutcome)
  router.get("/api/v1/causal/chain/:id", getCausalChain)
  router.get("/api/v1/causal/chains", getCausalChains)
  router.post("/api/v1/causal/query", queryCausalGraph)
  router.get("/api/v1/causal/patterns", getCausalPatterns)
  router.get("/api/v1/causal/patterns/success", getSuccessPatterns)
  router.get("/api/v1/causal/patterns/failure", getFailurePatterns)
  router.get("/api/v1/causal/stats", getCausalStats)
  router.post("/api/v1/causal/suggest", getCausalSuggestions)
  router.get("/api/v1/causal/trends", getCausalTrends)
  router.get("/api/v1/causal/insights/:agentId", getAgentInsights)
  router.get("/api/v1/causal/lessons/:outcomeId", getLessons)
  router.get("/api/v1/causal/graph", getCausalGraphData)
  router.get("/api/v1/causal/mermaid", getCausalMermaid)
  router.get("/api/v1/causal/health", causalHealth)
}

// ============================================================================
// Export router class for testing
// ============================================================================

export { Router }
