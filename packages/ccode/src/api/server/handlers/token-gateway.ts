/**
 * Unified Token Gateway Handler
 *
 * Department-level AI quota management system.
 * Features:
 * - Department token pools
 * - User allocation from pools
 * - Alert thresholds and notifications
 * - Automatic downgrade/block behavior
 * - Usage analytics and reporting
 *
 * Part of Phase 6: Unified Token Gateway
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { homedir } from "os"
import { join } from "path"
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "fs"

// ============================================================================
// Types
// ============================================================================

type AlertLevel = "info" | "warning" | "critical"
type OverageAction = "block" | "downgrade" | "notify"
type ModelTier = "premium" | "standard" | "budget"

interface DepartmentPool {
  id: string
  name: string
  description: string
  /** Monthly token budget */
  monthlyBudget: number
  /** Current month's usage */
  monthlyUsed: number
  /** Alert threshold percentage (0-100) */
  alertThreshold: number
  /** What to do when budget exceeded */
  overageAction: OverageAction
  /** Model tier allowed when downgraded */
  downgradeTier: ModelTier
  /** Users in this department */
  users: string[]
  /** Created timestamp */
  createdAt: string
  /** Updated timestamp */
  updatedAt: string
}

interface UserAllocation {
  userId: string
  departmentId: string
  /** Personal daily limit (from department pool) */
  dailyLimit: number
  /** Personal monthly limit (from department pool) */
  monthlyLimit: number
  /** Current day's usage */
  dailyUsed: number
  /** Current month's usage */
  monthlyUsed: number
  /** Whether user is currently blocked */
  blocked: boolean
  /** Current model tier */
  currentTier: ModelTier
  /** Last usage timestamp */
  lastUsedAt: string
}

interface GatewayAlert {
  id: string
  departmentId: string
  userId?: string
  level: AlertLevel
  message: string
  threshold: number
  currentUsage: number
  acknowledged: boolean
  createdAt: string
  acknowledgedAt?: string
  acknowledgedBy?: string
}

interface UsageRecord {
  id: string
  userId: string
  departmentId: string
  inputTokens: number
  outputTokens: number
  model: string
  timestamp: string
  costUsd: number
}

interface GatewayStats {
  totalDepartments: number
  totalUsers: number
  totalTokensUsed: number
  totalCostUsd: number
  activeAlerts: number
  blockedUsers: number
  downgradedUsers: number
  topDepartments: Array<{ id: string; name: string; usage: number; budget: number }>
  topUsers: Array<{ userId: string; department: string; usage: number }>
}

interface GatewayConfig {
  /** Default monthly budget for new departments */
  defaultMonthlyBudget: number
  /** Default alert threshold percentage */
  defaultAlertThreshold: number
  /** Default overage action */
  defaultOverageAction: OverageAction
  /** Model pricing (per 1M tokens) */
  modelPricing: Record<string, { input: number; output: number }>
  /** Model tier mappings */
  modelTiers: Record<ModelTier, string[]>
  /** Enable automatic alerts */
  alertsEnabled: boolean
  /** Alert notification webhook */
  alertWebhook?: string
}

// ============================================================================
// Storage
// ============================================================================

const STORAGE_DIR = join(homedir(), ".codecoder", "token-gateway")

function ensureStorageDir(): void {
  if (!existsSync(STORAGE_DIR)) {
    mkdirSync(STORAGE_DIR, { recursive: true })
  }
}

function loadJson<T>(filename: string, defaultValue: T): T {
  ensureStorageDir()
  const filepath = join(STORAGE_DIR, filename)
  if (!existsSync(filepath)) {
    return defaultValue
  }
  try {
    return JSON.parse(readFileSync(filepath, "utf-8"))
  } catch {
    return defaultValue
  }
}

function saveJson(filename: string, data: unknown): void {
  ensureStorageDir()
  const filepath = join(STORAGE_DIR, filename)
  writeFileSync(filepath, JSON.stringify(data, null, 2))
}

