/**
 * Internationalized Messages Configuration Loader.
 *
 * Loads message templates from configuration files:
 * - Default: packages/ccode/src/config/messages.default.json
 * - User: ~/.codecoder/messages.json
 *
 * User configuration is merged on top of defaults, allowing selective overrides.
 *
 * @example
 * ```ts
 * import { t, getMessages } from "@/config/messages"
 *
 * // Simple message
 * const msg = t("task.acknowledged") // "🚀 收到，正在处理..."
 *
 * // Message with parameters
 * const error = t("task.failed", { error: "Network timeout" })
 * // "❌ 处理失败: Network timeout"
 * ```
 */

import path from "path"
import os from "os"
import { parse as parseJsonc, type ParseError as JsoncParseError, printParseErrorCode } from "jsonc-parser"
import { mergeDeep } from "remeda"
import { Log } from "@/util/log"

// Import default messages configuration
import defaultMessages from "./messages.default.json"

const log = Log.create({ service: "messages-loader" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Task lifecycle messages.
 */
export interface TaskMessages {
  acknowledged: string
  start_processing: string
  processing: string
  thinking: string
  progress: string
  progress_no_percent: string
  completed: string
  completed_with_summary: string
  failed: string
  failed_with_summary: string
  generating_result: string
  end_marker: string
  task_id_suffix: string
}

/**
 * Approval/HitL messages.
 */
export interface ApprovalMessages {
  title: string
  confirm_action: string
  confirm_with_info: string
  confirm_with_args: string
  approve: string
  approve_always: string
  reject: string
  approved: string
  approved_by: string
  rejected: string
  rejected_by: string
  rejected_with_reason: string
  pending: string
  waiting: string
  queue_title: string
  queue_empty: string
  select_prompt: string
}

/**
 * Status indicator messages.
 */
export interface StatusMessages {
  auto_approve: string
  pending_approval: string
  denied: string
  tool_executed: string
  tool_executing: string
  answer_received: string
  answer_failed: string
  option_selected: string
  pass: string
  needs_improvement: string
  existing_capabilities: string
  risk_warning: string
}

/**
 * Error messages.
 */
export interface ErrorMessages {
  load_failed: string
  approve_failed: string
  reject_failed: string
  operation_failed: string
  operation_failed_retry: string
  config_save_failed: string
  config_load_failed: string
  connection_lost: string
  telegram_not_configured: string
  verification_failed: string
  error_prefix: string
}

/**
 * Authorization/binding messages.
 */
export interface AuthMessages {
  binding_success: string
}

/**
 * Search-related messages.
 */
export interface SearchMessages {
  no_results: string
}

/**
 * Autonomous mode messages.
 */
export interface AutonomousMessages {
  task_completed: string
  task_incomplete: string
  status_solved: string
  status_not_solved: string
  build_success: string
  build_failed: string
  decision_paused: string
}

/**
 * Context management messages.
 */
export interface ContextMessages {
  clear_failed: string
  compact_failed: string
  clear_error_retry: string
}

/**
 * All message categories.
 */
export interface AllMessages {
  task: TaskMessages
  approval: ApprovalMessages
  status: StatusMessages
  error: ErrorMessages
  auth: AuthMessages
  search: SearchMessages
  autonomous: AutonomousMessages
  context: ContextMessages
}

/**
 * Root messages configuration.
 */
export interface MessagesConfig {
  locale: string
  version: string
  messages: AllMessages
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration Paths
// ─────────────────────────────────────────────────────────────────────────────

/** User configuration directory */
const CONFIG_DIR = path.join(os.homedir(), ".codecoder")

/** User messages configuration file */
const MESSAGES_PATH = path.join(CONFIG_DIR, "messages.json")

// ─────────────────────────────────────────────────────────────────────────────
// Loading Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Load a JSON/JSONC file.
 * Returns null if file doesn't exist or parsing fails.
 */
async function loadJsonFile<T>(filepath: string): Promise<T | null> {
  try {
    const text = await Bun.file(filepath).text()
    const errors: JsoncParseError[] = []
    const data = parseJsonc(text, errors, { allowTrailingComma: true })

    if (errors.length) {
      const errorDetails = errors.map((e) => printParseErrorCode(e.error)).join(", ")
      log.warn("JSONC parse errors", { path: filepath, errors: errorDetails })
      return null
    }

    return data as T
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      return null
    }
    log.error("Failed to load config file", { path: filepath, error })
    return null
  }
}

/**
 * Load messages configuration.
 *
 * Priority (lowest to highest):
 * 1. Default messages (bundled)
 * 2. User messages (~/.codecoder/messages.json)
 */
export async function loadMessages(): Promise<MessagesConfig> {
  // Start with default configuration
  let config = defaultMessages as MessagesConfig

  // Try to load user configuration
  const userConfig = await loadJsonFile<Partial<MessagesConfig>>(MESSAGES_PATH)

  if (userConfig) {
    // Deep merge user config on top of defaults
    config = mergeDeep(config, userConfig) as MessagesConfig
    log.debug("Loaded user messages config", {
      path: MESSAGES_PATH,
      locale: userConfig.locale ?? config.locale,
    })
  }

  return config
}

// ─────────────────────────────────────────────────────────────────────────────
// Cached Configuration
// ─────────────────────────────────────────────────────────────────────────────

/** Cached messages configuration */
let cachedConfig: MessagesConfig | null = null

/**
 * Get messages configuration (cached).
 *
 * Loads configuration on first call and caches it for subsequent calls.
 */
export async function getMessages(): Promise<MessagesConfig> {
  if (!cachedConfig) {
    cachedConfig = await loadMessages()
  }
  return cachedConfig
}

/**
 * Get messages configuration synchronously (uses default if not loaded).
 *
 * Call `getMessages()` first to ensure user config is loaded.
 */
export function getMessagesSync(): MessagesConfig {
  return cachedConfig ?? (defaultMessages as MessagesConfig)
}

/**
 * Reload messages configuration (clears cache).
 */
export async function reloadMessages(): Promise<MessagesConfig> {
  cachedConfig = null
  return getMessages()
}

// ─────────────────────────────────────────────────────────────────────────────
// Translation Function
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Get a translated message with optional parameter interpolation.
 *
 * @param key - Dot-notation key path (e.g., "task.failed", "approval.approve")
 * @param params - Optional parameters to interpolate (e.g., { error: "timeout" })
 * @returns The translated message, or the key if not found
 *
 * @example
 * ```ts
 * t("task.acknowledged")
 * // => "🚀 收到，正在处理..."
 *
 * t("task.failed", { error: "Network timeout" })
 * // => "❌ 处理失败: Network timeout"
 *
 * t("approval.rejected_with_reason", {
 *   approver: "admin",
 *   reason: "Too risky",
 *   time: "2024-01-01 12:00"
 * })
 * // => "❌ 已拒绝\n审批人: admin\n原因: Too risky\n时间: 2024-01-01 12:00"
 * ```
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const config = getMessagesSync()
  const parts = key.split(".")

  if (parts.length !== 2) {
    log.warn("Invalid message key format", { key })
    return key
  }

  const [category, messageKey] = parts
  const allMessages = config.messages as unknown as Record<string, Record<string, string>>
  const categoryMessages = allMessages[category!]

  if (!categoryMessages) {
    log.warn("Unknown message category", { category })
    return key
  }

  const template = categoryMessages[messageKey!]
  if (!template) {
    log.warn("Unknown message key", { key })
    return key
  }

  // If no params, return template as-is
  if (!params) {
    return template
  }

  // Interpolate parameters: replace {key} with value
  return template.replace(/\{(\w+)\}/g, (match, paramKey) => {
    const value = params[paramKey]
    return value !== undefined ? String(value) : match
  })
}

/**
 * Create a scoped translation function for a specific category.
 *
 * @param category - Message category (e.g., "task", "approval")
 * @returns A translation function scoped to that category
 *
 * @example
 * ```ts
 * const taskT = createScopedT("task")
 * taskT("acknowledged") // => "🚀 收到，正在处理..."
 * taskT("failed", { error: "timeout" }) // => "❌ 处理失败: timeout"
 * ```
 */
export function createScopedT(category: keyof AllMessages): (key: string, params?: Record<string, string | number>) => string {
  return (key, params) => t(`${category}.${key}`, params)
}

// ─────────────────────────────────────────────────────────────────────────────
// Direct Category Accessors
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Get the current locale.
 */
export function getLocale(): string {
  return getMessagesSync().locale
}

/**
 * Get all messages for a specific category.
 */
export function getCategoryMessages<K extends keyof AllMessages>(category: K): AllMessages[K] {
  return getMessagesSync().messages[category]
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience Exports
// ─────────────────────────────────────────────────────────────────────────────

/** Task messages accessor */
export const taskT = createScopedT("task")

/** Approval messages accessor */
export const approvalT = createScopedT("approval")

/** Status messages accessor */
export const statusT = createScopedT("status")

/** Error messages accessor */
export const errorT = createScopedT("error")

/** Auth messages accessor */
export const authT = createScopedT("auth")

/** Search messages accessor */
export const searchT = createScopedT("search")

/** Autonomous messages accessor */
export const autonomousT = createScopedT("autonomous")

/** Context messages accessor */
export const contextT = createScopedT("context")
