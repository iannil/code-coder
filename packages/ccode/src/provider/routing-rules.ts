/**
 * LLM Routing Rules Configuration
 *
 * Defines task classification rules, role-based model permissions,
 * and routing strategies for intelligent LLM selection.
 *
 * Part of Phase 14: Intelligent LLM Router
 * Updated in Phase 3 of hardcoded keywords cleanup: Configuration now loaded from JSON.
 */

import { z } from "zod"
import path from "path"
import os from "os"
import { parse as parseJsonc, type ParseError as JsoncParseError, printParseErrorCode } from "jsonc-parser"
import { mergeDeep } from "remeda"
import { Log } from "@/util/log"

// Import default routing configuration
import defaultRoutingConfig from "@/config/routing.default.json"

const log = Log.create({ service: "routing-config" })

// Helper to avoid Zod v4.1.8 + Bun escapeRegex issue with .default([])
const defaultArray = <T extends z.ZodTypeAny>(schema: T) =>
  z.array(schema).optional().transform((v) => v ?? [])

// ============================================================================
// Types & Schemas
// ============================================================================

/** Task types for classification */
export const TaskType = z.enum(["coding", "analysis", "chat", "sensitive"])
export type TaskType = z.infer<typeof TaskType>

/** User roles for RBAC */
export const UserRole = z.enum(["admin", "developer", "intern", "guest"])
export type UserRole = z.infer<typeof UserRole>

/** Model tier categories */
export const ModelTier = z.enum(["premium", "standard", "budget", "local"])
export type ModelTier = z.infer<typeof ModelTier>

/** Classification rule for detecting task types */
export const ClassificationRule = z.object({
  /** Unique rule identifier */
  id: z.string(),
  /** Task type this rule detects */
  taskType: TaskType,
  /** Priority (lower = higher priority) */
  priority: z.number().int().min(0).default(10),
  /** Regex patterns to match (any match triggers rule) */
  patterns: defaultArray(z.string()),
  /** Keywords to match (any match triggers rule) */
  keywords: defaultArray(z.string()),
  /** Agent names that trigger this task type */
  agents: defaultArray(z.string()),
  /** Whether rule is enabled */
  enabled: z.boolean().default(true),
})
export type ClassificationRule = z.infer<typeof ClassificationRule>

/** Model definition for routing */
export const RoutableModel = z.object({
  /** Model ID (e.g., "claude-3-opus", "gpt-4o-mini") */
  id: z.string(),
  /** Display name */
  name: z.string(),
  /** Provider (anthropic, openai, google, ollama) */
  provider: z.string(),
  /** Model tier for permission checks */
  tier: ModelTier,
  /** Task types this model is optimized for */
  optimizedFor: defaultArray(TaskType),
  /** Cost per 1M tokens (for cost optimization) */
  costPer1M: z.number().default(0),
  /** Whether model is available */
  available: z.boolean().default(true),
  /** Whether model runs locally (for sensitive data) */
  isLocal: z.boolean().default(false),
})
export type RoutableModel = z.infer<typeof RoutableModel>

/** Role permission configuration */
export const RolePermission = z.object({
  /** Role name */
  role: UserRole,
  /** Allowed model tiers */
  allowedTiers: z.array(ModelTier),
  /** Specific model IDs allowed (overrides tier restrictions) */
  allowedModels: defaultArray(z.string()),
  /** Specific model IDs denied (overrides tier allowances) */
  deniedModels: defaultArray(z.string()),
  /** Daily token limit */
  dailyTokenLimit: z.number().int().min(0).default(1_000_000),
  /** Monthly token limit */
  monthlyTokenLimit: z.number().int().min(0).default(10_000_000),
})
export type RolePermission = z.infer<typeof RolePermission>

/** Routing decision result */
export const RoutingDecision = z.object({
  /** Selected model ID */
  modelId: z.string(),
  /** Selected model display name */
  modelName: z.string(),
  /** Provider to use */
  provider: z.string(),
  /** Detected task type */
  taskType: TaskType,
  /** User's role */
  userRole: UserRole,
  /** Whether a fallback was used */
  isFallback: z.boolean().default(false),
  /** Reason for the decision */
  reason: z.string(),
  /** Warnings (e.g., quota warning) */
  warnings: defaultArray(z.string()),
})
export type RoutingDecision = z.infer<typeof RoutingDecision>

/** Task-to-model preferences schema */
export const TaskModelPreferences = z.object({
  coding: z.array(z.string()).default([]),
  analysis: z.array(z.string()).default([]),
  chat: z.array(z.string()).default([]),
  sensitive: z.array(z.string()).default([]),
})
export type TaskModelPreferences = z.infer<typeof TaskModelPreferences>

/** Routing configuration */
export const RoutingConfig = z.object({
  /** Whether routing is enabled */
  enabled: z.boolean().default(true),
  /** Default model for unclassified tasks */
  defaultModelId: z.string().default("claude-3-5-sonnet"),
  /** Default role for unknown users */
  defaultRole: UserRole.default("guest"),
  /** Classification rules */
  rules: defaultArray(ClassificationRule),
  /** Role permissions */
  rolePermissions: defaultArray(RolePermission),
  /** Available models */
  models: defaultArray(RoutableModel),
  /** Enable DLP integration for sensitive content detection */
  enableDlpIntegration: z.boolean().default(true),
  /** Force local model for DLP-detected sensitive content */
  forceLocalForSensitive: z.boolean().default(true),
  /** Task-to-model preferences */
  taskModelPreferences: TaskModelPreferences.optional(),
})
export type RoutingConfig = z.infer<typeof RoutingConfig>

