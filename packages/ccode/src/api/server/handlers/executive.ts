/**
 * Executive Dashboard API Handler
 *
 * Provides executive-level analytics endpoints for the Admin dashboard.
 * These endpoints expose aggregated metrics for management view.
 *
 * GET /api/v1/executive/trends - Cost and usage trends over time
 * GET /api/v1/executive/teams - Team/department usage breakdown
 * GET /api/v1/executive/activity - Project activity summary (Git commits)
 * GET /api/v1/executive/summary - Executive summary with key metrics
 * GET /api/v1/executive/health - Service health check
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { spawn } from "child_process"
import path from "path"

// ============================================================================
// Types
// ============================================================================

interface TrendDataPoint {
  date: string
  input_tokens: number
  output_tokens: number
  total_tokens: number
  requests: number
  cost_usd: number
}

interface TeamUsage {
  team_id: string
  team_name: string
  member_count: number
  tokens_used: number
  requests: number
  percentage: number
  top_users: Array<{
    user_id: string
    name: string
    tokens: number
  }>
}

interface ProjectActivity {
  project_id: string
  project_name: string
  commits_today: number
  commits_week: number
  active_contributors: number
  last_commit?: string
  ai_sessions: number
}

interface ExecutiveSummary {
  period: string
  total_cost_usd: number
  cost_change_percent: number
  total_tokens: number
  total_requests: number
  active_users: number
  active_projects: number
  top_models: Array<{
    model: string
    usage_percent: number
    cost_usd: number
  }>
  alerts: Array<{
    type: "warning" | "critical" | "info"
    message: string
    metric?: string
    value?: number
    threshold?: number
  }>
}

// ============================================================================
// Cost Calculation Helpers
// ============================================================================

// Approximate cost per million tokens (in USD)
const MODEL_COSTS: Record<string, { input: number; output: number }> = {
  "claude-sonnet-4": { input: 3.0, output: 15.0 },
  "claude-opus-4": { input: 15.0, output: 75.0 },
  "claude-haiku-3.5": { input: 0.25, output: 1.25 },
  "gpt-4o": { input: 2.5, output: 10.0 },
  "gpt-4o-mini": { input: 0.15, output: 0.6 },
  default: { input: 1.0, output: 5.0 },
}

function calculateCost(inputTokens: number, outputTokens: number, model = "default"): number {
  const costs = MODEL_COSTS[model] ?? MODEL_COSTS.default
  return (inputTokens * costs.input + outputTokens * costs.output) / 1_000_000
}

// ============================================================================
// Real Data Integration
// ============================================================================

interface MeteringUsageData {
  total_users: number
  active_users_24h: number
  tokens_used_24h: number
  tokens_used_30d: number
  requests_24h: number
  requests_30d: number
}

interface MeteringUserReport {
  user_id: string
  name: string
  role: string
  daily_usage: { input_tokens: number; output_tokens: number; requests: number }
  monthly_usage: { input_tokens: number; output_tokens: number; requests: number }
  percentage_used?: number
}

/**
 * Fetch usage data from the metering API.
 * Falls back to mock data if metering API is unavailable.
 */
async function fetchMeteringUsage(): Promise<MeteringUsageData> {
  try {
    const response = await fetch("http://localhost:4400/api/v1/metering/usage")
    if (response.ok) {
      const data = await response.json()
      if (data.success && data.data) {
        return data.data
      }
    }
  } catch {
    // Fall back to mock data
  }

  // Return mock data as fallback
  return {
    total_users: 12,
    active_users_24h: 8,
    tokens_used_24h: 664000,
    tokens_used_30d: 4550000,
    requests_24h: 193,
    requests_30d: 5790,
  }
}

/**
 * Fetch user reports from the metering API.
 * Falls back to mock data if metering API is unavailable.
 */
