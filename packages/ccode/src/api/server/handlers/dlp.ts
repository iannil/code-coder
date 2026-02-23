/**
 * DLP (Data Leakage Prevention) API Handler
 *
 * Provides sensitive data detection and protection endpoints.
 * Integrates with Zero-Gateway for security sandbox.
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { Global } from "@/global"
import path from "path"
import fs from "fs/promises"

// ============================================================================
// Types
// ============================================================================

type DlpAction = "block" | "redact" | "warn" | "log"
type DlpRuleType = "regex" | "keyword" | "pattern" | "custom"

interface DlpRule {
  id: string
  name: string
  description?: string
  type: DlpRuleType
  pattern: string
  action: DlpAction
  enabled: boolean
  categories: string[]
  replacement?: string
  priority: number
  match_count: number
  created_at: string
  updated_at: string
}

interface DlpIncident {
  id: string
  rule_id: string
  rule_name: string
  action_taken: DlpAction
  content_preview: string
  user_id?: string
  session_id?: string
  triggered_at: string
  metadata?: Record<string, unknown>
}

interface DlpWhitelistEntry {
  id: string
  pattern: string
  description?: string
  created_at: string
  created_by?: string
}

interface DlpConfig {
  enabled: boolean
  default_action: DlpAction
  log_incidents: boolean
  notify_on_block: boolean
  max_incidents_per_session: number
}

interface DlpSummary {
  total_rules: number
  active_rules: number
  incidents_24h: number
  incidents_7d: number
  top_triggered_rules: Array<{
    rule_id: string
    rule_name: string
    count: number
  }>
}

// ============================================================================
// Storage
// ============================================================================

const DLP_DIR = path.join(Global.Path.data, "dlp")

async function ensureDlpDir(): Promise<void> {
  await fs.mkdir(DLP_DIR, { recursive: true })
}

async function loadRules(): Promise<DlpRule[]> {
  await ensureDlpDir()
  const rulesFile = path.join(DLP_DIR, "rules.json")

  try {
    const content = await fs.readFile(rulesFile, "utf-8")
    return JSON.parse(content)
  } catch {
    // Return default rules if none exist
    const defaultRules: DlpRule[] = [
      {
        id: "rule-api-keys",
        name: "API Keys",
        description: "Detect API keys in format sk-xxx, pk-xxx, etc.",
        type: "regex",
        pattern: "(sk|pk|api|token)[-_][a-zA-Z0-9]{20,}",
        action: "redact",
        enabled: true,
        categories: ["credentials"],
        replacement: "[REDACTED_API_KEY]",
        priority: 1,
        match_count: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
      {
        id: "rule-aws-keys",
        name: "AWS Access Keys",
        description: "Detect AWS access key IDs",
        type: "regex",
        pattern: "AKIA[0-9A-Z]{16}",
        action: "block",
        enabled: true,
        categories: ["credentials", "cloud"],
        replacement: "[REDACTED_AWS_KEY]",
        priority: 1,
        match_count: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
      {
        id: "rule-credit-cards",
        name: "Credit Cards",
        description: "Detect credit card numbers",
        type: "regex",
        pattern: "\\b(?:\\d{4}[- ]?){3}\\d{4}\\b",
        action: "redact",
        enabled: true,
        categories: ["pii", "financial"],
        replacement: "[REDACTED_CC]",
        priority: 2,
        match_count: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
      {
        id: "rule-ssn",
        name: "Social Security Numbers",
        description: "Detect US Social Security Numbers",
        type: "regex",
        pattern: "\\b\\d{3}-\\d{2}-\\d{4}\\b",
        action: "block",
        enabled: true,
        categories: ["pii"],
        replacement: "[REDACTED_SSN]",
        priority: 1,
        match_count: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
      {
        id: "rule-email",
        name: "Email Addresses",
        description: "Detect email addresses",
        type: "regex",
        pattern: "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}",
        action: "warn",
        enabled: false,
        categories: ["pii"],
        replacement: "[REDACTED_EMAIL]",
        priority: 3,
        match_count: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
    ]
    await saveRules(defaultRules)
    return defaultRules
  }
}

async function saveRules(rules: DlpRule[]): Promise<void> {
  await ensureDlpDir()
  const rulesFile = path.join(DLP_DIR, "rules.json")
  await fs.writeFile(rulesFile, JSON.stringify(rules, null, 2))
}

async function loadIncidents(): Promise<DlpIncident[]> {
  await ensureDlpDir()
  const incidentsFile = path.join(DLP_DIR, "incidents.json")

  try {
    const content = await fs.readFile(incidentsFile, "utf-8")
    return JSON.parse(content)
  } catch {
    return []
  }
}

async function saveIncidents(incidents: DlpIncident[]): Promise<void> {
  await ensureDlpDir()
  const incidentsFile = path.join(DLP_DIR, "incidents.json")
  await fs.writeFile(incidentsFile, JSON.stringify(incidents, null, 2))
}

async function loadWhitelist(): Promise<DlpWhitelistEntry[]> {
  await ensureDlpDir()
  const whitelistFile = path.join(DLP_DIR, "whitelist.json")

  try {
    const content = await fs.readFile(whitelistFile, "utf-8")
    return JSON.parse(content)
  } catch {
    return []
  }
}

async function saveWhitelist(whitelist: DlpWhitelistEntry[]): Promise<void> {
  await ensureDlpDir()
  const whitelistFile = path.join(DLP_DIR, "whitelist.json")
  await fs.writeFile(whitelistFile, JSON.stringify(whitelist, null, 2))
}

async function loadConfig(): Promise<DlpConfig> {
  await ensureDlpDir()
  const configFile = path.join(DLP_DIR, "config.json")

  try {
    const content = await fs.readFile(configFile, "utf-8")
    return JSON.parse(content)
  } catch {
    const defaultConfig: DlpConfig = {
      enabled: true,
      default_action: "warn",
      log_incidents: true,
      notify_on_block: true,
      max_incidents_per_session: 100,
    }
    await saveConfig(defaultConfig)
    return defaultConfig
  }
}

async function saveConfig(config: DlpConfig): Promise<void> {
  await ensureDlpDir()
  const configFile = path.join(DLP_DIR, "config.json")
  await fs.writeFile(configFile, JSON.stringify(config, null, 2))
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

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/v1/dlp/summary
 * Get DLP summary with statistics
 */
