/**
 * Global Context Hub Handler
 *
 * Provides cross-user and cross-department knowledge sharing capabilities.
 * Implements the "全局上下文枢纽" for enterprise AI collaboration.
 *
 * Knowledge Categories:
 * - PRD: Product requirement documents
 * - meeting_notes: Meeting summaries and decisions
 * - lessons_learned: Post-mortems and experience sharing
 * - risk_log: Risk assessments and mitigations
 * - architecture: System design documents
 * - runbook: Operational procedures
 *
 * Visibility Levels:
 * - private: Only visible to the creator
 * - department: Visible to department members
 * - global: Visible to all authenticated users
 *
 * Part of Phase 4: Global Context Hub
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { Global } from "@/global"
import path from "path"
import fs from "fs/promises"

// ============================================================================
// Types
// ============================================================================

/** Knowledge category for organization */
export type KnowledgeHubCategory =
  | "prd"
  | "meeting_notes"
  | "lessons_learned"
  | "risk_log"
  | "architecture"
  | "runbook"
  | "faq"
  | "onboarding"
  | "decision"
  | "custom"

/** Visibility level for access control */
export type KnowledgeVisibility = "private" | "department" | "global"

/** Knowledge entry in the hub */
export interface HubKnowledgeEntry {
  id: string
  title: string
  content: string
  summary?: string
  category: KnowledgeHubCategory
  visibility: KnowledgeVisibility
  tags: string[]
  department_id?: string
  created_by: string
  created_at: string
  updated_at: string
  view_count: number
  helpful_count: number
  related_entries?: string[]
  metadata?: Record<string, unknown>
}

/** Search request */
export interface HubSearchRequest {
  query: string
  categories?: KnowledgeHubCategory[]
  visibility?: KnowledgeVisibility
  department_id?: string
  tags?: string[]
  limit?: number
}

/** Search result */
export interface HubSearchResult {
  entry: HubKnowledgeEntry
  relevance_score: number
  matched_tags: string[]
  snippet: string
}

/** Hub statistics */
export interface HubStats {
  total_entries: number
  by_category: Record<KnowledgeHubCategory, number>
  by_visibility: Record<KnowledgeVisibility, number>
  top_tags: Array<{ tag: string; count: number }>
  recent_entries: HubKnowledgeEntry[]
  most_viewed: HubKnowledgeEntry[]
  most_helpful: HubKnowledgeEntry[]
}

// ============================================================================
// Storage
// ============================================================================

const HUB_DIR = path.join(Global.Path.data, "context-hub")
const ENTRIES_FILE = path.join(HUB_DIR, "entries.json")
const INDEX_FILE = path.join(HUB_DIR, "index.json")

interface HubIndex {
  by_tag: Record<string, string[]>
  by_category: Record<string, string[]>
  by_department: Record<string, string[]>
}

async function ensureHubDir(): Promise<void> {
  await fs.mkdir(HUB_DIR, { recursive: true })
}

async function loadEntries(): Promise<HubKnowledgeEntry[]> {
  await ensureHubDir()

  try {
    const content = await fs.readFile(ENTRIES_FILE, "utf-8")
    return JSON.parse(content)
  } catch {
    return []
  }
}

async function saveEntries(entries: HubKnowledgeEntry[]): Promise<void> {
  await ensureHubDir()
  await fs.writeFile(ENTRIES_FILE, JSON.stringify(entries, null, 2))

  // Rebuild index
  await rebuildIndex(entries)
}

async function rebuildIndex(entries: HubKnowledgeEntry[]): Promise<void> {
  const index: HubIndex = {
    by_tag: {},
    by_category: {},
    by_department: {},
  }

  for (const entry of entries) {
    // Index by tags
    for (const tag of entry.tags) {
      index.by_tag[tag] ??= []
      index.by_tag[tag].push(entry.id)
    }

    // Index by category
    index.by_category[entry.category] ??= []
    index.by_category[entry.category].push(entry.id)

    // Index by department
    if (entry.department_id) {
      index.by_department[entry.department_id] ??= []
      index.by_department[entry.department_id].push(entry.id)
    }
  }

  await fs.writeFile(INDEX_FILE, JSON.stringify(index, null, 2))
}