const loadPools = (): DepartmentPool[] => loadJson("pools.json", [])
const savePools = (pools: DepartmentPool[]): void => saveJson("pools.json", pools)

const loadAllocations = (): UserAllocation[] => loadJson("allocations.json", [])
const saveAllocations = (allocations: UserAllocation[]): void => saveJson("allocations.json", allocations)

const loadAlerts = (): GatewayAlert[] => loadJson("alerts.json", [])
const saveAlerts = (alerts: GatewayAlert[]): void => saveJson("alerts.json", alerts)

const loadUsageRecords = (): UsageRecord[] => loadJson("usage.json", [])
const saveUsageRecords = (records: UsageRecord[]): void => saveJson("usage.json", records)

const loadConfig = (): GatewayConfig =>
  loadJson("config.json", {
    defaultMonthlyBudget: 10_000_000, // 10M tokens
    defaultAlertThreshold: 80,
    defaultOverageAction: "notify" as OverageAction,
    modelPricing: {
      "claude-3-opus": { input: 15, output: 75 },
      "claude-3-sonnet": { input: 3, output: 15 },
      "claude-3-haiku": { input: 0.25, output: 1.25 },
      "gpt-4": { input: 10, output: 30 },
      "gpt-4-turbo": { input: 10, output: 30 },
      "gpt-3.5-turbo": { input: 0.5, output: 1.5 },
    },
    modelTiers: {
      premium: ["claude-3-opus", "gpt-4"],
      standard: ["claude-3-sonnet", "gpt-4-turbo"],
      budget: ["claude-3-haiku", "gpt-3.5-turbo"],
    },
    alertsEnabled: true,
  })
const saveConfig = (config: GatewayConfig): void => saveJson("config.json", config)

// ============================================================================
// Helper Functions
// ============================================================================

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

function calculateCost(inputTokens: number, outputTokens: number, model: string, config: GatewayConfig): number {
  const pricing = config.modelPricing[model] || { input: 1, output: 1 }
  return (inputTokens * pricing.input + outputTokens * pricing.output) / 1_000_000
}

function checkAndCreateAlerts(
  departmentId: string,
  userId: string | undefined,
  currentUsage: number,
  budget: number,
  alerts: GatewayAlert[],
  config: GatewayConfig,
): GatewayAlert[] {
  if (!config.alertsEnabled) return alerts

  const percentage = (currentUsage / budget) * 100
  const existingAlert = alerts.find(
    (a) => a.departmentId === departmentId && a.userId === userId && !a.acknowledged && a.threshold === config.defaultAlertThreshold,
  )

  if (percentage >= config.defaultAlertThreshold && !existingAlert) {
    const level: AlertLevel = percentage >= 100 ? "critical" : percentage >= 90 ? "warning" : "info"

    const newAlert: GatewayAlert = {
      id: generateId(),
      departmentId,
      userId,
      level,
      message: userId
        ? `User ${userId} has used ${percentage.toFixed(1)}% of their allocation`
        : `Department has used ${percentage.toFixed(1)}% of monthly budget`,
      threshold: config.defaultAlertThreshold,
      currentUsage,
      acknowledged: false,
      createdAt: new Date().toISOString(),
    }

    return [...alerts, newAlert]
  }

  return alerts
}

function getModelForTier(tier: ModelTier, config: GatewayConfig): string {
  const models = config.modelTiers[tier]
  return models?.[0] || "claude-3-haiku"
}

// ============================================================================
// Route Handlers
// ============================================================================

