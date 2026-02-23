/**
 * Budget API Handler
 *
 * Provides budget management endpoints for cost control and alerts.
 * Integrates with Zero-Gateway for quota management.
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { Global } from "@/global"
import path from "path"
import fs from "fs/promises"

// ============================================================================
// Types
// ============================================================================

type BudgetPeriod = "daily" | "weekly" | "monthly"
type BudgetAlertSeverity = "info" | "warning" | "critical"

interface BudgetThreshold {
  percentage: number
  severity: BudgetAlertSeverity
  notify: boolean
  channels?: string[]
}

interface BudgetConfig {
  id: string
  name: string
  period: BudgetPeriod
  budget_usd: number
  spend_usd: number
  thresholds: BudgetThreshold[]
  enabled: boolean
  created_at: string
  updated_at: string
  team_id?: string
  user_ids?: string[]
}

interface BudgetAlert {
  id: string
  budget_id: string
  budget_name: string
  severity: BudgetAlertSeverity
  message: string
  threshold_percentage: number
  current_percentage: number
  current_spend_usd: number
  budget_usd: number
  triggered_at: string
  acknowledged: boolean
  acknowledged_at?: string
  acknowledged_by?: string
}

interface BudgetSummary {
  total_budget_usd: number
  total_spend_usd: number
  percentage_used: number
  period: BudgetPeriod
  active_alerts: number
  budgets: Array<{
    id: string
    name: string
    budget_usd: number
    spend_usd: number
    percentage: number
    status: "ok" | "warning" | "critical"
  }>
}

// ============================================================================
// Storage
// ============================================================================

const BUDGET_DIR = path.join(Global.Path.data, "budgets")

async function ensureBudgetDir(): Promise<void> {
  await fs.mkdir(BUDGET_DIR, { recursive: true })
}

async function loadBudgets(): Promise<BudgetConfig[]> {
  await ensureBudgetDir()
  const budgetFile = path.join(BUDGET_DIR, "budgets.json")

  try {
    const content = await fs.readFile(budgetFile, "utf-8")
    return JSON.parse(content)
  } catch {
    return []
  }
}

async function saveBudgets(budgets: BudgetConfig[]): Promise<void> {
  await ensureBudgetDir()
  const budgetFile = path.join(BUDGET_DIR, "budgets.json")
  await fs.writeFile(budgetFile, JSON.stringify(budgets, null, 2))
}

async function loadAlerts(): Promise<BudgetAlert[]> {
  await ensureBudgetDir()
  const alertFile = path.join(BUDGET_DIR, "alerts.json")

  try {
    const content = await fs.readFile(alertFile, "utf-8")
    return JSON.parse(content)
  } catch {
    return []
  }
}

async function saveAlerts(alerts: BudgetAlert[]): Promise<void> {
  await ensureBudgetDir()
  const alertFile = path.join(BUDGET_DIR, "alerts.json")
  await fs.writeFile(alertFile, JSON.stringify(alerts, null, 2))
}

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

function calculateBudgetStatus(percentage: number): "ok" | "warning" | "critical" {
  if (percentage >= 90) return "critical"
  if (percentage >= 70) return "warning"
  return "ok"
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/v1/budgets/summary
 * Get budget summary with all budgets and totals
 */
