/**
 * Agent Keywords Configuration Loader.
 *
 * Loads trigger keywords from configuration files:
 * - Default: packages/ccode/src/agent/keywords.default.json
 * - User: ~/.codecoder/keywords.json
 *
 * User configuration is merged on top of defaults, allowing selective overrides.
 */

import path from "path"
import os from "os"
import { parse as parseJsonc, type ParseError as JsoncParseError, printParseErrorCode } from "jsonc-parser"
import { mergeDeep } from "remeda"
import { Log } from "@/util/log"

// Import default keywords configuration
import defaultKeywords from "../agent/keywords.default.json"

const log = Log.create({ service: "keywords-loader" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Trigger rule for agent detection.
 */
export type TriggerRule = string | AdvancedTriggerRule

/**
 * Advanced trigger rule with type and options.
 */
export interface AdvancedTriggerRule {
  type: "keyword" | "pattern" | "context" | "event"
  value: string
  priority?: number
  description?: string
}

/**
 * Keywords configuration for a single agent.
 */
export interface AgentKeywords {
  triggers: TriggerRule[]
  aliases: string[]
  priority: number
  enabled?: boolean
}

/**
 * Default agent settings.
 */
export interface DefaultsConfig {
  agent: string
  cli_agent: string
  im_agent: string
  use_implicit_matching: boolean
}

/**
 * Root keywords configuration.
 */
export interface KeywordsConfig {
  version: string
  agents: Record<string, AgentKeywords>
  defaults: DefaultsConfig
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration Paths
// ─────────────────────────────────────────────────────────────────────────────

/** User configuration directory */
const CONFIG_DIR = path.join(os.homedir(), ".codecoder")

/** User keywords configuration file */
const KEYWORDS_PATH = path.join(CONFIG_DIR, "keywords.json")

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
 * Load keywords configuration.
 *
 * Priority (lowest to highest):
 * 1. Default keywords (bundled)
 * 2. User keywords (~/.codecoder/keywords.json)
 */
export async function loadKeywords(): Promise<KeywordsConfig> {
  // Start with default configuration
  let config = defaultKeywords as KeywordsConfig

  // Try to load user configuration
  const userConfig = await loadJsonFile<Partial<KeywordsConfig>>(KEYWORDS_PATH)

  if (userConfig) {
    // Deep merge user config on top of defaults
    config = mergeDeep(config, userConfig) as KeywordsConfig
    log.debug("Loaded user keywords config", {
      path: KEYWORDS_PATH,
      agents: Object.keys(userConfig.agents ?? {}).length,
    })
  }

  return config
}

// ─────────────────────────────────────────────────────────────────────────────
// Cached Configuration
// ─────────────────────────────────────────────────────────────────────────────

/** Cached keywords configuration */
let cachedConfig: KeywordsConfig | null = null

/**
 * Get keywords configuration (cached).
 *
 * Loads configuration on first call and caches it for subsequent calls.
 */
export async function getKeywords(): Promise<KeywordsConfig> {
  if (!cachedConfig) {
    cachedConfig = await loadKeywords()
  }
  return cachedConfig
}

/**
 * Get keywords configuration synchronously (uses default if not loaded).
 *
 * Call `getKeywords()` first to ensure user config is loaded.
 */
export function getKeywordsSync(): KeywordsConfig {
  return cachedConfig ?? (defaultKeywords as KeywordsConfig)
}

/**
 * Reload keywords configuration (clears cache).
 */
export async function reloadKeywords(): Promise<KeywordsConfig> {
  cachedConfig = null
  return getKeywords()
}

// ─────────────────────────────────────────────────────────────────────────────
// Detection Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Detect agent from @mention alias.
 *
 * @param message - User message to analyze
 * @param config - Keywords configuration
 * @returns Agent name if found, null otherwise
 *
 * @example
 * ```ts
 * detectAlias("@macro 分析PMI", config) // => "macro"
 * detectAlias("@宏观 分析PMI", config) // => "macro"
 * ```
 */
export function detectAlias(message: string, config: KeywordsConfig): string | null {
  const text = message.trim()

  // Must start with @
  if (!text.startsWith("@")) {
    return null
  }

  // Extract the mention (first word after @)
  const parts = text.split(/\s+/)
  const mention = parts[0]?.slice(1).toLowerCase()
  if (!mention) {
    return null
  }

  // Search through all agents for matching alias
  for (const [agentName, keywords] of Object.entries(config.agents)) {
    if (keywords.enabled === false) {
      continue
    }

    // Check exact agent name match
    if (agentName.toLowerCase() === mention) {
      return agentName
    }

    // Check aliases
    for (const alias of keywords.aliases) {
      if (alias.toLowerCase() === mention) {
        return agentName
      }
    }
  }

  return null
}

/**
 * Detect agent from message content using keyword triggers.
 *
 * This performs implicit keyword matching. Note: disabled by default
 * to avoid misrouting.
 *
 * @param message - User message to analyze
 * @param config - Keywords configuration
 * @returns Agent name with highest priority match, or null
 */
export function detectTrigger(message: string, config: KeywordsConfig): string | null {
  // Check if implicit matching is enabled
  if (!config.defaults.use_implicit_matching) {
    return null
  }

  const text = message.toLowerCase()
  let bestMatch: { agent: string; priority: number } | null = null

  for (const [agentName, keywords] of Object.entries(config.agents)) {
    if (keywords.enabled === false) {
      continue
    }

    for (const trigger of keywords.triggers) {
      let matched = false
      let priority = keywords.priority

      if (typeof trigger === "string") {
        matched = text.includes(trigger.toLowerCase())
      } else {
        switch (trigger.type) {
          case "keyword":
            matched = text.includes(trigger.value.toLowerCase())
            break
          case "pattern":
          case "context":
            try {
              matched = new RegExp(trigger.value, "i").test(text)
            } catch {
              matched = false
            }
            break
          case "event":
            // Events are handled separately
            matched = false
            break
        }
        priority = trigger.priority ?? keywords.priority
      }

      if (matched) {
        if (!bestMatch || priority > bestMatch.priority) {
          bestMatch = { agent: agentName, priority }
        }
        break // Only count first match per agent
      }
    }
  }

  return bestMatch?.agent ?? null
}

/**
 * Combined agent detection: alias → trigger → default.
 *
 * @param message - User message to analyze
 * @param defaultAgent - Fallback agent if no match found
 * @param config - Keywords configuration
 * @returns Detected agent name (never returns null)
 */
export function detectAgent(
  message: string,
  defaultAgent: string,
  config: KeywordsConfig
): string {
  // 1. Try @mention alias detection
  const aliasMatch = detectAlias(message, config)
  if (aliasMatch) {
    return aliasMatch
  }

  // 2. Try implicit keyword matching (if enabled)
  const triggerMatch = detectTrigger(message, config)
  if (triggerMatch) {
    return triggerMatch
  }

  // 3. Fallback to default
  return defaultAgent
}

/**
 * Get the trigger priority for a specific keyword.
 *
 * @param keyword - The keyword to look up
 * @param agentName - The agent to search in
 * @param config - Keywords configuration
 * @returns Priority value, or default (5) if not found
 */
export function getTriggerPriority(
  keyword: string,
  agentName: string,
  config: KeywordsConfig
): number {
  const agent = config.agents[agentName]
  if (!agent) {
    return 5
  }

  const lowerKeyword = keyword.toLowerCase()
  for (const trigger of agent.triggers) {
    if (typeof trigger === "string") {
      if (trigger.toLowerCase() === lowerKeyword) {
        return agent.priority
      }
    } else if (trigger.value.toLowerCase() === lowerKeyword) {
      return trigger.priority ?? agent.priority
    }
  }

  return agent.priority
}
