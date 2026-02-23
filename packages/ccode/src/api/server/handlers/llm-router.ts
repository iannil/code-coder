/**
 * Intelligent LLM Router Handler
 *
 * Routes LLM requests to optimal models based on:
 * - Task type classification (coding, analysis, chat, sensitive)
 * - User role permissions (RBAC)
 * - Budget constraints
 * - DLP sensitive content detection
 *
 * Part of Phase 14: Intelligent LLM Router
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { Global } from "@/global"
import path from "path"
import fs from "fs/promises"
import {
  type TaskType,
  type UserRole,
  type RoutingConfig,
  type RoutingDecision,
  type ClassificationRule,
  type RoutableModel,
  type RolePermission,
  DEFAULT_ROUTING_CONFIG,
  TASK_MODEL_PREFERENCES,
  canRoleAccessModel,
  findBestModel,
} from "@/provider/routing-rules"

// ============================================================================
// Storage
// ============================================================================

const ROUTER_DIR = path.join(Global.Path.data, "llm-router")

async function ensureRouterDir(): Promise<void> {
  await fs.mkdir(ROUTER_DIR, { recursive: true })
}

async function loadConfig(): Promise<RoutingConfig> {
  await ensureRouterDir()
  const configFile = path.join(ROUTER_DIR, "config.json")

  try {
    const content = await fs.readFile(configFile, "utf-8")
    return { ...DEFAULT_ROUTING_CONFIG, ...JSON.parse(content) }
  } catch {
    await saveConfig(DEFAULT_ROUTING_CONFIG)
    return DEFAULT_ROUTING_CONFIG
  }
}

async function saveConfig(config: RoutingConfig): Promise<void> {
  await ensureRouterDir()
  const configFile = path.join(ROUTER_DIR, "config.json")
  await fs.writeFile(configFile, JSON.stringify(config, null, 2))
}

async function loadRoutingHistory(): Promise<RoutingHistoryEntry[]> {
  await ensureRouterDir()
  const historyFile = path.join(ROUTER_DIR, "history.json")

  try {
    const content = await fs.readFile(historyFile, "utf-8")
    return JSON.parse(content)
  } catch {
    return []
  }
}

async function saveRoutingHistory(history: RoutingHistoryEntry[]): Promise<void> {
  await ensureRouterDir()
  const historyFile = path.join(ROUTER_DIR, "history.json")
  // Keep only last 1000 entries
  const trimmed = history.slice(-1000)
  await fs.writeFile(historyFile, JSON.stringify(trimmed, null, 2))
}

// ============================================================================
// Types
// ============================================================================

interface RoutingHistoryEntry {
  id: string
  timestamp: string
  userId?: string
  userRole: UserRole
  taskType: TaskType
  selectedModel: string
  isFallback: boolean
  reason: string
  contentPreview: string
}

interface RouteRequestInput {
  content: string
  userId?: string
  userRole?: UserRole
  preferredModel?: string
  agentName?: string
}

interface RouterStats {
  totalRequests: number
  byTaskType: Record<TaskType, number>
  byModel: Record<string, number>
  byRole: Record<UserRole, number>
  fallbackCount: number
  sensitiveCount: number
}

// ============================================================================
// Task Classifier
// ============================================================================

/**
 * Classify content into a task type based on configured rules
 */
export function classifyTask(
  content: string,
  agentName: string | undefined,
  rules: ClassificationRule[],
): { taskType: TaskType; matchedRule: string; confidence: number } {
  // Sort rules by priority (lower = higher priority)
  const sortedRules = [...rules].filter((r) => r.enabled).sort((a, b) => a.priority - b.priority)

  for (const rule of sortedRules) {
    // Check agent name match
    if (agentName && rule.agents.length > 0) {
      const normalizedAgent = agentName.startsWith("@") ? agentName : `@${agentName}`
      if (rule.agents.some((a) => a.toLowerCase() === normalizedAgent.toLowerCase())) {
        return { taskType: rule.taskType, matchedRule: rule.id, confidence: 1.0 }
      }
    }

    // Check pattern matches
    if (rule.patterns.length > 0) {
      for (const pattern of rule.patterns) {
        try {
          const regex = new RegExp(pattern, "i")
          if (regex.test(content)) {
            return { taskType: rule.taskType, matchedRule: rule.id, confidence: 0.9 }
          }
        } catch {
          // Skip invalid regex
        }
      }
    }

    // Check keyword matches
    if (rule.keywords.length > 0) {
      const lowerContent = content.toLowerCase()
      const matchedKeywords = rule.keywords.filter((kw) => lowerContent.includes(kw.toLowerCase()))
      if (matchedKeywords.length > 0) {
        const confidence = Math.min(0.5 + matchedKeywords.length * 0.1, 0.85)
        return { taskType: rule.taskType, matchedRule: rule.id, confidence }
      }
    }
  }

  // Default to chat if no rules match
  return { taskType: "chat", matchedRule: "default", confidence: 0.3 }
}