/** GET /api/v1/gateway/stats - Get gateway statistics */
export async function getGatewayStats(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const pools = loadPools()
  const allocations = loadAllocations()
  const alerts = loadAlerts()
  const records = loadUsageRecords()

  const totalTokensUsed = records.reduce((sum, r) => sum + r.inputTokens + r.outputTokens, 0)
  const totalCostUsd = records.reduce((sum, r) => sum + r.costUsd, 0)

  const blockedUsers = allocations.filter((a) => a.blocked).length
  const downgradedUsers = allocations.filter((a) => a.currentTier !== "premium").length

  const departmentUsage: Record<string, number> = {}
  for (const record of records) {
    departmentUsage[record.departmentId] = (departmentUsage[record.departmentId] || 0) + record.inputTokens + record.outputTokens
  }

  const topDepartments = pools
    .map((p) => ({
      id: p.id,
      name: p.name,
      usage: departmentUsage[p.id] || 0,
      budget: p.monthlyBudget,
    }))
    .sort((a, b) => b.usage - a.usage)
    .slice(0, 5)

  const userUsage: Record<string, { usage: number; department: string }> = {}
  for (const record of records) {
    if (!userUsage[record.userId]) {
      userUsage[record.userId] = { usage: 0, department: record.departmentId }
    }
    userUsage[record.userId].usage += record.inputTokens + record.outputTokens
  }

  const topUsers = Object.entries(userUsage)
    .map(([userId, data]) => ({ userId, department: data.department, usage: data.usage }))
    .sort((a, b) => b.usage - a.usage)
    .slice(0, 10)

  const stats: GatewayStats = {
    totalDepartments: pools.length,
    totalUsers: allocations.length,
    totalTokensUsed,
    totalCostUsd,
    activeAlerts: alerts.filter((a) => !a.acknowledged).length,
    blockedUsers,
    downgradedUsers,
    topDepartments,
    topUsers,
  }

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: stats }),
  }
}

/** GET /api/v1/gateway/config - Get gateway configuration */
export async function getGatewayConfig(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const config = loadConfig()

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: config }),
  }
}

/** PUT /api/v1/gateway/config - Update gateway configuration */
export async function updateGatewayConfig(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const bodyText = req.body ? await new Response(req.body).text() : "{}"
  const body = JSON.parse(bodyText)
  const currentConfig = loadConfig()

  const updatedConfig: GatewayConfig = {
    ...currentConfig,
    ...body,
  }

  saveConfig(updatedConfig)

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: updatedConfig }),
  }
}

/** GET /api/v1/gateway/pools - List department pools */
export async function listPools(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const pools = loadPools()

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: pools }),
  }
}

/** GET /api/v1/gateway/pools/:id - Get a department pool */
export async function getPool(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const pools = loadPools()
  const pool = pools.find((p) => p.id === params.id)

  if (!pool) {
    return {
      status: 404,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "Pool not found" }),
    }
  }

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: pool }),
  }
}

/** POST /api/v1/gateway/pools - Create a department pool */
export async function createPool(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const bodyText = req.body ? await new Response(req.body).text() : "{}"
  const body = JSON.parse(bodyText)
  const config = loadConfig()
  const pools = loadPools()

  const pool: DepartmentPool = {
    id: generateId(),
    name: body.name || "New Department",
    description: body.description || "",
    monthlyBudget: body.monthlyBudget ?? config.defaultMonthlyBudget,
    monthlyUsed: 0,
    alertThreshold: body.alertThreshold ?? config.defaultAlertThreshold,
    overageAction: body.overageAction ?? config.defaultOverageAction,
    downgradeTier: body.downgradeTier ?? "standard",
    users: body.users || [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  }

  pools.push(pool)
  savePools(pools)

  return {
    status: 201,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: pool }),
  }
}

/** PUT /api/v1/gateway/pools/:id - Update a department pool */
export async function updatePool(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const bodyText = req.body ? await new Response(req.body).text() : "{}"
  const body = JSON.parse(bodyText)
  const pools = loadPools()
  const index = pools.findIndex((p) => p.id === params.id)

  if (index === -1) {
    return {
      status: 404,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "Pool not found" }),
    }
  }

  const updated: DepartmentPool = {
    ...pools[index],
    ...body,
    id: pools[index].id, // Don't allow ID change
    createdAt: pools[index].createdAt,
    updatedAt: new Date().toISOString(),
  }

  pools[index] = updated
  savePools(pools)

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: updated }),
  }
}