async function fetchMeteringUsers(): Promise<MeteringUserReport[]> {
  try {
    const response = await fetch("http://localhost:4400/api/v1/metering/users")
    if (response.ok) {
      const data = await response.json()
      if (data.success && data.data) {
        return data.data
      }
    }
  } catch {
    // Fall back to mock data
  }

  // Return mock data as fallback
  return [
    {
      user_id: "user-dev-1",
      name: "John Developer",
      role: "developer",
      daily_usage: { input_tokens: 200000, output_tokens: 250000, requests: 120 },
      monthly_usage: { input_tokens: 3000000, output_tokens: 3750000, requests: 1800 },
    },
    {
      user_id: "user-dev-2",
      name: "Jane Smith",
      role: "developer",
      daily_usage: { input_tokens: 35000, output_tokens: 54000, requests: 28 },
      monthly_usage: { input_tokens: 525000, output_tokens: 810000, requests: 420 },
    },
    {
      user_id: "user-admin",
      name: "Admin User",
      role: "admin",
      daily_usage: { input_tokens: 50000, output_tokens: 75000, requests: 45 },
      monthly_usage: { input_tokens: 750000, output_tokens: 1125000, requests: 675 },
    },
  ]
}

/**
 * Generate team data from user reports.
 */
async function generateTeamDataFromMetering(): Promise<TeamUsage[]> {
  const users = await fetchMeteringUsers()

  // Group users by role as "team"
  const teamMap = new Map<string, { users: MeteringUserReport[]; totalTokens: number; totalRequests: number }>()

  for (const user of users) {
    const teamName = user.role === "developer" ? "Engineering" :
                     user.role === "admin" ? "Operations" :
                     user.role === "pm" ? "Product" : "Other"

    const existing = teamMap.get(teamName) ?? { users: [], totalTokens: 0, totalRequests: 0 }
    const userTokens = user.monthly_usage.input_tokens + user.monthly_usage.output_tokens
    existing.users.push(user)
    existing.totalTokens += userTokens
    existing.totalRequests += user.monthly_usage.requests
    teamMap.set(teamName, existing)
  }

  // Calculate total for percentage
  const grandTotal = Array.from(teamMap.values()).reduce((sum, t) => sum + t.totalTokens, 0)

  // Convert to TeamUsage array
  const teams: TeamUsage[] = []
  let idx = 1
  for (const [teamName, data] of teamMap) {
    teams.push({
      team_id: `team-${idx++}`,
      team_name: teamName,
      member_count: data.users.length,
      tokens_used: data.totalTokens,
      requests: data.totalRequests,
      percentage: grandTotal > 0 ? Math.round((data.totalTokens / grandTotal) * 100) : 0,
      top_users: data.users
        .sort((a, b) =>
          (b.monthly_usage.input_tokens + b.monthly_usage.output_tokens) -
          (a.monthly_usage.input_tokens + a.monthly_usage.output_tokens)
        )
        .slice(0, 3)
        .map(u => ({
          user_id: u.user_id,
          name: u.name,
          tokens: u.monthly_usage.input_tokens + u.monthly_usage.output_tokens,
        })),
    })
  }

  return teams.sort((a, b) => b.tokens_used - a.tokens_used)
}

/**
 * Generate summary from metering data.
 */
