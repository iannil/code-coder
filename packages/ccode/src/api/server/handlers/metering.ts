/**
 * Metering API Handler
 *
 * Provides metering and quota endpoints for the Admin dashboard.
 * These endpoints expose token usage statistics and quota management.
 *
 * GET /api/v1/metering/usage - Overall usage statistics
 * GET /api/v1/metering/users - Per-user usage breakdown
 * GET /api/v1/metering/quotas - Quota limits
 * PUT /api/v1/metering/quotas/:userId - Update user quota
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"

// ============================================================================
// Types
// ============================================================================

interface UsageRecord {
  user_id: string
  date: string
  input_tokens: number
  output_tokens: number
  requests: number
}

interface UserQuota {
  user_id: string
  daily_input_limit: number
  daily_output_limit: number
  monthly_input_limit: number
  monthly_output_limit: number
}

interface UsageMetrics {
  total_users: number
  active_users_24h: number
  tokens_used_24h: number
  tokens_used_30d: number
  requests_24h: number
  requests_30d: number
}

interface UserUsageReport {
  user_id: string
  name: string
  email?: string
  role: string
  daily_usage: {
    input_tokens: number
    output_tokens: number
    requests: number
  }
  monthly_usage: {
    input_tokens: number
    output_tokens: number
    requests: number
  }
  quota: UserQuota
  percentage_used: number
  last_active?: string
}

// ============================================================================
// Storage Keys
// ============================================================================

const METERING_NAMESPACE = "metering"
const QUOTA_NAMESPACE = "quota"
const USER_NAMESPACE = "user"

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

function getToday(): string {
  return new Date().toISOString().split("T")[0]
}

function getThirtyDaysAgo(): string {
  const date = new Date()
  date.setDate(date.getDate() - 30)
  return date.toISOString().split("T")[0]
}

// ============================================================================
// In-Memory Fallback Data
// ============================================================================

const DEFAULT_QUOTA: UserQuota = {
  user_id: "default",
  daily_input_limit: 1_000_000,
  daily_output_limit: 500_000,
  monthly_input_limit: 30_000_000,
  monthly_output_limit: 15_000_000,
}

// Simulated usage data for development
const mockUsageData: Map<string, UsageRecord[]> = new Map()
const mockQuotas: Map<string, UserQuota> = new Map()
const mockUsers: Map<string, { name: string; email: string; role: string; lastActive: string }> = new Map([
  ["user-admin", { name: "Admin User", email: "admin@company.com", role: "admin", lastActive: new Date().toISOString() }],
  ["user-dev-1", { name: "John Developer", email: "john@company.com", role: "developer", lastActive: new Date().toISOString() }],
  ["user-dev-2", { name: "Jane Smith", email: "jane@company.com", role: "developer", lastActive: new Date().toISOString() }],
])

// Initialize mock data
function initMockData() {
  const today = getToday()

  if (mockUsageData.size === 0) {
    mockUsageData.set("user-admin", [
      { user_id: "user-admin", date: today, input_tokens: 50000, output_tokens: 75000, requests: 45 },
    ])
    mockUsageData.set("user-dev-1", [
      { user_id: "user-dev-1", date: today, input_tokens: 200000, output_tokens: 250000, requests: 120 },
    ])
    mockUsageData.set("user-dev-2", [
      { user_id: "user-dev-2", date: today, input_tokens: 35000, output_tokens: 54000, requests: 28 },
    ])
  }

  if (mockQuotas.size === 0) {
    mockQuotas.set("user-admin", { ...DEFAULT_QUOTA, user_id: "user-admin" })
    mockQuotas.set("user-dev-1", { ...DEFAULT_QUOTA, user_id: "user-dev-1", daily_input_limit: 500_000, daily_output_limit: 250_000 })
    mockQuotas.set("user-dev-2", { ...DEFAULT_QUOTA, user_id: "user-dev-2" })
  }
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/v1/metering/usage
 *
 * Get overall usage statistics.
 */