/**
 * Check if content contains DLP-sensitive patterns
 * Uses built-in pattern detection for performance and simplicity
 */
function checkDlpSensitive(content: string): boolean {
  const sensitivePatterns = [
    /\b\d{3}-\d{2}-\d{4}\b/, // SSN
    /\b(?:\d{4}[- ]?){3}\d{4}\b/, // Credit card
    /AKIA[0-9A-Z]{16}/, // AWS key
    /(sk|pk|api|token)[-_][a-zA-Z0-9]{20,}/i, // API keys
    /-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----/, // Private keys
    /password\s*[:=]\s*["'][^"']+["']/i, // Password assignments
  ]

  return sensitivePatterns.some((pattern) => pattern.test(content))
}

// ============================================================================
// Model Router
// ============================================================================

/**
 * Route a request to the optimal model
 */
export async function routeToModel(input: RouteRequestInput): Promise<RoutingDecision> {
  const config = await loadConfig()

  if (!config.enabled) {
    return {
      modelId: config.defaultModelId,
      modelName: "Default Model",
      provider: "anthropic",
      taskType: "chat",
      userRole: input.userRole || config.defaultRole,
      isFallback: false,
      reason: "Routing disabled, using default model",
      warnings: [],
    }
  }

  const userRole = input.userRole || config.defaultRole
  const warnings: string[] = []

  // Step 1: Classify task type
  let { taskType, matchedRule, confidence } = classifyTask(input.content, input.agentName, config.rules)

  // Step 2: Check for sensitive content (DLP integration)
  if (config.enableDlpIntegration) {
    const isSensitive = checkDlpSensitive(input.content)
    if (isSensitive) {
      taskType = "sensitive"
      matchedRule = "dlp-detection"
      confidence = 1.0
      warnings.push("Sensitive content detected, routing to local model")
    }
  }

  // Step 3: Find the best model for this task and role
  let selectedModel: RoutableModel | undefined

  // If user prefers a specific model, try to use it
  if (input.preferredModel) {
    const preferred = config.models.find((m) => m.id === input.preferredModel && m.available)
    if (preferred && canRoleAccessModel(userRole, preferred.id, preferred, config.rolePermissions)) {
      selectedModel = preferred
    } else if (preferred) {
      warnings.push(`Preferred model "${input.preferredModel}" not allowed for role "${userRole}"`)
    }
  }

  // If sensitive and forceLocalForSensitive, override to local model
  if (taskType === "sensitive" && config.forceLocalForSensitive) {
    const localModels = config.models.filter((m) => m.isLocal && m.available)
    selectedModel = localModels[0]
    if (!selectedModel) {
      warnings.push("No local model available for sensitive content, using fallback")
    }
  }

  // Find best model if not already selected
  if (!selectedModel) {
    selectedModel = findBestModel(taskType, userRole, config.models, config.rolePermissions)
  }

  // Final fallback
  const isFallback = !selectedModel
  if (!selectedModel) {
    // Try any available model
    selectedModel = config.models.find((m) => m.available)
    if (!selectedModel) {
      return {
        modelId: config.defaultModelId,
        modelName: "Default Model",
        provider: "anthropic",
        taskType,
        userRole,
        isFallback: true,
        reason: "No available models found",
        warnings: ["No models available, request may fail"],
      }
    }
    warnings.push("Using fallback model due to permission or availability constraints")
  }

  // Build reason string
  const reason = buildRoutingReason(taskType, matchedRule, confidence, selectedModel, userRole, isFallback)

  // Record routing decision
  await recordRoutingDecision({
    userId: input.userId,
    userRole,
    taskType,
    selectedModel: selectedModel.id,
    isFallback,
    reason,
    contentPreview: input.content.slice(0, 100),
  })

  return {
    modelId: selectedModel.id,
    modelName: selectedModel.name,
    provider: selectedModel.provider,
    taskType,
    userRole,
    isFallback,
    reason,
    warnings,
  }
}