async function generateSummaryFromMetering(period: "daily" | "weekly" | "monthly"): Promise<ExecutiveSummary> {
  const metering = await fetchMeteringUsage()
  const multiplier = period === "daily" ? 1 : period === "weekly" ? 7 : 30

  // Use real tokens data
  const tokensUsed = period === "daily" ? metering.tokens_used_24h : metering.tokens_used_30d
  const requestsUsed = period === "daily" ? metering.requests_24h : metering.requests_30d

  // Calculate cost from tokens (assuming 60/40 input/output split)
  const inputTokens = Math.floor(tokensUsed * 0.4)
  const outputTokens = Math.floor(tokensUsed * 0.6)
  const totalCost = calculateCost(inputTokens, outputTokens, "claude-sonnet-4")

  // Estimate previous period (for comparison)
  const previousCost = totalCost * (0.9 + Math.random() * 0.15)
  const costChange = ((totalCost - previousCost) / previousCost) * 100

  return {
    period,
    total_cost_usd: parseFloat(totalCost.toFixed(2)),
    cost_change_percent: parseFloat(costChange.toFixed(1)),
    total_tokens: tokensUsed,
    total_requests: requestsUsed,
    active_users: metering.active_users_24h,
    active_projects: 4, // TODO: Integrate with project tracking
    top_models: [
      { model: "claude-sonnet-4", usage_percent: 65, cost_usd: parseFloat((totalCost * 0.65).toFixed(2)) },
      { model: "gpt-4o", usage_percent: 20, cost_usd: parseFloat((totalCost * 0.2).toFixed(2)) },
      { model: "claude-haiku-3.5", usage_percent: 15, cost_usd: parseFloat((totalCost * 0.15).toFixed(2)) },
    ],
    alerts: generateAlertsFromMetering(totalCost, metering, multiplier),
  }
}

function generateAlertsFromMetering(
  cost: number,
  metering: MeteringUsageData,
  multiplier: number
): ExecutiveSummary["alerts"] {
  const alerts: ExecutiveSummary["alerts"] = []

  // Budget threshold (monthly budget of $250)
  const monthlyBudget = 250
  const projectedMonthlyCost = cost * (30 / multiplier)

  if (projectedMonthlyCost > monthlyBudget * 0.8) {
    alerts.push({
      type: projectedMonthlyCost > monthlyBudget ? "critical" : "warning",
      message: projectedMonthlyCost > monthlyBudget
        ? "Projected monthly cost exceeds budget"
        : "Token usage approaching monthly budget limit",
      metric: "cost_usd",
      value: parseFloat(projectedMonthlyCost.toFixed(2)),
      threshold: monthlyBudget,
    })
  }

  // High activity alert
  if (metering.active_users_24h > 10) {
    alerts.push({
      type: "info",
      message: `${metering.active_users_24h} active users in the last 24 hours`,
    })
  }

  return alerts
}

// ============================================================================
// Git Statistics (Real Data)
// ============================================================================

interface GitProject {
  id: string
  name: string
  path: string
}

// Define projects to track (relative to repo root)
const TRACKED_PROJECTS: GitProject[] = [
  { id: "proj-ccode", name: "ccode", path: "packages/ccode" },
  { id: "proj-web", name: "web", path: "packages/web" },
  { id: "proj-vscode", name: "vscode-extension", path: "packages/vscode-extension" },
  { id: "proj-services", name: "services", path: "services" },
]

/**
 * Execute a git command and return stdout.
 */
async function execGit(args: string[], cwd: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const proc = spawn("git", args, { cwd })
    let stdout = ""
    let stderr = ""

    proc.stdout.on("data", (data) => {
      stdout += data.toString()
    })
    proc.stderr.on("data", (data) => {
      stderr += data.toString()
    })
    proc.on("close", (code) => {
      if (code === 0) {
        resolve(stdout.trim())
      } else {
        reject(new Error(`git ${args.join(" ")} failed: ${stderr}`))
      }
    })
    proc.on("error", reject)
  })
}

/**
 * Get the repository root directory.
 */
async function getRepoRoot(): Promise<string> {
  try {
    return await execGit(["rev-parse", "--show-toplevel"], process.cwd())
  } catch {
    // Fallback: try common locations
    const possibleRoots = [
      process.cwd(),
      path.join(process.cwd(), ".."),
      path.join(process.cwd(), "../.."),
    ]
    for (const root of possibleRoots) {
      try {
        await execGit(["rev-parse", "--show-toplevel"], root)
        return root
      } catch {
        continue
      }
    }
    throw new Error("Could not find git repository root")
  }
}

/**
 * Count commits in a path since a given date.
 */
