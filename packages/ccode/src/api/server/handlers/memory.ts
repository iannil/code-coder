/**
 * Memory API Handlers
 *
 * Handles memory system operations:
 * - Daily notes (flow layer)
 * - Long-term memory (sediment layer)
 * - Memory consolidation
 */

import type { RouteHandler } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import {
  loadDailyNotes,
  appendDailyNote,
  listDailyNoteDates,
  loadLongTermMemory,
  updateCategory,
  mergeToCategory,
  getMemorySections,
  consolidateMemory,
  getConsolidationStats,
  getMemorySummary,
  createEntry,
  parseDate,
} from "@/memory-markdown"

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    return "{}"
  }
  return await new Response(body).text()
}

// ============================================================================
// Daily Notes Handlers (Flow Layer)
// ============================================================================

/**
 * List all daily note dates
 * GET /api/memory/daily
 */
export const listDailyDates: RouteHandler = async () => {
  try {
    const dates = await listDailyNoteDates()
    return jsonResponse({ success: true, data: dates })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get daily notes for a specific date
 * GET /api/memory/daily/:date
 */
export const getDailyNotes: RouteHandler = async (_req, params) => {
  try {
    const dateStr = params.date
    if (!dateStr) {
      return errorResponse("Date required (YYYY-MM-DD format)", 400)
    }

    // Parse date string to Date object
    const date = parseDate(dateStr)
    if (!date) {
      return errorResponse("Invalid date format. Use YYYY-MM-DD", 400)
    }

    const notes = await loadDailyNotes(date, 1)
    return jsonResponse({ success: true, data: notes })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Append a note to today's daily notes
 * POST /api/memory/daily
 */
export const appendDailyNoteHandler: RouteHandler = async (req) => {
  try {
    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.type || !body.content) {
      return errorResponse("Type and content required", 400)
    }

    const entry = createEntry(body.type, body.content, body.metadata)
    await appendDailyNote(entry)

    return jsonResponse({ success: true, data: entry }, 201)
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// Long-Term Memory Handlers (Sediment Layer)
// ============================================================================

/**
 * Get long-term memory content
 * GET /api/memory/long-term
 */
export const getLongTermMemory: RouteHandler = async () => {
  try {
    const content = await loadLongTermMemory()
    return jsonResponse({ success: true, data: { content } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Get memory sections
 * GET /api/memory/sections
 */
export const getMemorySectionsHandler: RouteHandler = async () => {
  try {
    const sections = await getMemorySections()
    return jsonResponse({ success: true, data: sections })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Update a specific category in long-term memory
 * PUT /api/memory/category/:category
 */
export const updateCategoryHandler: RouteHandler = async (req, params) => {
  try {
    const category = params.category
    if (!category) {
      return errorResponse("Category required", 400)
    }

    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.content) {
      return errorResponse("Content required", 400)
    }

    await updateCategory(category as any, body.content)
    return jsonResponse({ success: true })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Merge content into a category
 * POST /api/memory/category/:category/merge
 */
export const mergeToCategoryHandler: RouteHandler = async (req, params) => {
  try {
    const category = params.category
    if (!category) {
      return errorResponse("Category required", 400)
    }

    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    if (!body.content) {
      return errorResponse("Content required", 400)
    }

    await mergeToCategory(category as any, body.content)
    return jsonResponse({ success: true })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// Consolidation Handlers
// ============================================================================

/**
 * Get consolidation statistics
 * GET /api/memory/consolidation/stats
 */
export const getConsolidationStatsHandler: RouteHandler = async () => {
  try {
    const stats = await getConsolidationStats()
    return jsonResponse({ success: true, data: stats })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Trigger memory consolidation
 * POST /api/memory/consolidation
 */
export const triggerConsolidation: RouteHandler = async (req) => {
  try {
    const bodyText = await readRequestBody(req.body)
    const body = JSON.parse(bodyText)

    const result = await consolidateMemory({
      days: body.days,
      preserveOriginal: body.preserveOriginal,
      minImportance: body.minImportance,
    })

    return jsonResponse({ success: true, data: result })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// Summary Handler
// ============================================================================

/**
 * Get memory summary
 * GET /api/memory/summary
 */
export const getMemorySummaryHandler: RouteHandler = async () => {
  try {
    const summary = await getMemorySummary()
    return jsonResponse({ success: true, data: summary })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