// ============================================================================
// Configuration Paths
// ============================================================================

/** User configuration directory */
const CONFIG_DIR = path.join(os.homedir(), ".codecoder")

/** User routing configuration file */
const ROUTING_CONFIG_PATH = path.join(CONFIG_DIR, "routing.json")

// ============================================================================
// Configuration Loading
// ============================================================================

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

/** Cached routing configuration */
let cachedConfig: RoutingConfig | null = null

/**
 * Get the default routing configuration from bundled JSON.
 */
function getDefaultConfig(): RoutingConfig {
  return defaultRoutingConfig as unknown as RoutingConfig
}

/**
 * Load routing configuration.
 *
 * Priority (lowest to highest):
 * 1. Default routing config (bundled)
 * 2. User routing config (~/.codecoder/routing.json)
 */
export async function loadRoutingConfig(): Promise<RoutingConfig> {
  // Start with default configuration
  let config = getDefaultConfig()

  // Try to load user configuration
  const userConfig = await loadJsonFile<Partial<RoutingConfig>>(ROUTING_CONFIG_PATH)

  if (userConfig) {
    // Deep merge user config on top of defaults
    config = mergeDeep(config, userConfig) as RoutingConfig
    log.debug("Loaded user routing config", {
      path: ROUTING_CONFIG_PATH,
      models: userConfig.models?.length ?? 0,
      rules: userConfig.rules?.length ?? 0,
    })
  }

  return config
}

/**
 * Get routing configuration (cached).
 *
 * Loads configuration on first call and caches it for subsequent calls.
 */
export async function getRoutingConfig(): Promise<RoutingConfig> {
  if (!cachedConfig) {
    cachedConfig = await loadRoutingConfig()
  }
  return cachedConfig
}

/**
 * Get routing configuration synchronously (uses default if not loaded).
 *
 * Call `getRoutingConfig()` first to ensure user config is loaded.
 */
export function getRoutingConfigSync(): RoutingConfig {
  return cachedConfig ?? getDefaultConfig()
}

/**
 * Reload routing configuration (clears cache).
 */
export async function reloadRoutingConfig(): Promise<RoutingConfig> {
  cachedConfig = null
  return getRoutingConfig()
}

// ============================================================================
// Task-to-Model Mapping
// ============================================================================

/**
 * Get task model preferences from config.
 * Falls back to default preferences if not configured.
 */
export function getTaskModelPreferences(): Record<TaskType, string[]> {
  const config = getRoutingConfigSync()
  const prefs = config.taskModelPreferences
  if (prefs) {
    return {
      coding: prefs.coding,
      analysis: prefs.analysis,
      chat: prefs.chat,
      sensitive: prefs.sensitive,
    }
  }
  // Fallback to defaults from JSON config
  const defaultConfig = getDefaultConfig()
  const defaultPrefs = defaultConfig.taskModelPreferences
  if (defaultPrefs) {
    return {
      coding: defaultPrefs.coding,
      analysis: defaultPrefs.analysis,
      chat: defaultPrefs.chat,
      sensitive: defaultPrefs.sensitive,
    }
  }
  // Ultimate fallback
  return {
    coding: ["claude-3-5-sonnet", "gpt-4o", "ollama-codellama", "ollama-deepseek-coder"],
    analysis: ["claude-3-opus", "o1", "gpt-4o", "claude-3-5-sonnet"],
    chat: ["gpt-4o-mini", "claude-3-haiku", "ollama-llama3"],
    sensitive: ["ollama-llama3", "ollama-codellama", "ollama-deepseek-coder"],
  }
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Get model tier cost rank (lower = cheaper)
 */
export function getTierCostRank(tier: ModelTier): number {
  const ranks: Record<ModelTier, number> = {
    local: 0,
    budget: 1,
    standard: 2,
    premium: 3,
  }
  return ranks[tier]
}

/**
 * Check if a role can access a model tier
 */
export function canRoleAccessTier(role: UserRole, tier: ModelTier, permissions: RolePermission[]): boolean {
  const perm = permissions.find((p) => p.role === role)
  if (!perm) return false
  return perm.allowedTiers.includes(tier)
}

/**
 * Check if a role can access a specific model
 */
export function canRoleAccessModel(
  role: UserRole,
  modelId: string,
  model: RoutableModel,
  permissions: RolePermission[],
): boolean {
  const perm = permissions.find((p) => p.role === role)
  if (!perm) return false

  // Check explicit denials first
  if (perm.deniedModels.includes(modelId)) {
    return false
  }

  // Check explicit allowances
  if (perm.allowedModels.includes(modelId)) {
    return true
  }

  // Check tier-based access
  return perm.allowedTiers.includes(model.tier)
}

/**
 * Find the best available model for a task type and role
 */
export function findBestModel(
  taskType: TaskType,
  role: UserRole,
  models: RoutableModel[],
  permissions: RolePermission[],
): RoutableModel | undefined {
  const preferences = getTaskModelPreferences()[taskType]

  // Try each preferred model in order
  for (const modelId of preferences) {
    const model = models.find((m) => m.id === modelId && m.available)
    if (model && canRoleAccessModel(role, modelId, model, permissions)) {
      return model
    }
  }

  // Fallback to any available model the role can access
  const availableModels = models
    .filter((m) => m.available && canRoleAccessModel(role, m.id, m, permissions))
    .sort((a, b) => getTierCostRank(a.tier) - getTierCostRank(b.tier))

  return availableModels[0]
}