export async function getDlpSummary(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const rules = await loadRules()
    const incidents = await loadIncidents()

    const now = Date.now()
    const oneDayAgo = now - 24 * 60 * 60 * 1000
    const sevenDaysAgo = now - 7 * 24 * 60 * 60 * 1000

    const incidents24h = incidents.filter((i) => new Date(i.triggered_at).getTime() > oneDayAgo).length
    const incidents7d = incidents.filter((i) => new Date(i.triggered_at).getTime() > sevenDaysAgo).length

    // Count incidents by rule
    const ruleCounts = new Map<string, { rule_id: string; rule_name: string; count: number }>()
    for (const incident of incidents) {
      const existing = ruleCounts.get(incident.rule_id)
      if (existing) {
        existing.count++
      } else {
        ruleCounts.set(incident.rule_id, {
          rule_id: incident.rule_id,
          rule_name: incident.rule_name,
          count: 1,
        })
      }
    }

    const topTriggeredRules = Array.from(ruleCounts.values())
      .sort((a, b) => b.count - a.count)
      .slice(0, 5)

    const summary: DlpSummary = {
      total_rules: rules.length,
      active_rules: rules.filter((r) => r.enabled).length,
      incidents_24h: incidents24h,
      incidents_7d: incidents7d,
      top_triggered_rules: topTriggeredRules,
    }

    return jsonResponse({ success: true, data: summary })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/dlp/config
 * Get DLP configuration
 */
export async function getDlpConfig(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const config = await loadConfig()
    return jsonResponse({ success: true, data: config })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/v1/dlp/config
 * Update DLP configuration
 */
export async function updateDlpConfig(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<DlpConfig>

    const config = await loadConfig()
    const updated = { ...config, ...input }
    await saveConfig(updated)

    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/dlp/rules
 * List all DLP rules
 */
export async function listDlpRules(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const rules = await loadRules()
    return jsonResponse({ success: true, data: rules })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/dlp/rules
 * Create a new DLP rule
 */
export async function createDlpRule(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<DlpRule>

    if (!input.name) {
      return errorResponse("Rule name is required", 400)
    }
    if (!input.pattern) {
      return errorResponse("Pattern is required", 400)
    }

    const rules = await loadRules()
    const now = new Date().toISOString()

    const rule: DlpRule = {
      id: `rule-${Date.now()}`,
      name: input.name,
      description: input.description,
      type: input.type ?? "regex",
      pattern: input.pattern,
      action: input.action ?? "warn",
      enabled: input.enabled ?? true,
      categories: input.categories ?? [],
      replacement: input.replacement,
      priority: input.priority ?? 10,
      match_count: 0,
      created_at: now,
      updated_at: now,
    }

    // Validate regex pattern
    try {
      new RegExp(rule.pattern)
    } catch {
      return errorResponse("Invalid regex pattern", 400)
    }

    rules.push(rule)
    await saveRules(rules)

    return jsonResponse({ success: true, data: rule }, 201)
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/v1/dlp/rules/:id
 * Update a DLP rule
 */
export async function updateDlpRule(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Rule ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as Partial<DlpRule>

    const rules = await loadRules()
    const index = rules.findIndex((r) => r.id === id)

    if (index === -1) {
      return errorResponse(`Rule "${id}" not found`, 404)
    }

    // Validate regex pattern if changed
    if (input.pattern) {
      try {
        new RegExp(input.pattern)
      } catch {
        return errorResponse("Invalid regex pattern", 400)
      }
    }

    const updated: DlpRule = {
      ...rules[index],
      ...input,
      id, // Preserve ID
      updated_at: new Date().toISOString(),
    }

    rules[index] = updated
    await saveRules(rules)

    return jsonResponse({ success: true, data: updated })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/v1/dlp/rules/:id
 * Delete a DLP rule
 */
export async function deleteDlpRule(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Rule ID is required", 400)
    }

    const rules = await loadRules()
    const filtered = rules.filter((r) => r.id !== id)

    if (filtered.length === rules.length) {
      return errorResponse(`Rule "${id}" not found`, 404)
    }

    await saveRules(filtered)

    return jsonResponse({ success: true, data: { deleted: id } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/dlp/whitelist
 * List whitelist entries
 */
export async function listDlpWhitelist(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const whitelist = await loadWhitelist()
    return jsonResponse({ success: true, data: whitelist })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/dlp/whitelist
 * Add a whitelist entry
 */
export async function addDlpWhitelist(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as { pattern: string; description?: string }

    if (!input.pattern) {
      return errorResponse("Pattern is required", 400)
    }

    const whitelist = await loadWhitelist()

    const entry: DlpWhitelistEntry = {
      id: `whitelist-${Date.now()}`,
      pattern: input.pattern,
      description: input.description,
      created_at: new Date().toISOString(),
    }

    whitelist.push(entry)
    await saveWhitelist(whitelist)

    return jsonResponse({ success: true, data: entry }, 201)
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/v1/dlp/whitelist/:id
 * Remove a whitelist entry
 */
export async function deleteDlpWhitelist(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Whitelist entry ID is required", 400)
    }

    const whitelist = await loadWhitelist()
    const filtered = whitelist.filter((w) => w.id !== id)

    if (filtered.length === whitelist.length) {
      return errorResponse(`Whitelist entry "${id}" not found`, 404)
    }

    await saveWhitelist(filtered)

    return jsonResponse({ success: true, data: { deleted: id } })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/dlp/incidents
 * List DLP incidents
 */
export async function listDlpIncidents(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = req.url
    const limitParam = url.searchParams.get("limit")
    const limit = limitParam ? parseInt(limitParam, 10) : 50

    let incidents = await loadIncidents()

    // Sort by triggered_at descending (most recent first)
    incidents.sort((a, b) => new Date(b.triggered_at).getTime() - new Date(a.triggered_at).getTime())

    // Apply limit
    incidents = incidents.slice(0, limit)

    return jsonResponse({ success: true, data: incidents })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/dlp/scan
 * Scan content for sensitive data (internal API for integration)
 */
export async function scanContent(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as {
      content: string
      user_id?: string
      session_id?: string
    }

    if (!input.content) {
      return errorResponse("Content is required", 400)
    }

    const config = await loadConfig()
    if (!config.enabled) {
      return jsonResponse({
        success: true,
        data: {
          matched: false,
          action: "allow",
          content: input.content,
        },
      })
    }

    const rules = await loadRules()
    const whitelist = await loadWhitelist()
    const incidents = await loadIncidents()

    let finalContent = input.content
    let highestAction: DlpAction = "log"
    const matchedRules: Array<{ rule: DlpRule; matches: string[] }> = []

    // Check whitelist first
    const whitelistPatterns = whitelist.map((w) => new RegExp(w.pattern, "gi"))
    for (const pattern of whitelistPatterns) {
      finalContent = finalContent.replace(pattern, "[WHITELISTED]")
    }

    // Apply rules sorted by priority
    const sortedRules = rules.filter((r) => r.enabled).sort((a, b) => a.priority - b.priority)

    for (const rule of sortedRules) {
      try {
        const regex = new RegExp(rule.pattern, "gi")
        const matches = finalContent.match(regex)

        if (matches && matches.length > 0) {
          matchedRules.push({ rule, matches })

          // Track highest severity action
          const actionPriority: Record<DlpAction, number> = {
            log: 0,
            warn: 1,
            redact: 2,
            block: 3,
          }
          if (actionPriority[rule.action] > actionPriority[highestAction]) {
            highestAction = rule.action
          }

          // Apply action
          if (rule.action === "redact" || rule.action === "block") {
            const replacement = rule.replacement ?? "[REDACTED]"
            finalContent = finalContent.replace(regex, replacement)
          }

          // Update match count
          rule.match_count += matches.length

          // Log incident
          if (config.log_incidents) {
            incidents.push({
              id: `incident-${Date.now()}`,
              rule_id: rule.id,
              rule_name: rule.name,
              action_taken: rule.action,
              content_preview: matches[0].slice(0, 50) + (matches[0].length > 50 ? "..." : ""),
              user_id: input.user_id,
              session_id: input.session_id,
              triggered_at: new Date().toISOString(),
            })
          }
        }
      } catch {
        // Skip invalid regex
      }
    }

    // Save updated rules and incidents
    await saveRules(rules)
    await saveIncidents(incidents)

    return jsonResponse({
      success: true,
      data: {
        matched: matchedRules.length > 0,
        action: highestAction === "block" ? "block" : "allow",
        content: highestAction === "block" ? null : finalContent,
        matched_rules: matchedRules.map((m) => ({
          rule_id: m.rule.id,
          rule_name: m.rule.name,
          action: m.rule.action,
          match_count: m.matches.length,
        })),
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