async function loadIndex(): Promise<HubIndex> {
  await ensureHubDir()

  try {
    const content = await fs.readFile(INDEX_FILE, "utf-8")
    return JSON.parse(content)
  } catch {
    return { by_tag: {}, by_category: {}, by_department: {} }
  }
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

function extractSnippet(content: string, query: string, maxLength = 200): string {
  const queryWords = query.toLowerCase().split(/\s+/)
  const contentLower = content.toLowerCase()

  // Find first occurrence of any query word
  let bestIndex = 0
  for (const word of queryWords) {
    const idx = contentLower.indexOf(word)
    if (idx !== -1 && (bestIndex === 0 || idx < bestIndex)) {
      bestIndex = idx
    }
  }

  // Extract snippet around the match
  const start = Math.max(0, bestIndex - 50)
  const end = Math.min(content.length, start + maxLength)
  let snippet = content.slice(start, end)

  if (start > 0) snippet = "..." + snippet
  if (end < content.length) snippet = snippet + "..."

  return snippet
}

function calculateRelevance(entry: HubKnowledgeEntry, query: string, queryTags: string[]): number {
  let score = 0
  const queryLower = query.toLowerCase()
  const queryWords = queryLower.split(/\s+/).filter((w) => w.length > 2)

  // Title match (highest weight)
  const titleLower = entry.title.toLowerCase()
  for (const word of queryWords) {
    if (titleLower.includes(word)) score += 0.3
  }
  if (titleLower.includes(queryLower)) score += 0.5

  // Content match
  const contentLower = entry.content.toLowerCase()
  for (const word of queryWords) {
    if (contentLower.includes(word)) score += 0.1
  }

  // Tag match
  const entryTags = new Set(entry.tags.map((t) => t.toLowerCase()))
  for (const tag of queryTags) {
    if (entryTags.has(tag.toLowerCase())) score += 0.2
  }

  // Popularity boost
  score += Math.min(entry.view_count / 1000, 0.1)
  score += Math.min(entry.helpful_count / 100, 0.1)

  return Math.min(score, 1.0)
}

function extractTags(content: string): string[] {
  // Extract hashtags
  const hashtagMatch = content.match(/#[\w\u4e00-\u9fa5]+/g) ?? []
  const tags = hashtagMatch.map((t) => t.slice(1).toLowerCase())

  // Extract common technical terms
  const techTerms = [
    "api",
    "database",
    "frontend",
    "backend",
    "security",
    "performance",
    "testing",
    "deployment",
    "architecture",
    "design",
    "bug",
    "feature",
    "refactor",
  ]

  const contentLower = content.toLowerCase()
  for (const term of techTerms) {
    if (contentLower.includes(term) && !tags.includes(term)) {
      tags.push(term)
    }
  }

  return [...new Set(tags)].slice(0, 10)
}

function checkAccess(
  entry: HubKnowledgeEntry,
  userId: string,
  userDepartment?: string,
): boolean {
  if (entry.visibility === "global") return true
  if (entry.visibility === "private") return entry.created_by === userId
  if (entry.visibility === "department") {
    return entry.department_id === userDepartment || entry.created_by === userId
  }
  return false
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/v1/hub/stats
 * Get hub statistics
 */
export async function getHubStats(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const entries = await loadEntries()

    const byCategory: Record<string, number> = {}
    const byVisibility: Record<string, number> = {}
    const tagCounts: Record<string, number> = {}

    for (const entry of entries) {
      byCategory[entry.category] = (byCategory[entry.category] ?? 0) + 1
      byVisibility[entry.visibility] = (byVisibility[entry.visibility] ?? 0) + 1

      for (const tag of entry.tags) {
        tagCounts[tag] = (tagCounts[tag] ?? 0) + 1
      }
    }

    const topTags = Object.entries(tagCounts)
      .map(([tag, count]) => ({ tag, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 10)

    const recentEntries = [...entries]
      .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
      .slice(0, 5)

    const mostViewed = [...entries].sort((a, b) => b.view_count - a.view_count).slice(0, 5)

    const mostHelpful = [...entries].sort((a, b) => b.helpful_count - a.helpful_count).slice(0, 5)

    const stats: HubStats = {
      total_entries: entries.length,
      by_category: byCategory as Record<KnowledgeHubCategory, number>,
      by_visibility: byVisibility as Record<KnowledgeVisibility, number>,
      top_tags: topTags,
      recent_entries: recentEntries,
      most_viewed: mostViewed,
      most_helpful: mostHelpful,
    }

    return jsonResponse({ success: true, data: stats })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/hub/entries
 * List knowledge entries with filtering
 */
export async function listHubEntries(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = req.url
    const category = url.searchParams.get("category") as KnowledgeHubCategory | null
    const visibility = url.searchParams.get("visibility") as KnowledgeVisibility | null
    const departmentId = url.searchParams.get("department_id")
    const userId = url.searchParams.get("user_id") ?? "anonymous"
    const userDepartment = url.searchParams.get("user_department") ?? undefined
    const limitParam = url.searchParams.get("limit")
    const limit = limitParam ? parseInt(limitParam, 10) : 50

    let entries = await loadEntries()

    // Filter by access
    entries = entries.filter((e) => checkAccess(e, userId, userDepartment))

    // Filter by category
    if (category) {
      entries = entries.filter((e) => e.category === category)
    }

    // Filter by visibility
    if (visibility) {
      entries = entries.filter((e) => e.visibility === visibility)
    }

    // Filter by department
    if (departmentId) {
      entries = entries.filter((e) => e.department_id === departmentId)
    }

    // Sort by updated_at descending
    entries.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())

    // Apply limit
    entries = entries.slice(0, limit)

    return jsonResponse({ success: true, data: entries })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/hub/entries/:id
 * Get a specific entry
 */
export async function getHubEntry(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params
    const url = req.url
    const userId = url.searchParams.get("user_id") ?? "anonymous"
    const userDepartment = url.searchParams.get("user_department") ?? undefined

    if (!id) {
      return errorResponse("Entry ID is required", 400)
    }

    const entries = await loadEntries()
    const entry = entries.find((e) => e.id === id)

    if (!entry) {
      return errorResponse(`Entry "${id}" not found`, 404)
    }

    // Check access
    if (!checkAccess(entry, userId, userDepartment)) {
      return errorResponse("Access denied", 403)
    }

    // Increment view count
    entry.view_count++
    await saveEntries(entries)

    return jsonResponse({ success: true, data: entry })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/hub/entries
 * Create a new knowledge entry
 */
export async function createHubEntry(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<HubKnowledgeEntry> & { user_id?: string }

    if (!input.title) {
      return errorResponse("Title is required", 400)
    }
    if (!input.content) {
      return errorResponse("Content is required", 400)
    }

    const entries = await loadEntries()
    const now = new Date().toISOString()

    // Auto-extract tags if not provided
    const tags = input.tags ?? extractTags(`${input.title} ${input.content}`)

    // Generate summary if not provided
    const summary = input.summary ?? input.content.slice(0, 200) + (input.content.length > 200 ? "..." : "")

    const entry: HubKnowledgeEntry = {
      id: `hub-${Date.now()}`,
      title: input.title,
      content: input.content,
      summary,
      category: input.category ?? "custom",
      visibility: input.visibility ?? "department",
      tags,
      department_id: input.department_id,
      created_by: input.user_id ?? "anonymous",
      created_at: now,
      updated_at: now,
      view_count: 0,
      helpful_count: 0,
      related_entries: input.related_entries,
      metadata: input.metadata,
    }

    entries.push(entry)
    await saveEntries(entries)

    return jsonResponse({ success: true, data: entry }, 201)
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/v1/hub/entries/:id
 * Update a knowledge entry
 */
export async function updateHubEntry(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params
    const url = req.url
    const userId = url.searchParams.get("user_id") ?? "anonymous"

    if (!id) {
      return errorResponse("Entry ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<HubKnowledgeEntry>

    const entries = await loadEntries()
    const index = entries.findIndex((e) => e.id === id)

    if (index === -1) {
      return errorResponse(`Entry "${id}" not found`, 404)
    }

    // Check ownership
    if (entries[index].created_by !== userId) {
      return errorResponse("Only the creator can update this entry", 403)
    }

    const updated: HubKnowledgeEntry = {
      ...entries[index],
      ...input,
      id, // Preserve ID
      created_by: entries[index].created_by, // Preserve creator
      created_at: entries[index].created_at, // Preserve created_at
      updated_at: new Date().toISOString(),
    }

    entries[index] = updated
    await saveEntries(entries)

    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/v1/hub/entries/:id
 * Delete a knowledge entry
 */
export async function deleteHubEntry(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params
    const url = req.url
    const userId = url.searchParams.get("user_id") ?? "anonymous"

    if (!id) {
      return errorResponse("Entry ID is required", 400)
    }

    const entries = await loadEntries()
    const entry = entries.find((e) => e.id === id)

    if (!entry) {
      return errorResponse(`Entry "${id}" not found`, 404)
    }

    // Check ownership
    if (entry.created_by !== userId) {
      return errorResponse("Only the creator can delete this entry", 403)
    }

    const filtered = entries.filter((e) => e.id !== id)
    await saveEntries(filtered)

    return jsonResponse({ success: true, data: { deleted: id } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/hub/search
 * Search knowledge entries
 */
export async function searchHub(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as HubSearchRequest & { user_id?: string; user_department?: string }

    if (!input.query) {
      return errorResponse("Query is required", 400)
    }

    const userId = input.user_id ?? "anonymous"
    const userDepartment = input.user_department
    const limit = input.limit ?? 20

    let entries = await loadEntries()

    // Filter by access
    entries = entries.filter((e) => checkAccess(e, userId, userDepartment))

    // Filter by categories
    if (input.categories && input.categories.length > 0) {
      entries = entries.filter((e) => input.categories!.includes(e.category))
    }

    // Filter by visibility
    if (input.visibility) {
      entries = entries.filter((e) => e.visibility === input.visibility)
    }

    // Filter by department
    if (input.department_id) {
      entries = entries.filter((e) => e.department_id === input.department_id)
    }

    // Filter by tags
    if (input.tags && input.tags.length > 0) {
      const searchTags = new Set(input.tags.map((t) => t.toLowerCase()))
      entries = entries.filter((e) => e.tags.some((t) => searchTags.has(t.toLowerCase())))
    }

    // Calculate relevance and rank
    const queryTags = input.tags ?? []
    const results: HubSearchResult[] = entries
      .map((entry) => {
        const relevance = calculateRelevance(entry, input.query, queryTags)
        const matchedTags = entry.tags.filter((t) =>
          queryTags.some((qt) => qt.toLowerCase() === t.toLowerCase())
        )
        const snippet = extractSnippet(entry.content, input.query)

        return {
          entry,
          relevance_score: relevance,
          matched_tags: matchedTags,
          snippet,
        }
      })
      .filter((r) => r.relevance_score > 0.1)
      .sort((a, b) => b.relevance_score - a.relevance_score)
      .slice(0, limit)

    return jsonResponse({ success: true, data: results })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/hub/entries/:id/helpful
 * Mark an entry as helpful
 */
export async function markHelpful(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Entry ID is required", 400)
    }

    const entries = await loadEntries()
    const entry = entries.find((e) => e.id === id)

    if (!entry) {
      return errorResponse(`Entry "${id}" not found`, 404)
    }

    entry.helpful_count++
    await saveEntries(entries)

    return jsonResponse({ success: true, data: { helpful_count: entry.helpful_count } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/hub/tags
 * List all tags with counts
 */
export async function listHubTags(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const entries = await loadEntries()
    const tagCounts: Record<string, number> = {}

    for (const entry of entries) {
      for (const tag of entry.tags) {
        tagCounts[tag] = (tagCounts[tag] ?? 0) + 1
      }
    }

    const tags = Object.entries(tagCounts)
      .map(([tag, count]) => ({ tag, count }))
      .sort((a, b) => b.count - a.count)

    return jsonResponse({ success: true, data: tags })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/hub/categories
 * List available categories with descriptions
 */
export async function listHubCategories(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const categories = [
    { id: "prd", name: "PRD", description: "Product requirement documents" },
    { id: "meeting_notes", name: "Meeting Notes", description: "Meeting summaries and decisions" },
    { id: "lessons_learned", name: "Lessons Learned", description: "Post-mortems and experience sharing" },
    { id: "risk_log", name: "Risk Log", description: "Risk assessments and mitigations" },
    { id: "architecture", name: "Architecture", description: "System design documents" },
    { id: "runbook", name: "Runbook", description: "Operational procedures" },
    { id: "faq", name: "FAQ", description: "Frequently asked questions" },
    { id: "onboarding", name: "Onboarding", description: "New employee guides" },
    { id: "decision", name: "Decision", description: "Technical and business decisions" },
    { id: "custom", name: "Custom", description: "Other knowledge" },
  ]

  return jsonResponse({ success: true, data: categories })
}

/**
 * POST /api/v1/hub/context
 * Get relevant context for an agent query (internal API)
 */
export async function getAgentContext(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as {
      query: string
      agent?: string
      user_id?: string
      user_department?: string
      max_entries?: number
    }

    if (!input.query) {
      return errorResponse("Query is required", 400)
    }

    const userId = input.user_id ?? "anonymous"
    const userDepartment = input.user_department
    const maxEntries = input.max_entries ?? 5

    let entries = await loadEntries()

    // Filter by access
    entries = entries.filter((e) => checkAccess(e, userId, userDepartment))

    // Prioritize certain categories based on agent
    let priorityCategories: KnowledgeHubCategory[] = []
    if (input.agent) {
      switch (input.agent) {
        case "macro":
          priorityCategories = ["risk_log", "decision", "meeting_notes"]
          break
        case "decision":
          priorityCategories = ["decision", "lessons_learned", "risk_log"]
          break
        case "architect":
          priorityCategories = ["architecture", "prd", "decision"]
          break
        case "tdd-guide":
          priorityCategories = ["runbook", "lessons_learned", "faq"]
          break
        default:
          priorityCategories = []
      }
    }

    // Calculate relevance with priority boost
    const queryTags = extractTags(input.query)
    const results = entries
      .map((entry) => {
        let relevance = calculateRelevance(entry, input.query, queryTags)

        // Boost priority categories
        if (priorityCategories.includes(entry.category)) {
          relevance += 0.2
        }

        return { entry, relevance }
      })
      .filter((r) => r.relevance > 0.15)
      .sort((a, b) => b.relevance - a.relevance)
      .slice(0, maxEntries)

    // Format context for agent consumption
    const context = results.map((r) => ({
      title: r.entry.title,
      category: r.entry.category,
      content: r.entry.summary ?? r.entry.content.slice(0, 500),
      relevance: Math.round(r.relevance * 100),
      source_id: r.entry.id,
    }))

    return jsonResponse({ success: true, data: { context, count: context.length } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