/** DELETE /api/v1/gateway/pools/:id - Delete a department pool */
export async function deletePool(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const pools = loadPools()
  const filtered = pools.filter((p) => p.id !== params.id)

  if (filtered.length === pools.length) {
    return {
      status: 404,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "Pool not found" }),
    }
  }

  savePools(filtered)

  // Also remove allocations for this pool
  const allocations = loadAllocations()
  const filteredAllocations = allocations.filter((a) => a.departmentId !== params.id)
  saveAllocations(filteredAllocations)

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true }),
  }
}

/** GET /api/v1/gateway/allocations - List user allocations */
export async function listAllocations(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const departmentId = req.url.searchParams.get("departmentId")
  const allocations = loadAllocations()

  const filtered = departmentId ? allocations.filter((a) => a.departmentId === departmentId) : allocations

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: filtered }),
  }
}

/** GET /api/v1/gateway/allocations/:userId - Get user allocation */
export async function getAllocation(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const allocations = loadAllocations()
  const allocation = allocations.find((a) => a.userId === params.userId)

  if (!allocation) {
    return {
      status: 404,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "Allocation not found" }),
    }
  }

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: allocation }),
  }
}

/** POST /api/v1/gateway/allocations - Create or update user allocation */
export async function upsertAllocation(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const bodyText = req.body ? await new Response(req.body).text() : "{}"
  const body = JSON.parse(bodyText)
  const allocations = loadAllocations()
  const pools = loadPools()

  if (!body.userId || !body.departmentId) {
    return {
      status: 400,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "userId and departmentId are required" }),
    }
  }

  const pool = pools.find((p) => p.id === body.departmentId)
  if (!pool) {
    return {
      status: 400,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "Department pool not found" }),
    }
  }

  const existingIndex = allocations.findIndex((a) => a.userId === body.userId)
  const allocation: UserAllocation = {
    userId: body.userId,
    departmentId: body.departmentId,
    dailyLimit: body.dailyLimit ?? Math.floor(pool.monthlyBudget / 30),
    monthlyLimit: body.monthlyLimit ?? Math.floor(pool.monthlyBudget / pool.users.length || pool.monthlyBudget),
    dailyUsed: existingIndex >= 0 ? allocations[existingIndex].dailyUsed : 0,
    monthlyUsed: existingIndex >= 0 ? allocations[existingIndex].monthlyUsed : 0,
    blocked: body.blocked ?? false,
    currentTier: body.currentTier ?? "premium",
    lastUsedAt: existingIndex >= 0 ? allocations[existingIndex].lastUsedAt : new Date().toISOString(),
  }

  if (existingIndex >= 0) {
    allocations[existingIndex] = allocation
  } else {
    allocations.push(allocation)
  }

  // Add user to pool if not already there
  if (!pool.users.includes(body.userId)) {
    pool.users.push(body.userId)
    savePools(pools)
  }

  saveAllocations(allocations)

  return {
    status: existingIndex >= 0 ? 200 : 201,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: allocation }),
  }
}

/** DELETE /api/v1/gateway/allocations/:userId - Delete user allocation */
export async function deleteAllocation(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const allocations = loadAllocations()
  const filtered = allocations.filter((a) => a.userId !== params.userId)

  if (filtered.length === allocations.length) {
    return {
      status: 404,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "Allocation not found" }),
    }
  }

  saveAllocations(filtered)

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true }),
  }
}