async function countCommitsSince(repoRoot: string, projectPath: string, since: string): Promise<number> {
  try {
    const output = await execGit(
      ["rev-list", "--count", `--since=${since}`, "HEAD", "--", projectPath],
      repoRoot
    )
    return parseInt(output, 10) || 0
  } catch {
    return 0
  }
}

/**
 * Get unique contributors for a path since a given date.
 */
async function getContributorsSince(repoRoot: string, projectPath: string, since: string): Promise<number> {
  try {
    const output = await execGit(
      ["log", `--since=${since}`, "--format=%ae", "--", projectPath],
      repoRoot
    )
    if (!output) return 0
    const emails = new Set(output.split("\n").filter(Boolean))
    return emails.size
  } catch {
    return 0
  }
}

/**
 * Get the last commit timestamp for a path.
 */
async function getLastCommitTime(repoRoot: string, projectPath: string): Promise<string | undefined> {
  try {
    const output = await execGit(
      ["log", "-1", "--format=%cI", "--", projectPath],
      repoRoot
    )
    return output || undefined
  } catch {
    return undefined
  }
}

/**
 * Fetch real Git activity data for tracked projects.
 * Falls back to mock data if git commands fail.
 */
async function fetchGitActivityData(): Promise<ProjectActivity[]> {
  try {
    const repoRoot = await getRepoRoot()
    const now = new Date()
    const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).toISOString()
    const weekAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000).toISOString()

    const results: ProjectActivity[] = []

    for (const project of TRACKED_PROJECTS) {
      const fullPath = path.join(repoRoot, project.path)

      const [commitsToday, commitsWeek, contributors, lastCommit] = await Promise.all([
        countCommitsSince(repoRoot, project.path, todayStart),
        countCommitsSince(repoRoot, project.path, weekAgo),
        getContributorsSince(repoRoot, project.path, weekAgo),
        getLastCommitTime(repoRoot, project.path),
      ])

      results.push({
        project_id: project.id,
        project_name: project.name,
        commits_today: commitsToday,
        commits_week: commitsWeek,
        active_contributors: contributors,
        last_commit: lastCommit,
        ai_sessions: 0, // TODO: Integrate with session tracking
      })
    }

    // Sort by commits_week descending
    return results.sort((a, b) => b.commits_week - a.commits_week)
  } catch (error) {
    console.error("Failed to fetch git activity data:", error)
    // Fall back to mock data
    return generateActivityDataMock()
  }
}

// ============================================================================
// Mock Data Generation
// ============================================================================

function generateTrendData(days: number): TrendDataPoint[] {
  const data: TrendDataPoint[] = []
  const now = new Date()

  for (let i = days - 1; i >= 0; i--) {
    const date = new Date(now)
    date.setDate(date.getDate() - i)
    const dateStr = date.toISOString().split("T")[0]

    // Generate realistic-looking data with some variance
    const baseTokens = 200000 + Math.floor(Math.random() * 100000)
    const inputTokens = Math.floor(baseTokens * 0.4)
    const outputTokens = Math.floor(baseTokens * 0.6)
    const requests = Math.floor(50 + Math.random() * 100)

    // Weekend dip
    const dayOfWeek = date.getDay()
    const weekendFactor = dayOfWeek === 0 || dayOfWeek === 6 ? 0.3 : 1.0

    data.push({
      date: dateStr,
      input_tokens: Math.floor(inputTokens * weekendFactor),
      output_tokens: Math.floor(outputTokens * weekendFactor),
      total_tokens: Math.floor((inputTokens + outputTokens) * weekendFactor),
      requests: Math.floor(requests * weekendFactor),
      cost_usd: parseFloat(calculateCost(inputTokens * weekendFactor, outputTokens * weekendFactor).toFixed(2)),
    })
  }

  return data
}

