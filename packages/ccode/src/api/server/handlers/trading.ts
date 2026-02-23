/**
 * Trading Review API Handler
 *
 * Provides trading diary and review endpoints for the @trader agent integration.
 * Supports recording trades, journaling, generating reviews, and P&L statistics.
 *
 * POST /api/v1/trading/trades - Record a new trade
 * GET  /api/v1/trading/trades - List trades with date range
 * PUT  /api/v1/trading/trades/:id - Update/close a trade
 * GET  /api/v1/trading/trades/:id - Get trade details
 *
 * POST /api/v1/trading/journal - Save journal entry
 * GET  /api/v1/trading/journal/:date - Get journal for date
 * GET  /api/v1/trading/journal - List recent journal entries
 *
 * POST /api/v1/trading/review - Generate review for period
 * GET  /api/v1/trading/reviews - List recent reviews
 *
 * GET  /api/v1/trading/stats - Get P&L statistics
 * GET  /api/v1/trading/summary - Get daily summary
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"

// ============================================================================
// Types
// ============================================================================

export type TradeDirection = "long" | "short"
export type TradeOutcome = "win" | "loss" | "break_even" | "open"
export type AssetClass = "stock" | "futures" | "forex" | "crypto" | "options" | "bond" | "commodity" | "other"
export type ReviewPeriod = "daily" | "weekly" | "monthly" | "quarterly" | "yearly"

export interface TradeEntry {
  id: string
  symbol: string
  asset_class: AssetClass
  direction: TradeDirection
  entry_price: number
  exit_price?: number
  quantity: number
  entry_time: string
  exit_time?: string
  stop_loss?: number
  take_profit?: number
  outcome: TradeOutcome
  pnl?: number
  strategy?: string
  entry_reason: string
  exit_reason?: string
  tags: string[]
  attachments: string[]
  notes: string
}

export interface JournalEntry {
  date: string
  summary: string
  market_context?: string
  emotional_state?: string
  lessons: string[]
  next_day_goals: string[]
  mistakes: string[]
  wins: string[]
  score?: number
  created_at: string
  updated_at: string
}

export interface ReviewStats {
  total_trades: number
  winning_trades: number
  losing_trades: number
  win_rate: number
  total_pnl: number
  avg_win: number
  avg_loss: number
  risk_reward_ratio: number
  largest_win: number
  largest_loss: number
  avg_holding_hours: number
  most_traded_symbol?: string
  best_strategy?: string
}

export interface TradingReview {
  id: string
  period: ReviewPeriod
  start_date: string
  end_date: string
  stats: ReviewStats
  analysis: string
  patterns: string[]
  improvements: string[]
  goals: string[]
  created_at: string
}

export interface DailySummary {
  date: string
  trades_count: number
  win_rate: number
  total_pnl: number
  open_positions: number
  journal_exists: boolean
}

// ============================================================================
// In-Memory Store (Production would use SQLite/Postgres)
// ============================================================================

const trades: Map<string, TradeEntry> = new Map()
const journals: Map<string, JournalEntry> = new Map()
const reviews: TradingReview[] = []

// ============================================================================
// Helper Functions
// ============================================================================

function generateId(): string {
  return `trade_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`
}

function calculatePnl(trade: TradeEntry): number | undefined {
  if (trade.exit_price === undefined) return undefined

  const multiplier = trade.direction === "long" ? 1 : -1
  return (trade.exit_price - trade.entry_price) * trade.quantity * multiplier
}

function determineOutcome(pnl: number | undefined): TradeOutcome {
  if (pnl === undefined) return "open"
  if (pnl > 0) return "win"
  if (pnl < 0) return "loss"
  return "break_even"
}

function calculateStats(tradeList: TradeEntry[]): ReviewStats {
  const closed = tradeList.filter((t) => t.outcome !== "open")
  if (closed.length === 0) {
    return {
      total_trades: 0,
      winning_trades: 0,
      losing_trades: 0,
      win_rate: 0,
      total_pnl: 0,
      avg_win: 0,
      avg_loss: 0,
      risk_reward_ratio: 0,
      largest_win: 0,
      largest_loss: 0,
      avg_holding_hours: 0,
    }
  }

  const wins = closed.filter((t) => t.outcome === "win")
  const losses = closed.filter((t) => t.outcome === "loss")

  const totalPnl = closed.reduce((sum, t) => sum + (t.pnl ?? 0), 0)
  const avgWin = wins.length > 0 ? wins.reduce((sum, t) => sum + (t.pnl ?? 0), 0) / wins.length : 0
  const avgLoss = losses.length > 0 ? Math.abs(losses.reduce((sum, t) => sum + (t.pnl ?? 0), 0)) / losses.length : 0

  const largestWin = Math.max(0, ...closed.map((t) => t.pnl ?? 0))
  const largestLoss = Math.min(0, ...closed.map((t) => t.pnl ?? 0))

  // Calculate holding hours
  const holdingHours = closed
    .filter((t) => t.exit_time)
    .map((t) => {
      const entry = new Date(t.entry_time).getTime()
      const exit = new Date(t.exit_time!).getTime()
      return (exit - entry) / (1000 * 60 * 60)
    })

  const avgHolding = holdingHours.length > 0 ? holdingHours.reduce((a, b) => a + b, 0) / holdingHours.length : 0

  // Find most traded symbol
  const symbolCounts = closed.reduce(
    (acc, t) => {
      acc[t.symbol] = (acc[t.symbol] ?? 0) + 1
      return acc
    },
    {} as Record<string, number>,
  )
  const mostTraded = Object.entries(symbolCounts).sort((a, b) => b[1] - a[1])[0]

  // Find best strategy
  const strategyPnl = closed.reduce(
    (acc, t) => {
      if (t.strategy) {
        acc[t.strategy] = (acc[t.strategy] ?? 0) + (t.pnl ?? 0)
      }
      return acc
    },
    {} as Record<string, number>,
  )
  const bestStrategy = Object.entries(strategyPnl)
    .filter(([, pnl]) => pnl > 0)
    .sort((a, b) => b[1] - a[1])[0]

  return {
    total_trades: closed.length,
    winning_trades: wins.length,
    losing_trades: losses.length,
    win_rate: (wins.length / closed.length) * 100,
    total_pnl: totalPnl,
    avg_win: avgWin,
    avg_loss: avgLoss,
    risk_reward_ratio: avgLoss > 0 ? avgWin / avgLoss : 0,
    largest_win: largestWin,
    largest_loss: largestLoss,
    avg_holding_hours: avgHolding,
    most_traded_symbol: mostTraded?.[0],
    best_strategy: bestStrategy?.[0],
  }
}

function filterTradesByDateRange(startDate?: string, endDate?: string): TradeEntry[] {
  const all = Array.from(trades.values())

  if (!startDate && !endDate) return all

  const start = startDate ? new Date(startDate) : new Date(0)
  const end = endDate ? new Date(endDate) : new Date()

  return all.filter((t) => {
    const entryDate = new Date(t.entry_time)
    return entryDate >= start && entryDate <= end
  })
}

// ============================================================================
// Trade Handlers
// ============================================================================

export async function handleCreateTrade(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = (await req.json()) as Partial<TradeEntry>

    if (!body.symbol || !body.direction || !body.entry_price || !body.quantity || !body.entry_reason) {
      return errorResponse("Missing required fields: symbol, direction, entry_price, quantity, entry_reason", 400)
    }

    const trade: TradeEntry = {
      id: generateId(),
      symbol: body.symbol,
      asset_class: body.asset_class ?? "stock",
      direction: body.direction,
      entry_price: body.entry_price,
      exit_price: body.exit_price,
      quantity: body.quantity,
      entry_time: body.entry_time ?? new Date().toISOString(),
      exit_time: body.exit_time,
      stop_loss: body.stop_loss,
      take_profit: body.take_profit,
      outcome: "open",
      pnl: undefined,
      strategy: body.strategy,
      entry_reason: body.entry_reason,
      exit_reason: body.exit_reason,
      tags: body.tags ?? [],
      attachments: body.attachments ?? [],
      notes: body.notes ?? "",
    }

    // Calculate P&L if exit price provided
    if (trade.exit_price !== undefined) {
      trade.pnl = calculatePnl(trade)
      trade.outcome = determineOutcome(trade.pnl)
    }

    trades.set(trade.id, trade)

    return jsonResponse({ success: true, trade })
  } catch (error) {
    return errorResponse(`Failed to create trade: ${error}`, 500)
  }
}

export async function handleListTrades(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const startDate = url.searchParams.get("start_date") ?? undefined
    const endDate = url.searchParams.get("end_date") ?? undefined
    const limit = parseInt(url.searchParams.get("limit") ?? "100", 10)
    const offset = parseInt(url.searchParams.get("offset") ?? "0", 10)

    const filtered = filterTradesByDateRange(startDate, endDate)
    const sorted = filtered.sort((a, b) => new Date(b.entry_time).getTime() - new Date(a.entry_time).getTime())
    const paginated = sorted.slice(offset, offset + limit)

    return jsonResponse({
      success: true,
      trades: paginated,
      total: filtered.length,
      limit,
      offset,
    })
  } catch (error) {
    return errorResponse(`Failed to list trades: ${error}`, 500)
  }
}

export async function handleGetTrade(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const id = params.id
  if (!id) return errorResponse("Trade ID required", 400)

  const trade = trades.get(id)
  if (!trade) return errorResponse("Trade not found", 404)

  return jsonResponse({ success: true, trade })
}

export async function handleUpdateTrade(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const id = params.id
    if (!id) return errorResponse("Trade ID required", 400)

    const trade = trades.get(id)
    if (!trade) return errorResponse("Trade not found", 404)

    const body = (await req.json()) as Partial<TradeEntry>

    // Update allowed fields
    const updated: TradeEntry = {
      ...trade,
      exit_price: body.exit_price ?? trade.exit_price,
      exit_time: body.exit_time ?? body.exit_price ? new Date().toISOString() : trade.exit_time,
      exit_reason: body.exit_reason ?? trade.exit_reason,
      stop_loss: body.stop_loss ?? trade.stop_loss,
      take_profit: body.take_profit ?? trade.take_profit,
      strategy: body.strategy ?? trade.strategy,
      tags: body.tags ?? trade.tags,
      notes: body.notes ?? trade.notes,
    }

    // Recalculate P&L and outcome
    if (updated.exit_price !== undefined) {
      updated.pnl = calculatePnl(updated)
      updated.outcome = determineOutcome(updated.pnl)
    }

    trades.set(id, updated)

    return jsonResponse({ success: true, trade: updated })
  } catch (error) {
    return errorResponse(`Failed to update trade: ${error}`, 500)
  }
}

// ============================================================================
// Journal Handlers
// ============================================================================

export async function handleSaveJournal(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = (await req.json()) as Partial<JournalEntry>

    if (!body.date || !body.summary) {
      return errorResponse("Missing required fields: date, summary", 400)
    }

    const now = new Date().toISOString()
    const existing = journals.get(body.date)

    const entry: JournalEntry = {
      date: body.date,
      summary: body.summary,
      market_context: body.market_context,
      emotional_state: body.emotional_state,
      lessons: body.lessons ?? [],
      next_day_goals: body.next_day_goals ?? [],
      mistakes: body.mistakes ?? [],
      wins: body.wins ?? [],
      score: body.score,
      created_at: existing?.created_at ?? now,
      updated_at: now,
    }

    journals.set(body.date, entry)

    return jsonResponse({ success: true, journal: entry })
  } catch (error) {
    return errorResponse(`Failed to save journal: ${error}`, 500)
  }
}

export async function handleGetJournal(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const date = params.date
  if (!date) return errorResponse("Date required", 400)

  const entry = journals.get(date)
  if (!entry) return errorResponse("Journal entry not found", 404)

  return jsonResponse({ success: true, journal: entry })
}

export async function handleListJournals(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const limit = parseInt(url.searchParams.get("limit") ?? "30", 10)

    const all = Array.from(journals.values())
    const sorted = all.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())
    const limited = sorted.slice(0, limit)

    return jsonResponse({
      success: true,
      journals: limited,
      total: all.length,
    })
  } catch (error) {
    return errorResponse(`Failed to list journals: ${error}`, 500)
  }
}

// ============================================================================
// Review Handlers
// ============================================================================

export async function handleGenerateReview(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = (await req.json()) as { period: ReviewPeriod }
    const period = body.period ?? "daily"

    const today = new Date()
    let startDate: Date
    let endDate = today

    switch (period) {
      case "daily":
        startDate = new Date(today.toDateString())
        break
      case "weekly": {
        const dayOfWeek = today.getDay()
        startDate = new Date(today)
        startDate.setDate(today.getDate() - dayOfWeek)
        break
      }
      case "monthly":
        startDate = new Date(today.getFullYear(), today.getMonth(), 1)
        break
      case "quarterly": {
        const quarterStart = Math.floor(today.getMonth() / 3) * 3
        startDate = new Date(today.getFullYear(), quarterStart, 1)
        break
      }
      case "yearly":
        startDate = new Date(today.getFullYear(), 0, 1)
        break
      default:
        startDate = new Date(today.toDateString())
    }

    const tradeList = filterTradesByDateRange(startDate.toISOString(), endDate.toISOString())
    const stats = calculateStats(tradeList)

    // Generate basic analysis (in production, call @trader agent)
    const analysis =
      stats.total_trades > 0
        ? `本期共交易 ${stats.total_trades} 次，胜率 ${stats.win_rate.toFixed(1)}%，总盈亏 ${stats.total_pnl.toFixed(2)}。${
            stats.win_rate >= 50 ? "表现良好" : "需要改进交易策略"
          }。`
        : "本期无交易记录。"

    const review: TradingReview = {
      id: `review_${Date.now()}`,
      period,
      start_date: startDate.toISOString().split("T")[0],
      end_date: endDate.toISOString().split("T")[0],
      stats,
      analysis,
      patterns: stats.best_strategy ? [`${stats.best_strategy} 策略表现最佳`] : [],
      improvements: stats.win_rate < 50 ? ["提高入场时机把握", "严格执行止损"] : [],
      goals: ["保持交易纪律", "记录每笔交易"],
      created_at: new Date().toISOString(),
    }

    reviews.push(review)

    return jsonResponse({ success: true, review })
  } catch (error) {
    return errorResponse(`Failed to generate review: ${error}`, 500)
  }
}

export async function handleListReviews(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const limit = parseInt(url.searchParams.get("limit") ?? "10", 10)

    const sorted = [...reviews].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
    const limited = sorted.slice(0, limit)

    return jsonResponse({
      success: true,
      reviews: limited,
      total: reviews.length,
    })
  } catch (error) {
    return errorResponse(`Failed to list reviews: ${error}`, 500)
  }
}

// ============================================================================
// Statistics Handlers
// ============================================================================

export async function handleGetStats(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const startDate = url.searchParams.get("start_date") ?? undefined
    const endDate = url.searchParams.get("end_date") ?? undefined

    const tradeList = filterTradesByDateRange(startDate, endDate)
    const stats = calculateStats(tradeList)

    return jsonResponse({ success: true, stats })
  } catch (error) {
    return errorResponse(`Failed to get stats: ${error}`, 500)
  }
}

export async function handleGetSummary(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const today = new Date().toISOString().split("T")[0]
    const todayStart = new Date(today)
    const todayEnd = new Date(today)
    todayEnd.setHours(23, 59, 59, 999)

    const todayTrades = filterTradesByDateRange(todayStart.toISOString(), todayEnd.toISOString())
    const stats = calculateStats(todayTrades)
    const allTrades = Array.from(trades.values())
    const openPositions = allTrades.filter((t) => t.outcome === "open").length

    const summary: DailySummary = {
      date: today,
      trades_count: stats.total_trades,
      win_rate: stats.win_rate,
      total_pnl: stats.total_pnl,
      open_positions: openPositions,
      journal_exists: journals.has(today),
    }

    return jsonResponse({ success: true, summary })
  } catch (error) {
    return errorResponse(`Failed to get summary: ${error}`, 500)
  }
}

// ============================================================================
// Route Registration Helper
// ============================================================================

export const tradingRoutes = {
  "POST /api/v1/trading/trades": handleCreateTrade,
  "GET /api/v1/trading/trades": handleListTrades,
  "GET /api/v1/trading/trades/:id": handleGetTrade,
  "PUT /api/v1/trading/trades/:id": handleUpdateTrade,
  "POST /api/v1/trading/journal": handleSaveJournal,
  "GET /api/v1/trading/journal/:date": handleGetJournal,
  "GET /api/v1/trading/journal": handleListJournals,
  "POST /api/v1/trading/review": handleGenerateReview,
  "GET /api/v1/trading/reviews": handleListReviews,
  "GET /api/v1/trading/stats": handleGetStats,
  "GET /api/v1/trading/summary": handleGetSummary,
}