export async function getBudgetSummary(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const budgets = await loadBudgets()
    const alerts = await loadAlerts()

    const totalBudget = budgets.reduce((sum, b) => sum + b.budget_usd, 0)
    const totalSpend = budgets.reduce((sum, b) => sum + b.spend_usd, 0)
    const activeAlerts = alerts.filter((a) => !a.acknowledged).length

    const summary: BudgetSummary = {
      total_budget_usd: totalBudget,
      total_spend_usd: totalSpend,
      percentage_used: totalBudget > 0 ? Math.round((totalSpend / totalBudget) * 100) : 0,
      period: "monthly",
      active_alerts: activeAlerts,
      budgets: budgets.map((b) => ({
        id: b.id,
        name: b.name,
        budget_usd: b.budget_usd,
        spend_usd: b.spend_usd,
        percentage: b.budget_usd > 0 ? Math.round((b.spend_usd / b.budget_usd) * 100) : 0,
        status: calculateBudgetStatus(b.budget_usd > 0 ? (b.spend_usd / b.budget_usd) * 100 : 0),
      })),
    }

    return jsonResponse({ success: true, data: summary })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/budgets
 * List all budget configurations
 */
export async function listBudgets(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const budgets = await loadBudgets()
    return jsonResponse({ success: true, data: budgets })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/budgets/:id
 * Get a specific budget
 */
export async function getBudget(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Budget ID is required", 400)
    }

    const budgets = await loadBudgets()
    const budget = budgets.find((b) => b.id === id)

    if (!budget) {
      return errorResponse(`Budget "${id}" not found`, 404)
    }

    return jsonResponse({ success: true, data: budget })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/budgets
 * Create a new budget
 */
export async function createBudget(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<BudgetConfig>

    if (!input.name) {
      return errorResponse("Budget name is required", 400)
    }
    if (!input.budget_usd || input.budget_usd <= 0) {
      return errorResponse("Valid budget amount is required", 400)
    }

    const budgets = await loadBudgets()
    const now = new Date().toISOString()

    const budget: BudgetConfig = {
      id: `budget-${Date.now()}`,
      name: input.name,
      period: input.period ?? "monthly",
      budget_usd: input.budget_usd,
      spend_usd: 0,
      thresholds: input.thresholds ?? [
        { percentage: 70, severity: "warning", notify: true },
        { percentage: 90, severity: "critical", notify: true },
      ],
      enabled: input.enabled ?? true,
      created_at: now,
      updated_at: now,
      team_id: input.team_id,
      user_ids: input.user_ids,
    }

    budgets.push(budget)
    await saveBudgets(budgets)

    return jsonResponse({ success: true, data: budget }, 201)
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/v1/budgets/:id
 * Update a budget
 */
export async function updateBudget(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Budget ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<BudgetConfig>

    const budgets = await loadBudgets()
    const index = budgets.findIndex((b) => b.id === id)

    if (index === -1) {
      return errorResponse(`Budget "${id}" not found`, 404)
    }

    const updated: BudgetConfig = {
      ...budgets[index],
      ...input,
      id, // Preserve ID
      updated_at: new Date().toISOString(),
    }

    budgets[index] = updated
    await saveBudgets(budgets)

    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/v1/budgets/:id
 * Delete a budget
 */
export async function deleteBudget(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Budget ID is required", 400)
    }

    const budgets = await loadBudgets()
    const filtered = budgets.filter((b) => b.id !== id)

    if (filtered.length === budgets.length) {
      return errorResponse(`Budget "${id}" not found`, 404)
    }

    await saveBudgets(filtered)

    return jsonResponse({ success: true, data: { deleted: id } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/budgets/alerts
 * List budget alerts
 */
export async function listBudgetAlerts(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = req.url
    const acknowledged = url.searchParams.get("acknowledged")

    let alerts = await loadAlerts()

    if (acknowledged !== null) {
      const filterAcknowledged = acknowledged === "true"
      alerts = alerts.filter((a) => a.acknowledged === filterAcknowledged)
    }

    return jsonResponse({ success: true, data: alerts })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/budgets/alerts/:id/acknowledge
 * Acknowledge a budget alert
 */
export async function acknowledgeBudgetAlert(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Alert ID is required", 400)
    }

    const alerts = await loadAlerts()
    const index = alerts.findIndex((a) => a.id === id)

    if (index === -1) {
      return errorResponse(`Alert "${id}" not found`, 404)
    }

    alerts[index] = {
      ...alerts[index],
      acknowledged: true,
      acknowledged_at: new Date().toISOString(),
    }

    await saveAlerts(alerts)

    return jsonResponse({ success: true, data: alerts[index] })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/budgets/:id/record
 * Record spend against a budget (internal API for metering integration)
 */
export async function recordBudgetSpend(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Budget ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as { amount_usd: number }

    if (!input.amount_usd || input.amount_usd < 0) {
      return errorResponse("Valid spend amount is required", 400)
    }

    const budgets = await loadBudgets()
    const index = budgets.findIndex((b) => b.id === id)

    if (index === -1) {
      return errorResponse(`Budget "${id}" not found`, 404)
    }

    // Update spend
    budgets[index].spend_usd += input.amount_usd
    budgets[index].updated_at = new Date().toISOString()

    // Check thresholds and create alerts
    const percentage = (budgets[index].spend_usd / budgets[index].budget_usd) * 100
    const alerts = await loadAlerts()

    for (const threshold of budgets[index].thresholds) {
      if (percentage >= threshold.percentage) {
        // Check if alert already exists
        const existingAlert = alerts.find(
          (a) => a.budget_id === id && a.threshold_percentage === threshold.percentage && !a.acknowledged
        )

        if (!existingAlert) {
          alerts.push({
            id: `alert-${Date.now()}`,
            budget_id: id,
            budget_name: budgets[index].name,
            severity: threshold.severity,
            message: `${budgets[index].name} has exceeded ${threshold.percentage}% of budget`,
            threshold_percentage: threshold.percentage,
            current_percentage: Math.round(percentage),
            current_spend_usd: budgets[index].spend_usd,
            budget_usd: budgets[index].budget_usd,
            triggered_at: new Date().toISOString(),
            acknowledged: false,
          })
        }
      }
    }

    await saveBudgets(budgets)
    await saveAlerts(alerts)

    return jsonResponse({
      success: true,
      data: {
        budget: budgets[index],
        alerts_triggered: alerts.filter((a) => a.budget_id === id && !a.acknowledged).length,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