export async function getUsage(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    initMockData()

    // Calculate aggregate metrics
    let tokens24h = 0
    let tokens30d = 0
    let requests24h = 0
    let requests30d = 0
    const activeUsers = new Set<string>()

    const today = getToday()
    const thirtyDaysAgo = getThirtyDaysAgo()

    for (const [userId, records] of mockUsageData) {
      for (const record of records) {
        if (record.date === today) {
          tokens24h += record.input_tokens + record.output_tokens
          requests24h += record.requests
          activeUsers.add(userId)
        }
        if (record.date >= thirtyDaysAgo) {
          tokens30d += record.input_tokens + record.output_tokens
          requests30d += record.requests
        }
      }
    }

    const metrics: UsageMetrics = {
      total_users: mockUsers.size,
      active_users_24h: activeUsers.size,
      tokens_used_24h: tokens24h,
      tokens_used_30d: tokens30d,
      requests_24h: requests24h,
      requests_30d: requests30d,
    }

    return jsonResponse({
      success: true,
      data: metrics,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/metering/users
 *
 * Get per-user usage breakdown.
 */
export async function getUsersUsage(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    initMockData()

    const today = getToday()
    const reports: UserUsageReport[] = []

    for (const [userId, userInfo] of mockUsers) {
      const records = mockUsageData.get(userId) ?? []
      const quota = mockQuotas.get(userId) ?? { ...DEFAULT_QUOTA, user_id: userId }

      // Calculate daily and monthly usage
      const dailyUsage = { input_tokens: 0, output_tokens: 0, requests: 0 }
      const monthlyUsage = { input_tokens: 0, output_tokens: 0, requests: 0 }
      const thirtyDaysAgo = getThirtyDaysAgo()

      for (const record of records) {
        if (record.date === today) {
          dailyUsage.input_tokens += record.input_tokens
          dailyUsage.output_tokens += record.output_tokens
          dailyUsage.requests += record.requests
        }
        if (record.date >= thirtyDaysAgo) {
          monthlyUsage.input_tokens += record.input_tokens
          monthlyUsage.output_tokens += record.output_tokens
          monthlyUsage.requests += record.requests
        }
      }

      // Calculate percentage used (based on daily limits)
      const totalDailyUsed = dailyUsage.input_tokens + dailyUsage.output_tokens
      const totalDailyLimit = quota.daily_input_limit + quota.daily_output_limit
      const percentageUsed = totalDailyLimit > 0 ? (totalDailyUsed / totalDailyLimit) * 100 : 0

      reports.push({
        user_id: userId,
        name: userInfo.name,
        email: userInfo.email,
        role: userInfo.role,
        daily_usage: dailyUsage,
        monthly_usage: monthlyUsage,
        quota,
        percentage_used: Math.round(percentageUsed * 100) / 100,
        last_active: userInfo.lastActive,
      })
    }

    // Sort by usage descending
    reports.sort((a, b) => {
      const aTotal = a.daily_usage.input_tokens + a.daily_usage.output_tokens
      const bTotal = b.daily_usage.input_tokens + b.daily_usage.output_tokens
      return bTotal - aTotal
    })

    return jsonResponse({
      success: true,
      data: reports,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/metering/quotas
 *
 * Get all quota configurations.
 */
export async function getQuotas(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    initMockData()

    const quotas: UserQuota[] = []
    for (const quota of mockQuotas.values()) {
      quotas.push(quota)
    }

    return jsonResponse({
      success: true,
      data: {
        default: DEFAULT_QUOTA,
        users: quotas,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/v1/metering/quotas/:userId
 *
 * Update quota for a specific user.
 */
export async function updateQuota(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { userId } = params

    if (!userId) {
      return errorResponse("userId is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<UserQuota>

    initMockData()

    // Get existing quota or use default
    const existingQuota = mockQuotas.get(userId) ?? { ...DEFAULT_QUOTA, user_id: userId }

    // Update quota with provided values
    const updatedQuota: UserQuota = {
      user_id: userId,
      daily_input_limit: input.daily_input_limit ?? existingQuota.daily_input_limit,
      daily_output_limit: input.daily_output_limit ?? existingQuota.daily_output_limit,
      monthly_input_limit: input.monthly_input_limit ?? existingQuota.monthly_input_limit,
      monthly_output_limit: input.monthly_output_limit ?? existingQuota.monthly_output_limit,
    }

    mockQuotas.set(userId, updatedQuota)

    return jsonResponse({
      success: true,
      data: updatedQuota,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/metering/record
 *
 * Record usage for a user. Internal endpoint for tracking token consumption.
 */
export async function recordUsage(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as {
      user_id: string
      input_tokens: number
      output_tokens: number
    }

    if (!input.user_id) {
      return errorResponse("user_id is required", 400)
    }

    initMockData()

    const today = getToday()
    const records = mockUsageData.get(input.user_id) ?? []

    // Find or create today's record
    let todayRecord = records.find((r) => r.date === today)
    if (!todayRecord) {
      todayRecord = {
        user_id: input.user_id,
        date: today,
        input_tokens: 0,
        output_tokens: 0,
        requests: 0,
      }
      records.push(todayRecord)
    }

    // Update record
    todayRecord.input_tokens += input.input_tokens || 0
    todayRecord.output_tokens += input.output_tokens || 0
    todayRecord.requests += 1

    mockUsageData.set(input.user_id, records)

    return jsonResponse({
      success: true,
      data: todayRecord,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