function buildRoutingReason(
  taskType: TaskType,
  matchedRule: string,
  confidence: number,
  model: RoutableModel,
  role: UserRole,
  isFallback: boolean,
): string {
  const parts: string[] = []

  parts.push(`Task classified as "${taskType}" (rule: ${matchedRule}, confidence: ${(confidence * 100).toFixed(0)}%)`)
  parts.push(`Selected ${model.name} (${model.tier} tier) for role "${role}"`)

  if (isFallback) {
    parts.push("(fallback selection)")
  }

  if (model.isLocal) {
    parts.push("(local model for data privacy)")
  }

  return parts.join(". ")
}

async function recordRoutingDecision(entry: Omit<RoutingHistoryEntry, "id" | "timestamp">): Promise<void> {
  const history = await loadRoutingHistory()

  history.push({
    id: `route-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
    timestamp: new Date().toISOString(),
    ...entry,
  })

  await saveRoutingHistory(history)
}

// ============================================================================
// Request Body Helper
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

// ============================================================================
// Route Handlers
// ============================================================================

/**
 * POST /api/v1/router/route
 * Route a request to the optimal model
 */
export async function routeRequest(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as RouteRequestInput

    if (!input.content) {
      return errorResponse("Content is required", 400)
    }

    const decision = await routeToModel(input)

    return jsonResponse({ success: true, data: decision })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/router/config
 * Get routing configuration
 */
export async function getRouterConfig(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const config = await loadConfig()
    return jsonResponse({ success: true, data: config })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/v1/router/config
 * Update routing configuration
 */
export async function updateRouterConfig(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<RoutingConfig>

    const current = await loadConfig()
    const updated: RoutingConfig = {
      ...current,
      ...input,
    }

    await saveConfig(updated)

    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/router/roles
 * List role permissions
 */
export async function listRolePermissions(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const config = await loadConfig()
    return jsonResponse({ success: true, data: config.rolePermissions })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/router/roles/:role
 * Get a specific role's permissions
 */
export async function getRolePermission(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { role } = params

    if (!role) {
      return errorResponse("Role is required", 400)
    }

    const config = await loadConfig()
    const permission = config.rolePermissions.find((p) => p.role === role)

    if (!permission) {
      return errorResponse(`Role "${role}" not found`, 404)
    }

    return jsonResponse({ success: true, data: permission })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/v1/router/roles/:role
 * Update a role's permissions
 */
export async function updateRolePermission(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { role } = params

    if (!role) {
      return errorResponse("Role is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<RolePermission>

    const config = await loadConfig()
    const index = config.rolePermissions.findIndex((p) => p.role === role)

    if (index === -1) {
      return errorResponse(`Role "${role}" not found`, 404)
    }

    const updated: RolePermission = {
      ...config.rolePermissions[index],
      ...input,
      role: config.rolePermissions[index].role, // Don't allow role name change
    }

    // Create new array to maintain immutability
    const newPermissions = [...config.rolePermissions]
    newPermissions[index] = updated

    await saveConfig({ ...config, rolePermissions: newPermissions })

    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/router/models
 * List available models
 */
export async function listRouterModels(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const config = await loadConfig()
    return jsonResponse({ success: true, data: config.models })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/v1/router/models/:modelId
 * Update a model's configuration
 */
export async function updateRouterModel(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { modelId } = params

    if (!modelId) {
      return errorResponse("Model ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<RoutableModel>

    const config = await loadConfig()
    const index = config.models.findIndex((m) => m.id === modelId)

    if (index === -1) {
      return errorResponse(`Model "${modelId}" not found`, 404)
    }

    const updated: RoutableModel = {
      ...config.models[index],
      ...input,
      id: config.models[index].id, // Don't allow ID change
    }

    const newModels = [...config.models]
    newModels[index] = updated

    await saveConfig({ ...config, models: newModels })

    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/router/rules
 * List classification rules
 */
export async function listClassificationRules(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const config = await loadConfig()
    return jsonResponse({ success: true, data: config.rules })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/router/rules
 * Add a classification rule
 */
export async function addClassificationRule(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Omit<ClassificationRule, "id">

    if (!input.taskType) {
      return errorResponse("Task type is required", 400)
    }

    const config = await loadConfig()
    const rule: ClassificationRule = {
      id: `rule-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      taskType: input.taskType,
      priority: input.priority ?? 10,
      patterns: input.patterns ?? [],
      keywords: input.keywords ?? [],
      agents: input.agents ?? [],
      enabled: input.enabled ?? true,
    }

    // Validate patterns
    for (const pattern of rule.patterns) {
      try {
        new RegExp(pattern)
      } catch {
        return errorResponse(`Invalid regex pattern: ${pattern}`, 400)
      }
    }

    await saveConfig({ ...config, rules: [...config.rules, rule] })

    return jsonResponse({ success: true, data: rule }, 201)
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/v1/router/rules/:ruleId
 * Delete a classification rule
 */
export async function deleteClassificationRule(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { ruleId } = params

    if (!ruleId) {
      return errorResponse("Rule ID is required", 400)
    }

    const config = await loadConfig()
    const filtered = config.rules.filter((r) => r.id !== ruleId)

    if (filtered.length === config.rules.length) {
      return errorResponse(`Rule "${ruleId}" not found`, 404)
    }

    await saveConfig({ ...config, rules: filtered })

    return jsonResponse({ success: true, data: { deleted: ruleId } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/router/stats
 * Get routing statistics
 */
export async function getRouterStats(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const history = await loadRoutingHistory()

    const stats: RouterStats = {
      totalRequests: history.length,
      byTaskType: { coding: 0, analysis: 0, chat: 0, sensitive: 0 },
      byModel: {},
      byRole: { admin: 0, developer: 0, intern: 0, guest: 0 },
      fallbackCount: 0,
      sensitiveCount: 0,
    }

    for (const entry of history) {
      stats.byTaskType[entry.taskType]++
      stats.byModel[entry.selectedModel] = (stats.byModel[entry.selectedModel] || 0) + 1
      stats.byRole[entry.userRole]++
      if (entry.isFallback) stats.fallbackCount++
      if (entry.taskType === "sensitive") stats.sensitiveCount++
    }

    return jsonResponse({ success: true, data: stats })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/router/history
 * Get routing history
 */
export async function getRouterHistory(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = req.url
    const limitParam = url.searchParams.get("limit")
    const limit = limitParam ? parseInt(limitParam, 10) : 50

    let history = await loadRoutingHistory()

    // Sort by timestamp descending
    history.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())

    // Apply limit
    history = history.slice(0, limit)

    return jsonResponse({ success: true, data: history })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/router/classify
 * Classify content without routing (for testing)
 */
export async function classifyContent(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as { content: string; agentName?: string }

    if (!input.content) {
      return errorResponse("Content is required", 400)
    }

    const config = await loadConfig()
    const result = classifyTask(input.content, input.agentName, config.rules)

    // Also check DLP
    let dlpSensitive = false
    if (config.enableDlpIntegration) {
      dlpSensitive = checkDlpSensitive(input.content)
    }

    return jsonResponse({
      success: true,
      data: {
        ...result,
        dlpSensitive,
        finalTaskType: dlpSensitive ? "sensitive" : result.taskType,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/router/health
 * Health check
 */
export async function routerHealth(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const config = await loadConfig()

    return jsonResponse({
      success: true,
      data: {
        status: config.enabled ? "healthy" : "disabled",
        modelsAvailable: config.models.filter((m) => m.available).length,
        rulesActive: config.rules.filter((r) => r.enabled).length,
        rolesConfigured: config.rolePermissions.length,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