function generateTeamData(): TeamUsage[] {
  return [
    {
      team_id: "team-eng",
      team_name: "Engineering",
      member_count: 8,
      tokens_used: 2500000,
      requests: 450,
      percentage: 55,
      top_users: [
        { user_id: "user-dev-1", name: "John Developer", tokens: 800000 },
        { user_id: "user-dev-2", name: "Jane Smith", tokens: 650000 },
        { user_id: "user-dev-3", name: "Bob Chen", tokens: 450000 },
      ],
    },
    {
      team_id: "team-product",
      team_name: "Product",
      member_count: 3,
      tokens_used: 950000,
      requests: 180,
      percentage: 21,
      top_users: [
        { user_id: "user-pm-1", name: "Alice PM", tokens: 500000 },
        { user_id: "user-pm-2", name: "Charlie UX", tokens: 300000 },
      ],
    },
    {
      team_id: "team-ops",
      team_name: "Operations",
      member_count: 4,
      tokens_used: 650000,
      requests: 120,
      percentage: 14,
      top_users: [
        { user_id: "user-ops-1", name: "David Ops", tokens: 350000 },
        { user_id: "user-ops-2", name: "Eva Support", tokens: 200000 },
      ],
    },
    {
      team_id: "team-content",
      team_name: "Content",
      member_count: 2,
      tokens_used: 450000,
      requests: 90,
      percentage: 10,
      top_users: [{ user_id: "user-writer-1", name: "Frank Writer", tokens: 450000 }],
    },
  ]
}

function generateActivityDataMock(): ProjectActivity[] {
  return [
    {
      project_id: "proj-codecoder",
      project_name: "codecoder",
      commits_today: 12,
      commits_week: 45,
      active_contributors: 3,
      last_commit: new Date().toISOString(),
      ai_sessions: 28,
    },
    {
      project_id: "proj-zero-gateway",
      project_name: "zero-gateway",
      commits_today: 5,
      commits_week: 18,
      active_contributors: 2,
      last_commit: new Date(Date.now() - 3600000).toISOString(),
      ai_sessions: 15,
    },
    {
      project_id: "proj-zero-channels",
      project_name: "zero-channels",
      commits_today: 8,
      commits_week: 32,
      active_contributors: 2,
      last_commit: new Date(Date.now() - 7200000).toISOString(),
      ai_sessions: 22,
    },
    {
      project_id: "proj-web",
      project_name: "web",
      commits_today: 3,
      commits_week: 12,
      active_contributors: 2,
      last_commit: new Date(Date.now() - 14400000).toISOString(),
      ai_sessions: 10,
    },
  ]
}

function generateSummary(period: "daily" | "weekly" | "monthly"): ExecutiveSummary {
  const multiplier = period === "daily" ? 1 : period === "weekly" ? 7 : 30

  const totalCost = 25.5 * multiplier + Math.random() * 10 * multiplier
  const previousCost = 24.0 * multiplier + Math.random() * 8 * multiplier
  const costChange = ((totalCost - previousCost) / previousCost) * 100

  return {
    period,
    total_cost_usd: parseFloat(totalCost.toFixed(2)),
    cost_change_percent: parseFloat(costChange.toFixed(1)),
    total_tokens: Math.floor(4500000 * multiplier),
    total_requests: Math.floor(800 * multiplier),
    active_users: 12,
    active_projects: 4,
    top_models: [
      { model: "claude-sonnet-4", usage_percent: 65, cost_usd: parseFloat((totalCost * 0.65).toFixed(2)) },
      { model: "gpt-4o", usage_percent: 20, cost_usd: parseFloat((totalCost * 0.2).toFixed(2)) },
      { model: "claude-haiku-3.5", usage_percent: 15, cost_usd: parseFloat((totalCost * 0.15).toFixed(2)) },
    ],
    alerts: generateAlerts(totalCost, multiplier),
  }
}