/** POST /api/v1/gateway/record - Record token usage (called by metering middleware) */
export async function recordUsage(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const bodyText = req.body ? await new Response(req.body).text() : "{}"
  const body = JSON.parse(bodyText)
  const { userId, inputTokens, outputTokens, model } = body

  if (!userId || inputTokens === undefined || outputTokens === undefined) {
    return {
      status: 400,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "userId, inputTokens, and outputTokens are required" }),
    }
  }

  const config = loadConfig()
  const allocations = loadAllocations()
  const pools = loadPools()
  let alerts = loadAlerts()
  const records = loadUsageRecords()

  // Find user's allocation
  const allocationIndex = allocations.findIndex((a) => a.userId === userId)
  if (allocationIndex === -1) {
    return {
      status: 404,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "User allocation not found" }),
    }
  }

  const allocation = allocations[allocationIndex]
  const pool = pools.find((p) => p.id === allocation.departmentId)

  if (!pool) {
    return {
      status: 404,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "Department pool not found" }),
    }
  }

  const totalTokens = inputTokens + outputTokens
  const costUsd = calculateCost(inputTokens, outputTokens, model || "claude-3-sonnet", config)

  // Update allocation
  allocation.dailyUsed += totalTokens
  allocation.monthlyUsed += totalTokens
  allocation.lastUsedAt = new Date().toISOString()

  // Update pool
  pool.monthlyUsed += totalTokens
  pool.updatedAt = new Date().toISOString()

  // Check if user should be blocked or downgraded
  if (allocation.monthlyUsed >= allocation.monthlyLimit) {
    if (pool.overageAction === "block") {
      allocation.blocked = true
    } else if (pool.overageAction === "downgrade") {
      allocation.currentTier = pool.downgradeTier
    }
  }

  // Check and create alerts
  alerts = checkAndCreateAlerts(pool.id, userId, allocation.monthlyUsed, allocation.monthlyLimit, alerts, config)
  alerts = checkAndCreateAlerts(pool.id, undefined, pool.monthlyUsed, pool.monthlyBudget, alerts, config)

  // Record usage
  const record: UsageRecord = {
    id: generateId(),
    userId,
    departmentId: allocation.departmentId,
    inputTokens,
    outputTokens,
    model: model || "unknown",
    timestamp: new Date().toISOString(),
    costUsd,
  }

  records.push(record)

  // Save all changes
  allocations[allocationIndex] = allocation
  saveAllocations(allocations)
  savePools(pools)
  saveAlerts(alerts)
  saveUsageRecords(records)

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      success: true,
      data: {
        record,
        allocation: {
          dailyUsed: allocation.dailyUsed,
          dailyLimit: allocation.dailyLimit,
          monthlyUsed: allocation.monthlyUsed,
          monthlyLimit: allocation.monthlyLimit,
          blocked: allocation.blocked,
          currentTier: allocation.currentTier,
        },
      },
    }),
  }
}

/** GET /api/v1/gateway/check/:userId - Check if user can make request */
export async function checkQuota(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const allocations = loadAllocations()
  const pools = loadPools()
  const config = loadConfig()

  const allocation = allocations.find((a) => a.userId === params.userId)

  if (!allocation) {
    // No allocation = no restrictions (for backwards compatibility)
    return {
      status: 200,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        success: true,
        data: {
          allowed: true,
          tier: "premium" as ModelTier,
          allowedModels: config.modelTiers.premium,
        },
      }),
    }
  }

  if (allocation.blocked) {
    return {
      status: 200,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        success: true,
        data: {
          allowed: false,
          reason: "User is blocked due to quota exceeded",
          tier: allocation.currentTier,
          allowedModels: [],
        },
      }),
    }
  }

  const pool = pools.find((p) => p.id === allocation.departmentId)
  const exceededDaily = allocation.dailyUsed >= allocation.dailyLimit
  const exceededMonthly = allocation.monthlyUsed >= allocation.monthlyLimit
  const poolExceeded = pool ? pool.monthlyUsed >= pool.monthlyBudget : false

  if (exceededDaily || exceededMonthly || poolExceeded) {
    if (pool?.overageAction === "block") {
      return {
        status: 200,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          success: true,
          data: {
            allowed: false,
            reason: exceededDaily ? "Daily limit exceeded" : exceededMonthly ? "Monthly limit exceeded" : "Department budget exceeded",
            tier: allocation.currentTier,
            allowedModels: [],
          },
        }),
      }
    }

    // Downgrade mode
    const downgradeTier = pool?.downgradeTier || "budget"
    return {
      status: 200,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        success: true,
        data: {
          allowed: true,
          tier: downgradeTier,
          allowedModels: config.modelTiers[downgradeTier],
          warning: "Quota exceeded, downgraded to " + downgradeTier + " tier",
        },
      }),
    }
  }

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      success: true,
      data: {
        allowed: true,
        tier: allocation.currentTier,
        allowedModels: config.modelTiers[allocation.currentTier],
        remaining: {
          daily: allocation.dailyLimit - allocation.dailyUsed,
          monthly: allocation.monthlyLimit - allocation.monthlyUsed,
        },
      },
    }),
  }
}