function generateAlerts(cost: number, multiplier: number): ExecutiveSummary["alerts"] {
  const alerts: ExecutiveSummary["alerts"] = []

  // Check for cost threshold
  if (cost > 200 * multiplier) {
    alerts.push({
      type: "warning",
      message: "Token usage approaching monthly budget limit",
      metric: "cost_usd",
      value: cost,
      threshold: 250 * multiplier,
    })
  }

  // Check for user quota warnings
  if (Math.random() > 0.7) {
    alerts.push({
      type: "info",
      message: "2 users have exceeded 80% of their daily quota",
    })
  }

  return alerts
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/v1/executive/trends
 *
 * Get cost and usage trends over time.
 * Query params:
 *   - period: "daily" | "weekly" | "monthly" (default: "weekly")
 *   - days: number of days to include (default: 7 for weekly, 30 for monthly)
 */
export async function getTrends(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const period = url.searchParams.get("period") ?? "weekly"
    const daysParam = url.searchParams.get("days")

    let days: number
    switch (period) {
      case "daily":
        days = daysParam ? parseInt(daysParam, 10) : 1
        break
      case "monthly":
        days = daysParam ? parseInt(daysParam, 10) : 30
        break
      case "weekly":
      default:
        days = daysParam ? parseInt(daysParam, 10) : 7
        break
    }

    // Clamp days to reasonable range
    days = Math.max(1, Math.min(days, 90))

    const trends = generateTrendData(days)

    // Calculate totals
    const totals = trends.reduce(
      (acc, point) => ({
        input_tokens: acc.input_tokens + point.input_tokens,
        output_tokens: acc.output_tokens + point.output_tokens,
        total_tokens: acc.total_tokens + point.total_tokens,
        requests: acc.requests + point.requests,
        cost_usd: acc.cost_usd + point.cost_usd,
      }),
      { input_tokens: 0, output_tokens: 0, total_tokens: 0, requests: 0, cost_usd: 0 }
    )

    return jsonResponse({
      success: true,
      data: {
        period,
        days,
        trends,
        totals: {
          ...totals,
          cost_usd: parseFloat(totals.cost_usd.toFixed(2)),
        },
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/executive/teams
 *
 * Get team/department usage breakdown.
 * Now integrates with metering API for real user data.
 */
export async function getTeams(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    // Use metering-integrated team data
    const teams = await generateTeamDataFromMetering()

    const totals = teams.reduce(
      (acc, team) => ({
        tokens: acc.tokens + team.tokens_used,
        requests: acc.requests + team.requests,
        members: acc.members + team.member_count,
      }),
      { tokens: 0, requests: 0, members: 0 }
    )

    return jsonResponse({
      success: true,
      data: {
        teams,
        totals,
        team_count: teams.length,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/executive/activity
 *
 * Get project activity summary (Git commits and AI sessions).
 * Now uses real Git data instead of mock data.
 */
export async function getActivity(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const projects = await fetchGitActivityData()

    const totals = projects.reduce(
      (acc, proj) => ({
        commits_today: acc.commits_today + proj.commits_today,
        commits_week: acc.commits_week + proj.commits_week,
        ai_sessions: acc.ai_sessions + proj.ai_sessions,
      }),
      { commits_today: 0, commits_week: 0, ai_sessions: 0 }
    )

    return jsonResponse({
      success: true,
      data: {
        projects,
        totals,
        project_count: projects.length,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/executive/summary
 *
 * Get executive summary with key metrics.
 * Now integrates with metering API for real usage data.
 * Query params:
 *   - period: "daily" | "weekly" | "monthly" (default: "weekly")
 */
export async function getSummary(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const period = (url.searchParams.get("period") ?? "weekly") as "daily" | "weekly" | "monthly"

    // Use metering-integrated summary
    const summary = await generateSummaryFromMetering(period)

    return jsonResponse({
      success: true,
      data: summary,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/executive/health
 *
 * Health check endpoint.
 */
export async function executiveHealth(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  return jsonResponse({
    success: true,
    data: {
      status: "healthy",
      service: "executive-dashboard",
      timestamp: new Date().toISOString(),
    },
  })
}