/** GET /api/v1/gateway/alerts - List alerts */
export async function listAlerts(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const acknowledged = req.url.searchParams.get("acknowledged")
  const departmentId = req.url.searchParams.get("departmentId")
  let alerts = loadAlerts()

  if (acknowledged !== null) {
    const showAcknowledged = acknowledged === "true"
    alerts = alerts.filter((a) => a.acknowledged === showAcknowledged)
  }

  if (departmentId) {
    alerts = alerts.filter((a) => a.departmentId === departmentId)
  }

  // Sort by createdAt descending
  alerts.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: alerts }),
  }
}

/** POST /api/v1/gateway/alerts/:id/acknowledge - Acknowledge an alert */
export async function acknowledgeAlert(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const bodyText = req.body ? await new Response(req.body).text() : "{}"
  const body = JSON.parse(bodyText)
  const alerts = loadAlerts()
  const index = alerts.findIndex((a) => a.id === params.id)

  if (index === -1) {
    return {
      status: 404,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ success: false, error: "Alert not found" }),
    }
  }

  alerts[index].acknowledged = true
  alerts[index].acknowledgedAt = new Date().toISOString()
  alerts[index].acknowledgedBy = body.acknowledgedBy || "admin"

  saveAlerts(alerts)

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: alerts[index] }),
  }
}

/** POST /api/v1/gateway/reset-daily - Reset daily usage (called by cron) */
export async function resetDailyUsage(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const allocations = loadAllocations()

  for (const allocation of allocations) {
    allocation.dailyUsed = 0
  }

  saveAllocations(allocations)

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, message: "Daily usage reset for all users" }),
  }
}

/** POST /api/v1/gateway/reset-monthly - Reset monthly usage (called by cron) */
export async function resetMonthlyUsage(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const allocations = loadAllocations()
  const pools = loadPools()

  for (const allocation of allocations) {
    allocation.monthlyUsed = 0
    allocation.blocked = false
    allocation.currentTier = "premium"
  }

  for (const pool of pools) {
    pool.monthlyUsed = 0
  }

  saveAllocations(allocations)
  savePools(pools)

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, message: "Monthly usage reset for all users and pools" }),
  }
}

/** GET /api/v1/gateway/usage - Get usage history */
export async function getUsageHistory(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const userId = req.url.searchParams.get("userId")
  const departmentId = req.url.searchParams.get("departmentId")
  const limitStr = req.url.searchParams.get("limit")
  const limit = limitStr ? parseInt(limitStr, 10) : 100

  let records = loadUsageRecords()

  if (userId) {
    records = records.filter((r) => r.userId === userId)
  }

  if (departmentId) {
    records = records.filter((r) => r.departmentId === departmentId)
  }

  // Sort by timestamp descending
  records.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())

  // Apply limit
  records = records.slice(0, limit)

  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, data: records }),
  }
}

/** GET /api/v1/gateway/health - Health check */
export async function gatewayHealth(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  return {
    status: 200,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ success: true, status: "healthy" }),
  }
}
